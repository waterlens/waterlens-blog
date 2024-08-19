#import "@preview/simplebnf:0.1.1": *
#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)
/*
e ::= (const c) | (ref x) | (primref p) | (if e1 e2 e3) | (seq e1 e2) | (assign x e) | (lambda (x) e) | (letrec ((x1 e1) . . . (xn en)) e) | (call e0 e1)
*/

#bnf(
  Prod(
    $e$,
    {
      Or[#const($c$)][]
      Or[#ref($x$)][]
      Or[#primref($p$)][]
      Or[#iff($e_1$, $e_2$, $e_3$)][]
      Or[#seq($e_1$, $e_2$)][]
      Or[#assign($x$, $e$)][]
      Or[#lambda($x$, $e$)][]
      Or[#letrec($x_1$, $e_1$, $x_n$, $e_n$, $e$)][]
      Or[#call($e_0$, $e_1$)][]
    },
  ),
)
