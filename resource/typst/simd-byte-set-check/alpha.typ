#import "@preview/asciim:0.1.0": make-ascii-matrix

#set text(
  font: (
    (name: "Arial Unicode MS", covers: regex("[\u2400-\u243f]")),
    "Hack Nerd Font Mono",
  ),
  size: 10pt,
  fallback: true,
)

#set page(width: 80em, height: auto, margin: 1em)
#set align(center)

#let c1 = red
#let c2 = blue

#let frames = (
  ("col", 0, 10, c1, "0b01"),
  ("row", 5, 5, c1, "0b01"),
  ("row", 7, 7, c1, "0b01"),

  ("col", 1, 15, c2, "0b10"),
  ("row", 4, 4, c2, "0b10"),
  ("row", 6, 6, c2, "0b10"),
)

#let masks = (
  ((0, 5), (10, 5), c1.transparentize(85%)),
  ((0, 7), (10, 7), c1.transparentize(85%)),

  ((1, 4), (15, 4), c2.transparentize(85%)),
  ((1, 6), (15, 6), c2.transparentize(85%)),
)

#make-ascii-matrix(frames, masks)
