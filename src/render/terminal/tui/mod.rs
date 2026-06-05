//! Scrollable TUI reader built on the same `RenderedDoc` as the one-shot path.
//!
//! Content is rendered through [`super::ansi`] at the viewport width and parsed
//! into ratatui text via `ansi-to-tui`, so the TUI and the pipe-friendly output
//! stay pixel-identical. opaline themes the chrome (borders, status bar,
//! outline, popups); the document content keeps the silkprint theme.

mod actions;
mod bands;
mod browser;
mod chrome;
mod diagrams;
mod draw;
mod images;
mod links;
mod math;
mod state;
mod text;

use std::collections::BTreeMap;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};
use std::time::Duration;
use url::Url;

use ansi_to_tui::IntoText;
use notify::Watcher;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
#[cfg(test)]
use ratatui::style::Modifier;
#[cfg(test)]
use ratatui::text::Span;
use ratatui::text::{Line, Text};
use ratatui::widgets::ListState;
use ratatui_image::picker::Picker;

use crate::ThemeSource;
use crate::render::input::markdown_body_for_path;
use crate::render::origin::{DocumentOrigin, is_markdown_url, same_remote_origin};
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use self::actions::{Action, KeyBindings};
use self::bands::{
    BandSpec, band_specs, font_dirs_fingerprint, generated_key, rgb_to_color, theme_fingerprint,
};
use self::browser::{
    Bookmark, BrowserEntry, BrowserEntryKind, bookmarks_from_config, browser_entries,
    global_search_entries, is_markdown_path,
};
use self::chrome::Chrome;
use self::images::Placement;
use self::links::{
    link_preview, link_regions_from_osc, open_target, resolve_jailed, shift_line, uri_scheme,
};
use self::state::{NavEntry, TabState};
#[cfg(test)]
use self::text::base64_encode;
use self::text::{copy_osc52, selected_text, source_line_for_block, truncate_plain};
use super::caps::{Capabilities, ColorTier, GlyphTier, GraphicsProtocol};
use super::glyphs::Glyphs;
use super::model::{Block, LinkTarget, RenderedDoc};
use super::style::ContentStyleResolver;

const OUTLINE_WIDTH: u16 = 30;
const BROWSER_WIDTH: u16 = 34;

/// Upper bound on the rows a single image/diagram band may reserve. Bands are
/// normally sized to the image's natural height and scrolled through; this only
/// guards against a pathologically tall input flooding the content flow.
const MAX_BAND_ROWS: u16 = 400;
const MAX_MATH_BAND_ROWS: u16 = 800;
const IMAGE_PREFETCH_MIN_ROWS: u16 = 48;
const MOUSE_SCROLL_ROWS: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Browser,
    Content,
    Outline,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Search,
    GlobalSearch,
}

#[derive(Clone)]
struct ThemeSnapshot {
    theme: ResolvedTheme,
    chrome: Chrome,
    theme_idx: usize,
    current_theme_name: Option<String>,
    saved_theme_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TerminalTuiOptions {
    pub glyph_override: Option<GlyphTier>,
    pub images: bool,
    pub base_dir: Option<PathBuf>,
    pub watch_path: Option<PathBuf>,
    pub origin: Option<DocumentOrigin>,
    pub font_dirs: Vec<PathBuf>,
    pub settings: Option<super::config::ReaderSettings>,
}

impl Default for TerminalTuiOptions {
    fn default() -> Self {
        Self {
            glyph_override: None,
            images: true,
            base_dir: None,
            watch_path: None,
            origin: None,
            font_dirs: Vec::new(),
            settings: None,
        }
    }
}

/// Launch the interactive reader. Sets up and tears down the terminal
/// (panic-safe via ratatui's restore hook) and runs the event loop.
pub fn run(
    body: &str,
    theme: ResolvedTheme,
    theme_name: &str,
    options: TerminalTuiOptions,
) -> io::Result<()> {
    let TerminalTuiOptions {
        glyph_override,
        images,
        base_dir,
        watch_path,
        origin,
        font_dirs,
        settings,
    } = options;
    // Query the terminal's graphics protocol + font size before entering the
    // alternate screen. `None` (or `--no-images`) falls back to text-only.
    let picker = images.then(Picker::from_query_stdio).and_then(Result::ok);
    let settings = settings.unwrap_or_else(super::config::load_settings);
    let mut app = App::new_with_settings_and_origin(
        body,
        theme,
        theme_name,
        glyph_override,
        picker,
        base_dir,
        watch_path,
        origin,
        settings,
    );
    app.font_dirs = font_dirs;
    let current_path = app.active().path.clone();
    let session = super::config::load_session();
    app.restore_session_tabs(&session, current_path.as_deref());
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
    tabs: Vec<TabState>,
    active_tab: usize,
    theme: ResolvedTheme,
    glyphs: Glyphs,
    keybindings: KeyBindings,
    picker: Option<Picker>,

    theme_names: Vec<String>,
    theme_idx: usize,
    current_theme_name: Option<String>,
    saved_theme_name: Option<String>,
    chrome: Chrome,
    outline_visible: bool,
    browser_visible: bool,
    browser_root: Option<PathBuf>,
    browser_entries: Vec<BrowserEntry>,
    browser_state: ListState,
    bookmarks: Vec<Bookmark>,
    focus: Focus,

    mode: Mode,

    show_help: bool,
    show_bookmarks: bool,
    bookmark_state: ListState,
    bookmark_area: Rect,
    show_picker: bool,
    picker_state: ListState,
    picker_saved: Option<ThemeSnapshot>,
    picker_area: Rect,
    global_query: String,

    pending_g: bool,
    pending_bracket: Option<char>,
    drag_row: Option<u16>,
    selection_anchor: Option<(usize, u16)>,
    selection_cursor: Option<(usize, u16)>,
    status_message: Option<String>,
    quit: bool,

    font_dirs: Vec<PathBuf>,
    content_area: Rect,
    browser_area: Option<Rect>,
    outline_area: Option<Rect>,
    status_area: Rect,
}

impl App {
    fn active(&self) -> &TabState {
        let idx = self.active_tab.min(self.tabs.len().saturating_sub(1));
        &self.tabs[idx]
    }

    fn active_mut(&mut self) -> &mut TabState {
        let idx = self.active_tab.min(self.tabs.len().saturating_sub(1));
        &mut self.tabs[idx]
    }

