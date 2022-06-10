use crate::process::current_thread;
use crate::syscall::SysResult;
use core::ptr::{null, null_mut};

use super::{
    consts::*,
    map::{bpf_map_lookup_helper, bpf_map_ops},
};

pub type BpfHelperFn = fn(u64, u64, u64, u64, u64) -> i64;

pub const HELPER_FN_COUNT: usize = 17;
pub static HELPER_FN_TABLE: [BpfHelperFn; HELPER_FN_COUNT] = [
    bpf_nop,
    bpf_map_lookup_elem,
    bpf_map_update_elem,
    bpf_map_delete_elem,
    bpf_probe_read,
    bpf_ktime_get_ns,
    bpf_trace_printk,
    bpf_get_prandom_u32,
    bpf_get_smp_processor_id,
    bpf_nop, // bpf_skb_store_bytes
    bpf_nop, // bpf_l3_csum_replace
    bpf_nop, // bpf_l4_csum_replace
    bpf_nop, // bpf_tail_call
    bpf_nop, // bpf_clone_redirect
    bpf_get_current_pid_tgid,
    bpf_nop, // bpf_get_current_uid_gid
    bpf_get_current_comm,
];

// WARNING: be careful to use bpf_probe_read, bpf_get_current_pid_tgid & bpf_get_current_comm
// in syscall contexts. obtaining current process information may cause deadlock!

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
    // all Err variants are converted into 0 (NULL pointer)
    match res {
        Ok(val) => val as i64,
        Err(_) => 0,
    }
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

fn probe_read_user(dst: *mut u8, src: *const u8, len: usize) -> SysResult {
    let thread = current_thread().unwrap();
    let vm = thread.vm.lock();
    let src_slice = unsafe { vm.check_read_array(src, len)? };
    let dst_slice = unsafe { core::slice::from_raw_parts_mut(dst, len) };
    dst_slice.copy_from_slice(src_slice);
    Ok(0)
}

// TODO: probe read in kernel address space
// long bpf_probe_read(void *dst, u32 size, const void *unsafe_ptr)
fn bpf_probe_read(dst: u64, size: u64, src: u64, _1: u64, _2: u64) -> i64 {
    let res = probe_read_user(
        dst as usize as *mut u8,
        src as usize as *const u8,
        size as usize,
    );
    convert_result(res)
}

// u64 bpf_ktime_get_ns(void)
// return current ktime
fn bpf_ktime_get_ns(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    crate::arch::timer::timer_now().as_nanos() as i64
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

fn bpf_get_prandom_u32(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    todo!()
}

fn bpf_get_smp_processor_id(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    crate::arch::cpu::id() as i64
}

fn bpf_get_current_pid_tgid(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> i64 {
    let thread = current_thread().unwrap();
    let pid = thread.proc.busy_lock().pid.get() as i64;
    // NOTE: tgid is the same with pid
    (pid << 32) | pid
}

fn bpf_get_current_comm(dst: u64, buf_size: u64, _1: u64, _2: u64, _3: u64) -> i64 {
    let thread = current_thread().unwrap();
    let exec_str = thread.proc.busy_lock().exec_path.clone();
    let exec_path = exec_str.as_bytes();
    let len = exec_path.len();
    if (buf_size as usize) < len + 1 {
        return -1;
    }

    // NOTE: String is NOT null-terminated. we cannot copy len + 1 bytes directly.
    let dst_ptr = dst as *mut u8;
    unsafe {
        let dst_slice = core::slice::from_raw_parts_mut(dst_ptr, len);
        dst_slice.copy_from_slice(exec_path);
        *dst_ptr.add(len) = 0;
    }
    len as i64
}
