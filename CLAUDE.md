# SilkPrint — Project Instructions

## What is this?

Rust CLI that converts Markdown into stunning PDFs. Pipeline: Markdown → comrak AST → Typst markup → World trait compile → PDF.

## Architecture

- `src/lib.rs` — Public API: `render()`, `render_to_typst()`
- `src/cli.rs` — Clap derive CLI with styled help
- `src/main.rs` — Entry point, miette error handler, tracing setup
- `src/error.rs` — `SilkprintError` enum (thiserror + miette)
- `src/warnings.rs` — `SilkprintWarning` + `WarningCollector`
- `src/render/` — Pipeline orchestration, markdown parsing, Typst emission
- `src/theme/` — TOML theme parsing, token resolution, WCAG contrast
- `src/fonts/` — Font loading via rust-embed

## Key Dependencies

- comrak 0.50 — Markdown parser (extensions enabled at RUNTIME, not features)
- typst 0.14 + typst-pdf 0.14 — Direct World trait impl (no typst-as-lib)
- thiserror + miette — Error handling (no anyhow)
- serde_yaml_ng — YAML front matter (not serde_yml — RUSTSEC advisory)
- rust-embed — Font bundling with compression

## Build & Test

```bash
cargo check          # Type-check
cargo clippy         # Lint (strict pedantic config)
cargo test           # Run all tests
cargo run -- --help  # CLI help
cargo run -- tests/fixtures/basic.md -o /tmp/test.pdf
```

## Conventions

- Edition 2024, rust-version 1.85
- `unsafe_code = "forbid"`, `unwrap_used = "deny"`
- pedantic clippy lints at warn level
- thiserror for typed errors, miette for rich diagnostics
- All comrak extensions enabled at runtime via `Options.extension.*`
- Colors are two-level resolved within `[colors]` table
- tmTheme XML served as virtual file at `/__silkprint_theme.tmTheme`
- PDF metadata via `#set document()` in Typst source, NOT `PdfOptions`

## Theme System

40 built-in themes across 8 families. TOML format with 3-layer token hierarchy.
Default theme: `silk-light`. Theme files in `themes/` directory.