    #[cfg(test)]
    fn new(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
    ) -> Self {
        Self::new_with_origin(
            body,
            theme,
            theme_name,
            glyph_override,
            picker,
            base_dir,
            watch_path,
            None,
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn new_with_origin(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
        origin: Option<DocumentOrigin>,
    ) -> Self {
        Self::new_with_settings_and_origin(
            body,
            theme,
            theme_name,
            glyph_override,
            picker,
            base_dir,
            watch_path,
            origin,
            super::config::load_settings(),
        )
    }

    #[cfg(test)]
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
        Self::new_with_settings_and_origin(
            body,
            theme,
            theme_name,
            glyph_override,
            picker,
            base_dir,
            watch_path,
            None,
            super::config::ReaderSettings {
                reader: saved,
                user: super::config::UserConfig::default(),
            },
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn new_with_settings(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
        settings: super::config::ReaderSettings,
    ) -> Self {
        Self::new_with_settings_and_origin(
            body,
            theme,
            theme_name,
            glyph_override,
            picker,
            base_dir,
            watch_path,
            None,
            settings,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_settings_and_origin(
        body: &str,
        theme: ResolvedTheme,
        theme_name: &str,
        glyph_override: Option<GlyphTier>,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
        origin: Option<DocumentOrigin>,
        settings: super::config::ReaderSettings,
    ) -> Self {
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

        let glyphs = Glyphs::new(glyph_override.unwrap_or(GlyphTier::NerdFont));
        let tab = TabState::from_body(body, picker.clone(), base_dir, watch_path, origin);
        let keybindings = KeyBindings::from_config(&settings.user.keybindings);
        let bookmarks = bookmarks_from_config(&settings.user.bookmarks);
        let saved = settings.reader;
        let outline_visible = saved.outline.unwrap_or(tab.doc.outline.len() > 1);
        let saved_theme_name = saved.theme;

        Self {
            tabs: vec![tab],
            active_tab: 0,
            theme,
            glyphs,
            keybindings,
            picker,
            chrome: Chrome::for_theme(theme_name),
            theme_names,
            theme_idx,
            current_theme_name,
            saved_theme_name,
            outline_visible,
            browser_visible: false,
            browser_root: None,
            browser_entries: Vec::new(),
            browser_state: ListState::default(),
            bookmarks,
            focus: Focus::Content,
            mode: Mode::Normal,
            show_help: false,
            show_bookmarks: false,
            bookmark_state: ListState::default(),
            bookmark_area: Rect::default(),
            show_picker: false,
            picker_state: ListState::default(),
            picker_saved: None,
            picker_area: Rect::default(),
            global_query: String::new(),
            pending_g: false,
            pending_bracket: None,
            drag_row: None,
            selection_anchor: None,
            selection_cursor: None,
            status_message: None,
            quit: false,
            font_dirs: Vec::new(),
            content_area: Rect::default(),
            browser_area: None,
            outline_area: None,
            status_area: Rect::default(),
        }
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
        // Watch the input file's directory for changes (robust to editors that
        // save via atomic rename). `_watcher` must stay alive for the loop.
        let (tx, rx) = std::sync::mpsc::channel();
        let _watcher = self.active_mut().path.clone().and_then(|path| {
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
            if self.active_mut().images.poll_ready() {
                needs_redraw = true;
            }
            if needs_redraw {
                terminal.draw(|frame| self.draw(frame))?;
                needs_redraw = false;
            }
            let poll_timeout = if self.active_mut().images.has_pending() {
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
        self.save_session();
        Ok(())
    }

    fn restore_session_tabs(
        &mut self,
        session: &super::config::ReaderSession,
        current_path: Option<&Path>,
    ) {
        if session.tabs.is_empty() {
            return;
        }
        let current_key = current_path.map(session_path_key);
        let saved_contains_current = current_key.as_ref().is_some_and(|current| {
            session
                .tabs
                .iter()
                .any(|tab| session_path_key(&tab.path) == *current)
        });
        let mut restored: Vec<TabState> = session
            .tabs
            .iter()
            .filter_map(|tab| self.load_session_tab(tab))
            .collect();
        if restored.is_empty() {
            return;
        }
        if !saved_contains_current && let Some(current) = self.tabs.pop() {
            restored.insert(0, current);
            self.active_tab = 0;
        } else {
            self.active_tab = session.active_tab.min(restored.len() - 1);
        }
        self.tabs = restored;
    }

    fn load_session_tab(&self, saved: &super::config::SessionTab) -> Option<TabState> {
        let body = std::fs::read_to_string(&saved.path).ok()?;
        let body = markdown_body_for_path(&saved.path, body);
        let base = saved
            .path
            .canonicalize()
            .ok()
            .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
            .or_else(|| saved.path.parent().map(std::path::Path::to_path_buf));
        let mut tab = TabState::from_body(
            &body,
            self.picker.clone(),
            base,
            Some(saved.path.clone()),
            Some(DocumentOrigin::local(saved.path.clone())),
        );
        tab.scroll = saved.scroll;
        Some(tab)
    }

    fn save_session(&self) {
        super::config::save_session(&self.reader_session());
    }

    fn reader_session(&self) -> super::config::ReaderSession {
        let mut active_tab = 0;
        let mut tabs = Vec::new();
        for (idx, tab) in self.tabs.iter().enumerate() {
            if let Some(path) = tab.path.clone() {
                if idx == self.active_tab {
                    active_tab = tabs.len();
                }
                tabs.push(super::config::SessionTab {
                    path,
                    scroll: tab.scroll,
                });
            }
        }
        super::config::ReaderSession { active_tab, tabs }
    }

    /// Re-read and re-walk the watched file (live reload).
    fn reload(&mut self) {
        let Some(path) = self.active_mut().path.clone() else {
            return;
        };
        let Ok(body) = std::fs::read_to_string(&path) else {
            return;
        };
        let body = markdown_body_for_path(&path, body);
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
        self.active_mut().doc =
            super::walk::walk_with_origin(root, &mut warnings, self.active_mut().origin.as_ref());
        self.active_mut().source = body.to_string();
        self.active_mut().title =
            super::layout::sanitize(self.active().doc.title.as_deref().unwrap_or("silkprint"))
                .into_owned();
        self.active_mut().images.clear_cache();
        self.active_mut().image_placements.clear();
        let has_outline = !self.active().doc.outline.is_empty();
        match self.active_mut().outline_state.selected() {
            Some(sel) if sel >= self.active().doc.outline.len() => {
                self.active_mut()
                    .outline_state
                    .select(has_outline.then_some(0));
            }
            None if has_outline => self.active_mut().outline_state.select(Some(0)),
            _ => {}
        }
        if !has_outline {
            self.focus = Focus::Content; // outline may have vanished
        }
        let len = self.active().doc.blocks.len();
        self.active_mut().details_open.retain(|idx, _| *idx < len);
        self.active_mut().theme_dirty = true; // force ensure_content to re-render
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
        let body = markdown_body_for_path(path, body);
        let base = path
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .or_else(|| path.parent().map(std::path::Path::to_path_buf));
        self.active_mut().path = Some(path.to_path_buf());
        self.active_mut().origin = Some(DocumentOrigin::local(path.to_path_buf()));
        self.active_mut().images.set_base_dir(base.clone());
        self.active_mut().base_dir = base;
        self.active_mut().scroll = 0;
        // A new document invalidates the prior search.
        self.active_mut().search_query.clear();
        self.active_mut().matches.clear();
        self.active_mut().match_idx = 0;
        self.rewalk(&body);
        self.active_mut().pending_anchor = anchor;
        true
    }

    fn load_origin(&mut self, origin: &DocumentOrigin, anchor: Option<String>) -> bool {
        match origin {
            DocumentOrigin::Local(path) => self.load_path(path, anchor),
            DocumentOrigin::Remote(url) => self.load_remote_doc(url, anchor),
        }
    }

    fn load_remote_doc(&mut self, url: &Url, anchor: Option<String>) -> bool {
        let input = crate::render::remote::RemoteInput::Url(url.clone());
        let Ok(remote) = crate::render::remote::fetch_remote_document(&input) else {
            self.status_message = Some(format!(
                "can't fetch {}",
                truncate_plain(super::layout::sanitize(url.as_str()).as_ref(), 48)
            ));
            return false;
        };
        self.active_mut().path = None;
        self.active_mut().base_dir = None;
        self.active_mut().images.set_base_dir(None);
        self.active_mut().origin = Some(remote.origin);
        self.active_mut().scroll = 0;
        self.active_mut().search_query.clear();
        self.active_mut().matches.clear();
        self.active_mut().match_idx = 0;
        self.rewalk(&remote.body);
        self.active_mut().pending_anchor = anchor;
        true
    }

    /// Follow a link to a local document, recording the current view so the
    /// reader can navigate back to it.
    fn open_local_doc(&mut self, path: &std::path::Path, anchor: Option<String>) {
        let from = self.active_mut().origin.clone();
        let from_scroll = self.active().scroll;
        if self.load_path(path, anchor) {
            if let Some(origin) = from {
                self.active_mut().back.push(NavEntry {
                    origin,
                    scroll: from_scroll,
                });
            }
            self.active_mut().forward.clear();
            self.status_message = Some(format!(
                "opened {}",
                truncate_plain(&self.active().title, 40)
            ));
        }
    }

    fn open_remote_doc(&mut self, url: &Url, anchor: Option<String>) {
        let from = self.active_mut().origin.clone();
        let from_scroll = self.active().scroll;
        if self.load_remote_doc(url, anchor) {
            if let Some(origin) = from {
                self.active_mut().back.push(NavEntry {
                    origin,
                    scroll: from_scroll,
                });
            }
            self.active_mut().forward.clear();
            self.status_message = Some(format!(
                "opened {}",
                truncate_plain(&self.active().title, 40)
            ));
        }
    }

    /// Return to the previously viewed document, restoring its scroll offset.
    fn go_back(&mut self) {
        let Some(entry) = self.active_mut().back.pop() else {
            self.status_message = Some("no page to go back to".to_string());
            return;
        };
        let from = self.active_mut().origin.clone();
        let from_scroll = self.active().scroll;
        if self.load_origin(&entry.origin, None) {
            if let Some(origin) = from {
                self.active_mut().forward.push(NavEntry {
                    origin,
                    scroll: from_scroll,
                });
            }
            self.active_mut().scroll = entry.scroll; // draw() clamps once the layout is known
        }
    }

    /// Re-open the document a `go_back` left, restoring its scroll offset.
    fn go_forward(&mut self) {
        let Some(entry) = self.active_mut().forward.pop() else {
            self.status_message = Some("no page to go forward to".to_string());
            return;
        };
        let from = self.active_mut().origin.clone();
        let from_scroll = self.active().scroll;
        if self.load_origin(&entry.origin, None) {
            if let Some(origin) = from {
                self.active_mut().back.push(NavEntry {
                    origin,
                    scroll: from_scroll,
                });
            }
            self.active_mut().scroll = entry.scroll;
        }
    }

    fn next_tab(&mut self) {
        let len = self.tabs.len();
        if len <= 1 {
            self.status_message = Some("only one tab".to_string());
            return;
        }
        self.active_tab = (self.active_tab + 1) % len;
        self.status_message = Some(format!("tab {}/{}", self.active_tab + 1, len));
    }

    fn prev_tab(&mut self) {
        let len = self.tabs.len();
        if len <= 1 {
            self.status_message = Some("only one tab".to_string());
            return;
        }
        self.active_tab = (self.active_tab + len - 1) % len;
        self.status_message = Some(format!("tab {}/{}", self.active_tab + 1, len));
    }

    fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            self.status_message = Some("can't close last tab".to_string());
            return;
        }
        let title = self.tabs[self.active_tab].title.clone();
        self.tabs.remove(self.active_tab);
        self.active_tab = self.active_tab.min(self.tabs.len() - 1);
        self.status_message = Some(format!("closed {}", truncate_plain(&title, 36)));
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
        let resolved = resolve_jailed(path_part, self.active().base_dir.as_deref())?;
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

    fn remote_markdown_target(&self, url: &str) -> Option<(Url, Option<String>)> {
        let base = self.active().origin.as_ref()?.remote_url()?;
        let target = Url::parse(url).or_else(|_| base.join(url)).ok()?;
        if !same_remote_origin(base, &target) || !is_markdown_url(&target) {
            return None;
        }
        let mut doc_url = target;
        let anchor = doc_url.fragment().map(str::to_string);
        doc_url.set_fragment(None);
        Some((doc_url, anchor))
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
        if !self.active().theme_dirty && width == self.active().rendered_width {
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
        let doc = details_view(&self.active().doc, &self.active().details_open);
        let (ansi, offsets) =
            super::ansi::render_with_offsets(&doc, &self.theme, &caps, self.glyphs);
        let link_regions = if self.active().doc.links.is_empty() {
            Vec::new()
        } else {
            let mut link_caps = caps;
            link_caps.is_tty = true;
            let (linked_ansi, _) =
                super::ansi::render_with_offsets(&doc, &self.theme, &link_caps, self.glyphs);
            link_regions_from_osc(&linked_ansi)
        };
        self.active_mut().content = ansi.into_text().unwrap_or_else(|_| Text::raw(ansi.clone()));

        let resolver = ContentStyleResolver::new(&self.theme);
        let page_bg = resolver
            .page_background()
            .map_or(Color::Reset, rgb_to_color);
        let body_fg = resolver.body_color().map_or(Color::Reset, rgb_to_color);
        self.active_mut().content_bg = page_bg;
        self.active_mut().content_fg = body_fg;
        // ansi-to-tui leaves text spans with a Reset background, which paints as
        // the terminal's own default — black on a dark profile even for a light
        // document theme. Pin every unset span background to the page color so
        // content sits on its theme's surface, not the terminal's.
        for line in &mut self.active_mut().content.lines {
            for span in &mut line.spans {
                if span.style.bg.is_none() || span.style.bg == Some(Color::Reset) {
                    span.style.bg = Some(page_bg);
                }
            }
        }

        self.active_mut().block_spans = offsets;
        self.active_mut().block_jump = self
            .active_mut()
            .block_spans
            .iter()
            .map(|(start, _)| *start)
            .collect();
        self.active_mut().link_regions = link_regions;
        self.active_mut().image_placements.clear();
        if self.active_mut().images.enabled() {
            let image_width = width.saturating_sub(2);
            self.reserve_bands(image_width);
        }
        self.active_mut().rendered_width = width;
        self.active_mut().theme_dirty = false;
        self.clamp_scroll();
    }

    /// Replace each graphical block — inline images, diagrams, and math — with
    /// a blank band sized to the image it will draw, record where to draw it,
    /// and keep outline jump offsets in sync with the shift. Replacing (rather
    /// than covering) the source means the mermaid text / image alt never peeks
    /// out below an image that is shorter than its source block.
    fn reserve_bands(&mut self, content_width: u16) {
        let bands = band_specs(&self.active().doc, &self.theme);
        let theme = self.theme.clone();
        let theme_key = theme_fingerprint(&theme);
        let font_key = font_dirs_fingerprint(&self.font_dirs);
        let cell = self.active_mut().images.cell();
        // Size bands to the image's natural height (bounded only against
        // pathological inputs). Tall diagrams get a tall band and are scrolled
        // through — the draw path crops to whatever slice is on screen.
        let mut delta: isize = 0;
        for (block_index, spec) in bands {
            let (orig_start, block_height) = self.active().block_spans[block_index];
            if block_height == 0 {
                continue;
            }
            let (key, dims, max_rows) = match spec {
                BandSpec::Image(src) => {
                    let dims = self
                        .active_mut()
                        .images
                        .get(&src)
                        .map(|l| (l.width, l.height));
                    (src, dims, MAX_BAND_ROWS)
                }
                BandSpec::Mermaid { source, bg } => {
                    let key = generated_key("mermaid", &source, theme_key, bg, font_key);
                    let dims = self
                        .active_mut()
                        .images
                        .ensure_generated(&key, || diagrams::mermaid_image(&source, &theme, bg))
                        .map(|l| (l.width, l.height));
                    (key, dims, MAX_BAND_ROWS)
                }
                BandSpec::Math { source, bg } => {
                    let key = generated_key("math", &source, theme_key, bg, font_key);
                    let font_dirs = self.font_dirs.clone();
                    let dims = self
                        .active_mut()
                        .images
                        .ensure_generated(&key, || {
                            math::math_image(&source, &theme, &font_dirs, bg)
                        })
                        .map(|l| (l.width, l.height));
                    (key, dims, MAX_MATH_BAND_ROWS)
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
                .min(self.active().content.lines.len());
            let end = (start + block_height).min(self.active().content.lines.len());
            let band = usize::from(img_rows);
            self.active_mut()
                .content
                .lines
                .splice(start..end, std::iter::repeat_with(Line::default).take(band));
            let shift =
                isize::try_from(band).unwrap_or(0) - isize::try_from(end - start).unwrap_or(0);
            delta += shift;
            self.active_mut()
                .link_regions
                .retain(|region| region.line < start || region.line >= end);
            for region in &mut self.active_mut().link_regions {
                if region.line >= end {
                    region.line = shift_line(region.line, shift);
                }
            }
            for jump in self
                .active_mut()
                .block_jump
                .iter_mut()
                .skip(block_index + 1)
            {
                *jump = usize::try_from(isize::try_from(*jump).unwrap_or(0) + shift).unwrap_or(0);
            }
            self.active_mut().image_placements.push(Placement {
                src: key,
                line: u16::try_from(start).unwrap_or(u16::MAX),
                rows: img_rows,
            });
        }
    }

    fn content_len(&self) -> u16 {
        u16::try_from(self.active().content.lines.len()).unwrap_or(u16::MAX)
    }

    fn max_scroll(&self) -> u16 {
        self.content_len().saturating_sub(self.active().viewport_h)
    }

    fn clamp_scroll(&mut self) {
        self.set_scroll(self.active().scroll);
    }

    fn apply_theme(&mut self, idx: usize) {
        if let Some(name) = self.theme_names.get(idx).cloned() {
            self.theme = load_theme_or_default(&name);
            self.chrome = Chrome::for_theme(&name);
            self.theme_idx = idx;
            self.current_theme_name = Some(name.clone());
            self.saved_theme_name = Some(name);
            self.active_mut().theme_dirty = true;
            // Generated rasters bake in the old theme's colors and must be rebuilt.
            self.active_mut().images.clear_generated();
        }
    }

    // ─── Input ───────────────────────────────────────────────────

    fn on_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.show_bookmarks {
            self.bookmark_key(code);
            return;
        }
        if self.show_picker {
            self.picker_key(code);
            return;
        }
        match self.mode {
            Mode::Search => {
                self.search_key(code);
                return;
            }
            Mode::GlobalSearch => {
                self.global_search_key(code);
                return;
            }
            Mode::Normal => {}
        }
        self.status_message = None;
        self.normal_key(code, mods);
    }

    fn normal_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        let half = self.active().viewport_h / 2;
        let page = self.active_mut().viewport_h.saturating_sub(2).max(1);
        let was_g = self.pending_g;
        let was_bracket = self.pending_bracket.take();
        self.pending_g = false;

        if let Some(action) = self.keybindings.action_for(code, mods) {
            self.run_action(action);
            return;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('t') => self.open_picker(),
            KeyCode::Char('e') => self.toggle_browser(),
            KeyCode::Char('o') => {
                self.outline_visible = !self.outline_visible;
                if !self.outline_visible && self.focus == Focus::Outline {
                    self.focus = Focus::Content; // don't strand focus on a hidden pane
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.active_mut().search_query.clear();
            }
            KeyCode::Char('S') => self.start_global_search(),
            KeyCode::Char('B') => self.open_bookmarks(),
            KeyCode::Char('z') => self.toggle_details_at_cursor(),
            KeyCode::Char('r') => self.reveal_raw_at_cursor(),
            KeyCode::Char('E') => self.open_editor(),
            KeyCode::Tab => self.step_focus(),
            KeyCode::Char('n') => self.jump_match(true),
            KeyCode::Char('N') => self.jump_match(false),
            KeyCode::Char('b') | KeyCode::Backspace => self.go_back(),
            KeyCode::Char('f') => self.go_forward(),
            KeyCode::Char('L') => self.next_tab(),
            KeyCode::Char('H') => self.prev_tab(),
            KeyCode::Char('x') => self.close_tab(),
            KeyCode::Char('g') => {
                if was_g {
                    self.set_scroll(0);
                } else {
                    self.pending_g = true;
                }
            }
            KeyCode::Char(']') => {
                if was_bracket == Some(']') {
                    self.jump_heading(true);
                } else {
                    self.pending_bracket = Some(']');
                }
            }
            KeyCode::Char('[') => {
                if was_bracket == Some('[') {
                    self.jump_heading(false);
                } else {
                    self.pending_bracket = Some('[');
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
            KeyCode::Enter if self.focus == Focus::Browser => self.open_browser_selection(),
            KeyCode::Enter if self.focus == Focus::Outline => self.jump_to_selected_heading(),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Home => self.set_scroll(0),
            _ => {}
        }
    }

    fn run_action(&mut self, action: Action) {
        let half = self.active().viewport_h / 2;
        let page = self.active_mut().viewport_h.saturating_sub(2).max(1);
        match action {
            Action::Quit => self.quit = true,
            Action::Help => self.show_help = true,
            Action::Theme => self.open_picker(),
            Action::ToggleBrowser => self.toggle_browser(),
            Action::ToggleOutline => {
                self.outline_visible = !self.outline_visible;
                if !self.outline_visible && self.focus == Focus::Outline {
                    self.focus = Focus::Content;
                }
            }
            Action::Search => {
                self.mode = Mode::Search;
                self.active_mut().search_query.clear();
            }
            Action::GlobalSearch => self.start_global_search(),
            Action::Bookmarks => self.open_bookmarks(),
            Action::ToggleDetails => self.toggle_details_at_cursor(),
            Action::RevealRaw => self.reveal_raw_at_cursor(),
            Action::Edit => self.open_editor(),
            Action::ToggleFocus => self.step_focus(),
            Action::NextMatch => self.jump_match(true),
            Action::PrevMatch => self.jump_match(false),
            Action::Back => self.go_back(),
            Action::Forward => self.go_forward(),
            Action::Top => self.set_scroll(0),
            Action::Bottom => self.set_scroll(self.max_scroll()),
            Action::HalfDown => self.scroll_by(i32::from(half)),
            Action::HalfUp => self.scroll_by(-i32::from(half)),
            Action::PageDown => self.scroll_by(i32::from(page)),
            Action::PageUp => self.scroll_by(-i32::from(page)),
            Action::Down => self.move_down(),
            Action::Up => self.move_up(),
            Action::HeadingNext => self.jump_heading(true),
            Action::HeadingPrev => self.jump_heading(false),
            Action::TabNext => self.next_tab(),
            Action::TabPrev => self.prev_tab(),
            Action::TabClose => self.close_tab(),
        }
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        if self.show_help {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.show_help = false;
            }
            return;
        }
        if self.show_bookmarks {
            self.bookmark_mouse(mouse);
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
            MouseEventKind::Up(MouseButton::Left) => self.mouse_up(),
            MouseEventKind::Moved => self.mouse_moved(mouse),
            _ => {}
        }
    }

    fn mouse_scroll(&mut self, mouse: MouseEvent, down: bool) {
        self.status_message = None;
        if self
            .browser_area
            .is_some_and(|area| contains(area, mouse.column, mouse.row))
        {
            for _ in 0..MOUSE_SCROLL_ROWS {
                self.browser_step(down);
            }
            self.focus = Focus::Browser;
            return;
        }
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
        if self.select_browser_at(mouse.column, mouse.row) {
            return;
        }
        if self.select_outline_at(mouse.column, mouse.row) {
            return;
        }
        if contains(self.content_area, mouse.column, mouse.row) {
            self.focus = Focus::Content;
            let line = usize::from(self.active().scroll)
                .saturating_add(usize::from(mouse.row.saturating_sub(self.content_area.y)));
            let col = mouse.column.saturating_sub(self.content_area.x);
            if self.activate_link_at(line, col) {
                return;
            }
            self.drag_row = Some(mouse.row);
            self.selection_anchor = Some((line, col));
            self.selection_cursor = Some((line, col));
        }
    }

    fn mouse_drag(&mut self, mouse: MouseEvent) {
        if self.selection_anchor.is_some() && contains(self.content_area, mouse.column, mouse.row) {
            let line = usize::from(self.active().scroll)
                .saturating_add(usize::from(mouse.row.saturating_sub(self.content_area.y)));
            let col = mouse.column.saturating_sub(self.content_area.x);
            self.selection_cursor = Some((line, col));
            self.status_message = Some("selecting text".to_string());
            return;
        }
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

    fn mouse_up(&mut self) {
        let selection = self
            .selection_anchor
            .zip(self.selection_cursor)
            .and_then(|(start, end)| selected_text(&self.active().content.lines, start, end));
        self.selection_anchor = None;
        self.selection_cursor = None;
        self.drag_row = None;
        if let Some(text) = selection {
            if copy_osc52(&text).is_ok() {
                self.status_message = Some(format!("copied {} chars", text.chars().count()));
            } else {
                self.status_message = Some("copy failed".to_string());
            }
        }
    }

    fn mouse_moved(&mut self, mouse: MouseEvent) {
        if !contains(self.content_area, mouse.column, mouse.row) {
            self.clear_hover_message();
            return;
        }
        let line = usize::from(self.active().scroll)
            .saturating_add(usize::from(mouse.row.saturating_sub(self.content_area.y)));
        let col = mouse.column.saturating_sub(self.content_area.x);
        if let Some(target) = self.link_at(line, col) {
            self.status_message = Some(link_preview(&target));
        } else {
            self.clear_hover_message();
        }
    }

    fn clear_hover_message(&mut self) {
        if self
            .status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("link: "))
        {
            self.status_message = None;
        }
    }

    fn select_browser_at(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.browser_area else {
            return false;
        };
        if !contains(area, column, row)
            || row <= area.y
            || row >= area.y.saturating_add(area.height).saturating_sub(1)
        {
            return false;
        }
        let visible_idx = usize::from(row.saturating_sub(area.y).saturating_sub(1));
        let idx = self.browser_state.offset().saturating_add(visible_idx);
        if idx >= self.browser_entries.len() {
            return false;
        }
        self.browser_state.select(Some(idx));
        self.focus = Focus::Browser;
        self.open_browser_selection();
        true
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
        let idx = self
            .active_mut()
            .outline_state
            .offset()
            .saturating_add(visible_idx);
        if idx >= self.active().doc.outline.len() {
            return false;
        }
        self.active_mut().outline_state.select(Some(idx));
        self.jump_to_selected_heading();
        true
    }

    fn step_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Content if self.browser_visible => Focus::Browser,
            Focus::Content | Focus::Browser
                if self.outline_visible && !self.active().doc.outline.is_empty() =>
            {
                Focus::Outline
            }
            _ => Focus::Content,
        };
    }

    fn toggle_browser(&mut self) {
        self.browser_visible = !self.browser_visible;
        if self.browser_visible {
            self.refresh_browser();
            self.focus = Focus::Browser;
        } else if self.focus == Focus::Browser {
            self.focus = Focus::Content;
        }
    }

    fn browser_root_or_default(&self) -> PathBuf {
        self.browser_root
            .clone()
            .or_else(|| self.active().base_dir.clone())
            .or_else(|| {
                self.active()
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn refresh_browser(&mut self) {
        let root = self.browser_root_or_default();
        self.browser_root = Some(root.clone());
        self.browser_entries = browser_entries(&root);
        self.browser_state
            .select((!self.browser_entries.is_empty()).then_some(0));
    }

    fn browser_step(&mut self, forward: bool) {
        if self.browser_entries.is_empty() {
            return;
        }
        let len = self.browser_entries.len();
        let cur = self.browser_state.selected().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.browser_state.select(Some(next));
    }

    fn open_browser_selection(&mut self) {
        let Some(idx) = self.browser_state.selected() else {
            return;
        };
        let Some(entry) = self.browser_entries.get(idx).cloned() else {
            return;
        };
        match entry.kind {
            BrowserEntryKind::Parent | BrowserEntryKind::Directory => {
                self.browser_root = Some(entry.path);
                self.refresh_browser();
            }
            BrowserEntryKind::Markdown => {
                self.open_path_in_tab(&entry.path);
            }
            BrowserEntryKind::SearchResult { line } => self.open_search_result(&entry.path, line),
        }
    }

    fn open_path_in_tab(&mut self, path: &Path) -> bool {
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
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(|| path.parent().map(Path::to_path_buf));
        let tab = TabState::from_body(
            &body,
            self.picker.clone(),
            base,
            Some(path.to_path_buf()),
            Some(DocumentOrigin::local(path.to_path_buf())),
        );
        let title = tab.title.clone();
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.focus = Focus::Content;
        self.status_message = Some(format!("opened {}", truncate_plain(&title, 40)));
        true
    }

    fn open_search_result(&mut self, path: &Path, line: usize) {
        let query = self.global_query.clone();
        if !self.open_path_in_tab(path) {
            return;
        }
        if !query.is_empty() {
            self.active_mut().search_query = query;
            let width = self.content_area.width.max(80);
            self.ensure_content(width);
            self.run_search();
            self.scroll_to_search_result_line(line);
        }
        self.status_message = Some(format!(
            "opened {}:{line}",
            truncate_plain(&path.display().to_string(), 42)
        ));
    }

    fn scroll_to_search_result_line(&mut self, line: usize) {
        if self.active_mut().matches.is_empty() {
            return;
        }
        let needle = self.active_mut().search_query.to_lowercase();
        let occurrence = self
            .active()
            .source
            .lines()
            .take(line)
            .filter(|source_line| source_line.to_lowercase().contains(&needle))
            .count()
            .saturating_sub(1);
        let idx = occurrence.min(self.active_mut().matches.len().saturating_sub(1));
        if let Some(&matched_line) = self.active_mut().matches.get(idx) {
            self.active_mut().match_idx = idx;
            self.set_scroll(u16::try_from(matched_line).unwrap_or(u16::MAX));
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            Focus::Browser => self.browser_step(true),
            Focus::Outline => self.outline_step(true),
            Focus::Content => self.scroll_by(1),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Browser => self.browser_step(false),
            Focus::Outline => self.outline_step(false),
            Focus::Content => self.scroll_by(-1),
        }
    }

    fn scroll_by(&mut self, delta: i32) {
        let next = i32::from(self.active().scroll) + delta;
        let clamped = next.clamp(0, i32::from(self.max_scroll()));
        self.set_scroll(u16::try_from(clamped).unwrap_or(0));
    }

    fn set_scroll(&mut self, scroll: u16) {
        let scroll = scroll.min(self.max_scroll());
        if self.active().scroll != scroll {
            self.active_mut().scroll = scroll;
        }
    }

    fn outline_step(&mut self, forward: bool) {
        if self.active().doc.outline.is_empty() {
            return;
        }
        let len = self.active().doc.outline.len();
        let cur = self.active_mut().outline_state.selected().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.active_mut().outline_state.select(Some(next));
    }

    fn jump_heading(&mut self, forward: bool) {
        if self.active().doc.outline.is_empty() {
            return;
        }
        let scroll = usize::from(self.active().scroll);
        let next = if forward {
            self.active()
                .doc
                .outline
                .iter()
                .enumerate()
                .find(|(_, item)| self.heading_line(item).is_some_and(|line| line > scroll))
                .map_or(0, |(idx, _)| idx)
        } else {
            self.active()
                .doc
                .outline
                .iter()
                .enumerate()
                .rev()
                .find(|(_, item)| self.heading_line(item).is_some_and(|line| line < scroll))
                .map_or(self.active().doc.outline.len() - 1, |(idx, _)| idx)
        };
        self.active_mut().outline_state.select(Some(next));
        self.jump_to_selected_heading();
    }

    fn heading_line(&self, item: &super::model::OutlineItem) -> Option<usize> {
        self.active().block_jump.get(item.block_index).copied()
    }

    fn jump_to_selected_heading(&mut self) {
        let Some(sel) = self.active_mut().outline_state.selected() else {
            return;
        };
        let Some(block_index) = self
            .active()
            .doc
            .outline
            .get(sel)
            .map(|item| item.block_index)
        else {
            return;
        };
        if let Some(offset) = self.active().block_jump.get(block_index).copied() {
            self.set_scroll(u16::try_from(offset).unwrap_or(u16::MAX));
        }
        self.focus = Focus::Content;
    }

    fn jump_to_anchor(&mut self, anchor: &str) -> bool {
        let anchor = anchor.trim_start_matches('#');
        let Some(idx) = self
            .active()
            .doc
            .outline
            .iter()
            .position(|item| item.anchor == anchor)
        else {
            self.status_message = Some(format!("missing #{anchor}"));
            return false;
        };
        self.active_mut().outline_state.select(Some(idx));
        self.jump_to_selected_heading();
        self.status_message = Some(format!("jumped to #{anchor}"));
        true
    }

    fn toggle_details_at_cursor(&mut self) {
        let line = usize::from(self.active().scroll);
        let Some(idx) =
            self.active()
                .block_spans
                .iter()
                .enumerate()
                .find_map(|(idx, (start, height))| {
                    let end = start.saturating_add(*height).max(start.saturating_add(1));
                    (line >= *start && line < end).then_some(idx)
                })
        else {
            self.status_message = Some("no details block here".to_string());
            return;
        };
        let Some(open) = self.active().doc.blocks.get(idx).and_then(|block| {
            if let Block::Details { open, .. } = block {
                Some(*open)
            } else {
                None
            }
        }) else {
            self.status_message = Some("no details block here".to_string());
            return;
        };
        let next = !self
            .active()
            .details_open
            .get(&idx)
            .copied()
            .unwrap_or(open);
        self.active_mut().details_open.insert(idx, next);
        self.active_mut().theme_dirty = true;
        self.status_message = Some(if next {
            "details expanded".to_string()
        } else {
            "details folded".to_string()
        });
    }

    fn reveal_raw_at_cursor(&mut self) {
        let raw_idx = self
            .raw_line_at_rendered_cursor()
            .unwrap_or(usize::from(self.active().scroll));
        let Some(line) = self.active_mut().source.lines().nth(raw_idx) else {
            self.status_message = Some("raw: <end of file>".to_string());
            return;
        };
        self.status_message = Some(format!(
            "raw: {}",
            truncate_plain(super::layout::sanitize(line).as_ref(), 68)
        ));
    }

    fn raw_line_at_rendered_cursor(&self) -> Option<usize> {
        let line = usize::from(self.active().scroll);
        let (target_idx, _) =
            self.active()
                .block_spans
                .iter()
                .enumerate()
                .find(|(_, (start, height))| {
                    let end = start.saturating_add(*height).max(start.saturating_add(1));
                    line >= *start && line < end
                })?;
        let mut start_line = 0;
        for (idx, block) in self
            .active()
            .doc
            .blocks
            .iter()
            .enumerate()
            .take(target_idx + 1)
        {
            let found = source_line_for_block(block, &self.active().source, start_line)?;
            if idx == target_idx {
                return Some(found);
            }
            start_line = found.saturating_add(1);
        }
        None
    }

    fn open_editor(&mut self) {
        let Some(path) = self.active_mut().path.clone() else {
            self.status_message = Some("no local file for editor".to_string());
            return;
        };
        let Some((program, args)) = editor_command() else {
            self.status_message = Some("set EDITOR to edit this file".to_string());
            return;
        };
        match run_editor(&program, &args, &path) {
            Ok(status) => {
                self.status_message = Some(format!(
                    "editor {} {}",
                    if status.success() { "saved" } else { "exited" },
                    truncate_plain(&path.display().to_string(), 42),
                ));
                self.reload();
            }
            Err(err) => {
                self.status_message = Some(format!(
                    "editor failed: {}",
                    truncate_plain(&err.to_string(), 42)
                ));
            }
        }
    }

    fn activate_link_at(&mut self, line: usize, col: u16) -> bool {
        let Some(target) = self.link_at(line, col) else {
            return false;
        };
        self.activate_target(target);
        true
    }

    fn link_at(&self, line: usize, col: u16) -> Option<LinkTarget> {
        self.active()
            .link_regions
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
                } else if let Some((url, anchor)) = self.remote_markdown_target(&url) {
                    self.open_remote_doc(&url, anchor);
                } else {
                    self.open_url(&url);
                }
            }
        }
    }

    fn open_url(&mut self, url: &str) {
        let label = truncate_plain(super::layout::sanitize(url).as_ref(), 54);
        match open_target(url, self.active_mut().base_dir.as_deref()) {
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

    fn start_global_search(&mut self) {
        self.mode = Mode::GlobalSearch;
        self.global_query.clear();
    }

    fn global_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.global_query.clear();
            }
            KeyCode::Enter => {
                self.run_global_search();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.global_query.pop();
            }
            KeyCode::Char(c) => self.global_query.push(c),
            _ => {}
        }
    }

    fn run_global_search(&mut self) {
        let query = self.global_query.trim().to_string();
        if query.is_empty() {
            self.status_message = Some("empty workspace search".to_string());
            return;
        }
        let root = self.browser_root_or_default();
        self.browser_root = Some(root.clone());
        self.browser_entries = global_search_entries(&root, &query);
        self.browser_state
            .select((!self.browser_entries.is_empty()).then_some(0));
        self.browser_visible = true;
        self.focus = Focus::Browser;
        self.status_message = if self.browser_entries.is_empty() {
            Some(format!("no workspace matches for {query:?}"))
        } else {
            Some(format!("{} workspace matches", self.browser_entries.len()))
        };
    }

    fn search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.active_mut().search_query.clear();
            }
            KeyCode::Enter => {
                self.run_search();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.active_mut().search_query.pop();
            }
            KeyCode::Char(c) => self.active_mut().search_query.push(c),
            _ => {}
        }
    }

    fn run_search(&mut self) {
        self.active_mut().match_idx = 0;
        if self.active_mut().search_query.is_empty() {
            self.active_mut().matches.clear();
            return;
        }
        let needle = self.active_mut().search_query.to_lowercase();
        self.active_mut().matches = self
            .active()
            .content
            .lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                text.to_lowercase().contains(&needle).then_some(idx)
            })
            .collect();
        if !self.active_mut().matches.is_empty() {
            self.set_scroll(u16::try_from(self.active().matches[0]).unwrap_or(0));
        }
    }

    fn jump_match(&mut self, forward: bool) {
        if self.active_mut().matches.is_empty() {
            return;
        }
        let len = self.active_mut().matches.len();
        self.active_mut().match_idx = if forward {
            (self.active().match_idx + 1) % len
        } else {
            (self.active().match_idx + len - 1) % len
        };
        let line = self.active().matches[self.active().match_idx];
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
        self.active_mut().theme_dirty = true;
        self.active_mut().images.clear_generated();
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

    // ─── Bookmarks ───────────────────────────────────────────────

    fn open_bookmarks(&mut self) {
        if self.bookmarks.is_empty() {
            self.status_message = Some("no bookmarks configured".to_string());
            return;
        }
        self.show_bookmarks = true;
        self.bookmark_state.select(Some(0));
    }

    fn bookmark_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.show_bookmarks = false,
            KeyCode::Enter => self.open_bookmark_selection(),
            KeyCode::Char('j') | KeyCode::Down => self.bookmark_step(true),
            KeyCode::Char('k') | KeyCode::Up => self.bookmark_step(false),
            _ => {}
        }
    }

    fn bookmark_step(&mut self, forward: bool) {
        if self.bookmarks.is_empty() {
            return;
        }
        let len = self.bookmarks.len();
        let cur = self.bookmark_state.selected().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.bookmark_state.select(Some(next));
    }

    fn bookmark_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.bookmark_step(true),
            MouseEventKind::ScrollUp => self.bookmark_step(false),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(idx) = self.bookmark_index_at(mouse.column, mouse.row) {
                    self.bookmark_state.select(Some(idx));
                    self.open_bookmark_selection();
                }
            }
            _ => {}
        }
    }

    fn bookmark_index_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.bookmark_area;
        if !contains(area, column, row)
            || row <= area.y
            || row >= area.y.saturating_add(area.height).saturating_sub(1)
        {
            return None;
        }
        let visible_idx = usize::from(row.saturating_sub(area.y).saturating_sub(1));
        let idx = self.bookmark_state.offset().saturating_add(visible_idx);
        (idx < self.bookmarks.len()).then_some(idx)
    }

    fn open_bookmark_selection(&mut self) {
        let Some(idx) = self.bookmark_state.selected() else {
            return;
        };
        let Some(bookmark) = self.bookmarks.get(idx).cloned() else {
            return;
        };
        self.show_bookmarks = false;
        if bookmark.path.is_dir() {
            self.browser_root = Some(bookmark.path);
            self.browser_visible = true;
            self.refresh_browser();
            self.focus = Focus::Browser;
            self.status_message = Some(format!("bookmark {}", truncate_plain(&bookmark.name, 32)));
        } else if is_markdown_path(&bookmark.path) {
            self.open_path_in_tab(&bookmark.path);
        } else {
            self.status_message = Some(format!(
                "bookmark target unavailable: {}",
                truncate_plain(&bookmark.path.display().to_string(), 40)
            ));
        }
    }
}

