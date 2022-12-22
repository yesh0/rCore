---
marp: true
theme: uncover
_class: lead
paginate: true
backgroundColor: white
backgroundImage: url('https://marp.app/assets/hero-background.svg')
---

<style>
section {
  text-align: left;
}

ul {
  margin-left: 1em;
}

section::after {
  font-size: 0.8em;
}

table {
  width: 100%;
}
</style>
<style scoped>
section {
  text-align: center;
}
</style>

# eBPF 验证器

<br>

**ebpf-analyzer**
[github.com/yesh0/ebpf-analyzer](https://github.com/yesh0/ebpf-analyzer)

**ebpf-hitchhiking**
[yesh0.github.io/ebpf-analyzer](https://yesh0.github.io/ebpf-analyzer)

<!-- 大家好，我做的主要是 eBPF 验证器相关的内容。但是在介绍我这部分内容之前，我们先来看看这方面目前的现状。 -->

---

### eBPF 相关项目介绍

<!--
eBPF 大概不用多加介绍了。eBPF 源与 Linux 内核，可以在操作系统内核中运行用户提供的经沙盒保护的程序。它可以在不改动内核或是内核模块的情况下安全地扩展内核的功能。
在 Linux 相关领域里它的应用当然很多啦。但是 Linux 生态里生态外其实也都有一大堆相关项目，这里进行一些介绍。

第一个要提的是 eBPF for Windows。它是在 Windows 上建立 eBPF 程序的运行环境的一个尝试。
它用到了很多下面列举了的项目，比如 PREVAIL 和 ubpf，最终实现了 eBPF 程序从验证到 JIT 到在内核里面运行的这样一个完整的流程。


-->

| [ebpf-for-windows](https://github.com/microsoft/ebpf-for-windows) | [PREVAIL](https://github.com/vbpf/ebpf-verifier) |
|:-----------------------------------------------------------------:|:------------------------------------------------:|
| Windows 下的 eBPF 实现                                            | C++ 实现的 eBPF 验证器                           |

| [ubpf](https://github.com/iovisor/ubpf) | [rbpf](https://github.com/qmonnet/rbpf) |
|:---------------------------------------:|:---------------------------------------:|
| C 实现的用户态 eBPF                     | Rust 实现的用户态 eBPF                  |

| [libbpf](https://github.com/libbpf/libbpf) | [generic-ebpf](https://github.com/generic-ebpf/generic-ebpf) |
|:------------------------------------------:|:------------------------------------------------------------:|
| eBPF 加载库                                | FreeBSD eBPF 尝试                                            |

<style scoped>
td, th {
  width: 50%;
}
</style>

---

### 项目目标

1. 文档

<!--
eBPF 基金会有在进行 eBPF 的标准化工作，我这边的文档也记录了一些现有内核文档和实际行为的不符之处。
但是那边的工作看起来全面很多，总之把唯一一个看起来不太对劲的地方反馈过去之后可能考虑把我这方面的文档删掉，然后把链接指向那边。

另外的文档就是对 Linux 现有验证器的源码阅读笔记。

源码文档的话 `#[deny(missing_docs)]` 硬性要求所有公开的 API 都有文档，这方面应该还可以。
-->

2. 项目

<!--
实现了与 zCore 的整合，未考虑 map，可以在 20 行内实现程序的验证。
-->

---

### 谢谢大家！
