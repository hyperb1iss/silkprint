# SilkPrint Implementation Specification

> Transform Markdown into magazine-quality PDFs with electric elegance

**Spec Version:** 2.2
**Tool Version:** 0.1.0
**Author:** Stefanie Jane <stef@hyperbliss.tech>
**License:** Apache-2.0
**Repository:** `github.com/hyperb1iss/silkprint`

---

## 1. Project Overview

SilkPrint is a Rust CLI tool that converts Markdown documents into beautifully typeset PDFs. It
supports every Markdown feature under the sun — GFM, math, alerts, emojis, syntax
highlighting, footnotes, task lists, and more — rendered through a professional typesetting engine
with a pluggable theme system ranging from clean print-ready monochrome to the full SilkCircuit
Neon aesthetic.

### 1.1 Core Value Proposition

- **Single static binary** — no runtime dependencies, no "install Python/Node first"
- **Professional typography** — kerning, ligatures, hyphenation, justified paragraphs via Typst
- **Stunning defaults** — beautiful output with zero configuration
- **Complete Markdown support** — every feature, every extension, every edge case
- **Theme system** — 40 built-in themes across 8 aesthetic families + user-defined TOML themes with full font configurability
- **Sub-second rendering** — Typst compiles a 12-page document in ~400ms (warm) / ~1.5s (cold)

### 1.2 Target Users

- Developers converting README/docs to polished PDFs
- Technical writers producing styled documentation
- Anyone who wants beautiful PDFs from Markdown without learning LaTeX or Typst

---

## 2. Architecture

### 2.1 Pipeline

```
                ┌─────────────┐
                │  Input .md  │
                └──────┬──────┘
                       │
                       ▼
            ┌──────────────────┐
            │   Front Matter   │  ← Extract YAML metadata
            │     Parser       │    (title, author, date, theme, lang)
            └────────┬─────────┘
                     │
                     ▼
            ┌──────────────────┐
            │   Markdown AST   │  ← comrak with extensions enabled at runtime
            │     (comrak)     │    (GFM, footnotes, math, alerts,
            └────────┬─────────┘     description lists, emoji, etc.)
                     │
                     ▼
            ┌──────────────────┐
            │  Theme Resolver  │  ← Load built-in or custom theme
            │   (TOML → Typst) │    Merge layers: default → theme → overrides
            └────────┬─────────┘     Validate colors, fonts, WCAG contrast
                     │
                     ▼
            ┌──────────────────┐
            │  Typst Emitter   │  ← AST nodes → Typst markup
            │  (AST → .typ)    │    Apply theme as Typst show/set rules
            └────────┬─────────┘     Emit every element type
                     │
                     ▼
            ┌──────────────────┐
            │  Typst Compiler  │  ← Direct World trait implementation
            │  (compile + PDF) │    Font loading, image resolution
            └────────┬─────────┘     PDF metadata via #set document()
                     │
                     ▼
            ┌──────────────────┐
            │   Output .pdf    │
            └──────────────────┘
```

### 2.2 Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Markdown parser | **comrak 0.50** | Richest extension set (GFM, footnotes, math, description lists, front matter, alerts, emoji shortcodes, underline, superscript, subscript, highlights, wikilinks), full AST |
| Typesetting engine | **Typst 0.14** (embedded) | Professional typography with zero layout code; handles line/page breaks, justification, tables, hyphenation |
| Typst integration | **Direct `World` trait impl** | ~150 lines of boilerplate, but full control over font loading, file resolution, compilation — no third-party wrapper dependency |
| PDF export | **typst-pdf 0.14** | Native Typst PDF backend — high-quality output, font subsetting, bookmarks. Metadata (title, author) set via `#set document()` in Typst source, not `PdfOptions` |
| CLI framework | **clap 4.5** (derive) | Ecosystem standard, custom styled `--help` output |
| Theme format | **TOML** | Rust-native config format, human-readable, excellent serde support |
| Font bundling | **rust-embed** with compression | Ship Inter, Source Serif 4, JetBrains Mono; compressed at build time to reduce binary size |
| Syntax highlighting | **Typst built-in + tmTheme generation** | Typst uses tmTheme (TextMate) files for syntax coloring. SilkPrint generates tmTheme XML at runtime from `[syntax.*]` TOML tables → passed via `#set raw(theme: <bytes>)` |
| Error handling | **thiserror + miette** | thiserror for typed errors, miette for rich diagnostic rendering; no anyhow (miette::Report handles the type-erased case) |
| YAML parsing | **serde_yaml_ng** | Front matter deserialization (maintained fork; serde_yml has RustSec advisory RUSTSEC-2025-0068) |

### 2.3 Non-Goals (v0.1)

- Interactive/live preview mode (future)
- EPUB output (future)
- Watch mode / hot reload (future)
- Multi-file / book mode (future)
- Custom Typst template injection (future — themes only)
- stdin piping (future — v0.2)

---

## 3. Project Structure

```
silkprint/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Lint + test + clippy
│       └── release.yml         # cargo-dist automated releases
├── src/
│   ├── main.rs                 # Entry point, CLI dispatch, miette handler
│   ├── lib.rs                  # Public API (render function)
│   ├── cli.rs                  # Clap argument definitions + styled help
│   ├── render/
│   │   ├── mod.rs              # Pipeline orchestration
│   │   ├── frontmatter.rs      # YAML front matter extraction
│   │   ├── markdown.rs         # comrak AST → Typst content translation
│   │   ├── preamble.rs         # Theme + front matter → Typst set/show rules preamble
│   │   ├── image.rs            # Image path resolution, format validation, placeholders
│   │   ├── emoji.rs            # Emoji shortcode → Unicode resolution
│   │   └── typst.rs            # Typst World trait impl, compile, PDF export
│   ├── theme/
│   │   ├── mod.rs              # Theme loading, resolution, validation
│   │   ├── tokens.rs           # Token hierarchy types (primitives → semantic)
│   │   ├── builtin.rs          # Built-in theme registry (embedded TOML)
│   │   ├── syntax.rs           # Syntax highlighting color mapping
│   │   ├── tmtheme.rs          # [syntax.*] TOML → tmTheme XML generation
│   │   └── contrast.rs         # WCAG contrast ratio validation
│   ├── fonts/
│   │   └── mod.rs              # Font loading, bundled font registry
│   ├── warnings.rs             # Non-fatal warning system
│   └── error.rs                # Error types (thiserror + miette)
├── themes/                     # Built-in theme TOML source files (40 themes + 2 base)
│   ├── _base-syntax-light.toml # Internal base syntax for light themes (not user-selectable)
│   ├── _base-syntax-dark.toml  # Internal base syntax for dark themes (not user-selectable)
│   ├── signature/              # Signature collection
│   │   ├── silk-light.toml
│   │   ├── silk-dark.toml
│   │   ├── manuscript.toml
│   │   └── monochrome.toml
│   ├── silkcircuit/            # SilkCircuit design system variants
│   │   ├── silkcircuit-neon.toml
│   │   ├── silkcircuit-vibrant.toml
│   │   ├── silkcircuit-soft.toml
│   │   ├── silkcircuit-glow.toml
│   │   └── silkcircuit-dawn.toml
│   ├── greyscale/              # Greyscale family
│   │   ├── greyscale-warm.toml
│   │   ├── greyscale-cool.toml
│   │   └── high-contrast.toml
│   ├── classic/                # Classic / Literary
│   │   ├── academic.toml
│   │   ├── typewriter.toml
│   │   ├── newspaper.toml
│   │   └── parchment.toml
│   ├── futuristic/             # Futuristic / Sci-Fi
│   │   ├── cyberpunk.toml
│   │   ├── terminal.toml
│   │   ├── hologram.toml
│   │   ├── synthwave.toml
│   │   └── matrix.toml
│   ├── nature/                 # Nature collection
│   │   ├── forest.toml
│   │   ├── ocean.toml
│   │   ├── sunset.toml
│   │   ├── arctic.toml
│   │   └── sakura.toml
│   ├── artistic/               # Artistic / Bold
│   │   ├── noir.toml
│   │   ├── candy.toml
│   │   ├── blueprint.toml
│   │   └── witch.toml
│   └── devfavs/                # Developer favorites
│       ├── nord.toml
│       ├── dracula.toml
│       ├── solarized-light.toml
│       ├── solarized-dark.toml
│       ├── catppuccin-mocha.toml
│       ├── catppuccin-latte.toml
│       ├── gruvbox-dark.toml
│       ├── gruvbox-light.toml
│       ├── tokyo-night.toml
│       └── rose-pine.toml
├── fonts/                      # Bundled font files (compressed at compile time)
│   ├── inter/
│   │   ├── Inter-Regular.ttf
│   │   ├── Inter-Medium.ttf
│   │   ├── Inter-SemiBold.ttf
│   │   └── Inter-Bold.ttf
│   ├── source-serif/
│   │   ├── SourceSerif4-Regular.ttf
│   │   ├── SourceSerif4-Italic.ttf
│   │   ├── SourceSerif4-SemiBold.ttf
│   │   └── SourceSerif4-Bold.ttf
│   └── jetbrains-mono/
│       ├── JetBrainsMono-Regular.ttf
│       ├── JetBrainsMono-Italic.ttf
│       ├── JetBrainsMono-Bold.ttf
│       └── JetBrainsMono-BoldItalic.ttf
├── tests/
│   ├── integration.rs          # End-to-end pipeline tests
│   ├── fixtures/               # Test markdown files
│   │   ├── basic.md
│   │   ├── full-features.md
│   │   ├── code-blocks.md
│   │   ├── tables.md
│   │   ├── lists.md
│   │   ├── alerts.md
│   │   ├── math.md
│   │   ├── emojis.md
│   │   ├── footnotes.md
│   │   ├── frontmatter.md
│   │   ├── images.md
│   │   ├── wikilinks.md
│   │   └── edge-cases.md
│   └── themes/
│       ├── custom-test.toml
│       └── invalid-test.toml
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md
├── LICENSE
├── README.md
└── SPEC.md                     # This document
```

---

## 4. CLI Interface

### 4.1 Commands & Arguments

