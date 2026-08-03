//! The block-structure walker that turns a parsed [`Document`] into Waterlens
//! HTML.
//!
//! # How the walk works
//!
//! The parser applies *inline* substitutions eagerly: by the time we hold a
//! [`Document`], every block's content and title is already an
//! Asciidoctor-compatible HTML *fragment* (with `<strong>`, `<a href>`,
//! escaped special characters, and so on). This crate therefore never parses
//! inline markup itself — its whole job is to wrap those fragments in the
//! block-level scaffolding this backend emits, in document order.
//!
//! [`Renderer`] holds the output buffer and exposes one method per structural
//! concern. [`Renderer::block`] is the dispatch point: it drops comment blocks,
//! then matches on the [`Block`] variant (and, for delimited blocks, on
//! [`IsBlock::resolved_context`]) and delegates. Compound blocks recurse back
//! into [`Renderer::blocks`] over their [`FindBlocks::child_blocks`], so the
//! same machinery handles arbitrary nesting.
//!
//! # Relationship to the `html5` backend
//!
//! This is a *sibling* of Asciidoctor's stock `html5` backend, not a
//! customization layer over it. The differences are pervasive and deliberate:
//! a bespoke document shell (fonts, KaTeX, Mermaid, navigation, licence
//! footer), shorter wrapper class names (`listing` rather than `listingblock`),
//! bare `<p>` paragraphs, `rem` image dimensions promoted to inline styles, and
//! CJK line-break collapsing. See `README.md` for the full list.

use std::path::Path;

use asciidoc_parser::{
    Document, HasSpan, SafeMode,
    attributes::Attrlist,
    blocks::{
        AdmonitionBlock, Block, Break, BreakType, ColumnStyle, CompoundDelimitedContext,
        ContentModel, FindBlocks, Frame, Grid, HorizontalAlignment, IsBlock, ListBlock, ListItem,
        ListItemMarker, ListType, MediaBlock, MediaType, QuoteBlock, QuoteType, SectionBlock,
        SectionType, SimpleBlockStyle, Stripes, TableBlock, TableCell, TableCellContent,
        TableColumn, TableRow, VerticalAlignment,
    },
    document::InterpretedValue,
};

use crate::{
    Options,
    cjk::collapse_cjk_newlines,
    html::{class_attribute, class_list, escape_attribute, escape_text, id_attribute},
    path::{FileResolver, decode_document_attribute, looks_like_uri, resolve_web_path},
    title::{self, Title},
};

/// This backend's own version, reported in the generator `<meta>` tag.
const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The `highlight.js` release the `hljs` syntax highlighter adapter links from
/// the CDN.
const HIGHLIGHT_JS_VERSION: &str = "11.11.1";

/// The default `highlight.js` colour scheme, used when the document sets no
/// `highlightjs-theme`.
const DEFAULT_HIGHLIGHTJS_THEME: &str = "atom-one-light";

/// Reads a document attribute's value, returning an empty string for a
/// value-less set attribute and `None` only when it is unset.
pub(crate) fn attribute_str(document: &Document<'_>, name: &str) -> Option<String> {
    match document.attribute_value(name) {
        InterpretedValue::Value(value) => Some(value),
        InterpretedValue::Set => Some(String::new()),
        InterpretedValue::Unset => None,
    }
}

/// Reads a document attribute's value, falling back to `default` when it is
/// unset — Asciidoctor's `node.attr 'name', 'default'`.
fn attribute_or(document: &Document<'_>, name: &str, default: &str) -> String {
    attribute_str(document, name).unwrap_or_else(|| default.to_string())
}

/// The `asset-uri-scheme` document attribute, normalized to a `scheme:`
/// prefix — empty when the attribute is set to an empty value, so the
/// `scheme://` URL it prefixes stays well-formed.
fn asset_uri_scheme(document: &Document<'_>) -> String {
    match attribute_or(document, "asset-uri-scheme", "https") {
        scheme if scheme.is_empty() => String::new(),
        scheme => format!("{scheme}:"),
    }
}

/// Escapes the double quotes of a *document-attribute* value destined for a
/// quoted HTML attribute.
///
/// The parser substitutes such values ahead of time (special characters,
/// attribute references), so the `&`, `<`, and `>` are already escaped and
/// must not be touched again — that is the double-escaping bug from the meta
/// tags. Quotes, however, are not part of the substitution's escape set, so
/// they are handled here, keeping the attribute well-formed.
fn escape_quotes(value: &str) -> String {
    value.replace('"', "&quot;")
}

/// The active *client-side* syntax highlighter, resolved from the
/// `source-highlighter` document attribute.
///
/// Only `highlight.js` is modelled, because it is the only highlighter this
/// backend's `hljs` adapter registers. It performs no server-side tokenizing,
/// so honoring it is purely a matter of the CSS classes on the source block's
/// `<pre>`/`<code>` plus a `<link>`/`<script>` pair after the footer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Highlighter {
    /// `:source-highlighter: highlightjs` (also `highlight.js`).
    HighlightJs,
}

/// Semantic attributes carried by an ordered list segment.
///
/// Keeping these as data until the `<ol>` is emitted avoids passing around
/// preformatted HTML fragments and centralizes escaping in [`Renderer::open_olist`].
#[derive(Clone, Copy, Default)]
struct OrderedListAttributes<'a> {
    start: Option<&'a str>,
    reversed: bool,
}

impl<'a> OrderedListAttributes<'a> {
    fn from_attrlist(attrlist: Option<&'a Attrlist<'_>>) -> Self {
        Self {
            start: attrlist
                .and_then(|attributes| attributes.named_attribute("start"))
                .map(|attribute| attribute.value()),
            reversed: attrlist.is_some_and(|attributes| attributes.has_option("reversed")),
        }
    }
}

impl Highlighter {
    fn from_document(document: &Document<'_>) -> Option<Self> {
        match attribute_str(document, "source-highlighter").as_deref() {
            Some("highlightjs" | "highlight.js") => Some(Self::HighlightJs),
            _ => None,
        }
    }
}

/// The STEM notation a stem block renders in, selecting its block math
/// delimiter pair (Asciidoctor's `BLOCK_MATH_DELIMITERS`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum StemType {
    /// AsciiMath, delimited by `\$ … \$`.
    AsciiMath,

    /// LaTeX math, delimited by `\[ … \]`.
    LatexMath,
}

impl StemType {
    /// Maps a `stem`-attribute or block-style value to a STEM type, mirroring
    /// Asciidoctor's `STEM_TYPE_ALIASES`: `latexmath`/`latex`/`tex` select
    /// LaTeX, every other value (including an absent one) selects AsciiMath.
    fn from_value(value: &str) -> Self {
        match value {
            "latexmath" | "latex" | "tex" => Self::LatexMath,
            _ => Self::AsciiMath,
        }
    }

    /// The block math delimiter pair (`open`, `close`).
    fn block_delimiters(self) -> (&'static str, &'static str) {
        match self {
            Self::AsciiMath => (r"\$", r"\$"),
            Self::LatexMath => (r"\[", r"\]"),
        }
    }
}

/// How the `sectanchors` document attribute decorates a non-discrete section
/// heading with its `<a class="anchor">` self-link.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SectionAnchors {
    /// `sectanchors` is unset: no anchor is injected.
    None,

    /// `:sectanchors:` — the anchor is prepended before the title text.
    Before,

    /// `:sectanchors: after` — the anchor is appended after the title text.
    After,
}

/// Which horizontal band of a table a row belongs to. The head band renders
/// its cells as `<th>` and never paragraph-wraps or style-wraps them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TableSection {
    Head,
    Body,
    Foot,
}

/// Whether `block` is one this backend renders to nothing.
///
/// This is how comments are dropped. `asciidoc-parser` keeps them in the parse
/// tree (so other tools can inspect them) and leaves it to the backend to
/// discard them, matching Asciidoctor. Attribute-entry blocks likewise carry no
/// output of their own.
fn renders_nothing(block: &Block<'_>) -> bool {
    // `////` comment blocks.
    if block.resolved_context().as_ref() == "comment" {
        return true;
    }

    if block.resolved_context().as_ref() == "attribute" {
        return true;
    }

    // A `[comment]`-styled *paragraph* is a comment block in Asciidoctor and
    // renders to nothing. On a delimited block (a `----` listing, say) the
    // style is decoration: Asciidoctor renders the block per its delimiter.
    if matches!(block, Block::Simple(simple) if simple.style() == SimpleBlockStyle::Paragraph)
        && block.declared_style() == Some("comment")
    {
        return true;
    }

    // A paragraph made up entirely of `//` line comments reaches the renderer
    // with its comment lines stripped to empty content; drop it rather than
    // emitting an empty `<p>`.
    matches!(block, Block::Simple(simple) if simple.style() == SimpleBlockStyle::Paragraph)
        && block
            .span()
            .data()
            .lines()
            .all(|line| line.trim_start().starts_with("//") || line.trim().is_empty())
}

/// Whether `block` renders through the source-block path — a `<pre>` carrying
/// the `highlight` class and a language-naming `<code>`.
fn is_source_listing(block: &Block<'_>) -> bool {
    if matches!(block, Block::Simple(simple) if simple.style() == SimpleBlockStyle::Source) {
        return true;
    }
    if block.declared_style() == Some("source") {
        return true;
    }

    // The `[,haskell]` shorthand: no declared block style, but a language sits
    // in the second positional attribute (Asciidoctor's `attributes[2]`).
    block.declared_style().is_none()
        && block
            .attrlist()
            .and_then(|attrlist| attrlist.nth_attribute(2))
            .is_some()
}

/// Renders `document` under `options`, as a complete page or body-only.
pub(crate) fn render_document(document: &Document<'_>, options: &Options) -> String {
    let mut renderer = Renderer {
        out: String::new(),
        title: title::render(document, options),
        icons_set: document.is_attribute_set("icons"),
        icons_font: attribute_str(document, "icons").as_deref() == Some("font"),
        iconsdir: attribute_or(document, "iconsdir", "./images/icons"),
        icontype: attribute_or(document, "icontype", "png"),
        imagesdir: attribute_str(document, "imagesdir").unwrap_or_default(),
        prewrap: document.is_attribute_set("prewrap"),
        source_highlighter: Highlighter::from_document(document),
        stem_type: StemType::from_value(&attribute_str(document, "stem").unwrap_or_default()),
        sectnumlevels: attribute_str(document, "sectnumlevels")
            .and_then(|value| value.parse().ok())
            .unwrap_or(3),
        sectlinks: document.is_attribute_set("sectlinks"),
        section_anchors: if !document.is_attribute_set("sectanchors") {
            SectionAnchors::None
        } else if attribute_str(document, "sectanchors").as_deref() == Some("after") {
            SectionAnchors::After
        } else {
            SectionAnchors::Before
        },
        cellbgcolor: attribute_str(document, "cellbgcolor"),
        max_width_attr: match attribute_str(document, "max-width") {
            Some(width) => format!(" style=\"max-width: {};\"", escape_quotes(&width)),
            None => String::new(),
        },
        // Filesystem-backed block assets resolve against the primary input
        // file's directory, or the process's current directory without one.
        files: FileResolver::new(
            options
                .input_file_path()
                .and_then(Path::parent)
                .unwrap_or_else(|| Path::new("")),
            options.effective_safe_mode(),
        ),
        standalone: options.is_standalone(),
    };

    if options.is_standalone() {
        renderer.document(document);
    } else {
        renderer.embedded_document(document);
    }

    // Asciidoctor's converted output carries no trailing newline; the final
    // `</html>` (or last body line) ends the string.
    renderer.out.pop();
    renderer.out
}

/// Holds the output buffer and the document-level settings the block
/// renderers consult.
struct Renderer {
    /// The accumulated markup.
    out: String,

