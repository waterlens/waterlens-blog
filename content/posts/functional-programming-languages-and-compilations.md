---
title: "An Overview of Functional Programming Languages and Compilations"
date: 2021-11-17T22:53:08+08:00
draft: false
mathjax: true
---

# 函数式编程语言及其编译：概述

## 介绍

函数式编程（下称 FP）脱胎于 Lambda Calculus（下称 LC），和其他不同的编程范式相比，它的最典型特征是高阶函数的应用。除此之外，一部分 FP 语言拥有一个更加复杂而强大的类型系统，并为此需要依赖与之匹配的类型推断、检查能力。相比于命令式的编程语言千篇一律的按值调用，FP 语言在求值策略的选择上更加灵活（尽管对于某种特定的 FP 语言，求值策略很有可能是一致的）而复杂。由于和 LC 的历史渊源，FP 语言对 “重复” 这一概念的实现，常常青睐于递归，而不是基于改变归纳变量的状态的 “循环”、“迭代”。与非 FP 语言相比，FP 语言往往强调较少的副作用乃至避免副作用。此外，还有自动柯里化、模式匹配等等特性。这些 FP 的特征，极大地改变了 FP 语言的编译实现。某些特性或功能，诸如高阶函数、递归、Continuation、自动柯里化等，如果编译器不能高效地实现，那么会导致 FP 语言实现的程序的运行效率的极度下降。而类型系统的实现，还兼具了理论上的一些困难抉择。

我们谈论编译这个概念时，需要注意的是它所描述的过程：将某种编程语言写成的源代码转换成另一种目标语言。广义来说目标语言的选取是任意的，因此不用拘泥于机器语言、汇编语言。因此在接下来的内容中，编译的目标语言可能是抽象的，也有可能是某种现有的编程语言，或者更加底层的语言。

## 中间表示

在进行编译时，编译器往往依赖一种或多种，对文本形式源代码中所指代的语言结构的表示方法，并在此基础上对这些结构进行变换或者将其映射到更为基础的结构。使用不同 IR 的编译器通常在编译的流程上存在着明显差异。下列列举的 IR 种类并不相互排斥：这意味着它们可能被组合在一起使用。

- $\lambda$ 演算
在 FP 语言中，最为常见的表示方法很显然是 LC 自身（以及它的变体）。这一做法的好处是十分显然的：FP 语言的特性和 LC 息息相关，这种特性所对应理论的形式化的描述也很可能是由某种拓展后的 LC 写成的，因此可以轻松地将源语言中的结构映射到更熟悉的结构上去，进而应用更通用的变换、优化手段。典型地来说，OCaml 编译器的 Clambda 和优化器所用的 Flambda 就使用了此类表示。

接下来所要介绍的中间表示形式大多是或等价于某种具有特定形式的 LC：

- CPS 形式的 IR
CPS, Continuation-Passing-Style，和直接风格相对应，在这种 “风格” 的表示中，函数、过程的 Continuation 需要被显式传递。通过 CPS 变换，我们可以将非 CPS 的表示转换为一种 CPS 表示。通过这种转换，程序的控制流以 Continuation 的形式暴露在外，从而更加便于编译器通过一系列普通的 $\beta$-规约 与 $\eta$-变换 优化程序结构 [^1]。尽管如此，原始的 CPS 形式的表示也存在很多问题，比如原始程序的 CPS 表示会十分复杂而冗长；难以对普通函数的进行统一的表示；难以优化不发生逃逸的跳转；实际实现中，过程记录常被放置到堆内存上，难以充分利用现代硬件的堆栈 保存过程调用的上下文等问题 [^2]。为了解决这些问题，发展出了很多 CPS 的变体。如 Kennedy 提出的 2nd-class Continuation 不再将 Continuation 视为普通的函数 [^3]，从而允许将 Continuation 直接编译为普通的跳转。SML/NJ 使用了 CPS 形式的 IR [^1]。

还有一些 IR 形式，例如：

- ANF 形式的 IR
这种 IR 的灵感自 CPS 表示 [^4]，但要求函数参数必须是平凡的，非平凡表达式的求值必须由 $let$ 进行绑定。这一形式具有结构上更简单直观（因为仍然使用 Direct Style 而非 CPS，但和 CPS 能力相同）、易于进行机器代码生成的优点。但是存在规约后不再封闭的问题（即 ANF 项的 $\beta$-规约的结果可能不是一个 ANF 项）[^2]。近年来，SPJ 使用 join point 拓展了 ANF，并成功在 GHC 中实现了这一形式的中间表示 [^9]。

