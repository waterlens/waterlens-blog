#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  I #assign($x space$ , $e$) gamma space rho space k space sigma =
  cases(
    I space e effect gamma space rho space k space sigma
      "if" rho(x) = vars(x', italic("op"), s, l_x') "and" reff in.not s,
    I space e value gamma space rho space k_1 space sigma
      "if" rho(x) = vars(x', italic("op"), s, l_x') "and" reff in s "where" \
      space.quad space.quad space.quad space.quad #box(
        [$
          & k_1 space e' space sigma_1 = k #se (assign(x' space, e'), const(c)) sigma_2 \
          & sigma_2 = sigma_1[l_x' |-> { #assignf } union sigma_1 (l_x')] \
          & c = tru "if" gamma = test "else" c' = void \
        $])
  )
$]