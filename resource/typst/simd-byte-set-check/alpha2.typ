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
#let c3 = green

#let frames = (
  ("col", 0, 0, c1, "0b01"),
  ("row", 5, 5, c1, "0b01"),
  ("row", 7, 7, c1, "0b01"),

  ("col", 1, 0xB, c2, "0b10"),
  ("row", 4, 7, c2, "0b10           "),

  ("col", 0xc, 0xf, c3, "0b100"),
  ("row", 4, 4, c3, "0b100 "),
  ("row", 6, 6, c3, "0b100 "),
)

#let masks = (
  ((0, 5), (0, 5), c1.transparentize(85%)),
  ((0, 7), (0, 7), c1.transparentize(85%)),

  ((1, 4), (0xB, 7), c2.transparentize(85%)),

  ((0xc, 4), (0xf, 4), c3.transparentize(85%)),
  ((0xc, 6), (0xf, 6), c3.transparentize(85%)), 
)

#make-ascii-matrix(frames, masks)
