//! Scrollable TUI reader built on the same `RenderedDoc` as the one-shot path.
//!
//! Content is rendered through [`super::ansi`] at the viewport width and parsed
//! into ratatui text via `ansi-to-tui`, so the TUI and the pipe-friendly output
//! stay pixel-identical. opaline themes the chrome (borders, status bar,
//! outline, popups); the document content keeps the silkprint theme.

mod chrome;
mod diagrams;
mod images;

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use ansi_to_tui::IntoText;
use notify::Watcher;
use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block as WBlock, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui_image::StatefulImage;
use ratatui_image::picker::Picker;
use unicode_width::UnicodeWidthChar;

use crate::ThemeSource;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use self::chrome::Chrome;
use self::images::{ImageStore, Placement};
use super::caps::{Capabilities, ColorTier, GlyphTier, GraphicsProtocol};
use super::glyphs::Glyphs;
use super::model::{Block, LinkTarget, RenderedDoc, Rgb};
use super::style::ContentStyleResolver;

const OUTLINE_WIDTH: u16 = 30;

/// Upper bound on the rows a single image/diagram band may reserve. Bands are
/// normally sized to the image's natural height and scrolled through; this only
/// guards against a pathologically tall input flooding the content flow.
const MAX_BAND_ROWS: u16 = 400;
const IMAGE_PREFETCH_MIN_ROWS: u16 = 48;
const MOUSE_SCROLL_ROWS: i32 = 3;

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

#[derive(Clone)]
struct LinkRegion {
    line: usize,
    start: u16,
    end: u16,
    target: LinkTarget,
}

/// A visited document in the back/forward history: its path and the scroll
/// offset at the time we left it, so returning restores the prior view.
#[derive(Clone)]
struct NavEntry {
    path: PathBuf,
    scroll: u16,
}

enum Osc8Target {
    Open(LinkTarget),
    Close,
}

#[derive(Clone)]
struct ThemeSnapshot {
    theme: ResolvedTheme,
    chrome: Chrome,
    theme_idx: usize,
    current_theme_name: Option<String>,
    saved_theme_name: Option<String>,
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
    let mouse = match MouseCapture::enable() {
        Ok(mouse) => mouse,
        Err(err) => {
            ratatui::restore();
            return Err(err);
        }
    };
    let result = app.run_loop(&mut terminal);
    drop(mouse);
    ratatui::restore();
    result
}

struct MouseCapture;

