#import "cp0-common.typ": *

#set page(height: auto, width: auto, margin: 1pt)

#E[
$
  italic("result")(e) = cases(
    e_2 space.quad &"if" e = #seq($e_1 space$, $e_2$),
    e &"otherwise"
  )
$
]