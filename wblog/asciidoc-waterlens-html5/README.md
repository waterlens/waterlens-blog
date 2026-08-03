# asciidoc-waterlens-html5

Waterlens' HTML5 backend for AsciiDoc, built on
[`asciidoc-parser`](https://github.com/asciidoc-rs/asciidoc-parser).

This is a Rust port of the `w-html` Asciidoctor converter that used to live in
`tools/asciidoc/convert.rb` (plus the `hljs.rb` highlighter adapter). It exists
so the blog can be built without a Ruby toolchain.

It is a *sibling* of Asciidoctor's stock `html5` backend rather than a
customization of it, and its structure follows
[`asciidoc-html5`](https://github.com/asciidoc-rs/asciidoc-html5): one
`Renderer` method per structural concern, dispatched from `Renderer::block`,
recursing through `FindBlocks::child_blocks` for compound blocks. Inline
substitution is the parser's job — every block's content and title arrives as
an Asciidoctor-compatible HTML fragment, and this crate only assembles block
structure around it.

## How the output differs from the stock `html5` backend

| Concern | Stock `html5` | This backend |
| --- | --- | --- |
| Page shell | Asciidoctor's default CSS, header, footer | Site fonts, `/style.css`, KaTeX under `stem`, Mermaid, `shownav` navigation, CC licence footer |
| Plain paragraph | `<div class="paragraph"><p>…</p></div>` | `<p>…</p>` |
| Sections | `<div class="sectN">` + `<div class="sectionbody">` | `<section class="sectN">`, no section body |
| Verbatim / wrappers | `listingblock`, `literalblock`, `stemblock`, `exampleblock`, `openblock`, `sidebarblock`, `verseblock`, `admonitionblock` | `listing`, `literal`, `stem`, `example`, `open`, `sidebar`, `verse`, `admonition` |
| Tables | `tableblock` classes | `table` classes, wrapped in `<div class="table-wrapper">` for horizontal scrolling |
| Image dimensions | `width`/`height` attributes only | a `rem` value becomes an inline `style` so diagrams scale with the text |
| CJK text | a source line break renders as a space | a line break between CJK characters is deleted (see `src/cjk.rs`) |
| Video | an embed | renders nothing (`convert_video` is empty in the original) |

Two deliberate departures from the Ruby original:

- **The generator tag.** It reads `Waterlens HTML Backend <version>` instead of
  naming Asciidoctor, which is no longer involved.
- **The footer year.** It comes from the `localyear` document attribute rather
  than the wall clock, so pinning a reference time (or `SOURCE_DATE_EPOCH`)
  makes a build reproducible.

## Verifying parity

`wblog-asciidoc` ships an example that renders a tree of documents, which is
how this port was checked against the Ruby converter:

```sh
cargo run -p wblog-asciidoc --example render_tree -- content /tmp/rust

for f in $(cd content && find . -name '*.adoc'); do
  asciidoctor -b w-html -r ./tools/asciidoc/convert.rb -r ./tools/asciidoc/hljs.rb \
    "content/$f" -o "/tmp/ruby/${f%.adoc}.html"
done

diff -r /tmp/ruby /tmp/rust
```

(Whitespace and the generator `<meta>` tag are expected to differ.)

## Constructs where `asciidoc-parser` differs from Asciidoctor

Reaching full parity took five source edits, because these five constructs are
read differently by `asciidoc-parser` 0.29.6 and no converter can compensate —
they are decided during parsing, before a backend sees anything. If you write
one of these shapes again, the two toolchains will disagree:

1. **A paragraph directly after a delimited block inside a nested list** needs
   an explicit `+` continuation. Asciidoctor attaches it to the item without
   one; the parser closes the list and splits it in two.
2. **A trailing `+` at the very end of a list item's text** stays literal
   instead of becoming a hard line break. The same `+` mid-text works.
3. **A list item's wrapped continuation lines** should not be indented past
   the item marker. Asciidoctor strips that indent; the parser keeps it, and
   the spaces reach the `<p>`.
4. **Colliding auto-generated section ids** are numbered in the wrong order:
   given `== DP` then `=== ??? DP`, Asciidoctor assigns `_dp` then `_dp_2`,
   the parser the other way around. Pin them with an explicit `[#id]`.
5. **Description-list terms are not fully substituted.** The parser applies
   only the *macros* step to a `term::` label, so `` `code` `` in a term stays
   as literal backticks. (The parser notes a pending parse/substitute split in
   its issue #461.)

Note that #5 only shows up in text that became a description list *by
accident*: a `::` followed by a space anywhere in a paragraph is a
description-list marker, and backticks do **not** protect it — the block is
identified before any inline substitution runs, so neither `` `x \:: α` `` nor
`` pass:[`x :: α`] `` helps. Break the `::`-plus-space sequence in the raw
line instead: `` `x pass:[::] α` `` renders as `<code>x :: α</code>`.

The title handling in `src/title.rs` works around a sixth gap — the parser
applies only *header* substitutions to the document title, so `= xref:.[Home]`
would keep its macro in source form. That one is worked around by reparsing the
raw title as a one-paragraph document; see the module docs for why.

Two further parser gaps are *partially* repaired in the renderer, with the
residuals documented here:

- **An attribute-decorated list after a nested list** (`* a`, `** nested`,
  blank line, `[.foo]`, `* c`) is merged by the parser into the previous
  list, with the attributes hung on the merged segment's first item. The
  ulist/olist renderers split the list back out at that item, restoring the
  wrapper attributes (`id`, roles, style, `start`, `reversed`). A segment
  whose markers differ from the list's own (`[.foo]` before `. c` inside a
  `*` list) is re-typed from its markers. One residual remains: the split
  list stays at the nesting depth the parser merged it into, while
  Asciidoctor hoists it to the top level — a cosmetic indentation difference
  in a rare shape. A description list merged the same way (`[.foo]` before
  `t:: d`) loses its `::` structure in the parse and cannot be repaired.
- **Inline attribute-set directives** (`{set:name:value}`, which set an
  attribute mid-document for later `{name}` references) are not processed by
  the parser, so the directive text reaches the output literally. This cannot
  be fixed in the renderer, because the parser has already substituted (or
  failed to substitute) the surrounding text.
