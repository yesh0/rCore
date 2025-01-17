# rCore Tracing via eBPF 项目文档

该项目为rCore在RISC-V架构下的build引入了一系列内核追踪的基础设施和能力。这包括了：`kprobes`设施，`bpf`模块以及相应的样例。本文档包含了`kprobes`模块，`bpf`设施中和操作系统相关的运行时模块和样例的实现说明。

## 目录

* `kprobes`内核追踪模块
    * [`kprobe\kretprobe`相关说明](./kprobes.md)

* `bpf`模块
    * [BPF模块](./bpf/bpf.md)
    * [ebpf2rv编译器](https://github.com/latte-c/ebpf2rv)

* rCore基础设施
    * [基础设施增强](./infrastructures.md)

* TODO
    * [TODO事项]

* 用户态程序（样例）
    * 本部分程序提供在`ucore-user`仓库中，相关文档请参照该仓库。
