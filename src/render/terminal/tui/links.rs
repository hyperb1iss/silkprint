use std::ffi::OsString;
use std::path::{Path, PathBuf};

use unicode_width::UnicodeWidthChar;

use crate::render::terminal::model::LinkTarget;

use super::text::truncate_plain;

#[derive(Clone)]
pub(super) struct LinkRegion {
    pub(super) line: usize,
    pub(super) start: u16,
    pub(super) end: u16,
    pub(super) target: LinkTarget,
}

enum Osc8Target {
    Open(LinkTarget),
    Close,
}

pub(super) fn link_preview(target: &LinkTarget) -> String {
    let label = match target {
        LinkTarget::Url(url) => crate::render::terminal::layout::sanitize(url).into_owned(),
        LinkTarget::Anchor(anchor) => format!("#{anchor}"),
    };
    format!("link: {}", truncate_plain(&label, 72))
}

pub(super) fn link_regions_from_osc(ansi: &str) -> Vec<LinkRegion> {
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

pub(super) fn shift_line(line: usize, shift: isize) -> usize {
    if shift >= 0 {
        line.saturating_add(shift.unsigned_abs())
    } else {
        line.saturating_sub(shift.unsigned_abs())
    }
}

fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

pub(super) fn open_target(url: &str, base_dir: Option<&Path>) -> Result<OsString, &'static str> {
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
    let path = Path::new(url);
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

pub(super) fn resolve_jailed(rel: &str, base: Option<&Path>) -> Option<PathBuf> {
    let path = Path::new(rel);
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

pub(super) fn uri_scheme(value: &str) -> Option<&str> {
    let (scheme, _rest) = value.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
    .then_some(scheme)
}
