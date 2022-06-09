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

### 已实现的Helper Functions

* `bpf_nop`：啥也不做（用于填补未实现函数位置）

* `bpf_map_lookup_elem`：查询BPF Map中的元素，是`bpf_map_lookup_helper`的包装
    * 参见[Map](./map.md)中关于`bpf_map_lookup_helper`的定义
    * 成功时返回对应值，失败时返回0，对应`NULL`

* `bpf_map_update_elem`：更新BPF Map中的元素，是`bpf_map_ops`的包装
    * 参见[Map](./map.md)中关于`bpf_map_ops`的定义
    * 成功时返回对应值，失败时返回-1

* `bpf_map_delete_elem`：删除BPF Map中的元素，时`bpf_map_ops`的包装
    * 参见[Map](./map.md)中关于`bpf_map_ops`的定义
    * 成功时返回对应值，失败时返回-1

* `bpf_probe_read`：读取特定内核区域内的数据
    * 将指定地址的数据拷贝到准备好的缓冲区中。
    * 成功时返回0，失败时返回-1

* `bpf_ktime_get_ns`：返回`ktime`，以纳秒的形式
    * 调用`arch::timer::timer_now()`返回时间

* `bpf_trace_printk`：输出内容
    * 实现采用Rust的格式化，即`{}`标识待格式化内容，全部为`u64`

* `bpf_get_smp_processor_id`：返回当前的CPU编号
    * 调用`arch::cpu::id()`返回CPU编号

* `bpf_get_current_pid_tgid`：返回当前进程的线程ID
    * 注意在当前进程——线程模型下，pid和tgid时一样的，通过得到当前线程号（进程号）返回

* `bpf_get_current_comm`：获得当前运行进程的执行路径
    * 将字符串拷贝到指定的缓冲区，拷贝的字符串以C的标准填入，即`'\0'`截止被加到末尾。

## 代码链接

[kernel/src/bpf/helpers.rs](../../src/bpf/helpers.rs)