    /// The rendered, partitioned document title. A document without one still
    /// gets the `untitled-label` fallback, so this is always present.
    title: Title,

    /// Whether the document sets `icons` at all, and whether it selects the
    /// font-based icon set. Together these pick the callout and admonition
    /// glyphs.
    icons_set: bool,
    icons_font: bool,

    /// Where icon images live, and their file extension.
    iconsdir: String,
    icontype: String,

    /// The directory block and inline image targets resolve against.
    imagesdir: String,

    /// Whether verbatim blocks wrap by default (`:prewrap:`, on by default).
    prewrap: bool,

    /// The active client-side syntax highlighter, if any.
    source_highlighter: Option<Highlighter>,

    /// The notation a bare `[stem]` block renders in.
    stem_type: StemType,

    /// The deepest section level that carries a section number.
    sectnumlevels: usize,

    /// The `sectlinks` / `sectanchors` heading decorations.
    sectlinks: bool,
    section_anchors: SectionAnchors,

    /// A document-wide table cell background colour, when set.
    cellbgcolor: Option<String>,

    /// The ` style="max-width: …;"` fragment stamped on the content and
    /// footnotes wrappers, or empty when `max-width` is unset.
    max_width_attr: String,

    /// Resolves filesystem-backed assets and enforces the selected safe mode.
    files: FileResolver,

    /// Whether a complete page is being emitted. Standalone output gets the
    /// Ruby converter's auto-set `iconfont-remote` attribute, which selects
    /// the Font Awesome CDN link under `:icons: font`.
    standalone: bool,
}

impl Renderer {
    /// Appends a line of markup followed by a newline, matching Asciidoctor's
    /// convention of one element per line with no indentation.
    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    // ---------------------------------------------------------------------
    // Document shell
    // ---------------------------------------------------------------------

    /// Emits the complete page: the `<head>`, the `<header>` (title,
    /// subtitle, and navigation), the content, the footnotes, and the licence
    /// footer.
    ///
    /// The `shownav` attribute drives two coupled choices: a navigation bar in
    /// the header, and the *absence* of the `<article>` wrapper that a prose
    /// page gets. Index-style pages set it; posts do not.
    fn document(&mut self, document: &Document<'_>) {
        let shownav = document.is_attribute_set("shownav");

        self.head(document);

        self.line(&format!("<body{}>", id_attribute(document.header().id())));

        if !shownav {
            self.line("<article>");
        }

        if !document.is_attribute_set("noheader") {
            self.header(document, shownav);
        }

        self.line("<hr>");

        let content = self.render_blocks_to_string(document.child_blocks());
        self.line(&format!(
            "<div id=\"content\"{}>\n{content}\n</div>",
            self.max_width_attr
        ));
        self.line("<hr>");

        self.footnotes(document);

        if !document.is_attribute_set("nofooter") {
            self.footer(document);
        }

        self.highlighter_footer(document);

        if !shownav {
            self.line("</article>");
        }

        self.line("</body>");
        self.line("</html>");
    }

    /// Emits the `<!DOCTYPE>`, the opening `<html>`, and the whole `<head>`:
    /// the web fonts and site stylesheet, the optional KaTeX and Mermaid
    /// assets, the metadata tags, and the title.
    fn head(&mut self, document: &Document<'_>) {
        let lang_attribute = if document.is_attribute_set("nolang") {
            String::new()
        } else {
            format!(
                " lang=\"{}\"",
                escape_quotes(&attribute_or(document, "lang", "en"))
            )
        };

        self.line("<!DOCTYPE html>");
        self.line(&format!("<html{lang_attribute}>"));
        self.line("<head>");
        self.line("<meta charset=\"utf-8\">");
        self.line("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
        self.line("<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin=\"\">");
        self.line(
            "<link href=\"https://fonts.googleapis.com/css2?family=Oxygen:wght@400;700&amp;display=swap\" rel=\"stylesheet\">",
        );
        self.line(
            "<link href=\"https://fonts.googleapis.com/css2?family=Noto+Sans+SC:wght@400;700&amp;display=swap\" rel=\"stylesheet\">",
        );
        self.line(
            "<link href=\"https://cdn.jsdelivr.net/npm/hack-font@3/build/web/hack.css\" rel=\"stylesheet\">",
        );
        self.line("<link rel=\"stylesheet\" href=\"/style.css\">");

        // STEM content is typeset in the browser by KaTeX's auto-render
        // extension, so the assets are pulled in only for a document that
        // enables `stem`.
        if document.is_attribute_set("stem") {
            self.line(
                r#"<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">"#,
            );
            self.line(
                r#"<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js" integrity="sha384-7zkQWkzuo3B5mTepMUcHkMB5jZaolc2xDwL6VFqjFALcbeS9Ggm/Yr2r3Dy4lfFg" crossorigin="anonymous"></script>"#,
            );
            self.line(
                r#"<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js" integrity="sha384-43gviWU0YVjaDtb/GhzOouOXtZMP/7XUzwPTstBeZFe/+rCMvRwr4yROQP43s0Xk" crossorigin="anonymous" onload="renderMathInElement(document.body);"></script>"#,
            );
        }

        if document.is_attribute_set("mermaid") {
            self.line(
                r#"<script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js" onload="mermaid.initialize({ startOnLoad: true });"></script>"#,
            );
        }

        self.line("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
        self.line(&format!(
            "<meta name=\"generator\" content=\"Waterlens HTML Backend {BACKEND_VERSION}\">"
        ));

        for name in ["description", "keywords", "author"] {
            if let Some(value) = attribute_str(document, name) {
                self.line(&format!(
                    "<meta name=\"{name}\" content=\"{}\">",
                    escape_quotes(&value)
                ));
            }
        }

        if document.is_attribute_set("favicon") {
            self.favicon(document);
        }

        // An explicit `pagetitle` wins over the document title; the title
        // element always carries plain text, so a marked-up doctitle is
        // stripped to its text.
        let title = match attribute_str(document, "pagetitle") {
            Some(pagetitle) => pagetitle,
            None => self.title.plain.clone(),
        };
        self.line(&format!("<title>{title}</title>"));

        // The stylesheet and icon-font links come after the title, matching
        // the Ruby converter's shell order. The document attributes involved
        // are *already substituted* (special characters escaped) by the
        // parser, so they are spliced into the markup verbatim; escaping them
        // again would double-escape.
        self.stylesheets(document);

        self.line("</head>");
    }

    /// Emits the stylesheet links: the `:stylesheet:` / `:webfonts:` /
    /// `:icons: font` handling the Ruby converter performs in its document
    /// shell. With none of those set (the site's normal state), nothing is
    /// emitted beyond the hardcoded `/style.css` link above.
    fn stylesheets(&mut self, document: &Document<'_>) {
        match attribute_str(document, "stylesheet").as_deref() {
            // The empty string and `DEFAULT` select the default stylesheet,
            // which in this backend only contributes the optional webfonts
            // link (Asciidoctor's `DEFAULT_STYLESHEET_KEYS`).
            Some("" | "DEFAULT") => {
                if let Some(webfonts) =
                    attribute_str(document, "webfonts").filter(|webfonts| !webfonts.is_empty())
                {
                    let href = format!(
                        "{}//fonts.googleapis.com/css?family={}",
                        asset_uri_scheme(document),
                        escape_quotes(&webfonts)
                    );
                    self.line(&format!("<link rel=\"stylesheet\" href=\"{href}\">"));
                }
            }
            Some(stylesheet) => {
                let stylesdir = attribute_or(document, "stylesdir", "");
                if document.is_attribute_set("linkcss") {
                    self.line(&format!(
                        "<link rel=\"stylesheet\" href=\"{}\">",
                        escape_quotes(&resolve_web_path(stylesheet, &stylesdir))
                    ));
                } else {
                    // The file's contents are inlined; a missing file yields
                    // an empty `<style>` element, matching Asciidoctor.
                    let stylesheet = decode_document_attribute(stylesheet);
                    let stylesdir = decode_document_attribute(&stylesdir);
                    let contents = self.read_asset(&stylesheet, &stylesdir).unwrap_or_default();
                    self.line(&format!("<style>\n{contents}\n</style>"));
                }
            }
            None => {}
        }

        // `:icons: font` glyphs come from Font Awesome; without its stylesheet
        // the `<i class="fa …">` elements render as nothing. The Ruby
        // converter's CLI sets `iconfont-remote` for every standalone
        // document, so the CDN link is the default here too — unless the
        // document mentions the attribute itself, in which case its own state
        // (an explicit `:iconfont-remote:` or `:iconfont-remote!:`) wins.
        if self.icons_font {
            let iconfont_remote = if document.has_attribute("iconfont-remote") {
                document.is_attribute_set("iconfont-remote")
            } else {
                self.standalone
            };
            if iconfont_remote {
                let cdn = format!(
                    "{}//cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css",
                    asset_uri_scheme(document)
                );
                let href = attribute_or(document, "iconfont-cdn", &cdn);
                self.line(&format!(
                    "<link rel=\"stylesheet\" href=\"{}\">",
                    escape_quotes(&href)
                ));
            } else {
                let name = attribute_or(document, "iconfont-name", "font-awesome");
                let stylesdir = attribute_or(document, "stylesdir", "");
                let href = resolve_web_path(&format!("{name}.css"), &stylesdir);
                self.line(&format!(
                    "<link rel=\"stylesheet\" href=\"{}\">",
                    escape_quotes(&href)
                ));
            }
        }
    }

    /// Emits the favicon `<link>`, deriving its MIME type from the target's
    /// extension the way Asciidoctor does.
    fn favicon(&mut self, document: &Document<'_>) {
        let href = attribute_str(document, "favicon").unwrap_or_default();

        let (icon_href, icon_type) = if href.is_empty() {
            ("favicon.ico".to_string(), "image/x-icon".to_string())
        } else {
            let extension = href.rsplit_once('.').map(|(_, ext)| ext);
            let icon_type = match extension {
                Some("ico") | None => "image/x-icon".to_string(),
                Some(ext) => format!("image/{ext}"),
            };
            (href, icon_type)
        };

        self.line(&format!(
            "<link rel=\"icon\" type=\"{}\" href=\"{}\">",
            escape_quotes(&icon_type),
            escape_quotes(&icon_href)
        ));
    }

    /// Emits the page `<header>`: the document title (partitioned into a main
    /// title and an optional subtitle), and — on a `shownav` page — the site
    /// navigation.
    fn header(&mut self, document: &Document<'_>, shownav: bool) {
        self.line("<header>");

        if !document.is_attribute_set("notitle") {
            let (main, subtitle) = (self.title.main.clone(), self.title.subtitle.clone());
            self.line(&format!("<h1>{main}</h1>"));
            if let Some(subtitle) = subtitle {
                self.line(&format!("<h2 class=\"subtitle\">{subtitle}</h2>"));
            }
        }

        if shownav {
            self.nav(document);
        }

        self.line("</header>");
    }

    /// Emits the site navigation, in the language the document declares.
    fn nav(&mut self, document: &Document<'_>) {
        let nav = match attribute_str(document, "lang").as_deref() {
            Some("zh-hans") => {
                "<nav>\n  \
                 <a href=\"/zh/index.html\">主页</a>\n  \
                 <a href=\"/zh/posts.html\">文章</a>\n  \
                 <a href=\"/zh/projects.html\">项目</a>\n  \
                 <a href=\"/zh/about.html\">关于</a>\n  \
                 <a href=\"/index.html\">English</a>\n  \
                 <a href=\"https://github.com/waterlens\">Github</a>\n\
                 </nav>"
            }
            _ => {
                "<nav>\n  \
                 <a href=\"/index.html\">Home</a>\n  \
                 <a href=\"/posts.html\">Posts</a>\n  \
                 <a href=\"/projects.html\">Projects</a>\n  \
                 <a href=\"/about.html\">About</a>\n  \
                 <a href=\"/zh/index.html\">中文</a>\n  \
                 <a href=\"https://github.com/waterlens\">Github</a>\n\
                 </nav>"
            }
        };
        self.line(nav);
    }

    /// Emits the Creative Commons licence footer, in the document's language.
    ///
    /// The closing year comes from the `localyear` attribute rather than the
    /// wall clock, so a build pinned to a reference time (or
    /// `SOURCE_DATE_EPOCH`) is reproducible.
    fn footer(&mut self, document: &Document<'_>) {
        let year = attribute_or(document, "localyear", "");

        // The English footer's "is licensed under" line ends in a space that
        // the following `<a>` needs; naming it keeps that space from being
        // invisible (and from being stripped by a trailing-whitespace tidier).
        let sp = " ";

        let footer = match attribute_str(document, "lang").as_deref() {
            Some("zh-hans") => format!(
                r#"<footer>
  <p>
    <a property="dct:title" rel="cc:attributionURL" href="/zh/index.html">本站</a>
    由 <span property="cc:attributionName">Waterlens</span>
    创作的一切内容 © 2021 - {year} 在
    <a href="http://creativecommons.org/licenses/by-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">
        知识共享 署名 - 相同方式共享 4.0 协议 <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1">
        <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1">
        <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1">
    </a>
    之条款下提供。
  </p>
</footer>"#
            ),
            _ => format!(
                r#"<footer>
  <p>
    The content on <a property="dct:title" rel="cc:attributionURL" href="/">this website</a>
    © 2021 - {year} by <span property="cc:attributionName">Waterlens</span>
    is licensed under{sp}
    <a href="http://creativecommons.org/licenses/by-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">
        CC BY-SA 4.0 <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1">
        <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1">
        <img alt="" style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1">
    </a>
  </p>
</footer>"#
            ),
        };

        self.line(&footer);
    }

    /// Emits the `highlight.js` stylesheet, loader, and per-language scripts
    /// after the footer — the `hljs` adapter's footer docinfo.
    fn highlighter_footer(&mut self, document: &Document<'_>) {
        if self.source_highlighter != Some(Highlighter::HighlightJs) {
            return;
        }

        let cdn_base_url = format!(
            "{}//cdnjs.cloudflare.com/ajax/libs",
            asset_uri_scheme(document)
        );
        let base_url = attribute_or(
            document,
            "highlightjsdir",
            &format!("{cdn_base_url}/highlight.js/{HIGHLIGHT_JS_VERSION}"),
        );
        let theme = attribute_or(document, "highlightjs-theme", DEFAULT_HIGHLIGHTJS_THEME);

        let languages = attribute_str(document, "highlightjs-languages")
            .map(|languages| {
                languages
                    .split(',')
                    .map(|language| {
                        format!(
                            "<script src=\"{base_url}/languages/{}.min.js\"></script>\n",
                            language.trim_start()
                        )
                    })
                    .collect::<String>()
            })
            .unwrap_or_default();

        self.line(&format!(
            // The blank line before the loader `<script>` is the newline that
            // follows the language-scripts interpolation in the `hljs`
            // adapter's docinfo template; each language script already ends in
            // one of its own, so the pair renders as a blank line whether or
            // not any languages are declared.
            "<link rel=\"stylesheet\" href=\"{base_url}/styles/{theme}.min.css\">\n\
             <script src=\"{base_url}/highlight.min.js\"></script>\n\
             {languages}\n\
             <script>\n\
             hljs.configure({{ignoreUnescapedHTML: true}});\n\
             hljs.highlightAll();\n\
             </script>"
        ));
    }

    /// Emits the body-only form: the document title (under `showtitle`)
    /// followed by the content and the footnotes block, with no page shell.
    fn embedded_document(&mut self, document: &Document<'_>) {
        if document.is_attribute_set("showtitle") && document.doctitle().is_some() {
            let title = self.title.main.clone();
            self.line(&format!("<h1>{title}</h1>"));
        }

        self.blocks(document.child_blocks());
        self.footnotes(document);
    }

    /// Emits the document-level footnotes block, when the document has
    /// footnotes and they are not suppressed by `nofootnotes`.
    fn footnotes(&mut self, document: &Document<'_>) {
        let footnotes = document.catalog().footnotes();
        if footnotes.is_empty() || document.is_attribute_set("nofootnotes") {
            return;
        }

        self.line(&format!(
            "<div id=\"footnotes\"{}>",
            self.max_width_attr.clone()
        ));
        for footnote in footnotes {
            let index = &footnote.index;
            self.line(&format!(
                "<div class=\"footnote\" id=\"_footnotedef_{index}\">"
            ));
            self.line(&format!(
                "<a href=\"#_footnoteref_{index}\">{index}</a>. {}",
                footnote.text
            ));
            self.line("</div>");
        }
        self.line("</div>");
    }

    // ---------------------------------------------------------------------
    // Block dispatch
    // ---------------------------------------------------------------------

    /// Walks a sequence of sibling blocks in document order.
    fn blocks<'src>(&mut self, blocks: impl Iterator<Item = &'src Block<'src>>) {
        for block in blocks {
            self.block(block);
        }
    }

    /// The dispatch point: routes one block to the matching renderer.
    fn block<'src>(&mut self, block: &'src Block<'src>) {
        if renders_nothing(block) {
            return;
        }

        match block {
            Block::Simple(simple) => match simple.style() {
                // A styled paragraph can convert to a different block: `[open]`
                // over a paragraph becomes an open block.
                SimpleBlockStyle::Paragraph => match block.declared_style() {
                    Some("open") => self.open_block(block),
                    Some("sidebar") => self.sidebar(block),
                    Some("example") => self.example(block),
                    _ => self.paragraph(block),
                },
                SimpleBlockStyle::Listing => self.listing(block),
                SimpleBlockStyle::Source => self.listing(block),
                SimpleBlockStyle::Literal => self.literal(block),
            },
            Block::Section(section) => self.section(block, section),
            Block::Preamble(_) => self.preamble(block),
            Block::Break(brk) => self.break_block(brk),
            Block::RawDelimited(_) => {
                // The delimiter decides what a raw delimited block is: a
                // `----` block is a listing no matter what style it carries
                // (`[sidebar]`/`[quote]`/`[stem]`/… over `----` still render
                // as a listing, matching Asciidoctor). Three styles
                // *specialize* the verbatim delimiters, however: `listing`
                // upgrades a `....` block, `source` upgrades a `....` or
                // `----` block to a source listing, and `literal` demotes a
                // `----` block to a literal. On a passthrough all of them are
                // decoration and the block stays a passthrough.
                let raw_context = block.raw_context();
                let context = match (block.declared_style(), raw_context.as_ref()) {
                    (Some("listing"), "literal") => "listing",
                    (Some("literal"), "listing") => "literal",
                    (Some("source"), "listing" | "literal") => "listing",
                    _ => raw_context.as_ref(),
                };
                match context {
                    "listing" => self.listing(block),
                    "literal" => self.literal(block),
                    "pass" => self.pass_block(block),
                    "stem" => self.stem(block),
                    other => self.unsupported(other),
                }
            }
            Block::CompoundDelimited(compound) => match compound.context_kind() {
                CompoundDelimitedContext::Open => self.open_block(block),
                CompoundDelimitedContext::Sidebar => self.sidebar(block),
                CompoundDelimitedContext::Example => self.example(block),
            },
            Block::Quote(quote) => self.quote(block, quote),
            Block::Admonition(admonition) => self.admonition(block, admonition),
            Block::List(list) => match list.type_() {
                ListType::Unordered => self.ulist(block, list),
                ListType::Ordered => self.olist(block, list),
                ListType::Description => self.dlist(block, list),
                ListType::Callout => self.colist(block, list),
            },
            Block::Table(table) => self.table(block, table),
            Block::Media(media) if media.type_() == MediaType::Image => self.image(block, media),
            Block::Media(media) if media.type_() == MediaType::Audio => self.audio(block, media),

            // `convert_video` is deliberately empty in this backend, so a
            // video block renders to nothing.
            Block::Media(media) if media.type_() == MediaType::Video => {}

            other => self.unsupported(&other.resolved_context()),
        }
    }

