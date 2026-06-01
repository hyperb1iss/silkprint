use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};

use crate::warnings::{SilkprintWarning, WarningCollector};

pub fn validate_links<'a>(
    root: &'a AstNode<'a>,
    input_path: Option<&Path>,
    warnings: &mut WarningCollector,
) {
    let base_dir = input_path.and_then(Path::parent);
    for node in root.descendants() {
        let target = match &node.data.borrow().value {
            NodeValue::Link(link) | NodeValue::Image(link) => Some(link.url.clone()),
            NodeValue::WikiLink(link) => Some(link.url.clone()),
            _ => None,
        };
        let Some(target) = target else {
            continue;
        };
        if target.starts_with('#') || target.starts_with("mailto:") {
            continue;
        }
        if target.starts_with("http://") || target.starts_with("https://") {
            validate_remote(&target, warnings);
        } else {
            validate_local(&target, base_dir, warnings);
        }
    }
}

fn validate_remote(url: &str, warnings: &mut WarningCollector) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(message) = crate::render::remote::validate_remote_link(url) {
        warnings.push(SilkprintWarning::LinkValidationFailed {
            target: url.to_string(),
            message,
        });
    }
}

fn validate_local(target: &str, base_dir: Option<&Path>, warnings: &mut WarningCollector) {
    let path = local_target_path(target, base_dir);
    if !path.exists() {
        warnings.push(SilkprintWarning::LinkValidationFailed {
            target: target.to_string(),
            message: "local target not found".to_string(),
        });
    }
}

fn local_target_path(target: &str, base_dir: Option<&Path>) -> PathBuf {
    let target = target.split_once('#').map_or(target, |(path, _)| path);
    let path = Path::new(target);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.unwrap_or_else(|| Path::new(".")).join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::validate_links;
    use crate::warnings::WarningCollector;

    #[test]
    fn warns_for_missing_local_targets() {
        let dir = tempfile::tempdir().expect("tempdir");
        let arena = comrak::Arena::new();
        let root = crate::render::markdown::parse(&arena, "[missing](missing.md)");
        let mut warnings = WarningCollector::new();

        validate_links(root, Some(&dir.path().join("doc.md")), &mut warnings);

        assert_eq!(warnings.warnings().len(), 1);
    }
}
