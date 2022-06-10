use crate::memory;
use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysError::*, SysResult};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::null;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::{mem, slice};
use rlibc::memcmp;

use super::consts::*;
use super::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MapAttr {
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct InternalMapAttr {
    pub key_size: usize,
    pub value_size: usize,
    pub max_entries: usize,
}

impl From<MapAttr> for InternalMapAttr {
    fn from(attr: MapAttr) -> Self {
        Self {
            key_size: attr.key_size as usize,
            value_size: attr.value_size as usize,
            max_entries: attr.max_entries as usize,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MapOpAttr {
    pub map_fd: u32,
    pub key: u64,
    pub value: u64,
    pub flags: u64,
}

pub trait BpfMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult;
    fn update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult;
    fn delete(&mut self, key: *const u8) -> SysResult;
    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult;
    fn get_attr(&self) -> InternalMapAttr;

    // this lookup is intended for the helper function
    fn lookup_helper(&self, key: *const u8) -> SysResult;
}

type HashCode = u32;
type MapKey = Box<[u8]>;
type MapValue = Box<[u8]>;

fn copy(dst: *mut u8, src: *const u8, len: usize) {
    let from = unsafe { from_raw_parts(src, len) };
    let to = unsafe { from_raw_parts_mut(dst, len) };
    to.copy_from_slice(from);
}

struct ArrayMap {
    attr: InternalMapAttr,
    storage: Vec<u8>,
}

struct HashMap {
    attr: InternalMapAttr,
    map: BTreeMap<HashCode, Vec<(MapKey, MapValue)>>,
    total_elems: usize, // total number of elements
}

impl ArrayMap {
    fn new(attr: InternalMapAttr) -> Self {
        let size = attr.max_entries * attr.value_size;
        let mut storage = Vec::with_capacity(size);
        storage.resize(size, 0u8);
        Self { attr, storage }
    }

    fn get_element_addr(&self, index: usize) -> usize {
        let offset = self.attr.value_size * index;
        self.storage.as_ptr() as usize + offset
    }
}

impl HashMap {
    fn new(attr: InternalMapAttr) -> Self {
        let map = BTreeMap::new();
        Self {
            attr,
            map,
            total_elems: 0,
        }
    }

    fn hash(kptr: *const u8, ksize: usize) -> HashCode {
        let seed: HashCode = 131313;
        let mut hash: HashCode = 0;
        for &i in unsafe { slice::from_raw_parts(kptr, ksize) } {
            hash = hash.wrapping_mul(seed).wrapping_add(i as HashCode);
        }
        hash
    }

    fn find(&self, kptr: *const u8) -> Option<&MapValue> {
        let hashcode = HashMap::hash(kptr, self.attr.key_size);
        if let Some(kvlist) = self.map.get(&hashcode) {
            for kv in kvlist {
                if unsafe { memcmp(kv.0.as_ptr(), kptr, self.attr.key_size) } == 0 {
                    return Some(&kv.1);
                }
            }
        }
        None
    }

    fn alloc(size: usize) -> Box<[u8]> {
        let mut storage = Vec::with_capacity(size);
        storage.resize(size, 0u8);
        storage.into_boxed_slice()
    }
}

impl BpfMap for ArrayMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult {
        let index = unsafe { *(key as *const u32) } as usize;
        if index >= self.attr.max_entries {
            return Err(ENOENT);
        }

        let p = self.get_element_addr(index);
        copy(value, p as *const u8, self.attr.value_size);
        Ok(0)
    }

    fn update(&mut self, key: *const u8, value: *const u8, _flags: u64) -> SysResult {
        let index = unsafe { *(key as *const u32) } as usize;
        if index >= self.attr.max_entries {
            return Err(ENOENT);
        }

        let p = self.get_element_addr(index);
        copy(p as *mut u8, value, self.attr.value_size);
        Ok(0)
    }

    fn delete(&mut self, key: *const u8) -> SysResult {
        Err(EINVAL)
    }

    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult {
        let out = next_key as *mut u32;
        let index = unsafe { *(key as *const u32) } as usize;
        if index >= self.attr.max_entries {
            unsafe {
                *out = 0u32;
            }
            return Ok(0);
        }

        if index < self.attr.max_entries - 1 {
            unsafe {
                *out = (index + 1) as u32;
            }
            Ok(0)
        } else {
            Err(ENOENT)
        }
    }

    fn get_attr(&self) -> InternalMapAttr {
        self.attr
    }

    fn lookup_helper(&self, key: *const u8) -> SysResult {
        let index = unsafe { *(key as *const u32) } as usize;
        if index >= self.attr.max_entries {
            return Err(ENOENT);
        }

        Ok(self.get_element_addr(index))
    }
}

impl BpfMap for HashMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult {
        if let Some(mv) = self.find(key) {
            copy(value, mv.as_ptr(), self.attr.value_size);
            Ok(0)
        } else {
            Err(ENOENT)
        }
    }

    fn update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult {
        // handle different flags, only 1 flags could be given

        // check flags
        if !(flags == BPF_ANY || flags == BPF_EXIST || flags == BPF_NOEXIST) {
            return Err(EINVAL);
        }

        // handle different cases
        let key_size = self.attr.key_size;
        let value_size = self.attr.value_size;
        if let Some(v) = self.find(key) {
            match flags {
                BPF_ANY | BPF_EXIST => {
                    copy(v.as_ptr() as *mut u8, value, value_size);
                    Ok(0)
                }
                _ => Err(EEXIST), // existing entry
            }
        } else {
            match flags {
                BPF_ANY | BPF_NOEXIST => {
                    if self.total_elems >= self.attr.max_entries {
                        return Err(ENOMEM); // should we return something else?
                    }
                    // create one, copy key and value into kernel space
                    let mut map_key = HashMap::alloc(key_size);
                    let mut map_value = HashMap::alloc(value_size);
                    copy(map_key.as_mut_ptr(), key, key_size);
                    copy(map_value.as_mut_ptr(), value, value_size);

                    let hashcode = HashMap::hash(key, key_size);
                    if let Some(vec) = self.map.get_mut(&hashcode) {
                        vec.push((map_key, map_value));
                    } else {
                        let vec = vec![(map_key, map_value)];
                        self.map.insert(hashcode, vec);
                    }
                    self.total_elems += 1;
                    Ok(0)
                }
                _ => Err(ENOENT),
            }
        }
    }

    fn delete(&mut self, key: *const u8) -> SysResult {
        let hashcode = HashMap::hash(key, self.attr.key_size);
        if let Some(kvlist) = self.map.get_mut(&hashcode) {
            for (i, kv) in kvlist.iter().enumerate() {
                if unsafe { memcmp(kv.0.as_ptr(), key, self.attr.key_size) } == 0 {
                    let _ = kvlist.remove(i);
                    self.total_elems -= 1;

                    // remove the empty Vec to avoid problems in next_key
                    if kvlist.is_empty() {
                        let _ = self.map.remove(&hashcode);
                    }
                    return Ok(0);
                }
            }
        }
        Err(ENOENT)
    }

    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult {
        let key_size = self.attr.key_size;
        let hashcode = HashMap::hash(key, key_size);

        let get_first_key = || {
            // returns the first valid key
            if let Some((_, first_vec)) = self.map.first_key_value() {
                let first_kv = first_vec.first().unwrap();
                copy(next_key, first_kv.0.as_ptr(), key_size);
                Ok(0)
            } else {
                // the hash map is empty
                Err(ENOENT)
            }
        };

        let mut iter = self.map.range(hashcode..);
        match iter.next() {
            Some((_, vec)) => {
                let mut opt_idx = None;
                for (i, kv) in vec.iter().enumerate() {
                    if unsafe { memcmp(kv.0.as_ptr(), key, key_size) } == 0 {
                        opt_idx = Some(i);
                        break;
                    }
                }
                if opt_idx.is_none() {
                    return get_first_key();
                }

                let index = opt_idx.unwrap();
                if index < vec.len() - 1 {
                    copy(next_key, vec[index + 1].0.as_ptr(), key_size);
                    return Ok(0);
                }

                // move on to next entry
                if let Some((_, next_vec)) = iter.next() {
                    let first_kv = next_vec.first().unwrap();
                    copy(next_key, first_kv.0.as_ptr(), key_size);
                    Ok(0)
                } else {
                    Err(ENOENT)
                }
            }
            None => get_first_key(),
        }
    }

    fn get_attr(&self) -> InternalMapAttr {
        self.attr
    }

    fn lookup_helper(&self, key: *const u8) -> SysResult {
        match self.find(key) {
            Some(map_key) => Ok(map_key.as_ptr() as usize),
            None => Err(ENOENT),
        }
    }
}

pub type SharedBpfMap = Arc<Mutex<dyn BpfMap + Send + Sync>>;

pub fn bpf_map_create(attr: MapAttr) -> SysResult {
    let internal_attr = InternalMapAttr::from(attr);
    match attr.map_type {
        BPF_MAP_TYPE_ARRAY => {
            // array index must have size of 4
            if internal_attr.key_size != 4 {
                return Err(EINVAL);
            }
            let map = ArrayMap::new(internal_attr);
            let shared_map = Arc::new(Mutex::new(map));
            let fd = bpf_allocate_fd();
            bpf_object_create_map(fd, shared_map);
            Ok(fd as usize)
        }
        BPF_MAP_TYPE_HASH => {
            let map = HashMap::new(internal_attr);
            let shared_map = Arc::new(Mutex::new(map));
            let fd = bpf_allocate_fd();
            bpf_object_create_map(fd, shared_map);
            Ok(fd as usize)
        }
        _ => Err(EINVAL),
    }
}

pub fn bpf_map_close(fd: u32) -> SysResult {
    bpf_object_remove(fd).map_or(Ok(0), |_| Err(ENOENT))
}

pub fn bpf_map_get_attr(fd: u32) -> Option<InternalMapAttr> {
    let bpf_objs = BPF_OBJECTS.lock();
    let obj = bpf_objs.get(&fd)?;
    let shared_map = obj.is_map()?;
    let attr = shared_map.lock().get_attr();
    Some(attr)
}

pub fn bpf_map_ops(fd: u32, op: usize, key: *const u8, value: *mut u8, flags: u64) -> SysResult {
    let bpf_objs = BPF_OBJECTS.lock();
    let obj = bpf_objs.get(&fd).ok_or(ENOENT)?;
    let shared_map = obj.is_map().ok_or(ENOENT)?;
    let mut map = shared_map.lock();
    match op {
        BPF_MAP_LOOKUP_ELEM => map.lookup(key, value),
        BPF_MAP_UPDATE_ELEM => map.update(key, value, flags),
        BPF_MAP_DELETE_ELEM => map.delete(key),
        BPF_MAP_GET_NEXT_KEY => map.next_key(key, value),
        _ => Err(EINVAL),
    }
}

pub fn bpf_map_lookup_helper(fd: u32, key: *const u8) -> SysResult {
    let bpf_objs = BPF_OBJECTS.lock();
    let obj = bpf_objs.get(&fd).ok_or(ENOENT)?;
    let shared_map = obj.is_map().ok_or(ENOENT)?;
    let map = shared_map.lock();
    map.lookup_helper(key)
}