    /// Renders `blocks` into a standalone string with no trailing newline —
    /// the counterpart to Asciidoctor's `node.content`.
    fn render_blocks_to_string<'src>(
        &mut self,
        blocks: impl Iterator<Item = &'src Block<'src>>,
    ) -> String {
        let saved = std::mem::take(&mut self.out);
        for block in blocks {
            self.block(block);
        }
        let rendered = std::mem::replace(&mut self.out, saved);
        rendered.strip_suffix('\n').unwrap_or(&rendered).to_string()
    }

    /// Emits the inner content shared by wrapper blocks (open, quote,
    /// admonition, sidebar, example): a compound block recurses over its
    /// nested blocks, while a simple block emits its rendered content on its
    /// own line, unwrapped.
    fn wrapped_content<'src>(&mut self, block: &'src Block<'src>) {
        if block.content_model() == ContentModel::Compound {
            self.blocks(block.child_blocks());
        } else {
            let content = block.rendered_content().unwrap_or_default();
            self.line(content);
        }
    }

    // ---------------------------------------------------------------------
    // Leaf blocks
    // ---------------------------------------------------------------------

    /// A paragraph. Unlike the stock `html5` backend, an unadorned paragraph
    /// is a *bare* `<p>` with no wrapper `<div class="paragraph">`; only a
    /// titled paragraph gets the wrapper. Its text has CJK line breaks
    /// collapsed.
    fn paragraph<'src>(&mut self, block: &'src Block<'src>) {
        let content = collapse_cjk_newlines(block.rendered_content().unwrap_or_default());
        let id = block.id();
        let roles = block.roles();

        if let Some(title) = block.title() {
            let attributes = if roles.is_empty() {
                match id {
                    Some(id) => format!(" id=\"{}\" class=\"paragraph\"", escape_attribute(id)),
                    None => " class=\"paragraph\"".to_string(),
                }
            } else {
                format!(
                    "{} class=\"{}\"",
                    id_attribute(id),
                    class_list(&[&["paragraph"], roles.as_slice()].concat())
                )
            };

            self.line(&format!("<div{attributes}>"));
            self.line(&format!("<div class=\"title\">{title}</div>"));
            self.line(&format!("<p>{content}</p>"));
            self.line("</div>");
            return;
        }

        let attributes = if roles.is_empty() {
            id_attribute(id)
        } else {
            format!(
                "{} class=\"{}\"",
                id_attribute(id),
                class_list(&[&["paragraph"], roles.as_slice()].concat())
            )
        };
        self.line(&format!("<p{attributes}>{content}</p>"));
    }

    /// A listing block (`----`, with or without the `source` style):
    /// `<div class="listing">` around a `<pre>`.
    ///
    /// With a client-side highlighter active, the `<pre>`/`<code>` classes take
    /// the highlighter-specific shape; without one, a `source` block still
    /// gets the `highlight` class and a language-naming `<code>`, and a plain
    /// listing gets a bare `<pre>`.
    fn listing<'src>(&mut self, block: &'src Block<'src>) {
        let is_source = is_source_listing(block);

        self.open_block_wrapper(block, "listing");
        self.captioned_title(block);
        self.line("<div class=\"content\">");

        let content = self.verbatim_content(block);
        let nowrap = self.is_nowrap(block);

        if is_source {
            let language = block
                .attrlist()
                .and_then(|attrlist| attrlist.named_or_positional_attribute("language", 2))
                .map(|attr| attr.value());

            let (pre_attrs, code_open) = self.source_pre_code(block, language, nowrap);
            self.line(&format!(
                "<pre{pre_attrs}>{code_open}{content}</code></pre>"
            ));
        } else {
            let class = if nowrap { " class=\"nowrap\"" } else { "" };
            self.line(&format!("<pre{class}>{content}</pre>"));
        }

        self.line("</div>");
        self.line("</div>");
    }

    /// Builds the `<pre>` attributes and the opening `<code …>` tag for a
    /// source block.
    ///
    /// Without a highlighter this is Asciidoctor's default shape. With
    /// `highlight.js` it follows the `hljs` adapter: the `<pre>` leads with
    /// `highlightjs`, and the `<code>` carries the language class plus
    /// `data-noescape`, with `data-lang` moved to the end.
    fn source_pre_code(
        &self,
        block: &Block<'_>,
        language: Option<&str>,
        nowrap: bool,
    ) -> (String, String) {
        let nowrap_class = if nowrap { " nowrap" } else { "" };
        let language = language.map(escape_attribute);

        match self.source_highlighter {
            None => {
                let code_open = match &language {
                    Some(language) => {
                        format!("<code class=\"language-{language}\" data-lang=\"{language}\">")
                    }
                    None => "<code>".to_string(),
                };

                (format!(" class=\"highlight{nowrap_class}\""), code_open)
            }

            Some(Highlighter::HighlightJs) => {
                // `data-id` moves onto the `<pre>`; the `<code>` gets the
                // language class, `data-noescape`, an optional `data-trim`,
                // and finally `data-lang` (which the adapter re-appends last).
                let data_id = block
                    .attrlist()
                    .and_then(|attrlist| attrlist.named_attribute("data-id"))
                    .map(|attr| format!(" data-id=\"{}\"", escape_attribute(attr.value())))
                    .unwrap_or_default();

                let mut code_attrs = format!(
                    " class=\"language-{} hljs\" data-noescape=\"true\"",
                    language.as_deref().unwrap_or("none")
                );
                if block.has_option("trim") {
                    code_attrs.push_str(" data-trim=\"\"");
                }
                if let Some(language) = &language {
                    code_attrs.push_str(&format!(" data-lang=\"{language}\""));
                }

                (
                    format!(" class=\"highlightjs highlight{nowrap_class}\"{data_id}"),
                    format!("<code{code_attrs}>"),
                )
            }
        }
    }

    /// A literal block (`....`, or an indented paragraph):
    /// `<div class="literal">` around a `<pre>`.
    fn literal<'src>(&mut self, block: &'src Block<'src>) {
        self.open_block_wrapper(block, "literal");
        self.block_title(block);
        self.line("<div class=\"content\">");

        let content = self.verbatim_content(block);
        let class = if self.is_nowrap(block) {
            " class=\"nowrap\""
        } else {
            ""
        };
        self.line(&format!("<pre{class}>{content}</pre>"));

        self.line("</div>");
        self.line("</div>");
    }

    /// A passthrough block (`++++`): its content is emitted raw and
    /// unescaped, with no wrapping element.
    fn pass_block<'src>(&mut self, block: &'src Block<'src>) {
        let content = block.rendered_content().unwrap_or_default();
        let mut lines: Vec<&str> = content.split('\n').collect();
        strip_surrounding_blank_lines(&mut lines);
        self.line(&lines.join("\n"));
    }

    /// A STEM block: `<div class="stem">` wrapping the equation in the math
    /// delimiter pair its notation selects.
    fn stem<'src>(&mut self, block: &'src Block<'src>) {
        let stem_type = match block.declared_style() {
            Some("asciimath") => StemType::AsciiMath,
            Some("latexmath") => StemType::LatexMath,
            _ => match block
                .attrlist()
                .and_then(|attrlist| attrlist.nth_attribute(2))
                .map(|attr| attr.value())
            {
                Some("latexmath" | "latex" | "tex") => StemType::LatexMath,
                Some("asciimath") => StemType::AsciiMath,
                _ => self.stem_type,
            },
        };
        let (open, close) = stem_type.block_delimiters();

        self.open_block_wrapper(block, "stem");
        self.block_title(block);
        self.line("<div class=\"content\">");

        let mut equation = block.rendered_content().unwrap_or_default().to_string();
        if stem_type == StemType::AsciiMath && equation.contains('\n') {
            equation = rewrite_asciimath_breaks(&equation, open, close);
        }
        if !(equation.starts_with(open) && equation.ends_with(close)) {
            equation = format!("{open}{equation}{close}");
        }

        self.line(&equation);
        self.line("</div>");
        self.line("</div>");
    }

    /// The blank-line-trimmed inner text of a verbatim block's `<pre>`.
    ///
    /// The parser has already applied inline substitutions (so special
    /// characters are escaped) and normalized indentation, leaving only the
    /// leading and trailing blank lines to trim.
    fn verbatim_content<'src>(&self, block: &'src Block<'src>) -> String {
        let content = block.rendered_content().unwrap_or_default();
        let mut lines: Vec<&str> = content.split('\n').collect();
        strip_surrounding_blank_lines(&mut lines);
        lines.join("\n")
    }

    /// Whether a verbatim block's `<pre>` should carry the `nowrap` class: the
    /// block declares the `nowrap` option, or the document disabled `prewrap`.
    fn is_nowrap(&self, block: &Block<'_>) -> bool {
        block.has_option("nowrap") || !self.prewrap
    }

    /// A thematic break (`<hr>`) or a page break.
    fn break_block(&mut self, brk: &Break<'_>) {
        match brk.type_() {
            BreakType::Thematic => self.line("<hr>"),
            BreakType::Page => self.line("<div style=\"page-break-after: always;\"></div>"),
        }
    }

    // ---------------------------------------------------------------------
    // Wrapper blocks
    // ---------------------------------------------------------------------

    /// An open block: `<div class="open"><div class="content">…</div></div>`.
    fn open_block<'src>(&mut self, block: &'src Block<'src>) {
        let style = block.declared_style().unwrap_or_default();

        // An `[abstract]` open block renders as a quote; `[partintro]` is only
        // valid inside a book part, which this backend does not produce, so it
        // is dropped.
        if style == "abstract" {
            self.open_block_wrapper_with_style(block, "quote abstract", "");
            self.block_title(block);
            self.line("<blockquote>");
            self.wrapped_content(block);
            self.line("</blockquote>");
            self.line("</div>");
            return;
        }
        if style == "partintro" {
            return;
        }

        let extra = if style == "open" { "" } else { style };
        self.open_block_wrapper_with_style(block, "open", extra);
        self.block_title(block);
        self.line("<div class=\"content\">");
        self.wrapped_content(block);
        self.line("</div>");
        self.line("</div>");
    }

    /// A sidebar block. Unlike most blocks, the title sits *inside* the
    /// content div.
    fn sidebar<'src>(&mut self, block: &'src Block<'src>) {
        self.open_block_wrapper(block, "sidebar");
        self.line("<div class=\"content\">");
        self.block_title(block);
        self.wrapped_content(block);
        self.line("</div>");
        self.line("</div>");
    }

    /// An example block, or — under `[%collapsible]` — a `<details>`
    /// disclosure widget.
    fn example<'src>(&mut self, block: &'src Block<'src>) {
        if block.has_option("collapsible") {
            let open = if block.has_option("open") {
                " open"
            } else {
                ""
            };
            let class_attr = match block.roles().first() {
                Some(_) => class_attribute("", &block.roles()),
                None => String::new(),
            };
            let summary = match block.title() {
                Some(title) => format!("<summary class=\"title\">{title}</summary>"),
                None => "<summary class=\"title\">Details</summary>".to_string(),
            };

            self.line(&format!(
                "<details{}{class_attr}{open}>",
                id_attribute(block.id())
            ));
            self.line(&summary);
            self.line("<div class=\"content\">");
            self.wrapped_content(block);
            self.line("</div>");
            self.line("</details>");
            return;
        }

        self.open_block_wrapper(block, "example");
        self.captioned_title(block);
        self.line("<div class=\"content\">");
        self.wrapped_content(block);
        self.line("</div>");
        self.line("</div>");
    }

    /// A quote or verse block, with an optional attribution footer.
    fn quote<'src>(&mut self, block: &'src Block<'src>, quote: &'src QuoteBlock<'src>) {
        match quote.type_() {
            QuoteType::Quote => {
                self.open_block_wrapper(block, "quoteblock");
                self.block_title(block);
                self.line("<blockquote>");
                self.wrapped_content(block);
                self.line("</blockquote>");
            }
            QuoteType::Verse => {
                self.open_block_wrapper(block, "verse");
                self.block_title(block);
                let content = block.rendered_content().unwrap_or_default();
                self.line(&format!("<pre class=\"content\">{content}</pre>"));
            }
        }

        self.attribution(quote);
        self.line("</div>");
    }

    /// The `<div class="attribution">` footer of a quote or verse block.
    fn attribution(&mut self, quote: &QuoteBlock<'_>) {
        let attribution = quote.attribution();
        let citetitle = quote.citetitle();
        if attribution.is_none() && citetitle.is_none() {
            return;
        }

        self.line("<div class=\"attribution\">");
        if let Some(attribution) = attribution {
            let line_break = if citetitle.is_some() { "<br>" } else { "" };
            self.line(&format!("&#8212; {attribution}{line_break}"));
        }
        if let Some(citetitle) = citetitle {
            self.line(&format!("<cite>{citetitle}</cite>"));
        }
        self.line("</div>");
    }

    /// An admonition: a two-cell table, the first cell holding the label (or
    /// icon) and the second the content.
    fn admonition<'src>(
        &mut self,
        block: &'src Block<'src>,
        admonition: &'src AdmonitionBlock<'src>,
    ) {
        let name = admonition.name();
        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute(&format!("admonition {name}"), &block.roles())
        ));
        self.line("<table>");
        self.line("<tr>");
        self.line("<td class=\"icon\">");

        let label = if self.icons_set {
            if self.icons_font {
                format!(
                    "<i class=\"fa icon-{name}\" title=\"{}\"></i>",
                    escape_attribute(admonition.label())
                )
            } else {
                format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    self.icon_uri(name),
                    escape_attribute(admonition.label())
                )
            }
        } else {
            format!("<div class=\"title\">{}</div>", admonition.label())
        };
        self.line(&label);

        self.line("</td>");
        self.line("<td class=\"content\">");
        self.block_title(block);
        self.wrapped_content(block);
        self.line("</td>");
        self.line("</tr>");
        self.line("</table>");
        self.line("</div>");
    }

    /// The URI of a named icon image, under `iconsdir` with the `icontype`
    /// extension.
    fn icon_uri(&self, name: &str) -> String {
        escape_quotes(&resolve_web_path(
            &escape_attribute(&format!("{name}.{}", self.icontype)),
            &self.iconsdir,
        ))
    }

    /// Reads a local site asset relative to the document base and `dir`.
    /// URI-backed assets are not fetched by this filesystem-only backend.
    fn read_asset(&self, target: &str, dir: &str) -> Option<String> {
        if looks_like_uri(target)
            || target.starts_with("//")
            || looks_like_uri(dir)
            || dir.starts_with("//")
        {
            return None;
        }

        let current_dir = (!dir.is_empty()).then(|| self.files.resolve_directory(dir));
        self.files.read(current_dir.as_deref(), target).ok()
    }

    // ---------------------------------------------------------------------
    // Lists
    // ---------------------------------------------------------------------

    /// An unordered list. Item text has CJK line breaks collapsed.
    fn ulist<'src>(&mut self, block: &'src Block<'src>, list: &'src ListBlock<'src>) {
        let checklist = list.is_checklist();
        let style = block.declared_style().unwrap_or_default();
        let interactive = block.has_option("interactive");
        let items: Vec<&Block<'_>> = list.child_blocks().collect();

        // `asciidoc-parser` merges an attribute-decorated list that follows a
        // nested list into the previous list, hanging the list's attributes
        // on its first item (see `split_list_at_attributes`). Render the
        // pre-attribute items as this list and start a fresh one at the
        // attributed item, carrying its attributes — the structure
        // Asciidoctor produces. The merged segment keeps its own item
        // markers, so a `. c` segment merged into a `*` list still renders as
        // an ordered list.
        match split_list_at_attributes(&items) {
            Some(split) if split > 0 => {
                self.open_ulist(block.id(), &block.roles(), checklist, style, block, true);
                for &item in &items[..split] {
                    self.list_item(item, checklist, interactive, false);
                }
                self.line("</ul>");
                self.line("</div>");

                let segment = &items[split..];
                if item_is_ordered(segment[0]) {
                    let item = segment[0];
                    let item_style = item
                        .declared_style()
                        .or_else(|| ordered_list_marker_style(item))
                        .unwrap_or("arabic");
                    let attributes = OrderedListAttributes::from_attrlist(item.attrlist());
                    self.open_olist(
                        item.id(),
                        &item.roles(),
                        item_style,
                        attributes,
                        block,
                        false,
                    );
                    for &item in segment {
                        self.list_item(item, false, false, true);
                    }
                    self.line("</ol>");
                    self.line("</div>");
                } else {
                    let item = segment[0];
                    let item_style = item.declared_style().unwrap_or_default();
                    self.open_ulist(
                        item.id(),
                        &item.roles(),
                        checklist,
                        item_style,
                        block,
                        false,
                    );
                    for &item in segment {
                        self.list_item(item, checklist, interactive, true);
                    }
                    self.line("</ul>");
                    self.line("</div>");
                }
            }
            _ => {
                self.open_ulist(block.id(), &block.roles(), checklist, style, block, true);
                for item in items {
                    self.list_item(item, checklist, interactive, false);
                }
                self.line("</ul>");
                self.line("</div>");
            }
        }
    }

    /// Opens a `<div class="ulist …">`/`<ul>` pair, honoring `checklist` and
    /// the list style, with the block's title on the first segment only.
    fn open_ulist(
        &mut self,
        id: Option<&str>,
        roles: &[&str],
        checklist: bool,
        style: &str,
        block: &Block<'_>,
        with_title: bool,
    ) {
        // `['ulist', ('checklist')?, style, *roles]` — a checklist puts its
        // marker class right after `ulist`.
        let mut classes: Vec<&str> = vec!["ulist"];
        if checklist {
            classes.push("checklist");
        }
        classes.push(style);
        classes.extend(roles);

        self.line(&format!(
            "<div{} class=\"{}\">",
            id_attribute(id),
            class_list(&classes)
        ));
        if with_title {
            self.block_title(block);
        }

        let ul_class = if checklist {
            " class=\"checklist\"".to_string()
        } else if style.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", escape_attribute(style))
        };
        self.line(&format!("<ul{ul_class}>"));
    }

    /// An ordered list. Item text has CJK line breaks collapsed.
    fn olist<'src>(&mut self, block: &'src Block<'src>, list: &'src ListBlock<'src>) {
        let style = olist_style(block, list);
        let items: Vec<&Block<'_>> = list.child_blocks().collect();

        // The same merged-list repair as `ulist`: an attributed item begins a
        // fresh list carrying the item's style and `start`/`reversed`
        // options. A `*` segment merged into a `.` list renders as an
        // unordered list again.
        match split_list_at_attributes(&items) {
            Some(split) if split > 0 => {
                let attributes = OrderedListAttributes::from_attrlist(list.attrlist());
                self.open_olist(block.id(), &block.roles(), style, attributes, block, true);
                for &item in &items[..split] {
                    self.list_item(item, false, false, false);
                }
                self.line("</ol>");
                self.line("</div>");

                let segment = &items[split..];
                if item_is_ordered(segment[0]) {
                    let item = segment[0];
                    let item_style = item
                        .declared_style()
                        .or_else(|| ordered_list_marker_style(item))
                        .unwrap_or(style);
                    let attributes = OrderedListAttributes::from_attrlist(item.attrlist());
                    self.open_olist(
                        item.id(),
                        &item.roles(),
                        item_style,
                        attributes,
                        block,
                        false,
                    );
                    for &item in segment {
                        self.list_item(item, false, false, true);
                    }
                    self.line("</ol>");
                    self.line("</div>");
                } else {
                    let item = segment[0];
                    let item_style = item.declared_style().unwrap_or_default();
                    self.open_ulist(item.id(), &item.roles(), false, item_style, block, false);
                    for &item in segment {
                        self.list_item(item, false, false, true);
                    }
                    self.line("</ul>");
                    self.line("</div>");
                }
            }
            _ => {
                let attributes = OrderedListAttributes::from_attrlist(list.attrlist());
                self.open_olist(block.id(), &block.roles(), style, attributes, block, true);
                for item in items {
                    self.list_item(item, false, false, false);
                }
                self.line("</ol>");
                self.line("</div>");
            }
        }
    }

    /// Opens a `<div class="olist …">`/`<ol>` pair with explicit attributes —
    /// shared by the main list and the repair-split segment. The block title
    /// lands on the first segment only.
    fn open_olist(
        &mut self,
        id: Option<&str>,
        roles: &[&str],
        style: &str,
        attributes: OrderedListAttributes<'_>,
        block: &Block<'_>,
        with_title: bool,
    ) {
        self.line(&format!(
            "<div{}{}>",
            id_attribute(id),
            class_attribute(&format!("olist {style}"), roles)
        ));
        if with_title {
            self.block_title(block);
        }

        let type_attr = match style {
            "loweralpha" => " type=\"a\"",
            "lowerroman" => " type=\"i\"",
            "upperalpha" => " type=\"A\"",
            "upperroman" => " type=\"I\"",
            _ => "",
        };
        let start_attr = attributes
            .start
            .map(|start| format!(" start=\"{}\"", escape_attribute(start)))
            .unwrap_or_default();
        let reversed_attr = if attributes.reversed { " reversed" } else { "" };

        self.line(&format!(
            "<ol class=\"{}\"{type_attr}{start_attr}{reversed_attr}>",
            escape_attribute(style)
        ));
    }

    /// A callout list: a plain `<ol>`, or a two-column `<table>` of numbered
    /// icons when the document sets `icons`.
    fn colist<'src>(&mut self, block: &'src Block<'src>, list: &'src ListBlock<'src>) {
        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute("colist arabic", &block.roles())
        ));
        self.block_title(block);

        if self.icons_set {
            self.line("<table>");
            for (index, item) in list
                .child_blocks()
                .filter_map(|block| match block {
                    Block::ListItem(item) => Some(item),
                    _ => None,
                })
                .enumerate()
            {
                self.colist_row(item, index + 1);
            }
            self.line("</table>");
        } else {
            self.line("<ol>");
            for item in list.child_blocks() {
                self.list_item(item, false, false, false);
            }
            self.line("</ol>");
        }

        self.line("</div>");
    }

    /// One `<tr>` of an icon-based callout list.
    fn colist_row<'src>(&mut self, list_item: &'src ListItem<'src>, num: usize) {
        let num_label = if self.icons_font {
            format!("<i class=\"conum\" data-value=\"{num}\"></i><b>{num}</b>")
        } else {
            let src = self.icon_uri(&format!("callouts/{num}"));
            format!("<img src=\"{src}\" alt=\"{num}\">")
        };

        self.line("<tr>");
        self.line(&format!("<td>{num_label}</td>"));

        let mut blocks = list_item.child_blocks();
        let text = blocks
            .next()
            .and_then(|block| block.rendered_content())
            .unwrap_or_default()
            .to_string();

        let content = self.render_blocks_to_string(blocks);
        if content.is_empty() {
            self.line(&format!("<td>{text}</td>"));
        } else {
            self.line(&format!("<td>{text}\n{content}</td>"));
        }

        self.line("</tr>");
    }

    /// A description list, in its labelled, `qanda`, or `horizontal` layout.
    fn dlist<'src>(&mut self, block: &'src Block<'src>, list: &'src ListBlock<'src>) {
        let style = block.declared_style();
        let entries = dlist_entries(list);

        match style {
            Some("qanda") => self.dlist_qanda(block, &entries),
            Some("horizontal") => self.dlist_horizontal(block, list, &entries),
            _ => self.dlist_labeled(block, style, &entries),
        }
    }

    /// The default `<dl>`/`<dt>`/`<dd>` layout.
    fn dlist_labeled(
        &mut self,
        block: &Block<'_>,
        style: Option<&str>,
        entries: &[DlistEntry<'_>],
    ) {
        let mut classes: Vec<&str> = vec!["dlist"];
        classes.push(style.unwrap_or_default());
        classes.extend(block.roles());

        self.line(&format!(
            "<div{} class=\"{}\">",
            id_attribute(block.id()),
            class_list(&classes)
        ));
        self.block_title(block);
        self.line("<dl>");

        let dt_open = if style.is_some() {
            "<dt>"
        } else {
            "<dt class=\"hdlist1\">"
        };
        for (terms, description) in entries {
            for term in terms {
                self.line(&format!("{dt_open}{term}</dt>"));
            }
            if let Some(item) = description {
                self.line("<dd>");
                self.dlist_body(item, false);
                self.line("</dd>");
            }
        }

        self.line("</dl>");
        self.line("</div>");
    }

    /// The `qanda` layout: a numbered `<ol>` of emphasized questions. Both the
    /// terms and the answers have CJK line breaks collapsed.
    fn dlist_qanda(&mut self, block: &Block<'_>, entries: &[DlistEntry<'_>]) {
        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute("qlist qanda", &block.roles())
        ));
        self.block_title(block);
        self.line("<ol>");

        for (terms, description) in entries {
            self.line("<li>");
            for term in terms {
                self.line(&format!("<p><em>{}</em></p>", collapse_cjk_newlines(term)));
            }
            if let Some(item) = description {
                self.dlist_body(item, true);
            }
            self.line("</li>");
        }

        self.line("</ol>");
        self.line("</div>");
    }

    /// The `horizontal` layout: a two-column `<table>`.
    fn dlist_horizontal(
        &mut self,
        block: &Block<'_>,
        list: &ListBlock<'_>,
        entries: &[DlistEntry<'_>],
    ) {
        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute("hdlist", &block.roles())
        ));
        self.block_title(block);
        self.line("<table>");

        let labelwidth = dlist_width(list, "labelwidth");
        let itemwidth = dlist_width(list, "itemwidth");
        if labelwidth.is_some() || itemwidth.is_some() {
            self.line("<colgroup>");
            self.line(&dlist_col(labelwidth.as_deref()));
            self.line(&dlist_col(itemwidth.as_deref()));
            self.line("</colgroup>");
        }

        let strong = if block.has_option("strong") {
            " strong"
        } else {
            ""
        };
        for (terms, description) in entries {
            self.line("<tr>");
            self.line(&format!("<td class=\"hdlist1{strong}\">"));
            for (index, term) in terms.iter().enumerate() {
                if index > 0 {
                    self.line("<br>");
                }
                self.line(term);
            }
            self.line("</td>");

            self.line("<td class=\"hdlist2\">");
            if let Some(item) = description {
                self.dlist_body(item, false);
            }
            self.line("</td>");
            self.line("</tr>");
        }

        self.line("</table>");
        self.line("</div>");
    }

    /// A description entry's body: its principal text as a bare `<p>` followed
    /// by any attached blocks. `collapse` selects whether the principal text
    /// has CJK line breaks collapsed, which the `qanda` layout does.
    fn dlist_body(&mut self, description: &ListItem<'_>, collapse: bool) {
        let blocks: Vec<&Block<'_>> = description.child_blocks().collect();

        let foldable = blocks
            .first()
            .is_some_and(|first| first.resolved_context().as_ref() == "paragraph");

        let attached = if foldable {
            let text = blocks[0].rendered_content().unwrap_or_default();
            if !text.is_empty() {
                let text = if collapse {
                    collapse_cjk_newlines(text)
                } else {
                    text.to_string()
                };
                self.line(&format!("<p>{text}</p>"));
            }
            &blocks[1..]
        } else {
            &blocks[..]
        };

        for &block in attached {
            self.block(block);
        }
    }

    /// One `<li>…</li>`: the principal text as a bare `<p>` with CJK line
    /// breaks collapsed, followed by any attached blocks.
    ///
    /// With `strip_attrs`, the item's id/roles are not emitted — used for the
    /// items of a repair-split list, whose first item carries attributes that
    /// were hoisted onto the new list wrapper.
    fn list_item<'src>(
        &mut self,
        item: &'src Block<'src>,
        checklist: bool,
        interactive: bool,
        strip_attrs: bool,
    ) {
        let Block::ListItem(list_item) = item else {
            return;
        };

        let li_open = if strip_attrs {
            "<li>".to_string()
        } else if let Some(id) = item.id() {
            format!(
                "<li id=\"{}\"{}>",
                escape_attribute(id),
                class_attribute("", &item.roles())
            )
        } else if item.roles().is_empty() {
            "<li>".to_string()
        } else {
            format!("<li{}>", class_attribute("", &item.roles()))
        };
        self.line(&li_open);

        let mut blocks = list_item.child_blocks();
        let principal = collapse_cjk_newlines(
            blocks
                .next()
                .and_then(|block| block.rendered_content())
                .unwrap_or_default(),
        );

        match (checklist, list_item.checkbox()) {
            (true, Some(checked)) => {
                let marker = self.checkbox_marker(checked, interactive);
                self.line(&format!("<p>{marker}{principal}</p>"));
            }
            _ => self.line(&format!("<p>{principal}</p>")),
        }

        for block in blocks {
            self.block(block);
        }

        self.line("</li>");
    }

    /// The checkbox glyph that prefixes a checklist item's text.
    fn checkbox_marker(&self, checked: bool, interactive: bool) -> &'static str {
        match (interactive, self.icons_font, checked) {
            (true, _, true) => "<input type=\"checkbox\" data-item-complete=\"1\" checked> ",
            (true, _, false) => "<input type=\"checkbox\" data-item-complete=\"0\"> ",
            (false, true, true) => "<i class=\"fa fa-check-square-o\"></i> ",
            (false, true, false) => "<i class=\"fa fa-square-o\"></i> ",
            (false, false, true) => "&#10003; ",
            (false, false, false) => "&#10063; ",
        }
    }

    // ---------------------------------------------------------------------
    // Tables
    // ---------------------------------------------------------------------

    /// A table, wrapped in a `<div class="table-wrapper">` so a wide table can
    /// scroll horizontally without stretching the page.
    fn table<'src>(&mut self, block: &'src Block<'src>, table: &'src TableBlock<'src>) {
        let frame = match table.frame() {
            Frame::All => "all",
            Frame::Ends => "ends",
            Frame::Sides => "sides",
            Frame::None => "none",
        };
        let grid = match table.grid() {
            Grid::All => "all",
            Grid::Rows => "rows",
            Grid::Cols => "cols",
            Grid::None => "none",
        };
        let mut classes = format!("table frame-{frame} grid-{grid}");

        if let Some(stripes) = table_stripes_class(table) {
            classes.push(' ');
            classes.push_str(&stripes);
        }

        let autowidth = table.is_autowidth();
        let has_width = table.width().is_some();
        let mut style_attr = String::new();
        if autowidth && !has_width {
            classes.push_str(" fit-content");
        } else {
            let tablewidth = table.width().unwrap_or(100);
            if tablewidth == 100 {
                classes.push_str(" stretch");
            } else {
                style_attr = format!(" style=\"width: {tablewidth}%;\"");
            }
        }

        if let Some(float) = table
            .attrlist()
            .and_then(|a| a.named_attribute("float"))
            .map(|attr| attr.value())
        {
            classes.push(' ');
            classes.push_str(&escape_attribute(float));
        }
        for role in block.roles() {
            classes.push(' ');
            classes.push_str(&escape_attribute(role));
        }

        self.line("<div class=\"table-wrapper\">");
        self.line(&format!(
            "<table{} class=\"{classes}\"{style_attr}>",
            id_attribute(block.id())
        ));

        if let Some(title) = block.title() {
            let caption = block.caption().unwrap_or_default();
            self.line(&format!(
                "<caption class=\"title\">{caption}{title}</caption>"
            ));
        }

        let rowcount = table.header_row().is_some() as usize
            + table.body_rows().len()
            + table.footer_row().is_some() as usize;
        if rowcount > 0 {
            self.colgroup(table, autowidth);

            if let Some(header) = table.header_row() {
                self.line("<thead>");
                self.table_row(header, TableSection::Head);
                self.line("</thead>");
            }
            if !table.body_rows().is_empty() {
                self.line("<tbody>");
                for row in table.body_rows() {
                    self.table_row(row, TableSection::Body);
                }
                self.line("</tbody>");
            }
            if let Some(footer) = table.footer_row() {
                self.line("<tfoot>");
                self.table_row(footer, TableSection::Foot);
                self.line("</tfoot>");
            }
        }

        self.line("</table>");
        self.line("</div>");
    }

    /// The `<colgroup>` of `<col>` elements.
    fn colgroup(&mut self, table: &TableBlock<'_>, autowidth: bool) {
        self.line("<colgroup>");
        if autowidth {
            for _ in table.columns() {
                self.line("<col>");
            }
        } else {
            let pcwidths = column_pcwidths(table.columns());
            for (col, pcwidth) in table.columns().iter().zip(pcwidths) {
                if col.is_autowidth() {
                    self.line("<col>");
                } else {
                    self.line(&format!("<col style=\"width: {pcwidth}%;\">"));
                }
            }
        }
        self.line("</colgroup>");
    }

    /// One `<tr>` and its cells.
    fn table_row<'src>(&mut self, row: &'src TableRow<'src>, section: TableSection) {
        self.line("<tr>");
        for cell in row.cells() {
            self.table_cell(cell, section);
        }
        self.line("</tr>");
    }

    /// One table cell.
    fn table_cell<'src>(&mut self, cell: &'src TableCell<'src>, section: TableSection) {
        let is_head = section == TableSection::Head;

        let content = match cell.content() {
            TableCellContent::AsciiDoc(ad) => {
                let rendered = self.render_blocks_to_string(ad.blocks().iter());
                format!("<div class=\"content\">{rendered}</div>")
            }
            TableCellContent::Simple(simple) => {
                if is_head {
                    simple.rendered().to_string()
                } else if cell.style() == ColumnStyle::Literal {
                    format!(
                        "<div class=\"literal\"><pre>{}</pre></div>",
                        simple.rendered()
                    )
                } else {
                    cell_paragraphs(cell, simple.rendered())
                }
            }
        };

        let tag = if is_head || cell.style() == ColumnStyle::Header {
            "th"
        } else {
            "td"
        };
        let h_align = match cell.h_align() {
            HorizontalAlignment::Left => "left",
            HorizontalAlignment::Center => "center",
            HorizontalAlignment::Right => "right",
        };
        let v_align = match cell.v_align() {
            VerticalAlignment::Top => "top",
            VerticalAlignment::Middle => "middle",
            VerticalAlignment::Bottom => "bottom",
        };
        let colspan = if cell.colspan() > 1 {
            format!(" colspan=\"{}\"", cell.colspan())
        } else {
            String::new()
        };
        let rowspan = if cell.rowspan() > 1 {
            format!(" rowspan=\"{}\"", cell.rowspan())
        } else {
            String::new()
        };
        let style = match &self.cellbgcolor {
            Some(color) => format!(" style=\"background-color: {};\"", escape_quotes(color)),
            None => String::new(),
        };

        self.line(&format!(
            "<{tag} class=\"table halign-{h_align} valign-{v_align}\"{colspan}{rowspan}{style}>{content}</{tag}>"
        ));
    }

    // ---------------------------------------------------------------------
    // Sections
    // ---------------------------------------------------------------------

    /// A section: `<section class="sectN"><hM id>title</hM>…</section>`, or —
    /// for a discrete heading — a bare `<hM>` with no wrapper.
    fn section<'src>(&mut self, block: &'src Block<'src>, section: &'src SectionBlock<'src>) {
        let level = section.level();
        let heading_level = (level + 1).min(6);
        let id = block.id();
        let title = self.section_heading_title(block, section);

        if section.section_type() == SectionType::Discrete {
            let style = block.declared_style().unwrap_or("discrete");
            let mut classes: Vec<&str> = vec![style];
            classes.extend(block.roles());
            self.line(&format!(
                "<h{heading_level}{} class=\"{}\">{title}</h{heading_level}>",
                id_attribute(id),
                class_list(&classes)
            ));
            return;
        }

        let title = self.decorate_section_heading(&title, id);

        if level == 0 {
            self.line(&format!(
                "<h{heading_level}{}{}>{title}</h{heading_level}>",
                id_attribute(id),
                class_attribute("sect0", &block.roles())
            ));
            self.blocks(block.child_blocks());
            return;
        }

        let content = self.render_blocks_to_string(block.child_blocks());
        self.line(&format!(
            "<section{}>",
            class_attribute(&format!("sect{level}"), &block.roles())
        ));
        self.line(&format!(
            "<h{heading_level}{}>{title}</h{heading_level}>",
            id_attribute(id)
        ));
        self.line(&content);
        self.line("</section>");
    }

    /// A section heading's displayed text: a caption prefix, else a section
    /// number, else the bare title.
    fn section_heading_title<'src>(
        &self,
        block: &'src Block<'src>,
        section: &'src SectionBlock<'src>,
    ) -> String {
        let title = section.section_title();

        if let Some(caption) = block.caption() {
            format!("{caption}{title}")
        } else if let Some(number) = section
            .section_number()
            .filter(|_| section.level() <= self.sectnumlevels)
        {
            format!("{number}. {title}")
        } else {
            title.to_string()
        }
    }

    /// Applies the `sectlinks` and `sectanchors` decorations to a heading.
    fn decorate_section_heading(&self, title: &str, id: Option<&str>) -> String {
        let Some(id) = id else {
            return title.to_string();
        };
        let href = escape_attribute(id);

        let mut title = if self.sectlinks {
            format!("<a class=\"link\" href=\"#{href}\">{title}</a>")
        } else {
            title.to_string()
        };

        match self.section_anchors {
            SectionAnchors::Before => {
                title = format!("<a class=\"anchor\" href=\"#{href}\"></a>{title}");
            }
            SectionAnchors::After => {
                title.push_str(&format!("<a class=\"anchor\" href=\"#{href}\"></a>"));
            }
            SectionAnchors::None => {}
        }

        title
    }

    /// The preamble renders its children directly, with no wrapper.
    fn preamble<'src>(&mut self, block: &'src Block<'src>) {
        self.blocks(block.child_blocks());
    }

    // ---------------------------------------------------------------------
    // Media
    // ---------------------------------------------------------------------

    /// A block image.
    ///
    /// Dimensions given in `rem` become an inline `style` rather than the
    /// HTML `width`/`height` attributes, which only accept pixel counts — this
    /// is what lets the blog size diagrams relative to the text.
    fn image<'src>(&mut self, block: &'src Block<'src>, media: &'src MediaBlock<'src>) {
        let macro_attrs = media.macro_attrlist();

        let named = |name: &str| -> Option<&str> {
            macro_attrs
                .named_attribute(name)
                .or_else(|| block.attrlist().and_then(|a| a.named_attribute(name)))
                .map(|attr| attr.value())
        };
        let positional = |name: &str, n: usize| -> Option<&str> {
            macro_attrs
                .named_or_positional_attribute(name, n)
                .or_else(|| block.attrlist().and_then(|a| a.named_attribute(name)))
                .map(|attr| attr.value())
        };

        let target = media.resolved_target();
        // `target` is a raw macro attribute, `imagesdir` an already-substituted
        // document attribute — escaping the target once, before the join,
        // yields a correctly single-escaped `src`.
        let src = escape_quotes(&resolve_web_path(
            &escape_attribute(target),
            &self.imagesdir,
        ));
        let alt = positional("alt", 1)
            .map(str::to_string)
            .unwrap_or_else(|| default_alt(target));

        // A `rem` dimension cannot ride on the HTML attribute, so it moves to
        // the inline style; pixel dimensions stay as attributes.
        let mut html_attrs = String::new();
        let mut styles: Vec<String> = Vec::new();
        for (name, value) in [
            ("width", positional("width", 2)),
            ("height", positional("height", 3)),
        ] {
            let Some(value) = value else { continue };
            if value.ends_with("rem") {
                styles.push(format!("{name}: {value};"));
            } else {
                html_attrs.push_str(&format!(" {name}=\"{}\"", escape_attribute(value)));
            }
        }
        if !styles.is_empty() {
            html_attrs.push_str(&format!(
                " style=\"{}\"",
                escape_attribute(&styles.join(" "))
            ));
        }

        // An SVG target can be inlined into the page or wrapped in an
        // `<object>` — the `inline` and `interactive` options the Ruby
        // converter supports. Both apply only to SVG targets.
        let is_svg = (positional("format", 4).is_some_and(|format| format == "svg")
            || target.contains(".svg"))
            && self.files.safe_mode() < SafeMode::Secure;
        let filesystem_imagesdir = decode_document_attribute(&self.imagesdir);
        let mut img = if is_svg && macro_attrs.has_option("inline") {
            self.read_asset(target, &filesystem_imagesdir)
                .and_then(|svg| inline_svg(&svg, positional("width", 2), positional("height", 3)))
                .unwrap_or_else(|| format!("<span class=\"alt\">{}</span>", escape_text(&alt)))
        } else if is_svg && macro_attrs.has_option("interactive") {
            let fallback = match named("fallback") {
                Some(fallback) => format!(
                    "<img src=\"{}\" alt=\"{}\"{html_attrs}>",
                    escape_quotes(&resolve_web_path(
                        &escape_attribute(fallback),
                        &self.imagesdir,
                    )),
                    escape_attribute(&alt)
                ),
                None => format!("<span class=\"alt\">{}</span>", escape_text(&alt)),
            };
            format!("<object type=\"image/svg+xml\" data=\"{src}\"{html_attrs}>{fallback}</object>")
        } else {
            format!(
                "<img src=\"{src}\" alt=\"{}\"{html_attrs}>",
                escape_attribute(&alt),
            )
        };

        if let Some(link) = named("link") {
            let window = named("window");
            let nofollow = macro_attrs.has_option("nofollow")
                || block.attrlist().is_some_and(|a| a.has_option("nofollow"));
            let noopener = macro_attrs.has_option("noopener")
                || block.attrlist().is_some_and(|a| a.has_option("noopener"));

            img = format!(
                "<a class=\"image\" href=\"{}\"{}>{img}</a>",
                escape_attribute(link),
                link_constraint_attrs(window, nofollow, noopener),
            );
        }

        let mut classes = String::from("imageblock");
        if let Some(float) = named("float") {
            classes.push(' ');
            classes.push_str(&escape_attribute(float));
        }
        if let Some(align) = named("align") {
            classes.push_str(&format!(" text-{}", escape_attribute(align)));
        }
        for role in media_roles(block, media) {
            classes.push(' ');
            classes.push_str(&escape_attribute(role));
        }

        self.line(&format!(
            "<div{} class=\"{classes}\">",
            id_attribute(block.id())
        ));
        self.line("<div class=\"content\">");
        self.line(&img);
        self.line("</div>");

        // A block image places its captioned title *after* the content div.
        if let Some(title) = block.title() {
            let caption = block.caption().unwrap_or_default();
            self.line(&format!("<div class=\"title\">{caption}{title}</div>"));
        }

        self.line("</div>");
    }

    /// An audio block.
    fn audio<'src>(&mut self, block: &'src Block<'src>, media: &'src MediaBlock<'src>) {
        let macro_attrs = media.macro_attrlist();
        let named = |name: &str| -> Option<&str> {
            macro_attrs.named_attribute(name).map(|attr| attr.value())
        };

        let src = escape_quotes(&resolve_web_path(
            &escape_attribute(media.resolved_target()),
            &self.imagesdir,
        ));
        let time_anchor = time_anchor(named("start"), named("end"));

        let autoplay = if macro_attrs.has_option("autoplay") {
            " autoplay"
        } else {
            ""
        };
        let controls = if macro_attrs.has_option("nocontrols") {
            ""
        } else {
            " controls"
        };
        let loop_attr = if macro_attrs.has_option("loop") {
            " loop"
        } else {
            ""
        };

        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute("audio", &media_roles(block, media))
        ));
        if let Some(title) = block.title() {
            self.line(&format!("<div class=\"title\">{title}</div>"));
        }
        self.line("<div class=\"content\">");
        self.line(&format!(
            "<audio src=\"{src}{time_anchor}\"{autoplay}{controls}{loop_attr}>"
        ));
        self.line("Your browser does not support the audio tag.");
        self.line("</audio>");
        self.line("</div>");
        self.line("</div>");
    }

    // ---------------------------------------------------------------------
    // Shared helpers
    // ---------------------------------------------------------------------

    /// Opens `<div id=… class="<base> <roles>">` for a leaf block wrapper.
    fn open_block_wrapper<'src>(&mut self, block: &'src Block<'src>, base_class: &str) {
        self.line(&format!(
            "<div{}{}>",
            id_attribute(block.id()),
            class_attribute(base_class, &block.roles())
        ));
    }

    /// Opens a block wrapper whose class list carries an extra style token
    /// between the base class and the roles.
    fn open_block_wrapper_with_style<'src>(
        &mut self,
        block: &'src Block<'src>,
        base_class: &str,
        style: &str,
    ) {
        let mut classes: Vec<&str> = vec![base_class, style];
        classes.extend(block.roles());
        self.line(&format!(
            "<div{} class=\"{}\">",
            id_attribute(block.id()),
            class_list(&classes)
        ));
    }

    /// The block's `<div class="title">…</div>`, if it has a title.
    fn block_title<'src>(&mut self, block: &'src Block<'src>) {
        if let Some(title) = block.title() {
            self.line(&format!("<div class=\"title\">{title}</div>"));
        }
    }

    /// A *captioned* block's title div, with the caption prefix ahead of the
    /// title text.
    fn captioned_title<'src>(&mut self, block: &'src Block<'src>) {
        if let Some(title) = block.title() {
            let caption = block.caption().unwrap_or_default();
            self.line(&format!("<div class=\"title\">{caption}{title}</div>"));
        }
    }

    /// A visible placeholder for a construct this backend does not handle,
    /// keeping the output well-formed while making the gap obvious.
    fn unsupported(&mut self, context: &str) {
        self.line(&format!(
            "<!-- asciidoc-waterlens-html5: unsupported block context '{context}' -->"
        ));
    }
}

