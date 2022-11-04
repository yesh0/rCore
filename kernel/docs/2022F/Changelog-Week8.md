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
</style>
<style scoped>
section {
  text-align: center;
}
</style>

# 期中进展汇报

<br>

### Linux 现有的 eBPF 验证器实现
### 独立出来的 eBPF 验证器库的设想

---

## Linux 的 eBPF 验证器

### 一些非“验证”性的工作

- 每一个函数所需的栈空间计算
  <!-- 一个函数的栈的空间是 512 字节，但大部分函数都不会把栈用光，所以提供这些信息可以让 JIT 以及解释器省点内核栈 -->
- 重定位信息处理
  <!-- 之所以这些重定位由内核处理，是因为验证器需要这些重定位指令提供的类型信息 -->

### 指令块级别的验证

- 跳转合规、不含不可达代码块

### 程序流程级别的验证（主体部分）

---

## Linux eBPF 验证器主体部分

### 目标：停机问题

遍历**每一种**可能的执行路径，确认指令、数据操作、函数调用等均合法
- 不是所有合法 eBPF 程序都能够通过验证
- 有很多时候需要 eBPF 程序员做一些处理

---

### 不能通过验证的 eBPF 范例

- 内存范围相关：
```c
for (int i = 0; i < data_end - data; i++)
  if (((char *) data)[i] == -1)  // out of bound
    return XDP_PASS;

for (char *p = data; p < data_end; p++)
  if (*p == -1)                  // out of bound
    return XDP_PASS;
```
- 栈上变量操作相关：放到栈上的没有对齐的变量难验证
  （汇编代码略）

---

### 与解释器结构相似

|                | 解释器   | 验证器                   |
|----------------|----------|--------------------------|
| 寄存器         | `u64`    | `bpf_reg_state`          |
| 操作寄存器或栈 | 直接更新 | 更新值的类型、范围、位图 |
| 分支跳转       | 更改 PC  | 预判以及遍历分支         |
| 内存相关       | 直接读写 | 检验边界条件等           |
| 函数调用       | 直接传参 | 进行参数类型检验等验证   |

---

### 标量的跟踪

储存了 `i32/u32/i64/u64` 分别的最大最小值以及一个位图，在进行相关操作时会相应更新
- 省空间但不太“智能”：很多时候不能够自动从 `a < b && b < c` 推出 `a < c`
- 较为复杂，有符号需要处理，32 位需要特殊处理，甚至因此有了 [CVE-2021-3490](https://nvd.nist.gov/vuln/detail/CVE-2021-3490)
- 经历过重构，但可能原来一个大函数内的上下文依赖到了不同小函数之间的依赖，某种程度上更难把握整体代码了

---

### 分支的跟踪

- 分支遍历
- 分支合并优化

---

### 调用 BPF Helper 函数的验证

- 参数类型验证
- 函数语义验证
