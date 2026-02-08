---
title: Footnote Showcase
author: SilkPrint Test Suite
---

# Footnotes

## Simple Footnotes

SilkPrint uses Typst[^1] as its typesetting engine, which provides professional-quality
output with features like kerning, ligatures, and hyphenation[^2].

The rendering pipeline converts Markdown to an AST using comrak[^3], then emits
Typst markup that gets compiled to PDF.

[^1]: Typst is a modern typesetting system, designed as an alternative to LaTeX.

[^2]: Hyphenation rules follow the document's language setting, defaulting to English.

[^3]: comrak is a CommonMark + GFM compatible Markdown parser written in Rust.

## Multiple References to Same Footnote

The theme system[^themes] supports 40 built-in themes. You can also create custom
themes[^themes] using the TOML format described in the specification.

[^themes]: See the SilkPrint specification section 5.2 for the full theme TOML schema.

## Footnotes with Rich Content

The project follows strict Rust conventions[^conventions] and uses a carefully
chosen dependency stack[^deps].

[^conventions]: The project uses:
    - **Edition 2024** with `rust-version = 1.85`
    - `unsafe_code = "forbid"` and `unwrap_used = "deny"`
    - Pedantic clippy lints at warn level

[^deps]: Key dependencies include:
    - `comrak 0.50` for Markdown parsing
    - `typst 0.14` for compilation
    - `thiserror + miette` for error handling (see [miette docs](https://docs.rs/miette))

## Footnote at End of Paragraph

All fonts are bundled using `rust-embed` with compression enabled, which keeps the
binary size reasonable while ensuring fonts are always available regardless of the
user's system configuration.[^fonts]

[^fonts]: SilkPrint bundles Inter, Source Serif 4, and JetBrains Mono.
