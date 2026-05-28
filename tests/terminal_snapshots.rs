//! Insta snapshot tests for the one-shot terminal renderer.
//!
//! Rendered in plain ASCII with color disabled and a fixed width, so the
//! snapshots capture structure/layout/glyph-tier output deterministically and
//! are robust against theme color tweaks. Run `cargo insta review` to accept
//! changes.

use silkprint::{ColorChoice, GlyphTier, RenderOptions, TerminalRenderOptions, render_to_terminal};

fn render_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture '{path}' should exist: {e}"));
    let options = RenderOptions::default();
    let terminal_options = TerminalRenderOptions {
        color: ColorChoice::Never,
        glyphs: Some(GlyphTier::Ascii),
        images: false,
        width: Some(80),
    };
    let (output, _warnings) = render_to_terminal(&input, None, &options, &terminal_options)
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
