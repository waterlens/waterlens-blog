#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  italic("visit")(opnd(e, rho, l_e), gamma, k, sigma) = cases(
    I space e space gamma space rho space k_1 space sigma space.quad
      &"if" sigma(l_e) = unvisited,
      &"where" k_1 = #sym.lambda space e' space sigma_1 space . space k
        space e' space sigma_1[l_e |-> e'],
    k space e' space gamma
      &"otherwise, where" e' = sigma(l_e)
  )
$
]