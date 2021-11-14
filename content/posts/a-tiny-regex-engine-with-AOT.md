---
title: "A Tiny Regex Engine With AOT"
date: 2021-11-13T14:18:49+08:00
draft: false
mathjax: true
tags: [cpp, regex, AOT]
---

The regular expression is initially developed during the process of formalizing the **regular language** in the 1950s.
Except for its application in theoretical computer science, the regular expression has also been a useful tool for searching in the strings.
There were therefore many **regular expression engines** for such a purpose, each one with different features.

## Syntax of Regular Expression
### Minimal definition
Assume that both $a$ and $b$ are regular expressions, $m$ is a single character:

- $\epsilon$: empty string

Denote to match the empty string.

- $m$: literal character

Denote to match the character itself.

- $ab$: concatenation

Denote a string can be accepted if it is the concatenation of a string accepted by $a$ and one accepted by $b$.

- $a|b$: alternative

Denote a string can be either a string accepted by $a$ or one accepted by $b$.

- $a\verb|*|$: Kleene star

Denote an accepted string can be obtained by repeating the string 0 or more times, which is accepted by $a$.

### Extension
From the point for more expressive when making a string pattern, we introduce some convenient extensions to the basic regex syntax.

Assume that $a$ is a regular expression, $m,n$ are single characters, and $l,u$ are integers:

- $a?$ 
    - equal to $\epsilon|a$.
- $a+$
    - equal to $aa*$.
- ^
    - mean to match the starting position of a string (or a line).
- \$
    - mean to match the ending position of a string (or a line).
- $.$
    - mean to match any character.
- $[mn]$
    - match $m$ or $n$. Same with $(m|n)$ but has a advantage of implementation.
- $[m-n]$
    - match a character ranging from $m$ to $n$.
- $a\\{l,u\\}$
    - mean to match specific number of repeating $a$, from $l$ to $u$.

## Implementation of Regular Expression

### Introduction
A **regex processor**, or **regex engine**, can transform a **regular expression** into one kind of representation so that it can begin to do string matching or string searching. There are so many different approaches to implementing such a processor.

The most typical and oldest method is converting the **regex** into a **DFA**. But the method exists the problem that an NFA with $n$ states will be converted into a DFA with $2^p$ states, which probably causes huge memory consumption. In practice, we may use **lazy construction** or **state cache** to assuage the drawback. Lazy construction means we don't construct all DFA states at once, instead we construct them only when we need them. **State cache** means we may drop unused states if a threshold has been reached and only keep those we used now (imagine the CPU cache). The last problem is that a DFA-based regex engine can't support the features that exceed the ability of **regular language**.

Another approach is to stay at the **NFA** stage. To simulate the **parallel** behaviors of a typical NFA, doing BFS is enough. You can also use the real parallel mechanism in a modern computer. But this method has worse theoretical time complexity than the DFA method.

However, if you use DFS to execute an NFA, what you obtain will be a so-called **Backtracking** implementation. The biggest disadvantage is that its time complexity is exponential if specific patterns like $a\verb|**|$ are encountered.  

The concrete implementations of the automaton vary. A possible way is to make a **state transition table** and move to another state that is found in the table with the given input and current state. Such a table occurs to a problem that it may be too sparse, which is not friendly to the memory usage. Another representation, a **state graph**, can be intuitively implemented with a linked list, where each state stores its possible out edges. Both approaches should be considered how is the extent they can make full use of the modern super-scalar CPU. For example, a huge state transition table may cause a CPU cache thrashing. The linked list method with too much pointer access may be inefficient with the same cache problem. And the latter way may also challenge the ability of the CPU to predict the branch.

An interesting idea is to compile the automaton into a virtual machine before executing it. The way has a benefit that it provides extra abstraction. And newly introduced features can be added more neatly (might through new opcodes). Though a new level of abstraction could cause extra cost in the execution time, many ad-hoc optimization techniques in usual compilers help. A great advantage comes that a virtual machine can be easily dynamically compiled into the native code, which performs well in many environments.

To avoid the possible attack of regular expression denial of service, most of the libraries use hybrid algorithms/implementations. If one algorithm runs for a too long time, a backup algorithm would like to be chosen.

### In the real world
- Go / Re2
    - DFA engine (inspire many implementations)
- Rust regex crate:
    - DFA engine
    - NFA engine with bounded backtracking
    - NFA engine, Pike VM (fallback)
- C PCRE 2:
    - NFA engine, Henry Spencer VM, JIT compilation support
    - DFA engine ?
- Perl:
    - NFA engine with backtracking
- C++ libstdc++
    - NFA engine with backtracking
    - NFA engine, Thompson VM (fallback)
- C++ boost
    - NFA engine with backtracking
- Javascript Google V8
    - (Before 2017: ignored)
    - (Since 2017) NFA engine, Henry Spencer VM, with always JIT compilation.
    - (Since 2021) No-JIT interpreter
    - (Since 2021) DFA engine

## Make a Toy Regex Engine

Now in this chapter let's consider making a toy regex engine. Compared with usual toy compilers, a regex engine is simpler for a beginner to handle its code.

