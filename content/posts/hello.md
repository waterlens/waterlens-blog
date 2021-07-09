---
title: "Hello World"
date: 2021-05-29T13:24:22+08:00
---

### Hello, World!

This is the first post of waterlens' blog.

The following code is a Y combinator in racket for testing syntax highlighting.

<pre><code class="language-racket match-braces rainbow-braces">
#lang racket
(define Y
  (lambda (f)
    ((lambda (x)
       (f (lambda (y) ((x x) y))))
     (lambda (x)
       (f (lambda (y) ((x x) y)))))))

(define Fact
  (Y
   (lambda (fact)
       (lambda (n)
         (if (zero? n)
             1
             (* n (fact (- n 1))))))))
(Fact 10)
</code></pre>
