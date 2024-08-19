#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#let fold = $italic("fold")$
#let op = $italic("op")$
#let visit = $italic("visit")$
#let result = $italic("result")$
#let copy = $italic("copy")$

#E[
  $
    copy(vars(x', op, s, l_x'), e, gamma, k, sigma) = cases(
      I const(c) space gamma space rho_0 space k space sigma
        &"if" e = const(c),
      k ref(x_1) sigma
        &"if" e = ref(x_1) "and" assignf in.not s_1 \
        &"where" x_1 "abbreviates" vars(x_1, op_1, s_1, l_x_1),
      fold space e space gamma space rho_0 space k space sigma
        &"if" e = app(op, gamma_1, l_gamma) \
        &space #box([$
          "and" &e = primref(p) \
          "or"  &e = lambda(x_1, e_1)
        $]),
      k primref(p) sigma
        &"if" gamma = value "and" e = primref(p),
      k const(tru) sigma
        &"if" gamma = test \ &#box([$
          "and" &e = primref(p)\
          "or"  &e = assign(x_1, e_1)\
          "or"  &e = lambda(x_1, e_1)\
        $]),
      k ref(x') sigma_1
        &"otherwise, where" sigma_1 = sigma[x' |-> {reff} union sigma(l_x')]
    )
  $
]