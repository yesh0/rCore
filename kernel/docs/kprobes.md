# Kprobes for rCore

关于Kprobes的基础相关内容见NickCao的[介绍](https://github.com/NickCao/ebpf-rs/blob/master/docs/src/Kprobes/intro.md)与相关[slides](https://github.com/NickCao/ebpf-rs/blob/master/slides/eBPF_and_Kprobes_on_rCore.pdf)，重复内容这里不再赘述。[我们的实现](https://github.com/latte-c/rCore/tree/bpf/kernel/src/kprobes)参考了[hm1229的实现](https://github.com/hm1229/rkprobes)与[Linux内核关于kprobes的文档](https://www.kernel.org/doc/html/latest/trace/kprobes.html)，下面介绍了一些实现上的区别。注意这里的讨论主要局限于riscv64架构，相关原因可见移植事项/架构选择一节。

## 功能增强

本节介绍了我们的实现与hm1229的参考实现的区别，包括设计和实现上的细节。

### 接口差异

#### Kprobe与Kretprobe

模块划分和接口命名上更多参考了Linux。如果你希望在某个函数返回时做一些操作，请使用kretprobe而不是kprobe。Kprobe的pre handler和post handler在被替换的指令被“执行”前后调用，而kretprobe的entry handler和exit handler的调用时机分别是函数进入时和函数返回时。实际上这里的kretprobe是基于kprobe实现的，对某个函数地址注册kretprobe会自动注册一个kprobe，因此这个地址上无法再注册另外的kprobe。如果只是想在函数入口处执行一些常规代码，可以用该kretprobe的entry handler。

我们`KProbe`数据结构的设计思路是向Linux靠拢，里面加入了用户自定义字段和活跃数量。类型为`usize`的`user_data`可以向handler提供一些必要的信息，其内容由此kprobe的注册者提供。当前活跃数量则是为了防止某个执行流仍活跃时此kprobe被移除。

`KRetProbe`数据结构也有相似的设计，在`user_data`之外还有`instance_limit`、`nr_instances`和`nr_misses`等字段。由于kretprobe需要对每个函数调用的实例进行跟踪，运行过程中可能会出现实例过多而资源不足的情况，如遇到递归层数过多的函数。为了避免这种情况，使用者可以指定kretprobe的最大实例数量，超过限制的函数调用将不会被追踪，此信息也会被记录在`nr_misses`中。

#### handler的签名

目前handler的签名是`fn(&mut TrapFrame, usize) -> isize`，第一个参数为中断上下文，第二个参数为上面提到的`user_data`。事实上第二个参数应该类似`&KProbe`，handler应该能读取到此kprobe对外暴露的某些信息。由于当前没有handler用得到，所以我懒得改了，如果你愿意可以把它改掉。

现在多数handler的返回值都是0，设计上这个返回值是有用的。如果handler返回非0值，则kprobe的后续流程将被跳过，控制流可能发生改变。借助于这一特性可以实现类似热补丁的功能，即动态地将某个函数整体替换掉：在被替换的函数入口处打上kprobe，然后把新的函数实现在handler中，注意将context中的sepc修改为原先的ra以返回到原先的调用者，最后handler返回非0值，中断返回过程将控制流交回原调用者。很明显，我们暂时没有这样的实际需求，所以直接忽略了handler的返回值。

### 指令类型

[NickCao的文档](https://github.com/NickCao/ebpf-rs/blob/master/slides/eBPF_and_Kprobes_on_rCore.pdf)中提及了对需要单步执行的指令的限制。我们的实现中放宽了一些限制，加入了对PC相关指令的软件模拟。需要注意的是特权级问题在这里缺少详细的讨论，我们忽略了相关问题。实际上遇到这些问题的概率较低，kprobe主要面向内核而不涉及太多特权级的麻烦事（例如：真的会有人把kprobe打到一条`sret`指令上吗？）。如果真的出现异常，kprobe的使用者应对此负责。

具体而言，被断点指令替换的常规指令会在另一个区域被单步执行。由于PC存在差异，直接执行PC相关指令会产生错误的效果。我们选择直接用软件模拟这些指令（包括auipc、分支指令、跳转和间接跳转以及对应的压缩指令），指令的效果将反映在保存的上下文（`TrapFrame`）中。断点命中后这些指令的执行流程与常规指令略有不同，但pre handler和post handler仍然适用。

### SMP支持

所有全局性数据结构使用`Mutex`保护。关于kretprobe设计与实现上的要点可直接见[这里](https://github.com/NickCao/ebpf-rs/blob/master/docs/src/Kprobes/intro.md#run-handler-on-function-return)，实际实现基本遵循了对应方式，代码中包含SMP环境下对递归函数的kretprobe测试。

### 试验性功能

目前Rust编译器似乎不支持对函数静态插桩`mcount`调用，所以难以获得函数的动态调用信息。我们尝试了利用kprobes动态插桩获取函数的调用关系，实现中对一段指令区域中的所有`jal/jalr`指令（包含压缩变体）挂载kprobe，在post handler中记录被调用函数的地址。此操作可递归进行，重复地对未记录的函数进行动态追踪和采样。采集到充分的函数调用信息后，可利用stacktrace等手段做进一步的信息收集。

## 模块与接口

我们的kprobes模块大致由以下部分组成：

+ `arch`模块
+ `kprobes.rs`
+ `kretprobes.rs`
+ `mod.rs`

我们不希望kprobes的实现与架构有过于紧密的依赖，因此做了一些必要的抽象，将架构相关的部分放在了`arch`子模块中。`kprobes.rs`和`kretprobes.rs`主要是kprobes和kretprobes的内部实现，具体详见代码。下面重点关注`mod.rs`与`arch`模块的内容。

### `mod.rs`

#### 类型

+ `HandlerFn`：作为handler的函数

```Rust
pub type HandlerFn = fn(&mut TrapFrame, usize) -> isize;
```

+ `Handler`：handler类型

```Rust
pub type Handler = dyn Fn(&mut TrapFrame, usize) -> isize + Sync + Send;
```

+ `KProbeArgs`：注册kprobe的参数

```Rust
pub struct KProbeArgs {
    pub pre_handler: Arc<Handler>,
    pub post_handler: Option<Arc<Handler>>,
    // Extra user-defined data. Kprobes will not touch it and pass it to handler as-is.
    pub user_data: usize,
}
```

+ `KRetProbeArgs`：注册kretprobe的参数

```Rust
pub struct KRetProbeArgs {
    pub exit_handler: Arc<Handler>,
    pub entry_handler: Option<Arc<Handler>>,
    pub limit: Option<usize>,
    pub user_data: usize,
}
```

+ `SingleStepType`：指令单步执行的类型 

```Rust
#[derive(PartialEq)]
pub enum SingleStepType {
    Unsupported,
    Execute,
    Emulate,
}
```

#### 方法

+ `register_kprobe`：注册kprobe

```Rust
pub fn register_kprobe(addr: usize, args: KProbeArgs) -> Option<()>
```

`addr`为目标地址，`args`为具体参数。若操作成功返回`Some(())`，否则返回`None`。

+ `unregister_kprobe`：取消注册kprobe

```Rust
pub fn unregister_kprobe(addr: usize) -> Option<()>
```

取消注册目标地址`addr`对应的kprobe。若操作成功返回`Some(())`，否则返回`None`。

+ `register_kretprobe`：注册kretprobe

```Rust
pub fn register_kretprobe(addr: usize, args: KRetProbeArgs) -> Option<()>
```

`addr`为目标地址，`args`为具体参数。若操作成功返回`Some(())`，否则返回`None`。

+ `unregister_kretprobe`：取消注册kretprobe

```Rust
pub fn unregister_kretprobe(addr: usize) -> Option<()>
```

取消注册目标地址`addr`对应的kretprobe。若操作成功返回`Some(())`，否则返回`None`。

+ `breakpoint_handler`：kprobe/kretprobe的断点处理程序。此例程在rCore的断点处理程序中被调用。

```Rust
pub fn breakpoint_handler(tf: &mut TrapFrame)
```

### `arch`模块

#### 类型

+ `InstructionBuffer`：对trampoline代码区的封装

```Rust
pub struct InstructionBuffer {
    addr: usize,
}

impl InstrutionBuffer {
    pub fn new() -> Self { ... }
    pub fn addr(&self) -> usize { self.addr }
    pub fn copy_in(&self, offset: usize, src_addr: usize, len: usize) { ... }
    pub fn copy_out(&self, offset: usize, src_addr: usize, len: usize) { ... }
    pub fn add_breakpoint(&self, offset: usize) { ... }
}

impl Drop for InstructionBuffer {
    fn drop(&mut self) { ... }
}
```

#### 方法

+ `invalidate_icache`：无效化指令缓存

```Rust
pub fn invalidate_icache()
```

+ `get_insn_length`：获得内存中指令的长度

```Rust
pub fn get_insn_length(addr: usize) -> usize
```

+ `get_insn_type`：获得内存中指令的单步执行类型

```Rust
pub fn get_insn_type(addr: usize) -> SingleStepType
```

+ `get_trapframe_pc`：从trapframe中获取pc

```Rust
pub fn get_trapframe_pc(tf: &TrapFrame) -> usize
```

+ `set_trapframe_pc`：设置保存的trapframe中的pc

```Rust
pub fn set_trapframe_pc(tf: &mut TrapFrame, pc: usize)
```

+ `get_trapframe_ra`：从trapframe中获取ra

```Rust
pub fn get_trapframe_ra(tf: &TrapFrame) -> usize
```

+ `set_trapframe_ra`：设置保存的trapframe中的ra

```Rust
pub fn set_trapframe_ra(tf: &mut TrapFrame, ra: usize)
```

+ `emulate_execution`：在给定上下文中模拟指令的执行效果，指令的“实际”pc由参数提供

```Rust
pub fn emulate_execution(tf: &mut TrapFrame, insn_addr: usize, pc: usize)
```

+ `inject_breakpoints`：向指定内存区域写入指定数量的断点指令

```Rust
pub fn inject_breakpoints(addr: usize, length: Option<usize>)
```

+ `alloc_breakpoint`：分配一个当前唯一的断点地址

```Rust
pub fn alloc_breakpoint() -> usize
```

+ `free_breakpoint`：释放由`alloc_breakpoint`分配的断点

```Rust
pub fn free_breakpoint(addr: usize)
```

## 移植事项

### 架构选择

我们选择仍然在riscv64架构上进行。原因很简单，kprobes需要对指令进行解码，riscv64的指令集相对比较简单，依赖的解码库（如riscv-decode）也比较轻量级。如果想要移植到x86等架构，可能会引入重量级的解码模块。

目前我们使用的解码库[riscv-decode](https://github.com/latte-c/riscv-decode)是原模块的增强版本，加入了对RVC压缩指令的支持。注意部分在32位和64位环境下意义不同的RVC指令解码没有实现。

### 断点处理

Kprobes功能依赖于断点指令，因此需要与OS的断点处理程序进行交互。注意不同架构上断点指令的具体形式和长度的差异，例如在x86上可能会将一条指令替换成一条`int3`和若干个`nop`。

### 指令模拟

实现指令模拟以支持更多类型指令的单步执行是锦上添花的功能，你完全可以选择不实现指令模拟。事实上，大部分的kprobe都会被打在function prologue上，一个最小实现不需要考虑其它复杂的情况。

### 代码区分配

这里的代码区指用于存放被替换指令的一段区域，也可以叫它trampoline。在大多数情况下，kprobe命中后控制流会来到这个区域，执行被断点替换的指令，然后再回到正常的控制流中。

这段空间的分配有两个值得留意的细节：
+ 对齐：特别对于RISC系架构，应满足指令的对齐要求。
+ 可执行权限：很明显这段区域应该可执行。

考虑到以上因素，实现中简单粗暴地给代码区分配了一个4k页，这直接满足了对齐要求。在riscv64的rCore上不需要额外给内核页可执行权限，在其它架构上给4k页设置可执行权限也不是什么太困难的事情。当然，代码区里通常只有2条指令（被替换的指令和一条断点指令），空间浪费比较严重。如果你在意这个问题，可以做一些优化。

## 扩展

这里列出了一些我们没有做的功能。如果你感兴趣，可以将它们作为练习。

### Kprobe与handler返回值的交互

这部分内容已经在前面介绍过了。其原始特性来自[Linux](https://www.kernel.org/doc/html/latest/trace/kprobes.html#changing-execution-path)。

### 重构实现

当前的实现依赖了较多全局共享数据结构和锁，稍有不慎容易死锁。应当减少lazy_static全局变量的数量。另外当前实现中handler中无法对全局kprobes信息进行修改，这在设计上是合理的，但你可以考虑取消这一限制来实现某些花式操作，比如在kprobe handler中注册新的kprobe。

### Uprobe

利用kprobes来追踪用户空间程序。实现这一功能涉及到的问题比较多，比如需要解析用户程序的符号表，uprobe的注册者和目标进程可能位于不同的地址空间（目标进程可能还不存在）。你可以考虑在进程切换或exec系统调用时检查是否应进行uprobe的指令替换。
