# markdown-strip

[![crates.io](https://img.shields.io/crates/v/markdown-strip.svg)](https://crates.io/crates/markdown-strip)
[![docs.rs](https://docs.rs/markdown-strip/badge.svg)](https://docs.rs/markdown-strip)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Strip Markdown formatting to plain text. Conservative, fast, zero dependencies.

Reduce Markdown to readable plain text while keeping the semantic content
intact and dropping only the formatting markers. It is handy for feeding LLM
output into text-to-speech, keyword matching, search indexing, or analytics —
anywhere the Markdown syntax is noise.

## Install

Add it to your `Cargo.toml`:

```toml
[dependencies]
markdown-strip = "0.1"
```

Or with cargo:

```sh
cargo add markdown-strip
```

## Usage

```rust
use markdown_strip::strip_markdown;

let md = "## Hello\n\n**bold** and *italic* with `code` and [a link](https://x).";
assert_eq!(strip_markdown(md), "Hello\n\nbold and italic with code and a link.");
```

It handles non-ASCII text (accents, em-dashes, CJK, emoji) without corruption:

```rust
use markdown_strip::strip_markdown;

let md = "# Über café\n\nnaïve — 日本語 with 🎉";
assert_eq!(strip_markdown(md), "Über café\n\nnaïve — 日本語 with 🎉");
```

## API

### `strip_markdown(s: &str) -> String`

Returns a new `String` with Markdown formatting removed from `s`. The input is
borrowed and never mutated. The output preserves line breaks; a trailing
newline is preserved only if the input had one.

## What it strips

| Markdown                       | Result            |
| ------------------------------ | ----------------- |
| `# H1` … `###### H6` (ATX)     | header text only  |
| `**bold**`, `__bold__`         | inner text        |
| `*italic*`, `_italic_`         | inner text        |
| `` `inline code` ``            | inner text        |
| ```` ``` ```` fenced blocks    | fence removed, body kept verbatim |
| `[text](url)`                  | `text`            |
| `![alt](url)`                  | `alt`             |
| `> quote`                      | `quote`           |
| `- item`, `* item`, `+ item`   | `item`            |
| `1. item`, `1) item`           | `item`            |

Fenced code-block bodies are kept as-is on purpose: LLM tool output is often
code you want to preserve verbatim.

## Behavior notes and limitations

This is a deliberately conservative, line-oriented stripper, not a full
CommonMark parser. In particular:

- Only ATX headers (`#`–`######` followed by a space) are recognized; Setext
  (underline) headers are left untouched.
- `snake_case_name` is preserved — underscores inside a word are not treated
  as emphasis.
- A digit run is only treated as an ordered-list marker when followed by `.`
  or `)` and a space, so `404. Not found` and `1.5 release` keep their text.
- One blockquote (`>`) marker is removed per line.
- Unterminated tokens (e.g. a lone `` ` `` or `[`) are left as-is rather than
  guessed at.

## Development

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
