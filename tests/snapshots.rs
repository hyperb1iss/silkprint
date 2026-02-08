//! Insta snapshot tests for Typst source output.
//!
//! Each test renders a fixture through `render_to_typst()` and snapshots
//! the resulting Typst markup. Run `cargo insta review` to accept changes.

use silkprint::{render_to_typst, RenderOptions};

/// Load a fixture from `tests/fixtures/` and render it to Typst source.
fn render_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture '{path}' should exist: {e}"));
    let options = RenderOptions::default();
    let (typst, _warnings) =
        render_to_typst(&input, &options).expect("render_to_typst should succeed");
    typst
}

#[test]
fn test_snapshot_basic() {
    let typst = render_fixture("basic.md");
    insta::assert_snapshot!("basic", typst);
}

#[test]
fn test_snapshot_minimal() {
    let typst = render_fixture("minimal.md");
    insta::assert_snapshot!("minimal", typst);
}

#[test]
fn test_snapshot_html_blocks() {
    let typst = render_fixture("html-blocks.md");
    insta::assert_snapshot!("html-blocks", typst);
}
