# SilkPrint Terminal Reader — Implementation Plan

Status: v3 (two Codex review passes folded in) · Target: new `silkprint read` subsystem · Author: Nova + Bliss

## 1. Goal

Add a terminal markdown reader to silkprint that renders gorgeously in the
terminal without leaving it — taking the same theme system that drives the PDF
and projecting it onto the terminal, with Nerd Font glyphs, true-color syntax,
inline images, and a TUI that bangs hard.

Two front-ends over one rendering core:

- **One-shot pretty-printer** — `silkprint read doc.md | less`, pipe-friendly
  ANSI with OSC 8 hyperlinks. Fills the throne left empty by abandoned `mdcat`.
- **Full TUI reader** — scrollable, searchable, collapsible outline, live theme
  switching, inline images. The hero experience.

The binary auto-selects: TTY → TUI, pipe or `--plain` → one-shot.

### Success criteria

- `silkprint read --plain tests/fixtures/*.md` emits correct styled ANSI that
  visually matches the PDF's color/structure decisions, with working OSC 8
  links and syntect-highlighted code.
- `silkprint read doc.md` in a TTY launches a TUI that scrolls, searches,
  shows a TOC sidebar, switches themes live, and renders images in Ghostty.
- The existing PDF pipeline is byte-for-byte unchanged (existing tests green).
- Graceful degradation verified at three tiers (color, glyphs, graphics) and
  inside tmux, at 80x24, and with `NO_COLOR`.

## 2. Architecture

### 2.1 The reuse seam — upstream sharing, NOT a shared emitter trait

The PDF pipeline today is: `markdown → comrak AST → Typst markup → PDF`. The
*primary* Typst-coupled stage is the AST walker `emit_node()` in
`src/render/markdown.rs:416-779`, but (review caught this) Typst-specific
behavior is also threaded through footnote collection, field-stack paragraphs,
list/task rendering, description lists, inline-HTML conversion
(`src/render/html.rs`), Mermaid collection, and the preamble show-rules
(`src/render/preamble.rs`). The terminal walker re-implements those node
behaviors for its own layout model rather than inheriting them — so "reuse"
means the *upstream* stages below, not the emit logic. Parity fixtures (same
markdown → both targets) guard against the two walkers drifting in coverage.

| Stage | Module | Reuse |
| --- | --- | --- |
| comrak parse (+ runtime extensions) | `src/render/markdown.rs:14-161` | as-is |
| front matter extraction | `src/render/frontmatter.rs` | as-is |
| theme load + 2-level color resolution | `src/theme/mod.rs` | as-is (hands back hex) |
| resolved theme tokens | `src/theme/tokens.rs` | as-is |
| syntax token color resolution | `src/theme/syntax.rs` | as-is |
| generated tmTheme XML | `src/theme/tmtheme.rs` | as-is (feed to syntect) |
| image *reference* discovery | `src/render/image.rs` (needs split) | shared `collect_image_refs()`; per-target prepare (§7) |

**Decision (open for review): a separate terminal walker, not a `NodeEmitter`
trait.** The earlier research suggested lifting `emit_node()` into a trait with
`TypstEmitter` + `TerminalEmitter` impls. I'm recommending against it:

- The emit *bodies* share almost nothing. Typst emits markup strings (`*bold*`);
  the terminal builds a width-aware layout model with styled spans, link
  regions, and image placements. A 25-method trait would be a shared dispatch
  table, not shared logic.
- comrak's AST node enum already enforces exhaustive node coverage via `match`,
  so the trait buys us no completeness guarantee we don't already have.
- Extracting the trait means refactoring the *working PDF path*, which is pure
  risk for a feature that lives beside it.

Instead: `render_to_terminal()` is a sibling pipeline that reuses the upstream
stages (parse, theme, front matter, syntax-color resolution; image after the
split in §7) and adds its own `walk.rs`. Where a node behavior is genuinely
identical (list numbering, footnote ordering, alert-kind classification),
extract a small shared semantic-lowering helper used by both walkers — but don't
pre-abstract the whole emit surface into a trait. Parity fixtures keep the two
walkers honest about construct coverage. (Codex concurred: rejecting the giant
trait is the right instinct; the work is defining this lowering/parity strategy.)

### 2.2 The RenderedDoc intermediate model

