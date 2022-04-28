use core::ptr::{null, null_mut};

use crate::trap::wall_tick;
use super::{map::bpf_map_ops, consts::*};

pub type BpfHelperFunc = unsafe fn(u64, u64, u64, u64, u64) -> u64;

// void *bpf_map_lookup_elem(struct bpf_map *map, const void *key)
unsafe fn bpf_map_lookup_elem(map_fd: u64, key: u64, _1: u64, _2: u64, _3: u64) -> u64 {
    let mut value: u64 = 0;
    // should we just directly unwrap ?
    bpf_map_ops(map_fd as u32, BPF_MAP_LOOKUP_ELEM, key as *const u8, (&mut value) as *mut u64 as *mut u8, 0).unwrap();
    return value;
}

// long bpf_map_update_elem(struct bpf_map *map, const void *key, const void *value, u64 flags)
unsafe fn bpf_map_update_elem(map_fd: u64, key: u64, value: u64, flags: u64, _1: u64) -> u64 {
    bpf_map_ops(map_fd as u32, BPF_MAP_UPDATE_ELEM, key as *const u8, value as *mut u8, flags).unwrap();
    0
}

// long bpf_map_delete_elem(struct bpf_map *map, const void *key)
unsafe fn bpf_map_delete_elem(map_fd: u64, key: u64, _1: u64, _2: u64, _3: u64) -> u64 {
    bpf_map_ops(map_fd as u32, BPF_MAP_DELETE_ELEM, key as *const u8, null_mut::<u8>(), 0).unwrap();
    0
}  

// long bpf_probe_read(void *dst, u32 size, const void *unsafe_ptr)
unsafe fn bpf_probe_read(dst: u64, size: u64, src: u64, _1: u64, _2: u64) -> u64 {
    let usize = size as usize;
    let dst_ptr = core::slice::from_raw_parts_mut(dst as *mut u8, usize);
    let src_ptr = core::slice::from_raw_parts(src as *const u8, usize);
    // copy
    for i in 0..usize {
        dst_ptr[i] = src_ptr[i];
    }
    size
}

// u64 bpf_ktime_get_ns(void)
// return current ktime
unsafe fn bpf_ktime_get_ns(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> u64 {
    return wall_tick() as u64;
}

// long bpf_trace_printk(const char *fmt, u32 fmt_size, ...)
unsafe fn bpf_trace_printk(fmt: u64, fmt_size: u64, p1: u64, p2: u64, p3: u64) -> u64 {
    let fmt = core::slice::from_raw_parts(fmt as *const u8, fmt_size as u32 as usize);
    print!(
        "{}",
        dyn_fmt::Arguments::new(core::str::from_utf8_unchecked(fmt), &[p1, p2, p3])
    );
    0 // ?
}