For simplicity, we only support the basic regex syntax as well as those extensions:
- $a?$
- $a+$
- $.$
- $[mn]$
- $[m-n]$

And we use the easiest backtracking method.

### Conversion from Regular Expression to NFA
We use Thompson's construction method to transform a regex into an NFA.
Assume that both $s$ and $t$ are regular expressions, $a$ is a single character:

What is the NFA of $r=a$?

<img src="https://upload.wikimedia.org/wikipedia/commons/9/93/Thompson-a-symbol.svg" style="zoom:70%;" />

What is the NFA of $r=st$?

<img src="https://upload.wikimedia.org/wikipedia/commons/5/55/Thompson-concat.svg" style="zoom:70%;" />

What is the NFA of $r=s|t$?

<img src="https://upload.wikimedia.org/wikipedia/commons/2/25/Thompson-or.svg" style="zoom:70%;" />

What is the NFA of $r=s*$?

<img src="https://upload.wikimedia.org/wikipedia/commons/8/8e/Thompson-kleene-star.svg" style="zoom:70%;" />

### Conversion from NFA to VM instructions

Assume $Gen\ s$ means a sequence of VM instructions generated for regex $s$, $a,m,n$ are single characters.

###### For a single character $a$:
```c
//  VM pseudo-code
    single   a
```
If we have found $a$ in the current position of the string, continue. Else jump to the failed state.

###### For the concatenation of $s,t$

```c
//  VM pseudo-code
    Gen     s
    Gen     t
```

###### For the alternatives of $s,t$
```c
//  VM pseudo-code
    split   L1 L2
L1:
    Gen     s
    jump    L3
L2:
    Gen     t
L3:
```

Split means we will push one label and the current position of the string into the stack, then jump to another label.
If a thread failed, we pop one label and a position from the stack to continue. If nothing can be popped, the process fails.

###### For the Kleene star of $s$
```c
//  VM pseudo-code
    split   L1 L2
L1:
    Gen     s
    split   L1 L2
L2:
```

###### For $s?$, which is equal to $\epsilon|s$
```c
//  VM pseudo-code
    split   L1 L2
L1:
    Gen     s
L2:
```

###### For $s+$, which is equal to $ss\verb|*|$
```c
//  VM pseudo-code
L1:
    Gen     s
    split   L1 L2
L2:
```

In fact this is an optimization to the following code:

```c
//  VM pseudo-code
    Gen     s
    split   L1 L2
L1:
    Gen     s
    split   L1 L2
L2:
```

###### Some other structural

- $.$
```c
//  VM pseudo-code
    any
```

If we have not reached the end of the string, continue. Else fails.

- $[m-n]$
```c
//  VM pseudo-code
    charset m n
```

If we meet characters ranging from $m$ to $n$ in the current position of the string, continue. Else fails.

### Example: a regex to VM instructions

How $\verb%[0-9a-zA-Z_][1-5]*.+|(123)?%$ is compiled into VM instructions?

```c
//  VM pseudo-code
    split   L4 L5
L4:
    charset 0-9A-Z_a-z
    split   L0 L1
L0:
    charset 1-5
    split   L0 L1
L1:
L2:
    any
    split   L2 L3
L3:
    jump    L6
L5:
    split   L7 L8
L7:
    single  1
    single  2
    single  3
L8:
L6:
    accept
```

### Ahead of Time Compilation to native code (x86-64)

The regex engines I mentioned above may have a feature, **JIT** compilation, or Just-in-time compilation.
This means those engines may choose to compile regex dynamically to the native code when necessary. If the engines decide to compile to the native code each time and only execute the native code, we call it **AOT** compilation or Ahead-of-time compilation.

We always compile to native code, so we call it AOT compilation.

For simplicity, we only make a matcher rather than a searcher. The matcher has a function prototype of:
```c++
using match_prototype = bool (*)(const char *);
// may looks like
bool match(const char* input);
```
If a given input is accepted by the regex, we call it is **matched**.
And we need a function to compile our regex to obtain such a function pointer for matching.

The translation from VM instructions to native code is very direct. More details about the following code may be found through searching **template interpreter**.

Notice the System V ABI requires the first argument to be placed in the `rdi` register. The return value should be placed into `rax` register. 

##### any
```c
cmp byte ptr[rdi], 0
jz  L_thread_fail
inc rdi
```

##### accept
```c
mov eax, 1
jmp L_match_return
```

##### jump n
```c
jmp L_n
```

##### single c
```c
cmp byte ptr[rdi], c
jne L_thread_fail
inc rdi
```

##### split L1 L2
```c
mov  rax, L2
push rax        // save next thread PC
push rdi        // save next match position
jmp L1
```

#### charset c1 c2
```c
xor eax, eax
movzx edx, byte ptr[rdi]
cmp dl, 0
jz L_thread_fail
lea ecx, dword ptr[rdx - c1]
cmp cl, (c2-c1)
setbe cl
or al, cl
jz L_thread_fail
inc rdi
```

Additionally, we need some code snippets for function entry and exit. They have appeared above as the jump targets.

#### L_entry
```c
push  rbp
mov   rbp, rsp
movzx eax, byte ptr[rdi]
cmp   al, 0
jnz   L_run
```