```
silkprint [OPTIONS] [INPUT]

Arguments:
  [INPUT]              Path to the Markdown file to render (optional with --list-themes)

Options:
  -o, --output <PATH>  Output path ("-" for stdout) [default: <input-stem>.pdf]
  -t, --theme <NAME>   Theme name or path to .toml [default: silk-light]
  -p, --paper <SIZE>   Paper size: a4, letter, a5, legal (case-insensitive) [default: a4]
      --list-themes    List all available themes and exit
      --check          Validate input + theme without rendering (exit code only)
      --dump-typst     Emit generated Typst markup to stdout instead of PDF
      --open           Open the PDF in system viewer after rendering
      --toc            Force-enable table of contents (overrides front matter)
      --no-toc         Force-disable table of contents
      --no-title-page  Suppress title page even if theme enables it
      --font-dir <DIR> Additional font search directory
      --color <WHEN>   Color output: auto, always, never [default: auto]
  -v, --verbose        Increase verbosity (-v, -vv, -vvv)
  -q, --quiet          Suppress all output except errors
  -V, --version        Print version
  -h, --help           Print help (styled with SilkCircuit colors)
```

### 4.2 Front Matter Support

Documents can override CLI options via YAML front matter:

```yaml
---
title: "My Document"
subtitle: "A deeper dive"
author: "Stefanie Jane"
date: 2026-02-07
lang: en                    # Affects hyphenation, smart quotes
theme: silkcircuit-neon
paper: letter
toc: true
toc-depth: 3               # Max heading depth in TOC
numbering: "1"              # Page number format: "1", "i", "1 / N", none
font-size: 11pt             # Override base font size
---
```

**Precedence:** CLI flags > front matter > theme defaults > built-in defaults

When a CLI flag overrides a front matter value, emit a verbose-mode note:
`Theme 'monochrome' (CLI) overrides 'silkcircuit-neon' (front matter)`

### 4.3 Flag Validation

Conflicting flag combinations are caught early:

| Combination | Behavior |
|---|---|
| `--quiet` + `--verbose` | Error: "cannot combine --quiet and --verbose" |
| `--check` + `--open` | Error: "--open requires rendering (incompatible with --check)" |
| `--dump-typst` + `--open` | Error: "--open requires PDF output" |
| `-o -` + `--open` | Error: "--open incompatible with stdout output" |
| `--dump-typst` + `-o <file>` | Allowed: write Typst source to file instead of stdout |
| `--list-themes` + output flags | Ignore output flags silently (list-themes is a mode) |
| `--toc` + `--no-toc` | Error: "cannot combine --toc and --no-toc" |

### 4.4 CLI Output Behavior

**Default (no flags):** Single summary line on success.

```
silkprint: output.pdf (12 pages, 340ms)
```

**`--verbose`:** Stage-by-stage progress with SilkCircuit-styled output.

```
  💎 silkprint v0.1.0
  ────────────────────────────────────
  ⚡ Parsing markdown             done
  🎨 Applying theme     silkcircuit-neon
  🔮 Rendering pages    ████████████ 12
  💜 Writing PDF         output.pdf
  ────────────────────────────────────
  ✓ 12 pages rendered in 340ms
```

**`--quiet`:** No output. Exit code only (0 = success, 1 = error).

**Warnings** (non-fatal issues) appear in default and verbose modes:

```
  ⚠ image 'diagram.png' not found, skipping
  ⚠ font 'Fira Sans' not available, falling back to 'Inter'
  ⚠ code block language 'brainfuck' not recognized for highlighting
```

**Errors** use `miette` for rich diagnostics:

```
  ✗ Theme error
  ╭─[silkcircuit-neon.toml:12:1]
  │
  12 │ primary = "not-a-color"
  │             ^^^^^^^^^^^ invalid hex color
  │
  ╰─
  help: Colors must be 3, 4, 6, or 8 digit hex values (e.g., #e135ff)
```

---

## 5. Theme System

### 5.1 Token Architecture

Three-layer hierarchy with two-level reference resolution within `[colors]`:

```
Layer 1: Primitives     Raw values — colors (hex), fonts (names), sizes (pt/em/mm)
Layer 2: Semantic        Role-based references to [colors] keys — text, headings, links
Layer 3: Component       Element-specific — blockquote, code_block, table, list, etc.
```

**Reference resolution:** All color fields in Layer 2/3 accept either a `[colors]` key name OR a
direct `#hex` value. Key lookup first, hex parse fallback. Color key names MUST NOT start with `#`.
Hex values MUST start with `#`.

**Resolution order:**
1. Resolve `[colors]` table first — keys within `[colors]` may reference other `[colors]` keys
   (one level of aliasing, e.g., `primary = "accent_blue"` where `accent_blue = "#4a5dbd"`)
2. Then resolve all Layer 2/3 color fields — each looks up the (now fully-resolved) `[colors]` table

This enables semantic aliases within `[colors]` itself:

```toml
[colors]
accent_blue = "#4a5dbd"         # primitive hex value
primary     = "accent_blue"     # alias → resolves to "#4a5dbd" in step 1

[headings]
color = "primary"               # resolves to "#4a5dbd" via the resolved [colors] table
```

### 5.2 Theme TOML Schema

```toml
# ═══════════════════════════════════════════════════════════════
# SilkPrint Theme Specification — Full Schema
# ═══════════════════════════════════════════════════════════════

[meta]
name        = "Silk Light"
version     = "1"                  # Schema version for forward compat
variant     = "light"              # "light" | "dark"
description = "Clean, warm, professional"
print_safe  = true                 # Validates: light bg, dark text, ink-efficient
extends     = ""                   # Optional: inherit from another theme

# ─── Layer 1: Primitives ──────────────────────────────────────

[colors]
white          = "#ffffff"
cream          = "#fafaf8"
surface        = "#f4f4f8"
surface_alt    = "#eaeaf0"
border_light   = "#e2e2e8"
border_strong  = "#c8c8d4"
text_primary   = "#1a1a2e"         # Softened black — never pure #000
text_secondary = "#555570"
text_muted     = "#8888a0"
accent_blue    = "#4a5dbd"
accent_green   = "#2d8659"
accent_amber   = "#a07c30"
accent_red     = "#c44d56"

[fonts]
heading          = "Inter"
heading_weight   = 600
heading_italic   = false
body             = "Source Serif 4"
body_weight      = 400
body_italic      = false
mono             = "JetBrains Mono"
mono_weight      = 400
mono_ligatures   = false           # Disabled in code for clarity

# Fallback chains (tried in order if primary unavailable)
heading_fallback = ["Inter", "Helvetica Neue", "Arial"]
body_fallback    = ["Source Serif 4", "Georgia", "Times New Roman"]
mono_fallback    = ["JetBrains Mono", "Fira Code", "SF Mono", "Cascadia Code"]

# Optional: bundle font files with theme (path relative to theme file)
# heading_source = "fonts/MyFont-SemiBold.ttf"
# body_source    = "fonts/MyFont-Regular.ttf"

[font_sizes]
body   = "11pt"
small  = "9pt"
code   = "10pt"
h1     = "33.5pt"
h2     = "27pt"
h3     = "21.5pt"
h4     = "17pt"
h5     = "14pt"
h6     = "11pt"

# ─── Layer 2: Semantic ────────────────────────────────────────

[page]
background     = "white"
margin_top     = "25mm"
margin_bottom  = "30mm"
margin_left    = "25mm"            # Symmetric for single-sided (default)
margin_right   = "25mm"
paper          = "a4"
columns        = 1                 # 1 or 2 (newspaper theme uses 2)
column_gap     = "12mm"            # Gap between columns when columns > 1

[text]
color           = "text_primary"
line_height     = 1.5               # Optimal for print/PDF at 11pt
paragraph_gap   = "0.85em"
justification   = "justify"         # "justify" → par(justify: true), "left"|"ragged-right" → par(justify: false)
spacing_mode    = "gap"             # "gap" | "indent" | "both" — see Typst mapping below
first_line_indent = "0pt"           # Used when spacing_mode is "indent" or "both"
# spacing_mode Typst mapping:
#   "gap"    → par.spacing = paragraph_gap, par.first-line-indent = 0pt
#   "indent" → par.spacing = 0pt, par.first-line-indent = first_line_indent
#              First paragraph after heading/blockquote/list: NO indent (use show rule)
#   "both"   → par.spacing = paragraph_gap, par.first-line-indent = first_line_indent
orphan_lines    = 2                 # Min lines at bottom of page before break
widow_lines     = 2                 # Min lines at top of page after break

[headings]
color          = "text_primary"
font           = "heading"
line_height    = 1.2
letter_spacing = "-0.015em"

# Per-level overrides (only specified fields override defaults above)
[headings.h1]
weight           = 700
line_height      = 1.1               # Per-level override (default: [headings].line_height)
border           = true              # Accent line below H1
above            = "36pt"            # Absolute units — NOT relative to heading size
below            = "12pt"
page_break_before = false            # Set true to force new page before H1

[headings.h2]
weight           = 600
line_height      = 1.15
border           = true
above            = "28pt"
below            = "8pt"
page_break_before = false

[headings.h3]
weight         = 600
line_height    = 1.2
above          = "22pt"
below          = "6pt"

[headings.h4]
weight         = 500               # Medium weight for lower headings
line_height    = 1.2
above          = "18pt"
below          = "4pt"

[headings.h5]
weight         = 500
line_height    = 1.25
above          = "18pt"
below          = "4pt"

[headings.h6]
weight         = 600
line_height    = 1.25
above          = "18pt"
below          = "4pt"
uppercase      = true              # Differentiate from body text
letter_spacing = "0.05em"

[code_block]
background        = "surface"
border_color      = "border_light"
border_radius     = "6pt"
padding_vertical  = "12pt"
padding_horizontal = "14pt"
line_height       = 1.45
left_accent       = false          # Colored left bar
left_accent_color = "accent_blue"
line_numbers      = false
language_label    = true           # Show language name in top-right
language_label_color = "text_muted"
language_label_size  = "8pt"
wrap              = true           # Soft-wrap long lines (false = clip)

[code_inline]
background     = "surface"
border_color   = "border_light"
border_radius  = "3pt"

[blockquote]
border_color      = "accent_blue"
border_width      = "2.5pt"
background        = "accent_blue"
background_opacity = 0.0           # No fill by default — border is sufficient
text_color        = "text_secondary"
italic            = true
left_padding      = "14pt"         # Space between border and text

[table]
header_background   = "surface"
header_border_color = "border_strong"
header_border_width = "1.5pt"
header_font         = "heading"
header_weight       = 600
row_border_color    = "border_light"
row_border_width    = "0.5pt"
stripe_background   = "cream"
vertical_lines      = false        # Tufte-style: horizontal only
cell_padding        = "6pt 10pt"   # vertical horizontal

[horizontal_rule]
color     = "border_light"
width     = "60%"
thickness = "0.5pt"
style     = "line"                 # "line" | "dots" | "diamonds"

[links]
color     = "accent_blue"
underline = true

[images]
max_width        = "100%"          # Of text block width
alignment        = "center"
border           = false
border_radius    = "4pt"
caption_font     = "body"
caption_size     = "small"
caption_color    = "text_muted"
caption_italic   = true
caption_position = "below"

[list]
bullet_color       = "text_secondary"
indent             = "20pt"
nested_indent      = "20pt"
task_checked_color = "accent_green"
task_unchecked_color = "text_muted"

# Bullet progression by nesting level
# Level 1: filled circle, Level 2: en-dash, Level 3: small circle
# Ordered: 1. then a. then i.

[footnotes]
separator_color   = "border_light"
separator_width   = "33%"
text_size         = "small"
number_color      = "accent_blue"
backref_color     = "accent_blue"

[alerts]
# GitHub-style alerts: NOTE, TIP, IMPORTANT, WARNING, CAUTION
# comrak extension field: `alerts` (not "admonitions")
note_color          = "accent_blue"
tip_color           = "accent_green"
important_color     = "accent_blue"
warning_color       = "accent_amber"
caution_color       = "accent_red"
border_width        = "3pt"
background_opacity  = 0.08
show_icon           = true         # Unicode icon per type
show_label          = true         # "Note:", "Tip:", "Important:", "Warning:", "Caution:"

[toc]
title             = "Contents"
title_size        = "h2"
entry_color       = "text_primary"
page_number_color = "text_muted"
leader_style      = "dots"        # "dots" | "line" | "none"
indent            = "1.5em"
max_depth         = 3

[page_numbers]
enabled    = true
position   = "bottom-center"      # "bottom-center" | "bottom-outside"
format     = "1"                   # "1" | "i" | "1 / N"
font       = "body"
size       = "small"
color      = "text_muted"
first_page = false                 # Suppress on title/first page

[title_page]
enabled          = true
title_font       = "heading"
title_size       = "42pt"
title_color      = "text_primary"
subtitle_color   = "text_secondary"
author_color     = "text_secondary"
date_color       = "text_muted"
separator_color  = "accent_blue"

[emphasis]
strikethrough_color = "text_muted"

[math]
color = "text_primary"

[highlight]
fill         = "accent_amber"       # Fill color for ==highlighted== text
fill_opacity = 0.25                 # Opacity of fill (0.0–1.0)
text_color   = ""                   # Empty = inherit from surrounding text
border_radius = "2pt"               # Rounded corners on the highlight box

[description_list]
term_font       = "heading"         # Font family for the term (dt)
term_weight     = 600               # SemiBold terms stand out from definitions
term_color      = "text_primary"
definition_indent = "20pt"          # Left indent for the definition (dd)
term_spacing    = "4pt"             # Gap between term and its definition
item_spacing    = "12pt"            # Gap between consecutive dt/dd pairs

# ─── Syntax Highlighting ──────────────────────────────────────
# All color fields can reference [colors] keys or use direct #hex.
# Each token supports color, bold, and italic.

[syntax]
background = "surface"

[syntax.text]
color = "text_primary"

[syntax.keyword]
color  = "#a626a4"
bold   = true

[syntax.string]
color  = "#50a14f"
italic = true

[syntax.number]
color = "#986801"

[syntax.function]
color  = "#4078f2"
bold   = true
italic = true

[syntax.type]
color = "#c18401"

[syntax.comment]
color  = "#a0a1a7"
italic = true

[syntax.constant]
color = "#e45649"

[syntax.boolean]
color = "#e45649"
bold  = true

[syntax.operator]
color = "text_primary"

[syntax.property]
color = "#4078f2"

[syntax.tag]
color = "#e45649"
bold  = true

[syntax.attribute]
color = "#986801"

[syntax.variable]
color = "#e45649"

[syntax.builtin]
color  = "#4078f2"
bold   = true

[syntax.punctuation]
color = "text_muted"

[syntax.escape]
color = "#986801"
bold  = true
```

