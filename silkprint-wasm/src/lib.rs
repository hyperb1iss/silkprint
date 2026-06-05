use serde::Serialize;
use silkprint::ThemeSource;
use silkprint::error::SilkprintError;
use silkprint::fonts::{add_external_font, clear_external_fonts};
use silkprint::warnings::WarningCollector;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
struct WasmThemeColors {
    bg: String,
    fg: String,
    accent: String,
}

#[derive(Debug, Serialize)]
struct WasmThemeInfo<'a> {
    name: &'a str,
    variant: &'a str,
    description: &'a str,
    family: &'a str,
    #[serde(rename = "printSafe")]
    print_safe: bool,
    colors: WasmThemeColors,
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
            colors: theme_colors(theme.name, theme.variant),
        })
        .collect()
}

fn theme_colors(name: &str, variant: &str) -> WasmThemeColors {
    let mut warnings = WarningCollector::new();
    let source = ThemeSource::BuiltIn(name.to_string());
    let Ok(theme) = silkprint::theme::load_theme(&source, &mut warnings) else {
        return fallback_theme_colors(variant);
    };
    let tokens = &theme.tokens;

    WasmThemeColors {
        bg: theme_color(&tokens.page.background, fallback_bg(variant)),
        fg: theme_color(&tokens.text.color, fallback_fg(variant)),
        accent: theme_color(
            first_non_empty(&[
                tokens.headings.h1.color.as_str(),
                tokens.headings.color.as_str(),
                tokens.links.color.as_str(),
                tokens.code_block.left_accent_color.as_str(),
            ]),
            fallback_accent(variant),
        ),
    }
}

fn fallback_theme_colors(variant: &str) -> WasmThemeColors {
    WasmThemeColors {
        bg: fallback_bg(variant).to_string(),
        fg: fallback_fg(variant).to_string(),
        accent: fallback_accent(variant).to_string(),
    }
}

fn theme_color(color: &str, fallback: &str) -> String {
    if color.is_empty() {
        fallback.to_string()
    } else {
        color.to_string()
    }
}

fn first_non_empty<'a>(colors: &[&'a str]) -> &'a str {
    colors
        .iter()
        .copied()
        .find(|color| !color.is_empty())
        .unwrap_or("")
}

fn fallback_bg(variant: &str) -> &'static str {
    if variant == "light" {
        "#ffffff"
    } else {
        "#1a1a2e"
    }
}

fn fallback_fg(variant: &str) -> &'static str {
    if variant == "light" {
        "#111827"
    } else {
        "#f8f8f2"
    }
}

fn fallback_accent(variant: &str) -> &'static str {
    if variant == "light" {
        "#6366f1"
    } else {
        "#80ffea"
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
/// Returns an array of theme metadata objects with resolved preview colors.
#[wasm_bindgen]
pub fn list_themes_detailed() -> String {
    serde_json::to_string(&detailed_themes())
        .ok()
        .unwrap_or_else(|| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::detailed_themes;

    #[test]
    fn detailed_theme_metadata_includes_preview_colors() {
        let themes = detailed_themes();
        let builtin_count = silkprint::theme::builtin::list_themes().len();

        assert_eq!(themes.len(), builtin_count);

        let Some(dawn) = themes.iter().find(|theme| theme.name == "silkcircuit-dawn") else {
            panic!("missing default theme metadata");
        };

        assert_eq!(dawn.family, "silkcircuit");
        assert_eq!(dawn.variant, "light");
        assert!(dawn.colors.bg.starts_with('#'));
        assert!(dawn.colors.fg.starts_with('#'));
        assert!(dawn.colors.accent.starts_with('#'));
    }
}