// ─── Free helpers ────────────────────────────────────────────────

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

fn session_path_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn details_view(doc: &RenderedDoc, states: &BTreeMap<usize, bool>) -> RenderedDoc {
    let mut doc = doc.clone();
    for (idx, block) in doc.blocks.iter_mut().enumerate() {
        if let Block::Details { open, .. } = block
            && let Some(state) = states.get(&idx)
        {
            *open = *state;
        }
    }
    doc
}

fn editor_command() -> Option<(String, Vec<String>)> {
    let raw = env::var("VISUAL")
        .ok()
        .or_else(|| env::var("EDITOR").ok())?;
    let mut parts = raw.split_whitespace();
    let program = parts.next()?.to_string();
    let args = parts.map(str::to_string).collect();
    Some((program, args))
}

fn run_editor(program: &str, args: &[String], path: &Path) -> io::Result<ExitStatus> {
    {
        let mut stdout = io::stdout();
        ratatui::crossterm::execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
    }
    ratatui::crossterm::terminal::disable_raw_mode()?;
    let status = ProcessCommand::new(program).args(args).arg(path).status();
    ratatui::crossterm::terminal::enable_raw_mode()?;
    {
        let mut stdout = io::stdout();
        ratatui::crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    }
    status
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
    use crate::render::terminal::config::{
        ReaderConfig, ReaderSession, ReaderSettings, SessionTab, UserConfig,
    };
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
        assert_eq!(app.active().doc.outline.len(), 1);
        std::fs::write(&path, "# One\n\n## Two\n").expect("rewrite");
        app.reload();
        assert_eq!(
            app.active().doc.outline.len(),
            2,
            "reload should pick up the new heading"
        );
    }

    #[test]
    fn reload_keeps_csv_files_as_tables() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "name,count\nalpha,1\n").expect("write");
        let mut app = App::new(
            "```csv\nname,count\nalpha,1\n```\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(path.clone()),
        );

        std::fs::write(&path, "name,count\nbeta,2\n").expect("rewrite");
        app.reload();

        let Some(Block::Table(table)) = app.active().doc.blocks.first() else {
            panic!("expected csv table: {:?}", app.active().doc.blocks);
        };
        assert_eq!(table.rows[0][0][0].text, "beta");
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
        assert_eq!(app.active().title, "Beta");
        assert_eq!(app.active().back.len(), 1);
        assert!(app.active().forward.is_empty());

        // Back returns to the first document; forward replays the jump.
        app.go_back();
        assert_eq!(app.active().title, "Alpha");
        assert_eq!(app.active().forward.len(), 1);
        app.go_forward();
        assert_eq!(app.active().title, "Beta");
    }

    #[test]
    fn remote_markdown_links_follow_only_same_origin_markdown() {
        let origin_url =
            Url::parse("https://raw.githubusercontent.com/o/r/HEAD/README.md").expect("url");
        let app = App::new_with_settings_and_origin(
            "# Remote\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            Some(DocumentOrigin::remote(origin_url)),
            ReaderSettings {
                reader: ReaderConfig::default(),
                user: UserConfig::default(),
            },
        );

        let (target, anchor) = app
            .remote_markdown_target(
                "https://raw.githubusercontent.com/o/r/HEAD/docs/guide.md#intro",
            )
            .expect("remote markdown target");

        assert_eq!(
            target.as_str(),
            "https://raw.githubusercontent.com/o/r/HEAD/docs/guide.md"
        );
        assert_eq!(anchor.as_deref(), Some("intro"));
        assert!(
            app.remote_markdown_target("https://example.com/o/r/HEAD/docs/guide.md")
                .is_none()
        );
        assert!(
            app.remote_markdown_target("https://raw.githubusercontent.com/o/r/HEAD/logo.png")
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn link_navigation_jail_blocks_escapes() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().expect("tempdir");
        let doc_dir = root.path().join("docs");
        let secret_dir = root.path().join("secret");
        std::fs::create_dir_all(&doc_dir).expect("doc dir");
        std::fs::create_dir_all(&secret_dir).expect("secret dir");
        std::fs::write(secret_dir.join("leak.md"), "# Secret\n").expect("secret");
        let a = doc_dir.join("a.md");
        std::fs::write(&a, "# A\n").expect("a");
        // A .md symlink inside the doc dir pointing at a file outside it.
        symlink(secret_dir.join("leak.md"), doc_dir.join("escape.md")).expect("symlink");

        let theme = load_theme_or_default("silk-light");
        let app = App::new(
            "# A\n",
            theme,
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(doc_dir.clone()),
            Some(a),
        );

        // canonicalize resolves the symlink outside the jail, so even a .md
        // target must be refused; absolute paths and traversal too.
        assert!(app.local_markdown_target("escape.md").is_none());
        assert!(app.local_markdown_target("/etc/hosts").is_none());
        assert!(app.local_markdown_target("../secret/leak.md").is_none());
    }

    #[test]
    fn outline_and_offsets_populated() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        assert_eq!(app.active().doc.outline.len(), 2, "two headings expected");
        assert_eq!(
            app.active().block_spans.len(),
            app.active().doc.blocks.len()
        );
    }

    #[test]
    fn scroll_clamps_to_content() {
        let mut app = sample();
        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        app.scroll_by(10_000);
        assert!(app.active().scroll <= app.max_scroll());
    }

    #[test]
    fn search_finds_matches() {
        let mut app = sample();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        app.active_mut().search_query = "section".to_string();
        app.run_search();
        assert!(
            !app.active().matches.is_empty(),
            "should find 'section' heading"
        );
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
            app.active().content_bg,
            Color::Reset,
            "light theme resolves a page bg"
        );
        let leaked = app
            .active()
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
        assert!(app.active().theme_dirty);
        terminal.draw(|f| app.draw(f)).expect("redraw");
        assert!(!app.active().theme_dirty, "redraw should re-render content");
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
    fn user_keybindings_add_tui_actions() {
        let mut user = UserConfig::default();
        user.keybindings
            .insert("heading_next".to_string(), "J".to_string());
        user.keybindings
            .insert("quit".to_string(), "ctrl-x".to_string());
        let mut app = App::new_with_settings(
            "# Top\n\none\n\n## Next\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderSettings {
                reader: ReaderConfig::default(),
                user,
            },
        );
        let backend = TestBackend::new(100, 8);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");

        app.normal_key(KeyCode::Char('J'), KeyModifiers::NONE);
        assert_eq!(app.active().outline_state.selected(), Some(1));

        app.normal_key(KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(app.quit);
    }

    #[test]
    fn tabs_switch_and_close_active_document_state() {
        let mut app = sample();
        app.tabs.push(TabState::from_body(
            "# Second\n\nBody\n",
            None,
            None,
            None,
            None,
        ));

        app.next_tab();
        assert_eq!(app.active_tab, 1);
        assert_eq!(app.active().title, "Second");

        app.prev_tab();
        assert_eq!(app.active_tab, 0);

        app.next_tab();
        app.close_tab();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab, 0);
        assert_ne!(app.active().title, "Second");
    }

    #[test]
    fn reader_session_records_local_tabs_and_active_scroll() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        std::fs::write(&a, "# A\n").expect("a");
        std::fs::write(&b, "# B\n").expect("b");
        let mut app = App::new(
            "# A\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            Some(Picker::halfblocks()),
            Some(dir.path().to_path_buf()),
            Some(a.clone()),
        );
        app.tabs.push(TabState::from_body(
            "# B\n",
            None,
            Some(dir.path().to_path_buf()),
            Some(b.clone()),
            Some(DocumentOrigin::local(b.clone())),
        ));
        app.tabs[0].scroll = 3;
        app.tabs[1].scroll = 7;
        app.active_tab = 1;

        let session = app.reader_session();

        assert_eq!(session.active_tab, 1);
        assert_eq!(session.tabs.len(), 2);
        assert_eq!(session.tabs[0].path, a);
        assert_eq!(session.tabs[0].scroll, 3);
        assert_eq!(session.tabs[1].path, b);
        assert_eq!(session.tabs[1].scroll, 7);
    }

    #[test]
    fn restore_session_tabs_recovers_saved_active_tab() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        std::fs::write(&a, "# A\n").expect("a");
        std::fs::write(&b, "# B\n").expect("b");
        let mut app = App::new(
            "# A\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            Some(Picker::halfblocks()),
            Some(dir.path().to_path_buf()),
            Some(a.clone()),
        );

        let session = ReaderSession {
            active_tab: 1,
            tabs: vec![
                SessionTab {
                    path: a.clone(),
                    scroll: 2,
                },
                SessionTab {
                    path: b.clone(),
                    scroll: 9,
                },
            ],
        };
        app.restore_session_tabs(&session, Some(a.as_path()));

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);
        assert_eq!(app.active().title, "B");
        assert_eq!(app.active().scroll, 9);
        assert!(app.active().images.enabled());
    }

    #[test]
    fn restore_session_tabs_keeps_new_requested_file_active() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let saved = dir.path().join("saved.md");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(&saved, "# Saved\n").expect("saved");
        let mut app = App::new(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current.clone()),
        );

        let session = ReaderSession {
            active_tab: 0,
            tabs: vec![SessionTab {
                path: saved,
                scroll: 4,
            }],
        };
        app.restore_session_tabs(&session, Some(current.as_path()));

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 0);
        assert_eq!(app.active().title, "Current");
        assert_eq!(app.tabs[1].title, "Saved");
    }

    #[test]
    fn browser_entries_list_directories_then_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("docs")).expect("docs");
        std::fs::write(dir.path().join("b.md"), "# B\n").expect("b");
        std::fs::write(dir.path().join("a.txt"), "ignored").expect("txt");

        let entries = browser_entries(dir.path());

        let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
        assert!(labels.contains(&"docs/"));
        assert!(labels.contains(&"b.md"));
        assert!(!labels.contains(&"a.txt"));
        let dir_pos = labels
            .iter()
            .position(|label| *label == "docs/")
            .expect("docs");
        let file_pos = labels.iter().position(|label| *label == "b.md").expect("b");
        assert!(dir_pos < file_pos);
    }

    #[test]
    fn browser_opens_markdown_files_in_new_tabs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let other = dir.path().join("other.md");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(&other, "# Other\n").expect("other");
        let mut app = App::new(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            Some(Picker::halfblocks()),
            Some(dir.path().to_path_buf()),
            Some(current),
        );

        app.toggle_browser();
        let idx = app
            .browser_entries
            .iter()
            .position(|entry| entry.path == other)
            .expect("other entry");
        app.browser_state.select(Some(idx));
        app.open_browser_selection();

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);
        assert_eq!(app.active().title, "Other");
        assert!(app.active().images.enabled());
    }

    #[test]
    fn browser_enters_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let subdir = dir.path().join("docs");
        std::fs::create_dir(&subdir).expect("docs");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(subdir.join("guide.md"), "# Guide\n").expect("guide");
        let mut app = App::new(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current),
        );

        app.toggle_browser();
        let idx = app
            .browser_entries
            .iter()
            .position(|entry| entry.path == subdir)
            .expect("docs entry");
        app.browser_state.select(Some(idx));
        app.open_browser_selection();

        assert_eq!(app.browser_root.as_deref(), Some(subdir.as_path()));
        assert!(
            app.browser_entries
                .iter()
                .any(|entry| entry.label == "guide.md")
        );
    }

    #[test]
    fn global_search_entries_find_nested_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).expect("docs");
        std::fs::write(dir.path().join("root.md"), "needle root\n").expect("root");
        std::fs::write(docs.join("guide.md"), "nope\nNeedle nested\n").expect("guide");
        std::fs::write(docs.join("ignore.txt"), "needle txt\n").expect("txt");

        let entries = global_search_entries(dir.path(), "needle");

        let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
        assert!(labels.iter().any(|label| label.starts_with("root.md:1")));
        assert!(
            labels
                .iter()
                .any(|label| label.starts_with("docs/guide.md:2"))
        );
        assert!(!labels.iter().any(|label| label.contains("ignore.txt")));
    }

    #[test]
    fn global_search_opens_result_in_new_tab() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let other = dir.path().join("other.md");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(&other, "# Other\n\nneedle here\n").expect("other");
        let mut app = App::new(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current),
        );

        app.start_global_search();
        app.global_query = "needle".to_string();
        app.run_global_search();
        let idx = app
            .browser_entries
            .iter()
            .position(|entry| entry.path == other)
            .expect("other result");
        app.browser_state.select(Some(idx));
        app.open_browser_selection();

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active().title, "Other");
        assert_eq!(app.active().search_query, "needle");
        assert!(!app.active().matches.is_empty());
    }

    #[test]
    fn global_search_opens_selected_result_occurrence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let other = dir.path().join("other.md");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(
            &other,
            "# Other\n\nneedle first\n\nmiddle\n\nneedle second\n",
        )
        .expect("other");
        let mut app = App::new(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current),
        );

        app.start_global_search();
        app.global_query = "needle".to_string();
        app.run_global_search();
        let idx = app
            .browser_entries
            .iter()
            .position(|entry| {
                entry.path == other
                    && matches!(entry.kind, BrowserEntryKind::SearchResult { line: 7 })
            })
            .expect("second result");
        app.browser_state.select(Some(idx));
        app.open_browser_selection();

        assert_eq!(app.active().title, "Other");
        assert_eq!(app.active().match_idx, 1);
        assert_eq!(usize::from(app.active().scroll), app.active().matches[1]);
    }

    #[test]
    fn bookmark_picker_opens_configured_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let docs = dir.path().join("docs");
        std::fs::create_dir(&docs).expect("docs");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(docs.join("guide.md"), "# Guide\n").expect("guide");
        let mut user = UserConfig::default();
        user.bookmarks
            .insert("docs".to_string(), docs.display().to_string());
        let mut app = App::new_with_settings(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current),
            ReaderSettings {
                reader: ReaderConfig::default(),
                user,
            },
        );

        app.open_bookmarks();
        app.open_bookmark_selection();

        assert!(app.browser_visible);
        assert_eq!(app.browser_root.as_deref(), Some(docs.as_path()));
        assert!(
            app.browser_entries
                .iter()
                .any(|entry| entry.label == "guide.md")
        );
    }

    #[test]
    fn bookmark_picker_opens_configured_markdown_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let current = dir.path().join("current.md");
        let saved = dir.path().join("saved.md");
        std::fs::write(&current, "# Current\n").expect("current");
        std::fs::write(&saved, "# Saved\n").expect("saved");
        let mut user = UserConfig::default();
        user.bookmarks
            .insert("saved".to_string(), saved.display().to_string());
        let mut app = App::new_with_settings(
            "# Current\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            Some(dir.path().to_path_buf()),
            Some(current),
            ReaderSettings {
                reader: ReaderConfig::default(),
                user,
            },
        );

        app.open_bookmarks();
        app.open_bookmark_selection();

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active().title, "Saved");
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

        let region = app.active()
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
        let region = app.active()
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
        assert!(app.active().scroll > 0);
    }

    #[test]
    fn bracket_chords_jump_between_headings() {
        let mut app = App::new_with_config(
            "# Top\n\none\n\ntwo\n\n## Middle\n\nthree\n\n## End\n",
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

        app.normal_key(KeyCode::Char(']'), KeyModifiers::NONE);
        app.normal_key(KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(app.active().outline_state.selected(), Some(1));

        app.normal_key(KeyCode::Char('['), KeyModifiers::NONE);
        app.normal_key(KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(app.active().outline_state.selected(), Some(0));
    }

    #[test]
    fn mouse_hover_previews_links_without_following() {
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
        let region = app.active()
            .link_regions
            .iter()
            .find(|region| matches!(&region.target, LinkTarget::Url(url) if url == "https://example.com"))
            .cloned()
            .expect("link region");

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: app.content_area.x.saturating_add(region.start),
            row: app
                .content_area
                .y
                .saturating_add(u16::try_from(region.line).unwrap_or(0)),
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            app.status_message.as_deref(),
            Some("link: https://example.com")
        );
        assert_eq!(app.active().title, "Title");

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: app.content_area.x,
            row: app.content_area.y,
            modifiers: KeyModifiers::NONE,
        });
        assert!(app.status_message.is_none());
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
        let before = app.active().scroll;

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: app.content_area.x.saturating_add(1),
            row: app.content_area.y.saturating_add(1),
            modifiers: KeyModifiers::NONE,
        });

        assert!(app.active().scroll > before);
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
            .active()
            .image_placements
            .iter()
            .find(|placement| placement.src == "big.png")
            .cloned()
            .expect("image placement");
        let first_rows = app
            .active()
            .images
            .pending_rows_for(&placement.src, placement.line);
        let first_visible_bottom =
            u32::from(app.active().scroll) + u32::from(app.content_area.height);
        assert!(
            first_rows
                .iter()
                .any(|row| u32::from(placement.line) + u32::from(*row) >= first_visible_bottom),
            "first draw should prefetch beyond visible rows"
        );

        app.set_scroll(app.max_scroll());
        terminal.draw(|f| app.draw(f)).expect("redraw");
        let rows = app
            .active()
            .images
            .pending_rows_for(&placement.src, placement.line);
        let scroll = u32::from(app.active().scroll);
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

    #[test]
    fn display_math_reserves_generated_image_band() {
        let mut app = App::new_with_config(
            "# Title\n\n```math\nE = m c^2\n```\n\nAfter\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            Some(Picker::halfblocks()),
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");

        let placement = app
            .active()
            .image_placements
            .iter()
            .find(|placement| placement.src.starts_with("\u{0}math:"))
            .expect("math placement");
        assert!(placement.rows > 0);
    }

    #[test]
    fn details_blocks_fold_and_expand() {
        let mut app = App::new_with_config(
            "<details><summary>More</summary><p>Hidden body</p></details>\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");

        let collapsed = app
            .active()
            .content
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(collapsed.contains("More"));
        assert!(!collapsed.contains("Hidden body"));

        app.toggle_details_at_cursor();
        terminal.draw(|f| app.draw(f)).expect("redraw");
        let expanded = app
            .active()
            .content
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(expanded.contains("Hidden body"));
    }

    #[test]
    fn reveal_raw_shows_source_line() {
        let mut app = App::new_with_config(
            "# Title\n\nraw body\n",
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        app.active_mut().scroll = 2;

        app.reveal_raw_at_cursor();

        assert_eq!(app.status_message.as_deref(), Some("raw: raw body"));
    }

    #[test]
    fn reveal_raw_uses_block_source_line_for_wrapped_content() {
        let body = "# Title\n\nthis paragraph is deliberately long enough to wrap inside a narrow test terminal\n";
        let mut app = App::new_with_config(
            body,
            load_theme_or_default("silk-light"),
            "silk-light",
            Some(GlyphTier::Unicode),
            None,
            None,
            None,
            ReaderConfig::default(),
        );
        let backend = TestBackend::new(34, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|f| app.draw(f)).expect("draw");
        let (start, height) = app.active().block_spans[1];
        assert!(height > 1, "paragraph should wrap");
        app.active_mut().scroll = u16::try_from(start + 1).expect("scroll");

        app.reveal_raw_at_cursor();

        assert!(matches!(
            app.status_message.as_deref(),
            Some(message)
                if message.starts_with(
                    "raw: this paragraph is deliberately long enough to wrap"
                )
        ));
    }

    #[test]
    fn selected_text_extracts_multiline_plain_content() {
        let lines = vec![
            Line::from("alpha beta"),
            Line::from("gamma delta"),
            Line::from("omega"),
        ];

        let selected = selected_text(&lines, (0, 6), (1, 5)).expect("selection");

        assert_eq!(selected, "beta\ngamma");
    }

    #[test]
    fn osc52_base64_encoding_matches_spec() {
        assert_eq!(base64_encode(b"silk"), "c2lsaw==");
    }

    fn content_span<'a>(app: &'a App, needle: &str) -> &'a Span<'static> {
        app.active()
            .content
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content.contains(needle))
            .unwrap_or_else(|| panic!("missing content span {needle:?}"))
    }
}
