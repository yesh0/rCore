use crate::trap::wall_tick;

// void *bpf_map_lookup_elem(struct bpf_map *map, const void *key)
extern "C" unsafe fn bpf_map_lookup_elem(map_fd: u32, key: *u8) {

}

// long bpf_map_update_elem(struct bpf_map *map, const void *key, const void *value, u64 flags)
extern "C" unsafe fn bpf_map_update_elem(map_fd: u32, key: *u8, value: *u8, flags: u64) -> i64 {
    0
}

// long bpf_map_delete_elem(struct bpf_map *map, const void *key)
extern "C" unsafe fn bpf_map_delete_elem(map_fd: u32, key: *u8) -> i64 {
    0
}

// long bpf_probe_read(void *dst, u32 size, const void *unsafe_ptr)
extern "C" unsafe fn bpf_probe_read(dst: *u8, size: u32, unsafe_ptr: *u8) -> i64 {
    0
}

// u64 bpf_ktime_get_ns(void)
// return current ktime
extern "C" unsafe fn bpf_ktime_get_ns() -> u64 {
    return wall_tick() as u64;
}

// long bpf_trace_printk(const char *fmt, u32 fmt_size, ...)
extern "C" unsafe fn bpf_trace_printk(fmt: *u8, fmt_size: u32, fmt_element: *u8) -> i64 {
    
    
    0
}