use silkprint::error::SilkprintError;
use silkprint::fonts::add_external_font;
use wasm_bindgen::prelude::*;

/// Register a font file for use by the renderer.
///
/// Call once per font file after WASM init, before the first render.
/// Accepts raw TTF/OTF bytes.
#[wasm_bindgen]
pub fn register_font(data: &[u8]) {
    add_external_font(data.to_vec());
}

/// Format a SilkprintError with full diagnostics for display in the browser.
fn format_error(e: &SilkprintError) -> String {
    match e {
        SilkprintError::TypstCompilation { diagnostics } => {
            let mut msg = String::from("Typst compilation failed:\n");
            for d in diagnostics {
                msg.push_str("  - ");
                msg.push_str(d);
                msg.push('\n');
            }
            msg
        }
        other => other.to_string(),
    }
}

/// Render markdown to PDF bytes using a built-in theme.
///
/// Returns the raw PDF as a `Uint8Array` in JavaScript.
#[wasm_bindgen]
pub fn render_pdf(markdown: &str, theme_name: &str) -> Result<Vec<u8>, JsError> {
    let options = silkprint::RenderOptions {
        theme: silkprint::ThemeSource::BuiltIn(theme_name.to_string()),
        theme_explicit: true,
        ..Default::default()
    };

    let (pdf_bytes, _warnings) = silkprint::render(markdown, None, &options)
        .map_err(|e| JsError::new(&format_error(&e)))?;

    Ok(pdf_bytes)
}

/// Render markdown to PDF bytes with explicit paper size.
///
/// Paper sizes: "a4", "letter", "a5", "legal" (case-insensitive).
#[wasm_bindgen]
pub fn render_pdf_with_options(
    markdown: &str,
    theme_name: &str,
    paper: &str,
) -> Result<Vec<u8>, JsError> {
    let paper_size = silkprint::PaperSize::from_str_case_insensitive(paper)
        .map_err(|e| JsError::new(&e.to_string()))?;

    let options = silkprint::RenderOptions {
        theme: silkprint::ThemeSource::BuiltIn(theme_name.to_string()),
        theme_explicit: true,
        paper: paper_size,
        ..Default::default()
    };

    let (pdf_bytes, _warnings) = silkprint::render(markdown, None, &options)
        .map_err(|e| JsError::new(&format_error(&e)))?;

    Ok(pdf_bytes)
}

/// Render markdown to Typst source markup (for debugging/inspection).
#[wasm_bindgen]
pub fn render_to_typst(markdown: &str, theme_name: &str) -> Result<String, JsError> {
    let options = silkprint::RenderOptions {
        theme: silkprint::ThemeSource::BuiltIn(theme_name.to_string()),
        theme_explicit: true,
        ..Default::default()
    };

    let (typst_source, _warnings) = silkprint::render_to_typst(markdown, &options)
        .map_err(|e| JsError::new(&format_error(&e)))?;

    Ok(typst_source)
}

/// Get all available theme names as a JSON array string.
///
/// Returns `["silk-light","silk-dark","silkcircuit-neon",...]`
#[wasm_bindgen]
pub fn list_themes_json() -> String {
    let themes = silkprint::theme::builtin::list_themes();
    let entries: Vec<String> = themes.iter().map(|t| format!("\"{}\"", t.name)).collect();
    format!("[{}]", entries.join(","))
}

/// Get detailed theme metadata as JSON.
///
/// Returns an array of `{name, variant, description, print_safe}` objects.
#[wasm_bindgen]
pub fn list_themes_detailed() -> String {
    let themes = silkprint::theme::builtin::list_themes();
    let entries: Vec<String> = themes
        .iter()
        .map(|t| {
            format!(
                "{{\"name\":\"{}\",\"variant\":\"{}\",\"description\":\"{}\",\"printSafe\":{}}}",
                t.name,
                t.variant,
                t.description.replace('"', "\\\""),
                t.print_safe,
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}
