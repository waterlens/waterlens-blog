//! Renderer tests, grouped by construct.
//!
//! Each expectation here was taken from the output of the Ruby `w-html`
//! converter this crate replaces, so the suite doubles as a record of the
//! markup contract the site's stylesheet is written against.

use crate::{Options, convert, convert_with};

/// Renders `source` as a complete page.
fn page(source: &str) -> String {
    convert_with(source, &Options::new().standalone(true))
}

/// Renders `source` body-only, which is what the block tests assert on.
fn body(source: &str) -> String {
    convert(source)
}

mod document_shell {
    use super::{Options, convert_with, page};

    #[test]
    fn emits_the_site_head_and_shell() {
        let html = page("= Title\n\nBody.");

        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n"));
        assert!(html.contains("<link rel=\"stylesheet\" href=\"/style.css\">"));
        assert!(html.contains("<title>Title</title>"));
        assert!(html.contains("<article>"));
        assert!(html.contains("<div id=\"content\">"));
        assert!(html.ends_with("</html>"));
    }

    #[test]
    fn the_lang_attribute_follows_the_lang_document_attribute() {
        assert!(page("= Title\n:lang: zh-hans\n\nBody.").contains("<html lang=\"zh-hans\">"));
    }

    #[test]
    fn nolang_drops_the_lang_attribute() {
        assert!(page("= Title\n:nolang:\n\nBody.").contains("<html>\n<head>"));
    }

    #[test]
    fn katex_is_linked_only_for_a_stem_document() {
        assert!(!page("= Title\n\nBody.").contains("katex"));
        assert!(page("= Title\n:stem: latexmath\n\nBody.").contains("katex.min.css"));
    }

    #[test]
    fn mermaid_is_linked_only_when_requested() {
        assert!(!page("= Title\n\nBody.").contains("mermaid"));
        assert!(page("= Title\n:mermaid:\n\nBody.").contains("mermaid.min.js"));
    }

    // `shownav` swaps the `<article>` wrapper for a navigation bar: index-style
    // pages get the nav, prose pages get the wrapper.
    #[test]
    fn shownav_replaces_the_article_wrapper_with_navigation() {
        let html = page("= Title\n:shownav:\n\nBody.");
        assert!(!html.contains("<article>"));
        assert!(html.contains("<nav>"));
        assert!(html.contains("<a href=\"/posts.html\">Posts</a>"));
    }

    #[test]
    fn navigation_is_localized_by_the_lang_attribute() {
        let html = page("= Title\n:shownav:\n:lang: zh-hans\n\nBody.");
        assert!(html.contains("<a href=\"/zh/posts.html\">文章</a>"));
        assert!(html.contains("<a href=\"/index.html\">English</a>"));
    }

    #[test]
    fn metadata_tags_follow_their_document_attributes() {
        let html =
            page("= Title\n:description: A page\n:author: Waterlens\n:keywords: a, b\n\nBody.");
        assert!(html.contains("<meta name=\"description\" content=\"A page\">"));
        assert!(html.contains("<meta name=\"author\" content=\"Waterlens\">"));
        assert!(html.contains("<meta name=\"keywords\" content=\"a, b\">"));
    }

    #[test]
    fn pagetitle_overrides_the_title_element_but_not_the_heading() {
        let html = page("= Real Title\n:pagetitle: Other\n\nBody.");
        assert!(html.contains("<title>Other</title>"));
        assert!(html.contains("<h1>Real Title</h1>"));
    }

    #[test]
    fn the_title_element_strips_markup_from_the_doctitle() {
        let html = page("= xref:.[Home]\n\nBody.");
        assert!(html.contains("<title>Home</title>"));
        assert!(html.contains("<h1><a href=\".\">Home</a></h1>"));
    }

    #[test]
    fn a_subtitle_becomes_a_second_heading() {
        let html = page("= Notes: Volume One\n\nBody.");
        assert!(html.contains("<h1>Notes</h1>"));
        assert!(html.contains("<h2 class=\"subtitle\">Volume One</h2>"));
    }

    #[test]
    fn the_footer_is_localized_and_can_be_suppressed() {
        assert!(page("= T\n\nBody.").contains("CC BY-SA 4.0"));
        assert!(page("= T\n:lang: zh-hans\n\nBody.").contains("知识共享"));
        assert!(!page("= T\n:nofooter:\n\nBody.").contains("<footer>"));
    }