### 5.3 Built-in Themes (40 Themes)

SilkPrint ships with **40 built-in themes** across 8 aesthetic families. Every theme is a
complete TOML file with colors, fonts, spacing, and syntax highlighting fully specified.

#### Signature Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `silk-light` | light | Yes | Clean, warm, professional — **the default** |
| `silk-dark` | dark | No | Deep navy-black, refined elegance |
| `manuscript` | light | Yes | Warm cream paper, serif-heavy, old-world feel |
| `monochrome` | light | Yes | Pure black on white, zero color, maximum ink efficiency |

#### SilkCircuit Collection (5 variants)

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `silkcircuit-neon` | dark | No | Full Neon (100%) — Electric Purple headings, Neon Cyan accents, Coral constants |
| `silkcircuit-vibrant` | dark | No | Vibrant (85%) — maximum vibrancy, saturated spectrum |
| `silkcircuit-soft` | dark | No | Soft (70%) — reduced chroma for extended reading |
| `silkcircuit-glow` | dark | No | Glow (110%) — maximum contrast, darkest backgrounds |
| `silkcircuit-dawn` | light | Yes | Dawn — deep purples and teals on warm cream |

#### Greyscale Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `greyscale-warm` | light | Yes | Warm grey tones with cream undertones, cozy and readable |
| `greyscale-cool` | light | Yes | Blue-tinted cool greys, clinical and modern |
| `high-contrast` | light | Yes | Extreme B&W, no mid-tones, maximum readability/accessibility |

#### Classic / Literary Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `academic` | light | Yes | Traditional academic paper, conservative and authoritative |
| `typewriter` | light | Yes | Raw mechanical feel, like typed on a real typewriter |
| `newspaper` | light | Yes | Dense editorial feel, bold headlines, ink that stains your fingers |
| `parchment` | light | Yes | Aged warm paper, old-world scholarly, candlewax and leather |

#### Futuristic / Sci-Fi Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `cyberpunk` | dark | No | Hot neon pink + cyan on deep dark, rain-soaked megacity |
| `terminal` | dark | No | Green phosphor on black, classic CRT, cursor blinking in the dark |
| `hologram` | dark | No | Clean blue/white sci-fi, floating projections in a sterile lab |
| `synthwave` | dark | No | Retro-future sunset, chrome sun melting into a grid horizon |
| `matrix` | dark | No | Green cascade on pure void black, reality decoded |

#### Nature Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `forest` | light | Yes | Deep greens, bark browns, dappled light through old-growth canopy |
| `ocean` | dark | No | Navy depths, seafoam teal, living coral accents |
| `sunset` | light | Yes | Warm amber to pink, golden hour painting everything warm |
| `arctic` | light | Yes | Ice blue, silver, crystalline polar silence |
| `sakura` | light | Yes | Cherry blossom pink, matcha green, petals on a garden path |

#### Artistic / Bold Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `noir` | dark | No | Film noir, stark shadows, a single red light cutting through dark |
| `candy` | light | No | Sweet pastels, pop art energy, sugar-coated without the toothache |
| `blueprint` | dark | No | Engineering blueprint, white lines on Prussian blue |
| `witch` | dark | No | Mystical purples, potion green, candlelit grimoire pages |

#### Developer Favorites Collection

| Theme | Variant | Print-Safe | Character |
|-------|---------|------------|-----------|
| `nord` | dark | No | Arctic blue-grey, calm and muted — the Aurora palette |
| `dracula` | dark | No | Dark purple elegance — pink, cyan, green, orange accents |
| `solarized-light` | light | Yes | Ethan Schoonover's classic — warm yellowed paper, precise accents |
| `solarized-dark` | dark | No | Ethan Schoonover's classic — teal depths, same precise accents |
| `catppuccin-mocha` | dark | No | Soothing warm pastels on dark base — cozy and gentle |
| `catppuccin-latte` | light | Yes | Soothing warm pastels on light base — the daytime variant |
| `gruvbox-dark` | dark | No | Retro groove — warm earth tones, bright accents on dark |
| `gruvbox-light` | light | Yes | Retro groove — faded accents on warm creamy paper |
| `tokyo-night` | dark | No | Deep indigo with soft neon — purple, blue, green pop |
| `rose-pine` | dark | No | Soho vibes — muted rose, gold, iris on dusky purple |

#### SilkCircuit Theme Color Mapping

Each SilkCircuit variant maps the design system colors to PDF elements:

| Role | Neon | Vibrant | Soft | Glow | Dawn |
|------|------|---------|------|------|------|
| Page BG | `#12101a` | `#0f0c1a` | `#1a1626` | `#0a0816` | `#faf8ff` |
| Text | `#f8f8f2` | `#f8f8f2` | `#f8f8f2` | `#ffffff` | `#2b2540` |
| Headings | `#80ffea` | `#00ffcc` | `#99ffee` | `#00ffff` | `#007f8e` |
| Accent | `#e135ff` | `#ff00ff` | `#e892ff` | `#ff00ff` | `#7e2bd5` |
| Code BG | `#0a0812` | `#08060f` | `#141220` | `#000000` | `#f1ecff` |
| Keywords | `#e135ff` | `#ff00ff` | `#e892ff` | `#ff00ff` | `#7e2bd5` |
| Functions | `#80ffea` | `#00ffcc` | `#99ffee` | `#00ffff` | `#007f8e` |
| Strings | `#ff99ff` | `#ff99ff` | `#ffc2ff` | `#ff99ff` | `#9c4a88` |
| Numbers | `#ff6ac1` | `#F78C6C` | `#ff99dd` | `#ff66ff` | `#c74a8c` |
| Types | `#f1fa8c` | `#ffcc00` | `#ffe699` | `#ffff00` | `#a88600` |
| Comments | `#8b85a0` | `#8b85a0` | `#8b85a0` | `#6a6a90` | `#8e84a8` |
| Success | `#50fa7b` | `#50fa7b` | `#66ff99` | `#00ff00` | `#2d8659` |
| Warning | `#f1fa8c` | `#f1fa8c` | `#ffe699` | `#ffff00` | `#a88600` |
| Error | `#ff6363` | `#ff6363` | `#ff6677` | `#ff0066` | `#c1272d` |

#### Greyscale Theme Color Mapping

