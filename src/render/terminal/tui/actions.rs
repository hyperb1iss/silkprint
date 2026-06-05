use std::collections::BTreeMap;

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Quit,
    Help,
    Theme,
    ToggleOutline,
    ToggleBrowser,
    Search,
    GlobalSearch,
    Bookmarks,
    ToggleDetails,
    RevealRaw,
    Edit,
    ToggleFocus,
    NextMatch,
    PrevMatch,
    Back,
    Forward,
    Top,
    Bottom,
    HalfDown,
    HalfUp,
    PageDown,
    PageUp,
    Down,
    Up,
    HeadingNext,
    HeadingPrev,
    TabNext,
    TabPrev,
    TabClose,
}

#[derive(Clone, Copy)]
struct KeyChord {
    code: KeyCode,
    mods: KeyModifiers,
}

#[derive(Default)]
pub(super) struct KeyBindings(Vec<(KeyChord, Action)>);

impl KeyBindings {
    pub(super) fn from_config(config: &BTreeMap<String, String>) -> Self {
        let bindings = config
            .iter()
            .filter_map(|(action, key)| Some((parse_key_chord(key)?, parse_action(action)?)))
            .collect();
        Self(bindings)
    }

    pub(super) fn action_for(&self, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
        self.0
            .iter()
            .find(|(chord, _)| chord.code == code && chord.mods == mods)
            .map(|(_, action)| *action)
    }
}

fn parse_action(action: &str) -> Option<Action> {
    match action.trim().replace('-', "_").as_str() {
        "quit" => Some(Action::Quit),
        "help" => Some(Action::Help),
        "theme" | "theme_picker" => Some(Action::Theme),
        "browser" | "file_browser" | "toggle_browser" => Some(Action::ToggleBrowser),
        "outline" | "toggle_outline" => Some(Action::ToggleOutline),
        "search" => Some(Action::Search),
        "global_search" | "workspace_search" | "search_all" => Some(Action::GlobalSearch),
        "bookmarks" | "bookmark_picker" => Some(Action::Bookmarks),
        "details" | "toggle_details" | "fold_details" => Some(Action::ToggleDetails),
        "raw" | "reveal_raw" => Some(Action::RevealRaw),
        "edit" | "editor" | "open_editor" => Some(Action::Edit),
        "focus" | "toggle_focus" => Some(Action::ToggleFocus),
        "next_match" => Some(Action::NextMatch),
        "prev_match" | "previous_match" => Some(Action::PrevMatch),
        "back" => Some(Action::Back),
        "forward" => Some(Action::Forward),
        "top" => Some(Action::Top),
        "bottom" => Some(Action::Bottom),
        "half_down" => Some(Action::HalfDown),
        "half_up" => Some(Action::HalfUp),
        "page_down" => Some(Action::PageDown),
        "page_up" => Some(Action::PageUp),
        "down" | "scroll_down" => Some(Action::Down),
        "up" | "scroll_up" => Some(Action::Up),
        "heading_next" | "next_heading" => Some(Action::HeadingNext),
        "heading_prev" | "heading_previous" | "prev_heading" | "previous_heading" => {
            Some(Action::HeadingPrev)
        }
        "tab_next" | "next_tab" => Some(Action::TabNext),
        "tab_prev" | "tab_previous" | "prev_tab" | "previous_tab" => Some(Action::TabPrev),
        "tab_close" | "close_tab" => Some(Action::TabClose),
        _ => None,
    }
}

fn parse_key_chord(key: &str) -> Option<KeyChord> {
    let trimmed = key.trim();
    if trimmed.is_empty() || matches!(trimmed, "[[" | "]]" | "gg") {
        return None;
    }
    let mut mods = KeyModifiers::NONE;
    let mut rest = trimmed;
    loop {
        let lower = rest.to_ascii_lowercase();
        if let Some(next) = lower
            .strip_prefix("ctrl-")
            .or_else(|| lower.strip_prefix("ctrl+"))
        {
            mods |= KeyModifiers::CONTROL;
            rest = &rest[rest.len() - next.len()..];
        } else if let Some(next) = lower
            .strip_prefix("alt-")
            .or_else(|| lower.strip_prefix("alt+"))
        {
            mods |= KeyModifiers::ALT;
            rest = &rest[rest.len() - next.len()..];
        } else if let Some(next) = lower
            .strip_prefix("shift-")
            .or_else(|| lower.strip_prefix("shift+"))
        {
            mods |= KeyModifiers::SHIFT;
            rest = &rest[rest.len() - next.len()..];
        } else {
            break;
        }
    }
    let lower = rest.to_ascii_lowercase();
    let code = match lower.as_str() {
        "space" => KeyCode::Char(' '),
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "page_up" => KeyCode::PageUp,
        "pagedown" | "page_down" => KeyCode::PageDown,
        _ => {
            let mut chars = rest.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(ch)
        }
    };
    Some(KeyChord { code, mods })
}
