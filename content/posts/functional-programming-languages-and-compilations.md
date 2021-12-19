---
title: "An Overview of Functional Programming Languages and Compilations"
date: 2021-11-17T22:53:08+08:00
draft: false
mathjax: true
tags: [functional programming,compilation,type theory]
---

# 函数式编程语言及其编译：概述

## 介绍

函数式编程（下称 FP）脱胎于 Lambda Calculus（下称 LC），和其他不同的编程范式相比，它的最典型特征是高阶函数的应用。除此之外，一部分 FP 语言拥有一个更加复杂而强大的类型系统，并为此需要依赖与之匹配的类型推断、检查能力。相比于命令式的编程语言千篇一律的按值调用，FP 语言在求值策略的选择上更加灵活（尽管对于某种特定的 FP 语言，求值策略很有可能是一致的）而复杂。由于和 LC 的历史渊源，FP 语言对 “重复” 这一概念的实现，常常青睐于递归，而不是基于改变归纳变量的状态的 “循环”、“迭代”。与非 FP 语言相比，FP 语言往往强调较少的副作用乃至避免副作用。此外，还有自动柯里化、模式匹配等等特性。这些 FP 的特征，极大地改变了 FP 语言的编译实现。某些特性或功能，诸如高阶函数、递归、Continuation、自动柯里化等，如果编译器不能高效地实现，那么会导致 FP 语言实现的程序的运行效率的极度下降。而类型系统的实现，还兼具了理论上的一些困难抉择。

很显然的是，那些观感上较为纯粹的函数式编程语言（比如 ML 系编程语言，Haskell），和具有一些函数式语言特性的语言（比如 Python，JavaScript，C++2a，Rust）的编译过程并不是特别一致 —— 后者会更传统一点。所以，本文会偏向描述和前者有关的编译过程相关的内容。

## 中间表示

在进行编译时，编译器往往依赖一种或多种，对文本形式源代码中所指代的语言结构的表示方法，并在此基础上对这些结构进行变换或者将其映射到更为基础的结构。使用不同 IR 的编译器通常在编译的流程上存在着明显差异。下列列举的 IR 种类并不相互排斥：这意味着它们可能被组合在一起使用。

- $\lambda$ 演算
在 FP 语言中，最为常见的表示方法很显然是 LC 自身（以及它的变体）。这一做法的好处是十分显然的：FP 语言的特性和 LC 息息相关，这种特性所对应理论的形式化的描述也很可能是由某种拓展后的 LC 写成的，因此可以轻松地将源语言中的结构映射到更熟悉的结构上去，进而应用更通用的变换、优化手段。典型地来说，OCaml 编译器的 Clambda 和优化器所用的 Flambda 就使用了此类表示。

接下来所要介绍的中间表示形式大多是或等价于某种具有特定形式的 LC：

- CPS 形式的 IR
CPS, Continuation-Passing-Style，和直接风格相对应，在这种 “风格” 的表示中，函数、过程的 Continuation 需要被显式传递。通过 CPS 变换，我们可以将非 CPS 的表示转换为一种 CPS 表示。通过这种转换，程序的控制流以 Continuation 的形式暴露在外，从而更加便于编译器通过一系列普通的 $\beta$-规约 与 $\eta$-变换 优化程序结构 [^1]。尽管如此，原始的 CPS 形式的表示也存在很多问题，比如原始程序的 CPS 表示会十分复杂而冗长；难以对普通函数的进行统一的表示；难以优化不发生逃逸的跳转；实际实现中，过程记录常被放置到堆内存上，难以充分利用现代硬件的堆栈 保存过程调用的上下文等问题 [^2]。为了解决这些问题，发展出了很多 CPS 的变体。如 Kennedy 提出的 2nd-class Continuation 不再将 Continuation 视为普通的高阶函数 [^3]，从而允许将 Continuation 直接编译为普通的跳转。SML/NJ 使用了 CPS 形式的 IR [^1]。

还有一些 IR 形式，例如：

- ANF 形式的 IR
这种 IR 的灵感自 CPS 表示 [^4]，但要求函数参数必须是平凡的，非平凡表达式的求值必须由 $let$ 进行绑定。这一形式具有结构上更简单直观（因为仍然使用 Direct Style 而非 CPS，但和 CPS 能力相同）、易于进行机器代码生成的优点。但是存在规约后不再封闭的问题（即 ANF 项的 $\beta$-规约的结果可能不是一个 ANF 项）[^2]。近年来，SPJ 使用 join point 拓展了 ANF，并成功在 GHC 中实现了这一形式的中间表示 [^9]。