- SSA 形式的 IR
这是一种在传统语言编译器中十分常见也最为著名的 IR 的形式，但是在 FP 语言编译器中并不常见。通常情况下，我们所见到的 SSA 形式的 IR，为不同基本块组成的连通图，其中基本块中主要包含模仿机器指令集的四元式 [^5]。尽管看起来与 CPS 差异巨大，仍然可以证明了 SSA 是 CPS 的一个子集 [^10] [^6]。也有 FP 语言的编译器使用这种表示（和通常见到的 SSA IR 不完全相同），如 MLton 优化编译器的 SSA 和 SSA2 [^7]。

- 图形式的 IR
这是一类范围较广的 IR，如 CFG，PDG 等，也可 SSA 形式的 IR 搭配使用。此处由于和本文关系不大，故不做过多赘述。

- C--/Cmm
这是一种具体、贴近底层的 IR，常用于向本机代码转换的最后阶段。很多 FP 语言的本机编译器（而不是虚拟机/字节码解释器）使用它。如 OCaml 本机编译器和 GHC 都使用了这一 IR。

## 类型检查与推导
一个编程语言的类型系统，将该编程语言的所能表达的项、数值、表达式等归类为不同的类型，并赋予这些类型特定的性质和作用规则。类型检查检验程序的项是否符合该编程语言的类型系统的约束。我们将在需要在程序执行期间进行等价性比较的类型检查称为动态类型检查，不需要的则称之为静态类型检查。对于那些进行动态类型检查的 FP 语言如 Scheme 来说，编译器不需要在编译时承担类型检查和推导的工作。

尽管一个需要静态类型检查的语言的编译器，可以通过简单地比较用户手动标注的类型和实际类型的等价性来实现这种校验，然而，在具有较复杂的类型系统的语言中，要求代码编写者对所有项都进行类型标注并不现实，这种情况下，一个实用的编译器还需具备类型推导的能力。

下面首先介绍几种类型系统：

##### Hindley-Milner 类型系统
  
HM 系统 [^11] [^12] 的定型规则如下：

$$
\begin{array}{cl}
 \displaystyle\frac{x:\sigma \in \Gamma}{\Gamma \vdash_D x:\sigma}&[\mathtt{Var}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e_0:\tau \rightarrow \tau' \quad\quad \Gamma \vdash_D e_1 : \tau }{\Gamma \vdash_D e_0\ e_1 : \tau'}&[\mathtt{App}]\\\\
 \displaystyle\frac{\Gamma,\;x:\tau\vdash_D e:\tau'}{\Gamma \vdash_D \lambda\ x\ .\ e : \tau \rightarrow \tau'}&[\mathtt{Abs}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e_0:\sigma \quad\quad \Gamma,\,x:\sigma \vdash_D e_1:\tau}{\Gamma \vdash_D \mathtt{let}\ x = e_0\ \mathtt{in}\ e_1 : \tau} &[\mathtt{Let}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e:\sigma' \quad \sigma' \sqsubseteq \sigma}{\Gamma \vdash_D e:\sigma}&[\mathtt{Inst}]\\\\
 \displaystyle\frac{\Gamma \vdash_D e:\sigma \quad \alpha \notin \text{free}(\Gamma)}{\Gamma \vdash_D e:\forall\ \alpha\ .\ \sigma}&[\mathtt{Gen}]\\\\
 \end{array}
$$

其中 $\tau$ 是多型变量，$\sigma$ 是单型变量，区别是前者者可以包含零或多个全称量词绑定的类型变量。

HM 类型系统的前四条定型规则是十分平凡的，唯一值得注意的是，在 $[\mathtt{Abs}]$ 规则的前提中，$x$ 是以单型的方式引入到环境中的，而在 $[\mathtt{Let}]$ 规则中，$x$ 是以多型的方式引入。因此，在 $let$ 表达式中可以通过应用 $[\mathtt{Inst}]$ 为不同的项规则特化出不同的类型，从而实现 $let$ 多态性。

和 TAPL 上介绍的基于约束的定型算法 [^13] 有略微不同的是，HM 类型系统的对象均为未定型的 $\lambda$ 项，而不具有任何显式的类型标注。在其他方面，这两种方法相当一致。HM 类型系统依赖类型推导来实现其多态类型，存在多种算法，如 Algorithm W 和 Algorithm J。两者的主要区别是如何处理 unify 类型过程中的副作用。前者稍显复杂但有利于 Soundness 的证明。

简单起见，我们介绍后者。Algorithm J 存在如下推导规则：