The terminal walker produces a **width-independent** document model, rendered to
ANSI or ratatui at draw time (so resize re-layouts correctly):

```
RenderedDoc {                  // width-independent
    blocks: Vec<Block>,        // Heading, Paragraph, CodeBlock, Quote, List,
                               //   Table, Alert, Image, Rule, Math, DescList…
    outline: Vec<OutlineItem>, // flattened heading tree → TOC sidebar
    links: HashMap<LinkId, LinkTarget>,  // URL / internal anchor, by id
}
// Link *targets* live here; Span.link: Option<LinkId> ties text to a target.

LaidOutDoc {                   // produced by layout.rs per (width, caps, theme)
    rows: Vec<Row>,                       // wrapped, ready to draw
    hit_regions: Vec<(Rect, LinkId)>,     // screen coords for OSC 8 / mouse / Enter
    image_placements: Vec<(Rect, ImageId)>,
}
```

Spans carry semantic style *roles* (e.g. `Heading(2)`, `CodeKeyword`,
`Emphasis`) plus an optional `LinkId` — **not** resolved colors (review). The
`ContentStyleResolver` (§2.3) applies concrete colors at layout/render time, so a
live theme switch (`t`) re-resolves roles → colors without re-walking the AST.
Logical content is stored unwrapped; wrapping,
table-width balancing, list indentation, and — review caught this —
screen-coordinate hit regions are computed by `layout.rs` into a `LaidOutDoc`
when width and capabilities are known, cached per (width, caps, theme). Keeping
hit regions out of `RenderedDoc` is what lets the TUI re-flow on `SIGWINCH`
without re-parsing, since link coordinates are inherently width-dependent.

### 2.3 Theming split — opaline for chrome, silkprint theme for content

This is the answer to "do we need opaline here": **yes, for the TUI chrome; no,
for the document content.** Two theme layers, one shared palette.

- **Document content** (headings, body, code, quotes, tables, alerts) is themed
  by silkprint's existing 24-section `ResolvedTheme` via a `ContentStyleResolver`
  (T0.5) that mirrors the same token + Typst-preamble-fallback resolution the PDF
  uses. Caveat caught in review: a couple of content colors are currently
  hardcoded in the Typst emitter (alert `#4a5dbd`, image border `#e2e2e8`) rather
  than theme-driven, so "matches the PDF" only holds once those are lifted into
  the theme. Decided (§8.3): T0.5 lifts those two into the theme schema, keeping
  their current values as default fallbacks so existing PDFs render identically,
  giving terminal and PDF one shared source of truth.
- **TUI chrome** (panel borders, status bar, scrollbar, search box, tab bar,
  help overlay, theme picker) is themed by **opaline** — its semantic tokens
  (`bg.*`, `border.*`, `accent.*`), gradients (`gradient_bar` for scroll
  progress), and the drop-in `ThemeSelector` widget give us a banging shell for
  free.

**The bridge:** map the active silkprint document theme to an opaline chrome
theme. The families overlap heavily (silkcircuit, nord, dracula, catppuccin,
gruvbox, tokyo-night, rose-pine all exist in both) — match by name when
possible; otherwise derive a chrome theme from the document palette via
opaline's `ThemeBuilder` + `darken()`/`lighten()`. Result: chrome and content
share one coherent palette, dark/light awareness included.

Note: opaline can also *generate syntect themes*, but for content code blocks we
deliberately feed syntect silkprint's existing tmTheme XML so code colors match
the PDF. Opaline's syntect adapter is not on the content path.

### 2.4 Module layout

```
src/render/terminal/
  mod.rs        render_to_terminal() pipeline; ties upstream → walk → ansi
  model.rs      RenderedDoc, Block, Span, Style, OutlineItem, LinkRegion
  walk.rs       comrak AST → RenderedDoc (the terminal walker)
  highlight.rs  syntect: tmTheme XML → highlighted code spans (cached SyntaxSet)
  layout.rs     width-aware wrapping, table balancing, list/quote indent
  glyphs.rs     GlyphSet (NerdFont | Unicode | Ascii) + tier selection
  ansi.rs       RenderedDoc → ANSI + OSC 8 (one-shot mode)
  caps.rs       capability detection: color tier, graphics protocol, glyph tier,
                NO_COLOR, COLORTERM, tmux, TTY
src/tui/
  mod.rs        app entry: terminal setup/teardown, panic-safe restore, event loop
  app.rs        App state: scroll, focus, search, active theme, open docs/tabs
  view.rs       ratatui draw: doc viewport (hero) + outline sidebar + status bar
  chrome.rs     opaline integration: doc theme → opaline chrome theme bridge
  image.rs      ratatui-image: viewport-aware placement, aspect ratio, tmux pass
  keys.rs       vim-style keybinding dispatch
  search.rs     content search, match highlight, jump-to-match
  theme_picker.rs  opaline ThemeSelector wiring + live re-render
```

