#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

/*
I(primref p)γρκ = κ(const true) if γ = Test κ(const void) if γ = Effect κ(primref p) if γ = Value  fold (primref p)γρκ if γ = App(op, γ1, lγ)
*/

#E[
$
  I #primref("p") gamma space rho space k =
  cases(
    k const(tru) &"if" gamma = test,
    k const(void) &"if" gamma = effect,
    k primref("p") &"if" gamma = value,
    italic("fold") primref("p") gamma space rho space k space.quad  &"if" gamma = app("op", gamma_1, l_gamma),
  )
$]