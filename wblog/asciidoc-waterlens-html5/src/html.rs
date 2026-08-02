//! Small helpers for assembling HTML output.
//!
//! The parser applies inline substitutions to block *content* and *titles*, so
//! those strings are emitted verbatim. These helpers only cover text this
//! crate itself places into markup: attribute values (ids, class lists) and
//! the occasional literal that has not been through the substitution pipeline.

/// Escapes `value` for inclusion inside a double-quoted HTML attribute.
///
/// Ids and roles come straight from the source and are dropped into `id="…"`
/// and `class="…"`. They are normally simple tokens, but we escape
/// defensively so a stray `"`, `&`, `<`, or `>` cannot break out of the
/// attribute.
pub(crate) fn escape_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Builds the ` id="…"` fragment for a block wrapper, or an empty string when
/// the block has no id. The leading space is included so call sites can splice
/// the result directly into an opening tag.
pub(crate) fn id_attribute(id: Option<&str>) -> String {
    match id {
        Some(id) => format!(" id=\"{}\"", escape_attribute(id)),
        None => String::new(),
    }
}

/// Joins a base class with any author-supplied roles into a single class list,
/// dropping empty entries — the `w-html` converter's `[base, style, role]
/// .compact.join ' '` idiom.
pub(crate) fn class_list(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| escape_attribute(part))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Builds the ` class="…"` fragment from a base class plus any roles, or an
/// empty string when nothing would be emitted.
pub(crate) fn class_attribute(base: &str, roles: &[&str]) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(1 + roles.len());
    if !base.is_empty() {
        parts.push(base);
    }
    parts.extend(roles.iter().copied());

    let classes = class_list(&parts);
    if classes.is_empty() {
        String::new()
    } else {
        format!(" class=\"{classes}\"")
    }
}

/// Strips XML/HTML tags from `value`, collapses runs of spaces, and trims the
/// result — Asciidoctor's `XmlSanitizeRx` pass, used for the sanitized
/// doctitle that becomes the `<title>` element.
pub(crate) fn sanitize(value: &str) -> String {
    if !value.contains('<') {
        return value.to_string();
    }

    let mut stripped = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => stripped.push(ch),
            _ => {}
        }
    }

    // `squeeze(' ')` collapses only runs of literal spaces, then `strip`
    // removes leading and trailing whitespace.
    let mut squeezed = String::with_capacity(stripped.len());
    let mut last_was_space = false;
    for ch in stripped.chars() {
        if ch == ' ' && last_was_space {
            continue;
        }
        last_was_space = ch == ' ';
        squeezed.push(ch);
    }

    squeezed.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{class_attribute, escape_attribute, id_attribute, sanitize};

    #[test]
    fn escape_attribute_escapes_markup_characters() {
        assert_eq!(
            escape_attribute("a & b < c > d \" e"),
            "a &amp; b &lt; c &gt; d &quot; e"
        );
        assert_eq!(escape_attribute("plain"), "plain");
    }

    #[test]
    fn id_attribute_formats_present_and_absent() {
        assert_eq!(id_attribute(Some("goals")), " id=\"goals\"");
        assert_eq!(id_attribute(None), "");
    }

    #[test]
    fn class_attribute_combines_base_and_roles_dropping_empties() {
        assert_eq!(class_attribute("ulist", &[]), " class=\"ulist\"");
        assert_eq!(
            class_attribute("ulist", &["centered"]),
            " class=\"ulist centered\""
        );
        assert_eq!(class_attribute("", &["only"]), " class=\"only\"");
        assert_eq!(class_attribute("", &[]), "");
    }

    #[test]
    fn sanitize_strips_tags_and_squeezes_spaces() {
        assert_eq!(sanitize("<a href=\".\">CESK 抽象机</a>"), "CESK 抽象机");
        assert_eq!(sanitize("plain title"), "plain title");
        assert_eq!(sanitize("  <b>a</b>   <i>b</i>  "), "a b");
    }
}