| Role | greyscale-warm | greyscale-cool | high-contrast |
|------|---------------|---------------|---------------|
| Page BG | `#F5F0E8` | `#EBEEF2` | `#FFFFFF` |
| Text | `#3D3632` | `#2B3038` | `#000000` |
| Headings | `#5C534A` | `#3E4550` | `#000000` |
| Accent | `#8B7D6B` | `#5A6A7A` | `#1A1A1A` |
| Code BG | `#EBE4D8` | `#DFE3EA` | `#F0F0F0` |
| Secondary | `#8A8078` | `#7A8594` | `#333333` |
| Border | `#D6CEC3` | `#C8CDD6` | `#000000` |

#### Classic / Literary Theme Color Mapping

| Role | academic | typewriter | newspaper | parchment |
|------|----------|------------|-----------|-----------|
| Page BG | `#FAFAF7` | `#F2EDE4` | `#F0EDE5` | `#F1E8D0` |
| Text | `#1A1A24` | `#1C1915` | `#1A1A1A` | `#3B2F20` |
| Headings | `#1E2A4A` | `#2A2520` | `#0D0D0D` | `#5C3D1E` |
| Accent | `#2B4D8C` | `#6B4F3A` | `#8C1A1A` | `#7B4A2B` |
| Code BG | `#F0EFE9` | `#E8E1D4` | `#E4E0D7` | `#E6DABB` |
| Secondary | `#5C5C6B` | `#706457` | `#4A4A4A` | `#7A6B55` |
| Border | `#D0CFC8` | `#C9BFB0` | `#2A2A2A` | `#C4B590` |

#### Futuristic / Sci-Fi Theme Color Mapping

| Role | cyberpunk | terminal | hologram | synthwave | matrix |
|------|-----------|----------|----------|-----------|--------|
| Page BG | `#0A0A12` | `#0C0C0C` | `#0B1628` | `#1A0A2E` | `#000000` |
| Text | `#D0D0E0` | `#33FF33` | `#C8DBF0` | `#E8D0F0` | `#00B300` |
| Headings | `#FF2E8B` | `#66FF66` | `#FFFFFF` | `#FF6EC7` | `#00FF41` |
| Accent | `#00F0FF` | `#00CC66` | `#4DA8FF` | `#FFA54F` | `#008F11` |
| Code BG | `#12121E` | `#111411` | `#101E35` | `#220E3A` | `#050A05` |
| Secondary | `#7878A0` | `#1AAF1A` | `#7A9AC0` | `#B088C8` | `#006B0A` |
| Border | `#FF2E8B` | `#1A6B1A` | `#2A4A70` | `#6B2FA0` | `#003B00` |

#### Nature Theme Color Mapping

| Role | forest | ocean | sunset | arctic | sakura |
|------|--------|-------|--------|--------|--------|
| Page BG | `#F4F2ED` | `#0D1B2A` | `#FFF8F0` | `#F0F4F8` | `#FDF8F5` |
| Text | `#1E2B1E` | `#C5DBE8` | `#3A2218` | `#1C2A38` | `#3A2B30` |
| Headings | `#2D4A2D` | `#7EC8C8` | `#C44B2B` | `#2E5080` | `#C45C78` |
| Accent | `#4A7C3F` | `#FF7F6B` | `#D4782F` | `#4A90C4` | `#5E8C5A` |
| Code BG | `#E8E6DD` | `#122438` | `#F5EDE0` | `#E4EAF0` | `#F5EDE8` |
| Secondary | `#5E6B52` | `#6A9AB5` | `#8B6B50` | `#6B8098` | `#8C6B72` |
| Border | `#8B7355` | `#1E3A5F` | `#E0C8A8` | `#B8C8D8` | `#E0C8CC` |

#### Artistic / Bold Theme Color Mapping

| Role | noir | candy | blueprint | witch |
|------|------|-------|-----------|-------|
| Page BG | `#0F0F0F` | `#FFF5FA` | `#1B3A5C` | `#110E18` |
| Text | `#D8D8D8` | `#3C2845` | `#D0E0F0` | `#C8B8D8` |
| Headings | `#F0F0F0` | `#E04080` | `#FFFFFF` | `#B040E0` |
| Accent | `#C41E1E` | `#30B0C0` | `#80C0FF` | `#40D890` |
| Code BG | `#1A1A1A` | `#F0E8F0` | `#163050` | `#1A1524` |
| Secondary | `#888888` | `#8868A0` | `#8AAAC8` | `#8070A0` |
| Border | `#333333` | `#E8C0D8` | `#2A5580` | `#3A2858` |

#### Developer Favorites Theme Color Mapping

| Role | nord | dracula | solarized-light | solarized-dark |
|------|------|---------|-----------------|----------------|
| Page BG | `#2E3440` | `#282A36` | `#FDF6E3` | `#002B36` |
| Text | `#D8DEE9` | `#F8F8F2` | `#657B83` | `#839496` |
| Headings | `#ECEFF4` | `#BD93F9` | `#073642` | `#93A1A1` |
| Accent | `#88C0D0` | `#FF79C6` | `#268BD2` | `#268BD2` |
| Code BG | `#3B4252` | `#44475A` | `#EEE8D5` | `#073642` |
| Secondary | `#81A1C1` | `#6272A4` | `#93A1A1` | `#586E75` |
| Border | `#4C566A` | `#44475A` | `#93A1A1` | `#586E75` |

| Role | catppuccin-mocha | catppuccin-latte | gruvbox-dark | gruvbox-light |
|------|------------------|------------------|--------------|---------------|
| Page BG | `#1E1E2E` | `#EFF1F5` | `#282828` | `#FBF1C7` |
| Text | `#CDD6F4` | `#4C4F69` | `#EBDBB2` | `#3C3836` |
| Headings | `#CBA6F7` | `#8839EF` | `#FABD2F` | `#B57614` |
| Accent | `#89B4FA` | `#1E66F5` | `#83A598` | `#076678` |
| Code BG | `#313244` | `#DCE0E8` | `#1D2021` | `#F9F5D7` |
| Secondary | `#A6ADC8` | `#6C6F85` | `#A89984` | `#665C54` |
| Border | `#45475A` | `#BCC0CC` | `#504945` | `#D5C4A1` |

| Role | tokyo-night | rose-pine |
|------|-------------|-----------|
| Page BG | `#1A1B26` | `#191724` |
| Text | `#A9B1D6` | `#E0DEF4` |
| Headings | `#7AA2F7` | `#C4A7E7` |
| Accent | `#BB9AF7` | `#EB6F92` |
| Code BG | `#24283B` | `#1F1D2E` |
| Secondary | `#565F89` | `#908CAA` |
| Border | `#414868` | `#26233A` |

### 5.4 Custom Themes

Users place custom `.toml` files in:

- `~/.config/silkprint/themes/` (user-global)
- `./.silkprint/themes/` (project-local)
- Or pass a direct path: `--theme ./my-theme.toml`

Custom themes can extend built-in themes by specifying only overrides:

```toml
[meta]
name = "My Custom Theme"
extends = "silk-light"     # Start from silk-light, override below

[colors]
accent_blue = "#7c3aed"   # Change accent to violet

[headings]
color = "accent_blue"      # Use the new violet for headings
```

### 5.5 Theme Resolution

```
 1. Build inheritance chain: theme → extends → extends...
 2. Detect cycles (error if any theme appears twice)
 3. Cap chain at depth 5 (error: ThemeInheritanceDepth if exceeded)
 4. Resolve from bottom up: deepest ancestor first, each descendant merges on top
 5. Array fields (fallback chains): REPLACE, not append (document loudly for theme authors)
 6. Auto-prepend primary font to fallback chain if not already present
 7. If [syntax.*] tables absent from FINAL merged result, inherit from _base-syntax-light or
    _base-syntax-dark (matched by [meta].variant). If a parent theme has [syntax.*], it carries
    forward through the merge in step 4 — base-syntax only applies when NO theme in the chain
    provides syntax tables
 8. Resolve all color references: [colors] table first (aliases within [colors]),
    then all semantic/component color fields
 9. Validate all values (hex format, valid units, required fields)
10. WCAG contrast warnings — check ALL foreground/background pairs:
    - Body text vs page background (4.5:1 AA)
    - Heading text vs page background (3:1 for large text H1-H3, 4.5:1 for H4-H6)
    - Link color vs page background (4.5:1)
    - Inline code text vs inline code background (4.5:1)
    - Blockquote text vs page background (4.5:1)
    - Table header text vs header background (4.5:1)
    - Each alert type text vs alert background (4.5:1)
    - Caption/footnote text vs page background (4.5:1)
    - Page number color vs page background (3:1)
    - Syntax token colors vs code block background (4.5:1 each)
11. If print_safe = true, run PrintSafeValidator (errors, not warnings):
    - Page background luminance >= 0.85 (near-white)
    - Code block background luminance >= 0.75
    - Primary text luminance <= 0.15 (near-black)
    - Body text contrast >= 7:1 (WCAG AAA)
    - Heading contrast >= 4.5:1
    - Accent color luminance <= 0.50 (no neon on paper)
    - No dark table headers (header bg luminance >= 0.80)
12. Generate tmTheme XML from resolved [syntax.*] tables
13. Convert resolved theme to Typst set/show rules
```

---

## 6. Typography Specification

### 6.1 Type Scale (Major Third — 1.250 ratio)

| Element | Size | Weight | Line Height | Font |
|---------|------|--------|-------------|------|
| Body | 11pt | 400 (Regular) | 1.5 | Source Serif 4 |
| Small / Caption | 9pt | 400 | 1.4 | Source Serif 4 |
| Inline Code | 10pt | 400 | inherit | JetBrains Mono |
| Code Block | 10pt | 400 | 1.45 | JetBrains Mono |
| H6 | 11pt | 600 + UPPERCASE | 1.25 | Inter |
| H5 | 14pt | 500 (Medium) | 1.25 | Inter |
| H4 | 17pt | 500 (Medium) | 1.2 | Inter |
| H3 | 21.5pt | 600 (SemiBold) | 1.2 | Inter |
| H2 | 27pt | 600 (SemiBold) | 1.15 | Inter |
| H1 | 33.5pt | 700 (Bold) | 1.1 | Inter |

**H6 note:** Same size as body but differentiated via Inter SemiBold + uppercase + 0.05em
letter-spacing. This creates clear visual separation without inflating the scale.

### 6.2 Spacing (Absolute Units)

Heading spacing uses **absolute units** to avoid the trap of `em` values scaling with heading
font size (which would produce absurdly large gaps).

