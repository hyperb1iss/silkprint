//! Scrollable TUI reader built on the same `RenderedDoc` as the one-shot path.
//!
//! Content is rendered through [`super::ansi`] at the viewport width and parsed
//! into ratatui text via `ansi-to-tui`, so the TUI and the pipe-friendly output
//! stay pixel-identical. opaline themes the chrome (borders, status bar,
//! outline, popups); the document content keeps the silkprint theme.

mod chrome;
mod diagrams;
mod images;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use ansi_to_tui::IntoText;
use notify::Watcher;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block as WBlock, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui_image::StatefulImage;
use ratatui_image::picker::Picker;

use crate::ThemeSource;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use self::chrome::Chrome;
use self::images::{ImageStore, Placement};
use super::caps::{Capabilities, ColorTier, GlyphTier, GraphicsProtocol};
use super::glyphs::Glyphs;
use super::model::{Block, RenderedDoc, Rgb};
use super::style::ContentStyleResolver;

const OUTLINE_WIDTH: u16 = 30;

/// Upper bound on the rows a single image/diagram band may reserve. Bands are
/// normally sized to the image's natural height and scrolled through; this only
/// guards against a pathologically tall input flooding the content flow.
const MAX_BAND_ROWS: u16 = 400;
const IMAGE_PREFETCH_ROWS: u16 = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Content,
    Outline,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Search,
}

/// A content region that renders as an image: an inline image or a mermaid
/// diagram.
enum BandSpec {
    Image(String),
    Mermaid { source: String, bg: Rgb },
}

/// Launch the interactive reader. Sets up and tears down the terminal
/// (panic-safe via ratatui's restore hook) and runs the event loop.
pub fn run(
    body: &str,
    theme: ResolvedTheme,
    theme_name: &str,
    glyph_override: Option<GlyphTier>,
    images: bool,
    base_dir: Option<PathBuf>,
    watch_path: Option<PathBuf>,
) -> io::Result<()> {
    // Query the terminal's graphics protocol + font size before entering the
    // alternate screen. `None` (or `--no-images`) falls back to text-only.
    let picker = images.then(Picker::from_query_stdio).and_then(Result::ok);
    let mut app = App::new(
        body,
        theme,
        theme_name,
        glyph_override,
        picker,
        base_dir,
        watch_path,
    );
    let mut terminal = ratatui::init();
    let result = app.run_loop(&mut terminal);
    ratatui::restore();
    result
}

#[allow(clippy::struct_excessive_bools)]
struct App {
    doc: RenderedDoc,
    theme: ResolvedTheme,
    glyphs: Glyphs,

    theme_names: Vec<String>,
    theme_idx: usize,
    chrome: Chrome,
    title: String,

    content: Text<'static>,
    content_bg: Color,
    content_fg: Color,
    /// Per-block `(start_line, line_count)` from the renderer (pre-band-insert).
    block_spans: Vec<(usize, usize)>,
    /// Per-block start line adjusted for inserted graphical bands (outline jumps).
    block_jump: Vec<usize>,
    rendered_width: u16,
    theme_dirty: bool,

    scroll: u16,
    viewport_h: u16,
    outline_visible: bool,
    focus: Focus,
    outline_state: ListState,

    mode: Mode,
    search_query: String,
    matches: Vec<usize>,
    match_idx: usize,

    show_help: bool,
    show_picker: bool,
    picker_state: ListState,
    picker_saved: usize,

    pending_g: bool,
    quit: bool,

    images: ImageStore,
    image_placements: Vec<Placement>,
    path: Option<PathBuf>,
}

impl App {
    fn new(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
    ) -> Self {
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, body);
        let mut warnings = WarningCollector::new();
        crate::render::markdown::check_content(root, &mut warnings);
        let doc = super::walk::walk(root, &mut warnings);

        let theme_names: Vec<String> = crate::theme::builtin::list_themes()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        let theme_idx = theme_names
            .iter()
            .position(|n| n == theme_name)
            .unwrap_or(0);

        let title =
            super::layout::sanitize(doc.title.as_deref().unwrap_or("silkprint")).into_owned();
        let glyphs = Glyphs::new(glyph_override.unwrap_or(GlyphTier::NerdFont));

