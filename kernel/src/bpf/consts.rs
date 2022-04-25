// see Linux kernel source /include/uapi/linux/bpf.h

// eBPF syscall commands
pub const BPF_MAP_CREATE: usize = 0;
pub const BPF_MAP_LOOKUP_ELEM: usize = 1;
pub const BPF_MAP_UPDATE_ELEM: usize = 2;
pub const BPF_MAP_DELETE_ELEM: usize = 3;
pub const BPF_MAP_GET_NEXT_KEY: usize = 4;
pub const BPF_PROG_LOAD: usize = 5;
pub const BPF_PROG_ATTACH: usize = 8;
pub const BPF_PROG_DETACH: usize = 9;

// eBPF map types
pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;
pub const BPF_MAP_TYPE_HASH: u32 = 1;
pub const BPF_MAP_TYPE_ARRAY: u32 = 2;
pub const BPF_MAP_TYPE_PROG_ARRAY: u32 = 3;