- SSA 形式的 IR
这是一种在传统语言编译器中十分常见也最为著名的 IR 的形式，但是在 FP 语言编译器中并不常见。通常情况下，我们所见到的 SSA 形式的 IR，为不同基本块组成的连通图，其中基本块中主要包含模仿机器指令集的四元式 [^5]。尽管看起来与 CPS 差异巨大，仍然可以证明了 SSA 是 CPS 的一个子集 [^10] [^6]。也有 FP 语言的编译器使用这种表示（和通常见到的 SSA IR 不完全相同），如 MLton 优化编译器的 SSA 和 SSA2 [^7]。

- 图式的 IR
这是一类范围较广的 IR，如 CFG，PDG 等，也可 SSA 形式的 IR 搭配使用。此处由于和本文关系不大，故不做过多赘述。

- C--/Cmm
这是一种具体、贴近底层的 IR，常用于向本机代码转换的最后阶段。很多 FP 语言的本机编译器（而不是虚拟机/字节码解释器）使用它。如 OCaml 本机编译器和 GHC 都使用了这一 IR。

## 类型检查与推导
一个编程语言的类型系统，将该编程语言的所能表达的项、数值、表达式等归类为不同的类型，并赋予这些类型特定的性质和作用规则。类型检查检验程序的项是否符合该编程语言的类型系统的约束。我们将在需要在程序执行期间进行等价性比较的类型检查称为动态类型检查，不需要的则称之为静态类型检查。对于那些进行动态类型检查的 FP 语言如 Scheme 来说，编译器不需要在编译时承担类型检查和推导的工作。

尽管一个需要静态类型检查的语言的编译器，可以通过简单地比较用户手动标注的类型和实际类型的等价性来实现这种校验，然而，在具有较复杂的类型系统的语言中，要求代码编写者对所有项都进行类型标注并不现实，这种情况下，一个实用的编译器还需具备类型推导的能力。

下面首先介绍几种经典的类型系统：

### Hindley-Milner 类型系统

HM 系统是一种经典而应用广泛的类型系统 [^11] [^12]，其主要优点是：
- 它是完备的
- 它不需要显式类型标注
- 它的类型推导算法是可判定的

主要限制是：
- 多态类型只能出现在进行 $let$ 绑定时

它的定型规则如下 [^8]：

$$
\begin{array}{cl}
 \displaystyle\frac{x:\sigma \in \Gamma}{\Gamma \vdash_D x:\sigma}&[\mathtt{Var}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e_0:\tau \rightarrow \tau' \quad\quad \Gamma \vdash_D e_1 : \tau }{\Gamma \vdash_D e_0\ e_1 : \tau'}&[\mathtt{App}]\\\\
 \displaystyle\frac{\Gamma,x:\tau\vdash_D e:\tau'}{\Gamma \vdash_D \lambda\ x\ .\ e : \tau \rightarrow \tau'}&[\mathtt{Abs}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e_0:\sigma \quad\quad \Gamma,x:\sigma \vdash_D e_1:\tau}{\Gamma \vdash_D \mathtt{let}\ x = e_0\ \mathtt{in}\ e_1 : \tau} &[\mathtt{Let}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e:\sigma' \quad \sigma' \sqsubseteq \sigma}{\Gamma \vdash_D e:\sigma}&[\mathtt{Inst}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e:\sigma \quad \alpha \notin \text{free}(\Gamma)}{\Gamma \vdash_D e:\forall\ \alpha\ .\ \sigma}&[\mathtt{Gen}]\\\\
 \end{array}
$$

其中 $\tau$ 是简单类型变量，$\sigma$ 是多态类型变量，区别是后者可以包含零或多个全称量词绑定的类型变量。

HM 类型系统的前四条定型规则是十分平凡的，唯一值得注意的是，在 $[\mathtt{Abs}]$ 规则的前提中，$x$ 是以简单类型的方式引入到环境中的，而在 $[\mathtt{Let}]$ 规则中，$x$ 是以多态类型的方式引入。因此，在 $let$ 表达式中可以通过应用 $[\mathtt{Inst}]$ 为不同的项规则特化出不同的类型，从而实现 $let$ 多态性。

和 TAPL 上介绍的基于约束的定型算法 [^13] 有略微不同的是，HM 类型系统的对象均为未定型的 $\lambda$ 项，而不具有任何显式的类型标注。在其他方面，这两种方法相当一致。HM 类型系统依赖类型推导来实现其多态类型，存在多种算法，如 Algorithm W 和 Algorithm J。两者的主要区别是如何处理 unify 类型过程中的副作用。前者稍显复杂但有利于 Soundness 的证明。

简单起见，我们介绍后者。Algorithm J 存在如下推导规则 [^8]：

