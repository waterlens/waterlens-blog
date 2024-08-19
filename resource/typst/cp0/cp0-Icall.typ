#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  I #call($e_1 space$, $e_2$) gamma space rho space k space sigma &=
    I space e_1 space gamma_1 space rho space k_1 space sigma_1 "where" \
    & space.quad space.quad #box([
      $
        italic("op") &= opnd(e_2, rho, l_e_2) space.quad  space.quad & l_e_2 "fresh" \
        gamma_1      &= app(italic("op"), gamma, l_gamma_1)          & l_e_2 "fresh" \
        sigma_1      &= sigma[l_e_2 |-> unvisited, l_gamma_1 |-> emptyset] \
        k_1 space e_1 ' space sigma_2 &= cases(
          k space e_1 ' space sigma_2 &"if" inlined in sigma_2(l_gamma_1),
          italic("visit")(italic("op"), value, k_2, sigma_2) &"otherwise, where" \
            &k_2 = #sym.lambda space e_2 ' space. space k #call($e_1 ' space $, $e_2 '$) \
        )
      $
    ])
$]