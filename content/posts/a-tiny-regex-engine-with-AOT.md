---
title: "A Tiny Regex Engine With AOT"
date: 2021-11-13T14:18:49+08:00
draft: true
mathjax: true
tags: [cpp, regex, AOT]
---

The regular expression is initially developed during the process of formalizing the **regular language** in the 1950s.
Except for its application in theoretical computer science, the regular expression has also been a useful tool for searching in the strings.
There were therefore many **regular expression engines** for such a purpose, each one with different features.

## Syntax of Regular Expression
##### Minimal definition
Assume that both $a$ and $b$ are regular expressions, $m$ is a single character:

- $\epsilon$: empty string

Denote to match the empty string.

- $m$: literal character

Denote to match the character itself.

- $ab$: concatenation

Denote a string can be accepted if it is the concatenation of a string accepted by $a$ and one accepted by $b$.

- $a|b$: alternative

Denote a string can be either a string accepted by $a$ or one accepted by $b$.

- $a*$: Kleene star

Denote an accepted string can be obtained by repeating the string 0 or more times, which is accepted by $a$.

##### Extension
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
A **regex processor**, or **regex engine**, is able to transform a **regular expression** into one kind of internal representation so that it can begin to do string matching or string searching. There are so many different approaches to implementing such a processor.

The most typical and oldest method is converting the **regex** into a **DFA**. But the method exists the problem that an NFA with $n$ states will be converted into a DFA with $2^p$ states, which probably causes huge memory consumption. In practice, we may use **lazy construction** or **state cache** to assuage the drawback. Lazy construction means we don't construct all DFA states at once, instead we construct them only when we really need them. **State cache** means we may drop unused states if a threshold has been reached and only keep those we used now (imagine the CPU cache). The last problem is that a DFA-based regex engine can't support the features that exceed the ability of **regular language**.

Another approach is to stay at the **NFA** stage. To simulate the **parallel** behaviors of a typical NFA, doing BFS is enough. You can use the real parallel mechanism in a mordern computer. But this method has worse theoretical time complexity than the DFA method.