        let mut outline_state = ListState::default();
        if !doc.outline.is_empty() {
            outline_state.select(Some(0));
        }
        let saved = super::config::load();
        let outline_visible = saved.outline.unwrap_or(doc.outline.len() > 1);

        Self {
            doc,
            theme,
            glyphs,
            chrome: Chrome::for_theme(theme_name),
            theme_names,
            theme_idx,
            title,
            content: Text::default(),
            content_bg: Color::Reset,
            content_fg: Color::Reset,
            block_spans: Vec::new(),
            block_jump: Vec::new(),
            rendered_width: 0,
            theme_dirty: true,
            scroll: 0,
            viewport_h: 1,
            outline_visible,
            focus: Focus::Content,
            outline_state,
            mode: Mode::Normal,
            search_query: String::new(),
            matches: Vec::new(),
            match_idx: 0,
            show_help: false,
            show_picker: false,
            picker_state: ListState::default(),
            picker_saved: 0,
            pending_g: false,
            quit: false,
            images: ImageStore::new(picker, base_dir),
            image_placements: Vec::new(),
            path: watch_path,
        }
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
        // Watch the input file's directory for changes (robust to editors that
        // save via atomic rename). `_watcher` must stay alive for the loop.
        let (tx, rx) = std::sync::mpsc::channel();
        let _watcher = self.path.clone().and_then(|path| {
            let mut watcher =
                notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    if res.is_ok() {
                        let _ = tx.send(());
                    }
                })
                .ok()?;
            let target = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(path.as_path());
            watcher
                .watch(target, notify::RecursiveMode::NonRecursive)
                .ok()?;
            Some(watcher)
        });

        // Redraw only when something changes — input, resize, or a file edit —
        // so an idle reader doesn't repaint the whole document on every tick.
        let mut needs_redraw = true;
        while !self.quit {
            if self.images.poll_ready() {
                needs_redraw = true;
            }
            if needs_redraw {
                terminal.draw(|frame| self.draw(frame))?;
                needs_redraw = false;
            }
            let poll_timeout = if self.images.has_pending() {
                Duration::from_millis(16)
            } else {
                Duration::from_millis(200)
            };
            if event::poll(poll_timeout)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.on_key(key.code, key.modifiers);
                        needs_redraw = true;
                    }
                    Event::Resize(..) => needs_redraw = true,
                    _ => {}
                }
            }
            if rx.try_iter().count() > 0 {
                self.reload();
                needs_redraw = true;
            }
        }
        self.save_config();
        Ok(())
    }

    /// Re-read and re-walk the watched file (live reload).
    fn reload(&mut self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        let Ok(body) = std::fs::read_to_string(&path) else {
            return;
        };
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, &body);
        let mut warnings = WarningCollector::new();
        crate::render::markdown::check_content(root, &mut warnings);
        self.doc = super::walk::walk(root, &mut warnings);
        self.title =
            super::layout::sanitize(self.doc.title.as_deref().unwrap_or("silkprint")).into_owned();
        self.images.clear_cache();
        self.image_placements.clear();
        match self.outline_state.selected() {
            Some(sel) if sel >= self.doc.outline.len() => {
                self.outline_state
                    .select((!self.doc.outline.is_empty()).then_some(0));
            }
            None if !self.doc.outline.is_empty() => self.outline_state.select(Some(0)),
            _ => {}
        }
        if self.doc.outline.is_empty() {
            self.focus = Focus::Content; // outline may have vanished
        }
        self.theme_dirty = true; // force ensure_content to re-render
    }

    fn save_config(&self) {
        let glyphs = match self.glyphs.tier() {
            GlyphTier::NerdFont => "nerdfont",
            GlyphTier::Unicode => "unicode",
            GlyphTier::Ascii => "ascii",
        };
        super::config::save(&super::config::ReaderConfig {
            theme: self.theme_names.get(self.theme_idx).cloned(),
            outline: Some(self.outline_visible),
            glyphs: Some(glyphs.to_string()),
        });
    }

    // ─── Content rendering ───────────────────────────────────────

    fn ensure_content(&mut self, width: u16) {
        if !self.theme_dirty && width == self.rendered_width {
            return;
        }
        let caps = Capabilities {
            color: ColorTier::TrueColor,
            glyphs: self.glyphs.tier(),
            graphics: GraphicsProtocol::None,
            width,
            height: 0,
            is_tty: false, // suppress OSC 8 — ratatui owns the screen
            in_tmux: false,
        };
        let (ansi, offsets) =
            super::ansi::render_with_offsets(&self.doc, &self.theme, &caps, self.glyphs);
        self.content = ansi.into_text().unwrap_or_else(|_| Text::raw(ansi.clone()));

        let resolver = ContentStyleResolver::new(&self.theme);
        self.content_bg = resolver
            .page_background()
            .map_or(Color::Reset, rgb_to_color);
        self.content_fg = resolver.body_color().map_or(Color::Reset, rgb_to_color);
        // ansi-to-tui leaves text spans with a Reset background, which paints as
        // the terminal's own default — black on a dark profile even for a light
        // document theme. Pin every unset span background to the page color so
        // content sits on its theme's surface, not the terminal's.
        for line in &mut self.content.lines {
            for span in &mut line.spans {
                if span.style.bg.is_none() || span.style.bg == Some(Color::Reset) {
                    span.style.bg = Some(self.content_bg);
                }
            }
        }

        self.block_spans = offsets;
        self.block_jump = self.block_spans.iter().map(|(start, _)| *start).collect();
        self.image_placements.clear();
        if self.images.enabled() {
            self.reserve_bands(width.saturating_sub(2));
        }
        self.rendered_width = width;
        self.theme_dirty = false;
        self.clamp_scroll();
    }

    /// Replace each graphical block — inline images and mermaid diagrams — with
    /// a blank band sized to the image it will draw, record where to draw it,
    /// and keep outline jump offsets in sync with the shift. Replacing (rather
    /// than covering) the source means the mermaid text / image alt never peeks
    /// out below an image that is shorter than its source block.
    fn reserve_bands(&mut self, content_width: u16) {
        let bands: Vec<(usize, BandSpec)> = {
            let resolver = ContentStyleResolver::new(&self.theme);
            let bg = resolver.page_background().unwrap_or(Rgb(0, 0, 0));
            self.doc
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(i, block)| match block {
                    Block::Image { src, .. } => Some((i, BandSpec::Image(src.clone()))),
                    Block::CodeBlock {
                        lang: Some(lang),
                        lines,
                    } if lang == "mermaid" => {
                        let source = lines
                            .iter()
                            .map(|spans| spans.iter().map(|s| s.text.as_str()).collect::<String>())
                            .collect::<Vec<_>>()
                            .join("\n");
                        Some((i, BandSpec::Mermaid { source, bg }))
                    }
                    _ => None,
                })
                .collect()
        };

        let theme = self.theme.clone();
        let cell = self.images.cell();
        // Size bands to the image's natural height (bounded only against
        // pathological inputs). Tall diagrams get a tall band and are scrolled
        // through — the draw path crops to whatever slice is on screen.
        let max_rows = MAX_BAND_ROWS;
        let mut delta: isize = 0;
        for (block_index, spec) in bands {
            let (orig_start, block_height) = self.block_spans[block_index];
            if block_height == 0 {
                continue;
            }
            let (key, dims) = match spec {
                BandSpec::Image(src) => {
                    let dims = self.images.get(&src).map(|l| (l.width, l.height));
                    (src, dims)
                }
                BandSpec::Mermaid { source, bg } => {
                    let key = format!("\u{0}mermaid:{source}");
                    let dims = self
                        .images
                        .ensure_generated(&key, || diagrams::mermaid_image(&source, &theme, bg))
                        .map(|l| (l.width, l.height));
                    (key, dims)
                }
            };
            let Some((w, h)) = dims else {
                continue;
            };
            let img_rows = images::reserved_rows(w, h, content_width, cell, max_rows);
            // Replace the source block's lines with exactly `img_rows` blank
            // lines so the image fills the band with no source text peeking out.
            let base = isize::try_from(orig_start).unwrap_or(0) + delta;
            let start = usize::try_from(base)
                .unwrap_or(0)
                .min(self.content.lines.len());
            let end = (start + block_height).min(self.content.lines.len());
            let band = usize::from(img_rows);
            self.content
                .lines
                .splice(start..end, std::iter::repeat_with(Line::default).take(band));
            let shift =
                isize::try_from(band).unwrap_or(0) - isize::try_from(end - start).unwrap_or(0);
            delta += shift;
            for jump in self.block_jump.iter_mut().skip(block_index + 1) {
                *jump = usize::try_from(isize::try_from(*jump).unwrap_or(0) + shift).unwrap_or(0);
            }
            self.image_placements.push(Placement {
                src: key,
                line: u16::try_from(start).unwrap_or(u16::MAX),
                rows: img_rows,
            });
        }
    }

    fn content_len(&self) -> u16 {
        u16::try_from(self.content.lines.len()).unwrap_or(u16::MAX)
    }

    fn max_scroll(&self) -> u16 {
        self.content_len().saturating_sub(self.viewport_h)
    }

    fn clamp_scroll(&mut self) {
        self.set_scroll(self.scroll);
    }

    fn apply_theme(&mut self, idx: usize) {
        if let Some(name) = self.theme_names.get(idx) {
            self.theme = load_theme_or_default(name);
            self.chrome = Chrome::for_theme(name);
            self.theme_idx = idx;
            self.theme_dirty = true;
            // Generated rasters bake in the old theme's colors and must be rebuilt.
            self.images.clear_generated();
        }
    }

    // ─── Input ───────────────────────────────────────────────────

    fn on_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.show_picker {
            self.picker_key(code);
            return;
        }
        if self.mode == Mode::Search {
            self.search_key(code);
            return;
        }
        self.normal_key(code, mods);
    }

    fn normal_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        let half = self.viewport_h / 2;
        let page = self.viewport_h.saturating_sub(2).max(1);
        let was_g = self.pending_g;
        self.pending_g = false;

        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('t') => self.open_picker(),
            KeyCode::Char('o') => {
                self.outline_visible = !self.outline_visible;
                if !self.outline_visible {
                    self.focus = Focus::Content; // don't strand focus on a hidden pane
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.search_query.clear();
            }
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Content && self.outline_visible {
                    Focus::Outline
                } else {
                    Focus::Content
                };
            }
            KeyCode::Char('n') => self.jump_match(true),
            KeyCode::Char('N') => self.jump_match(false),
            KeyCode::Char('g') => {
                if was_g {
                    self.set_scroll(0);
                } else {
                    self.pending_g = true;
                }
            }
            KeyCode::Char('G') | KeyCode::End => self.set_scroll(self.max_scroll()),
            KeyCode::Char('d') if mods.contains(KeyModifiers::CONTROL) => {
                self.scroll_by(i32::from(half));
            }
            KeyCode::Char('u') if mods.contains(KeyModifiers::CONTROL) => {
                self.scroll_by(-i32::from(half));
            }
            KeyCode::Char(' ') | KeyCode::PageDown => self.scroll_by(i32::from(page)),
            KeyCode::PageUp => self.scroll_by(-i32::from(page)),
            KeyCode::Enter if self.focus == Focus::Outline => self.jump_to_selected_heading(),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Home => self.set_scroll(0),
            _ => {}
        }
    }

    fn move_down(&mut self) {
        if self.focus == Focus::Outline {
            self.outline_step(true);
        } else {
            self.scroll_by(1);
        }
    }

    fn move_up(&mut self) {
        if self.focus == Focus::Outline {
            self.outline_step(false);
        } else {
            self.scroll_by(-1);
        }
    }

    fn scroll_by(&mut self, delta: i32) {
        let next = i32::from(self.scroll) + delta;
        let clamped = next.clamp(0, i32::from(self.max_scroll()));
        self.set_scroll(u16::try_from(clamped).unwrap_or(0));
    }

    fn set_scroll(&mut self, scroll: u16) {
        let scroll = scroll.min(self.max_scroll());
        if self.scroll != scroll {
            self.scroll = scroll;
        }
    }

    fn outline_step(&mut self, forward: bool) {
        if self.doc.outline.is_empty() {
            return;
        }
        let len = self.doc.outline.len();
        let cur = self.outline_state.selected().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.outline_state.select(Some(next));
    }

    fn jump_to_selected_heading(&mut self) {
        let Some(sel) = self.outline_state.selected() else {
            return;
        };
        let Some(item) = self.doc.outline.get(sel) else {
            return;
        };
        if let Some(offset) = self.block_jump.get(item.block_index) {
            self.set_scroll(u16::try_from(*offset).unwrap_or(u16::MAX));
        }
        self.focus = Focus::Content;
    }

    // ─── Search ──────────────────────────────────────────────────

    fn search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                self.run_search();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => self.search_query.push(c),
            _ => {}
        }
    }

    fn run_search(&mut self) {
        self.matches.clear();
        self.match_idx = 0;
        if self.search_query.is_empty() {
            return;
        }
        let needle = self.search_query.to_lowercase();
        for (idx, line) in self.content.lines.iter().enumerate() {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if text.to_lowercase().contains(&needle) {
                self.matches.push(idx);
            }
        }
        if !self.matches.is_empty() {
            self.set_scroll(u16::try_from(self.matches[0]).unwrap_or(0));
        }
    }

    fn jump_match(&mut self, forward: bool) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len();
        self.match_idx = if forward {
            (self.match_idx + 1) % len
        } else {
            (self.match_idx + len - 1) % len
        };
        let line = self.matches[self.match_idx];
        self.set_scroll(u16::try_from(line).unwrap_or(0));
    }

    // ─── Theme picker ────────────────────────────────────────────

    fn open_picker(&mut self) {
        self.show_picker = true;
        self.picker_saved = self.theme_idx;
        self.picker_state.select(Some(self.theme_idx));
    }

    fn picker_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.apply_theme(self.picker_saved);
                self.show_picker = false;
            }
            KeyCode::Enter => {
                self.show_picker = false;
                self.save_config();
            }
            KeyCode::Char('j') | KeyCode::Down => self.picker_step(true),
            KeyCode::Char('k') | KeyCode::Up => self.picker_step(false),
            _ => {}
        }
    }

    fn picker_step(&mut self, forward: bool) {
        if self.theme_names.is_empty() {
            return;
        }
        let len = self.theme_names.len();
        let cur = self.picker_state.selected().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.picker_state.select(Some(next));
        self.apply_theme(next); // live preview
    }

    // ─── Drawing ─────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let [body, status] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        let content_area = if self.outline_visible && !self.doc.outline.is_empty() {
            let [outline, content] =
                Layout::horizontal([Constraint::Length(OUTLINE_WIDTH), Constraint::Min(10)])
                    .areas(body);
            self.draw_outline(frame, outline);
            content
        } else {
            body
        };

        self.viewport_h = content_area.height;
        self.ensure_content(content_area.width);
        self.clamp_scroll(); // height-only resizes change max_scroll
        self.draw_content(frame, content_area);
        self.draw_status(frame, status);

        if self.show_picker {
            self.draw_picker(frame, area);
        }
        if self.show_help {
            self.draw_help(frame, area);
        }
    }

    fn draw_content(&mut self, frame: &mut Frame, area: Rect) {
        // Render only the visible slice (and highlight only those lines), so the
        // draw path is O(viewport) rather than O(document) per frame.
        let top = usize::from(self.scroll);
        let total = self.content.lines.len();
        let end = top.saturating_add(usize::from(area.height)).min(total);
        let mut visible: Vec<Line<'static>> = if top < total {
            self.content.lines[top..end].to_vec()
        } else {
            Vec::new()
        };
        if !self.search_query.is_empty() {
            let needle: Vec<char> = self.search_query.to_lowercase().chars().collect();
            let hl = search_highlight_style();
            for line in &mut visible {
                *line = highlight_line(line, &needle, hl);
            }
        }
        let para = Paragraph::new(Text::from(visible))
            .style(Style::default().fg(self.content_fg).bg(self.content_bg));
        frame.render_widget(para, area);

        self.images.begin_frame(images::ImageView {
            scroll: self.scroll,
            height: area.height,
            width: area.width.saturating_sub(2),
        });

        // Draw visible image bands as terminal-row tiles. Scrolling by one row
        // then reuses every cached tile except the newly exposed edge row.
        let scroll = u32::from(self.scroll);
        let viewport = u32::from(area.height);
        let prefetch = u32::from(IMAGE_PREFETCH_ROWS);
        let prefetch_top = scroll.saturating_sub(prefetch);
        let prefetch_bottom = scroll.saturating_add(viewport).saturating_add(prefetch);
        let placements = std::mem::take(&mut self.image_placements);
        for placement in &placements {
            let tile_width = area.width.saturating_sub(2);
            if tile_width == 0 {
                continue;
            }
            let band_top = u32::from(placement.line);
            let visible_rows = visible_band_rows(placement, scroll, viewport);
            if let Some((vis_top, vis_bottom)) = visible_rows {
                let rel = u16::try_from(vis_top - scroll).unwrap_or(0);
                let start_row = u16::try_from(vis_top - band_top).unwrap_or(0);
                let rows = u16::try_from(vis_bottom - vis_top).unwrap_or(0);
                for offset in 0..rows {
                    let tile_area = Rect {
                        x: area.x.saturating_add(2),
                        y: area.y.saturating_add(rel).saturating_add(offset),
                        width: tile_width,
                        height: 1,
                    };
                    if let Some(proto) = self.images.row_protocol(
                        &placement.src,
                        placement.line,
                        start_row.saturating_add(offset),
                        placement.rows,
                        tile_area,
                    ) {
                        frame.render_stateful_widget(StatefulImage::new(), tile_area, proto);
                    }
                }
            }
            let Some((want_top, want_bottom)) =
                visible_band_rows(placement, prefetch_top, prefetch_bottom - prefetch_top)
            else {
                continue;
            };
            for row_abs in want_top..want_bottom {
                if visible_rows
                    .is_some_and(|(vis_top, vis_bottom)| row_abs >= vis_top && row_abs < vis_bottom)
                {
                    continue;
                }
                let row = u16::try_from(row_abs - band_top).unwrap_or(0);
                self.images.prefetch_row(
                    &placement.src,
                    placement.line,
                    row,
                    placement.rows,
                    tile_width,
                );
            }
        }
        self.image_placements = placements;
        self.images.finish_frame();
    }

    fn draw_outline(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .doc
            .outline
            .iter()
            .map(|item| {
                let indent = "  ".repeat(usize::from(item.level.saturating_sub(1)));
                let marker = self.glyphs.outline_marker();
                ListItem::new(Line::from(vec![
                    Span::styled(indent, Style::default()),
                    Span::styled(
                        format!("{marker} "),
                        Style::default().fg(self.chrome.accent),
                    ),
                    Span::styled(
                        super::layout::sanitize(&item.title).into_owned(),
                        Style::default().fg(self.chrome.text),
                    ),
                ]))
            })
            .collect();

        let border = if self.focus == Focus::Outline {
            self.chrome.border_focused
        } else {
            self.chrome.border
        };
        let list = List::new(items)
            .block(
                WBlock::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border))
                    .title(Span::styled(
                        " Outline ",
                        Style::default()
                            .fg(self.chrome.accent2)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(
                Style::default()
                    .bg(self.chrome.panel_bg)
                    .fg(self.chrome.text),
            )
            .highlight_style(
                Style::default()
                    .bg(self.chrome.selection_bg)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, area, &mut self.outline_state);
    }

    fn draw_status(&mut self, frame: &mut Frame, area: Rect) {
        // Draw the progress meter as background-filled spaces rather than block
        // glyphs: a colored space is always exactly one cell tall, whereas full
        // blocks can overshoot the line height in some terminal fonts.
        const BAR_W: usize = 10;
        let max = self.max_scroll();
        let pct: u16 = if max == 0 {
            100
        } else {
            u16::try_from(u32::from(self.scroll) * 100 / u32::from(max)).unwrap_or(100)
        };
        let filled = (usize::from(pct) * BAR_W / 100).min(BAR_W);
        let bar_filled = Span::styled(" ".repeat(filled), Style::default().bg(self.chrome.accent));
        let bar_track = Span::styled(
            " ".repeat(BAR_W - filled),
            Style::default().bg(self.chrome.border),
        );

        let theme_name = self
            .theme_names
            .get(self.theme_idx)
            .cloned()
            .unwrap_or_default();

        let hint = if self.mode == Mode::Search {
            format!("/{}", super::layout::sanitize(&self.search_query))
        } else if !self.matches.is_empty() {
            format!(
                "match {}/{}  /search ?help t theme o outline q quit",
                self.match_idx + 1,
                self.matches.len()
            )
        } else {
            "j/k scroll  /search  t theme  o outline  ?help  q quit".to_string()
        };

        let accent = Style::default().fg(self.chrome.accent);
        let muted = Style::default().fg(self.chrome.muted);
        let left = Line::from(vec![
            Span::styled(
                format!(" {} ", self.glyphs.diamond()),
                accent.add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate_plain(&self.title, 28),
                Style::default()
                    .fg(self.chrome.text)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", muted),
            bar_filled,
            bar_track,
            Span::styled(format!(" {pct:>3}%  "), muted),
            Span::styled(theme_name, Style::default().fg(self.chrome.accent2)),
            Span::styled(format!("   {hint}"), muted),
        ]);
        let para = Paragraph::new(left).style(Style::default().bg(self.chrome.panel_bg));
        frame.render_widget(para, area);
    }

    fn draw_picker(&mut self, frame: &mut Frame, area: Rect) {
        let popup = centered_rect(46, 70, area);
        frame.render_widget(Clear, popup);
        let items: Vec<ListItem> = self
            .theme_names
            .iter()
            .map(|n| {
                ListItem::new(Line::from(Span::styled(
                    n.clone(),
                    Style::default().fg(self.chrome.text),
                )))
            })
            .collect();
        let list = List::new(items)
            .block(
                WBlock::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.chrome.border_focused))
                    .title(Span::styled(
                        " Theme  (↑↓ preview · Enter apply · Esc cancel) ",
                        Style::default()
                            .fg(self.chrome.accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(Style::default().bg(self.chrome.panel_bg))
            .highlight_style(
                Style::default()
                    .bg(self.chrome.selection_bg)
                    .fg(self.chrome.accent)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, popup, &mut self.picker_state);
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let popup = centered_rect(54, 60, area);
        frame.render_widget(Clear, popup);
        let rows = [
            ("j / k, ↑ / ↓", "scroll line"),
            ("Ctrl-d / Ctrl-u", "half page"),
            ("Space / PageDn", "page down"),
            ("g g / G", "top / bottom"),
            ("o", "toggle outline"),
            ("Tab", "switch focus"),
            ("Enter (outline)", "jump to heading"),
            ("/ then n / N", "search / next / prev"),
            ("t", "theme picker"),
            ("q / Esc", "quit"),
        ];
        let lines: Vec<Line> = rows
            .iter()
            .map(|(k, v)| {
                Line::from(vec![
                    Span::styled(
                        format!("  {k:<18}"),
                        Style::default().fg(self.chrome.accent),
                    ),
                    Span::styled((*v).to_string(), Style::default().fg(self.chrome.text)),
                ])
            })
            .collect();
        let para = Paragraph::new(Text::from(lines))
            .block(
                WBlock::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.chrome.border_focused))
                    .title(Span::styled(
                        " Keys ",
                        Style::default()
                            .fg(self.chrome.accent)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .style(Style::default().bg(self.chrome.panel_bg));
        frame.render_widget(para, popup);
    }
}

// ─── Free helpers ────────────────────────────────────────────────

fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

/// Style applied to search matches: electric yellow on black.
fn search_highlight_style() -> Style {
    Style::default()
        .bg(Color::Rgb(241, 250, 140))
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

fn highlight_line(line: &Line<'static>, needle: &[char], hl: Style) -> Line<'static> {
    let cells: Vec<(char, Style)> = line
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect();
    if cells.len() < needle.len() {
        return line.clone();
    }

    let lower: Vec<char> = cells
        .iter()
        .map(|(c, _)| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut marks = vec![false; cells.len()];
    let nlen = needle.len();
    let mut i = 0;
    while i + nlen <= lower.len() {
        if lower[i..i + nlen] == *needle {
            for mark in &mut marks[i..i + nlen] {
                *mark = true;
            }
            i += nlen;
        } else {
            i += 1;
        }
    }
    if !marks.iter().any(|m| *m) {
        return line.clone();
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut current: Option<(Style, bool)> = None;
    for (idx, (ch, style)) in cells.iter().enumerate() {
        let key = (*style, marks[idx]);
        if current != Some(key) {
            if let Some((st, marked)) = current.take() {
                spans.push(Span::styled(
                    std::mem::take(&mut buf),
                    if marked { st.patch(hl) } else { st },
                ));
            }
            current = Some(key);
        }
        buf.push(*ch);
    }
    if let Some((st, marked)) = current {
        spans.push(Span::styled(buf, if marked { st.patch(hl) } else { st }));
    }
    Line::from(spans)
}

fn load_theme_or_default(name: &str) -> ResolvedTheme {
    let mut warnings = WarningCollector::new();
    let source = ThemeSource::BuiltIn(name.to_string());
    crate::theme::load_theme(&source, &mut warnings).unwrap_or_else(|_| {
        let fallback = ThemeSource::BuiltIn("silk-light".to_string());
        let mut wc = WarningCollector::new();
        crate::theme::load_theme(&fallback, &mut wc).unwrap_or_else(|_| ResolvedTheme {
            tokens: crate::theme::tokens::ThemeTokens::default(),
            tmtheme_xml: String::new(),
        })
    })
}

fn truncate_plain(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

fn visible_band_rows(placement: &Placement, scroll: u32, viewport: u32) -> Option<(u32, u32)> {
    let band_top = u32::from(placement.line);
    let band_bottom = band_top + u32::from(placement.rows);
    let vis_top = band_top.max(scroll);
    let vis_bottom = band_bottom.min(scroll + viewport);
    (vis_top < vis_bottom).then_some((vis_top, vis_bottom))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);
    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    center
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample() -> App {
        let body = "# Title\n\nSome **bold** text.\n\n## Section\n\n- a\n- b\n\n```rust\nfn main() {}\n```\n";
        let theme = load_theme_or_default("silk-light");
        App::new(
            body,
            theme,
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
        )
    }

    #[test]
    fn renders_one_frame_without_panicking() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        assert!(app.content_len() > 0, "content should render some lines");
    }

    #[test]
    fn reload_rereads_the_watched_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "# One\n").expect("write");
        let theme = load_theme_or_default("silk-light");
        let mut app = App::new(
            "# One\n",
            theme,
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            Some(path.clone()),
        );
        assert_eq!(app.doc.outline.len(), 1);
        std::fs::write(&path, "# One\n\n## Two\n").expect("rewrite");
        app.reload();
        assert_eq!(
            app.doc.outline.len(),
            2,
            "reload should pick up the new heading"
        );
    }

    #[test]
    fn outline_and_offsets_populated() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        assert_eq!(app.doc.outline.len(), 2, "two headings expected");
        assert_eq!(app.block_spans.len(), app.doc.blocks.len());
    }

    #[test]
    fn scroll_clamps_to_content() {
        let mut app = sample();
        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        app.scroll_by(10_000);
        assert!(app.scroll <= app.max_scroll());
    }

    #[test]
    fn search_finds_matches() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        app.search_query = "section".to_string();
        app.run_search();
        assert!(!app.matches.is_empty(), "should find 'section' heading");
    }

    #[test]
    fn content_spans_carry_theme_background() {
        // Regression: ansi-to-tui leaves a Reset background that paints as the
        // terminal default (black on a dark profile) under light themes.
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        assert_ne!(
            app.content_bg,
            Color::Reset,
            "light theme resolves a page bg"
        );
        let leaked = app
            .content
            .lines
            .iter()
            .flat_map(|l| &l.spans)
            .any(|s| s.style.bg.is_none() || s.style.bg == Some(Color::Reset));
        assert!(
            !leaked,
            "every content span should carry an explicit background"
        );
    }

    #[test]
    fn theme_switch_marks_dirty_and_rerenders() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let before = app.theme_idx;
        app.apply_theme((before + 1) % app.theme_names.len());
        assert!(app.theme_dirty);
        terminal.draw(|f| app.draw(f)).expect("redraw");
        assert!(!app.theme_dirty, "redraw should re-render content");
    }
}
