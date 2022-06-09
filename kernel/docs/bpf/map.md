# BPF Map模块

本模块实现了一系列与BPF Map相关的内容，包括两种特定类型Map：哈希表和线性表的定义与实现，以及相关的CRUD操作。最终还提供了针对Map的管理和Helper Functions中针对Map操作的辅助函数。

## 定义

### `struct MapAttr`

结构体提供了针对Map的一系列属性的包装，包括了Map的类型（线性表、哈希表），键的大小，值的大小和最多条目数。

* 具体定义

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MapAttr {
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
}
```

### `struct InternalMapAttr`

同样标识了Map的一系列属性，但是用于内部实现表示。

* 具体定义

```rust
#[derive(Debug, Clone, Copy)]
pub struct InternalMapAttr {
    pub key_size: usize,
    pub value_size: usize,
    pub max_entries: usize,
}
```

* 相关方法
    * `impl From<MapAttr>`
        * 提供了从`MapAttr`转换到`InternalMapAttr`的转换过程（对应字段转换）

### `struct MapOpAttr`

结构体提供了针对Map的特定操作的描述符，包括了操作Map的fd，操作中用到的key，操作中可能用到的value以及根据标准定义的标志位。

* 具体定义

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MapOpAttr {
    pub map_fd: u32,
    pub key: u64,
    pub value: u64,
    pub flags: u64,
}
```

### `trait BpfMap`

该特性标识了实现一个真实的BPF Map需要提供的一系列函数。

* 具体定义

```rust
pub trait BpfMap {
    fn lookup(&self, key: *const u8, value: *mut u8) -> SysResult;
    fn update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult;
    fn delete(&mut self, key: *const u8) -> SysResult;
    fn next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult;
    fn get_attr(&self) -> InternalMapAttr;

    // this lookup is intended for the helper function
    fn lookup_helper(&self, key: *const u8) -> SysResult;
}
```

* 相关方法
    * `lookup(&self, key: *const u8, value: *mut u8) -> SysResult`
        * 通过给定的键查找一个值，查找到的值通过拷贝到给定的缓冲区实现。如果成功返回`Ok(0)`，失败根据情况返回`SysError`，具体根据标志位的行为请参见`bpf-helpers`标准规定。
    * `update(&mut self, key: *const u8, value: *const u8, flags: u64) -> SysResult`
        * 通过给定的键和标识位更新一个值，值同样通过指针提供。如果成功返回`Ok(0)`，失败根据情况返回`SysError`，具体根据标志位的行为请参见`bpf-helpers`标准规定。
    * `delete(&mut self, key: *const u8) -> SysResult`
        * 通过给定的键删除一个值，如果成功返回`Ok(0)`，失败情况返回`SysError`，具体根据标志位的行为请参见`bpf-helpers`标准规定。
    * `next_key(&self, key: *const u8, next_key: *mut u8) -> SysResult`
        * 通过给定的键返回迭代的下一个键，下一个键通过`next_key`拷贝出去，如果成功返回`Ok(0)`，如果失败返回`SysResult`，具体返回结果参见`bpf-helpers`标准规定。
    * `get_attr(&self) -> InternalMapAttr`
        * 返回自己的属性
    * `lookup_helper(&self, key: *const u8) -> SysResult`
        * 为`bpf_lookup_elem_helper`提供辅助的成员函数，返回成员地址。

### `type HashCode = u32`

### `type MapKey = Box<[u8]>`

### `type MapValue = Box<[u8]>`

### `struct ArrayMap`

线性表BPF Map的具体实现，即一系列的键值对。键就是下标。

* 具体定义

```rust
struct ArrayMap {
    attr: InternalMapAttr,
    storage: Vec<u8>,
}
```

* 相关方法
    * `impl BpfMap for ArrayMap`
        * 对`BpfMap`特性的相关实现
        * 实现的方法很简单，线性表的键就是在`Vec<u8>`中的下标，所以所有的CRUD操作都比较直觉，在对应下标进行操作即可。`get_element_addr`成员提供了获得对应下标的地址，用于和C语言进行指针的交互。
    * `new(attr: InternalMapAttr) -> Self`
        * 创建一个线性BPF Map
    * `get_element_addr(&self, index: usize) -> usize`
        * 通过给定的下标返回对应的地址

### `struct HashMap`

哈希表BPF Map的具体实现，内部采用`BTreeMap`来管理哈希值和键值对的对应，采用开散列的方法进行键值对的管理。哈希方法采用针对键的BKDR哈希，种子为131313。

* 具体定义

```rust
struct HashMap {
    attr: InternalMapAttr,
    map: BTreeMap<HashCode, Vec<(MapKey, MapValue)>>,
    total_elems: usize, // total number of elements
}
```

* 相关方法
    * `impl BpfMap for HashMap`
        * 对`BpfMap`特性的相关实现
        * 在进行CRUD操作的时候，先通过`hash`方法得到键的哈希值，然后取得`map`中的一系列键值对，并逐个进行键的比较，最终得到值的位置。
    * `new(attr: InternalMapAttr) -> Self`
        * 创建一个哈希BPF Map
    * `hash(kptr: *const u8, ksize: usize) -> HashCode`
        * 针对键的BKDR哈希
    * `find(&self, kptr: *const u8) -> Option<&MapValue>`
        * 完成在给定键的情况下对值的查找。查找过程按照上述过程，如果成功返回对应键的引用，如果失败返回None。
    * `alloc(size: usize) -> Box<[u8]>`
        * 分配一个给定长度的`u8`数组，用于新建的时候的内存分配

### `type SharedBpfMap = Arc<Mutex<dyn BpfMap + Send + Sync>>`

### `fn bpf_map_create(attr: MapAttr) -> SysResult`

公开接口，用于给定属性的情况下创建BPF Map，如果成功返回fd，如果失败返回`SysError::EINVAL`。提供的属性中可以是`BPF_MAP_TYPE_ARRAY`或`BPF_MAP_TYPE_HASH`，这会创建对应类型的BPF Map，其他类型的BPF Map没有实现。

### `fn bpf_map_close(fd: u32) -> SysResult`

公开接口，用于删除整个BPF Map

### `fn bpf_map_get_attr(fd: u32) -> Option<InternalMapAttr>`

公开接口，用于获得一个BPF Map的属性，目前一定会返回`Some`。

### `fn bpf_map_ops(fd: u32, op: usize, key: *const u8, value: *mut u8, flags: u64) -> SysResult`

公开接口，是针对BPF Map操作的统一接口。其中`op`可以是`BPF_MAP_LOOKUP_ELEM`、`BPF_MAP_UPDATE_ELEM`、`BPF_MAP_DELETE_ELEM`和`BPF_MAP_GET_NEXT_KEY`，会按照`op`调用Map的对应CRUD函数并传递`SysResult`结果。

### `fn bpf_map_lookup_helper(fd: u32, key: *const u8) -> SysResult`

公开接口，为了和`bpf-helpers`中的标准相对应，调用BPF Map的`lookup_helper`函数，直接返回对应值的地址位置。

## 代码链接

[kernel/src/bpf/map.rs](../../src/bpf/map.rs)