// -------------------------------------------------------------------------
// Free helpers
// -------------------------------------------------------------------------

/// A description list entry: the terms that share one description, and the
/// item carrying that description (absent for a trailing term-only entry).
type DlistEntry<'src> = (Vec<String>, Option<&'src ListItem<'src>>);

/// Regroups a description list's items into Asciidoctor's `[terms, dd]` pairs.
///
/// The parser models each `term::` line as its own list item, so consecutive
/// terms that share one description surface as term-only items (no child
/// blocks) followed by the item that carries the description.
fn dlist_entries<'src>(list: &'src ListBlock<'src>) -> Vec<DlistEntry<'src>> {
    let mut entries: Vec<DlistEntry<'src>> = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for item in list.child_blocks().filter_map(as_list_item) {
        pending.extend(dlist_term_text(item));

        // A described item (one with child blocks) closes the entry, taking
        // the terms accumulated so far with it.
        if item.child_blocks().next().is_some() {
            entries.push((std::mem::take(&mut pending), Some(item)));
        }
    }

    // Any terms left over had no description of their own.
    if !pending.is_empty() {
        entries.push((pending, None));
    }

    entries
}

/// Narrows a block to a list item, or `None` when it is something else.
fn as_list_item<'src>(block: &'src Block<'src>) -> Option<&'src ListItem<'src>> {
    match block {
        Block::ListItem(list_item) => Some(list_item),
        _ => None,
    }
}

