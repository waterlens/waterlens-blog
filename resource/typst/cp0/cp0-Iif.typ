#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  I #iff($e_1 space$, $e_2 space$, $e_3$) gamma space rho space k &=
  I space e_1 test rho space k_1 \
  "where" space.quad k_1 space.med e_1 ' &= 
    cases(
      I space e_2 space gamma_1 space rho space
        (#sym.lambda space e_2 ' . space k #se (e_1 ' , e_2 '))
        "if" italic("result")(e_1 ') = const(tru),
      I space e_3 space gamma_1 space rho space
        (#sym.lambda space e_3 ' . space k #se (e_1 ' , e_3 '))
        "if" italic("result")(e_1 ') = const(fals),
      I space e_2 space gamma_1 space rho space
        (#sym.lambda space e_2 ' space . space I space e_3 space gamma_1 space rho space k_2)
        "otherwise, where",
        space.quad space.quad space.quad #box(
          $
            k_2 space.med e_3 '= cases(
              k #se (e_1 ', e_2 ')
                "if" italic("result")(e_1 ') = italic("result")(e_2 ') = const(c),
              k #iff($e_1 ' space$, $e_2 ' space$, $e_3 '$)
                "otherwise")
          $)) \
  gamma_1 &=
    cases(
      value "if" gamma = app(italic("op"), gamma_1, l_gamma),
      gamma "otherwise"
    )
$
]