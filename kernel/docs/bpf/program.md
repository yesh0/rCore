# BPF程序模块

本模块提供了对于BPF程序的加载和运行支持，也提供了对于通过`clang`编译得到的ELF BPF程序的加载和重定向功能。

## 定义

### `struct MapFdEntry`

结构体包装了一个BPF Map条目。这个条目将会注入到BPF程序中。最主要的是记录了对应Map的fd。

* 具体定义

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MapFdEntry {
    pub name: *const u8,
    pub fd: u32,
}
```

### `struct ProgramLoadExAttr`

结构体包装了`load_ex`的相关描述符，也就是加载elf BPF程序的相关属性，包括了elf程序指针，elf程序大小，需要注入的Map条目们。

* 具体定义

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProgramLoadExAttr {
    pub elf_prog: u64,
    pub elf_size: u32,
    pub map_array_len: u32,
    pub map_array: *const MapFdEntry,
}
```

### `struct BpfProgram`

结构体包装了一个真正的BPF程序，包括了BPF字节码，Jit编译后得到的机器码以及需要注入的Map的fd。

* 具体定义

```rust
pub struct BpfProgram {
    bpf_insns: Option<Vec<u64>>,
    jited_prog: Option<Vec<u32>>, // TODO: should be something like Vec<u8>
    map_fd_table: Option<Vec<u32>>,
}
```

* 相关方法
    * `run(&self) -> i64`
        * 运行一个BPF程序（当且仅当有Jit编译的机器码的时候），如果没有的话解释器是缺失的。

### `fn bpf_program_load_ex(prog: &mut [u8], map_info: &[(String, u32)]) -> SysResult`

函数提供了load ex功能，该功能加载一个`clang`编译得到的elf BPF程序并对其中的`extern`变量进行重定向。

* 加载过程

该函数首先会检查ELF Header中的machine，如果不是BPF的话返回错误。

接下来根据提供的需要注入的BPF Map的相关信息（`map_info: &[(String, u32)]`）进行BPF Map的构建工作。接下来在`.symtab`段中寻找需要重定向的符号，如果找到和提供的`map_info`中BPF Map名称相同的符号，则定为对应Map的fd值。这些信息会加入到一个从变量地址到fd的对应表中。

接下来需要做的就是遍历BPF程序并进行重定向。所有的在`.symtab`中的符号需要按照现在的对应表进行重定向。

接下来进行Jit编译，将字节码编译成RISC-V机器码并保存。最终会通过`bpf_allocate_fd`为新的程序创建一个fd并通过`bpf_object_create_program`创建新的BPF程序。一切成功将会返回创建的fd。

## 代码链接

[kernel/src/bpf/program.rs](../../src/bpf/program.rs)