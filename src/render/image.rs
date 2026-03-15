use std::collections::HashMap;
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};
use scraper::{Html, Selector};

use crate::warnings::{SilkprintWarning, WarningCollector};

/// Virtual path prefix for downloaded remote images served through the Typst world.
pub const REMOTE_IMAGE_VPATH_PREFIX: &str = "/__remote_image_";

const MAX_REMOTE_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageMode {
    Compile,
    TypstOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedImage {
    Available { typst_path: String },
    Missing,
    Skipped,
}

#[derive(Debug, Default)]
pub struct PreparedImages {
    images: HashMap<String, PreparedImage>,
    remote_assets: HashMap<String, Vec<u8>>,
}

impl PreparedImages {
    pub fn prepare<'a>(
        root: &'a AstNode<'a>,
        mode: ImageMode,
        root_dir: Option<&Path>,
        warnings: &mut WarningCollector,
    ) -> Self {
        let mut prepared = Self::default();
        let mut next_remote_index = 0_usize;

        for node in root.descendants() {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Image(link) => prepared.prepare_source(
                    &link.url,
                    mode,
                    root_dir,
                    warnings,
                    &mut next_remote_index,
                ),
                NodeValue::HtmlBlock(block) => prepared.prepare_html(
                    &block.literal,
                    mode,
                    root_dir,
                    warnings,
                    &mut next_remote_index,
                ),
                NodeValue::HtmlInline(html) if html.contains("<img") => prepared.prepare_html(
                    html,
                    mode,
                    root_dir,
                    warnings,
                    &mut next_remote_index,
                ),
                _ => {}
            }
        }

        prepared
    }

    pub fn resolve(&self, src: &str) -> Option<&PreparedImage> {
        self.images.get(src)
    }

    pub fn remote_assets(&self) -> &HashMap<String, Vec<u8>> {
        &self.remote_assets
    }

    fn prepare_html(
        &mut self,
        html: &str,
        mode: ImageMode,
        root_dir: Option<&Path>,
        warnings: &mut WarningCollector,
        next_remote_index: &mut usize,
    ) {
        for src in collect_html_image_sources(html) {
            self.prepare_source(&src, mode, root_dir, warnings, next_remote_index);
        }
    }

    fn prepare_source(
        &mut self,
        src: &str,
        mode: ImageMode,
        root_dir: Option<&Path>,
        warnings: &mut WarningCollector,
        next_remote_index: &mut usize,
    ) {
        if self.images.contains_key(src) {
            return;
        }

        if is_remote_image(src) {
            self.prepare_remote(src, mode, warnings, next_remote_index);
            return;
        }

        let prepared = if let Some(root_dir) = root_dir {
            if let Some(path) = resolve_image_path(src, root_dir) {
                PreparedImage::Available {
                    typst_path: image_typst_path(src, &path),
                }
            } else {
                warnings.push(SilkprintWarning::ImageNotFound {
                    path: src.to_string(),
                });
                PreparedImage::Missing
            }
        } else {
            let path = Path::new(src);
            if path.is_absolute() && !path.exists() {
                warnings.push(SilkprintWarning::ImageNotFound {
                    path: src.to_string(),
                });
                PreparedImage::Missing
            } else {
                PreparedImage::Available {
                    typst_path: src.to_string(),
                }
            }
        };

        self.images.insert(src.to_string(), prepared);
    }

    fn prepare_remote(
        &mut self,
        src: &str,
        mode: ImageMode,
        warnings: &mut WarningCollector,
        next_remote_index: &mut usize,
    ) {
        match mode {
            ImageMode::TypstOnly => {
                warnings.push(SilkprintWarning::RemoteImageSkipped {
                    url: src.to_string(),
                });
                self.images.insert(src.to_string(), PreparedImage::Skipped);
            }
            ImageMode::Compile => {
                #[cfg(target_arch = "wasm32")]
                {
                    warnings.push(SilkprintWarning::RemoteImageSkipped {
                        url: src.to_string(),
                    });
                    self.images.insert(src.to_string(), PreparedImage::Skipped);
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    match fetch_remote_image(src) {
                        Ok((bytes, ext)) => {
                            let vpath = format!(
                                "{REMOTE_IMAGE_VPATH_PREFIX}{next_remote_index}.{ext}"
                            );
                            *next_remote_index += 1;
                            self.remote_assets.insert(vpath.clone(), bytes);
                            self.images.insert(
                                src.to_string(),
                                PreparedImage::Available { typst_path: vpath },
                            );
                        }
                        Err(message) => {
                            warnings.push(SilkprintWarning::RemoteImageFetchFailed {
                                url: src.to_string(),
                                message,
                            });
                            self.images.insert(src.to_string(), PreparedImage::Skipped);
                        }
                    }
                }
            }
        }
    }
}

