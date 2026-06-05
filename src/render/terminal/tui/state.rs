use std::collections::BTreeMap;
use std::path::PathBuf;

use ratatui::style::Color;
use ratatui::text::Text;
use ratatui::widgets::ListState;
use ratatui_image::picker::Picker;

use crate::render::origin::DocumentOrigin;
use crate::render::terminal::model::RenderedDoc;
use crate::warnings::WarningCollector;

use super::images::ImageStore;
use super::links::LinkRegion;

/// A visited document in the back/forward history and the scroll offset at the
/// time we left it, so returning restores the prior view.
#[derive(Clone)]
pub(super) struct NavEntry {
    pub(super) origin: DocumentOrigin,
    pub(super) scroll: u16,
}

pub(super) struct TabState {
    pub(super) doc: RenderedDoc,
    pub(super) source: String,
    pub(super) title: String,
    pub(super) content: Text<'static>,
    pub(super) content_bg: Color,
    pub(super) content_fg: Color,
    pub(super) link_regions: Vec<LinkRegion>,
    pub(super) block_spans: Vec<(usize, usize)>,
    pub(super) block_jump: Vec<usize>,
    pub(super) rendered_width: u16,
    pub(super) theme_dirty: bool,
    pub(super) scroll: u16,
    pub(super) viewport_h: u16,
    pub(super) outline_state: ListState,
    pub(super) search_query: String,
    pub(super) matches: Vec<usize>,
    pub(super) match_idx: usize,
    pub(super) images: ImageStore,
    pub(super) image_placements: Vec<super::images::Placement>,
    pub(super) details_open: BTreeMap<usize, bool>,
    pub(super) base_dir: Option<PathBuf>,
    pub(super) path: Option<PathBuf>,
    pub(super) origin: Option<DocumentOrigin>,
    pub(super) back: Vec<NavEntry>,
    pub(super) forward: Vec<NavEntry>,
    pub(super) pending_anchor: Option<String>,
}

impl TabState {
    pub(super) fn from_body(
        body: &str,
        picker: Option<Picker>,
        base_dir: Option<PathBuf>,
        watch_path: Option<PathBuf>,
        origin: Option<DocumentOrigin>,
    ) -> Self {
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, body);
        let mut warnings = WarningCollector::new();
        crate::render::markdown::check_content(root, &mut warnings);
        let origin = origin.or_else(|| watch_path.clone().map(DocumentOrigin::local));
        let doc =
            crate::render::terminal::walk::walk_with_origin(root, &mut warnings, origin.as_ref());
        let title =
            crate::render::terminal::layout::sanitize(doc.title.as_deref().unwrap_or("silkprint"))
                .into_owned();
        let mut outline_state = ListState::default();
        if !doc.outline.is_empty() {
            outline_state.select(Some(0));
        }
        Self {
            doc,
            source: body.to_string(),
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
            outline_state,
            search_query: String::new(),
            matches: Vec::new(),
            match_idx: 0,
            images: ImageStore::new(picker, base_dir.clone()),
            image_placements: Vec::new(),
            details_open: BTreeMap::new(),
            base_dir,
            path: watch_path,
            origin,
            back: Vec::new(),
            forward: Vec::new(),
            pending_anchor: None,
        }
    }
}
