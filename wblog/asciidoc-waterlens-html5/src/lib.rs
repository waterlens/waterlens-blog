//! Waterlens' HTML5 backend for [AsciiDoc](https://asciidoc.org).
//!
//! This crate converts AsciiDoc source (as parsed by [`asciidoc_parser`]) into
//! the HTML the waterlens blog is built from. It is a Rust port of the
//! `w-html` Asciidoctor converter that previously lived in
//! `tools/asciidoc/convert.rb`, and it replaces the Ruby toolchain entirely.
//!
//! It is a *sibling* of Asciidoctor's stock `html5` backend rather than a
//! customization of it. What differs:
//!
//! - **A bespoke page shell.** Web fonts, the site stylesheet, KaTeX (for a
//!   document that enables `stem`), Mermaid, a site navigation bar under the
//!   `shownav` attribute, and a Creative Commons licence footer.
//! - **Shorter wrapper class names.** `listing`, `literal`, `stem`, `example`,
//!   `open`, `sidebar`, `verse`, and `admonition` in place of Asciidoctor's
//!   `listingblock`, `literalblock`, and friends; tables use `table` rather
//!   than `tableblock` and are wrapped in a scrollable `table-wrapper`.
//! - **Bare paragraphs.** An unadorned paragraph is a `<p>`, not a
//!   `<div class="paragraph"><p>…</p></div>`.
//! - **`rem` image dimensions.** A `width`/`height` given in `rem` becomes an
//!   inline `style` rather than an HTML attribute, so diagrams scale with the
//!   text.
//! - **CJK line-break collapsing.** A source line break between two CJK
//!   characters is deleted rather than rendered as a space.
//! - **`<section>` elements.** Sections nest as `<section class="sectN">`
//!   rather than `<div class="sectN">` with a `sectionbody` child.
//!
//! Inline substitution is the parser's job: `asciidoc-parser` applies quotes,
//! replacements, macros, cross references, and attribute references eagerly at
//! parse time, so every block's content and title reaches this crate as an
//! Asciidoctor-compatible inline HTML fragment. This crate only assembles the
//! block structure around those fragments.
//!
//! # Examples
//!
//! Render a complete page from a file on disk:
//!
//! ```no_run
//! use asciidoc_waterlens_html5::{Options, convert_file_with};
//!
//! let html = convert_file_with("post.adoc", &Options::new().standalone(true))?;
//! println!("{html}");
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! Or *load* a document, inspect it, and render it separately:
//!
//! ```
//! let doc = asciidoc_waterlens_html5::load("= Hello\n\nWorld.");
//! assert_eq!(doc.doctitle(), Some("Hello"));
//! let html = asciidoc_waterlens_html5::convert_document(&doc);
//! assert!(html.contains("<p>World.</p>"));
//! ```

use std::{fs, io, path::Path};

use asciidoc_parser::Parser;

mod cjk;
mod html;
mod include;
mod options;
mod path;
mod renderer;
mod title;

pub use asciidoc_parser::{Document, ReferenceTime, SafeMode};
pub use options::Options;

#[cfg(test)]
mod tests;

/// Parses `source` as AsciiDoc and renders it to *embedded*, body-only HTML.
///
/// To render a complete page from a string, use [`convert_with`] under
/// [`Options::standalone(true)`](Options::standalone); the file entry point
/// [`convert_file`] is standalone by default.
pub fn convert(source: &str) -> String {
    convert_with(source, &Options::default())
}

/// Parses `source` as AsciiDoc and renders it under `options`.
pub fn convert_with(source: &str, options: &Options) -> String {
    let document = load_with(source, options);
    convert_document_with(&document, options)
}

/// Reads the AsciiDoc file at `path` and renders it to a complete HTML page.
///
/// # Errors
///
/// Returns the [`io::Error`] from reading `path` — for example, when the file
/// does not exist or does not contain valid UTF-8.
pub fn convert_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    convert_file_with(path, &Options::default().standalone(true))
}

/// Reads the AsciiDoc file at `path` and renders it under `options`.
///
/// The `path` is recorded as the primary document, so its `include::`
/// directives resolve against the file's own directory.
///
/// # Errors
///
/// Returns the [`io::Error`] from reading `path`.
pub fn convert_file_with<P: AsRef<Path>>(path: P, options: &Options) -> io::Result<String> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(convert_with(&source, &options.clone().input_file(path)))
}

/// Parses `source` into a [`Document`] without rendering it.
///
/// The returned document is fully owned (`Document<'static>`): the parser
/// copies what it needs from `source`, so the document does not borrow from it
/// and can be returned, stored, or moved freely.
pub fn load(source: &str) -> Document<'static> {
    load_with(source, &Options::default())
}

/// Parses `source` into a [`Document`] under `options`.
///
/// The source is normalized first — see [`normalize_source`].
pub fn load_with(source: &str, options: &Options) -> Document<'static> {
    let mut parser = options.apply(Parser::default());
    parser.parse(&normalize_source(source))
}

/// Strips trailing whitespace from every line of `source`.
///
/// Asciidoctor's preprocessing reader does this to every line it reads, so
/// trailing spaces in the source never reach the converter — not in a
/// paragraph, not inside a listing or STEM block. `asciidoc-parser` hands the
/// source through unchanged, so without this step a stray trailing space in a
/// verbatim block would survive into the rendered `<pre>`, and a line ending
/// in `" +"` would be a hard line break to Asciidoctor but literal text here.
///
/// Normalizing at the point the source enters this crate keeps every entry
/// point consistent, rather than leaving each caller to remember.
fn normalize_source(source: &str) -> String {
    // A source with no trailing whitespace to strip is the common case, and
    // the borrow-free early return keeps it allocation-light.
    if !source.lines().any(|line| line.trim_end() != line) {
        return source.to_string();
    }

    let mut normalized = String::with_capacity(source.len());
    for (index, line) in source.split('\n').enumerate() {
        if index > 0 {
            normalized.push('\n');
        }
        normalized.push_str(line.trim_end());
    }
    normalized
}

/// Reads the AsciiDoc file at `path` and parses it into a [`Document`].
///
/// # Errors
///
/// Returns the [`io::Error`] from reading `path`.
pub fn load_file<P: AsRef<Path>>(path: P) -> io::Result<Document<'static>> {
    load_file_with(path, &Options::default())
}

/// Reads the AsciiDoc file at `path` and parses it under `options`.
///
/// # Errors
///
/// Returns the [`io::Error`] from reading `path`.
pub fn load_file_with<P: AsRef<Path>>(path: P, options: &Options) -> io::Result<Document<'static>> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(load_with(&source, &options.clone().input_file(path)))
}

/// Renders an already-parsed [`Document`] to body-only HTML.
pub fn convert_document(document: &Document<'_>) -> String {
    renderer::render_document(document, &Options::default())
}

/// Renders an already-parsed [`Document`] under `options`.
pub fn convert_document_with(document: &Document<'_>, options: &Options) -> String {
    renderer::render_document(document, options)
}