Public API gains `silkprint::render_to_terminal(input, path, options:
&TerminalRenderOptions) -> Result<(String, Vec<SilkprintWarning>),
SilkprintError>` as a sibling to `render()` / `render_to_typst()`, matching their
`Result`-returning shape (review). `TerminalRenderOptions` carries
terminal-specific knobs (glyph tier, color tier, image mode, width override),
distinct from the PDF `RenderOptions`.

### 2.5 CLI shape

Introduce an optional clap subcommand while preserving today's bare behavior:

- `silkprint doc.md -o out.pdf` → PDF (unchanged, default when no subcommand)
- `silkprint read [FILE]` → TTY launches TUI; pipe/redirect → one-shot ANSI
- `silkprint read --plain FILE` → force one-shot even in a TTY
- `silkprint read --glyphs nerdfont|unicode|ascii FILE` → override glyph tier
- `silkprint read --no-images FILE` → disable graphics protocols
- `--theme`, `--no-toc`, `--color` carry over

clap derive supports an `Option<Command>` subcommand enum; absent subcommand
routes to the existing PDF path for back-compat. Two edges to handle explicitly
(review): shared flags (`--theme`, `--color`) must be `global = true` or factored
into a `#[command(flatten)]` arg struct so they work both before and after the
subcommand; and `read` becomes a reserved first token, so a file literally named
`read` must be passed as `./read`. Add parser tests for `read`, `./read`, and
shared flags on either side of the subcommand.

The TTY-vs-TUI auto-routing above is the *final* behavior. Wave 0 ships `read`
as **always one-shot** (no TUI yet); auto-routing turns on in Wave 1 (T1.7) once
the TUI shell exists.

## 3. Capability detection & degradation (`caps.rs`)

Three independent tiers, each detected and each with a defined fallback. Golden
rule from the design system: the reader must be *usable* at the lowest tier;
higher tiers *enhance*, never *create* the experience.

| Axis | Tiers (best → fallback) | Detection |
| --- | --- | --- |
| Color | truecolor → 256 → 16 ANSI → none | `COLORTERM`, `TERM`, `NO_COLOR` |
| Glyphs | NerdFont → Unicode → ASCII | `--glyphs`, `SILKPRINT_GLYPHS`, default NerdFont |
| Graphics | Kitty → iTerm2 → Sixel → half-block → OSC 8 link | env + terminal query (via ratatui-image) |

tmux: detect `$TMUX`; wrap graphics in passthrough and use Kitty U+10EEEE
Unicode-placeholder placement so the multiplexer moves images with the text.

## 4. Nerd Font glyph map (`glyphs.rs`)

Default to Nerd Font (Bliss wants the devicons), with Unicode then ASCII
fallbacks. Indicative mapping (final codepoints picked against nf-md / nf-dev):

| Use | NerdFont | Unicode | ASCII |
| --- | --- | --- | --- |
| note alert |  (info) | ℹ | [NOTE] |
| tip alert |  (bulb) | ✎ | [TIP] |
| important |  (flame) | ‼ | [IMPORTANT] |
| warning |  (alert) | ⚠ | [WARN] |
| caution |  (skull) | ⚡ | [CAUTION] |
| code fence lang | devicon ( rust,  py,  js) | · | lang label |
| link |  | ↪ | -> |
| TOC / heading | ›  bullets | › | > |
| task done / todo |  /  | ☑ / ☐ | [x] / [ ] |
| git branch (status bar) |  | ⎇ | branch |
| theme picker |  (palette) | ◐ | theme |

