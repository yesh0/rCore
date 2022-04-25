use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysError::*, SysResult};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::null;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;

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

trait BpfMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult;
    fn update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult;
    fn delete(&mut self, key: *const u8) -> SysResult;
    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult;
    fn get_attr(&self) -> InternalMapAttr;
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
}

lazy_static! {
    // TODO: Mutex in Mutex seems stupid. better solution?
    static ref BPF_MAPS: Mutex<BTreeMap<u32, Arc<Mutex<dyn BpfMap + Send + Sync>>>> =
        Mutex::new(BTreeMap::new());
}

// eBPF map fd base
const BPF_MAP_FD_BASE: u32 = 0x10000000;

pub fn is_map_fd(fd: u32) -> bool {
    fd >= BPF_MAP_FD_BASE
}

pub fn bpf_map_create(attr: MapAttr) -> SysResult {
    let internal_attr = InternalMapAttr::from(attr);
    let mut bpf_maps = BPF_MAPS.lock();
    let map_fd = bpf_maps.len() as u32 + BPF_MAP_FD_BASE;
    match attr.map_type {
        BPF_MAP_TYPE_ARRAY => {
            // array index must have size of 4
            if internal_attr.key_size != 4 {
                return Err(EINVAL);
            }
            let map = ArrayMap::new(internal_attr);
            bpf_maps.insert(map_fd, Arc::new(Mutex::new(map)));
            Ok(map_fd as usize)
        }
        _ => Err(EINVAL)
    }
}

pub fn bpf_map_close(fd: u32) -> SysResult {
    BPF_MAPS.lock().remove(&fd).map_or(Ok(0), |_| Err(ENOENT))
}

pub fn bpf_map_get_attr(fd: u32) -> Option<InternalMapAttr> {
    Some(BPF_MAPS.lock().get(&fd)?.lock().get_attr())
}

pub fn bpf_map_ops(fd: u32, op: usize, key: *const u8, value: *mut u8, flags: u64) -> SysResult {
    let bpf_maps = BPF_MAPS.lock();
    let map_wrapper = bpf_maps.get(&fd).ok_or(ENOENT)?;
    let mut map = map_wrapper.lock();
    match op {
        BPF_MAP_LOOKUP_ELEM => map.lookup(key, value),
        BPF_MAP_UPDATE_ELEM => map.update(key, value, flags),
        BPF_MAP_DELETE_ELEM => map.delete(key),
        BPF_MAP_GET_NEXT_KEY => map.next_key(key, value),
        _ => Err(EINVAL)
    }
}
