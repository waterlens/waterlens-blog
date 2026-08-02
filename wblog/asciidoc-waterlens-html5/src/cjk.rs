//! Collapsing of source line breaks between CJK characters.
//!
//! AsciiDoc treats a single newline inside a paragraph as a space, which is
//! right for space-separated scripts and wrong for CJK: a line break the
//! author inserted purely to keep the source readable would render as a
//! visible gap between two ideographs. This module reproduces the
//! `collapse_cjk_newlines` pass of the `w-html` Asciidoctor converter, which
//! deletes such a break when the text on both sides is CJK.
//!
//! The Ruby original is six sequential `gsub` passes over the already
//! substituted inline HTML. Each pass deletes a whitespace run containing at
//! least one newline when the text immediately before and after it matches
//! that pass's pair of contexts — bare CJK, CJK behind a closing tag, or CJK
//! ahead of an opening tag. Because every pass performs the same edit (delete
//! the run) they are reproduced here as one scan per pass over the run list,
//! which is equivalent to `gsub`'s single non-overlapping left-to-right sweep.

/// Whether `ch` falls in one of the CJK ranges the converter's `CJK_RE`
/// covers: CJK radicals through the unified ideographs, the compatibility
/// ideographs, the CJK compatibility forms, and the fullwidth forms
/// (fullwidth punctuation in particular).
fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{2e80}'..='\u{9fff}'
        | '\u{f900}'..='\u{faff}'
        | '\u{fe30}'..='\u{fe4f}'
        | '\u{ff01}'..='\u{ff60}')
}

/// Whether `ch` is whitespace for the purposes of these patterns.
///
/// Ruby's `\s` is ASCII-only, which matters here: the ideographic space
/// U+3000 lies inside the first CJK range above, so it must count as a CJK
/// character rather than as whitespace.
fn is_ascii_ws(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n' | '\u{b}' | '\u{c}')
}

/// The text that must precede a whitespace run for a pass to delete it.
#[derive(Clone, Copy)]
enum Left {
    /// A bare CJK character.
    Cjk,

    /// A CJK character followed by a closing tag (`中</em>`).
    CjkClosingTag,

    /// A closing tag, whatever it wraps (`</em>`).
    ClosingTag,
}

/// The text that must follow a whitespace run for a pass to delete it.
#[derive(Clone, Copy)]
enum Right {
    /// A bare CJK character.
    Cjk,

    /// An opening tag introducing a CJK character (`<em>中`).
    OpenTagCjk,

    /// An opening tag, whatever it introduces (`<em>`).
    OpenTag,
}

/// The six passes, in the order the Ruby converter applies them.
const PASSES: [(Left, Right); 6] = [
    (Left::Cjk, Right::Cjk),
    (Left::CjkClosingTag, Right::Cjk),
    (Left::Cjk, Right::OpenTagCjk),
    (Left::CjkClosingTag, Right::OpenTagCjk),
    (Left::Cjk, Right::OpenTag),
    (Left::ClosingTag, Right::Cjk),
];

/// Deletes the source line breaks that fall between CJK text in `text`,
/// returning the collapsed string.
///
/// `text` is inline HTML the parser has already substituted, so the scan must
/// reason about tags as well as characters — hence the tag-aware contexts.
pub(crate) fn collapse_cjk_newlines(text: &str) -> String {
    // The overwhelmingly common case in a non-CJK document is that nothing
    // matches; skipping the passes entirely keeps those documents free of the
    // per-pass allocation.
    if !text.contains('\n') || !text.chars().any(is_cjk) {
        return text.to_string();
    }

    let mut text = text.to_string();
    for (left, right) in PASSES {
        text = collapse_pass(&text, left, right);
    }
    text
}

/// Runs one pass: deletes every whitespace run that contains a newline and
/// sits between the pass's two contexts.
fn collapse_pass(text: &str, left: Left, right: Right) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;

    while index < text.len() {
        let Some(ch) = text[index..].chars().next() else {
            break;
        };

        if !is_ascii_ws(ch) {
            index += ch.len_utf8();
            continue;
        }

        // Expand to the maximal whitespace run starting here. `index` is
        // always at the start of a run, because the branch above advances past
        // every non-whitespace character.
        let start = index;
        let mut end = index;
        let mut has_newline = false;
        for ch in text[start..].chars() {
            if !is_ascii_ws(ch) {
                break;
            }
            has_newline |= ch == '\n';
            end += ch.len_utf8();
        }
        index = end;

        if has_newline && matches_left(&text[..start], left) && matches_right(&text[end..], right) {
            out.push_str(&text[copied..start]);
            copied = end;
        }
    }

    out.push_str(&text[copied..]);
    out
}

