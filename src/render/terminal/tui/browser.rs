use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::text::truncate_plain;

#[derive(Clone)]
pub(super) struct BrowserEntry {
    pub(super) path: PathBuf,
    pub(super) label: String,
    pub(super) kind: BrowserEntryKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BrowserEntryKind {
    Parent,
    Directory,
    Markdown,
    SearchResult { line: usize },
}

#[derive(Clone)]
pub(super) struct Bookmark {
    pub(super) name: String,
    pub(super) path: PathBuf,
}

pub(super) fn bookmarks_from_config(config: &BTreeMap<String, String>) -> Vec<Bookmark> {
    config
        .iter()
        .filter_map(|(name, path)| {
            let name = name.trim();
            let path = path.trim();
            (!name.is_empty() && !path.is_empty()).then(|| Bookmark {
                name: name.to_string(),
                path: expand_bookmark_path(path),
            })
        })
        .collect()
}

fn expand_bookmark_path(path: &str) -> PathBuf {
    if path == "~" {
        return env::var_os("HOME").map_or_else(|| PathBuf::from(path), PathBuf::from);
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

pub(super) fn browser_entries(root: &Path) -> Vec<BrowserEntry> {
    let mut entries = Vec::new();
    if let Some(parent) = root.parent().filter(|parent| *parent != root) {
        entries.push(BrowserEntry {
            path: parent.to_path_buf(),
            label: "..".to_string(),
            kind: BrowserEntryKind::Parent,
        });
    }
    let Ok(read_dir) = std::fs::read_dir(root) else {
        return entries;
    };
    let mut children: Vec<BrowserEntry> = read_dir
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            if name.starts_with('.') {
                return None;
            }
            let file_type = entry.file_type().ok()?;
            if file_type.is_dir() {
                return Some(BrowserEntry {
                    path,
                    label: format!("{name}/"),
                    kind: BrowserEntryKind::Directory,
                });
            }
            is_markdown_path(&path).then_some(BrowserEntry {
                path,
                label: name.clone(),
                kind: BrowserEntryKind::Markdown,
            })
        })
        .collect();
    children.sort_by(|a, b| {
        browser_entry_rank(a.kind)
            .cmp(&browser_entry_rank(b.kind))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    entries.extend(children);
    entries
}

fn browser_entry_rank(kind: BrowserEntryKind) -> u8 {
    match kind {
        BrowserEntryKind::Parent => 0,
        BrowserEntryKind::Directory => 1,
        BrowserEntryKind::Markdown => 2,
        BrowserEntryKind::SearchResult { .. } => 3,
    }
}

pub(super) fn global_search_entries(root: &Path, query: &str) -> Vec<BrowserEntry> {
    let needle = query.to_lowercase();
    let mut entries = Vec::new();
    for path in markdown_files_recursive(root) {
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel = rel.display().to_string();
        for (idx, line) in body.lines().enumerate() {
            if !line.to_lowercase().contains(&needle) {
                continue;
            }
            let line_no = idx + 1;
            let preview = truncate_plain(line.trim(), 48);
            entries.push(BrowserEntry {
                path: path.clone(),
                label: format!("{rel}:{line_no}  {preview}"),
                kind: BrowserEntryKind::SearchResult { line: line_no },
            });
        }
    }
    entries
}

fn markdown_files_recursive(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files);
    files
}

fn collect_markdown_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(root) else {
        return;
    };
    let mut dirs = Vec::new();
    let mut local_files = Vec::new();
    for entry in read_dir.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            dirs.push(path);
        } else if is_markdown_path(&path) {
            local_files.push(path);
        }
    }
    dirs.sort_by_key(|path| path.file_name().map(OsString::from));
    local_files.sort_by_key(|path| path.file_name().map(OsString::from));
    files.extend(local_files);
    for dir in dirs {
        collect_markdown_files(&dir, files);
    }
}

pub(super) fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd" | "mdwn" | "mkdn"
            )
        })
}
