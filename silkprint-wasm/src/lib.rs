use serde::Serialize;
use silkprint::error::SilkprintError;
use silkprint::fonts::{add_external_font, clear_external_fonts};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
struct WasmThemeInfo<'a> {
    name: &'a str,
    variant: &'a str,
    description: &'a str,
    family: &'a str,
    #[serde(rename = "printSafe")]
    print_safe: bool,
}

/// Register a font file for use by the renderer.
///
/// Call once per font file after WASM init, before the first render.
/// Accepts raw TTF/OTF bytes.
#[wasm_bindgen]
pub fn register_font(data: &[u8]) {
    add_external_font(data.to_vec());
}

/// Clear all previously registered fonts.
///
/// Useful for hot reload flows or when swapping font sets at runtime.
#[wasm_bindgen]
pub fn reset_fonts() {
    clear_external_fonts();
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

fn to_js_value<T>(value: &T) -> Result<JsValue, JsError>
where
    T: Serialize,
{
    serde_wasm_bindgen::to_value(value)
        .map_err(|err| JsError::new(&format!("failed to serialize WASM value: {err}")))
}

fn theme_names() -> Vec<&'static str> {
    silkprint::theme::builtin::list_themes()
        .iter()
        .map(|theme| theme.name)
        .collect()
}

fn detailed_themes() -> Vec<WasmThemeInfo<'static>> {
    silkprint::theme::builtin::list_themes()
        .iter()
        .map(|theme| WasmThemeInfo {
            name: theme.name,
            variant: theme.variant,
            description: theme.description,
            family: theme.family,
            print_safe: theme.print_safe,
        })
        .collect()
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

    let (pdf_bytes, _warnings) =
        silkprint::render(markdown, None, &options).map_err(|e| JsError::new(&format_error(&e)))?;

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

    let (pdf_bytes, _warnings) =
        silkprint::render(markdown, None, &options).map_err(|e| JsError::new(&format_error(&e)))?;

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

/// Get all available theme names as a JavaScript array.
#[wasm_bindgen]
pub fn list_themes() -> Result<JsValue, JsError> {
    to_js_value(&theme_names())
}

/// Get detailed theme metadata as structured JavaScript objects.
#[wasm_bindgen]
pub fn list_themes_structured() -> Result<JsValue, JsError> {
    to_js_value(&detailed_themes())
}

/// Get all available theme names as a JSON array string.
///
/// Returns `["silk-light","silk-dark","silkcircuit-neon",...]`
#[wasm_bindgen]
pub fn list_themes_json() -> String {
    serde_json::to_string(&theme_names())
        .ok()
        .unwrap_or_else(|| "[]".to_string())
}

/// Get detailed theme metadata as JSON.
///
/// Returns an array of `{name, variant, description, print_safe}` objects.
#[wasm_bindgen]
pub fn list_themes_detailed() -> String {
    serde_json::to_string(&detailed_themes())
        .ok()
        .unwrap_or_else(|| "[]".to_string())
}
