# BPF模块

本模块为rCore提供了BPF的相关运行时设施。针对BPF程序的即时编译器请参考`ebpf2rv`仓库。本模块提供的主要是`bpf(2)`和`bpf-helpers`，以及针对Maps的辅助设施。

## 目录

* [常量](./consts.md)
    * 提供一系列常量定义

* [Helper Functions](./helpers.md)
    * 提供`bpf-helpers`的大部分实现

* [Map](./map.md)
    * 提供不同BPF Maps的具体实现和全局管理设施

* [BPF程序](./program.md)
    * 提供对BPF程序的封装

* [内核跟踪](./tracepoints.md)
    * 提供了和`kprobe`之间的回调注册和交互

## 定义

### `enum BpfObject`

该类型标识了一个BPF对象，其可以是一个Map，也可以对应于一个BPF程序。

* 具体定义

```rust
pub enum BpfObject {
    Map(SharedBpfMap),
    Program(Arc<BpfProgram>),
}
```

* 相关方法
    * `is_map -> Option<&SharedBpfMap>`
        * 判断一个BPF对象是否是Map，如果是的话返回Map的引用。
    * `is_program -> Option<&Arc<BpfProgram>>`
        * 判断一个BPF对象是否是BPF程序，如果是的话返回程序的引用。

### `static ref BPF_OBJECTS: Mutex<BTreeMap<u32, BpfObject>>`

全局唯一的记录所有BPF对象和其fd对应绑定的记录表。

### `fn bpf_allocate_fd() -> u32`

为BPF对象分配一个独一无二的fd

### `fn bpf_object_create(fd: u32, obj: BpfObject)`

记录一个创建的BPF对象，将分配的fd和对象本身进行绑定。

### `fn bpf_object_create_map(fd: u32, map: SharedBpfMap)`

对`fn bpf_object_create(fd: u32, obj: BpfObject)`的简单封装，帮你构造一个`BpfObject::Map`并调用上述函数。

### `fn bpf_object_create_program(fd: u32, prog: BpfProgram)`

对`fn bpf_object_create(fd: u32, obj: BpfObject)`的简单封装，帮你构造一个`BpfObject::Program`并调用上述函数。

### `fn bpf_object_remove(fd: u32) -> Option<BpfObject>`

尝试根据给定的fd从全局的记录表中移除对应的BPF对象，如果移除**成功**的话将会直接返回被移除对象（移交所有权）。如果**失败**的话不会报错并返回None，针对全局记录表没有任何变化。

## 代码链接

[kernel/src/bpf/mod.rs](../../src/bpf/mod.rs)

## `bpf(2)`

该部分记录了bpf系统调用的相关说明。

### `fn sys_bpf(&self, cmd: usize, attr_ptr: usize, _size: usize) -> SysResult`

其中`cmd`对应于BPF系统调用的子命令，`attr_ptr`用于提供子命令的相关属性。接下来针对不同子命令进行相关说明。

* `BPF_MAP_CREATE`
    * 创建一个BPF Map，提供的属性为`MapAttr`，通过调用`bpf_map_create`实现。
* `BPF_MAP_****_ELEM`
    * 对BPF Map中的元素进行操作。具体可以是`LOOKUP`，`UPDATE`，`DELETE`或者`GET_NEXT_KEY`。通过调用自身的`handle_map_ops`实现，最终转发给`bpf_map_ops`。
* `BPF_PROG_ATTACH`
    * 将BPF程序挂载到内核追踪点上。提供的属性为`AttachTarget`结构体，通过调用`bpf_program_attach`实现。
* `BPF_PROG_LOAD_EX`
    * 加载ELF BPF程序，提供的属性为`ProgramLoadExAttr`，通过调用`bpf_program_load_ex`实现。
* 其他：返回`SysError:EINVAL`。