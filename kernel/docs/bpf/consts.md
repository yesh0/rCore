# BPF Consts 模块

本部分提供了一系列用于BPF系统调用和类型标识的常量，同时也标识了LLVM编译器编译得到的需要进行重定向的BPF程序中的一系列常量。

## 定义

### 系统调用相关常量：

```rust
pub const BPF_MAP_CREATE: usize = 0;          // 创建BPF Map
pub const BPF_MAP_LOOKUP_ELEM: usize = 1;     // 查询BPF Map
pub const BPF_MAP_UPDATE_ELEM: usize = 2;     // 更新BPF Map中一个元素
pub const BPF_MAP_DELETE_ELEM: usize = 3;     // 删除BPF Map中一个元素
pub const BPF_MAP_GET_NEXT_KEY: usize = 4;    // 迭代BPF Map
pub const BPF_PROG_LOAD: usize = 5;           // 加载BPF程序
pub const BPF_PROG_ATTACH: usize = 8;         // 将BPF程序附加到插桩点上
pub const BPF_PROG_DETACH: usize = 9;         // 将BPF程序从插桩点上解除

pub const BPF_PROG_LOAD_EX: usize = 1000;     // 加载BPF程序（ELF，进行重定向）
```

### BPF Map类型

```rust
pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;       // 无特定类型
pub const BPF_MAP_TYPE_HASH: u32 = 1;         // 指定哈希表
pub const BPF_MAP_TYPE_ARRAY: u32 = 2;        // 指定线性表
pub const BPF_MAP_TYPE_PROG_ARRAY: u32 = 3;   // BPF程序表
```

### LLVM重定向常量

见[llvm_reloc](https://www.kernel.org/doc/html/latest/bpf/llvm_reloc.html)中的更详细定义。

```rust
pub const R_BPF_NONE: u32 = 0;
pub const R_BPF_64_64: u32 = 1;
pub const R_BPF_64_ABS64: u32 = 2;
pub const R_BPF_64_ABS32: u32 = 3;
pub const R_BPF_64_NODYLD32: u32 = 4;
pub const R_BPF_64_32: u32 = 10;
```

### BPF Map标志位

```rust
pub const BPF_ANY: u64 = 0;         // 任意
pub const BPF_NOEXIST: u64 = 1;     // 要求元素必须不存在
pub const BPF_EXIST: u64 = 2;       // 要求元素必须存在
pub const BPF_F_LOCK: u64 = 4;      // Unused
```

## 代码链接

[kernel/src/bpf/consts.rs](../../src/bpf/consts.rs)