    #[test]
    fn max_width_styles_the_content_wrapper() {
        assert!(
            page("= T\n:max-width: 50rem\n\nBody.")
                .contains("<div id=\"content\" style=\"max-width: 50rem;\">")
        );
    }

    #[test]
    fn embedded_output_has_no_page_shell() {
        let html = convert_with("= T\n\nBody.", &Options::new());
        assert!(!html.contains("<!DOCTYPE"));
        assert!(!html.contains("<footer>"));
        assert_eq!(html, "<p>Body.</p>");
    }
}

mod paragraphs {
    use super::body;

    // The defining difference from Asciidoctor's `html5` backend: a plain
    // paragraph is a bare `<p>`, with no wrapper div.
    #[test]
    fn a_plain_paragraph_is_a_bare_p_element() {
        assert_eq!(body("Just text."), "<p>Just text.</p>");
    }

    #[test]
    fn a_role_adds_the_paragraph_class_alongside_it() {
        assert_eq!(
            body("[.centered]\nText."),
            "<p class=\"paragraph centered\">Text.</p>"
        );
    }

    #[test]
    fn an_id_alone_stays_on_the_bare_p_element() {
        assert_eq!(body("[#intro]\nText."), "<p id=\"intro\">Text.</p>");
    }

    #[test]
    fn a_title_promotes_the_paragraph_to_a_wrapper_div() {
        assert_eq!(
            body(".A title\nText."),
            "<div class=\"paragraph\">\n<div class=\"title\">A title</div>\n<p>Text.</p>\n</div>"
        );
    }

    #[test]
    fn cjk_line_breaks_are_collapsed_in_paragraph_text() {
        assert_eq!(body("中文\n测试"), "<p>中文测试</p>");
    }

    #[test]
    fn latin_line_breaks_are_left_alone() {
        assert_eq!(body("hello\nworld"), "<p>hello\nworld</p>");
    }
}

mod sections {
    use super::body;

    #[test]
    fn a_section_nests_in_a_section_element() {
        let html = body("= Doc\n\n== One\n\nText.");
        assert!(html.contains("<section class=\"sect1\">"));
        assert!(html.contains("<h2 id=\"_one\">One</h2>"));
        assert!(html.contains("</section>"));

        // Unlike the stock backend, there is no `sectionbody` wrapper.
        assert!(!html.contains("sectionbody"));
    }

    #[test]
    fn a_discrete_heading_is_a_bare_heading_with_its_style_as_a_class() {
        let html = body("= Doc\n\n[discrete]\n== Aside\n\nText.");
        assert!(html.contains("<h2 id=\"_aside\" class=\"discrete\">Aside</h2>"));
        assert!(!html.contains("<section"));
    }

    #[test]
    fn a_discrete_heading_keeps_its_roles_after_the_style() {
        assert!(
            body("= Doc\n\n[discrete.centered]\n== Aside\n\nText.")
                .contains("class=\"discrete centered\"")
        );
    }

    #[test]
    fn the_preamble_renders_without_a_wrapper() {
        let html = body("= Doc\n\nLead text.\n\n== One\n\nText.");
        assert!(!html.contains("id=\"preamble\""));
        assert!(html.contains("<p>Lead text.</p>"));
    }
}

mod verbatim {
    use super::body;

    #[test]
    fn a_plain_listing_block_uses_the_short_class_name() {
        assert_eq!(
            body("----\ncode\n----"),
            "<div class=\"listing\">\n<div class=\"content\">\n<pre>code</pre>\n</div>\n</div>"
        );
    }

    #[test]
    fn a_source_block_names_its_language_on_the_code_element() {
        assert!(body("[source,rust]\n----\nfn main() {}\n----").contains(
            "<pre class=\"highlight\"><code class=\"language-rust\" data-lang=\"rust\">"
        ));
    }

    // The `[,lang]` shorthand is a source block too, which the blog relies on.
    #[test]
    fn the_comma_shorthand_is_a_source_block() {
        assert!(body("[,haskell]\n----\nid x = x\n----").contains("class=\"language-haskell\""));
    }

    // With highlight.js active the adapter reshapes the attributes and moves
    // `data-lang` to the end.
    #[test]
    fn highlight_js_reshapes_the_pre_and_code_attributes() {
        let html = body(
            "= Doc\n:source-highlighter: highlight.js\n\n[source,ocaml]\n----\nlet x = 1\n----",
        );
        assert!(html.contains(
            "<pre class=\"highlightjs highlight\"><code class=\"language-ocaml hljs\" data-noescape=\"true\" data-lang=\"ocaml\">"
        ));
    }

