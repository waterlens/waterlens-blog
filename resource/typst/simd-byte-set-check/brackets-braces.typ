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

#let color = red

#let frames = (
  ("col",11, 11, color, ""),
  ("col",13, 13, color, ""),
  ("row", 5, 5, color, ""),
  ("row", 7, 7, color, ""),
)

#let masks = (
  ((11, 5), (11, 5), color.transparentize(85%)),
  ((11, 7), (11, 7), color.transparentize(85%)),
  ((13, 5), (13, 5), color.transparentize(85%)),
  ((13, 7), (13, 7), color.transparentize(85%)),
)

#make-ascii-matrix(frames, masks)
