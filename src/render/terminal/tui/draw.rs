use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block as WBlock, Borders, Clear, List, ListItem, Paragraph};
use ratatui_image::StatefulImage;

use crate::render::terminal::layout::sanitize;

use super::bands::visible_band_rows;
use super::browser::BrowserEntryKind;
use super::text::{highlight_line, search_highlight_style, truncate_plain};
use super::{
    App, BROWSER_WIDTH, Focus, IMAGE_PREFETCH_MIN_ROWS, Mode, OUTLINE_WIDTH, centered_rect, images,
};

impl App {
    pub(super) fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let [body, status] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
        self.status_area = status;

        let body = if self.browser_visible {
            let [browser, rest] =
                Layout::horizontal([Constraint::Length(BROWSER_WIDTH), Constraint::Min(10)])
                    .areas(body);
            self.browser_area = Some(browser);
            self.draw_browser(frame, browser);
            rest
        } else {
            self.browser_area = None;
            body
        };

        let content_area = if self.outline_visible && !self.active().doc.outline.is_empty() {
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

        self.active_mut().viewport_h = content_area.height;
        self.ensure_content(content_area.width);
        if let Some(anchor) = self.active_mut().pending_anchor.take() {
            self.jump_to_anchor(&anchor);
        }
        self.clamp_scroll();
        self.draw_content(frame, content_area);
        self.draw_status(frame, status);

        if self.show_picker {
            self.draw_picker(frame, area);
        }
        if self.show_bookmarks {
            self.draw_bookmarks(frame, area);
        }
        if self.show_help {
            self.draw_help(frame, area);
        }
    }

    fn draw_content(&mut self, frame: &mut Frame, area: Rect) {
        let top = usize::from(self.active().scroll);
        let total = self.active().content.lines.len();
        let end = top.saturating_add(usize::from(area.height)).min(total);
        let mut visible: Vec<Line<'static>> = if top < total {
            self.active().content.lines[top..end].to_vec()
        } else {
            Vec::new()
        };
        if !self.active_mut().search_query.is_empty() {
            let needle: Vec<char> = self
                .active_mut()
                .search_query
                .to_lowercase()
                .chars()
                .collect();
            let hl = search_highlight_style();
            for line in &mut visible {
                *line = highlight_line(line, &needle, hl);
            }
        }
        let para = Paragraph::new(Text::from(visible)).style(
            Style::default()
                .fg(self.active().content_fg)
                .bg(self.active().content_bg),
        );
        frame.render_widget(para, area);

        let current_scroll = self.active().scroll;
        self.active_mut().images.begin_frame(images::ImageView {
            scroll: current_scroll,
            height: area.height,
            width: area.width.saturating_sub(2),
        });