    #[test]
    fn highlight_js_falls_back_to_a_none_language() {
        assert!(
            body("= Doc\n:source-highlighter: highlightjs\n\n[source]\n----\nx\n----")
                .contains("<code class=\"language-none hljs\" data-noescape=\"true\">")
        );
    }

    // The footer docinfo is asserted as one exact block: its interior blank
    // line comes from the adapter's template and is easy to lose, and a
    // `contains`-only check cannot see it.
    #[test]
    fn highlight_js_links_its_assets_after_the_footer() {
        let cdn = "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1";
        let html = super::page(
            "= Doc\n:source-highlighter: highlightjs\n:highlightjs-languages: ocaml, scheme\n\nBody.",
        );

        assert!(html.contains(&format!(
            "<link rel=\"stylesheet\" href=\"{cdn}/styles/atom-one-light.min.css\">\n\
             <script src=\"{cdn}/highlight.min.js\"></script>\n\
             <script src=\"{cdn}/languages/ocaml.min.js\"></script>\n\
             <script src=\"{cdn}/languages/scheme.min.js\"></script>\n\
             \n\
             <script>\n\
             hljs.configure({{ignoreUnescapedHTML: true}});\n\
             hljs.highlightAll();\n\
             </script>"
        )));
    }

    // A document with no `highlightjs-languages` still gets the blank line,
    // because the template's newline stands alone there.
    #[test]
    fn highlight_js_footer_keeps_its_blank_line_without_languages() {
        let html = super::page("= Doc\n:source-highlighter: highlightjs\n\nBody.");
        assert!(html.contains("/highlight.min.js\"></script>\n\n<script>\n"));
    }

    #[test]
    fn a_literal_block_uses_the_short_class_name() {
        assert!(body("....\nliteral\n....").starts_with("<div class=\"literal\">"));
    }

    #[test]
    fn a_passthrough_block_is_emitted_raw() {
        assert_eq!(body("++++\n<hr class=\"x\">\n++++"), "<hr class=\"x\">");
    }

    #[test]
    fn a_stem_block_is_wrapped_in_its_math_delimiters() {
        let html = body("= Doc\n:stem: latexmath\n\n[stem]\n++++\nx = 1\n++++");
        assert!(html.contains("<div class=\"stem\">"));
        assert!(html.contains(r"\[x = 1\]"));
    }
}

mod wrappers {
    use super::body;

    #[test]
    fn an_open_block_uses_the_short_class_name() {
        assert!(body("--\nText.\n--").starts_with("<div class=\"open\">\n<div class=\"content\">"));
    }

    #[test]
    fn an_open_block_keeps_its_roles() {
        assert!(body("[.centered]\n--\nText.\n--").starts_with("<div class=\"open centered\">"));
    }

    #[test]
    fn an_admonition_renders_as_a_two_cell_table() {
        let html = body("TIP: Be careful.");
        assert!(html.starts_with("<div class=\"admonition tip\">"));
        assert!(html.contains("<td class=\"icon\">\n<div class=\"title\">Tip</div>"));
        // A one-line admonition has simple content, which is emitted
        // unwrapped — no `<p>` — matching Asciidoctor's `node.content`.
        assert!(html.contains("<td class=\"content\">\nBe careful.\n</td>"));
    }

    #[test]
    fn a_quote_block_carries_its_attribution() {
        let html = body("[quote,Someone,A Book]\n____\nWords.\n____");
        assert!(html.contains("<div class=\"quoteblock\">"));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("&#8212; Someone<br>"));
        assert!(html.contains("<cite>A Book</cite>"));
    }

    #[test]
    fn a_verse_block_uses_the_short_class_name() {
        assert!(body("[verse]\n____\nA line.\n____").starts_with("<div class=\"verse\">"));
    }

    #[test]
    fn an_example_block_uses_the_short_class_name() {
        assert!(body("====\nText.\n====").starts_with("<div class=\"example\">"));
    }

    #[test]
    fn a_sidebar_places_its_title_inside_the_content() {
        assert!(body(".Aside\n****\nText.\n****").starts_with(
            "<div class=\"sidebar\">\n<div class=\"content\">\n<div class=\"title\">Aside</div>"
        ));
    }
}