impl MouseCapture {
    fn enable() -> io::Result<Self> {
        let mut stdout = io::stdout();
        ratatui::crossterm::execute!(stdout, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for MouseCapture {
    fn drop(&mut self) {
        let _ = {
            let mut stdout = io::stdout();
            ratatui::crossterm::execute!(stdout, DisableMouseCapture)
        };
    }
}

#[allow(clippy::struct_excessive_bools)]
struct App {
    doc: RenderedDoc,
    theme: ResolvedTheme,
    glyphs: Glyphs,

    theme_names: Vec<String>,
    theme_idx: usize,
    current_theme_name: Option<String>,
    saved_theme_name: Option<String>,
    chrome: Chrome,
    title: String,

    content: Text<'static>,
    content_bg: Color,
    content_fg: Color,
    link_regions: Vec<LinkRegion>,
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
    picker_saved: Option<ThemeSnapshot>,
    picker_area: Rect,

    pending_g: bool,
    drag_row: Option<u16>,
    status_message: Option<String>,
    quit: bool,

    images: ImageStore,
    image_placements: Vec<Placement>,
    base_dir: Option<PathBuf>,
    path: Option<PathBuf>,
    back: Vec<NavEntry>,
    forward: Vec<NavEntry>,
    /// Heading anchor to jump to once the next document's layout is computed.
    pending_anchor: Option<String>,
    content_area: Rect,
    outline_area: Option<Rect>,
    status_area: Rect,
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
        Self::new_with_config(
            body,
            theme,
            theme_name,
            glyph_override,
            picker,
            base_dir,
            watch_path,
            super::config::load(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_config(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
        saved: super::config::ReaderConfig,
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
        let current_theme_name = theme_names
            .get(theme_idx)
            .filter(|name| name.as_str() == theme_name)
            .cloned();

        let title =
            super::layout::sanitize(doc.title.as_deref().unwrap_or("silkprint")).into_owned();
        let glyphs = Glyphs::new(glyph_override.unwrap_or(GlyphTier::NerdFont));

        let mut outline_state = ListState::default();
        if !doc.outline.is_empty() {
            outline_state.select(Some(0));
        }
        let outline_visible = saved.outline.unwrap_or(doc.outline.len() > 1);
        let saved_theme_name = saved.theme;

        Self {
            doc,
            theme,
            glyphs,
            chrome: Chrome::for_theme(theme_name),
            theme_names,
            theme_idx,
            current_theme_name,
            saved_theme_name,
            title,
            content: Text::default(),
            content_bg: Color::Reset,
            content_fg: Color::Reset,
            link_regions: Vec::new(),
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
            picker_saved: None,
            picker_area: Rect::default(),
            pending_g: false,
            drag_row: None,
            status_message: None,
            quit: false,
            images: ImageStore::new(picker, base_dir.clone()),
            image_placements: Vec::new(),
            base_dir,
            path: watch_path,
            back: Vec::new(),
            forward: Vec::new(),
            pending_anchor: None,
            content_area: Rect::default(),
            outline_area: None,
            status_area: Rect::default(),
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
                    Event::Mouse(mouse) => {
                        self.on_mouse(mouse);
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
        self.rewalk(&body);
    }

    /// Parse and walk `body` into the active document, resetting derived state
    /// (title, image caches, outline selection) but leaving navigation history,
    /// the current path, and the scroll offset to the caller.
    fn rewalk(&mut self, body: &str) {
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, body);
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

    // ─── Cross-document navigation ───────────────────────────────

    /// Load a local document into the reader, pointing image resolution and the
    /// link jail at its directory. Reads first, so a missing file leaves the
    /// current view untouched. The `anchor`, if any, is applied once the next
    /// layout is computed. Returns whether the load succeeded.
    fn load_path(&mut self, path: &std::path::Path, anchor: Option<String>) -> bool {
        let Ok(body) = std::fs::read_to_string(path) else {
            self.status_message = Some(format!(
                "can't open {}",
                truncate_plain(&path.display().to_string(), 48)
            ));
            return false;
        };
        let base = path
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .or_else(|| path.parent().map(std::path::Path::to_path_buf));
        self.path = Some(path.to_path_buf());
        self.images.set_base_dir(base.clone());
        self.base_dir = base;
        self.scroll = 0;
        // A new document invalidates the prior search.
        self.search_query.clear();
        self.matches.clear();
        self.match_idx = 0;
        self.rewalk(&body);
        self.pending_anchor = anchor;
        true
    }

    /// Follow a link to a local document, recording the current view so the
    /// reader can navigate back to it.
    fn open_local_doc(&mut self, path: &std::path::Path, anchor: Option<String>) {
        let from = self.path.clone();
        let from_scroll = self.scroll;
        if self.load_path(path, anchor) {
            if let Some(path) = from {
                self.back.push(NavEntry {
                    path,
                    scroll: from_scroll,
                });
            }
            self.forward.clear();
            self.status_message = Some(format!("opened {}", truncate_plain(&self.title, 40)));
        }
    }

    /// Return to the previously viewed document, restoring its scroll offset.
    fn go_back(&mut self) {
        let Some(entry) = self.back.pop() else {
            self.status_message = Some("no page to go back to".to_string());
            return;
        };
        let from = self.path.clone();
        let from_scroll = self.scroll;
        if self.load_path(&entry.path, None) {
            if let Some(path) = from {
                self.forward.push(NavEntry {
                    path,
                    scroll: from_scroll,
                });
            }
            self.scroll = entry.scroll; // draw() clamps once the layout is known
        }
    }

    /// Re-open the document a `go_back` left, restoring its scroll offset.
    fn go_forward(&mut self) {
        let Some(entry) = self.forward.pop() else {
            self.status_message = Some("no page to go forward to".to_string());
            return;
        };
        let from = self.path.clone();
        let from_scroll = self.scroll;
        if self.load_path(&entry.path, None) {
            if let Some(path) = from {
                self.back.push(NavEntry {
                    path,
                    scroll: from_scroll,
                });
            }
            self.scroll = entry.scroll;
        }
    }

    /// Resolve a link URL to a local Markdown file (and optional `#anchor`),
    /// jailed to the document directory. `None` when it carries a scheme,
    /// escapes the jail, or doesn't point at Markdown — those fall back to the
    /// system opener.
    fn local_markdown_target(&self, url: &str) -> Option<(PathBuf, Option<String>)> {
        let (path_part, anchor) = match url.split_once('#') {
            Some((p, a)) if !p.is_empty() => (p, Some(a.to_string())),
            _ => (url, None),
        };
        if uri_scheme(path_part).is_some() {
            return None;
        }
        let resolved = resolve_jailed(path_part, self.base_dir.as_deref())?;
        let is_markdown = resolved
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "md" | "markdown" | "mdown" | "mkd" | "mdwn" | "mkdn"
                )
            });
        is_markdown.then_some((resolved, anchor))
    }

    fn save_config(&self) {
        super::config::save(&self.reader_config());
    }

    fn reader_config(&self) -> super::config::ReaderConfig {
        let glyphs = match self.glyphs.tier() {
            GlyphTier::NerdFont => "nerdfont",
            GlyphTier::Unicode => "unicode",
            GlyphTier::Ascii => "ascii",
        };
        super::config::ReaderConfig {
            theme: self
                .current_theme_name
                .clone()
                .or_else(|| self.saved_theme_name.clone()),
            outline: Some(self.outline_visible),
            glyphs: Some(glyphs.to_string()),
        }
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
        let link_regions = if self.doc.links.is_empty() {
            Vec::new()
        } else {
            let mut link_caps = caps;
            link_caps.is_tty = true;
            let (linked_ansi, _) =
                super::ansi::render_with_offsets(&self.doc, &self.theme, &link_caps, self.glyphs);
            link_regions_from_osc(&linked_ansi)
        };
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
        self.link_regions = link_regions;
        self.image_placements.clear();
        if self.images.enabled() {
            let image_width = width.saturating_sub(2);
            self.reserve_bands(image_width);
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
            self.link_regions
                .retain(|region| region.line < start || region.line >= end);
            for region in &mut self.link_regions {
                if region.line >= end {
                    region.line = shift_line(region.line, shift);
                }
            }
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
        if let Some(name) = self.theme_names.get(idx).cloned() {
            self.theme = load_theme_or_default(&name);
            self.chrome = Chrome::for_theme(&name);
            self.theme_idx = idx;
            self.current_theme_name = Some(name.clone());
            self.saved_theme_name = Some(name);
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
        self.status_message = None;
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
            KeyCode::Char('b') | KeyCode::Backspace => self.go_back(),
            KeyCode::Char('f') => self.go_forward(),
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

    fn on_mouse(&mut self, mouse: MouseEvent) {
        if self.show_help {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.show_help = false;
            }
            return;
        }
        if self.show_picker {
            self.picker_mouse(mouse);
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => self.mouse_scroll(mouse, true),
            MouseEventKind::ScrollUp => self.mouse_scroll(mouse, false),
            MouseEventKind::Down(MouseButton::Left) => self.mouse_down(mouse),
            MouseEventKind::Drag(MouseButton::Left) => self.mouse_drag(mouse),
            MouseEventKind::Up(MouseButton::Left) => self.drag_row = None,
            _ => {}
        }
    }

    fn mouse_scroll(&mut self, mouse: MouseEvent, down: bool) {
        self.status_message = None;
        if self
            .outline_area
            .is_some_and(|area| contains(area, mouse.column, mouse.row))
        {
            for _ in 0..MOUSE_SCROLL_ROWS {
                self.outline_step(down);
            }
            self.focus = Focus::Outline;
            return;
        }
        let delta = if down {
            MOUSE_SCROLL_ROWS
        } else {
            -MOUSE_SCROLL_ROWS
        };
        self.scroll_by(delta);
        self.focus = Focus::Content;
    }

    fn mouse_down(&mut self, mouse: MouseEvent) {
        self.status_message = None;
        if self.select_outline_at(mouse.column, mouse.row) {
            return;
        }
        if contains(self.content_area, mouse.column, mouse.row) {
            self.focus = Focus::Content;
            self.drag_row = Some(mouse.row);
            let line = usize::from(self.scroll)
                .saturating_add(usize::from(mouse.row.saturating_sub(self.content_area.y)));
            let col = mouse.column.saturating_sub(self.content_area.x);
            self.activate_link_at(line, col);
        }
    }

    fn mouse_drag(&mut self, mouse: MouseEvent) {
        let Some(prev) = self.drag_row else {
            return;
        };
        let delta = i32::from(mouse.row) - i32::from(prev);
        if delta != 0 {
            self.scroll_by(-delta);
            self.drag_row = Some(mouse.row);
            self.status_message = None;
        }
    }

    fn select_outline_at(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.outline_area else {
            return false;
        };
        if !contains(area, column, row)
            || row <= area.y
            || row >= area.y.saturating_add(area.height).saturating_sub(1)
        {
            return false;
        }
        let visible_idx = usize::from(row.saturating_sub(area.y).saturating_sub(1));
        let idx = self.outline_state.offset().saturating_add(visible_idx);
        if idx >= self.doc.outline.len() {
            return false;
        }
        self.outline_state.select(Some(idx));
        self.jump_to_selected_heading();
        true
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

    fn jump_to_anchor(&mut self, anchor: &str) -> bool {
        let anchor = anchor.trim_start_matches('#');
        let Some(idx) = self
            .doc
            .outline
            .iter()
            .position(|item| item.anchor == anchor)
        else {
            self.status_message = Some(format!("missing #{anchor}"));
            return false;
        };
        self.outline_state.select(Some(idx));
        self.jump_to_selected_heading();
        self.status_message = Some(format!("jumped to #{anchor}"));
        true
    }

    fn activate_link_at(&mut self, line: usize, col: u16) -> bool {
        let Some(target) = self.link_at(line, col) else {
            return false;
        };
        self.activate_target(target);
        true
    }

    fn link_at(&self, line: usize, col: u16) -> Option<LinkTarget> {
        self.link_regions
            .iter()
            .find(|region| region.line == line && col >= region.start && col < region.end)
            .map(|region| region.target.clone())
    }

    fn activate_target(&mut self, target: LinkTarget) {
        match target {
            LinkTarget::Anchor(anchor) => {
                self.jump_to_anchor(&anchor);
            }
            LinkTarget::Url(url) => {
                if let Some((path, anchor)) = self.local_markdown_target(&url) {
                    self.open_local_doc(&path, anchor);
                } else {
                    self.open_url(&url);
                }
            }
        }
    }

    fn open_url(&mut self, url: &str) {
        let label = truncate_plain(super::layout::sanitize(url).as_ref(), 54);
        match open_target(url, self.base_dir.as_deref()) {
            Ok(target) => match open::that_detached(&target) {
                Ok(()) => self.status_message = Some(format!("opened {label}")),
                Err(err) => {
                    self.status_message = Some(format!(
                        "open failed: {}",
                        truncate_plain(&err.to_string(), 48)
                    ));
                }
            },
            Err(reason) => {
                self.status_message = Some(format!("blocked link: {reason}"));
            }
        }
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
        self.picker_saved = Some(self.theme_snapshot());
        self.picker_state.select(Some(self.theme_idx));
    }

    fn picker_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                if let Some(saved) = self.picker_saved.take() {
                    self.restore_theme(saved);
                }
                self.show_picker = false;
            }
            KeyCode::Enter => {
                self.picker_saved = None;
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

    fn theme_snapshot(&self) -> ThemeSnapshot {
        ThemeSnapshot {
            theme: self.theme.clone(),
            chrome: self.chrome,
            theme_idx: self.theme_idx,
            current_theme_name: self.current_theme_name.clone(),
            saved_theme_name: self.saved_theme_name.clone(),
        }
    }

    fn restore_theme(&mut self, snapshot: ThemeSnapshot) {
        self.theme = snapshot.theme;
        self.chrome = snapshot.chrome;
        self.theme_idx = snapshot.theme_idx;
        self.current_theme_name = snapshot.current_theme_name;
        self.saved_theme_name = snapshot.saved_theme_name;
        self.theme_dirty = true;
        self.images.clear_generated();
    }

    fn picker_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.picker_step(true),
            MouseEventKind::ScrollUp => self.picker_step(false),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(idx) = self.picker_index_at(mouse.column, mouse.row) {
                    self.picker_state.select(Some(idx));
                    self.apply_theme(idx);
                    self.picker_saved = None;
                    self.show_picker = false;
                    self.save_config();
                }
            }
            MouseEventKind::Up(MouseButton::Left) => self.drag_row = None,
            _ => {}
        }
    }

    fn picker_index_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.picker_area;
        if !contains(area, column, row)
            || row <= area.y
            || row >= area.y.saturating_add(area.height).saturating_sub(1)
        {
            return None;
        }
        let visible_idx = usize::from(row.saturating_sub(area.y).saturating_sub(1));
        let idx = self.picker_state.offset().saturating_add(visible_idx);
        (idx < self.theme_names.len()).then_some(idx)
    }

    // ─── Drawing ─────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let [body, status] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
        self.status_area = status;

        let content_area = if self.outline_visible && !self.doc.outline.is_empty() {
            let [outline, content] =
                Layout::horizontal([Constraint::Length(OUTLINE_WIDTH), Constraint::Min(10)])
                    .areas(body);
            self.outline_area = Some(outline);
            self.draw_outline(frame, outline);
            content
        } else {
            self.outline_area = None;
            body
        };
        self.content_area = content_area;

        self.viewport_h = content_area.height;
        self.ensure_content(content_area.width);
        // A linked-document jump needs the freshly computed block offsets, so it
        // waits until the new layout exists rather than firing at navigation.
        if let Some(anchor) = self.pending_anchor.take() {
            self.jump_to_anchor(&anchor);
        }
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
        let prefetch = viewport
            .saturating_mul(2)
            .max(u32::from(IMAGE_PREFETCH_MIN_ROWS));
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

        let hint = if let Some(message) = &self.status_message {
            super::layout::sanitize(message).into_owned()
        } else if self.mode == Mode::Search {
            format!("/{}", super::layout::sanitize(&self.search_query))
        } else if !self.matches.is_empty() {
            format!(
                "match {}/{}  /search ?help t theme o outline q quit",
                self.match_idx + 1,
                self.matches.len()
            )
        } else {
            "j/k scroll  /search  b/f back  t theme  o outline  ?help  q quit".to_string()
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
        self.picker_area = popup;
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
            ("click link", "follow .md link / open url"),
            ("b / f, Bksp", "history back / forward"),
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

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

fn link_regions_from_osc(ansi: &str) -> Vec<LinkRegion> {
    let mut chars = ansi.chars().peekable();
    let mut regions = Vec::new();
    let mut target: Option<LinkTarget> = None;
    let mut active: Option<(usize, usize, LinkTarget)> = None;
    let mut line = 0usize;
    let mut col = 0usize;

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.next() {
                Some(']') => {
                    flush_link_region(&mut active, line, col, &mut regions);
                    match osc8_target(&read_osc(&mut chars)) {
                        Some(Osc8Target::Open(next)) => target = Some(next),
                        Some(Osc8Target::Close) => target = None,
                        None => {}
                    }
                }
                Some('[') => skip_csi(&mut chars),
                _ => {}
            }
            continue;
        }
        if ch == '\n' {
            flush_link_region(&mut active, line, col, &mut regions);
            line = line.saturating_add(1);
            col = 0;
            continue;
        }
        let width = char_width(ch);
        if let Some(link) = target.as_ref().filter(|_| !ch.is_whitespace() && width > 0) {
            active.get_or_insert_with(|| (line, col, link.clone()));
        } else {
            flush_link_region(&mut active, line, col, &mut regions);
        }
        col = col.saturating_add(width);
    }
    flush_link_region(&mut active, line, col, &mut regions);
    regions
}

fn read_osc(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut payload = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\u{7}' {
            break;
        }
        if ch == '\u{1b}' && matches!(chars.peek(), Some('\\')) {
            chars.next();
            break;
        }
        payload.push(ch);
    }
    payload
}

fn skip_csi(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for ch in chars.by_ref() {
        if ('\u{40}'..='\u{7e}').contains(&ch) {
            break;
        }
    }
}

fn osc8_target(payload: &str) -> Option<Osc8Target> {
    let value = payload.strip_prefix("8;;")?;
    if value.is_empty() {
        return Some(Osc8Target::Close);
    }
    Some(Osc8Target::Open(
        if let Some(anchor) = value.strip_prefix('#') {
            LinkTarget::Anchor(anchor.to_string())
        } else {
            LinkTarget::Url(value.to_string())
        },
    ))
}

fn flush_link_region(
    active: &mut Option<(usize, usize, LinkTarget)>,
    line: usize,
    end: usize,
    regions: &mut Vec<LinkRegion>,
) {
    let Some((start_line, start, target)) = active.take() else {
        return;
    };
    if start_line != line || start >= end {
        return;
    }
    regions.push(LinkRegion {
        line,
        start: u16::try_from(start).unwrap_or(u16::MAX),
        end: u16::try_from(end).unwrap_or(u16::MAX),
        target,
    });
}

fn shift_line(line: usize, shift: isize) -> usize {
    if shift >= 0 {
        line.saturating_add(shift.unsigned_abs())
    } else {
        line.saturating_sub(shift.unsigned_abs())
    }
}

fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

fn open_target(url: &str, base_dir: Option<&std::path::Path>) -> Result<OsString, &'static str> {
    if let Some(scheme) = uri_scheme(url) {
        return if matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto"
        ) {
            Ok(OsString::from(url))
        } else {
            Err("unsupported scheme")
        };
    }
    let path = std::path::Path::new(url);
    if path.is_absolute() {
        return Err("absolute path");
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err("path escapes document");
    }
    let Some(base) = base_dir else {
        return Ok(OsString::from(url));
    };
    let canon_base = base
        .canonicalize()
        .map_err(|_| "document directory unavailable")?;
    let target = canon_base
        .join(path)
        .canonicalize()
        .map_err(|_| "local link missing")?;
    if !target.starts_with(&canon_base) {
        return Err("path escapes document");
    }
    Ok(target.into_os_string())
}

