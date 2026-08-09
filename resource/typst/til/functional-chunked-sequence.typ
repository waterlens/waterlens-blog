#import "@preview/cetz:0.4.1"

#set page(width: auto, height: auto, margin: 2pt)

#let cell = 0.64
#let edge = 1.05pt
#let dot-radius = 0.075
#let arrow-length = 0.40
#let arrow-width = 0.36

#cetz.canvas({
  import cetz.draw: *

  let arrow(start, tip, direction) = {
    let sx = tip.at(0)
    let sy = tip.at(1)

    if direction == "left" {
      line(start, (sx + arrow-length, sy), stroke: edge)
      line(
        tip,
        (sx + arrow-length, sy + arrow-width / 2),
        (sx + arrow-length, sy - arrow-width / 2),
        close: true,
        fill: black,
        stroke: none,
      )
    } else if direction == "right" {
      line(start, (sx - arrow-length, sy), stroke: edge)
      line(
        tip,
        (sx - arrow-length, sy + arrow-width / 2),
        (sx - arrow-length, sy - arrow-width / 2),
        close: true,
        fill: black,
        stroke: none,
      )
    } else if direction == "down" {
      line(start, (sx, sy + arrow-length), stroke: edge)
      line(
        tip,
        (sx - arrow-width / 2, sy + arrow-length),
        (sx + arrow-width / 2, sy + arrow-length),
        close: true,
        fill: black,
        stroke: none,
      )
    }
  }

  let null-marker(center) = {
    let x = center.at(0)
    let y = center.at(1)
    let radius = 0.145
    circle(center, radius: radius, fill: white, stroke: 0.8pt)
    line(
      (x - radius * 0.78, y - radius * 0.78),
      (x + radius * 0.78, y + radius * 0.78),
      stroke: 0.8pt,
    )
  }

  // x is the left edge; y is the vertical center.
  let horizontal-array(x, y, count: 4, filled: (), null-at: none) = {
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

    if null-at != none {
      null-marker((x + (null-at + 0.5) * cell, y))
    }
  }

  // x is the horizontal center; y is the top edge.
  let vertical-array(x, y, count: 4, filled: ()) = {
    rect(
      (x - cell / 2, y),
      (x + cell / 2, y - count * cell),
      fill: white,
      stroke: edge,
    )

    for i in range(1, count) {
      line(
        (x - cell / 2, y - i * cell),
        (x + cell / 2, y - i * cell),
        stroke: edge,
      )
    }

    for i in filled {
      circle(
        (x, y - (i + 0.5) * cell),
        radius: dot-radius,
        fill: black,
        stroke: none,
      )
    }
  }

  let top-y = 0
  let middle-y = -2.29
  let bottom-y = -6.87
  let left-x = -5.12
  let right-x = 2.59
  let hub-x = -1.5 * cell
  let child-top = -3.64
  let left-child-xs = (
    left-x + 0.5 * cell - 0.12,
    left-x + 1.5 * cell,
    left-x + 2.5 * cell + 0.12,
  )
  let lower-child-top = -8.22
  let lower-child-x = left-x + 0.5 * cell
  let row-x = -2.83

  // Draw edges first so that array borders and pointer dots sit on top.
  arrow((0, 1.22), (0, cell / 2 + 0.10), "down")

  for y in (top-y, middle-y, bottom-y) {
    arrow((-cell, y), (left-x + 4 * cell + 0.12, y), "left")
    arrow((cell, y), (right-x - 0.12, y), "right")
  }

  arrow((0, top-y), (0, middle-y + cell / 2 + 0.12), "down")
  arrow((0, middle-y), (0, bottom-y + cell / 2 + 0.12), "down")

  for i in range(0, 3) {
    let x = left-x + (i + 0.5) * cell
    arrow((x, middle-y), (x, child-top + 0.14), "down")
  }

  let right-child-x = right-x + 0.5 * cell
  arrow(
    (right-child-x, middle-y),
    (right-child-x, child-top + 0.14),
    "down",
  )

  arrow(
    (lower-child-x, bottom-y),
    (lower-child-x, lower-child-top + 0.14),
    "down",
  )

  for i in range(0, 4) {
    let y = lower-child-top - (i + 0.5) * cell
    arrow((lower-child-x, y), (row-x - 0.12, y), "right")
  }

  // Spine nodes and their left/right chunks.
  horizontal-array(hub-x, top-y, count: 3, filled: (0, 1, 2))
  horizontal-array(left-x, top-y, filled: (0, 1))
  horizontal-array(right-x, top-y, filled: (0, 1, 2))

  horizontal-array(hub-x, middle-y, count: 3, filled: (0, 1, 2))
  horizontal-array(left-x, middle-y, filled: (0, 1, 2))
  horizontal-array(right-x, middle-y, filled: (0,))

  horizontal-array(
    hub-x,
    bottom-y,
    count: 3,
    filled: (0, 2),
    null-at: 1,
  )
  horizontal-array(left-x, bottom-y, filled: (0,))
  horizontal-array(right-x, bottom-y)

  // Chunks hanging below the middle level.
  for i in range(0, 3) {
    vertical-array(left-child-xs.at(i), child-top, filled: (0, 1, 2, 3))
  }
  vertical-array(right-child-x, child-top, filled: (0, 1, 2, 3))

  // The lower-left chunk points to four full chunks.
  vertical-array(
    lower-child-x,
    lower-child-top,
    filled: (0, 1, 2, 3),
  )

  for i in range(0, 4) {
    let row-y = lower-child-top - 0.22 - i * (cell + 0.08)
    horizontal-array(row-x, row-y, filled: (0, 1, 2, 3))
  }
})