mod lists {
    use super::body;

    #[test]
    fn an_unordered_list_wraps_each_item_text_in_a_paragraph() {
        assert_eq!(
            body("* one\n* two"),
            "<div class=\"ulist\">\n<ul>\n<li>\n<p>one</p>\n</li>\n<li>\n<p>two</p>\n</li>\n</ul>\n</div>"
        );
    }

    #[test]
    fn cjk_line_breaks_are_collapsed_in_list_item_text() {
        assert!(body("* 中文\n测试").contains("<p>中文测试</p>"));
    }

    #[test]
    fn an_ordered_list_names_its_numbering_style() {
        let html = body(". one\n. two");
        assert!(html.starts_with("<div class=\"olist arabic\">"));
        assert!(html.contains("<ol class=\"arabic\">"));
    }

    #[test]
    fn a_description_list_tags_its_terms() {
        let html = body("term:: definition");
        assert!(html.starts_with("<div class=\"dlist\">"));
        assert!(html.contains("<dt class=\"hdlist1\">term</dt>"));
        assert!(html.contains("<dd>\n<p>definition</p>\n</dd>"));
    }

    #[test]
    fn a_checklist_marks_its_items() {
        let html = body("* [x] done\n* [ ] todo");
        assert!(html.starts_with("<div class=\"ulist checklist\">"));
        assert!(html.contains("<p>&#10003; done</p>"));
        assert!(html.contains("<p>&#10063; todo</p>"));
    }

    #[test]
    fn a_nested_list_renders_inside_its_parent_item() {
        assert!(body("* outer\n** inner").contains("<p>outer</p>\n<div class=\"ulist\">"));
    }
}

mod tables {
    use super::body;

    // Tables get a scroll wrapper and the short `table` class rather than
    // Asciidoctor's `tableblock`.
    #[test]
    fn a_table_is_wrapped_for_horizontal_scrolling() {
        let html = body("|===\n| a | b\n|===");
        assert!(html.starts_with("<div class=\"table-wrapper\">\n<table"));
        assert!(html.contains("class=\"table frame-all grid-all stretch\""));
        assert!(html.ends_with("</table>\n</div>"));
    }

    #[test]
    fn cells_carry_the_short_class_name_and_alignment() {
        let html = body("|===\n| a\n|===");
        assert!(html.contains("<td class=\"table halign-left valign-top\">"));
        assert!(html.contains("<p class=\"table\">a</p>"));
    }

    #[test]
    fn a_header_row_renders_plain_th_cells() {
        let html = body("[options=\"header\"]\n|===\n| h1 | h2\n| a | b\n|===");
        assert!(html.contains("<thead>"));
        assert!(html.contains("<th class=\"table halign-left valign-top\">h1</th>"));
    }
}

mod media {
    use super::body;

    #[test]
    fn a_block_image_renders_with_its_alt_text() {
        let html = body("image::/a/b.svg[Alt text]");
        assert!(html.starts_with("<div class=\"imageblock\">"));
        assert!(html.contains("<img src=\"/a/b.svg\" alt=\"Alt text\">"));
    }

    // A `rem` dimension cannot ride on the HTML attribute, so it becomes an
    // inline style — this is what lets the blog size diagrams to the text.
    #[test]
    fn a_rem_width_becomes_an_inline_style() {
        assert!(
            body("image::/a/b.svg[Alt, 40rem]")
                .contains("<img src=\"/a/b.svg\" alt=\"Alt\" style=\"width: 40rem;\">")
        );
    }

    #[test]
    fn a_pixel_width_stays_an_html_attribute() {
        assert!(
            body("image::/a/b.svg[Alt, 400]")
                .contains("<img src=\"/a/b.svg\" alt=\"Alt\" width=\"400\">")
        );
    }

    #[test]
    fn an_image_role_lands_on_the_wrapper() {
        assert!(
            body("[.centered]\nimage::/a/b.svg[Alt]").contains("class=\"imageblock centered\"")
        );
    }

    // `convert_video` is deliberately empty in this backend.
    #[test]
    fn a_video_block_renders_nothing() {
        assert_eq!(body("video::x[]"), "");
    }
}

mod breaks {
    use super::body;

    #[test]
    fn a_thematic_break_is_an_hr() {
        assert_eq!(body("'''"), "<hr>");
    }

