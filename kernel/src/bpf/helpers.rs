use crate::{syscall::SysResult, trap::uptime_msec};
use core::ptr::{null, null_mut};

use super::{
    consts::*,
    map::{bpf_map_lookup_helper, bpf_map_ops},
};

pub type BpfHelperFn = fn(u64, u64, u64, u64, u64) -> i64;

pub const HELPER_FN_COUNT: usize = 7;
pub static HELPER_FN_TABLE: [BpfHelperFn; HELPER_FN_COUNT] = [
    bpf_nop,
    bpf_map_lookup_elem,
    bpf_map_update_elem,
    bpf_map_delete_elem,
    bpf_probe_read,
    bpf_ktime_get_ns,
    bpf_trace_printk,
];

// NOTE: all Err variants are transformed into -1 to distinguish from a valid pointer
fn convert_result(result: SysResult) -> i64 {
    match result {
        Ok(val) => val as i64,
        Err(_) => -1,
    }
}

fn bpf_nop(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    0
}

// void *bpf_map_lookup_elem(struct bpf_map *map, const void *key)
fn bpf_map_lookup_elem(map_fd: u64, key: u64, _1: u64, _2: u64, _3: u64) -> i64 {
    let res = bpf_map_lookup_helper(map_fd as u32, key as *const u8);
    convert_result(res)
}

// long bpf_map_update_elem(struct bpf_map *map, const void *key, const void *value, u64 flags)
fn bpf_map_update_elem(map_fd: u64, key: u64, value: u64, flags: u64, _1: u64) -> i64 {
    let res = bpf_map_ops(
        map_fd as u32,
        BPF_MAP_UPDATE_ELEM,
        key as *const u8,
        value as *mut u8,
        flags,
    );
    convert_result(res)
}

// long bpf_map_delete_elem(struct bpf_map *map, const void *key)
fn bpf_map_delete_elem(map_fd: u64, key: u64, _1: u64, _2: u64, _3: u64) -> i64 {
    let res = bpf_map_ops(
        map_fd as u32,
        BPF_MAP_DELETE_ELEM,
        key as *const u8,
        null_mut::<u8>(),
        0,
    );
    convert_result(res)
}

// long bpf_probe_read(void *dst, u32 size, const void *unsafe_ptr)
fn bpf_probe_read(dst: u64, size: u64, src: u64, _1: u64, _2: u64) -> i64 {
    todo!()
}

// u64 bpf_ktime_get_ns(void)
// return current ktime
fn bpf_ktime_get_ns(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    return (uptime_msec() * 1000000) as i64
}

// long bpf_trace_printk(const char *fmt, u32 fmt_size, ...)
fn bpf_trace_printk(fmt: u64, fmt_size: u64, p1: u64, p2: u64, p3: u64) -> i64 {
    // TODO: check pointer
    let fmt = unsafe { core::slice::from_raw_parts(fmt as *const u8, fmt_size as u32 as usize) };
    println!(
        "{}",
        dyn_fmt::Arguments::new(
            unsafe { core::str::from_utf8_unchecked(fmt) },
            &[p1, p2, p3]
        )
    );
    0 // TODO: return number of bytes written
}
