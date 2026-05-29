//! Scrollable TUI reader built on the same `RenderedDoc` as the one-shot path.
//!
//! Content is rendered through [`super::ansi`] at the viewport width and parsed
//! into ratatui text via `ansi-to-tui`, so the TUI and the pipe-friendly output
//! stay pixel-identical. opaline themes the chrome (borders, status bar,
//! outline, popups); the document content keeps the silkprint theme.

mod chrome;
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

/// Launch the interactive reader. Sets up and tears down the terminal
/// (panic-safe via ratatui's restore hook) and runs the event loop.
pub fn run(
    body: &str,
    theme: ResolvedTheme,
    theme_name: &str,
    glyph_override: Option<GlyphTier>,
    base_dir: Option<PathBuf>,
    watch_path: Option<PathBuf>,
) -> io::Result<()> {
    // Query the terminal's graphics protocol + font size before entering the
    // alternate screen. `None` falls back to text-only (image placeholders).
    let picker = Picker::from_query_stdio().ok();
    let mut app = App::new(body, theme, theme_name, glyph_override, picker, base_dir, watch_path);
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
    block_offsets: Vec<usize>,
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
            block_offsets: Vec::new(),
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
            let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            })
            .ok()?;
            let target = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(path.as_path());
            watcher.watch(target, notify::RecursiveMode::NonRecursive).ok()?;
            Some(watcher)
        });

        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(Duration::from_millis(200))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.on_key(key.code, key.modifiers);
            }
            if rx.try_iter().count() > 0 {
                self.reload();
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
                self.outline_state.select((!self.doc.outline.is_empty()).then_some(0));
            }
            None if !self.doc.outline.is_empty() => self.outline_state.select(Some(0)),
            _ => {}
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

        self.block_offsets = offsets;
        self.image_placements.clear();
        if self.images.enabled() {
            self.reserve_images(width.saturating_sub(2));
        }
        self.rendered_width = width;
        self.theme_dirty = false;
        self.clamp_scroll();
    }

    /// Insert blank rows into the content for each top-level image and record
    /// where to draw the image widget.
    fn reserve_images(&mut self, content_width: u16) {
        let image_blocks: Vec<(usize, String)> = self
            .doc
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(i, block)| match block {
                Block::Image { src, .. } => {
                    self.block_offsets.get(i).map(|&line| (line, src.clone()))
                }
                _ => None,
            })
            .collect();

        let mut inserted = 0usize;
        for (orig_line, src) in image_blocks {
            let Some(loaded) = self.images.get(&src, content_width) else {
                continue;
            };
            let rows = loaded.rows;
            let line = orig_line + inserted;
            let insert_at = (line + 1).min(self.content.lines.len());
            for _ in 0..rows.saturating_sub(1) {
                self.content.lines.insert(insert_at, Line::default());
            }
            inserted += usize::from(rows.saturating_sub(1));
            self.image_placements.push(Placement {
                src,
                line: u16::try_from(line).unwrap_or(u16::MAX),
                rows,
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
        self.scroll = self.scroll.min(self.max_scroll());
    }

    fn apply_theme(&mut self, idx: usize) {
        if let Some(name) = self.theme_names.get(idx) {
            self.theme = load_theme_or_default(name);
            self.chrome = Chrome::for_theme(name);
            self.theme_idx = idx;
            self.theme_dirty = true;
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
            KeyCode::Char('o') => self.outline_visible = !self.outline_visible,
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
                    self.scroll = 0;
                } else {
                    self.pending_g = true;
                }
            }
            KeyCode::Char('G') | KeyCode::End => self.scroll = self.max_scroll(),
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
            KeyCode::Home => self.scroll = 0,
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
        self.scroll = u16::try_from(clamped).unwrap_or(0);
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
        if let Some(offset) = self.block_offsets.get(item.block_index) {
            self.scroll = u16::try_from(*offset).unwrap_or(0).min(self.max_scroll());
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
            self.scroll = u16::try_from(self.matches[0])
                .unwrap_or(0)
                .min(self.max_scroll());
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
        self.scroll = u16::try_from(line).unwrap_or(0).min(self.max_scroll());
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
        let content = if self.search_query.is_empty() {
            self.content.clone()
        } else {
            let needle: Vec<char> = self.search_query.to_lowercase().chars().collect();
            highlight_matches(&self.content, &needle)
        };
        let para = Paragraph::new(content)
            .style(Style::default().fg(self.content_fg).bg(self.content_bg))
            .scroll((self.scroll, 0));
        frame.render_widget(para, area);

        // Draw images over their reserved bands, but only when the full band is
        // in view (ratatui-image fits-to-area and can't clip a partial band).
        if self.image_placements.is_empty() {
            return;
        }
        let placements = self.image_placements.clone();
        let content_width = self.rendered_width;
        let scroll = u32::from(self.scroll);
        let viewport = u32::from(area.height);
        for placement in placements {
            let top = u32::from(placement.line);
            let bottom = top + u32::from(placement.rows);
            if top < scroll || bottom > scroll + viewport {
                continue;
            }
            let rel = u16::try_from(top - scroll).unwrap_or(0);
            let img_area = Rect {
                x: area.x.saturating_add(2),
                y: area.y.saturating_add(rel),
                width: area.width.saturating_sub(2),
                height: placement.rows.min(area.height.saturating_sub(rel)),
            };
            if img_area.width == 0 || img_area.height == 0 {
                continue;
            }
            if let Some(loaded) = self
                .images
                .get(&placement.src, content_width.saturating_sub(2))
            {
                frame.render_stateful_widget(StatefulImage::new(), img_area, &mut loaded.protocol);
            }
        }
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
            format!("/{}", self.search_query)
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

/// Return a copy of `text` with every (case-insensitive) occurrence of `needle`
/// highlighted. `needle` is pre-lowercased characters.
fn highlight_matches(text: &Text<'static>, needle: &[char]) -> Text<'static> {
    if needle.is_empty() {
        return text.clone();
    }
    let hl = Style::default()
        .bg(Color::Rgb(241, 250, 140))
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let lines: Vec<Line<'static>> = text
        .lines
        .iter()
        .map(|line| highlight_line(line, needle, hl))
        .collect();
    Text::from(lines)
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
        assert_eq!(app.doc.outline.len(), 2, "reload should pick up the new heading");
    }

    #[test]
    fn outline_and_offsets_populated() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        assert_eq!(app.doc.outline.len(), 2, "two headings expected");
        assert_eq!(app.block_offsets.len(), app.doc.blocks.len());
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
