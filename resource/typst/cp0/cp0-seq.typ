#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  se(e_1, e_2) = cases(
    e_2 &"if" e_1 = const(void) "else" \
    seq(seq(e_1 space, e_3), e_4) &"if" e_2 = seq(e_3 space, e_4) \
    seq(e_1 space, e_2) &"otherwise"
  )
$
]