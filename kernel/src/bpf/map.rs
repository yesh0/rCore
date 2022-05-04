use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysError::*, SysResult};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::null;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use super::*;
use super::consts::*;

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

type HashType = u32; // emmm

#[derive(Clone, Copy)]
struct MapValue {
    pub ptr: *const u8,
    marker: PhantomData<&'static [u8]>, // how to handle this?
}

fn null_map_value() -> MapValue {
    MapValue {
        ptr: null(),
        marker: PhantomData,
    }
}

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
    map: BTreeMap<HashType, MapValue>,
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
