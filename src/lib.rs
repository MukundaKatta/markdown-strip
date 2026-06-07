//! # markdown-strip
//!
//! Reduce Markdown text to plain text. Conservative — keeps semantic
//! content intact, drops only formatting markers. Intended for piping
//! LLM output into TTS, keyword matching, or analytics.
//!
//! Handles: ATX headers (`# `, `## ` …), bold/italic (`**x**`, `*x*`,
//! `__x__`, `_x_`), inline code (backtick spans), fenced code blocks,
//! links (`[text](url)` → `text`), images (`![alt](url)` → `alt`),
//! blockquote markers (`> `), and bullet/number list markers.
//!
//! ## Example
//!
//! ```
//! use markdown_strip::strip_markdown;
//! let md = "## Hello\n\n**bold** and *italic* with `code` and [a link](https://x).";
//! let plain = strip_markdown(md);
//! assert_eq!(plain, "Hello\n\nbold and italic with code and a link.");
//! ```
//!
//! Non-ASCII text (accents, em-dashes, CJK, emoji) is preserved verbatim:
//!
//! ```
//! use markdown_strip::strip_markdown;
//! assert_eq!(strip_markdown("# Über café — 日本語 🎉"), "Über café — 日本語 🎉");
//! ```

#![deny(missing_docs)]

/// Strip Markdown formatting from `s`, returning a new plain-text `String`.
///
/// The input is borrowed and never mutated. Line breaks are preserved; a
/// trailing newline is kept only if the input had one. Non-ASCII content is
/// preserved byte-for-byte.
///
/// See the [crate-level docs](crate) for the full list of constructs handled.
///
/// # Examples
///
/// ```
/// use markdown_strip::strip_markdown;
/// assert_eq!(strip_markdown("**bold** [link](http://u)"), "bold link");
/// ```
pub fn strip_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_fence = false;

    for line in s.lines() {
        // Fenced code blocks: skip the fences themselves; keep the body
        // as-is (LLM tool output is often code we want to preserve).
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        let stripped = strip_line(line);
        out.push_str(&stripped);
        out.push('\n');
    }

    // Trim the final pushed newline if input didn't end with one.
    if !s.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn strip_line(line: &str) -> String {
    let mut s = line.to_string();

    // ATX headers: leading `#{1,6}\s+`
    s = strip_atx_header(&s);

    // Blockquote: leading `>` optionally followed by space.
    s = strip_blockquote(&s);

    // Bullet/number list markers at the start.
    s = strip_list_marker(&s);

    // Inline tokens (links, images, bold, italic, code).
    s = strip_inline(&s);

    s
}

fn strip_atx_header(s: &str) -> String {
    let leading_ws: String = s.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &s[leading_ws.len()..];
    let mut hashes = 0;
    for c in rest.chars().take(6) {
        if c == '#' {
            hashes += 1;
        } else {
            break;
        }
    }
    if hashes > 0 && rest[hashes..].starts_with(' ') {
        format!("{leading_ws}{}", &rest[hashes + 1..])
    } else {
        s.to_string()
    }
}

fn strip_blockquote(s: &str) -> String {
    let leading_ws: String = s.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &s[leading_ws.len()..];
    if let Some(stripped) = rest.strip_prefix("> ") {
        format!("{leading_ws}{stripped}")
    } else if let Some(stripped) = rest.strip_prefix('>') {
        format!("{leading_ws}{stripped}")
    } else {
        s.to_string()
    }
}

fn strip_list_marker(s: &str) -> String {
    let leading_ws: String = s.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &s[leading_ws.len()..];
    // Unordered: -, *, +
    if let Some(stripped) = rest
        .strip_prefix("- ")
        .or(rest.strip_prefix("* "))
        .or(rest.strip_prefix("+ "))
    {
        return format!("{leading_ws}{stripped}");
    }
    // Ordered: N. or N) where N is digits
    let mut digits = 0;
    for c in rest.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits > 0
        && rest.len() > digits + 1
        && (rest.as_bytes()[digits] == b'.' || rest.as_bytes()[digits] == b')')
        && rest.as_bytes()[digits + 1] == b' '
    {
        return format!("{leading_ws}{}", &rest[digits + 2..]);
    }
    s.to_string()
}