/// Resolve an image path relative to the input file's directory.
pub fn resolve_image_path(image_src: &str, root_dir: &Path) -> Option<PathBuf> {
    if is_remote_image(image_src) {
        return None;
    }

    let path = Path::new(image_src);

    if path.is_absolute() {
        return path.exists().then_some(path.to_path_buf());
    }

    let resolved = root_dir.join(path);
    resolved.exists().then_some(resolved)
}

pub fn is_remote_image(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}

fn collect_html_image_sources(html: &str) -> Vec<String> {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("img").expect("valid img selector");

    document
        .select(&selector)
        .filter_map(|element| element.value().attr("src"))
        .map(str::to_string)
        .collect()
}

fn image_typst_path(original_src: &str, resolved_path: &Path) -> String {
    if Path::new(original_src).is_absolute() {
        resolved_path.to_string_lossy().into_owned()
    } else {
        original_src.to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_remote_image(url: &str) -> Result<(Vec<u8>, String), String> {
    use std::time::Duration;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();

    let mut response = agent
        .get(url)
        .header("User-Agent", concat!("silkprint/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|err| err.to_string())?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_REMOTE_IMAGE_BYTES)
        .read_to_vec()
        .map_err(|err| err.to_string())?;

    let ext = detect_image_extension(&bytes, content_type.as_str(), url)
        .ok_or_else(|| "unsupported or unknown image format".to_string())?;

    Ok((bytes, ext.to_string()))
}

fn detect_image_extension(bytes: &[u8], content_type: &str, url: &str) -> Option<&'static str> {
    let normalized_content_type = content_type
        .split(';')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();

    match normalized_content_type.as_str() {
        "image/png" => return Some("png"),
        "image/jpeg" => return Some("jpg"),
        "image/gif" => return Some("gif"),
        "image/svg+xml" => return Some("svg"),
        "image/webp" => return Some("webp"),
        _ => {}
    }

    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return Some("webp");
    }
    if looks_like_svg(bytes) {
        return Some("svg");
    }

    extension_from_url(url)
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };

    let trimmed = text.trim_start();
    trimmed.starts_with("<svg")
        || (trimmed.starts_with("<?xml") && trimmed.contains("<svg"))
}

fn extension_from_url(url: &str) -> Option<&'static str> {
    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment.split('?').next().unwrap_or(without_fragment);
    let path = without_query.rsplit('/').next().unwrap_or(without_query);
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)?;

    match extension.as_str() {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "gif" => Some("gif"),
        "svg" => Some("svg"),
        "webp" => Some("webp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use comrak::Arena;
    use tempfile::tempdir;

    use super::*;
    use crate::render::markdown;

    #[test]
    fn collects_html_image_sources_from_nested_markup() {
        let html = r#"<div><img src="one.png"><p><img src="two.svg"></p></div>"#;
        let sources = collect_html_image_sources(html);
        assert_eq!(sources, vec!["one.png", "two.svg"]);
    }

    #[test]
    fn resolves_relative_image_path_against_input_directory() {
        let dir = tempdir().expect("should create temp dir");
        let image_path = dir.path().join("chart.svg");
        std::fs::write(&image_path, "<svg xmlns=\"http://www.w3.org/2000/svg\"/>")
            .expect("should write temp image");

        let resolved = resolve_image_path("chart.svg", dir.path()).expect("should resolve image");
        assert_eq!(resolved, image_path);
    }

    #[test]
    fn detects_png_extension_from_signature() {
        let bytes = b"\x89PNG\r\n\x1a\nrest";
        assert_eq!(
            detect_image_extension(bytes, "application/octet-stream", "https://example.com/file"),
            Some("png")
        );
    }

    #[test]
    fn typst_only_mode_marks_remote_images_as_skipped() {
        let arena = Arena::new();
        let root = markdown::parse(&arena, "![badge](https://example.com/badge.png)");
        let mut warnings = WarningCollector::new();

        let images = PreparedImages::prepare(root, ImageMode::TypstOnly, None, &mut warnings);

        assert_eq!(
            images.resolve("https://example.com/badge.png"),
            Some(&PreparedImage::Skipped)
        );
        assert_eq!(warnings.warnings().len(), 1);
        assert!(matches!(
            warnings.warnings()[0],
            SilkprintWarning::RemoteImageSkipped { .. }
        ));
    }
}
