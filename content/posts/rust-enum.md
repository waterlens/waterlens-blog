---
title: "Enum Optimization in Rust"
date: 2021-07-09T11:54:39+08:00
tags:
  - rust
  - optimization
---

In the rust, the enum type is a powerful abstraction to represent a sort of different **variants** of the same type, like what the other languages have done.
It is also called **sum type** if we describe it as an ADT.
The origin idea of such a structure comes from **ALGOL 68**, but unluckily C didn't learn it well and many people just consider enum as a group of integer constants.
## General Implementation
Generally, we implement an enum type by the way called **tagged union**.
Let's start from the example:
<pre><code class="language-rust match-braces rainbow-braces">
enum Action {
    Quit,
    Print(&'static str),
    Draw{x: i32, y: i32},
}
</pre></code>
We can simulate it in C++ like this:
<pre><code class="language-cpp match-braces rainbow-braces">
enum class ActionKind {
  Quit,
  Print,
  Draw,
};

struct Action {
  ActionKind kind; // tag or discriminator
  union {
    const char * s;
    struct {
      int x; 
      int y;
    };
  };
};
</pre></code>
Hmmm, that just seems a little noisy.
So the WG21 introduced the `std::variant` since C++17. But To be frank, the visitor-style access is disgusting.
## Memory Layout
Let's forgot C++ and come back to Rust.
One of the most widely used enum in rust is `std::option::Option`, and its prototype is this:
<script type="text/plain" class="language-rust match-braces rainbow-braces">
enum Option<T> {
    None,
    Some(T),
}
</script>
And we may use it by the way like `Option<bool>`.
so what's its memory layout?
As the general tagged union approach described above, we guess it might be this:
```plaintext
+---------+----------+
| kind: 1 | value: 1 |
+---------+----------+
```
Do we really need 2x size just for an extra tag to indicate if a `bool` is contained?

But before complaining about the costy rust enum, we need check the size of `Option<bool>` to assure our guess is right.
<script type="text/plain" class="language-rust match-braces rainbow-braces">
fn main() {
    println!("{}", std::mem::size_of::<Option<bool>>());
}
</script>
And Rust Playground gives following executing result:
```plaintext
Compiling playground v0.0.1 (/playground)
  Finished dev [unoptimized + debuginfo] target(s) in 2.59s
    Running `target/debug/playground`
1
```
Fantastic! 
1 byte only!
And the question comes that what optimizations the `rustc` have done to reduce the size of space an enum type really occupies?

In fact, such an optimization called **smart enum optimization** is universal.
Besides, there is already a similar and earlier ad-hoc optimization called **null pointer optimization**.
Here I'd like to illustrate both.
## Null Pointer Optimization
The effect has been described in [here](https://doc.rust-lang.org/std/option/index.html#representation), and the main topic is:

Rust guarantees to optimize the following types T such that `Option<T>` has the same size as T:
- `Box<U>`
- `&U`
- `&mut U`
- `fn`, `extern "C" fn`
- `num::NonZero*`
- `ptr::NonNull<U>`
- `#[repr(transparent)]` struct around one of the types in this list.

or rather, for those types which have an integer bottom type and their value can't be zero, if we use `Option<T>` to wrap it, there is no need to place an extra tag because we can use the unused bit pattern -- zero, to indicate `Option<T>` contains `None`. Notice that zero only has one bit pattern, so the tag must be a **unit** variant.

## Smart Enum Optimization
Is **null pointer optimization** enough? No.
In many cases, the conditions don't satisfy the need for NPO.
For example, `Option<bool>` will still occupy 2 bytes rather than 1 byte, because `bool` does use the zero pattern.
Apart from that, NPO takes only a zero bit pattern to store the tag, and the enums having more complicated variants fail to be optimized.

Thus the developers use a more general means. 
After this [refactor](https://github.com/rust-lang/rust/pull/45225), things become better by **niche filling**.
The compiler will collet unused bit patterns of one of the variants and decide if they can be used to indicate another variant.
The implementation details are ignored here and the effect of SEO is obvious.

Now the `Option<Option<Option<bool>>>` will keep the size of 1 byte.
And `Option<char>` only takes 4 bytes because Unicode only uses 21 bits.
More specially, the following enum, `enum A { B(bool), C, D, E }` only uses 1 byte.