use std::fmt::Write;

use ego_tree::NodeRef;
use scraper::node::{Element, Node};

use crate::render::escape::escape_typst_string;
use crate::render::image::PreparedImages;
use crate::warnings::WarningCollector;

use super::{Context, emit_children};

pub(super) fn emit(
    node: NodeRef<'_, Node>,
    el: &Element,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) {
    let href = el.attr("href").unwrap_or("");
    let _ = write!(out, "#link(\"{}\")", escape_typst_string(href));

    let mut content = String::new();
    emit_children(node, &mut content, images, warnings, Context::Inline);

    if !content.is_empty() {
        let _ = write!(out, "[{content}]");
    }
}