/// Whether the text before a whitespace run satisfies `left`.
fn matches_left(before: &str, left: Left) -> bool {
    match left {
        Left::Cjk => before.chars().next_back().is_some_and(is_cjk),
        Left::ClosingTag => closing_tag_start(before).is_some(),
        Left::CjkClosingTag => closing_tag_start(before)
            .is_some_and(|start| before[..start].chars().next_back().is_some_and(is_cjk)),
    }
}

/// Whether the text after a whitespace run satisfies `right`.
fn matches_right(after: &str, right: Right) -> bool {
    match right {
        Right::Cjk => after.chars().next().is_some_and(is_cjk),
        Right::OpenTag => open_tag_end(after).is_some(),
        Right::OpenTagCjk => {
            open_tag_end(after).is_some_and(|end| after[end..].chars().next().is_some_and(is_cjk))
        }
    }
}

/// The byte offset at which a trailing `</name>` closing tag begins, or `None`
/// when `before` does not end in one. The tag name is ASCII letters only,
/// matching the converter's case-insensitive `</[a-z]+>`.
fn closing_tag_start(before: &str) -> Option<usize> {
    let without_gt = before.strip_suffix('>')?;

    // The byte offset of the first character in the trailing run of ASCII
    // letters; `None` when the `>` is not preceded by at least one letter.
    let name_start = without_gt
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_alphabetic())
        .last()
        .map(|(index, _)| index)?;

    // Those letters must be introduced by `</` for this to be a closing tag.
    let before_name = &without_gt[..name_start];
    before_name
        .strip_suffix("</")
        .map(|before_tag| before_tag.len())
}

/// The byte offset just past a leading `<name…>` opening tag, or `None` when
/// `after` does not start with one. Matches the converter's `<[a-z][^>]*>`.
fn open_tag_end(after: &str) -> Option<usize> {
    let rest = after.strip_prefix('<')?;
    if !rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let close = rest.find('>')?;
    Some(1 + close + 1)
}

#[cfg(test)]
mod tests {
    use super::collapse_cjk_newlines;

    #[test]
    fn collapses_a_break_between_two_ideographs() {
        assert_eq!(collapse_cjk_newlines("中文\n测试"), "中文测试");
    }

    #[test]
    fn collapses_a_break_carrying_surrounding_spaces() {
        assert_eq!(collapse_cjk_newlines("中文  \n  测试"), "中文测试");
    }

    #[test]
    fn keeps_a_break_between_latin_words() {
        assert_eq!(collapse_cjk_newlines("hello\nworld"), "hello\nworld");
    }

    #[test]
    fn keeps_a_break_between_latin_and_cjk() {
        assert_eq!(collapse_cjk_newlines("hello\n中文"), "hello\n中文");
    }

    #[test]
    fn collapses_across_a_closing_tag() {
        assert_eq!(
            collapse_cjk_newlines("<em>中文</em>\n测试"),
            "<em>中文</em>测试"
        );
    }

    #[test]
    fn collapses_across_an_opening_tag() {
        assert_eq!(
            collapse_cjk_newlines("中文\n<em>测试</em>"),
            "中文<em>测试</em>"
        );
    }

    #[test]
    fn collapses_between_a_closing_and_an_opening_tag() {
        assert_eq!(
            collapse_cjk_newlines("<em>中文</em>\n<code>测试</code>"),
            "<em>中文</em><code>测试</code>"
        );
    }

    #[test]
    fn collapses_before_an_opening_tag_wrapping_latin() {
        // Pass five drops the break ahead of any opening tag once the text
        // before it is CJK, even when the tag introduces Latin text.
        assert_eq!(
            collapse_cjk_newlines("中文\n<code>fn</code>"),
            "中文<code>fn</code>"
        );
    }

    #[test]
    fn collapses_after_a_closing_tag_wrapping_latin() {
        assert_eq!(
            collapse_cjk_newlines("<code>fn</code>\n中文"),
            "<code>fn</code>中文"
        );
    }

    #[test]
    fn treats_the_ideographic_space_as_cjk_not_whitespace() {
        // U+3000 lies inside the first CJK range, so it anchors a collapse
        // rather than being swallowed as whitespace.
        assert_eq!(collapse_cjk_newlines("中\u{3000}\n文"), "中\u{3000}文");
    }

    #[test]
    fn leaves_text_without_newlines_untouched() {
        assert_eq!(collapse_cjk_newlines("中文 测试"), "中文 测试");
    }

    #[test]
    fn collapses_several_breaks_in_one_paragraph() {
        assert_eq!(collapse_cjk_newlines("第一\n第二\n第三"), "第一第二第三");
    }
}
