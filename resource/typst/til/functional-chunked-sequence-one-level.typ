#import "@preview/cetz:0.4.1"

#set page(width: auto, height: auto, margin: 2pt)
#set text(font: "New Computer Modern", size: 11pt)

#let cell = 0.70
#let edge = 1.05pt
#let dot-radius = 0.075
#let arrow-length = 0.46
#let arrow-width = 0.44

#cetz.canvas({
  import cetz.draw: *

  let arrow(start, tip, direction) = {
    let x = tip.at(0)
    let y = tip.at(1)

    if direction == "left" {
      line(start, (x + arrow-length, y), stroke: edge)
      line(
        tip,
        (x + arrow-length, y + arrow-width / 2),
        (x + arrow-length, y - arrow-width / 2),
        close: true,
        fill: black,
        stroke: none,
      )
    } else if direction == "right" {
      line(start, (x - arrow-length, y), stroke: edge)
      line(
        tip,
        (x - arrow-length, y + arrow-width / 2),
        (x - arrow-length, y - arrow-width / 2),
        close: true,
        fill: black,
        stroke: none,
      )
    } else if direction == "down" {
      line(start, (x, y + arrow-length), stroke: edge)
      line(
        tip,
        (x - arrow-width / 2, y + arrow-length),
        (x + arrow-width / 2, y + arrow-length),
        close: true,
        fill: black,
        stroke: none,
      )
    }
  }

  // x is the left edge; y is the vertical center.
  let horizontal-array(x, y, count: 4, filled: ()) = {
    rect(
      (x, y - cell / 2),
      (x + count * cell, y + cell / 2),
      fill: white,
      stroke: edge,
    )

    for i in range(1, count) {
      line(
        (x + i * cell, y - cell / 2),
        (x + i * cell, y + cell / 2),
        stroke: edge,
      )
    }

    for i in filled {
      circle(
        (x + (i + 0.5) * cell, y),
        radius: dot-radius,
        fill: black,
        stroke: none,
      )
    }
  }

  let front-x = -5.56
  let back-x = 2.74
  let hub-x = -1.5 * cell
  let chunk-y = -2.38
  let middle-xs = (-5.94, -2.94, 0.10, 3.13)

  // Pointers are drawn first so their tails disappear beneath the dots.
  arrow((0, 1.56), (0, cell / 2 + 0.16), "down")
  arrow((-cell, 0), (front-x + 4 * cell + 0.05, 0), "left")
  arrow((cell, 0), (back-x - 0.05, 0), "right")
  arrow((0, 0), (0, -1.35), "down")

  // The brace denotes the sequence stored in Middle.
  bezier(
    (-5.94, -1.93),
    (-4.60, -1.60),
    (-5.93, -1.70),
    (-5.35, -1.60),
    stroke: 0.65pt,
  )
  line(
    (-4.60, -1.60),
    (-0.15, -1.60),
    (0, -1.40),
    (0.15, -1.60),
    (4.60, -1.60),
    stroke: 0.65pt,
  )
  bezier(
    (4.60, -1.60),
    (5.94, -1.93),
    (5.35, -1.60),
    (5.93, -1.70),
    stroke: 0.65pt,
  )

  horizontal-array(hub-x, 0, count: 3, filled: (0, 1, 2))
  horizontal-array(front-x, 0, filled: (1, 2, 3))
  horizontal-array(back-x, 0, filled: (0, 1))

  horizontal-array(middle-xs.at(0), chunk-y, filled: (1, 2, 3))
  horizontal-array(middle-xs.at(1), chunk-y, filled: (1, 2))
  horizontal-array(middle-xs.at(2), chunk-y, filled: (0, 1, 2, 3))
  horizontal-array(middle-xs.at(3), chunk-y, filled: (2,))

  content(
    (-5.86, 1.47),
    anchor: "west",
    text(size: 11pt, weight: "bold")[One level],
  )
  content((-4.16, 0.79), text(size: 11pt)[Front])
  content((4.14, 0.79), text(size: 11pt)[Back])
  content((-1.18, -1.06), text(size: 11pt)[Middle])
})
