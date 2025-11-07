#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  I #ref("x") gamma space rho space k =

  cases(
    k const(void) sigma &"if" gamma = effect,
    k ref(x') sigma_1   &"if" italic("op") = null "or" assignf in s,
                        &#box(
                          $
                            "where" &rho(x) = vars(x', italic("op"), s, l_x') \
                                    &sigma_1 = sigma[l_x' |-> {reff} union sigma(l_x')]
                          $
                        ),
   italic("visit")(italic("op"), value, k_1, sigma) space.quad &"otherwise, where" rho(x) = vars(x', italic("op"), s, l_x'),
     &"and" k_1 = #sym.lambda space e space sigma_2 space . italic("copy") (rho(x), italic("result")(e), gamma, k, sigma_2)
  )
$
]