/// The index of the first list item that carries list-level attributes, or
/// `None` when no item does.
///
/// Asciidoctor attaches attributes written above a list to the *list* — a
/// list item cannot carry an id, roles, a style, or a `start` option in the
/// source (an attribute line on an item's own line is read as literal text).
/// `asciidoc-parser` misbehaves only when such an attribute-decorated list
/// follows a list whose last item has a nested list: it merges the new list
/// into the old one and hangs the attributes on the merged segment's first
/// item. The ulist/olist renderers split at that item, restoring the separate
/// list Asciidoctor produces.
fn split_list_at_attributes(items: &[&Block<'_>]) -> Option<usize> {
    items.iter().position(|item| {
        item.id().is_some()
            || !item.roles().is_empty()
            || item.declared_style().is_some()
            || item
                .attrlist()
                .is_some_and(|attrlist| attrlist.named_attribute("start").is_some())
            || item
                .attrlist()
                .is_some_and(|attrlist| attrlist.has_option("reversed"))
    })
}

/// Whether `item`'s marker makes it an ordered-list item (`.`, `a.`, `i)`,
/// `7.`, …) rather than an unordered or description item.
fn item_is_ordered(item: &Block<'_>) -> bool {
    matches!(
        item,
        Block::ListItem(list_item)
            if matches!(
                list_item.list_item_marker(),
                ListItemMarker::Dots(_)
                    | ListItemMarker::ArabicNumeral(_)
                    | ListItemMarker::AlphaListLower(_)
                    | ListItemMarker::AlphaListCapital(_)
                    | ListItemMarker::RomanNumeralLower(_)
                    | ListItemMarker::RomanNumeralUpper(_)
            )
    )
}

/// The numbering style an ordered-list item's marker implies — the per-item
/// counterpart of the parser's `ListBlock::marker_style`.
fn ordered_list_marker_style(item: &Block<'_>) -> Option<&'static str> {
    let Block::ListItem(list_item) = item else {
        return None;
    };
    match list_item.list_item_marker() {
        ListItemMarker::Dots(span) => match span.data().len() {
            2 => Some("loweralpha"),
            3 => Some("lowerroman"),
            4 => Some("upperalpha"),
            5 => Some("upperroman"),
            _ => Some("arabic"),
        },
        ListItemMarker::ArabicNumeral(_) => Some("arabic"),
        ListItemMarker::AlphaListLower(_) => Some("loweralpha"),
        ListItemMarker::AlphaListCapital(_) => Some("upperalpha"),
        ListItemMarker::RomanNumeralLower(_) => Some("lowerroman"),
        ListItemMarker::RomanNumeralUpper(_) => Some("upperroman"),
        _ => None,
    }
}

/// The rendered term text of a description-list item.
fn dlist_term_text(list_item: &ListItem<'_>) -> Option<String> {
    match list_item.list_item_marker() {
        ListItemMarker::DefinedTerm { term, .. } => Some(term.rendered().to_string()),
        _ => None,
    }
}

/// The `stripes-<value>` class for a table, or `None` when no striping
/// applies.
fn table_stripes_class(table: &TableBlock<'_>) -> Option<String> {
    let has_attr = table
        .attrlist()
        .and_then(|a| a.named_attribute("stripes"))
        .is_some();
    let stripes = table.stripes();
    if !has_attr && stripes == Stripes::None {
        return None;
    }

    let value = match stripes {
        Stripes::None => "none",
        Stripes::Even => "even",
        Stripes::Odd => "odd",
        Stripes::All => "all",
        Stripes::Hover => "hover",
    };
    Some(format!("stripes-{value}"))
}

/// A `labelwidth`/`itemwidth` value for a horizontal description list, with
/// any trailing `%` removed.
fn dlist_width(list: &ListBlock<'_>, name: &str) -> Option<String> {
    list.attrlist()
        .and_then(|attrlist| attrlist.named_attribute(name))
        .map(|attr| attr.value().trim_end_matches('%').to_string())
}

/// A `<col>` for a horizontal description list, sized when a width was given.
fn dlist_col(width: Option<&str>) -> String {
    match width {
        Some(width) => format!("<col style=\"width: {}%;\">", escape_attribute(width)),
        None => "<col>".to_string(),
    }
}

/// The numbering style of an ordered list.
fn olist_style<'src>(block: &'src Block<'src>, list: &'src ListBlock<'src>) -> &'src str {
    block
        .declared_style()
        .or_else(|| list.marker_style())
        .unwrap_or("arabic")
}

/// The role(s) placed on a media block's wrapper. A `role=` attribute inside
/// the macro wins outright over the block's shorthand role(s).
fn media_roles<'src>(block: &'src Block<'src>, media: &'src MediaBlock<'src>) -> Vec<&'src str> {
    match media.macro_attrlist().named_attribute("role") {
        Some(role) => role.value().split_whitespace().collect(),
        None => block.roles(),
    }
}

/// The `#t=start,end` media fragment for a timed audio or video source.
fn time_anchor(start: Option<&str>, end: Option<&str>) -> String {
    if start.is_none() && end.is_none() {
        return String::new();
    }
    let start = escape_attribute(start.unwrap_or_default());
    let end = end
        .map(escape_attribute)
        .map(|end| format!(",{end}"))
        .unwrap_or_default();
    format!("#t={start}{end}")
}

/// Removes leading and trailing blank lines from `lines`.
fn strip_surrounding_blank_lines(lines: &mut Vec<&str>) {
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
}

/// Rewrites the internal breaks of a multi-line AsciiMath equation, closing
/// and reopening the delimiter pair around each `<br>` so each expression
/// stands alone. Mirrors Asciidoctor's `StemBreakRx` pass.
fn rewrite_asciimath_breaks(equation: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(equation.len());
    let mut rest = equation;

    while let Some(start) = rest.find('\n') {
        // Expand to the whole run of whitespace containing this newline.
        let run_start = rest[..start]
            .rfind(|ch: char| ch != ' ' && ch != '\\')
            .map(|index| index + 1)
            .unwrap_or(0);
        let mut run_end = start;
        for ch in rest[start..].chars() {
            if ch == '\n' || ch == '\\' || ch == ' ' {
                run_end += ch.len_utf8();
            } else {
                break;
            }
        }

        let run = &rest[run_start..run_end];
        let breaks = run.matches('\n').count();

        // A single newline that is neither a blank-line run nor a trailing
        // backslash continuation is an ordinary line break inside the
        // expression, which Asciidoctor leaves alone.
        if breaks < 2 && !run.contains('\\') {
            out.push_str(&rest[..run_end]);
            rest = &rest[run_end..];
            continue;
        }

        out.push_str(&rest[..run_start]);
        out.push_str(close);
        for _ in 1..breaks {
            out.push_str("\n<br>");
        }
        out.push('\n');
        out.push_str(open);
        rest = &rest[run_end..];
    }

    out.push_str(rest);
    out
}

/// The default `alt` text for an image: the target's basename with `_`/`-`
/// turned into spaces and the extension dropped.
fn default_alt(target: &str) -> String {
    let spaced = target.replace(['_', '-'], " ");
    let last_segment = spaced.rsplit(['/', '\\']).next().unwrap_or(&spaced);

    match last_segment.rfind('.') {
        Some(dot) if dot > 0 => last_segment[..dot].to_string(),
        _ => last_segment.to_string(),
    }
}

/// Prepares an SVG document for inlining, mirroring Asciidoctor's
/// `read_svg_contents`: strips everything before the `<svg` start tag, and —
/// when `width`/`height` are given — rewrites the start tag, dropping its own
/// `width`/`height`/`style` attributes and appending the requested ones.
///
/// Returns `None` when the file carries no `<svg` element (where Asciidoctor
/// would emit the raw file; rendering the alt text is the safer failure).
fn inline_svg(svg: &str, width: Option<&str>, height: Option<&str>) -> Option<String> {
    let svg = strip_svg_preamble(svg)?;
    let start_tag_end = svg.find('>')?;
    let old_start_tag = &svg[..=start_tag_end];

    let has_dimensions = width.is_some() || height.is_some();
    let mut new_start_tag = old_start_tag.to_string();
    if has_dimensions {
        new_start_tag = strip_dimension_attributes(&new_start_tag);
        for (dim, value) in [("width", width), ("height", height)] {
            if let Some(value) = value {
                // `chop` removes the trailing `>`; the attribute is appended
                // before it.
                new_start_tag = format!(
                    "{} {dim}=\"{}\">",
                    &new_start_tag[..new_start_tag.len() - 1],
                    escape_attribute(value)
                );
            }
        }
    }

    if has_dimensions {
        Some(format!("{new_start_tag}{}", &svg[old_start_tag.len()..]))
    } else {
        Some(svg.to_string())
    }
}

/// The slice of `svg` starting at its `<svg` start tag, or `None` when the
/// file has none — the analog of Asciidoctor's `SvgPreambleRx` strip.
fn strip_svg_preamble(svg: &str) -> Option<&str> {
    let mut search_from = 0;
    while let Some(relative) = svg[search_from..].find("<svg") {
        let at = search_from + relative;
        let after = at + "<svg".len();
        let follows = svg.as_bytes().get(after);
        if after >= svg.len()
            || matches!(
                follows,
                Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x0b' | b'\x0c' | b'>')
            )
        {
            return Some(&svg[at..]);
        }
        search_from = at + 1;
    }
    None
}

/// Removes every ` width=…`, ` height=…`, or ` style=…` attribute from an SVG
/// start tag — Asciidoctor's `DimensionAttributeRx` pass.
fn strip_dimension_attributes(tag: &str) -> String {
    let mut out = String::new();
    let mut rest = tag;

    while let Some((start, value_start)) = find_dimension_attribute(rest) {
        let value = &rest[value_start..];
        let Some(quote @ ('"' | '\'')) = value.chars().next() else {
            out.push_str(&rest[..value_start]);
            rest = &rest[value_start..];
            continue;
        };
        let Some(closing_quote) = value[quote.len_utf8()..].find(quote) else {
            out.push_str(&rest[..value_start]);
            rest = &rest[value_start..];
            continue;
        };

        out.push_str(&rest[..start]);
        rest = &value[quote.len_utf8() + closing_quote + quote.len_utf8()..];
    }

    out.push_str(rest);
    out
}

/// Finds the next whitespace-prefixed `width=`, `height=`, or `style=` and
/// returns the attribute start plus the start of its quoted value.
fn find_dimension_attribute(tag: &str) -> Option<(usize, usize)> {
    for (start, whitespace) in tag.char_indices() {
        if !whitespace.is_ascii_whitespace() {
            continue;
        }

        let name_start = start + whitespace.len_utf8();
        let candidate = &tag[name_start..];
        let Some(name) = ["width", "height", "style"]
            .into_iter()
            .find(|name| candidate.starts_with(name))
        else {
            continue;
        };

        let equals = name_start + name.len();
        if tag.as_bytes().get(equals) == Some(&b'=') {
            return Some((start, equals + 1));
        }
    }

    None
}

/// The `target`/`rel` attributes a link's `window` and `nofollow`/`noopener`
/// options contribute.
fn link_constraint_attrs(window: Option<&str>, nofollow: bool, noopener: bool) -> String {
    let rel = if nofollow { Some("nofollow") } else { None };

    match window {
        Some(window) => {
            let rel_noopener = if window == "_blank" || noopener {
                match rel {
                    Some(rel) => format!(" rel=\"{rel} noopener\""),
                    None => " rel=\"noopener\"".to_string(),
                }
            } else {
                String::new()
            };
            format!(" target=\"{}\"{rel_noopener}", escape_attribute(window))
        }
        None => match rel {
            Some(rel) => format!(" rel=\"{rel}\""),
            None => String::new(),
        },
    }
}

/// Splits a cell's rendered content into `<p class="table">` paragraphs,
/// applying the column style's inline wrapper.
fn cell_paragraphs(cell: &TableCell<'_>, content: &str) -> String {
    let (open, close) = match cell.style() {
        ColumnStyle::Emphasis => ("<em>", "</em>"),
        ColumnStyle::Strong => ("<strong>", "</strong>"),
        ColumnStyle::Monospace => ("<code>", "</code>"),
        _ => ("", ""),
    };
    let wrap = |para: &str| {
        if open.is_empty() {
            para.to_string()
        } else {
            format!("{open}{para}{close}")
        }
    };

    let raw = cell.span().data().trim();
    let paragraphs: Vec<String> = if raw.contains("\n\n") {
        content
            .split("\n\n")
            .map(str::trim)
            .filter(|para| !para.is_empty())
            .map(wrap)
            .collect()
    } else if content.is_empty() {
        vec![]
    } else {
        vec![wrap(content)]
    };

    if paragraphs.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"table\">{}</p>",
            paragraphs.join("</p>\n<p class=\"table\">")
        )
    }
}

