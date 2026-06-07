use markdown_strip::strip_markdown;

#[test]
fn strips_headers() {
    assert_eq!(strip_markdown("# Title"), "Title");
    assert_eq!(strip_markdown("## Subhead"), "Subhead");
}

#[test]
fn strips_bold_italic() {
    assert_eq!(strip_markdown("**bold** and *italic*"), "bold and italic");
    assert_eq!(strip_markdown("__bold__ and _italic_"), "bold and italic");
}

#[test]
fn strips_inline_code() {
    assert_eq!(
        strip_markdown("use `cargo build` to build"),
        "use cargo build to build"
    );
}

#[test]
fn strips_links_keeping_text() {
    assert_eq!(
        strip_markdown("see [docs](https://example.com)"),
        "see docs"
    );
}

#[test]
fn strips_images_keeping_alt() {
    assert_eq!(
        strip_markdown("![logo](https://example.com/x.png) heading"),
        "logo heading"
    );
}

#[test]
fn strips_list_markers() {
    let md = "- one\n- two\n- three";
    let plain = strip_markdown(md);
    assert_eq!(plain, "one\ntwo\nthree");
}

#[test]
fn strips_ordered_list() {
    let md = "1. one\n2. two\n3. three";
    let plain = strip_markdown(md);
    assert_eq!(plain, "one\ntwo\nthree");
}

#[test]
fn strips_blockquote() {
    assert_eq!(strip_markdown("> quoted line"), "quoted line");
}

#[test]
fn keeps_fenced_code_body() {
    let md = "before\n```\nlet x = 1;\n```\nafter";
    let plain = strip_markdown(md);
    assert!(plain.contains("let x = 1;"));
    assert!(!plain.contains("```"));
}

#[test]
fn does_not_strip_underscores_in_words() {
    // `snake_case_name` shouldn't be touched.
    assert_eq!(
        strip_markdown("the variable snake_case_name is set"),
        "the variable snake_case_name is set"
    );
}

#[test]
fn complex_paragraph() {
    let md = "## Hello\n\n**bold** and *italic* with `code` and [a link](https://x).";
    assert_eq!(
        strip_markdown(md),
        "Hello\n\nbold and italic with code and a link."
    );
}

#[test]
fn preserves_non_ascii_text() {
    // Regression test: plain (non-token) UTF-8 must not be corrupted.
    let md = "## Über café\n\nnaïve — 日本語 with 🎉";
    assert_eq!(strip_markdown(md), "Über café\n\nnaïve — 日本語 with 🎉");
}

#[test]
fn preserves_trailing_newline() {
    assert_eq!(strip_markdown("# Title\n"), "Title\n");
}

#[test]
fn empty_string_round_trips() {
    assert_eq!(strip_markdown(""), "");
}