| Element | Above | Below |
|---------|-------|-------|
| H1 | 36pt | 12pt |
| H2 | 28pt | 8pt |
| H3 | 22pt | 6pt |
| H4–H6 | 18pt | 4pt |
| Paragraph | 0 | 0.85em (body-relative) |
| Code block | 16pt | 16pt |
| Blockquote | 16pt | 16pt |
| Table | 16pt | 16pt |
| Horizontal rule | 28pt | 28pt |
| List item | 0 | 3pt |
| Alert box | 16pt | 16pt |

### 6.3 Page Layout

Symmetric margins for single-sided documents (the default). All margins are
theme-configurable.

| Property | Value (A4) | Value (Letter) |
|----------|-----------|----------------|
| Width | 210mm | 216mm (8.5in) |
| Height | 297mm | 279mm (11in) |
| Top margin | 25mm | 25mm |
| Bottom margin | 30mm | 30mm |
| Left margin | 25mm | 25mm |
| Right margin | 25mm | 25mm |
| Text block width | 160mm | 166mm |
| Chars per line | ~72 | ~74 |

At 11pt Source Serif 4 with ~5.5pt average character width in a 160mm (453pt) text block:
~82 characters. However, proportional text with mixed case and punctuation averages lower;
real-world English prose typically lands at **~72 characters per line** — within the 66–75
optimal range.

### 6.4 Microtypography

- **Smart quotes:** `"` → `"` `"`, `'` → `'` `'` — locale-aware via `lang` parameter
- **Em/en dashes:** `---` → `—`, `--` → `–`
- **Ligatures:** ON for paragraph text and blockquotes only. OFF everywhere else (code, links,
  tables, headings, list items) to avoid corrupting identifiers and URLs
- **Hyphenation:** Enabled for body text only. Disabled in headings, code, tables, list items.
  Minimum fragment: 3 characters before/after break
- **Justification:** Full justification for body (theme-configurable). Left-aligned for
  headings, code, lists, tables
- **Kerning:** Automatic via Typst's rustybuzz text shaping
- **Oldstyle figures:** Enabled for body text in themes that use serif fonts. Lining figures
  for headings and tables

---

## 7. Markdown Feature Support

### 7.1 Complete Feature Matrix

**Everything is P0.** The tool renders every Markdown feature at launch.

| Feature | comrak Extension | Typst Rendering |
|---------|-----------------|-----------------|
| Paragraphs | core | `#par()` |
| Headings (H1–H6) | core | `#heading()` with per-level show rules |
| Bold / Italic | core | `*bold*`, `_italic_` |
| Strikethrough | GFM | `#strike()` themed color |
| Underline | comrak | `#underline()` |
| Superscript | comrak | `#super()` |
| Subscript | comrak | `#sub()` |
| Highlight / Mark | comrak | `#highlight(fill: theme)` — themed fill color from `[highlight]` |
| Links (inline & ref) | core | `#link()` with themed color + underline |
| Autolinks | GFM | `#link()` |
| Images | core | `#image()` with sizing + caption |
| Unordered lists | core | `#list()` with themed bullets |
| Ordered lists | core | `#enum()` with themed numbering |
| Nested lists | core | Progressive indent + bullet/number style |
| Task lists (checkboxes) | GFM | Custom checkbox glyphs, themed colors |
| Definition / description lists | comrak | `#terms()` with themed term styling from `[description_list]` |
| Code blocks (fenced) | core | `#raw(block: true, lang: ...)` with Typst highlighting |
| Code blocks (indented) | core | `#raw(block: true)` |
| Inline code | core | `#raw()` with background box |
| Tables (GFM) | GFM | `#table()` Tufte-style, column alignment |
| Blockquotes | core | `#quote()` with left border |
| Nested blockquotes | core | Progressive indent + muted border |
| Horizontal rules | core | `#line()` themed style |
| Line breaks (hard) | core | `#linebreak()` |
| Footnotes | comrak | Typst native `#footnote()` — automatic numbering + page-bottom placement |
| Math (inline `$...$`) | comrak (`math_dollars`) | Typst `$...$` (inline — no spaces inside delimiters). **v0.1: Typst-native math only** — `\frac{a}{b}` will error; users must write Typst math (`frac(a, b)`). LaTeX→Typst translation via `mitex` Typst package deferred to v0.2 |
| Math (display `$$...$$`) | comrak (`math_dollars`) | Typst `$ ... $` (display — spaces inside delimiters). Same Typst-native constraint applies |
| Front matter (YAML) | comrak (`front_matter_delimiter = Some("---")`) | Metadata extraction → title page + PDF metadata |
| Alerts (GitHub-style callouts) | comrak (`alerts`) | Themed boxes: NOTE, TIP, IMPORTANT, WARNING, CAUTION |
| Emoji shortcodes | comrak | `:rocket:` → Unicode emoji character in PDF |
| HTML entities | core | `&amp;` → `&`, `&mdash;` → `—`, etc. |
| Escape sequences | core | `\*` → literal `*` |
| Table of contents | generated | `#outline()` from heading tree |
| Title page | generated | From front matter (title, subtitle, author, date) |
| Page numbers | generated | Themed footer |
| PDF metadata | generated | Title, author, date, producer in PDF info dict |
| Wikilinks | comrak (`wikilinks_title_after_pipe`) | `#link()` — `[[url\|title]]` → `#link("url")[title]`, `[[page]]` → `#link("page")[page]` |
| PDF bookmarks | generated | From heading tree (clickable outline) |

### 7.2 Syntax Highlighting

Typst's built-in `raw` block handles syntax highlighting for **100+ languages** natively. No
syntect dependency needed. However, Typst does **not** expose individual syntax tokens via show
rules — custom syntax coloring requires **tmTheme files** (TextMate theme format).

**Architecture:** SilkPrint generates tmTheme XML at runtime from each theme's `[syntax.*]` TOML
tables. The XML is served as a virtual file via `World::file()` at path `/__silkprint_theme.tmTheme`,
and referenced in the Typst preamble via `#set raw(theme: "/__silkprint_theme.tmTheme")`.

**Token → TextMate scope mapping:**

| TOML Token | TextMate Scope |
|---|---|
| `syntax.keyword` | `keyword` |
| `syntax.string` | `string` |
| `syntax.number` | `constant.numeric` |
| `syntax.function` | `entity.name.function` |
| `syntax.type` | `entity.name.type`, `support.type` |
| `syntax.comment` | `comment` |
| `syntax.constant` | `constant` |
| `syntax.boolean` | `constant.language` |
| `syntax.operator` | `keyword.operator` |
| `syntax.property` | `variable.other.property` |
| `syntax.tag` | `entity.name.tag` |
| `syntax.attribute` | `entity.other.attribute-name` |
| `syntax.variable` | `variable` |
| `syntax.builtin` | `support.function` |
| `syntax.punctuation` | `punctuation` |
| `syntax.escape` | `constant.character.escape` |

The `theme/tmtheme.rs` module generates valid tmTheme XML from these mappings. Each token
supports `color`, `bold`, and `italic` properties.

**Base syntax inheritance:** Themes that omit `[syntax.*]` tables inherit from `_base-syntax-light`
or `_base-syntax-dark` (matched by `[meta].variant`). Only themes with established palettes (e.g.,
developer favorites) need to specify syntax overrides. This reduces most theme files to ~80-100
lines.

### 7.3 Emoji Rendering

Emoji shortcodes (`:heart:`, `:fire:`, `:sparkles:`) are resolved to Unicode emoji characters
by comrak's shortcode extension. These render in the PDF using the system's emoji font or
Typst's fallback font handling. For consistent cross-platform rendering, the emitter uses
Unicode codepoints directly.

### 7.4 Image Handling

- **Path resolution:** Relative to the input `.md` file's directory
- **Supported formats:** PNG, JPEG, GIF, SVG (all supported by Typst)
- **Remote URLs:** Not downloaded — emit a warning and skip
- **Max width:** Capped at text block width (100%), aspect ratio preserved
- **Alt text → caption:** If alt text is non-empty, rendered as a styled caption below
- **Missing images:** Emit a warning, render a placeholder with the path

### 7.5 Alert Syntax (GitHub-style)

comrak's `alerts` extension supports GitHub-style alert blocks. The extension field is
`options.extension.alerts = true` (not "admonitions"). Five standard types:

```markdown
> [!NOTE]
> Useful information that users should know.

> [!TIP]
> Helpful advice for doing things better.

> [!IMPORTANT]
> Key information users need to know.

> [!WARNING]
> Urgent info that needs immediate user attention.

> [!CAUTION]
> Advises about risks or negative outcomes.
```

Each type gets a themed color, optional icon, and label. The `[alerts]` theme section
controls all styling.

### 7.6 Wikilinks

comrak's `wikilinks_title_after_pipe` extension parses `[[target|display text]]` links (GitHub
convention — target before pipe, display text after).

**Typst rendering:**

| Markdown | Typst Output | Behavior |
|---|---|---|
| `[[page]]` | `#link("page")[page]` | Target used as both URL and display text |
| `[[url\|title]]` | `#link("url")[title]` | Pipe separates target from display |
| `[[page.md]]` | `#link("page.md")[page.md]` | Extensions preserved as-is |

Wikilinks are treated as **opaque URLs** — SilkPrint does not resolve them to anchors, strip
extensions, or validate targets. They render identically to inline links with themed color and
underline from `[links]`. This keeps the emitter simple; future versions may add internal
anchor resolution for multi-file mode.

### 7.7 Footnotes

comrak parses reference-style footnotes (`[^1]` markers with `[^1]: definition` blocks) into
`FootnoteDefinition` and `FootnoteReference` AST nodes. SilkPrint translates these to Typst's
native `#footnote()` function, which automatically handles:

- Sequential numbering (restarting per-page or continuous, depending on Typst defaults)
- Superscript markers at the reference site
- Collected definitions at the page bottom with a separator line

**Translation:**

```typst
// Markdown: Something important[^1]
// [^1]: The source for this claim.
Something important#footnote[The source for this claim.]
```

**Styling via `[footnotes]` theme section:**

```typst
#show footnote.entry: it => {
  let loc = it.note.location()
  // Separator line
  line(length: 33%, stroke: 0.5pt + rgb("#e2e2e8"))
  v(4pt)
  // Footnote text with themed size and number color
  set text(size: 9pt)
  [#text(fill: rgb("#4a5dbd"))[#it.note.counter.display()] #it.note.body]
}
```

