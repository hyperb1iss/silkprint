//! Scrollable TUI reader built on the same `RenderedDoc` as the one-shot path.
//!
//! Content is rendered through [`super::ansi`] at the viewport width and parsed
//! into ratatui text via `ansi-to-tui`, so the TUI and the pipe-friendly output
//! stay pixel-identical. opaline themes the chrome (borders, status bar,
//! outline, popups); the document content keeps the silkprint theme.

mod chrome;

use std::io;

use ansi_to_tui::IntoText;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::ThemeSource;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use self::chrome::Chrome;
use super::caps::{Capabilities, ColorTier, GlyphTier, GraphicsProtocol};
use super::glyphs::Glyphs;
use super::model::RenderedDoc;

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
pub fn run(body: &str, theme_name: &str, glyph_override: Option<GlyphTier>) -> io::Result<()> {
    let mut app = App::new(body, theme_name, glyph_override);
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
}

impl App {
    fn new(body: &str, theme_name: &str, glyph_override: Option<GlyphTier>) -> Self {
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, body);
        let mut warnings = WarningCollector::new();
        crate::render::markdown::check_content(root, &mut warnings);
        let doc = super::walk::walk(root, &mut warnings);

        let theme = load_theme_or_default(theme_name);
        let theme_names: Vec<String> = crate::theme::builtin::list_themes()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        let theme_idx = theme_names
            .iter()
            .position(|n| n == theme_name)
            .unwrap_or(0);

        let title = doc
            .title
            .clone()
            .unwrap_or_else(|| "silkprint".to_string());
        let glyphs = Glyphs::new(glyph_override.unwrap_or(GlyphTier::NerdFont));

        let mut outline_state = ListState::default();
        if !doc.outline.is_empty() {
            outline_state.select(Some(0));
        }
        let outline_visible = doc.outline.len() > 1;

        Self {
            doc,
            theme,
            glyphs,
            chrome: Chrome::for_theme(theme_name),
            theme_names,
            theme_idx,
            title,
            content: Text::default(),
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
        }
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.on_key(key.code, key.modifiers);
                }
            }
        }
        Ok(())
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
        self.block_offsets = offsets;
        self.rendered_width = width;
        self.theme_dirty = false;
        self.clamp_scroll();
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
        let next = if forward { (cur + 1) % len } else { (cur + len - 1) % len };
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
            self.scroll = u16::try_from(self.matches[0]).unwrap_or(0).min(self.max_scroll());
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
            KeyCode::Enter => self.show_picker = false,
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
        let next = if forward { (cur + 1) % len } else { (cur + len - 1) % len };
        self.picker_state.select(Some(next));
        self.apply_theme(next); // live preview
    }

    // ─── Drawing ─────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let [body, status] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

        let content_area = if self.outline_visible && !self.doc.outline.is_empty() {
            let [outline, content] = Layout::horizontal([
                Constraint::Length(OUTLINE_WIDTH),
                Constraint::Min(10),
            ])
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
        let para = Paragraph::new(self.content.clone())
            .style(Style::default().fg(self.chrome.text).bg(self.chrome.bg))
            .scroll((self.scroll, 0));
        frame.render_widget(para, area);
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
                    Span::styled(format!("{marker} "), Style::default().fg(self.chrome.accent)),
                    Span::styled(item.title.clone(), Style::default().fg(self.chrome.text)),
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
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border))
                    .title(Span::styled(
                        " Outline ",
                        Style::default().fg(self.chrome.accent2).add_modifier(Modifier::BOLD),
                    )),
            )
            .style(Style::default().bg(self.chrome.panel_bg).fg(self.chrome.text))
            .highlight_style(
                Style::default()
                    .bg(self.chrome.selection_bg)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, area, &mut self.outline_state);
    }

    fn draw_status(&mut self, frame: &mut Frame, area: Rect) {
        let max = self.max_scroll();
        let pct: u16 = if max == 0 {
            100
        } else {
            u16::try_from(u32::from(self.scroll) * 100 / u32::from(max)).unwrap_or(100)
        };
        let bar = progress_bar(pct, 10);

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
            Span::styled(format!(" {} ", self.glyphs.diamond()), accent.add_modifier(Modifier::BOLD)),
            Span::styled(
                truncate_plain(&self.title, 28),
                Style::default().fg(self.chrome.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", muted),
            Span::styled(bar, accent),
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
            .map(|n| ListItem::new(Line::from(Span::styled(n.clone(), Style::default().fg(self.chrome.text)))))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.chrome.border_focused))
                    .title(Span::styled(
                        " Theme  (↑↓ preview · Enter apply · Esc cancel) ",
                        Style::default().fg(self.chrome.accent).add_modifier(Modifier::BOLD),
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
                    Span::styled(format!("  {k:<18}"), Style::default().fg(self.chrome.accent)),
                    Span::styled((*v).to_string(), Style::default().fg(self.chrome.text)),
                ])
            })
            .collect();
        let para = Paragraph::new(Text::from(lines)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.chrome.border_focused))
                .title(Span::styled(
                    " Keys ",
                    Style::default().fg(self.chrome.accent).add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().bg(self.chrome.panel_bg));
        frame.render_widget(para, popup);
    }
}

// ─── Free helpers ────────────────────────────────────────────────

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

fn progress_bar(pct: u16, width: usize) -> String {
    let filled = (usize::from(pct) * width / 100).min(width);
    let mut bar = String::with_capacity(width + 2);
    bar.push('\u{2595}');
    for i in 0..width {
        bar.push(if i < filled { '\u{2588}' } else { '\u{2591}' });
    }
    bar.push('\u{258f}');
    bar
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
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample() -> App {
        let body = "# Title\n\nSome **bold** text.\n\n## Section\n\n- a\n- b\n\n```rust\nfn main() {}\n```\n";
        App::new(body, "silk-light", Some(GlyphTier::Unicode))
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
