use crate::sync::SpinLock as Mutex;
use crate::syscall::{SysError::*, SysResult};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::null;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;

#[derive(Clone, Copy)]
struct MapAttr {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
}

struct InternalMapAttr {
    key_size: usize,
    value_size: usize,
    max_entries: usize,
}

struct MapOpAttr {
    map_fd: u32,
    key: u64,
    value: u64,
    flags: u64,
}

trait BpfMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult;
    fn update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult;
    fn delete(&mut self, key: *const u8) -> SysResult;
    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult;
}

pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;
pub const BPF_MAP_TYPE_HASH: u32 = 1;
pub const BPF_MAP_TYPE_ARRAY: u32 = 2;
pub const BPF_MAP_TYPE_PROG_ARRAY: u32 = 3;

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
}

lazy_static! {
    static ref BPF_MAPS: Mutex<BTreeMap<u32, Arc<dyn BpfMap + Sync + Send>>> =
        Mutex::new(BTreeMap::new());
}