The `separator_color`, `separator_width`, `text_size`, `number_color` fields from `[footnotes]`
map directly to the show rule parameters above. `backref_color` is reserved for future back-link
styling (Typst does not currently support clickable back-references in footnotes).

### 7.8 Description Lists

comrak's `description_lists` extension parses `<dt>`/`<dd>` pairs. SilkPrint translates these
to Typst's `#terms()` function:

```typst
// Markdown:
// Term 1
// : Definition for term 1
//
// Term 2
// : Definition for term 2

#terms(
  [*Term 1*], [Definition for term 1],
  [*Term 2*], [Definition for term 2],
)
```

**Styling via `[description_list]` theme section:** Term font, weight, and color are applied via
`#set text()` inside the term slots. Definition indent is controlled by `#set terms(indent: ...)`.
Spacing between items uses `#set terms(spacing: ...)`.

---

## 8. Dependencies

### 8.1 Cargo.toml

```toml
[package]
name = "silkprint"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
authors = ["Stefanie Jane <stef@hyperbliss.tech>"]
description = "Transform Markdown into stunning PDFs with electric elegance"
readme = "README.md"
homepage = "https://github.com/hyperb1iss/silkprint"
repository = "https://github.com/hyperb1iss/silkprint"
license = "Apache-2.0"
keywords = ["markdown", "pdf", "typesetting", "cli", "typography"]
categories = ["command-line-utilities", "text-processing"]

[lib]
name = "silkprint"
path = "src/lib.rs"

[[bin]]
name = "silkprint"
path = "src/main.rs"

[dependencies]
# Markdown parsing — shortcodes feature enables emoji crate; extensions enabled at RUNTIME
# via Options.extension.* fields (all default to false). bon builder excluded intentionally.
comrak = { version = "0.50", default-features = false, features = ["shortcodes"] }

# Typst typesetting engine — direct World trait impl (no typst-as-lib wrapper)
typst = "0.14"
typst-pdf = "0.14"

# CLI
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
owo-colors = "4"
indicatif = "0.17"

# Error handling — thiserror for types, miette for rendering (no anyhow)
thiserror = "2.0"
miette = { version = "7", features = ["fancy"] }

# Configuration & serialization
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
serde_yaml_ng = "0.44"            # YAML front matter (replaces archived serde_yml)

# Font embedding — compressed at compile time
rust-embed = { version = "8", features = ["compression"] }

# Utilities
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
directories = "5"                  # XDG config paths
open = "5"                         # Cross-platform "open file" for --open flag

[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
tempfile = "3"
pretty_assertions = "1"
assert_cmd = "2"                   # CLI integration testing
predicates = "3"                   # Assertion helpers for assert_cmd
lopdf = "0.34"                     # PDF structure validation in tests

# ════════════════════════════════════════════════════════════════
# Lints — from conventions/rust/Cargo.toml.template
# ════════════════════════════════════════════════════════════════

[lints.rust]
unsafe_code = "forbid"
missing_docs = { level = "allow", priority = 1 }

[lints.clippy]
all = { level = "deny", priority = 0 }
style = { level = "warn", priority = 1 }
perf = { level = "deny", priority = 1 }
pedantic = { level = "warn", priority = 10 }      # warn not deny — pragmatic during dev
missing_errors_doc = { level = "allow", priority = 20 }
missing_panics_doc = { level = "allow", priority = 20 }
missing_safety_doc = { level = "allow", priority = 20 }
module_name_repetitions = { level = "allow", priority = 21 }
significant_drop_tightening = { level = "allow", priority = 21 }
must_use_candidate = { level = "allow", priority = 21 }
cast_precision_loss = { level = "warn", priority = 22 }
cast_possible_truncation = { level = "warn", priority = 22 }
cast_sign_loss = { level = "warn", priority = 22 }
as_conversions = { level = "warn", priority = 22 }
out_of_bounds_indexing = { level = "deny", priority = 30 }
enum_glob_use = { level = "deny", priority = 30 }
unwrap_used = { level = "deny", priority = 30 }
undocumented_unsafe_blocks = { level = "deny", priority = 30 }
dbg_macro = { level = "warn", priority = 31 }
todo = { level = "warn", priority = 31 }
implicit_clone = { level = "warn", priority = 33 }
inefficient_to_string = { level = "warn", priority = 33 }
string_lit_as_bytes = { level = "warn", priority = 33 }
too_many_lines = { level = "warn", priority = 34 }
cognitive_complexity = { level = "warn", priority = 34 }
result_large_err = { level = "warn", priority = 35 }
manual_let_else = { level = "warn", priority = 36 }
redundant_else = { level = "warn", priority = 36 }
semicolon_if_nothing_returned = { level = "warn", priority = 36 }
cloned_instead_of_copied = { level = "warn", priority = 36 }
flat_map_option = { level = "warn", priority = 36 }
from_iter_instead_of_collect = { level = "warn", priority = 36 }
needless_pass_by_value = { level = "warn", priority = 36 }
trivially_copy_pass_by_ref = { level = "warn", priority = 36 }
if_not_else = { level = "warn", priority = 36 }
match_same_arms = { level = "warn", priority = 36 }
needless_continue = { level = "warn", priority = 36 }
wildcard_imports = { level = "deny", priority = 37 }
print_stdout = { level = "warn", priority = 37 }
print_stderr = { level = "warn", priority = 37 }
multiple_crate_versions = { level = "allow", priority = 40 }

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

[package.metadata.deb]
maintainer = "Stefanie Jane <stef@hyperbliss.tech>"
copyright = "2026, silkprint Contributors <https://github.com/hyperb1iss/silkprint>"
license-file = ["LICENSE", "4"]
extended-description = "Transform Markdown into stunning PDFs with electric elegance"
depends = "$auto"
section = "utility"
priority = "optional"
assets = [
    ["target/release/silkprint", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/silkprint/README", "644"],
]

[package.metadata.generate-rpm]
assets = [
    { source = "target/release/silkprint", dest = "/usr/bin/silkprint", mode = "755" },
    { source = "README.md", dest = "/usr/share/doc/silkprint/README", mode = "644" },
]

[profile.release]
opt-level = 2                      # Speed over size — font data dominates binary anyway,
                                   # and Typst compilation benefits from optimized codegen
lto = true
codegen-units = 1
strip = true
```

### 8.2 comrak Runtime Extension Configuration

Cargo features (`shortcodes`) only gate compile-time dependencies. All parsing extensions must be
enabled at runtime via `Options.extension.*` fields (all default to `false`):

```rust
let mut options = comrak::Options::default();

// Core extensions
options.extension.strikethrough = true;
options.extension.table = true;
options.extension.autolink = true;
options.extension.tasklist = true;
options.extension.superscript = true;
options.extension.subscript = true;
options.extension.footnotes = true;
options.extension.description_lists = true;
options.extension.highlight = true;
options.extension.underline = true;

// Math, front matter, alerts
options.extension.math_dollars = true;        // $...$ and $$...$$
// Note: math_code exists but intentionally excluded (code-fence math not needed)
options.extension.front_matter_delimiter = Some("---".to_owned());  // Option<String>, not bool!
options.extension.alerts = true;              // GitHub-style > [!NOTE] blocks

// Emoji and wikilinks
options.extension.shortcodes = true;          // Requires `shortcodes` Cargo feature
options.extension.wikilinks_title_after_pipe = true;  // [[url|title]] (GitHub convention)
```

**Intentionally excluded extensions:**

| Extension | Reason |
|---|---|
| `spoiler` | Discord/Reddit feature, not standard markdown |
| `greentext` | 4chan-style, not relevant |
| `multiline_block_quotes` | `>>>` syntax, niche |
| `math_code` | Code-fence math, redundant with `math_dollars` |
| `subtext` | Discord-style `-#` subscript blocks |
| `wikilinks_title_before_pipe` | Using `title_after_pipe` instead |

### 8.3 Public Library API

The `lib.rs` exposes a clean public API for programmatic use:

```rust
/// Render markdown to PDF bytes.
pub fn render(input: &str, options: RenderOptions) -> Result<Vec<u8>, SilkprintError>;

/// Render markdown to Typst source (intermediate representation).
pub fn render_to_typst(input: &str, options: RenderOptions) -> Result<String, SilkprintError>;

pub struct RenderOptions {
    pub theme: ThemeSource,
    pub paper: PaperSize,
    pub font_dirs: Vec<PathBuf>,
    pub toc: Option<bool>,         // None = use front matter / theme default
    pub title_page: Option<bool>,
}

pub enum ThemeSource {
    BuiltIn(String),
    Custom(PathBuf),
    Inline(String),  // Raw TOML string
}
```

Everything else is `pub(crate)`. The `main.rs` is thin: parse CLI args, call `render()`, handle
errors with miette, set up tracing.

---

## 9. Implementation Phases

### Phase 1: Foundation (MVP)

**Goal:** Render full-featured Markdown to a beautiful PDF with the default theme.

**Deliverables:**

1. **Project scaffolding** — Cargo.toml, directory structure, CLAUDE.md, LICENSE, .gitignore
2. **CLI skeleton** — clap argument parsing with styled help, `--open`, `--check`, `--dump-typst`,
   `-o -` stdout support, `--color`, `--verbose` / `--quiet`
3. **Markdown parsing** — comrak with extensions enabled at runtime via `Options.extension.*`,
   front matter extraction via serde_yaml_ng
4. **Typst emitter (complete)** — Translate ALL AST node types to Typst markup:
   - Paragraphs, headings (H1–H6), bold, italic, strikethrough, underline
   - Superscript, subscript, highlight/mark
   - Links (inline, reference, autolink), images with captions
   - Ordered lists, unordered lists, nested lists, task lists
   - Definition/description lists
   - Fenced and indented code blocks with Typst-native syntax highlighting
   - Inline code with background
   - Blockquotes (including nested)
   - Horizontal rules
   - Tables (GFM with column alignment)
   - Footnotes with superscript markers and page-bottom rendering
   - Math (inline and display — Typst-native syntax only; LaTeX→Typst translation deferred to v0.2)
   - Alerts (NOTE, TIP, IMPORTANT, WARNING, CAUTION)
   - Emoji shortcodes → Unicode characters
   - HTML entities
   - Escape sequences
5. **Typst compilation** — Direct World trait implementation, font loading, image
   resolution relative to input file, PDF export with metadata via `#set document()`
