#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  I #lambda("x", "e") gamma space rho space k =
  cases(
    k const(tru) sigma &"if" gamma = test,
    k const(void) sigma &"if" gamma = effect,
    I space e value rho_1 k_1 sigma_1 &"if" gamma = value "where" x "abbreviates" vars(x, null, s, l_x) \
      & space #box([
        $
          &x' "abbreviates" vars(x, null, s, l_x) space.quad space.quad x', l_x' "fresh" \
          &rho_1 = rho[x |-> x'] \
          &sigma_1 = sigma[l_x' |-> emptyset] \
          &k_1 = #sym.lambda space e' . space k #lambda($x'$, $e'$) \
        $
      ]),
    italic("fold") #lambda("x", "e") gamma space rho space k space.quad  &"if" gamma = app("op", gamma_1, l_gamma),
  )
$]