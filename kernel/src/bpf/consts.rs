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

// custom commands
pub const BPF_PROG_LOAD_EX: usize = 1000;

// eBPF map types
pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;
pub const BPF_MAP_TYPE_HASH: u32 = 1;
pub const BPF_MAP_TYPE_ARRAY: u32 = 2;
pub const BPF_MAP_TYPE_PROG_ARRAY: u32 = 3;

// eBPF LLVM relocations
// see https://www.kernel.org/doc/html/latest/bpf/llvm_reloc.html
pub const R_BPF_NONE: u32 = 0;
pub const R_BPF_64_64: u32 = 1;
pub const R_BPF_64_ABS64: u32 = 2;
pub const R_BPF_64_ABS32: u32 = 3;
pub const R_BPF_64_NODYLD32: u32 = 4;
pub const R_BPF_64_32: u32 = 10;

pub const BPF_ANY: u64 = 0;
pub const BPF_NOEXIST: u64 = 1;
pub const BPF_EXIST: U64 = 2;
pub const BPF_F_LOCK: u64 = 4;