6. **Default theme** — `silk-light` theme fully implemented
7. **Font embedding** — Inter, Source Serif 4, JetBrains Mono bundled via rust-embed
8. **Page numbers** — Themed footer (one line of Typst, too basic to defer)
9. **PDF metadata** — Title, author, date, "Created with SilkPrint" producer
10. **PDF bookmarks** — Clickable outline from heading tree
11. **Error handling** — miette diagnostics for all error types
12. **Warning system** — Non-fatal warnings for missing images, font fallbacks, unknown languages
13. **`--list-themes`** — Even with only silk-light, shows the system is extensible

**Acceptance criteria:**
- `silkprint README.md` produces a beautiful, fully-featured PDF
- Every Markdown feature renders correctly
- `silkprint README.md --open` renders and opens the PDF
- `silkprint README.md --dump-typst` outputs valid Typst
- `silkprint README.md --check` validates without rendering
- `silkprint README.md -o -` writes PDF to stdout
- Font embedding works (PDF is self-contained)
- CLI help is styled with SilkCircuit colors
- Errors are beautiful. Warnings are visible.

### Phase 2: Theme Engine

**Goal:** Full theme system with all 40 built-in themes across 8 families.

**Deliverables:**

1. **Theme TOML parser** — Deserialize full schema into typed Rust structs
2. **Token resolution** — Reference resolution (name → hex), validation, WCAG contrast warnings
3. **Theme inheritance** — `extends` field with cycle detection, depth cap at 5, array-replace
   semantics
4. **All 40 built-in themes** (silk-light already from Phase 1):
   - Signature: `silk-dark`, `manuscript`, `monochrome`
   - SilkCircuit: `silkcircuit-neon`, `silkcircuit-vibrant`, `silkcircuit-soft`,
     `silkcircuit-glow`, `silkcircuit-dawn`
   - Greyscale: `greyscale-warm`, `greyscale-cool`, `high-contrast`
   - Classic: `academic`, `typewriter`, `newspaper`, `parchment`
   - Futuristic: `cyberpunk`, `terminal`, `hologram`, `synthwave`, `matrix`
   - Nature: `forest`, `ocean`, `sunset`, `arctic`, `sakura`
   - Artistic: `noir`, `candy`, `blueprint`, `witch`
   - Dev Favorites: `nord`, `dracula`, `solarized-light`, `solarized-dark`,
     `catppuccin-mocha`, `catppuccin-latte`, `gruvbox-dark`, `gruvbox-light`,
     `tokyo-night`, `rose-pine`
5. **Custom theme loading** — XDG paths, project-local, direct path
6. **Font configurability** — Full per-theme font selection with fallback chains and optional
   bundled font sources
7. **Syntax highlight theming** — `[syntax.*]` tables → tmTheme XML generation → `#set raw(theme: ...)`
   Base syntax inheritance from `_base-syntax-light/dark` for themes that omit `[syntax.*]`
8. **Table of contents** — Auto-generated `#outline()` from headings, themed
9. **Title page** — Generated from front matter, themed

**Acceptance criteria:**
- All 40 themes produce visually distinct, polished PDFs
- Each SilkCircuit variant matches the STYLE_GUIDE exactly
- Custom TOML themes load with proper inheritance
- Invalid themes produce clear miette diagnostics with source spans
- WCAG contrast warnings fire for problematic color combinations

### Phase 3: HTML Output

**Goal:** Beautiful single-page HTML alongside PDF.

1. **HTML emitter** — AST to semantic HTML5 with CSS classes, same pipeline as Typst emitter
2. **Theme → CSS** — Generate complete CSS stylesheets from TOML themes, CSS custom properties
3. **Syntax highlighting** — highlight.js or Prism embedded for 100+ language support
4. **Self-contained output** — Inline all CSS, fonts (base64), and assets into one HTML file
5. **`--format html|pdf`** — CLI flag for output selection (default: pdf)
6. **Browser launch** — `--open` works for HTML too (opens in default browser)
7. **Responsive layout** — Mobile-friendly with print media queries for Ctrl+P
8. **Visual debugging** — `--render-pages <dir>` renders PDF pages to PNG for AI-assisted QA

**Acceptance criteria:**
- `silkprint doc.md --format html --open` produces a stunning page in your browser
- Every theme looks great in both PDF and HTML
- Syntax highlighting is gorgeous with theme-matched colors
- HTML is fully self-contained (works offline, single file)

### Phase 4: Polish & Distribution

**Goal:** Ship it.

1. **cargo-dist setup** — Automated releases for Linux (x64/arm64), macOS (x64/arm64), Windows
2. **Homebrew formula** — `brew install hyperb1iss/tap/silkprint`
3. **AUR package** — For the Arch Linux early-adopter crowd
4. **crates.io publishing** — `cargo install silkprint`
5. **GitHub Actions CI** — Lint, test, clippy, cross-platform build
6. **Shell completions** — bash, zsh, fish via `clap_complete`
7. **README** — Per conventions template, with rendered PDF screenshots
8. **CLAUDE.md** — Project-specific AI instructions

---

## 10. Typst Integration Details

### 10.1 World Trait Implementation

Implement the `World` trait directly against `typst` 0.14 (~150 lines of boilerplate). This gives
full control over font loading, file resolution, and compilation without depending on a third-party
wrapper. The `typst-as-lib` crate was designed for templating (inject data into placeholders);
SilkPrint generates complete Typst source, making a direct `World` impl the cleaner fit.

```rust
use typst::World;
use typst::foundations::{Bytes, Datetime};
use typst::text::{Font, FontBook};

struct SilkWorld {
    library: typst::Library,
    book: FontBook,
    fonts: Vec<Font>,
    main_source: typst::Source,
    // File resolver rooted at input file's parent directory
    root: PathBuf,
}

impl World for SilkWorld {
    fn library(&self) -> &typst::Library { &self.library }
    fn book(&self) -> &FontBook { &self.book }
    fn main(&self) -> typst::Source { self.main_source.clone() }
    fn source(&self, id: FileId) -> FileResult<typst::Source> { /* ... */ }
    fn file(&self, id: FileId) -> FileResult<Bytes> { /* ... */ }
    fn font(&self, index: usize) -> Option<Font> { self.fonts.get(index).cloned() }
    fn today(&self, _offset: Option<i64>) -> Option<Datetime> { /* ... */ }
}

// Compile to a PagedDocument
let world = SilkWorld::new(typst_source, &fonts, &input_dir)?;
let document = typst::compile(&world);
// document is Warned<SourceResult<Document>> — handle warnings + errors
let paged = match document.output {
    Ok(doc) => doc,
    Err(diagnostics) => return Err(SilkprintError::TypstCompilation {
        diagnostics: diagnostics.iter().map(|d| d.message.to_string()).collect(),
    }),
};

// Export to PDF — metadata set via #set document() in the Typst preamble
let pdf_options = typst_pdf::PdfOptions {
    timestamp: Some(typst_pdf::Timestamp::now()),
    ..Default::default()
};
let pdf_bytes = typst_pdf::pdf(&paged, &pdf_options)
    .map_err(|diags| SilkprintError::TypstCompilation {
        diagnostics: diags.iter().map(|d| d.message.to_string()).collect(),
    })?;
```

**Key points:**
- **No `title`/`author` in `PdfOptions`** — those are set in the Typst source via
  `#set document(title: "...", author: ("...",))`. The Typst compiler embeds them in PDF metadata
  automatically during compilation
- **`SourceResult` errors** require explicit mapping — the error type `Vec<SourceDiagnostic>` does
  not implement `std::error::Error`, so `?` won't auto-convert
- **Font discovery is silent-fail** — Typst falls back without errors if fonts aren't found.
  Validate that required fonts loaded into `FontBook` after world construction
- **File resolution sandbox** — the `World::file()` impl must be rooted at the input file's
  parent directory for image paths to resolve correctly
- **Compilation warnings** from `Warned<...>` should be collected and mapped to
  `SilkprintWarning` entries for display

### 10.2 Generated Typst Structure

The emitter produces a complete Typst document. Example output:

```typst
// ─── Document Metadata (embedded in PDF info dict) ───────
#set document(
  title: "My Document",
  author: ("Stefanie Jane",),
)

// ─── Theme Configuration (set/show rules) ────────────────
#set page(
  paper: "a4",
  margin: (top: 25mm, bottom: 30mm, left: 25mm, right: 25mm),
  fill: rgb("#ffffff"),
  numbering: "1",
  number-align: center + bottom,
)

// Apply tmTheme for syntax highlighting (generated from [syntax.*] TOML)
// The tmTheme XML is served as a virtual file via World::file() at "/__silkprint_theme.tmTheme"
#set raw(theme: "/__silkprint_theme.tmTheme")

#set text(
  font: ("Source Serif 4", "Georgia", "Times New Roman"),
  size: 11pt,
  fill: rgb("#1a1a2e"),
  lang: "en",
  hyphenate: true,
  ligatures: true,
)

#set par(
  justify: true,
  leading: 0.5em,      // Conversion: leading = (line_height - 1.0) * font_size → (1.5 - 1.0) * 1em = 0.5em
  spacing: 0.85em,
)

// Heading styles with absolute spacing
#show heading.where(level: 1): it => {
  v(36pt)
  block(below: 12pt)[
    #set text(font: "Inter", size: 33.5pt, weight: 700, fill: rgb("#1a1a2e"))
    #it.body
  ]
  line(length: 100%, stroke: 0.5pt + rgb("#e2e2e8"))
}

// H6 with uppercase treatment
#show heading.where(level: 6): it => {
  v(18pt)
  block(below: 4pt)[
    #set text(font: "Inter", size: 11pt, weight: 600, fill: rgb("#1a1a2e"),
              tracking: 0.05em)
    #upper(it.body)
  ]
}

// Code block styling with soft-wrap
#show raw.where(block: true): it => {
  block(
    fill: rgb("#f4f4f8"),
    stroke: 0.5pt + rgb("#e2e2e8"),
    radius: 6pt,
    inset: (x: 14pt, y: 12pt),
    width: 100%,
    breakable: true,
  )[
    #set text(font: "JetBrains Mono", size: 10pt, ligatures: false)
    #set par(justify: false, leading: 0.45em)
    #it
  ]
}

// Inline code
#show raw.where(block: false): it => {
  box(
    fill: rgb("#f4f4f8"),
    stroke: 0.5pt + rgb("#e2e2e8"),
    radius: 3pt,
    inset: (x: 3pt, y: 1.5pt),
  )[
    #set text(font: "JetBrains Mono", size: 10pt, ligatures: false)
    #it
  ]
}

// Links — themed color + underline + no ligatures
#show link: it => {
  set text(fill: rgb("#4a5dbd"), ligatures: false)
  underline(it)
}

// Blockquote — left border accent with optional background
#show quote.where(block: true): it => {
  block(
    stroke: (left: 2.5pt + rgb("#4a5dbd")),
    inset: (left: 14pt, y: 8pt, right: 8pt),
    width: 100%,
  )[
    #set text(fill: rgb("#555570"))
    #emph(it.body)
  ]
}

// Table — Tufte-style (horizontal lines only), striped rows
#set table(
  stroke: none,
  inset: (x: 10pt, y: 6pt),
)
#show table.cell.where(y: 0): set text(
  font: "Inter", weight: 600, fill: rgb("#1a1a2e"),
)
#show table.cell.where(y: 0): set cell(
  fill: rgb("#f4f4f8"),
  stroke: (bottom: 1.5pt + rgb("#c8c8d4")),
)
// Row borders: thin bottom line on every row
#set table(row-gutter: 0pt)
// Note: stripe_background applied via table.cell.where(y: calc.rem(y, 2) == 1) in emitter

// Footnote styling
#show footnote.entry: it => {
  line(length: 33%, stroke: 0.5pt + rgb("#e2e2e8"))
  v(4pt)
  set text(size: 9pt)
  [#text(fill: rgb("#4a5dbd"))[#it.note.counter.display()] #it.note.body]
}

// ─── Document Content ─────────────────────────────────────
= My Document Title

This is a paragraph with *bold* and _italic_ text, plus `inline code`.

== Section Heading

#quote(block: true)[
  This is a blockquote with a themed left border.
]

// Alert box (GitHub-style)
#block(
  fill: rgb("#4a5dbd").lighten(92%),
  stroke: (left: 3pt + rgb("#4a5dbd")),
  radius: (right: 4pt),
  inset: 12pt,
  width: 100%,
)[
  *ℹ Note* \
  This is important information.
]

// Emoji renders as Unicode
This is a heart: ❤️ and a sparkle: ✨
```

