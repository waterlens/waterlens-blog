//! Rendering and partitioning of the document title.
//!
//! # Why this is not just `Document::doctitle`
//!
//! Asciidoctor applies the *title* substitution group to a document title —
//! the same steps a paragraph gets, so `= xref:.[Home]` becomes an anchor and
//! `= *Bold*` becomes `<strong>`. `asciidoc-parser` (0.29.x) applies only the
//! *header* group, which covers special characters, attribute references, and
//! the inline pass macro, but not quotes, macros, replacements, or post
//! replacements. [`Document::doctitle`] therefore hands back a title with its
//! macros still in source form.
//!
//! Applying the missing steps directly is not an option: the parser's
//! substitution pipeline is crate-private. So this module renders the title by
//! *reparsing its raw source as a one-paragraph document* and taking that
//! paragraph's rendered content, which runs the full normal substitution group
//! exactly once. The sub-parser is seeded with the document's own header
//! attributes so an attribute reference in the title still resolves.
//!
//! When the reparse cannot apply — a title supplied by a `:doctitle:` or
//! `:title:` attribute entry rather than the `= ` line, or source that does not
//! parse to a single paragraph — this falls back to the parser's own title,
//! which is what [`Document::doctitle`] would have given.

use asciidoc_parser::{
    Document, Parser,
    blocks::{FindBlocks, IsBlock},
    document::InterpretedValue,
    parser::ModificationContext,
};

use crate::{Options, html::sanitize};

/// A document title, rendered and partitioned into its parts.
pub(crate) struct Title {
    /// The portion of the title before the final subtitle separator — the
    /// whole title when there is no subtitle. Inline HTML.
    pub(crate) main: String,

    /// The portion after the final subtitle separator, if any. Inline HTML.
    pub(crate) subtitle: Option<String>,

    /// The full title with its markup stripped, for the `<title>` element.
    pub(crate) plain: String,
}

/// Renders `document`'s title.
///
/// A document without a title still has one for display purposes: the
/// converter asks Asciidoctor for the doctitle *with a fallback*, which yields
/// the `untitled-label` attribute (`Untitled` by default). So this returns a
/// title for every document.
pub(crate) fn render(document: &Document<'_>, options: &Options) -> Title {
    let Some(doctitle) = document.doctitle() else {
        let fallback = match document.attribute_value("untitled-label") {
            InterpretedValue::Value(label) => label,
            _ => "Untitled".to_string(),
        };
        return Title {
            plain: fallback.clone(),
            main: fallback,
            subtitle: None,
        };
    };

    // The reparse only applies to a title that came from the `= ` line. A
    // `:doctitle:`/`:title:` entry overrides it with an already-substituted
    // attribute value, which the parser reports through `doctitle` alone.
    let rendered = document
        .header()
        .title_source()
        .filter(|_| document.header().title() == Some(doctitle))
        .and_then(|source| render_inline(source.data(), document, options))
        .unwrap_or_else(|| doctitle.to_string());

    let (main, subtitle) = partition(&rendered, separator(document));

    Title {
        plain: sanitize(&rendered),
        main,
        subtitle,
    }
}

/// Renders `source` as inline AsciiDoc by parsing it as a one-paragraph
/// document under the same settings as `document`.
///
/// Returns `None` when the source does not parse to a single leading
/// paragraph — a title beginning with `.` or `[`, say, which would be read as
/// block metadata rather than text — so the caller can fall back.
fn render_inline(source: &str, document: &Document<'_>, options: &Options) -> Option<String> {
    let mut parser = options.apply(Parser::default());

    // Seed the document's own header attributes so an attribute reference in
    // the title resolves to the same value it would in the body. They are
    // document-overridable, matching how they were declared.
    for attribute in document.header().attributes() {
        let name = attribute.name().data();
        parser = match attribute.value() {
            InterpretedValue::Value(value) => {
                parser.with_intrinsic_attribute(name, value, ModificationContext::Anywhere)
            }
            InterpretedValue::Set => {
                parser.with_intrinsic_attribute_bool(name, true, ModificationContext::Anywhere)
            }
            InterpretedValue::Unset => {
                parser.with_intrinsic_attribute_bool(name, false, ModificationContext::Anywhere)
            }
        };
    }

    let parsed = parser.parse(source);
    let mut blocks = parsed.child_blocks();

    let first = blocks.next()?;
    if first.resolved_context().as_ref() != "paragraph" {
        return None;
    }

    Some(first.rendered_content()?.to_string())
}

/// The subtitle separator: the `title-separator` attribute (defaulting to a
/// colon) followed by a space.
fn separator(document: &Document<'_>) -> String {
    let separator = match document.attribute_value("title-separator") {
        InterpretedValue::Value(value) if !value.is_empty() => value,
        _ => ":".to_string(),
    };
    format!("{separator} ")
}

/// Splits `title` at its *final* separator, matching Asciidoctor's
/// `Document::Title`, which partitions from the end so only the last
/// occurrence counts.
fn partition(title: &str, separator: String) -> (String, Option<String>) {
    match title.rfind(&separator) {
        Some(index) => (
            title[..index].to_string(),
            Some(title[index + separator.len()..].to_string()),
        ),
        None => (title.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::{partition, render};
    use crate::{Options, load};

    #[test]
    fn renders_a_macro_in_the_title() {
        let doc = load("= xref:.[Home]\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "<a href=\".\">Home</a>");
        assert_eq!(title.subtitle, None);
        assert_eq!(title.plain, "Home");
    }

    #[test]
    fn renders_formatting_in_the_title() {
        let doc = load("= A *bold* title\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "A <strong>bold</strong> title");
    }

    #[test]
    fn partitions_a_subtitle_after_rendering() {
        let doc = load("= xref:.[Notes]: Volume One\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "<a href=\".\">Notes</a>");
        assert_eq!(title.subtitle.as_deref(), Some("Volume One"));
        assert_eq!(title.plain, "Notes: Volume One");
    }

    #[test]
    fn resolves_an_attribute_reference_in_the_title() {
        let doc = load("= The {project} Handbook\n:project: Widget\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "The Widget Handbook");
    }

    #[test]
    fn escapes_special_characters_exactly_once() {
        let doc = load("= Fish & Chips <tag>\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "Fish &amp; Chips &lt;tag&gt;");
    }

    #[test]
    fn falls_back_for_a_title_supplied_by_an_attribute_entry() {
        let doc = load("= Section Title\n:title: Override\n\nBody.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "Override");
    }

    // Asciidoctor's `use_fallback` gives a title-less document the
    // `untitled-label` value, which the converter then shows as its `<h1>`.
    #[test]
    fn falls_back_to_the_untitled_label_for_a_document_without_a_title() {
        let doc = load("Just a paragraph.");
        let title = render(&doc, &Options::new());
        assert_eq!(title.main, "Untitled");
        assert_eq!(title.plain, "Untitled");
        assert_eq!(title.subtitle, None);
    }

    #[test]
    fn partition_splits_at_the_final_separator() {
        assert_eq!(
            partition("a: b: c", ": ".to_string()),
            ("a: b".to_string(), Some("c".to_string()))
        );
        assert_eq!(
            partition("no separator", ": ".to_string()),
            ("no separator".to_string(), None)
        );
    }
}