$$
\begin{array}{cl}
\displaystyle\frac{x:\sigma \in \Gamma \quad \tau = \mathit{inst}(\sigma)}{\Gamma \vdash_J x:\tau}&[\mathtt{JVar}]\\\\
\displaystyle\frac{\Gamma \vdash_J e_0:\tau_0 \quad \Gamma \vdash_J e_1 : \tau_1 \quad \tau'=\mathit{newvar} \quad \mathit{unify}(\tau_0,\ \tau_1 \rightarrow \tau') }{\Gamma \vdash_J e_0\ e_1 : \tau'}&[\mathtt{JApp}]\\\\
\displaystyle\frac{\tau = \mathit{newvar} \quad \Gamma,\;x:\tau\vdash_J e:\tau'}{\Gamma \vdash_J \lambda\ x\ .\ e : \tau \rightarrow \tau'}&[\mathtt{JAbs}]\\\\
\displaystyle\frac{\Gamma \vdash_J e_0:\tau \quad\quad \Gamma,\,x:\bar{\Gamma}(\tau) \vdash_J e_1:\tau'}{\Gamma \vdash_J \mathtt{let}\ x = e_0\ \mathtt{in}\ e_1 :  \tau'}&[\mathtt{JLet}]
\end{array}
$$

其中：
- 第一条规则表明我们可以在将一个多型实例化出一个单型 $\tau$，使得 $\sigma \sqsubseteq \tau$，并在 $\Gamma$ 类型环境中将具有该多型的变量 $x$ 定型为 $\tau$；
- 第二条规则则生成一个新的类型变量 $\tau'$ 来表示可能的应用的结果类型。由 $[\mathtt{App}]$ 规则反演可知，$e_0$ 应该具有形如 $\tau_1 \rightarrow \tau'$ 的类型，因此如果 $\tau_1 \rightarrow \tau'$ 和 $e_0$ 的已知类型 $\tau_0$ 统一，我们则可推出 $e_0\ e_1$ 具有 $\tau'$ 类型（该类型变量应该已经在 unify 操作中被替换成了一个已知的类型）。
- 第三条规则十分直观，不作描述。
- 第四条规则中，$\bar{\Gamma}(\tau) = \forall\ \hat{\alpha}\ .\ \tau$ 且 $\hat{\alpha} = \textrm{free}(\tau) - \textrm{free}(\Gamma)$，即，尽可能全称量化在 $\tau$ 中的自由类型变量，但是不能全程量化现有的类型上下文 $\Gamma$ 中的自由类型变量。目的是使得 $let$ 绑定中的 $x$ 具有可能的最泛化的类型。

[^1]: Appel, A. (1991). Compiling with Continuations. Cambridge: Cambridge University Press.
[^2]: Cong, Y., Osvald, L., Essertel, G., & Rompf, T. (2019). Compiling with Continuations, or without? Whatever.. Proc. ACM Program. Lang., 3(ICFP).
[^3]: Kennedy, A. (2007). Compiling with Continuations, Continued. In Proceedings of the 12th ACM SIGPLAN International Conference on Functional Programming (pp. 177–190). Association for Computing Machinery.
[^4]: Flanagan, C., Sabry, A., Duba, B., & Felleisen, M. (1993). The Essence of Compiling with Continuations. SIGPLAN Not., 28(6), 237–247.
[^5]: Aho, A., Lam, M., Sethi, R., & Ullman, J. (2006). Compilers: Principles, Techniques, and Tools (2nd Edition). Addison-Wesley Longman Publishing Co., Inc..
[^6]:Appel, A. (1998). SSA is Functional Programming. SIGPLAN Not., 33(4), 17–20.
[^7]: IntermediateLanguage. (2021). Retrieved 30 November 2021, from http://mlton.org/IntermediateLanguage
[^8]:
[^9]: Maurer, L., Downen, P., Ariola, Z., & Peyton Jones, S. (2017). Compiling without Continuations. SIGPLAN Not., 52(6), 482–494.
[^10]: Kelsey, R. (1995). A Correspondence between Continuation Passing Style and Static Single Assignment Form. SIGPLAN Not., 30(3), 13–22.
[^11]: Hindley, R. (1969). The Principal Type-Scheme of an Object in Combinatory Logic. In Transactions of the American Mathematical Society (Vol. 146, p. 29). JSTOR.
[^12]: Milner, R. (1978). A theory of type polymorphism in programming. In Journal of Computer and System Sciences (Vol. 17, Issue 3, pp. 348–375). Elsevier BV.
[^13]: Pierce, B. C. (2002). Types and Programming Languages (1st ed). The MIT Press.
