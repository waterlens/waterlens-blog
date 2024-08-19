#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)
/*
I(const c)γρκ = κ(const void) if γ = Effect κ(const true) if γ = Test and c 6= false κ(const c) otherwise
*/

#E[
$
  I #const("c") gamma space rho space k =
  cases(
    k const(void) space.quad &"if" gamma = effect,
    k const(tru) &"if" gamma = test "and" c != fals,
    k const(c) &"otherwise",
  )
$]