/// Resolve a relative link path against the document directory, returning the
/// canonicalized target only when it stays inside the (canonicalized) base.
/// Absolute paths and any `..`/root escape are rejected, mirroring the jail in
/// [`open_target`] — the reader must not read files outside the document tree.
fn resolve_jailed(rel: &str, base: Option<&std::path::Path>) -> Option<PathBuf> {
    let path = std::path::Path::new(rel);
    if path.is_absolute() {
        return None;
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return None;
    }
    let canon_base = base?.canonicalize().ok()?;
    let target = canon_base.join(path).canonicalize().ok()?;
    target.starts_with(&canon_base).then_some(target)
}

fn uri_scheme(value: &str) -> Option<&str> {
    let (scheme, _rest) = value.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
    .then_some(scheme)
}

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
    use crate::render::terminal::config::ReaderConfig;
    use crate::render::terminal::model::{Mods, Role};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample() -> App {
        sample_with_config(ReaderConfig::default())
    }

    fn sample_with_config(saved: ReaderConfig) -> App {
        let body = "# Title\n\nSome **bold** text.\n\n## Section\n\n- a\n- b\n\n```rust\nfn main() {}\n```\n";
        let theme = load_theme_or_default("silk-light");
        App::new_with_config(
            body,
            theme,
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            saved,
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
    fn follows_local_markdown_links_with_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        std::fs::write(&a, "# Alpha\n\n[to b](b.md)\n").expect("write a");
        std::fs::write(&b, "# Beta\n\n## Deep\n").expect("write b");
        std::fs::write(dir.path().join("note.txt"), "hi").expect("write note");
        let theme = load_theme_or_default("silk-light");
        let mut app = App::new(
            "# Alpha\n\n[to b](b.md)\n",
            theme,
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(a.clone()),
        );

        // A relative .md link resolves to a local target, splitting any anchor.
        let (target, anchor) = app.local_markdown_target("b.md").expect("local md target");
        assert!(target.ends_with("b.md"));
        assert_eq!(anchor, None);
        let (_t, frag) = app
            .local_markdown_target("b.md#deep")
            .expect("md target with anchor");
        assert_eq!(frag.as_deref(), Some("deep"));

        // External schemes, non-markdown files, and jail escapes are not
        // navigated in-reader (they fall back to the system opener).
        assert!(app.local_markdown_target("https://example.com").is_none());
        assert!(app.local_markdown_target("note.txt").is_none());
        assert!(app.local_markdown_target("../escape.md").is_none());

        // Following the link swaps documents and records back history.
        app.open_local_doc(&target, None);
        assert_eq!(app.title, "Beta");
        assert_eq!(app.back.len(), 1);
        assert!(app.forward.is_empty());

        // Back returns to the first document; forward replays the jump.
        app.go_back();
        assert_eq!(app.title, "Alpha");
        assert_eq!(app.forward.len(), 1);
        app.go_forward();
        assert_eq!(app.title, "Beta");
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
    fn content_spans_color_markdown_emphasis_with_theme_accents() {
        let mut app = App::new_with_config(
            "# Title\n\nPlain **bold** and *italic* and ***both*** and ~~gone~~.\n",
            load_theme_or_default("silkcircuit-glow"),
            "silkcircuit-glow",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let resolver = ContentStyleResolver::new(&app.theme);

        let bold = content_span(&app, "bold");
        assert!(bold.style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(
            bold.style.fg,
            resolver
                .resolve(Role::Body, Mods::default().with_bold())
                .fg
                .map(rgb_to_color)
        );

        let italic = content_span(&app, "italic");
        assert!(italic.style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(
            italic.style.fg,
            resolver
                .resolve(Role::Body, Mods::default().with_italic())
                .fg
                .map(rgb_to_color)
        );

        let both = content_span(&app, "both");
        assert!(both.style.add_modifier.contains(Modifier::BOLD));
        assert!(both.style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(
            both.style.fg,
            resolver
                .resolve(Role::Body, Mods::default().with_bold().with_italic())
                .fg
                .map(rgb_to_color)
        );

        let gone = content_span(&app, "gone");
        assert!(gone.style.add_modifier.contains(Modifier::CROSSED_OUT));
        assert_eq!(
            gone.style.fg,
            resolver
                .resolve(Role::Body, Mods::default().with_strikethrough())
                .fg
                .map(rgb_to_color)
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

    #[test]
    fn reader_config_records_selected_builtin_theme() {
        let mut app = sample();
        let next = (app.theme_idx + 1) % app.theme_names.len();
        app.apply_theme(next);

        assert_eq!(
            app.reader_config().theme,
            app.theme_names.get(next).cloned()
        );
    }

    #[test]
    fn reader_config_preserves_saved_theme_for_custom_theme() {
        let app = App::new_with_config(
            "# Title\n",
            load_theme_or_default("silk-light"),
            "Custom Theme",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig {
                theme: Some("silkcircuit-dawn".to_string()),
                outline: Some(true),
                glyphs: Some("unicode".to_string()),
            },
        );

        assert_eq!(
            app.reader_config().theme.as_deref(),
            Some("silkcircuit-dawn")
        );
    }

    #[test]
    fn picker_escape_restores_custom_theme_snapshot() {
        let mut app = App::new_with_config(
            "# Title\n",
            load_theme_or_default("silk-light"),
            "Custom Theme",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig {
                theme: Some("silkcircuit-dawn".to_string()),
                outline: Some(true),
                glyphs: Some("unicode".to_string()),
            },
        );

        app.open_picker();
        app.picker_step(true);
        assert!(app.current_theme_name.is_some());
        app.picker_key(KeyCode::Esc);

        assert!(app.current_theme_name.is_none());
        assert_eq!(
            app.reader_config().theme.as_deref(),
            Some("silkcircuit-dawn")
        );
    }

    #[test]
    fn open_target_allows_safe_links_and_blocks_risky_targets() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        std::fs::write(base.join("chapter.md"), "# Chapter\n").expect("chapter");
        std::fs::write(base.join("1:note.md"), "# Note\n").expect("note");

        assert!(open_target("https://example.com", Some(base)).is_ok());
        assert!(open_target("HTTPS://example.com", Some(base)).is_ok());
        assert!(open_target("mailto:hi@example.com", Some(base)).is_ok());
        assert!(open_target("MAILTO:hi@example.com", Some(base)).is_ok());
        assert!(open_target("chapter.md", Some(base)).is_ok());
        assert!(open_target("1:note.md", Some(base)).is_ok());
        assert!(open_target("javascript:alert(1)", Some(base)).is_err());
        assert!(open_target("/etc/passwd", Some(base)).is_err());
        assert!(open_target("../secret.md", Some(base)).is_err());

        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().expect("outside");
            let secret = outside.path().join("secret.md");
            std::fs::write(&secret, "# Secret\n").expect("secret");
            std::os::unix::fs::symlink(&secret, base.join("linked-secret.md")).expect("symlink");
            assert!(open_target("linked-secret.md", Some(base)).is_err());
        }
    }

    #[test]
    fn osc_link_regions_ignore_styled_non_links() {
        let regions = link_regions_from_osc(
            "\u{1b}[4mfoo\u{1b}[0m \u{1b}]8;;https://example.com\u{1b}\\\
             \u{1b}[4mfoo\u{1b}[0m\u{1b}]8;;\u{1b}\\",
        );

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start, 4);
        assert!(matches!(
            &regions[0].target,
            LinkTarget::Url(url) if url == "https://example.com"
        ));
    }

    #[test]
    fn osc_link_regions_keep_anchor_targets() {
        let regions = link_regions_from_osc("\u{1b}]8;;#target\u{1b}\\Jump\u{1b}]8;;\u{1b}\\");

        assert_eq!(regions.len(), 1);
        assert!(matches!(
            &regions[0].target,
            LinkTarget::Anchor(anchor) if anchor == "target"
        ));
    }

    #[test]
    fn link_regions_track_rendered_link_cells() {
        let mut app = App::new_with_config(
            "# Title\n\nA [SilkPrint](https://example.com) link.\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");

        let region = app
            .link_regions
            .iter()
            .find(|region| matches!(&region.target, LinkTarget::Url(url) if url == "https://example.com"))
            .expect("link region");

        assert!(app.link_at(region.line, region.start).is_some());
        assert!(app.link_at(region.line, region.end).is_none());
    }

    #[test]
    fn link_regions_ignore_matching_plain_text() {
        let mut app = App::new_with_config(
            "# Title\n\nfoo [foo](https://example.com)\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let region = app
            .link_regions
            .iter()
            .find(|region| matches!(&region.target, LinkTarget::Url(url) if url == "https://example.com"))
            .expect("link region");

        assert!(region.start > 2, "plain leading foo should not be linked");
        assert!(app.link_at(region.line, 2).is_none());
        assert!(app.link_at(region.line, region.start).is_some());
    }

    #[test]
    fn anchor_link_activation_jumps_to_heading() {
        let mut app = App::new_with_config(
            "# Top\n\n[Jump](#target)\n\none\n\ntwo\n\nthree\n\n## Target\n\nArrived.\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(100, 8);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");

        assert!(app.jump_to_anchor("target"));
        assert!(app.scroll > 0);
    }

    #[test]
    fn mouse_wheel_scrolls_content() {
        let body = format!(
            "# Title\n\n{}\n",
            (0..40)
                .map(|idx| format!("line {idx}"))
                .collect::<Vec<_>>()
                .join("\n\n")
        );
        let mut app = App::new_with_config(
            &body,
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(100, 8);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let before = app.scroll;

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: app.content_area.x.saturating_add(1),
            row: app.content_area.y.saturating_add(1),
            modifiers: KeyModifiers::NONE,
        });

        assert!(app.scroll > before);
    }

    #[test]
    fn draw_content_prefetches_scroll_horizon_and_cancels_old_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let image_path = dir.path().join("big.png");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            128,
            4096,
            image::Rgba([64, 96, 160, 255]),
        ))
        .save(&image_path)
        .expect("save image");
        let mut app = App::new_with_config(
            "# Title\n\n![Big](big.png)\n\nAfter\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            Some(Picker::halfblocks()),
            Some(dir.path().to_path_buf()),
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let placement = app
            .image_placements
            .iter()
            .find(|placement| placement.src == "big.png")
            .cloned()
            .expect("image placement");
        let first_rows = app.images.pending_rows_for(&placement.src, placement.line);
        let first_visible_bottom = u32::from(app.scroll) + u32::from(app.content_area.height);
        assert!(
            first_rows
                .iter()
                .any(|row| u32::from(placement.line) + u32::from(*row) >= first_visible_bottom),
            "first draw should prefetch beyond visible rows"
        );

        app.set_scroll(app.max_scroll());
        terminal.draw(|f| app.draw(f)).expect("redraw");
        let rows = app.images.pending_rows_for(&placement.src, placement.line);
        let scroll = u32::from(app.scroll);
        let viewport = u32::from(app.content_area.height);
        let prefetch = viewport
            .saturating_mul(2)
            .max(u32::from(IMAGE_PREFETCH_MIN_ROWS));
        let top = scroll.saturating_sub(prefetch);
        let bottom = scroll.saturating_add(viewport).saturating_add(prefetch);

        assert!(
            !rows.is_empty(),
            "redraw should keep new horizon rows pending"
        );
        assert!(
            rows.iter().all(|row| {
                let abs = u32::from(placement.line) + u32::from(*row);
                abs >= top && abs < bottom
            }),
            "rows outside the new scroll horizon should be canceled"
        );
    }

    fn content_span<'a>(app: &'a App, needle: &str) -> &'a Span<'static> {
        app.content
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content.contains(needle))
            .unwrap_or_else(|| panic!("missing content span {needle:?}"))
    }
}
