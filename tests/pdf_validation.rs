//! PDF structural validation tests.
//!
//! Renders fixtures to PDF bytes via the public `render()` API, then
//! parses and inspects the output with `lopdf`.

use std::path::Path;

use lopdf::Document;
use silkprint::{render, RenderOptions};

/// Render a fixture file to raw PDF bytes.
fn render_fixture_to_pdf(name: &str) -> Vec<u8> {
    let path = format!("tests/fixtures/{name}");
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture '{path}' should exist: {e}"));
    let options = RenderOptions::default();
    let (pdf_bytes, _warnings) = render(&input, Some(Path::new(&path)), &options)
        .expect("render should produce PDF bytes");
    pdf_bytes
}

// ── Structure ────────────────────────────────────────────────────

#[test]
fn test_pdf_valid_structure() {
    let bytes = render_fixture_to_pdf("basic.md");
    Document::load_mem(&bytes).expect("PDF should parse successfully with lopdf");
}

#[test]
fn test_pdf_has_pages() {
    let bytes = render_fixture_to_pdf("basic.md");
    let doc = Document::load_mem(&bytes).expect("PDF should parse");

    // The catalog's /Pages tree must contain at least one /Page object.
    let pages = doc.get_pages();
    assert!(
        !pages.is_empty(),
        "PDF should contain at least one page, found 0"
    );
}

// ── Metadata ─────────────────────────────────────────────────────

#[test]
fn test_pdf_metadata() {
    let bytes = render_fixture_to_pdf("basic.md");
    let doc = Document::load_mem(&bytes).expect("PDF should parse");

    // The trailer should reference an /Info dictionary.
    // Typst always embeds document metadata when #set document() is used.
    let has_info = doc.trailer.get(b"Info").is_ok();

    // Even if Info isn't in the trailer, check for XMP metadata stream
    // (Typst may use XMP instead of the classic Info dict).
    let has_xmp = doc
        .objects
        .values()
        .any(|obj| format!("{obj:?}").contains("Metadata"));

    assert!(
        has_info || has_xmp,
        "PDF should contain metadata (Info dict or XMP stream)"
    );
}

// ── Fonts ────────────────────────────────────────────────────────

#[test]
fn test_pdf_fonts_embedded() {
    let bytes = render_fixture_to_pdf("basic.md");
    let doc = Document::load_mem(&bytes).expect("PDF should parse");

    // Scan all objects for /Type /Font entries — at least one font
    // must be embedded for the document to render text.
    let font_count = doc
        .objects
        .values()
        .filter(|obj| {
            if let lopdf::Object::Dictionary(dict) = obj {
                dict.get(b"Type")
                    .is_ok_and(|v| matches!(v, lopdf::Object::Name(n) if n == b"Font"))
            } else {
                false
            }
        })
        .count();

    assert!(
        font_count > 0,
        "PDF should have at least one embedded font, found {font_count}"
    );
}

// ── Minimal fixture ──────────────────────────────────────────────

#[test]
fn test_pdf_minimal_renders() {
    let bytes = render_fixture_to_pdf("minimal.md");
    let doc = Document::load_mem(&bytes).expect("minimal PDF should parse");
    assert!(
        !doc.get_pages().is_empty(),
        "minimal PDF should have at least one page"
    );
}