/// Truncates to four decimal places, matching Asciidoctor's column width
/// arithmetic.
fn truncate4(value: f64) -> f64 {
    (value * 10_000.0).trunc() / 10_000.0
}

/// Rounds half-up to four decimal places.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Formats a percentage width the way Asciidoctor does: an integral value
/// prints without a fractional part.
fn format_pcwidth(value: f64) -> String {
    if (value - value.round()).abs() < f64::EPSILON {
        format!("{}", value.round() as i64)
    } else {
        let formatted = format!("{value:.4}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// The proportional percentage width of each table column.
fn column_pcwidths(columns: &[TableColumn]) -> Vec<String> {
    let n = columns.len();

    let autowidth_count = columns.iter().filter(|c| c.is_autowidth()).count();
    let width_base: f64 = columns
        .iter()
        .filter(|c| !c.is_autowidth())
        .map(|c| c.width() as f64)
        .sum();

    let mut base = width_base;
    let mut autowidth_value = 0.0;
    if autowidth_count > 0 && width_base <= 100.0 {
        autowidth_value = truncate4((100.0 - width_base) / autowidth_count as f64);
        base = 100.0;
    }

    let mut pcwidths = vec![0.0_f64; n];
    let mut total = 0.0_f64;
    let mut last = 0.0_f64;
    for (i, col) in columns.iter().enumerate() {
        let width = if col.is_autowidth() {
            autowidth_value
        } else {
            col.width() as f64
        };
        let pc = truncate4(width * 100.0 / base);
        pcwidths[i] = pc;
        total += pc;
        last = pc;
    }

    if n > 0 && (total - 100.0).abs() > f64::EPSILON {
        pcwidths[n - 1] = round4(100.0 - total + last);
    }

    pcwidths.iter().map(|pc| format_pcwidth(*pc)).collect()
}
