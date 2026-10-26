//! md2txt: strip Markdown from a file or stdin, write to a file or stdout.
//!
//! Usage:
//!     cargo run --bin md2txt [options]   # or: cargo install markdown-strip
//!
//! Options:
//!     --input, -i <path>   Markdown input file. Omit (or use `-`) to read stdin.
//!     --output, -o <path>  Plain-text output file. Omit (or use `-`) to write stdout.
//!
//! Examples:
//!     cargo run --bin md2txt -- --input README.md --output README.txt
//!     cargo run --bin md2txt -- --input README.md            # → stdout
//!     cat README.md | cargo run --bin md2txt                 # stdin → stdout
//!     cat README.md | cargo run --bin md2txt -- -o out.txt   # stdin → file

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};

use clap::Parser;
use markdown_strip::strip_markdown;

#[derive(Parser)]
#[command(
    name = "md2txt",
    version,
    about = "Strip Markdown formatting to plain text, reading from a file or stdin and writing to a file or stdout.",
    long_about = "Strip Markdown formatting to plain text, reading from a file or stdin and writing to a file or stdout.\n\n\
        Handles ATX headers, bold/italic, inline code, fenced code blocks (body preserved), links \
        (text kept, URL dropped), images (alt kept), blockquote markers, and bullet/number list markers.\n\n\
        Input is processed line-by-line and output streamed. The binary never buffers the whole file."
)]
struct Cli {
    /// Markdown input file. Omit (or use '-') to read from stdin.
    ///
    /// Takes a Markdown file path or reads stdin when omitted. To force stdin use '-'.
    #[arg(short = 'i', long, value_name = "PATH")]
    input: Option<String>,

    /// Plain-text output file. Omit (or use '-') to write to stdout.
    ///
    /// Writes to a text file or stdout when omitted. To force stdout use '-'.
    #[arg(short = 'o', long, value_name = "PATH")]
    output: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli.input.as_deref(), cli.output.as_deref()) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(input: Option<&str>, output: Option<&str>) -> Result<(), String> {
    // Input: a file path, or stdin when omitted / '-'.
    let reader: Box<dyn BufRead> = match input {
        None | Some("-") => Box::new(io::stdin().lock()),
        Some(path) => match File::open(path) {
            Ok(f) => Box::new(io::BufReader::new(f)),
            Err(e) => return Err(format!("cannot open input '{path}': {e}")),
        },
    };

    // Output: a file, or stdout when omitted / '-'.
    let writer: Box<dyn Write> = match output {
        None | Some("-") => Box::new(io::stdout().lock()),
        Some(path) => match File::create(path) {
            Ok(f) => Box::new(BufWriter::new(f)),
            Err(e) => return Err(format!("cannot create output '{path}': {e}")),
        },
    };

    stream_strip(reader, writer).map_err(|e| format!("io error: {e}"))
}

/// Streams input line by line so output is produced incrementally, while
/// staying byte-identical to `strip_markdown` on the whole input:
///
/// - Fence lines (``` / ~~~ after trimming leading whitespace) toggle fence
///   state and are dropped, just like the library does.
/// - Inside a fence, lines pass through verbatim.
/// - Line endings are normalised to `\n` (the library splits with
///   `str::lines()`), and a missing final newline stays missing.
fn stream_strip(mut reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut in_fence = false;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        // Split off the terminator; a trailing \r (CRLF) is dropped by the
        // split, matching `str::lines()`.
        let (body, has_nl) = match line.strip_suffix('\n') {
            Some(rest) => (rest.strip_suffix('\r').unwrap_or(rest), true),
            None => (line.as_str(), false),
        };

        let trimmed = body.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }

        let stripped = if in_fence {
            Cow::Borrowed(body)
        } else {
            Cow::Owned(strip_markdown(body))
        };
        writer.write_all(stripped.as_bytes())?;
        if has_nl {
            writer.write_all(b"\n")?;
        }
    }
    writer.flush()
}