    #[test]
    fn a_page_break_is_a_styled_div() {
        assert_eq!(
            body("<<<"),
            "<div style=\"page-break-after: always;\"></div>"
        );
    }
}

mod source_normalization {
    use super::body;

    // Asciidoctor's reader strips trailing whitespace from every source line,
    // so it never reaches the output — not even inside a verbatim block.
    #[test]
    fn trailing_whitespace_is_stripped_inside_a_listing() {
        assert_eq!(
            body("----\ncode   \nmore\t\n----"),
            "<div class=\"listing\">\n<div class=\"content\">\n<pre>code\nmore</pre>\n</div>\n</div>"
        );
    }

    #[test]
    fn a_trailing_plus_becomes_a_hard_line_break() {
        assert_eq!(body("one +\ntwo"), "<p>one<br>\ntwo</p>");
    }
}

mod document_attributes {
    use super::page;

    // Document attribute values arrive from the parser with special
    // characters already escaped; the meta tags must not escape them again.
    #[test]
    fn meta_tags_are_not_double_escaped() {
        let html =
            page("= T\n:description: Desc & More\n:keywords: a & b <c>\n:author: AT&T\n\nBody.");
        assert!(html.contains("<meta name=\"description\" content=\"Desc &amp; More\">"));
        assert!(html.contains("<meta name=\"keywords\" content=\"a &amp; b &lt;c&gt;\">"));
        assert!(html.contains("<meta name=\"author\" content=\"AT&amp;T\">"));
        assert!(!html.contains("&amp;amp;"));
    }

    #[test]
    fn empty_and_quoted_document_attributes_keep_their_value_semantics() {
        let html = page("= T\n:description:\n:max-width:\n:cellbgcolor:\n\n|===\n|cell\n|===");
        assert!(html.contains("<meta name=\"description\" content=\"\">"));
        assert!(html.contains("style=\"max-width: ;\""));
        assert!(html.contains("style=\"background-color: ;\""));

        let quoted = page("= T\n:description: A \"quote\" & more\n\nBody.");
        assert!(
            quoted
                .contains("<meta name=\"description\" content=\"A &quot;quote&quot; &amp; more\">")
        );
    }

    #[test]
    fn image_src_escapes_the_target_but_not_the_imagesdir() {
        let html = page("= T\n:imagesdir: /a&b\n\nimage::rel&c.png[Alt]");
        assert!(html.contains("<img src=\"/a&amp;b/rel&amp;c.png\" alt=\"Alt\">"));
        assert!(!html.contains("&amp;amp;"));
    }

    #[test]
    fn image_src_escapes_quotes_from_imagesdir() {
        let html = page("= T\n:imagesdir: /a\"b\n\nimage::image.png[Alt]");
        assert!(html.contains("<img src=\"/a&quot;b/image.png\" alt=\"Alt\">"));
    }

    // `:icons: font` glyphs need Font Awesome's stylesheet; without it the
    // `<i class="fa …">` elements render as nothing. Standalone output links
    // the CDN, like the Ruby converter's CLI.
    #[test]
    fn icons_font_links_font_awesome() {
        let html = page("= T\n:icons: font\n\nTIP: Careful.");
        assert!(html.contains(
            "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css"
        ));
        assert!(html.contains("<i class=\"fa icon-tip\" title=\"Tip\"></i>"));
    }

    #[test]
    fn icons_font_without_remote_links_a_local_stylesheet() {
        let html =
            page("= T\n:icons: font\n:iconfont-remote!:\n:iconfont-name: custom\n\nTIP: Careful.");
        assert!(html.contains("<link rel=\"stylesheet\" href=\"./custom.css\">"));
        assert!(!html.contains("cdnjs"));
    }
}

mod stylesheet_attribute {
    use super::{Options, convert_with, page};
    use crate::SafeMode;

    #[test]
    fn a_missing_stylesheet_yields_an_empty_style_element() {
        let html = page("= T\n:stylesheet: missing.css\n\nBody.");
        assert!(html.contains("<style>\n\n</style>"));
    }

