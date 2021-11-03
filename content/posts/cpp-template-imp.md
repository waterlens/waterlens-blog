---
title: "Implement Imperative Program with C++ Templates"
date: 2021-11-02T14:16:42+08:00
mathjax: true
tags: [cpp, metaprogramming]
---

How to implement a simple imperative programming language (in a abstract syntax tree form) with C++ templates ?

## Natural Number Definition (Peano axioms)

- $0$ is a natural number.
- For every natural number $n$, $S(n)$ is a natural number.
- For all natural numbers $m$ and $n$, $m = n$ $\Leftrightarrow$ $S(m) = S(n)$.
- For every natural number $n$, $S(n) = 0$ is false.

So, write them in c++ templates:

```cpp
struct O {};
template <typename N> struct S {};

using Zero = O;
using One = S<O>;
using Two = S<S<O>>;
using Three = S<S<S<O>>>;

template <typename N1, typename N2> struct eqn {};
template <> struct eqn<O, O> { using result = True; };
template <typename N> struct eqn<O, S<N>> { using result = False; };
template <typename N> struct eqn<S<N>, O> { using result = False; };
template <typename N1, typename N2> struct eqn<S<N1>, S<N2>> {
  using result = typename eqn<N1, N2>::result;
};
```

And then we can define arithmetic operations now.
## Arithmetic Operations

### Addition
$$n + 0 = 0$$
$$m + S(n) = S(m + n)$$

### Multiplication
$$m * 0 = 0$$
$$m * S(n) = m + m * n$$

both can be transformed into the following code snippets:

```cpp
template <typename N1, typename N2> struct plus {};
template <typename N> struct plus<N, O> { using result = N; };
template <typename N1, typename N2> struct plus<N1, S<N2>> {
  using result = S<typename plus<N1, N2>::result>;
};
```

```cpp
template <typename N1, typename N2> struct mult {};
template <typename N> struct mult<N, O> { using result = O; };
template <typename N1, typename N2> struct mult<N1, S<N2>> {
  using result = typename plus<N1, typename mult<N1, N2>::result>::result;
};
```

## Boolean Operations

```cpp
struct True {};
struct False {};

template <typename B> struct negb {};
template <> struct negb<True> { using result = False; };
template <> struct negb<False> { using result = True; };

template <typename B1, typename B2> struct andb {};
template <typename B> struct andb<True, B> { using result = B; };
template <typename B> struct andb<False, B> { using result = False; };
```

### Arithmetic Evaluation

The basic form of our program can be written with BNF:
$$
\begin{align}
aexp :=&\ aexp + aexp \\\\\\
		  |&\ aexp * aexp \\\\\\
      |&\ \textbf{num}
\end{align}
$$

$$
\begin{align}
bexp :=&\ bexp \land bexp \\\\\\ 
			|&\ \neg\ bexp \\\\\\
			|&\ aexp = aexp \\\\\\
			|&\ aexp \le aexp \\\\\\
			|&\ \textbf{true}\ |\ \textbf{false}
\end{align}
$$

According to these, we write their evaluator:
```cpp
template <typename N> struct ANum {};
template <typename A1, typename A2> struct APlus {};
template <typename A1, typename A2> struct AMult {};

template <typename A> struct aeval {};
template <typename N> struct aeval<ANum<N>> {  using result = N; };
template <typename A1, typename A2>
struct aeval<APlus<A1, A2>> {
  using result = typename plus<typename aeval<A1>::result,
                               typename aeval<A2>::result>::result;
};
template <typename A1, typename A2>
struct aeval<AMult<A1, A2>> {
  using result = typename mult<typename aeval<A1>::result,
                               typename aeval<A2>::result>::result;
};
```

```cpp
struct BTrue {};
struct BFalse {};
template <typename B> struct BNot {};
template <typename B1, typename B2> struct BAnd {};

// beval
template <typename B> struct beval {};
template <> struct beval<BTrue> { using result = True; };
template <> struct beval<BFalse> { using result = False; };
template <typename A1, typename A2> struct beval<BLe<A1, A2>> {
  using result = typename len<typename aeval<A1>::result,
                              typename aeval<A2>::result>::result;
};
template <typename B> struct beval< BNot<B>> {
  using result = typename negb<typename beval<B>::result>::result;
};
template <typename B1, typename B2> struct beval<BAnd<B1, B2>> {
  using result = typename andb<typename beval<B1>::result,
                               typename beval<B2>::result>::result;
};
```