fn strip_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Image: ![alt](url) -> alt
        if i + 1 < bytes.len() && bytes[i] == b'!' && bytes[i + 1] == b'[' {
            if let Some((alt, end)) = parse_link(&s[i + 1..]) {
                out.push_str(&alt);
                i += 1 + end;
                continue;
            }
        }
        // Link: [text](url) -> text
        if bytes[i] == b'[' {
            if let Some((text, end)) = parse_link(&s[i..]) {
                out.push_str(&text);
                i += end;
                continue;
            }
        }
        // Inline code `code` -> code
        if bytes[i] == b'`' {
            if let Some(end_rel) = s[i + 1..].find('`') {
                out.push_str(&s[i + 1..i + 1 + end_rel]);
                i += 2 + end_rel;
                continue;
            }
        }
        // Bold **x** or __x__ → x. Italic *x* or _x_ → x. We handle the
        // bold case first so we don't strip half a bold pair as italic.
        if i + 1 < bytes.len() && (bytes[i] == b'*' && bytes[i + 1] == b'*') {
            if let Some(end_rel) = s[i + 2..].find("**") {
                out.push_str(&s[i + 2..i + 2 + end_rel]);
                i += 4 + end_rel;
                continue;
            }
        }
        if i + 1 < bytes.len() && (bytes[i] == b'_' && bytes[i + 1] == b'_') {
            if let Some(end_rel) = s[i + 2..].find("__") {
                out.push_str(&s[i + 2..i + 2 + end_rel]);
                i += 4 + end_rel;
                continue;
            }
        }
        if bytes[i] == b'*' {
            if let Some(end_rel) = s[i + 1..].find('*') {
                out.push_str(&s[i + 1..i + 1 + end_rel]);
                i += 2 + end_rel;
                continue;
            }
        }
        if bytes[i] == b'_' && is_word_boundary(bytes, i) {
            if let Some(end_rel) = s[i + 1..].find('_') {
                out.push_str(&s[i + 1..i + 1 + end_rel]);
                i += 2 + end_rel;
                continue;
            }
        }
        // Default: copy one whole UTF-8 character. All the branches above
        // only fire on ASCII bytes, so `i` always sits on a char boundary
        // here. We must copy the full multi-byte sequence rather than a
        // single byte, otherwise non-ASCII text (accents, em-dashes, CJK,
        // emoji) would be corrupted.
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&s[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Number of bytes in the UTF-8 sequence whose leading byte is `b`.
///
/// `b` is assumed to be the first byte of a valid UTF-8 character (true here
/// because the caller only ever points `i` at char boundaries).
fn utf8_char_len(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn parse_link(s: &str) -> Option<(String, usize)> {
    // s starts with '['; find ']('; then ')'.
    let bytes = s.as_bytes();
    if bytes[0] != b'[' {
        return None;
    }
    let close_text = s[1..].find("](")?;
    let after_url_off = 1 + close_text + 2;
    let close_url = s[after_url_off..].find(')')?;
    let text = s[1..1 + close_text].to_string();
    Some((text, after_url_off + close_url + 1))
}

fn is_word_boundary(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = bytes[i - 1];
    !prev.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(strip_markdown(""), "");
    }

    #[test]
    fn plain_text_is_unchanged() {
        assert_eq!(strip_markdown("just plain text"), "just plain text");
    }

    #[test]
    fn header_levels_one_through_six() {
        for n in 1..=6 {
            let md = format!("{} Title", "#".repeat(n));
            assert_eq!(strip_markdown(&md), "Title", "level {n}");
        }
    }

    #[test]
    fn seven_hashes_is_not_a_header() {
        // Only levels 1-6 are valid ATX headers; the marker is left intact.
        assert_eq!(strip_markdown("####### too deep"), "####### too deep");
    }

    #[test]
    fn hash_without_space_is_not_a_header() {
        assert_eq!(strip_markdown("#hashtag"), "#hashtag");
    }

    #[test]
    fn nested_blockquote_strips_one_level() {
        // strip_markdown removes a single leading "> " marker per line.
        assert_eq!(strip_markdown("> quoted"), "quoted");
        assert_eq!(strip_markdown(">no space"), "no space");
    }

    #[test]
    fn ordered_list_with_paren() {
        assert_eq!(strip_markdown("1) first"), "first");
    }

    #[test]
    fn bare_number_is_not_a_list_marker() {
        // "404." with no following space is ordinary text, not a list item.
        assert_eq!(strip_markdown("404. is a status"), "is a status");
        assert_eq!(strip_markdown("1.5 release"), "1.5 release");
    }

    #[test]
    fn underscore_inside_word_is_kept() {
        assert_eq!(strip_markdown("snake_case_name"), "snake_case_name");
    }

    #[test]
    fn standalone_emphasis_underscore_is_stripped() {
        assert_eq!(strip_markdown("an _emphasised_ word"), "an emphasised word");
    }

    #[test]
    fn unterminated_inline_code_is_left_alone() {
        assert_eq!(strip_markdown("a `loose backtick"), "a `loose backtick");
    }

    #[test]
    fn link_text_is_kept_url_dropped() {
        assert_eq!(strip_markdown("[text](http://u)"), "text");
    }

    #[test]
    fn image_alt_is_kept() {
        assert_eq!(strip_markdown("![alt](http://u)"), "alt");
    }

    #[test]
    fn multiline_trailing_newline_is_preserved() {
        assert_eq!(strip_markdown("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn multiline_without_trailing_newline() {
        assert_eq!(strip_markdown("a\nb"), "a\nb");
    }

    // --- Regression: non-ASCII text must survive verbatim. ---
    // Previously the default branch copied a single byte (`bytes[i] as char`),
    // which corrupted every multi-byte UTF-8 character outside inline tokens.

    #[test]
    fn non_ascii_plain_text_is_preserved() {
        let s = "café — naïve 日本語";
        assert_eq!(strip_markdown(s), s);
    }

    #[test]
    fn non_ascii_in_header_is_preserved() {
        assert_eq!(strip_markdown("# Über"), "Über");
    }

    #[test]
    fn non_ascii_in_list_item_is_preserved() {
        assert_eq!(strip_markdown("- naïve"), "naïve");
    }

    #[test]
    fn emoji_is_preserved() {
        assert_eq!(strip_markdown("**bold** 🎉 done"), "bold 🎉 done");
    }

    #[test]
    fn non_ascii_link_text_is_preserved() {
        assert_eq!(strip_markdown("[café](http://u)"), "café");
    }

    #[test]
    fn utf8_char_len_classifies_leading_bytes() {
        assert_eq!(utf8_char_len(b'a'), 1);
        assert_eq!(utf8_char_len("é".as_bytes()[0]), 2);
        assert_eq!(utf8_char_len("日".as_bytes()[0]), 3);
        assert_eq!(utf8_char_len("🎉".as_bytes()[0]), 4);
    }

    #[test]
    fn parse_link_reports_text_and_length() {
        let (text, len) = parse_link("[hi](u)rest").unwrap();
        assert_eq!(text, "hi");
        assert_eq!(&"[hi](u)rest"[len..], "rest");
    }

    #[test]
    fn parse_link_returns_none_without_closing_paren() {
        assert!(parse_link("[hi](u").is_none());
    }
}