### 10.3 Font Loading Strategy

Fonts are embedded in the binary at compile time via `rust-embed` with compression:

```rust
#[derive(RustEmbed)]
#[folder = "fonts/"]
struct BundledFonts;
```

At runtime, fonts are loaded in this order:
1. Bundled fonts (always available, decompressed on demand)
2. Theme-specified font sources (if `heading_source` etc. are set)
3. User-specified `--font-dir` paths
4. System fonts (via `typst-kit`'s `fonts()` or `fontdb` crate for platform font enumeration)

**Validation:** After loading, verify that the theme's required fonts (`heading`, `body`, `mono`)
exist in the font book. If not, emit a warning and attempt fallback chain.

---

## 11. Error & Warning System

### 11.1 Error Hierarchy

```rust
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SilkprintError {
    #[error("Failed to read input file: {path}")]
    #[diagnostic(help("Check that the file exists and is readable"))]
    InputRead {
        path: String,
        #[source] source: std::io::Error,
    },

    #[error("Invalid front matter in document")]
    #[diagnostic(code(silkprint::frontmatter))]
    FrontMatter {
        #[source_code] src: miette::NamedSource<String>,  // Arc-backed
        #[label("parse error here")] span: miette::SourceSpan,
    },

    #[error("Theme '{name}' not found")]
    #[diagnostic(
        help("Did you mean: {suggestions}\nRun `silkprint --list-themes` for all options"),
        code(silkprint::theme::not_found)
    )]
    ThemeNotFound { name: String, suggestions: String },  // Top 3 fuzzy matches

    #[error("Invalid theme configuration")]
    #[diagnostic(code(silkprint::theme::invalid))]
    ThemeInvalid {
        #[source_code] src: miette::NamedSource<String>,
        #[label("{message}")] span: miette::SourceSpan,
        message: String,
    },

    #[error("Theme inheritance cycle detected: {chain}")]
    #[diagnostic(code(silkprint::theme::cycle))]
    ThemeCycle { chain: String },

    #[error("Theme inheritance depth exceeded (max 5): {chain}")]
    #[diagnostic(code(silkprint::theme::depth))]
    ThemeInheritanceDepth { chain: String },

    #[error("Invalid paper size: {size}")]
    #[diagnostic(help("Valid sizes: a4, letter, a5, legal"))]
    InvalidPaperSize { size: String },

    #[error("Conflicting CLI options")]
    #[diagnostic(code(silkprint::cli::conflict))]
    ConflictingOptions { details: String },

    #[error("No fonts available for '{role}' — all fallbacks exhausted")]
    #[diagnostic(code(silkprint::font::exhausted))]
    FontExhausted { role: String, tried: Vec<String> },

    #[error("Typst compilation failed")]
    #[diagnostic(
        code(silkprint::render::typst),
        help("This is likely a SilkPrint bug — please report it with your input file")
    )]
    TypstCompilation { diagnostics: Vec<String> },

    #[error("Rendering failed")]
    #[diagnostic(code(silkprint::render), help("{hint}"))]
    RenderFailed { details: String, hint: String },

    #[error("Failed to write output: {path}")]
    OutputWrite {
        path: String,
        #[source] source: std::io::Error,
    },
}
```

### 11.2 Warnings (Non-Fatal)

```rust
pub enum SilkprintWarning {
    ImageNotFound { path: String },
    FontNotAvailable { name: String, fallback: String },
    UnknownLanguage { lang: String },
    UnrecognizedFrontMatter { field: String },
    ContrastRatio { element: String, ratio: f64, minimum: f64 },
    RemoteImageSkipped { url: String },
}
```

Warnings are collected during rendering and displayed after completion (in default and verbose
modes). They do NOT cause a non-zero exit code. `--quiet` suppresses them.

---

## 12. Testing Strategy

### 12.1 Unit Tests

- **Theme parsing** — Valid TOML → correct structs, invalid TOML → proper miette errors
- **Token resolution** — Color references resolve correctly, cycles detected, depth cap enforced
- **Front matter extraction** — YAML parsing, field mapping, precedence with CLI args
- **Markdown → Typst** — Each AST node type produces correct Typst markup (snapshot tested)
- **WCAG contrast** — Contrast ratio calculation, warning thresholds
- **Emoji resolution** — Shortcodes → Unicode codepoints

### 12.2 Integration Tests (via assert_cmd)

- **End-to-end rendering** — Markdown file → valid PDF, correct page count
- **Theme application** — Different themes produce different output
- **Error cases** — Missing files, invalid themes, malformed markdown
- **CLI flags** — `--check`, `--dump-typst`, `-o -`, `--list-themes`, `--open` (mock)
- **Exit codes** — 0 for success, 1 for errors
- **Warning output** — Warnings appear on stderr, don't affect exit code

### 12.3 Snapshot Tests (via insta)

- **Typst output** — Snapshot the generated Typst markup for each fixture file
- Ensures AST → Typst translation doesn't regress across changes

### 12.4 PDF Validation

- Parse output PDF with `lopdf` or similar to verify:
  - Valid PDF structure
  - Fonts are embedded
  - PDF metadata is set (title, author, producer)
  - Page count matches expected
- Visual regression testing deferred to v0.2 (rely on Typst output snapshots + manual review)

### 12.5 Fixture Files

```
tests/fixtures/
├── basic.md              # Paragraphs, headings, emphasis
├── code-blocks.md        # Multiple languages, inline code, long lines
├── tables.md             # GFM tables, alignment, dense data
├── full-features.md      # Every single supported feature (incl. highlight, underline, super/sub)
├── lists.md              # Nested, task lists, definition/description lists
├── alerts.md             # All GitHub-style alert types
├── math.md               # Inline + display math (Typst-native syntax)
├── emojis.md             # Shortcodes, Unicode emoji
├── footnotes.md          # Footnotes with back-references
├── frontmatter.md        # YAML front matter variations
├── images.md             # Relative paths, missing images, alt text
├── wikilinks.md          # [[page]], [[url|title]], edge cases
├── edge-cases.md         # Empty doc, single heading, huge table, long code lines
└── themes/
    ├── custom-test.toml
    └── invalid-test.toml
```

---

## 13. Performance Targets

| Operation | Target (warm) | Target (cold) | Measurement |
|-----------|--------------|---------------|-------------|
| CLI startup | < 30ms | < 50ms | Time to parse args |
| Font loading | < 5ms | < 200ms | Decompress + parse bundled fonts |
| Markdown parsing | < 10ms | < 10ms | 10KB document with comrak |
| Typst compilation | < 400ms | < 800ms | 12-page document |
| PDF export | < 100ms | < 100ms | Including font subsetting |
| **Total end-to-end** | **< 700ms** | **< 1.5s** | For a typical README |

Cold start includes first-time font decompression and text shaping initialization. Warm times
apply to subsequent invocations where OS-level caching helps.

---

## 14. Future Considerations (Post-v0.1)

### Near-term (v0.2–v0.3)

- **HTML output** — Same theme system, same markdown features, rendered as a single self-contained
  HTML page with embedded CSS, syntax highlighting (highlight.js/Prism), responsive layout, and
  theme-driven CSS custom properties. `--format html` flag. Every built-in theme generates a
  complete stylesheet. Print media queries for when users print the HTML page
- **PDF visual debugging** — Render PDF pages to images (PNG) for automated visual quality
  testing. Enables AI-assisted style review — agents can literally look at the output and judge
  typography, spacing, and color accuracy. `--render-pages <dir>` flag
- **stdin piping** — `cat doc.md | silkprint -o out.pdf`
- **Watch mode** — `silkprint watch doc.md` with live PDF refresh
- **Multi-file mode** — Concatenate multiple .md files into one PDF
- **Typst source mapping** — Map Typst errors back to Markdown line numbers
- **Header/footer customization** — Running headers with document/section title
- **Nix flake** — Official Nix package
- **Scoop/winget** — Windows package managers

### Medium-term

- **Book mode** — Chapter numbering, cross-references, bibliography
- **Custom Typst templates** — Advanced users inject raw Typst
- **Plugin system** — Custom AST transformers
- **EPUB output** — E-reader format
- **Slide mode** — Markdown → presentation PDF (like Marp)

### Long-term

- **Web UI (silkprint.md)** — Browser-based editor with theme picker, live preview, PDF download
- **API mode** — HTTP server for programmatic PDF generation
- **WASM build** — Run in the browser (enables web UI without server-side rendering)