#### L_match_fail
```c
xor   eax, eax
```

#### L_match_return
```c
mov   rsp, rbp
pop   rbp
ret
```

#### L_thread_fail
```c
cmp rsp, rbp
je L_match_fail
pop rdi
pop r9
jmp r9
```

We can copy and paste the binary of those snippets when we do linear iterations over the VM instructions, then fix the offset of the labels.
To ease the process, we use **xbyak**, a good assistant library. It enacts like a runtime assembler, helping to create machine code, setting execution flags for virtual memory, etc.

Now our code can generate the assembly for regex $\verb%[0-9a-zA-Z_][1-5]*.+|(123)?%$

```c
00007F2AF0F0B000  push rbp
00007F2AF0F0B001  mov rbp, rsp
00007F2AF0F0B004  movzx eax, byte ptr [rdi]
00007F2AF0F0B007  cmp al, 0x00
00007F2AF0F0B009  jnz 0x00007F2AF0F0B021
00007F2AF0F0B00F  xor eax, eax
00007F2AF0F0B011  mov rsp, rbp
00007F2AF0F0B014  pop rbp
00007F2AF0F0B015  ret
00007F2AF0F0B016  cmp rsp, rbp
00007F2AF0F0B019  jz 0x00007F2AF0F0B00F
00007F2AF0F0B01B  pop rdi
00007F2AF0F0B01C  pop r9
00007F2AF0F0B01E  jmp r9
00007F2AF0F0B021  mov rax, 0x7F2AF0F0B0B8
00007F2AF0F0B02B  push rax
00007F2AF0F0B02C  push rdi
00007F2AF0F0B02D  xor eax, eax
00007F2AF0F0B02F  movzx edx, byte ptr [rdi]
00007F2AF0F0B032  cmp dl, 0x00
00007F2AF0F0B035  jz 0x00007F2AF0F0B016
00007F2AF0F0B037  lea ecx, [rdx-0x30]
00007F2AF0F0B03A  cmp cl, 0x09
00007F2AF0F0B03D  setbe cl
00007F2AF0F0B040  or al, cl
00007F2AF0F0B042  lea ecx, [rdx-0x41]
00007F2AF0F0B045  cmp cl, 0x19
00007F2AF0F0B048  setbe cl
00007F2AF0F0B04B  or al, cl
00007F2AF0F0B04D  cmp dl, 0x5F
00007F2AF0F0B050  setz cl
00007F2AF0F0B053  or al, cl
00007F2AF0F0B055  lea ecx, [rdx-0x61]
00007F2AF0F0B058  cmp cl, 0x19
00007F2AF0F0B05B  setbe cl
00007F2AF0F0B05E  or al, cl
00007F2AF0F0B060  jz 0x00007F2AF0F0B016
00007F2AF0F0B062  inc rdi
00007F2AF0F0B065  mov rax, 0x7F2AF0F0B099
00007F2AF0F0B06F  push rax
00007F2AF0F0B070  push rdi
00007F2AF0F0B071  xor eax, eax
00007F2AF0F0B073  movzx edx, byte ptr [rdi]
00007F2AF0F0B076  cmp dl, 0x00
00007F2AF0F0B079  jz 0x00007F2AF0F0B016
00007F2AF0F0B07B  lea ecx, [rdx-0x31]
00007F2AF0F0B07E  cmp cl, 0x04
00007F2AF0F0B081  setbe cl
00007F2AF0F0B084  or al, cl
00007F2AF0F0B086  jz 0x00007F2AF0F0B016
00007F2AF0F0B088  inc rdi
00007F2AF0F0B08B  mov rax, 0x7F2AF0F0B099
00007F2AF0F0B095  push rax
00007F2AF0F0B096  push rdi
00007F2AF0F0B097  jmp 0x00007F2AF0F0B071
00007F2AF0F0B099  cmp byte ptr [rdi], 0x00
00007F2AF0F0B09C  jz 0x00007F2AF0F0B016
00007F2AF0F0B0A2  inc rdi
00007F2AF0F0B0A5  mov rax, 0x7F2AF0F0B0B3
00007F2AF0F0B0AF  push rax
00007F2AF0F0B0B0  push rdi
00007F2AF0F0B0B1  jmp 0x00007F2AF0F0B099
00007F2AF0F0B0B3  jmp 0x00007F2AF0F0B0DF
00007F2AF0F0B0B8  mov rax, 0x7F2AF0F0B0DF
00007F2AF0F0B0C2  push rax
00007F2AF0F0B0C3  push rdi
00007F2AF0F0B0C4  cmp word ptr [rdi], 0x3231
00007F2AF0F0B0C9  jnz 0x00007F2AF0F0B016
00007F2AF0F0B0CF  add rdi, 0x02
00007F2AF0F0B0D3  cmp byte ptr [rdi], 0x33
00007F2AF0F0B0D6  jnz 0x00007F2AF0F0B016
00007F2AF0F0B0DC  inc rdi
00007F2AF0F0B0DF  mov eax, 0x01
00007F2AF0F0B0E4  jmp 0x00007F2AF0F0B011
```