    #[test]
    fn an_existing_stylesheet_is_inlined() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("site.css"), "body { color: red; }").expect("write");
        let html = convert_with(
            "= T\n:stylesheet: site.css\n\nBody.",
            &Options::new()
                .standalone(true)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<style>\nbody { color: red; }\n</style>"));
    }

    #[test]
    fn linkcss_links_the_stylesheet() {
        let html = page("= T\n:stylesheet: custom.css\n:linkcss:\n\nBody.");
        assert!(html.contains("<link rel=\"stylesheet\" href=\"./custom.css\">"));
    }

    #[test]
    fn the_default_stylesheet_key_selects_the_webfonts_link() {
        let html = page("= T\n:stylesheet: DEFAULT\n:webfonts: Open+Sans\n\nBody.");
        assert!(html.contains(
            "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css?family=Open+Sans\">"
        ));
    }

    #[test]
    fn an_empty_stylesheet_value_selects_the_webfonts_link() {
        let html = page("= T\n:stylesheet:\n:webfonts: Open+Sans\n\nBody.");
        assert!(html.contains(
            "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css?family=Open+Sans\">"
        ));
    }

    #[test]
    fn a_uri_stylesdir_preserves_its_authority_separator() {
        let html = page(
            "= T\n:stylesheet: custom.css\n:stylesdir: https://cdn.example/css\n:linkcss:\n\nBody.",
        );
        assert!(
            html.contains("<link rel=\"stylesheet\" href=\"https://cdn.example/css/custom.css\">")
        );
    }

    #[test]
    fn an_escaped_stylesheet_path_is_decoded_for_file_lookup() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("site&theme.css"), "body { color: red; }")
            .expect("write stylesheet");
        let html = convert_with(
            "= T\n:stylesheet: site&theme.css\n\nBody.",
            &Options::new()
                .standalone(true)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<style>\nbody { color: red; }\n</style>"));
    }

    #[test]
    fn safe_mode_does_not_read_an_absolute_stylesheet_outside_the_base() {
        let base = tempfile::tempdir().expect("base dir");
        let outside = tempfile::NamedTempFile::new().expect("outside stylesheet");
        std::fs::write(outside.path(), "secret { display: block; }").expect("write stylesheet");
        let source = format!("= T\n:stylesheet: {}\n\nBody.", outside.path().display());
        let html = convert_with(
            &source,
            &Options::new()
                .standalone(true)
                .safe_mode(SafeMode::Safe)
                .input_file(base.path().join("doc.adoc")),
        );
        assert!(html.contains("<style>\n\n</style>"));
        assert!(!html.contains("secret { display: block; }"));
    }
}

mod include_directives {
    use super::{Options, convert_with};
    use crate::{SafeMode, load_with};

    #[test]
    fn an_include_resolves_against_the_primary_file_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("part.adoc"), "Included body.").expect("write");
        let html = convert_with(
            "= Doc\n\ninclude::part.adoc[]",
            &Options::new()
                .standalone(true)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<p>Included body.</p>"));
        assert!(!html.contains("Unresolved directive"));
    }

    #[test]
    fn a_nested_include_resolves_against_the_enclosing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("sub")).expect("mkdir");
        std::fs::write(dir.path().join("sub/outer.adoc"), "include::inner.adoc[]").expect("write");
        std::fs::write(dir.path().join("sub/inner.adoc"), "Inner body.").expect("write");
        let html = convert_with(
            "= Doc\n\ninclude::sub/outer.adoc[]",
            &Options::new()
                .standalone(true)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<p>Inner body.</p>"));
    }

    #[test]
    fn a_missing_include_reports_unresolved_directive() {
        let dir = tempfile::tempdir().expect("temp dir");
        let html = convert_with(
            "= Doc\n\ninclude::nope.adoc[]",
            &Options::new()
                .standalone(true)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("Unresolved directive"));
    }

    #[test]
    fn safe_mode_confines_absolute_includes_to_the_primary_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let html = convert_with(
            "= Doc\n\ninclude::/etc/hosts[]",
            &Options::new()
                .standalone(true)
                .safe_mode(SafeMode::Safe)
                .input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("Unresolved directive"));
        assert!(!html.contains("localhost"));
    }

    #[test]
    fn a_utf8_bom_is_removed_from_included_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("part.adoc"), "\u{feff}Included body.").expect("write");
        let html = convert_with(
            "= Doc\n\ninclude::part.adoc[]",
            &Options::new().input_file(dir.path().join("doc.adoc")),
        );
        assert_eq!(html, "<p>Included body.</p>");
    }

    #[test]
    fn a_non_utf8_include_reports_the_decoding_failure() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("part.adoc"), [0xff, 0xfe]).expect("write");
        let options = Options::new().input_file(dir.path().join("doc.adoc"));
        let document = load_with("= Doc\n\ninclude::part.adoc[]", &options);
        let warnings: Vec<String> = document
            .warnings()
            .map(|warning| warning.warning.to_string())
            .collect();
        assert!(warnings.iter().any(|warning| warning.contains("UTF-8")));
    }
}

