//! Integration tests for the `md2txt` binary.
//!
//! The binary is invoked as a real subprocess via `CARGO_BIN_EXE_md2txt`
//! (set by Cargo for bin targets), exercising the actual argv/stdin/stdout/
//! file plumbing. No extra dev-dependencies needed.
//!
//! The central invariant: for every input/output mode, the binary's output
//! must be byte-identical to `strip_markdown` run over the whole input.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use markdown_strip::strip_markdown;

const BIN: &str = env!("CARGO_BIN_EXE_md2txt");

/// A per-test scratch directory under the system temp dir, removed on drop.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("md2txt-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn write(&self, name: &str, contents: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, contents).expect("write fixture file");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Runs the binary with the given args and stdin, returning
/// (exit code, stdout, stderr).
fn run(args: &[&str], stdin: &[u8]) -> (Option<i32>, String, String) {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn md2txt");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for md2txt");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A representative Markdown document touching every stripping rule.
const SAMPLE: &str = r#"# Title

Some **bold** and _italic_ text with `inline code`.

- a bullet
1. a numbered item

> quoted text

[link text](https://example.com) and ![alt](img.png)

```rust
let x = 1; // **not** stripped
```

final line"#;

// --- core modes -----------------------------------------------------------

#[test]
fn stdin_to_stdout_matches_library() {
    let (code, stdout, stderr) = run(&[], SAMPLE.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, strip_markdown(SAMPLE));
}

#[test]
fn file_to_stdout_matches_library() {
    let fx = Fixture::new("file-to-stdout");
    let input = fx.write("in.md", SAMPLE.as_bytes());
    let (code, stdout, stderr) = run(&["-i", input.to_str().unwrap()], b"");
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, strip_markdown(SAMPLE));
}

#[test]
fn stdin_to_file_matches_library() {
    let fx = Fixture::new("stdin-to-file");
    let output = fx.path("out.txt");
    let (code, stdout, stderr) = run(&["-o", output.to_str().unwrap()], SAMPLE.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, "");
    assert_eq!(fs::read_to_string(&output).unwrap(), strip_markdown(SAMPLE));
}

#[test]
fn file_to_file_matches_library() {
    let fx = Fixture::new("file-to-file");
    let input = fx.write("in.md", SAMPLE.as_bytes());
    let output = fx.path("out.txt");
    let (code, stdout, stderr) = run(
        &[
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, "");
    assert_eq!(fs::read_to_string(&output).unwrap(), strip_markdown(SAMPLE));
}

// --- flag parsing ----------------------------------------------------------

#[test]
fn short_and_long_flags_are_equivalent() {
    let fx = Fixture::new("flag-equivalence");
    let input = fx.write("in.md", SAMPLE.as_bytes());
    let input = input.to_str().unwrap();
    let (_, via_short, _) = run(&["-i", input], b"");
    let (_, via_long, _) = run(&["--input", input], b"");
    assert_eq!(via_short, via_long);
    assert_eq!(via_short, strip_markdown(SAMPLE));
}

#[test]
fn equals_syntax_accepted() {
    let fx = Fixture::new("equals-syntax");
    let input = fx.write("in.md", SAMPLE.as_bytes());
    let output = fx.path("out.txt");
    let (code, stdout, stderr) = run(
        &[
            &format!("--input={}", input.to_str().unwrap()),
            &format!("--output={}", output.to_str().unwrap()),
        ],
        b"",
    );
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, "");
    assert_eq!(fs::read_to_string(&output).unwrap(), strip_markdown(SAMPLE));
}

#[test]
fn dash_sentinels_force_streams() {
    let (code, stdout, stderr) = run(&["-i", "-", "-o", "-"], SAMPLE.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, strip_markdown(SAMPLE));
}