        let scroll = u32::from(current_scroll);
        let viewport = u32::from(area.height);
        let prefetch = viewport
            .saturating_mul(2)
            .max(u32::from(IMAGE_PREFETCH_MIN_ROWS));
        let prefetch_top = scroll.saturating_sub(prefetch);
        let prefetch_bottom = scroll.saturating_add(viewport).saturating_add(prefetch);
        let placements = std::mem::take(&mut self.active_mut().image_placements);
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
                    if let Some(proto) = self.active_mut().images.row_protocol(
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
                self.active_mut().images.prefetch_row(
                    &placement.src,
                    placement.line,
                    row,
                    placement.rows,
                    tile_width,
                );
            }
        }
        self.active_mut().image_placements = placements;
        self.active_mut().images.finish_frame();
    }

    fn draw_browser(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .browser_entries
            .iter()
            .map(|entry| {
                let icon = match entry.kind {
                    BrowserEntryKind::Parent => "..",
                    BrowserEntryKind::Directory => "d",
                    BrowserEntryKind::Markdown => "m",
                    BrowserEntryKind::SearchResult { .. } => "s",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(self.chrome.accent)),
                    Span::styled(
                        sanitize(&entry.label).into_owned(),
                        Style::default().fg(self.chrome.text),
                    ),
                ]))
            })
            .collect();

        let border = if self.focus == Focus::Browser {
            self.chrome.border_focused
        } else {
            self.chrome.border
        };
        let title = self
            .browser_root
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Files");
        let title = if self
            .browser_entries
            .iter()
            .any(|entry| matches!(entry.kind, BrowserEntryKind::SearchResult { .. }))
        {
            format!(" Search {} ", truncate_plain(&self.global_query, 18))
        } else {
            format!(" {} ", truncate_plain(title, 22))
        };
        let list = List::new(items)
            .block(
                WBlock::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border))
                    .title(Span::styled(
                        title,
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
        frame.render_stateful_widget(list, area, &mut self.browser_state);
    }

    fn draw_outline(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .active()
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
                        sanitize(&item.title).into_owned(),
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
        frame.render_stateful_widget(list, area, &mut self.active_mut().outline_state);
    }

    fn draw_status(&mut self, frame: &mut Frame, area: Rect) {
        const BAR_W: usize = 10;
        let max = self.max_scroll();
        let pct: u16 = if max == 0 {
            100
        } else {
            u16::try_from(u32::from(self.active().scroll) * 100 / u32::from(max)).unwrap_or(100)
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
            sanitize(message).into_owned()
        } else if self.mode == Mode::Search {
            format!("/{}", sanitize(&self.active().search_query))
        } else if self.mode == Mode::GlobalSearch {
            format!("S {}", sanitize(&self.global_query))
        } else if !self.active_mut().matches.is_empty() {
            format!(
                "match {}/{}  /search ?help t theme o outline q quit",
                self.active().match_idx + 1,
                self.active_mut().matches.len()
            )
        } else {
            "j/k scroll  /search S all  e files B marks  z fold r raw  ?help".to_string()
        };

        let accent = Style::default().fg(self.chrome.accent);
        let muted = Style::default().fg(self.chrome.muted);
        let tab_label = format!(" [{}/{}] ", self.active_tab + 1, self.tabs.len());
        let left = Line::from(vec![
            Span::styled(
                format!(" {} ", self.glyphs.diamond()),
                accent.add_modifier(Modifier::BOLD),
            ),
            Span::styled(tab_label, muted),
            Span::styled(
                truncate_plain(&self.active().title, 28),
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

    fn draw_bookmarks(&mut self, frame: &mut Frame, area: Rect) {
        let popup = centered_rect(58, 60, area);
        self.bookmark_area = popup;
        frame.render_widget(Clear, popup);
        let items: Vec<ListItem> = self
            .bookmarks
            .iter()
            .map(|bookmark| {
                let path = truncate_plain(&bookmark.path.display().to_string(), 42);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{}  ", sanitize(&bookmark.name)),
                        Style::default().fg(self.chrome.accent),
                    ),
                    Span::styled(
                        sanitize(&path).into_owned(),
                        Style::default().fg(self.chrome.text),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(
                WBlock::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.chrome.border_focused))
                    .title(Span::styled(
                        " Bookmarks  (Enter open · Esc cancel) ",
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
        frame.render_stateful_widget(list, popup, &mut self.bookmark_state);
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let popup = centered_rect(54, 60, area);
        frame.render_widget(Clear, popup);
        let rows = [
            ("j / k, ↑ / ↓", "scroll line"),
            ("Ctrl-d / Ctrl-u", "half page"),
            ("Space / PageDn", "page down"),
            ("g g / G", "top / bottom"),
            ("[[ / ]]", "prev / next heading"),
            ("e", "file browser"),
            ("S", "workspace search"),
            ("B", "bookmarks"),
            ("z", "fold details"),
            ("r", "reveal raw"),
            ("E", "open $EDITOR"),
            ("drag", "copy selection"),
            ("o", "toggle outline"),
            ("Tab", "switch focus"),
            ("Enter (outline)", "jump to heading"),
            ("hover / click link", "preview / follow"),
            ("H / L, x", "prev / next / close tab"),
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