mod verbatim_styles {
    use super::body;

    // Asciidoctor renders a delimited block per its delimiter: `----` is a
    // listing even under `[sidebar]`, `[quote]`, `[stem]`, `[comment]`, …
    #[test]
    fn styled_listing_blocks_render_as_listings() {
        for style in [
            "sidebar", "quote", "example", "open", "verse", "comment", "stem",
        ] {
            let html = body(&format!("[{style}]\n----\ncontent\n----"));
            assert!(
                html.starts_with("<div class=\"listing\">"),
                "{style} over ---- should render a listing, got: {html}"
            );
        }
    }

    #[test]
    fn literal_style_demotes_a_listing_to_a_literal() {
        let html = body("[literal]\n----\ncontent\n----");
        assert!(html.starts_with("<div class=\"literal\">"));
    }

    #[test]
    fn listing_style_upgrades_a_literal_to_a_listing() {
        let html = body("[listing]\n....\ncontent\n....");
        assert!(html.starts_with("<div class=\"listing\">"));
    }

    #[test]
    fn source_style_upgrades_a_literal_and_ignores_a_passthrough() {
        let html = body("[source,rust]\n....\nfn main() {}\n....");
        assert!(html.contains("<pre class=\"highlight\"><code class=\"language-rust\""));
        // On a passthrough the style is decoration: the content stays raw.
        assert_eq!(
            body("[source]\n++++\n<div>raw</div>\n++++"),
            "<div>raw</div>"
        );
    }

    // A `[comment]`-styled paragraph is a comment block; a `[comment]`-styled
    // delimited block is not.
    #[test]
    fn comment_style_drops_paragraphs_but_not_listings() {
        assert_eq!(body("[comment]\nA paragraph.\n"), "");
        let html = body("[comment]\n----\ncontent\n----");
        assert!(html.starts_with("<div class=\"listing\">"));
    }
}

mod merged_lists {
    use super::body;

    // `asciidoc-parser` merges an attribute-decorated list after a nested
    // list into the previous list and hangs the attributes on the merged
    // segment's first item. The renderer splits it back out.
    #[test]
    fn an_attributed_list_after_a_nested_list_splits_with_its_attributes() {
        let html = body("* a\n** nested\n\n[.foo]\n* c");
        assert_eq!(
            html,
            "<div class=\"ulist\">\n<ul>\n<li>\n<p>a</p>\n<div class=\"ulist\">\n<ul>\n<li>\n<p>nested</p>\n</li>\n</ul>\n</div>\n</li>\n</ul>\n</div>\n<div class=\"ulist foo\">\n<ul>\n<li>\n<p>c</p>\n</li>\n</ul>\n</div>"
        );
    }

    #[test]
    fn an_attributed_ordered_segment_keeps_its_numbering() {
        let html = body("* a\n** nested\n\n[loweralpha,start=3]\n. c");
        assert!(html.contains("<div class=\"olist loweralpha\">"));
        assert!(html.contains("<ol class=\"loweralpha\" type=\"a\" start=\"3\">"));
        assert!(html.contains("<p>c</p>"));
    }

    #[test]
    fn an_unordered_segment_split_from_an_olist_has_no_ordered_style() {
        let html = body(". a\n.. nested\n\n[.foo]\n* c");
        assert!(html.contains("<div class=\"ulist foo\">\n<ul>"));
        assert!(!html.contains("ulist arabic foo"));
        assert!(!html.contains("<ul class=\"arabic\">"));
    }

    #[test]
    fn a_split_ulist_does_not_inherit_the_previous_lists_style() {
        let html = body("[square]\n* a\n** nested\n\n[.foo]\n* c");
        assert!(html.contains("<div class=\"ulist foo\">\n<ul>"));
        assert!(!html.contains("ulist square foo"));
    }
}

mod svg_images {
    use super::{Options, convert_with};
    use crate::SafeMode;

    #[test]
    fn an_interactive_svg_is_wrapped_in_an_object() {
        let html = convert_with(
            "= Doc\n\nimage::/i/chart.svg[Chart, opts=interactive, fallback=chart.png]",
            &Options::new(),
        );
        assert!(html.contains(
            "<object type=\"image/svg+xml\" data=\"/i/chart.svg\"><img src=\"chart.png\" alt=\"Chart\"></object>"
        ));
    }

