use std::path::{Path, PathBuf};

/// Resolve an image path relative to the input file's directory.
pub fn resolve_image_path(image_src: &str, root_dir: &Path) -> Option<PathBuf> {
    // Skip remote URLs
    if image_src.starts_with("http://") || image_src.starts_with("https://") {
        return None;
    }

    let path = Path::new(image_src);

    // If absolute, use as-is
    if path.is_absolute() {
        if path.exists() {
            return Some(path.to_path_buf());
        }
        return None;
    }

    // Resolve relative to root directory
    let resolved = root_dir.join(path);
    if resolved.exists() {
        Some(resolved)
    } else {
        None
    }
}
