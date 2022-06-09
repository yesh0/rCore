# BPF Helper Functions

提供了对于`bpf-helpers`的部分实现。

## 定义

### `type BpfHelperFn`

所有BPF Helper Function的类型

* 具体定义

```rust
pub type BpfHelperFn = fn(u64, u64, u64, u64, u64) -> i64;
```

### `const HELPER_FN_COUNT: usize`

BPF Helper Functions的个数

* 具体定义

```rust
pub const HELPER_FN_COUNT: usize = 17;
```

### `static HELPER_FN_TABLE: [BpfHelperFn; HELPER_FN_COUNT]`

BPF Helper Functions表，需要传递给编译好的BPF程序来寻找具体的函数。

* 具体定义

```rust
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
```

### 实现的Helper Functions

* `bpf_nop`：啥也不做（用于填补未实现函数位置）

* `bpf_map_lookup_elem`：查询BPF Map中的元素，是`bpf_map_lookup_helper`的包装
    * 参见[Map](./map.md)中关于`bpf_map_lookup_helper`的定义
    * 成功时返回对应值，失败时返回0，对应`NULL`

* `bpf_map_update_elem`：更新BPF Map中的元素，是`bpf_map_ops`的包装
    * 参见[Map](./map.md)中关于`bpf_map_ops`的定义
    * 成功时返回对应值，失败时返回-1

* 

## 代码链接

[kernel/src/bpf/helpers.rs](../../src/bpf/helpers.rs)