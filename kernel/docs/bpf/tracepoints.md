# BPF内核追踪交互模块

本模块为BPF模块提供了和`kprobe`，`kretprobe`进行交互的功能，能够将BPF程序挂载到目标的程序上。

## 定义

### `struct AttachTarget`

该结构体描述了一个挂载点，记录包括了挂载点目标（地址）和挂载的BPF程序的fd。

* 具体定义

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AttachTarget {
    pub target: *const u8,
    pub prog_fd: u32,
}
```

该部分和`bpf(2)`系统调用进行交互。

### `enum TracepointType`

该枚举描述了跟踪点的类型，是挂载在`kprobe`、`kretprobe`（入口）还是`kretprobe`（出口）的。

* 具体定义

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TracepointType {
    KProbe,
    KRetProbeEntry,
    KRetProbeExit,
}
```

### `struct Tracepoint`

该结构体描述了一个内核跟踪点。记录了包括跟踪点的类型和一个`token`记录id。

* 具体定义

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tracepoint {
    pub tp_type: TracepointType,
    pub token: usize,
}
```

* 相关方法
    * `new(tp_type: TracepointType, token: usize) -> Self`
        * 创建一个新的`Tracepoint`（仅构造）

### `static ref ATTACHED_PROGS: Mutex<BTreeMap<Tracepoint, Vec<Arc<BpfProgram>>>>`

全局独一的记录所有挂载到追踪点的BPF程序的记录表。

### `fn bpf_program_attach(target: &str, prog_fd: u32) -> SysResult`

该函数提供了向一个追踪点（`target`）挂载一个BPF程序的功能。首先函数会解析追踪点。其标识符应该满足：`{tracepoint-type}:{function-name}`，其中`{tracepoint-type}`可以为：`kprobe`，`kretprobe@entry`或`kretprobe@exit`。而`{function-name}`则是追踪函数的名称。在Rust中需要提供完整的标识符。

接下来会通过已经编写好的handler完成追踪点的注册。在追踪点被激发的时候将会调用其中挂载的BPF程序并将`trapframe`作为context参数传入BPF程序。

## 代码链接

[kernel/src/bpf/tracepoints.rs](../../src/bpf/tracepoints.rs)