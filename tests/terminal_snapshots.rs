//! Insta snapshot tests for the one-shot terminal renderer.
//!
//! Rendered in plain ASCII with color disabled and a fixed width, so the
//! snapshots capture structure/layout/glyph-tier output deterministically and
//! are robust against theme color tweaks. Run `cargo insta review` to accept
//! changes.

#![cfg(feature = "terminal")]

use silkprint::{ColorChoice, GlyphTier, RenderOptions, TerminalRenderOptions, render_to_terminal};

fn render_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture '{path}' should exist: {e}"));
    render_markdown(&input, 80)
}

fn render_markdown(input: &str, width: u16) -> String {
    let options = RenderOptions::default();
    let terminal_options = TerminalRenderOptions {
        color: ColorChoice::Never,
        glyphs: Some(GlyphTier::Ascii),
        images: false,
        width: Some(width),
    };
    let (output, _warnings) = render_to_terminal(input, None, &options, &terminal_options)
        .expect("render_to_terminal should succeed");
    output
}

#[test]
fn test_terminal_basic() {
    insta::assert_snapshot!("terminal_basic", render_fixture("basic.md"));
}

#[test]
fn test_terminal_alerts() {
    insta::assert_snapshot!("terminal_alerts", render_fixture("alerts.md"));
}

#[test]
fn test_terminal_code_blocks() {
    insta::assert_snapshot!("terminal_code_blocks", render_fixture("code-blocks.md"));
}

#[test]
fn test_terminal_full_features() {
    insta::assert_snapshot!("terminal_full_features", render_fixture("full-features.md"));
}

#[test]
fn terminal_tables_wrap_cells_instead_of_ellipsizing() {
    let output = render_markdown(
        "| Left | Right |\n\
         | --- | --- |\n\
         | alpha beta gamma | delta epsilon zeta |\n",
        32,
    );

    assert!(
        !output.contains('\u{2026}'),
        "table cells should wrap instead of ellipsizing:\n{output}"
    );
    assert!(output.contains("alpha"));
    assert!(output.contains("beta"));
    assert!(output.contains("gamma"));
    assert!(output.contains("epsilon"));
}
