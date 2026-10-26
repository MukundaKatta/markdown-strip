# markdown-strip

[![crates.io](https://img.shields.io/crates/v/markdown-strip.svg)](https://crates.io/crates/markdown-strip)

Strip Markdown formatting to plain text. Conservative, fast, zero deps.
Use it to feed LLM output into TTS, keyword matching, or analytics.

```rust
use markdown_strip::strip_markdown;
let md = "## Hello\n\n**bold** and *italic* with `code` and [a link](https://x).";
assert_eq!(strip_markdown(md), "Hello\n\nbold and italic with code and a link.");
```

Handles ATX headers, bold/italic, inline code, fenced code blocks
(body preserved), links (text kept, URL dropped), images (alt kept),
blockquote markers, and bullet/number list markers.

## CLI (`md2txt`)

A streaming CLI ships as the `md2txt` binary (uses `clap` behind the default
`cli` feature — the library itself stays zero-dep with `default-features = false`):

```sh
cargo run --bin md2txt -- --input README.md --output README.txt
cat README.md | cargo run --bin md2txt            # stdin → stdout
cargo run --bin md2txt -i README.md               # file → stdout
cargo install markdown-strip                      # installs md2txt
```

following parameters are accepted:

* `--input`/`-i`:
  takes a Markdown file path or reads stdin when omitted

* `--output`/`-o`:
  writes to a text file or stdout when omitted. To force stdout use `-`.

Input is processed line-by-line and output streamed. The binary never buffers
the whole file.

MIT or Apache-2.0.
