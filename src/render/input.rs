use std::path::{Path, PathBuf};

use crate::error::SilkprintError;

pub struct InputDocument {
    pub body: String,
    pub watch_path: Option<PathBuf>,
}

pub fn read_document_input(input_path: &Path) -> Result<InputDocument, SilkprintError> {
    if let Some(body) = direct_asset_markdown(input_path) {
        return Ok(InputDocument {
            body,
            watch_path: None,
        });
    }

    let body = std::fs::read_to_string(input_path).map_err(|e| SilkprintError::InputRead {
        path: input_path.display().to_string(),
        source: e,
    })?;
    Ok(InputDocument {
        body: markdown_body_for_path(input_path, body),
        watch_path: Some(input_path.to_path_buf()),
    })
}

pub fn markdown_body_for_path(path: &Path, body: String) -> String {
    if is_csv_path(path) {
        format!("```csv\n{}\n```\n", body.trim_end())
    } else {
        body
    }
}

pub fn direct_asset_markdown(input_path: &Path) -> Option<String> {
    if !is_direct_asset_path(input_path) {
        return None;
    }
    let file_name = input_path.file_name()?.to_string_lossy();
    let alt = input_path
        .file_stem()
        .map_or_else(|| file_name.clone(), |stem| stem.to_string_lossy());
    let alt = alt.replace(['[', ']'], "");
    Some(format!("![{alt}](<{file_name}>)\n"))
}

pub fn is_direct_asset_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "avif" | "ico" | "svg"
            )
        })
}

pub fn is_csv_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{direct_asset_markdown, markdown_body_for_path, read_document_input};

    #[test]
    fn direct_image_inputs_become_markdown_images() {
        let markdown = direct_asset_markdown(Path::new("screen shot.svg")).expect("asset");

        assert_eq!(markdown, "![screen shot](<screen shot.svg>)\n");
    }

    #[test]
    fn direct_asset_alt_text_drops_markdown_brackets() {
        let markdown = direct_asset_markdown(Path::new("[diagram].png")).expect("asset");

        assert_eq!(markdown, "![diagram](<[diagram].png>)\n");
    }

    #[test]
    fn direct_csv_inputs_become_csv_fences() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "name,count\nalpha,1\n").expect("write csv");

        let document = read_document_input(&path).expect("read document");

        assert_eq!(document.body, "```csv\nname,count\nalpha,1\n```\n");
        assert_eq!(document.watch_path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn markdown_body_for_path_leaves_plain_markdown_alone() {
        let body = "# Title\n\nbody\n".to_string();

        assert_eq!(
            markdown_body_for_path(Path::new("note.md"), body.clone()),
            body
        );
    }
}
