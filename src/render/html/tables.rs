use std::fmt::Write;

use ego_tree::NodeRef;
use scraper::node::Node;

use crate::render::image::PreparedImages;
use crate::warnings::WarningCollector;

use super::{Context, emit_children, parse_alignment};

pub(super) fn emit(
    node: NodeRef<'_, Node>,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) {
    let rows = collect_rows(node);
    if rows.is_empty() {
        return;
    }

    let num_cols = rows
        .iter()
        .map(|row| count_row_cells(*row))
        .max()
        .unwrap_or(0);

    if num_cols == 0 {
        return;
    }

    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: {num_cols},");

    for row in &rows {
        emit_row(*row, out, images, warnings);
    }

    out.push_str(")\n");
}

fn collect_rows(table_node: NodeRef<'_, Node>) -> Vec<NodeRef<'_, Node>> {
    let mut rows = Vec::new();

    for child in table_node.children() {
        if let Node::Element(ref el) = *child.value() {
            match el.name() {
                "tr" => rows.push(child),
                "thead" | "tbody" | "tfoot" => {
                    for grandchild in child.children() {
                        if let Node::Element(ref gc_el) = *grandchild.value()
                            && gc_el.name() == "tr"
                        {
                            rows.push(grandchild);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    rows
}

fn count_row_cells(row: NodeRef<'_, Node>) -> usize {
    row.children()
        .filter(|child: &NodeRef<'_, Node>| {
            if let Node::Element(ref el) = *child.value() {
                matches!(el.name(), "td" | "th")
            } else {
                false
            }
        })
        .count()
}

fn emit_row(
    row: NodeRef<'_, Node>,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) {
    for child in row.children() {
        if let Node::Element(ref el) = *child.value() {
            let tag = el.name();
            if matches!(tag, "td" | "th") {
                let mut cell_content = String::new();
                emit_children(
                    child,
                    &mut cell_content,
                    images,
                    warnings,
                    Context::TableCell,
                );

                let align = parse_alignment(el);

                let formatted = if tag == "th" {
                    format!("*{}*", cell_content.trim())
                } else {
                    cell_content.trim().to_string()
                };

                if let Some(a) = align {
                    let _ = writeln!(out, "  [#align({a})[{formatted}]],");
                } else {
                    let _ = writeln!(out, "  [{formatted}],");
                }
            }
        }
    }
}