    #[test]
    fn an_interactive_svg_without_a_fallback_renders_the_alt_text() {
        let html = convert_with(
            "= Doc\n\nimage::/i/chart.svg[Chart, opts=interactive]",
            &Options::new(),
        );
        assert!(html.contains(
            "<object type=\"image/svg+xml\" data=\"/i/chart.svg\"><span class=\"alt\">Chart</span></object>"
        ));
    }

    #[test]
    fn an_svg_alt_fallback_escapes_element_text() {
        let html = convert_with(
            "= Doc\n\nimage::/i/chart.svg[A <b> & \"c\", opts=interactive]",
            &Options::new(),
        );
        assert!(html.contains("<span class=\"alt\">A &lt;b&gt; &amp; \"c\"</span>"));
        assert!(!html.contains("<span class=\"alt\">A <b>"));
    }

    #[test]
    fn an_inline_svg_is_read_and_rewritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("img")).expect("mkdir");
        std::fs::write(
            dir.path().join("diagram.svg"),
            "<?xml version=\"1.0\"?>\n<svg width=\"100\" height=\"50\" style=\"background:#fff\">\n<rect/>\n</svg>",
        )
        .expect("write");
        let html = convert_with(
            "= Doc\n\nimage::diagram.svg[Diagram, 40, 20, opts=inline]",
            &Options::new().input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<svg width=\"40\" height=\"20\">"));
        assert!(!html.contains("<?xml"));
    }

    #[test]
    fn inline_svg_decodes_imagesdir_for_file_lookup() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir(dir.path().join("img&dir")).expect("mkdir");
        std::fs::write(dir.path().join("img&dir/diagram.svg"), "<svg><rect/></svg>")
            .expect("write svg");
        let html = convert_with(
            "= Doc\n:imagesdir: img&dir\n\nimage::diagram.svg[Diagram,opts=inline]",
            &Options::new().input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<svg><rect/></svg>"));
        assert!(!html.contains("<span class=\"alt\">"));
    }

    #[test]
    fn an_absolute_inline_svg_target_ignores_imagesdir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("diagram.svg");
        std::fs::write(&target, "<svg><rect/></svg>").expect("write svg");
        let source = format!(
            "= Doc\n:imagesdir: ignored\n\nimage::{}[Diagram,opts=inline]",
            target.display()
        );
        let html = convert_with(&source, &Options::new());
        assert!(html.contains("<svg><rect/></svg>"));
    }

    #[test]
    fn inline_svg_removes_multiline_dimension_attributes() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("diagram.svg"),
            "<svg\nwidth=\"100\"\theight='50'\nstyle=\"fill:red\"><rect/></svg>",
        )
        .expect("write svg");
        let html = convert_with(
            "= Doc\n\nimage::diagram.svg[Diagram,40,20,opts=inline]",
            &Options::new().input_file(dir.path().join("doc.adoc")),
        );
        assert!(html.contains("<svg width=\"40\" height=\"20\"><rect/></svg>"));
        assert!(!html.contains("width=\"100\""));
        assert!(!html.contains("height='50'"));
        assert!(!html.contains("style=\"fill:red\""));
    }

    #[test]
    fn secure_mode_renders_inline_svg_as_a_plain_image() {
        let html = convert_with(
            "= Doc\n\nimage::diagram.svg[Diagram,opts=inline]",
            &Options::new().safe_mode(SafeMode::Secure),
        );
        assert!(html.contains("<img src=\"diagram.svg\" alt=\"Diagram\">"));
        assert!(!html.contains("<span class=\"alt\">"));
    }

    #[test]
    fn an_inline_svg_that_cannot_be_read_renders_the_alt_text() {
        let html = convert_with(
            "= Doc\n\nimage::/i/missing.svg[Alt, opts=inline]",
            &Options::new(),
        );
        assert!(html.contains("<span class=\"alt\">Alt</span>"));
    }

    #[test]
    fn a_plain_svg_renders_as_an_image() {
        let html = convert_with("= Doc\n\nimage::/i/chart.svg[Chart]", &Options::new());
        assert!(html.contains("<img src=\"/i/chart.svg\" alt=\"Chart\">"));
    }
}