## Data Structrue

### Partial Map
```cpp
struct empty {};
template <typename K, typename V, typename M> struct record {};

// update
template <typename K, typename V, typename M> struct update {
  using result = record<K, V, M>;
};

// find
template <typename K, typename M> struct find {};
template <typename K> struct find<K, empty> { using result = None; };
template <typename K, typename V, typename M> struct find<K, record<K, V, M>> {
  using result = Some<V>;
};

template <typename K1, typename K2, typename V, typename M>
struct find<K1, record<K2, V, M>> {
  using result = typename find<K1, M>::result;
};
```

You may just know the code above is functional like a container you can store a key and find its value.

## Variable

Now we introduce the variable.
Then the evaluation must be under a particular context, which we call it **environment** or **state**.

Our extended form of program:
$$
\begin{align}
aexp :=& ... \\\\\\
      |&\ \textbf{id}
\end{align}
$$

To evaluate a $\textbf{id}$, we need an enviroment $\textbf{ST}$.
```cpp
template <typename ST, typename ID> struct aeval<ST, AId<ID>> {
  using result = typename unwrap<typename find<ID, ST>::result>::result;
};
```
$\textbf{ST}$ should be added to each of the evaluation rules mentioned above.

## Commands Evaluation

We extend our program with new syntax:
$$
\begin{align}
com :=&\ skip \\\\\\
     |&\ \textbf{id}\ :=\ aexp \\\\\\
     |&\ com\ ;\ com \\\\\\
     |&\ \textbf{if}\ bexp\ \textbf{then}\ com\ \textbf{else}\ com\ \textbf{end} \\\\\\
     |&\ \textbf{while}\ bexp\ \textbf{do}\ com\ \textbf{end}
\end{align}
$$

```cpp
struct CSkip {};
template <typename ID, typename A> struct CAsgn {};
template <typename C1, typename C2> struct CSeq {};
template <typename B, typename C1, typename C2> struct CIf {};
template <typename B, typename C> struct CWhile {};

// ceval
template <typename ST, typename C> struct ceval {};
template <typename ST> struct ceval<ST, CSkip> { using result = ST; };
template <typename ST, typename ID, typename A> struct ceval<ST, CAsgn<ID, A>> {
  using result = typename update<ID, typename aeval<ST, A>::result, ST>::result;
};
template <typename ST, typename C1, typename C2>
struct ceval<ST, CSeq<C1, C2>> {
  using result = typename ceval<typename ceval<ST, C1>::result, C2>::result;
};
template <typename ST, typename COND, typename C1, typename C2>
struct cond_eval {};
template <typename ST, typename C1, typename C2>
struct cond_eval<ST, True, C1, C2> {  using result = typename ceval<ST, C1>::result; };
template <typename ST, typename C1, typename C2>
struct cond_eval<ST, False, C1, C2> {
  using result = typename ceval<ST, C2>::result;
};
template <typename ST, typename B, typename C1, typename C2>
struct ceval<ST, CIf<B, C1, C2>> {
  using result =
      typename cond_eval<ST, typename beval<ST, B>::result, C1, C2>::result;
};
template <typename ST, typename B, typename C> struct ceval<ST, CWhile<B, C>> {
  using result = typename cond_eval<ST, typename beval<ST, B>::result,
                                    CSeq<C, CWhile<B, C>>, CSkip>::result;
};
```
## Trivial Check

This is a simple program to calculate the factorial of 6.

```lua
x = 6;
y = 1;
while !(x = 0) do
  y = x * y;
  x = x - 1
end
```

Transform it into the "template like":

```cpp
using init = CSeq<CAsgn<X, ANum<S<S<S<S<S<S<O>>>>>>>>, CAsgn<Y, ANum<One>>>;
using body = CSeq<CAsgn<Y, AMult<AId<X>, AId<Y>>>,
                  CAsgn<X, AMinus<AId<X>, ANum<One>>>>;
using cond = BNot<BEq<AId<X>, ANum<Zero>>>;
using prog = CSeq<init, CWhile<cond, body>>;
using result = ceval<update<X, Zero, update<Y, Zero, empty>::result>::result,
                      prog>::result;
static_assert(ind_nat_to_lit<unwrap<find<Y, result>::result>::result>::value == 720, "");
```