#[test]
fn output_file_overwritten() {
    let fx = Fixture::new("overwrite-output");
    let output = fx.write("out.txt", b"stale contents that must be replaced");
    let (code, _, _) = run(&["-o", output.to_str().unwrap()], b"# fresh\n");
    assert_eq!(code, Some(0));
    assert_eq!(fs::read_to_string(&output).unwrap(), "fresh\n");
}

// --- streaming fidelity ----------------------------------------------------

#[test]
fn crlf_input_matches_library() {
    let crlf = SAMPLE.replace('\n', "\r\n");
    let (code, stdout, _) = run(&[], crlf.as_bytes());
    assert_eq!(code, Some(0));
    // `str::lines()` semantics: CR terminators are dropped, LF normalised.
    assert_eq!(stdout, strip_markdown(&crlf));
    assert!(!stdout.contains('\r'));
}

#[test]
fn missing_trailing_newline_stays_missing() {
    let (code, stdout, _) = run(&[], b"no trailing newline");
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "no trailing newline");
}

#[test]
fn empty_input_empty_output() {
    let (code, stdout, stderr) = run(&[], b"");
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
}

#[test]
fn fenced_block_body_passes_through_verbatim() {
    // Markdown-looking content inside a fence must survive untouched,
    // and the fence lines themselves must be dropped.
    let md = "```\n# not a header\n**not bold** [not a link](x)\n```\nafter";
    let (code, stdout, _) = run(&[], md.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "# not a header\n**not bold** [not a link](x)\nafter"
    );
    assert_eq!(stdout, strip_markdown(md));
}

#[test]
fn tilde_fences_supported() {
    let md = "~~~\n**kept**\n~~~\n**stripped**";
    let (code, stdout, _) = run(&[], md.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(stdout, strip_markdown(md));
}

#[test]
fn large_input_matches_library() {
    // Enough lines to cross internal buffer boundaries while streaming.
    let md = "# head\n\n- item\n".repeat(10_000);
    let (code, stdout, _) = run(&[], md.as_bytes());
    assert_eq!(code, Some(0));
    assert_eq!(stdout, strip_markdown(&md));
}

// --- error handling ---------------------------------------------------------

#[test]
fn missing_input_file_errors() {
    let (code, stdout, stderr) = run(&["-i", "/nonexistent/md2txt-input.md"], b"");
    assert_eq!(code, Some(1));
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("error: cannot open input"), "{stderr}");
    assert!(stderr.contains("/nonexistent/md2txt-input.md"), "{stderr}");
}

#[test]
fn unwritable_output_errors() {
    let (code, _, stderr) = run(&["-o", "/nonexistent-dir-md2txt/out.txt"], b"hi\n");
    assert_eq!(code, Some(1));
    assert!(
        stderr.starts_with("error: cannot create output"),
        "{stderr}"
    );
}

// --- help and version -------------------------------------------------------

#[test]
fn version_flag_prints_package_version() {
    let (code, stdout, stderr) = run(&["--version"], b"");
    assert_eq!(code, Some(0));
    assert_eq!(stderr, "");
    assert_eq!(stdout, format!("md2txt {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flags_document_options() {
    for flag in ["-h", "--help"] {
        let (code, stdout, stderr) = run(&[flag], b"");
        assert_eq!(code, Some(0), "{flag} should exit 0");
        assert_eq!(stderr, "");
        assert!(stdout.contains("Usage:"), "{flag}");
        assert!(stdout.contains("--input <PATH>"), "{flag}");
        assert!(stdout.contains("--output <PATH>"), "{flag}");
    }
}

#[test]
fn long_help_uses_readme_wording() {
    let (code, stdout, _) = run(&["--help"], b"");
    assert_eq!(code, Some(0));
    // The per-option long help mirrors README.md wording.
    assert!(
        stdout.contains("Takes a Markdown file path or reads stdin when omitted."),
        "{stdout}"
    );
    assert!(
        stdout.contains("Writes to a text file or stdout when omitted."),
        "{stdout}"
    );
}
