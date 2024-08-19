#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#let fold = $italic("fold")$
#let op = $italic("op")$
#let visit = $italic("visit")$
#let result = $italic("result")$

#E[
$
  fold &primref(p) app(op, gamma_1, l_gamma) space rho space k space sigma = visit(op, value, k_1, sigma) \
    &#box([
      $
        "where" k_1 space e_1' space rho_1 = cases(
          k const(c') sigma_2 &"if" result(e_1') = const(c) "and" p(c) = c' \
                              &"where" sigma_2 = sigma_1[l_gamma |-> { inlined } union sigma_1(l_gamma)],
          k primref(p) sigma_1 space.quad &"otherwise"
        )
      $
    ]) \ \ \
  fold &lambda(x, e) app(op, gamma_1, l_gamma) space rho space k space sigma =
    I space e space gamma_1 space rho_1 space k_1 space sigma_1 \
      &#box([
        $
          "where" space.quad x &space.quad&&"abbreviates" vars(x, null, s, l_x) \
                             x' &space.quad&&"abbreviates" vars(x', op, sigma(l_x), l_x') space.quad space.quad x', l_x' "fresh" \
                             rho_1 &&&= rho[x |-> x'] \
                             sigma_1 &&&= sigma[l_x' |-> emptyset] \
                             k_1 space e' sigma_2 &&& = 
                                cases(
                                  visit(op, effect, k_2, sigma_2) space.quad &"if" reff in.not sigma_2(l_x') "and" assignf in.not sigma_2(l_x'),
                                  visit(op, effect, k_3, sigma_2) &"if" reff in.not sigma_2(l_x') "and" assignf in sigma_2(l_x'),
                                  visit(op, value, k_3, sigma_2) &"otherwise"
                                ) \
                            k_2 &&&= #sym.lambda e_1 ' space sigma_3 space . space k #se (e_1 ' space, e') space sigma_3[l_gamma |-> { inlined } union sigma_3(l_gamma)] \
                            k_3 &&&= #sym.lambda e_1 ' space sigma_3 space . space k #call(lambda($x' space$, $e'$), $e_1 '$) sigma_3[l_gamma |-> { inlined } union sigma_3(l_gamma)]

        $
      ])

$
]