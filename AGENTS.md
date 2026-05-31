# SilkPrint — Agent Instructions

## What is this?

Rust CLI that renders Markdown two ways from one themed engine: a **terminal
reader** (the default for a bare `silkprint <file>`, also `silkprint read`) and
**PDFs** (`silkprint pdf`, or any PDF flag like `-o`). Both pipelines share the
comrak parse + theme resolution:
- PDF: Markdown -> comrak AST -> Typst markup -> World trait compile -> PDF.
- Terminal: comrak AST -> width-independent `RenderedDoc` (semantic role model)
  -> ANSI (one-shot) or a ratatui TUI.

CLI routing is terminal-first: bare `silkprint file.md` reads (TUI in a TTY,
one-shot ANSI when piped); `-o`/`--check`/`--dump-typst`/`--open` or the `pdf`
subcommand force PDF. See `cli.rs::pdf_signaled` / `effective_input`.

## Architecture

- `src/lib.rs` — Public API: `render()`, `render_to_typst()`, `render_to_terminal()`, `run_terminal_tui()`
- `src/cli.rs` — Clap derive CLI, styled help, `pdf`/`read` subcommands, terminal-first routing
- `src/main.rs` — Entry point, mode dispatch, miette error handler, tracing setup
- `src/error.rs` — `SilkprintError` enum (thiserror + miette)
- `src/warnings.rs` — `SilkprintWarning` + `WarningCollector`
- `src/render/` — Pipeline orchestration, markdown parsing, Typst emission
- `src/render/terminal/` — Terminal reader, gated behind the `terminal` feature.
  `walk.rs` (AST -> `RenderedDoc`), `model.rs` (role-based block model), `ansi.rs`
  (one-shot renderer), `caps.rs` (color/glyph/graphics detection), `tui/` (ratatui
  reader: outline, search, theme picker, images, mermaid, in-doc link navigation)
- `src/theme/` — TOML theme parsing, token resolution, WCAG contrast
- `src/fonts/` — Font loading via rust-embed

## Key Dependencies

- comrak 0.50 — Markdown parser (extensions enabled at RUNTIME, not cargo features)
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
cargo run -- tests/fixtures/basic.md -o /tmp/test.pdf  # render a PDF
cargo run -- tests/fixtures/basic.md                   # read in the terminal
```

The terminal reader is behind the default `terminal` feature;
`--no-default-features` builds a PDF-only library (keeps `silkprint-wasm` clean).

## Conventions

- Edition 2024, rust-version 1.96
- `unsafe_code = "forbid"`, `unwrap_used = "deny"`
- Pedantic clippy lints at warn level
- thiserror for typed errors, miette for rich diagnostics
- All comrak extensions enabled at runtime via `Options.extension.*`
- Colors are two-level resolved within `[colors]` table
- tmTheme XML served as virtual file at `/__silkprint_theme.tmTheme`
- PDF metadata via `#set document()` in Typst source, NOT `PdfOptions`

## Theme System

40 built-in themes across 8 families. TOML format with 3-layer token hierarchy:
- Layer 1: Primitives (`[colors]` table — hex values, referenced by name)
- Layer 2: Semantic sections (`[text]`, `[headings]`, etc. — reference color names)
- Layer 3: Component sections (`[code_block]`, `[table]`, etc.)

Default theme: `silk-light`. Theme files in `themes/<family>/` directories.

### Theme Schema (24 sections)

Every theme TOML must define ALL of these sections:

```
[meta]          — name, version, variant (light/dark), description, print_safe, extends
[colors]        — Primitive hex colors, referenced by name in semantic sections
[fonts]         — heading, body, mono fonts + weights + fallbacks
[font_sizes]    — body, small, code, h1-h6
[page]          — background, margins, paper size, columns
[text]          — color, line_height, paragraph_gap, justification, spacing_mode
[headings]      — color, font, line_height, letter_spacing + per-level [headings.h1]-[headings.h6]
[code_block]    — background, border, padding, line_height, language_label, wrap
[code_inline]   — background, border_color, border_radius
[blockquote]    — border, background, text_color, italic
[table]         — header/row styling, stripe_background, cell_padding
[horizontal_rule] — color, width, thickness, style
[links]         — color, underline
[images]        — max_width, alignment, caption styling
[list]          — bullet_color, indent, task checkbox colors
[footnotes]     — separator, text_size, number/backref colors
[alerts]        — note/tip/important/warning/caution colors, border_width, icons
[toc]           — title, entry_color, leader_style, indent, max_depth
[page_numbers]  — enabled, position, format, font, size, color
[title_page]    — enabled, title/subtitle/author/date/separator styling
[emphasis]      — strikethrough_color
[math]          — color
[highlight]     — fill, fill_opacity, text_color, border_radius
[description_list] — term_font, term_weight, term_color, indent, spacing
[syntax]        — background + 16 token types (text, keyword, string, number, function,
                  type, comment, constant, boolean, operator, property, tag, attribute,
                  variable, builtin, punctuation, escape) each with color, bold, italic
```

### Theme Registration

Themes are embedded at compile time in `src/theme/builtin.rs`:
- `const THEME_TOML: &str = include_str!("../../themes/family/name.toml");`
- `get_builtin_theme(name)` match arm returns the TOML
- `list_themes()` has metadata for `--list-themes` display

### Bundled Fonts (only these available)

- **Inter** — Sans-serif, used for headings
- **Source Serif 4** — Serif, used for body text
- **JetBrains Mono** — Monospace, used for code

System fallbacks: Helvetica Neue, Arial, Georgia, Times New Roman, Fira Code, SF Mono, Cascadia Code

### Color Reference Resolution

Colors in semantic sections can reference `[colors]` keys by name:
- `color = "text_primary"` resolves to `[colors].text_primary`
- `color = "#ff0000"` is used as a literal hex value
- Two-level resolution: a color value can reference another color key

### Print Safety Rules

- Light themes with `print_safe = true` must have:
  - Background luminance > 0.85
  - Text contrast ratio > 7:1 against background
  - Reasonable ink coverage
- Dark themes are never print-safe

## File Layout

```
themes/
  _base-syntax-light.toml    # Fallback syntax colors for light themes
  _base-syntax-dark.toml     # Fallback syntax colors for dark themes
  signature/                  # silk-light, silk-dark, manuscript, monochrome
  silkcircuit/                # neon, vibrant, soft, glow, dawn
  greyscale/                  # warm, cool, high-contrast
  classic/                    # academic, typewriter, newspaper, parchment
  futuristic/                 # cyberpunk, terminal, hologram, synthwave, matrix
  nature/                     # forest, ocean, sunset, arctic, sakura
  artistic/                   # noir, candy, blueprint, witch
  developer/                  # nord, dracula, solarized-*, catppuccin-*, gruvbox-*, tokyo-night, rose-pine
```