Alert kinds map onto silkprint's existing `AlertTokens` (note/tip/important/
warning/caution colors + `show_icon`), so the colored callout boxes reuse theme
colors and only swap the glyph by tier. Caveat: Nerd Font glyphs vary in cell
width — measure with `unicode-width` and verify in real terminals (anti-pattern
#5).

## 5. TUI design (the "bang hard" layer)

Layout paradigm: a reader-tuned **persistent multi-panel** — collapsible outline
sidebar (left), the rendered document as the hero (center), status bar (bottom),
optional tab bar (top) for multiple open docs.

```
┌─ ◈ silkprint · doc.md ─────────────────────────── silkcircuit-neon ─┐
│ OUTLINE        │  Document Title  (rasterized heading, P2)           │
│ › Intro        │                                                     │
│ › Setup        │  Body text, wrapped + justified per theme…          │
│   · Install    │                                                     │
│ › Usage        │   tip  themed callout box with nerd glyph          │
│                │                                                     │
│                │   rust  fn main() { … }   ← syntect, PDF-matched   │
├────────────────┴─────────────────────────────────────────────────────┤
│ §3/12 ▕▆▆▆▆▆░░░░░░░░░░▏ 24% │ /search  ?help  t theme  o outline  q quit │
└────────────────────────────────────────────────────────────────────────┘
```

"Bang hard" details:

- opaline `gradient_bar` for the scroll-progress meter in the status bar
  (SilkCircuit gradient), gradient title/heading underlines via `gradient_spans`.
- opaline `ThemeSelector` on `t` — live preview re-renders both chrome and
  document content as you arrow through 39 themes.
- Synchronized output (`CSI ?2026 h/l`) wrapping each frame on top of ratatui's
  double buffering; differential redraws; 30 FPS cap; braille spinner for image
  decode / large-doc parse.
- Background-layer depth (`bg.base → bg.surface → bg.overlay`) for panels rather
  than heavy borders; dimmed unfocused panels.
- OSC 8 hyperlinks on every link; `Enter` follows internal anchors via the
  outline.

Keybindings (vim lingua franca, contextual footer + `?` overlay):

| Key | Action | Key | Action |
| --- | --- | --- | --- |
| `j`/`k` | scroll line | `gg`/`G` | top / bottom |
| `Ctrl-d`/`Ctrl-u` | half page | `/` `n` `N` | search / next / prev |
| `o` | toggle outline | `t` | theme picker |
| `Tab` | cycle focus | `Enter` | follow link / jump |
| `?` | help overlay | `q` | quit |

Never bind `Ctrl-C`/`Ctrl-Z`/`Ctrl-\`. Panic-safe terminal restore. Minimum
80x24 with a resize-message gate below it.

## 6. Phased plan

### Wave 0 — Foundation & one-shot (the keystone)

- [ ] **T0.0** Dependency + feature-gate spike: add a `terminal` feature pulled
      in via `cli`; declare ratatui / ratatui-image (`default-features = false`,
      explicit features) / syntect / opaline / notify / crossterm under the
      existing `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` block so
      they physically can't reach wasm. → verify: `cargo tree -d -e features`
      shows a single ratatui 0.30 / crossterm 0.29; `cargo check -p silkprint-wasm
      --target wasm32-unknown-unknown` passes (terminal deps absent). Unblocks the
      rest of the wave.
- [ ] **T0.1** Split `image.rs`: shared `collect_image_refs()` +
      `prepare_pdf_images()` (existing behavior, unchanged) +
      `prepare_terminal_images()` (local paths / bytes / dimensions).
      → verify: PDF image fixtures byte-identical; terminal prepare returns dims.
- [ ] **T0.2** `caps.rs`: capability detection (color/glyph/graphics/tmux/TTY).
      → verify: unit tests over faked env vars.
- [ ] **T0.3** `model.rs`: RenderedDoc / LaidOutDoc / Block / Span / Outline /
      LinkId. → verify: `cargo check`, doc-tests.
- [ ] **T0.4** `glyphs.rs`: GlyphSet + tier selection. → verify: unit tests per tier.
- [ ] **T0.5** `ContentStyleResolver`: resolve content styles from theme tokens
      *plus* the Typst-preamble fallback defaults so terminal matches PDF; handle
      the two hardcoded content colors (alert `#4a5dbd`, image border `#e2e2e8`)
      per §8.3. → verify: resolver output diffed against preamble defaults, 3 themes.
- [ ] **T0.6** `highlight.rs`: syntect + existing tmTheme → spans; cached
      SyntaxSet/ThemeSet. → verify: highlighted rust block matches the PDF tmTheme.
- [ ] **T0.7** `walk.rs`: comrak AST → RenderedDoc; cover every node type; shared
      semantic-lowering helpers where behavior is identical to the Typst walker.
      → verify: per-node unit tests; exhaustive `match`; parity fixtures.
- [ ] **T0.8** `layout.rs`: width-aware wrapping + table balancing + indent +
      `LaidOutDoc` hit regions. → verify: wrap/table tests at widths 40/80/120.
- [ ] **T0.9** `ansi.rs`: RenderedDoc → ANSI + OSC 8; image → half-block/link.
      → verify: golden ANSI snapshots on `tests/fixtures/*.md`.
- [ ] **T0.10** CLI `read` subcommand (**one-shot only this wave — no TTY
      auto-routing**), `--plain`, `global = true` shared flags,
      `render_to_terminal()` public API. → verify: `silkprint read basic.md`
      emits ANSI; parser tests for `read` / `./read` / flag placement; PDF tests green.

Ordering: T0.0 first (unblocks all). Then T0.1–T0.4 (parallel) → T0.5/T0.6/T0.7
→ T0.8 → T0.9/T0.10. Ships a usable, always-one-shot renderer (no broken TTY
path) and takes mdcat's empty throne.

### Wave 1 — TUI shell that bangs

- [ ] **T1.1** `src/tui/mod.rs` + `app.rs`: ratatui+crossterm app, alt screen,
      raw mode, panic-safe restore, resize handling, clean SIGINT exit.
      → verify: manual run; resize at 80x24/120x40; Ctrl-C restores terminal.
- [ ] **T1.2** `view.rs`: doc viewport + collapsible outline + status bar layout.
      → verify: screenshot via ghostty automation.
- [ ] **T1.3** `chrome.rs`: opaline bridge (doc theme → opaline chrome theme);
      gradient scroll meter. → verify: chrome palette matches doc theme across 3 themes.
- [ ] **T1.4** `keys.rs`: vim keybindings + contextual footer + `?` overlay.
      → verify: manual; every footer key works.
- [ ] **T1.5** `search.rs`: live search, match highlight, jump. → verify: manual + unit on matcher.
- [ ] **T1.6** `theme_picker.rs`: opaline ThemeSelector + live re-render.
      → verify: manual, switching themes re-renders content+chrome.
- [ ] **T1.7** Enable TTY auto-routing (deferred from Wave 0): `silkprint read
      FILE` → TUI in a TTY, one-shot when piped or `--plain`. Safe now that the
      shell (T1.1) exists. → verify: `silkprint read x.md` opens the TUI;
      `silkprint read x.md | cat` stays one-shot.

### Wave 2 — Images & rasterized typography (the wow)

- [ ] **T2.1** `image.rs`: ratatui-image placement, aspect ratio, viewport
      offset math, tmux passthrough + Unicode placeholders.
      → verify: manual in Ghostty (Kitty), iTerm2, and a half-block fallback term; inside tmux.
- [ ] **T2.2** Rasterized headings: render H1/H2 via embedded fonts (Inter) to
      PNG, place via Kitty; Text Sizing Protocol where available; bold+scaled
      fallback. → verify: manual; headings visually match PDF fonts.
- [ ] **T2.3** Mermaid + math inline: existing code produces Mermaid *SVG bytes*
      for Typst virtual files and emits math as Typst *markup* — neither yields a
      raster (review correction). Add explicit rasterization (Mermaid SVG → PNG
      via resvg; math → compile a minimal Typst snippet → image), or degrade to a
      fenced-text fallback when graphics are unavailable. → verify: manual on a
      mermaid/math fixture in a Kitty-capable terminal + the text fallback.
- [ ] **T2.4** Live reload: `notify` watch + re-render on change.
      → verify: edit file, TUI updates.

### Wave 3 — Polish & ship

- [ ] **T3.1** File discovery / multi-tab / session restore (markdown-reader parity, optional).
- [ ] **T3.2** Light/dark auto-detect, NO_COLOR, monochrome pass.
- [ ] **T3.3** Perf: viewport-only render, syntect cache, 30fps, sync output.
- [ ] **T3.4** Docs, README section, demo gif.

## 7. Crates & integration risks

| Crate | Role | Notes |
| --- | --- | --- |
| comrak 0.50 | parse | already a dep; reuse |
| syntect 5 | code highlighting | new; feed existing tmTheme XML |
| opaline 0.4 | TUI chrome theme + ThemeSelector | features: `widgets`, `gradients`, `syntect`? (no — content uses silkprint tmTheme) |
| ratatui 0.30 | TUI framework | must match opaline's ratatui 0.30 |
| crossterm 0.29 | terminal backend | match opaline |
| ratatui-image 11.0.2 | image protocols | normal-dep `ratatui ^0.30`; its `crossterm ^0.29` is dev-only, so opaline owns crossterm |
| image | decode | transitive via ratatui-image |
| notify | live reload (P2) | watch source file |

All of the above except comrak live behind the `terminal` Cargo feature (§7.5),
off for `default-features = false` consumers so `silkprint-wasm` stays clean.

**Integration risks (call out to reviewer):**

1. **ratatui version alignment — looks clean, prove it with a spike.** opaline
   0.4 pins ratatui 0.30 / crossterm 0.29; ratatui-image 11.0.2 normal-depends on
   `ratatui ^0.30` (its `crossterm ^0.29` is dev-only, so it doesn't constrain us
   — opaline owns the crossterm normal dep). No conflict is visible, so opaline's
   `widgets` feature (the ThemeSelector) is on the table. Two catches from the
   second review: ratatui-image's *default* `crossterm` feature forwards to
   `ratatui/crossterm` in the normal graph, so pin it `default-features = false`
   with explicit features; and "aligned in metadata" is not "resolves to one
   copy". T0.0 runs `cargo tree -d -e features` to prove a single ratatui before
   we build on it.
2. **Image-in-scroll placement** is the hardest piece. Placing Kitty/iTerm2
   images at the right cell offset inside a scrolling ratatui viewport, and
   moving/clearing them on scroll, is fiddly. Study markdown-reader and mdfried
   (both solved it). tmux passthrough compounds it.
3. **Nerd Font cell width** ambiguity (single/double/zero-width) can break
   alignment. Measure with `unicode-width`, test in 3+ terminals.
4. **"Perfect" is grid-bounded** on terminals without a graphics protocol —
   half-blocks + bold/scaled text, not pixel-perfect. Acceptable if degradation
   is graceful.
5. **WASM blast radius (review catch).** `silkprint-wasm` depends on `silkprint`
   with `default-features = false`, so gating must be *package-correct*, not the
   imaginary "on for the binary, off for the library" kind — Cargo features are
   package-wide. Approach, grounded in the current manifest (which already has a
   `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` block): declare the
   terminal/TUI deps there so they physically can't reach wasm, gate them behind
   a `terminal` feature, and pull `terminal` in through `cli` (the binary already
   sets `required-features = ["cli"]`). wasm/library consumers on
   `default-features = false` get none of it. `cargo tree -d -e features` +
   `cargo check -p silkprint-wasm --target wasm32-unknown-unknown` are standing
   gates (T0.0).

## 8. Open decisions for review

1. Separate terminal walker — *decided* (Codex concurred rejecting the giant
   trait). Open refinement: how much shared semantic-lowering to extract. Bias:
   only proven-identical helpers, guarded by parity fixtures (§2.1).
2. opaline `widgets` (ThemeSelector) vs token-only adapter — leaning `widgets`
   now the stack looks aligned; T0.0 confirms (§7.1).
3. **PDF content hardcodes** — *decided*: lift alert `#4a5dbd` and image border
   `#e2e2e8` into the theme schema, with those exact values as default fallbacks
   so existing PDFs are unchanged and terminal + PDF share one source of truth.
   Done as part of T0.5. (was open; resolved per review)
4. `read` subcommand vs `--read`/`--term` flag on the existing command (§2.5).
5. Scope of Wave 3 — file-tree/multi-tab in v1 or deferred?
6. Rasterized headings (T2.2) worth the complexity vs Text Sizing Protocol alone
   where supported?