$$
\begin{array}{cl}
\displaystyle\frac{x:\sigma \in \Gamma \quad \tau = \mathit{inst}(\sigma)}{\Gamma \vdash_J x:\tau}&[\mathtt{JVar}]\\\\
\displaystyle\frac{\Gamma \vdash_J e_0:\tau_0 \quad \Gamma \vdash_J e_1 : \tau_1 \quad \tau'=\mathit{newvar} \quad \mathit{unify}(\tau_0,\ \tau_1 \rightarrow \tau') }{\Gamma \vdash_J e_0\ e_1 : \tau'}&[\mathtt{JApp}]\\\\
\displaystyle\frac{\tau = \mathit{newvar} \quad \Gamma,x:\tau\vdash_J e:\tau'}{\Gamma \vdash_J \lambda\ x\ .\ e : \tau \rightarrow \tau'}&[\mathtt{JAbs}]\\\\
\displaystyle\frac{\Gamma \vdash_J e_0:\tau \quad\quad \Gamma,x:\bar{\Gamma}(\tau) \vdash_J e_1:\tau'}{\Gamma \vdash_J \mathtt{let}\ x = e_0\ \mathtt{in}\ e_1 :  \tau'}&[\mathtt{JLet}]
\end{array}
$$

其中：
- 第一条规则表明我们可以在将一个多态类型特化出一个简单类型 $\tau$，使得 $\sigma \sqsubseteq \tau$，并在 $\Gamma$ 类型环境中将具有该多态类型的变量 $x$ 定型为 $\tau$；
- 第二条规则则生成一个新的类型变量 $\tau'$ 来表示可能的应用的结果类型。由 $[\mathtt{App}]$ 规则反演可知，$e_0$ 应该具有形如 $\tau_1 \rightarrow \tau'$ 的类型，因此如果 $\tau_1 \rightarrow \tau'$ 和 $e_0$ 的已知类型 $\tau_0$ 统一，我们则可推出 $e_0\ e_1$ 具有 $\tau'$ 类型（该类型变量应该已经在 unify 操作中被替换成了一个已知的类型）。
- 第三条规则十分直观，不作描述。
- 第四条规则中，$\bar{\Gamma}(\tau) = \forall\ \hat{\alpha}\ .\ \tau$ 且 $\hat{\alpha} = \textrm{free}(\tau) - \textrm{free}(\Gamma)$，即，尽可能全称量化在 $\tau$ 中的自由类型变量，但是不能全称量化现有的类型上下文 $\Gamma$ 的自由类型变量。目的是使得 $let$ 绑定中的 $x$ 具有可能的最泛化的类型。

### System F 类型系统

和受限制的 $let$-多态相比，System F 类型系统引入了对类型的抽象和应用的机制。它在定型规则中额外增加了两条规则 [^13]：

$$
\begin{array}{cl}
\displaystyle\frac{\Gamma\vdash M\mathbin{:}\forall\alpha.\sigma}{\Gamma\vdash M\tau\mathbin{:}[\alpha\mapsto\tau]\sigma}&[\mathtt{TApp}]\\\\
\displaystyle\frac{\Gamma,\alpha\vdash M\mathbin{:}\sigma}{\Gamma\vdash\lambda\alpha.M\mathbin{:}\forall\alpha.\sigma}&[\mathtt{TAbs}]
\end{array}
$$

这解决了下面示例在 HM 类型系统和为 predicative 的 first-class 多态类型系统中进行类型检查的困难：
假设类型上下文 $\Gamma$ 中存在以下绑定：
$$
\begin{array}{cl}
\mathtt{id}&:\forall{\alpha}.\alpha\rightarrow{\alpha}\\\\
\mathtt{omega}&:(\forall{\alpha.\alpha\rightarrow{\alpha}})\rightarrow{(\forall{\alpha.\alpha\rightarrow{\alpha}})}\\\\
\mathtt{apply}&:\forall\gamma.\forall\delta.(\gamma\rightarrow\delta)\rightarrow\gamma\rightarrow\delta
\end{array}
$$
在 HM 系统中，这个类型上下文根本不可能存在，因为 HM 类型系统不允许 $\lambda$ 抽象具有 $(\forall\alpha.\alpha\rightarrow\alpha)\rightarrow(\forall\alpha.\alpha\rightarrow\alpha)$ 类型。

而在 predicative 的 first-class 多态系统中，仍然不允许将类型变量替换为另一个多态类型。因此，尽管 $\mathtt{omega\ id}$ 可以定型为 $\forall{\alpha.\alpha\rightarrow{\alpha}}$，我们仍然难以定型 $\mathtt{apply\ omega\ id}$ (不能进行 $[\gamma\mapsto\forall{\alpha.\alpha\rightarrow{\alpha}}][\delta\mapsto\forall{\alpha.\alpha\rightarrow{\alpha}}]$ 式的替换)。

在 System F 中，因为 $\mathtt{[TApp]}$ 规则，于是可以顺利进行类型变量替换从而定型 $\mathtt{apply\ omega\ id}$ 。

然而，这带来了类型推导上的麻烦。
假设类型上下文 $\Gamma$ 中存在以下绑定：
$$
\begin{array}{cl}
\mathtt{id}&:\forall{\alpha}.\alpha\rightarrow\alpha\\\\
\mathtt{choose}&:\forall\beta.\beta\rightarrow\beta\rightarrow\beta
\end{array}
$$

那么 $\mathtt{choose\ id}$ 究竟应该是 $\forall\beta. (\beta\rightarrow\beta)\rightarrow(\beta\rightarrow\beta)$ 还是 $(\forall{\alpha}.\alpha\rightarrow\alpha)\rightarrow(\forall{\alpha}.\alpha\rightarrow\alpha)$ 呢？（前者可以通过一次额外的类型泛化得到，但是两个类型的比较需要引入子类型，否则我们无法断言它们哪一个是更基础的类型）

可以证明，System F 的类型推导/检查算法是不可判定的 [^14]。实践上而言，使用 System F 类型系统使得编译器必须在某些时候要求用户显式标注类型以继续类型推导。

### 其他类型系统和类型系统的拓展

尽管这一部分和编程语言设计、静态分析、定理证明、形式化验证等领域关系更大，由于种类多样，也不太可能很好地涵盖在 “编译” 相关的综述中，但是为了这一章节的完整性，因此也不妨稍微提及。

- Dependent Type [^16]
  依赖类型。在 LC 中，项只依赖项；System F 中，存在多态的项依赖于类型。而在 Lambda P 系统中，存在依赖类型，允许一个类型依赖于项产生。比如说，对于一个长度为 n 的自然数向量，它可以具有 $vector\ n$ 类型。而如果有对应的 $\mathtt{push}$ 函数的话，它的类型签名就会是 $\Pi{n:nat}.\ nat\rightarrow vector\ n \rightarrow vector (n + 1)$。当然，依赖类型也会使得类型检查变得不可判定，所以侧重于通用编程的函数式编程语言往往不会采用这种类型系统。

- Refinement Type [^17]
  细化类型。有点类似于 Dependent Type，但是并不完全等价。Refinement Type 给类型加上了谓词，例如我们可以表达大于 5 的自然数的类型为 $n_{>5} : nat\ |\ n > 5$。不过从实用的角度考虑，不是所有的类型都能成为谓词，谓词也应该是可判定的。此外可能还需要通过定义 Subtyping 关系来联系带谓词的类型和它们的底层类型。

- Substructural Type System [^16]
  子结构类型。分为多种，其中以 Linear Type System 和 Affine Type System 较为常见。前者确保对象被且仅被使用一次，而后者确保对象被至多使用一次。

##### 实际编程语言中的类型系统

SML 97 使用 HM 类型系统（不过由于引用的存在，所以有 Value Restriction 限制泛化的发生）。OCaml 同样基于 HM 类型系统，但是加上了不少拓展以支持 OCaml 多样的功能特性。 Haskell 的核心类型系统发生过不少改变，最新的那个被称为 System FC [^15]（不过目前我无法理解）。

还有很多常用于定理证明的函数式编程语言，使用了 Dependent Type，如 Agda，Idris，Coq 等。

## 抽象机与虚拟机

正如在开头介绍中所说，编译的目标是多样的。主要用于原生开发的编程语言往往会被编译到特定、真实的硬件架构的机器指令，比如说 C 和 C++。而为了通用性、兼容性，一些编译器实现则会将编程语言编译到某种**抽象机器**（有时也叫虚拟机）上。对于 FP 语言而言，定义抽象机器的一个额外目的（也是最原始的目的）也许是为了说明对应语言的**语义**，比如说 SECD-Machine [^19]。

从很多角度来看，抽象机简化了编译器的构造：
- 抽象机可以针对语言的某些特性设计对应的、不太可能在真实硬件上出现的指令。
- 抽象机可以让编译器避开对于硬件细节的关注。
- 如果抽象机是实际的执行环境，那么它可以提供语言所需的运行时功能（比如说垃圾回收），以免编译器需要插入特定的处理代码。
- 如果抽象机是实际的执行环境，那么它可以提供了额外的隔离层次，可以简化安全相关功能的处理。

当然，抽象机也存在着问题，如果各硬件平台的抽象机实现，只是单纯地 “解释” 执行抽象指令，会造成极为严重的性能和效率损失。因此可能需要进一步的步骤：
- 只是将抽象机作为，方便较高层次抽象的优化的，某种**中间表示**，在完成该阶段后便开始像传统编译器一样，在编译期将抽象指令转换成机器指令。一种简单的做法是将每条抽象指令直接翻译为功能完全一致的机器代码片段，然后拼接起来，这样可以节省解释执行时指令分派的预测和缓存损耗。但是本质上仍然是低效的。
- 仍然使用抽象机的实现作为**实际的运行时**，但是在运行阶段，可能会结合收集到的 Profile 信息，将虚拟指令实时编译为机器指令。
- 较好的方法可能是利用现有的传统编译器基础设施，比如说 C 语言编译器、LLVM 或 JVM，将抽象指令编译成 C 语言、LLVM IR 或 JVM 字节码并传递给对应的工具链，然后便可摆脱剩余的工作了。

从结构来看，一个抽象机可能具有内存、程序计数器、虚拟寄存器等结构，和真实的物理机器在概念上十分类似。抽象机程序可能是一串抽象指令，就像汇编语言那样。抽象指令风格迥异，可以是隐式操作运行栈式的，也有像真实机器类似的寄存器操作。

##### 实际编程语言中的抽象机

**运行时环境**

考虑到 Java, Groovy，Kotlin，Scala，Clojure 等语言均使用 JVM 作为运行时，最有名、被广泛使用的抽象机/虚拟机应该是 JVM 了。与之对应的是 .NET 系语言的 CLI。它们都使用了栈机的形式，且抽象层次相对不高（每条指令对应的功能比较简单）。由于和本文主题关系较远，不作过多描述。

**FP 语言的抽象机**

对于 OCaml 和 MoscowML 来说，它们使用了 ZINC-Machine [^20]。另一个 FP 语言，Haskell，它的最著名实现 GHC 使用了 G-Machine [^21] [^23]，并发展为为 Spineless Tagless G-Machine [^22]。G-Machine 的显著特点是它基于**图规约**实现了非严格的求值策略：**惰性求值**。


## 代码转换
实用编译器的编译过程，不外乎通过语法解析器将人类可读的源代码转换为某种抽象的适合于当前阶段的中间表示，随后在这种表示上应用一系列的重写、变换，以进行优化或向底层执行环境贴近。这样的过程也被称作趟（Pass）。

尽管不同函数式编程语言的代码形式不尽相同，但是在忽略用户可读的语法后，可以说他们共享着很多较为相似的构造。将不同函数式编程语言的不同语法转换成这些相似构造的过程，受制于篇幅，不可能详尽列举，因此直接忽略。

### 一个实例

本节首先将使用如下的一个极简语言来进行说明如何将它**直接**转化为一种严格求值的**抽象机器的指令**形式 [^18]。限于时间因素，我们假定这个语言的 EBNF 既是它的具体语法，也代表了它的抽象结构（也就是我们可以自由地操作它们）。

```ebnf
<expr>  = <variable>
        | <constant>
        | <primitive> <expr> <expr>
        | if <expr> then <expr> else <expr>  
        | <expr> <expr>+      
        | λ <variable>+ . <expr>
        | let <variable> = <expr> in <expr>
        | letrec <variable> = <expr>
                   ...
                 <variable> = <expr> in <expr>
<primitive> = + | - | * | / | %
```
#### 目标抽象机器

##### 架构
该抽象机器具有一个指令内存，一个程序计数器指示当前正在执行的指令位置；这个抽象机器还有一个堆栈指针 SP，一个帧指针 FP，以及一个全局指针 GP；此外，为了存储分配的值，还有一块堆内存。

##### 数据布局

所有在堆上的值都进行了装箱。值可能具有以下几种不同的类型：
- 基本值：
  其中 v 是一个整数。
  ```plain
  +---+-----+
  | B |  v  |
  +---+-----+
  ```

- 函数值：
  其中 cp 指向了一段有效的代码内存的首地址。gp 指向了一个向量值，其中存储了自由变量的值构成的环境。ap 同样指向了一个向量值，存储了已经收集到的参数
  ```plain
  +---+------+------+------+
  | C |  cp  |  gp  |  ap  |
  +---+------+------+------+
  ```

- 向量值：
  其中 n 是向量值的长度，en 代表该向量的第 n 个元素。
  ```plain
  +---+---+------+-------+------+
  | V | n |  e1  |  ...  |  en  |
  +---+---+------+-------+------+
  ```

#### 转换方案

为了实现目标语言，我们需要一个转换方案 [^18]。核心是转换函数 `T`, 用 `T ( <e> ) tbl sl` 表示将 `<e>` 转换为对应的抽象指令。其中 `tbl` 表示一个映射，将变量名映射到其在栈上的地址或者捕获环境中的地址。`sl` 是一个整数，表示栈顶指针到函数参数区域的顶部，也是局部变量区域底部的距离，用于更方便地索引栈中的值。 

- **常量**：

  对于常量，使用 `@loadc <c>` 将其加载到栈顶，然后使用 `@box` 指令将其装箱。
  ```plain
  T ( <c> ) tbl sl =
      @loadc <c>
      @box
  ```

- **二元原始函数**：

  对于加减乘除和取余这些二元的原始函数，首先生成它的两个操作数表达式对应的指令序列，并且使用 `@unbox` 指令将结果拆箱置于栈顶。此时栈上已经准备好了两个操作数，我们使用对应 `<primitive>` 的指令来弹出操作数，计算它们的结果并置于栈顶。结果最后被装箱。
  ```plain
  T ( <primitive> <e1> <e2> ) tbl sl =
      T ( <e1> ) tbl sl
      @unbox
      T ( <e2> ) tbl sl
      @unbox
      @op <primitive>
      @box
  ```

- **条件判断**：

  对于条件判断，我们首先生成条件部分 `<e1>` 的指令序列，然后使用 `@unbox` 指令将栈顶的结果拆箱。如果结果为真，我们继续执行 then 部分 `<e2>` 的指令序列，否则我们使用 `@jmpz L1` 指令跳转到 else 部分 `<e3>` 的指令序列执行。
  ```plain
  T ( if <e1> then <e2> else <e3> ) tbl sl =
      T ( <e1> ) tbl sl
      @unbox
      @jmpz L1 
      T ( <e2> ) tbl sl
      @jmp L2
  L1: T ( <e3> ) tbl sl
  L2: ...
  ```
- **变量求值**：
  
  首先我们定义一个辅助函数 `getval!`，这个函数会根据当前的变量映射，生成对应的加载指令将变量值加载到栈顶。具体而言，如果 tbl 中变量是局部变量，我们使用 `@pushloc (sl - i)` 将其从 `mem[sp - (sl - i)]` 位置拷贝至栈顶；如果变量是非局部变量，我们使用 `@pushenv j` 将其从 `mem[gp + j]` 位置拷贝至栈顶。
  ```plain
  getval! <v> tbl sl =
      match tbl <v> with
        Stack address i -> @pushloc (sl - i)
        Env   address j -> @pushenv j 
  ```
  然后变量求值的逻辑和 `getval!` 相同。
  ```plain
  T ( <v> ) tbl sl = 
      getval! <v> tbl 
  ```
- **λ-abstraction 定义**：
  
  为了定义 λ-abstraction，需要捕获所有函数体内的自由变量：`<f1> ... <fg>`。因此使用 `getvar!` 依次将它们置于栈上。由于栈的高度会发生变化，所以传递给 `getvar!` 的 `sl` 距离也需要递增。随后使用 `@makevec g` 指令收集这 g 个自由变量的值为一个长度为 g 的向量值，并将其置于栈顶。再使用 `@makefunc L1` 创建一个函数对象。这个函数对象的 `cp` 参数指向 L1 标签所代表的地址，`ap` 参数指向一个长度为 0 的空向量以备后续使用，而 `gp` 参数则指向刚刚收集的那个捕获自由变量对应值的向量。通过这个过程，我们完成了**闭包转换**，而且形成的是一种**扁平闭包**。

  L1 和 L2 标签之间的指令序列代表了函数体。在函数入口点处，我们使用 `@targ k` 来判断参数个数。`@targ k` 首先判断 `sp - fp` 是否大于等于 k，如果是则表示调用函数时已经提供了足够的参数，因此可以继续执行下去。否则，我们需要重新收集这些参数，并且生成一个新的、只需要剩余参数的函数对象。具体如下：

  **参数不足时的处理**

  当执行到 `@targ k` 时，栈上已经放置了个数不足的参数值。显然不能简单地丢弃它们。因此我们重新新建一个向量值，其中依次放入栈上 `sp` 到 `fp` 间的参数值，然后将其置于栈顶。为了返回一个函数值，需要再次打包该函数，其中 `cp` 字段就是当前的 `pc`。 将 `ap` 字段设置为刚才收集的参数向量。而 `gp` 字段则设置为当前的 `gp`。重新收集的函数值将会被放置于栈顶。最后，我们使用栈上保存的调用者的 `gp`, `fp`, `sp` 恢复到调用前的状态，并确保重新收集的函数值被保留在栈顶。简而言之，通过这样的处理，我们可以实现**自动柯里化**。

  **参数过多时的处理**

  函数的返回使用 `@return k` 指令。若实际提供的参数不多于函数所需要的，则和以上操作类似，我们使用栈上保存的调用者的 `gp`, `fp`, `sp` 恢复到调用前的状态，并确保返回值被保留在栈顶。否则和 `@slide k` 等价地操作，释放已被使用的参数值占用的空间（栈顶值向下滑动 k 个位置），然后执行和 `@apply` 相同的操作（后续会介绍），它会利用未使用的参数来尝试调用当前栈顶的函数值。

  ```plain
  T ( λ <v1> ... <vn> . <e> ) =
      getvar! <f1> tbl sl
      getvar! <f2> tbl (sl + 1)
      ...
      getvar! <fg> tbl (sl + g)
      @makevec g
      @makefunc L1
      @jmp L2
  L1: @targ k
      Tb ( <e> ) tbl' 0
      @return k
  L2: ...
  where {<f1>, ... ,<fn>} =
            free (λ <v1> ... <vn> . <e>)
        tbl' =
            {
              v1 |-> Local 0, ... , vn |-> Local (1-n), 
              f1 |-> Env   0, ... , fn |-> Env (g-1)
            }
  ```

- **let 绑定**：

  实现 `let` 绑定的方法相对简单。首先生成 `<e0>` 对应的指令序列。然后，由于 `v` 对应的值已经被求出，向转换函数的变量映射中加入 `<v> |-> L (sl + 1)`，代表 `<v>` 为局部变量且栈中位置为 sl + 1。然后再用转换函数利用更新后的映射来生成 `<e1>` 对应的指令序列。在这段表达式执行完成后，使用 `@slide n` 释放 `<v>` 对应的值占用的空间（相当于栈顶值向下滑动 n 个位置）。

  ```plain
  T ( let <v> = <e0> in <e1> ) tbl sl =
      T ( <e0> ) tbl sl
      T ( <e1> ) tbl ++ {<v> |-> L (sl + 1)} (sl + 1)
      @slide 1
  ```

- **函数应用**：

  函数应用首先需要确定调用完成后的返回地址。为此定义 `@entry L` 指令，表示向堆栈中压入当前的 `gp`, `fp` 和 L 标签对应的指令地址，并且将 `fp` 指向栈顶。随后依次对函数的 n 个参数表达式从右到左依次生成对应的指令序列。最后我们生成 `@apply` 指令，执行以下操作：
    1. 检查栈顶元素是不是一个函数值，不是则报错。
    2. 解开函数值，获取函数的 `ap` 字段，将 `ap` 字段指向的参数向量中的各参数元素压入堆栈。这些参数是上一次调用函数时参数个数不足的情况下所保存的，和栈上已有的参数将一同被函数体使用。
    3. 使用函数值的`gp` 字段更新当前的 `gp` 值。
    4. 跳转至函数值的 `cp` 字段指向的指令地址。
  ```plain
  T （ <f> (<e1> ... <en>) ）tbl sl =
      @entry L1
      T ( <en> ) tbl (sl + 3)
        ...
      T ( <e1> ) tbl (sl + n + 2) 
      T ( <f> )  tbl (sl + n + 3)
      @apply
  L1: ... 
  ```

- **递归定义**：

  在 `letrec` 递归定义中，`<e1> ... <en>` 可能会依赖 `<v1> ... <vn>` 中的任何一个。如果不加任何处理地按照顺序生成 `<ei>` 的指令序列，那么很难在前面的指令序列中找到后续的 `<v>` 的值的位置。因此首先使用 `@alloc n` 预留 n 个空间。由于这 n 个位置相对而言是固定的，因此我们可以使用它们的位置更新变量地址映射，并依次生成 `<ei>` 对应的指令序列。在每次生成 `<e>` 对应的指令序列后，使用 `@rewrite` 将栈顶元素放置到预留的位置上。最后生成 `<e>` 对应的指令序列。和 let 绑定类似，在执行完成后还需要使用 `@slide` 释放掉对应的栈上空间。

  ```plain
  T ( letrec <v1> = <e1>
              ...
             <vn> = <en> in <e> ) tbl sl =
      @alloc n
      T ( <e1> ) tbl' (sl + n)
      @rewrite n
      ...
      T ( <en> ) tbl' (sl + n)
      @rewrite 1
      T ( <e> )  tbl' (sl + n)
      @slide n
  where tbl' = tbl ++ {<v1> |-> L (sl + 1), ..., <vn> |-> L (sl + n)}
  ```

### 其他结构的转换
上一小节的转换覆盖了很多基础的函数式编程语言特性，但是没有涵盖模式匹配，代数数据类型等结构的转换。接下来将对此进行一些简短的介绍。

#### 代数数据类型
在函数式编程语言中，常常使用代数数据类型。理论上而言，代数数据类型表明它们可以通过代数运算（如 +,*）来进行类型的组合。其中最重要的是积类型与和类型。和其背后代数理论的复杂不同，代数数据类型向具体实现的转换相对简单。积类型通常被实现为所组合的原始类型的拼接，可能会重排序以针对现代计算机结构进行优化。和类型可能被实现为 tagged union 的结构，通过提供一个类型标记，以区分不同的类型。

#### 模式匹配
模式匹配是函数式编程语言中最重要的结构之一，它可以用来检查语言中的项、值是否符合某个特定的模式。在很多语言中，模式匹配被用来解构数据类型。模式匹配实现的一种经典策略是将其转换为一个决策树 [^24]。尽管无法获得最优的决策树，但是有启发式的方法帮助获得较优的决策树 [^25]。对于具有非严格求值策略的语言如 Haskell 来说，实现策略可能会略有不同 [^23]。

<!--
## 代码优化
代码优化是一个十分复杂的问题，然而它可以帮助我们提高程序的性能。上一节中对一个简单函数式语言的转换，有效但并不高效。之前已经提及了趟的概念，针对优化而言，高度优化的编译器可能会有很多趟进行不同目的、功能的优化工作。接下来将简单地介绍一些常见的优化。
-->

[^1]: Appel, A. (1991). Compiling with Continuations. Cambridge: Cambridge University Press.
[^2]: Cong, Y., Osvald, L., Essertel, G., & Rompf, T. (2019). Compiling with Continuations, or without? Whatever.. Proc. ACM Program. Lang., 3(ICFP).
[^3]: Kennedy, A. (2007). Compiling with Continuations, Continued. In Proceedings of the 12th ACM SIGPLAN International Conference on Functional Programming (pp. 177–190). Association for Computing Machinery.
[^4]: Flanagan, C., Sabry, A., Duba, B., & Felleisen, M. (1993). The Essence of Compiling with Continuations. SIGPLAN Not., 28(6), 237–247.
[^5]: Aho, A., Lam, M., Sethi, R., & Ullman, J. (2006). Compilers: Principles, Techniques, and Tools (2nd Edition). Addison-Wesley Longman Publishing Co., Inc..
[^6]:Appel, A. (1998). SSA is Functional Programming. SIGPLAN Not., 33(4), 17–20.
[^7]: IntermediateLanguage. (2021). Retrieved 30 November 2021, from http://mlton.org/IntermediateLanguage
[^8]: Hindley–Milner type system. (2021, December 14). In Wikipedia. https://en.wikipedia.org/wiki/Hindley-Milner_type_system
[^9]: Maurer, L., Downen, P., Ariola, Z., & Peyton Jones, S. (2017). Compiling without Continuations. SIGPLAN Not., 52(6), 482–494.
[^10]: Kelsey, R. (1995). A Correspondence between Continuation Passing Style and Static Single Assignment Form. SIGPLAN Not., 30(3), 13–22.
[^11]: Hindley, R. (1969). The Principal Type-Scheme of an Object in Combinatory Logic. In Transactions of the American Mathematical Society (Vol. 146, p. 29). JSTOR.
[^12]: Milner, R. (1978). A theory of type polymorphism in programming. In Journal of Computer and System Sciences (Vol. 17, Issue 3, pp. 348–375). Elsevier BV.
[^13]: Pierce, B. C. (2002). Types and Programming Languages (1st ed). The MIT Press.
[^14]: Wells, J. B. (1999). Typability and type checking in System F are equivalent and undecidable. Annals of Pure and Applied Logic, 98(1-3), 111–156.
[^15]: R. A. Eisenberg. System FC, as implemented in GHC. University of Pennsylvania Technical Report MS-CIS-15-09, 2015.
[^16]: Pierce, B. C. (2004). Advanced Topics in Types and Programming Languages. The MIT Press.
[^17]: Freeman, T., & Pfenning, F. (1991). Refinement Types for ML. SIGPLAN Not., 26(6), 268–277.
[^18]: Wilhelm, R., & Seidl, H. (2010). Compiler Design.
[^19]: Landin, P. (1964). The Mechanical Evaluation of Expressions. The Computer Journal, 6(4), 308-320.
[^20]: Leroy, X. (1990). The ZINC experiment: an economical implementation of the ML language (Doctoral dissertation, INRIA).
[^21]: Johnsson, T. (1984, June). Efficient compilation of lazy evaluation. In Proceedings of the 1984 SIGPLAN symposium on Compiler construction (pp. 58-69).
[^22]: Jones, S. L. P. (1992). Implementing lazy functional languages on stock hardware: the Spineless Tagless G-machine. Journal of functional programming, 2(2), 127-202.
[^23]: Peyton Jones, S. L. (1987). The implementation of functional programming languages (prentice-hall international series in computer science). Prentice-Hall, Inc..
[^24]: Baudinet, M., & MacQueen, D. (1985). Tree pattern matching for ML.
[^25]: Maranget, L. (2008, September). Compiling pattern matching to good decision trees. In Proceedings of the 2008 ACM SIGPLAN workshop on ML (pp. 35-46).
