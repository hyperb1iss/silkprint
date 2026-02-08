//! Theme engine — loading, inheritance, color resolution, and validation.
//!
//! Implements the full theme pipeline from SPEC Section 5.5:
//! TOML source -> parse -> inheritance chain -> merge -> color resolve ->
//! syntax fallback -> WCAG checks -> tmTheme generation -> `ResolvedTheme`.

pub mod builtin;
pub mod contrast;
pub mod syntax;
pub mod tmtheme;
pub mod tokens;

use std::collections::{HashMap, HashSet};

use crate::error::SilkprintError;
use crate::warnings::WarningCollector;
use crate::ThemeSource;

use self::tokens::{SyntaxTokens, ThemeTokens};

/// Maximum depth for theme inheritance chains.
const MAX_INHERITANCE_DEPTH: usize = 5;

/// A fully resolved theme with all color references replaced with hex values,
/// inheritance merged, and syntax highlighting ready.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub tokens: ThemeTokens,
    pub tmtheme_xml: String,
}

/// Load and resolve a theme from the given source.
///
/// This is the main entry point for the theme engine. It:
/// 1. Parses the TOML source
/// 2. Builds and resolves the inheritance chain
/// 3. Resolves all color references
/// 4. Applies syntax fallbacks from base themes
/// 5. Runs WCAG contrast checks (warnings)
/// 6. Generates tmTheme XML for Typst
pub fn load_theme(
    source: &ThemeSource,
    warnings: &mut WarningCollector,
) -> Result<ResolvedTheme, SilkprintError> {
    let toml_source = load_toml_source(source)?;
    let tokens = parse_theme_toml(&toml_source, source)?;

    // Build inheritance chain and merge
    let merged = resolve_inheritance(tokens, source)?;

    // Resolve colors: two-level resolution within [colors], then all fields
    let resolved = resolve_all_colors(merged);

    // Apply syntax fallbacks if no syntax was defined in the chain
    let resolved = apply_syntax_fallback(resolved)?;

    // Run WCAG contrast checks
    run_contrast_checks(&resolved, warnings);

    // Generate tmTheme XML
    let syntax_background = resolve_color_ref(&resolved.syntax.background, &resolved.colors);
    let syntax_foreground = resolve_color_ref(&resolved.syntax.text.color, &resolved.colors);
    let syntax_styles = syntax::resolve_syntax_tokens(&resolved.syntax, &resolved.colors);
    let tmtheme_xml = tmtheme::generate_tmtheme(
        &resolved.meta.name,
        &syntax_background,
        &syntax_foreground,
        &syntax_styles,
    );

    Ok(ResolvedTheme {
        tokens: resolved,
        tmtheme_xml,
    })
}

/// Load raw TOML source string from the theme source.
fn load_toml_source(source: &ThemeSource) -> Result<String, SilkprintError> {
    match source {
        ThemeSource::BuiltIn(name) => builtin::get_builtin_theme(name)
            .map(String::from)
            .ok_or_else(|| {
                let available = builtin::list_themes();
                let suggestions = find_suggestions(name, &available);
                SilkprintError::ThemeNotFound {
                    name: name.clone(),
                    suggestions,
                }
            }),
        ThemeSource::Custom(path) => std::fs::read_to_string(path).map_err(|e| {
            SilkprintError::InputRead {
                path: path.display().to_string(),
                source: e,
            }
        }),
        ThemeSource::Inline(toml_str) => Ok(toml_str.clone()),
    }
}

/// Parse a TOML string into `ThemeTokens`.
fn parse_theme_toml(
    toml_source: &str,
    source: &ThemeSource,
) -> Result<ThemeTokens, SilkprintError> {
    toml::from_str(toml_source).map_err(|e| {
        let src_name = match source {
            ThemeSource::BuiltIn(name) => format!("builtin:{name}"),
            ThemeSource::Custom(path) => path.display().to_string(),
            ThemeSource::Inline(_) => "inline".to_string(),
        };
        SilkprintError::ThemeInvalid {
            src: miette::NamedSource::new(src_name, toml_source.to_string()),
            span: (0, toml_source.len().min(1)).into(),
            message: format!("TOML parse error: {e}"),
        }
    })
}

/// Build the inheritance chain and merge themes bottom-up.
///
/// Steps 1-5 from SPEC Section 5.5:
/// - Build chain via `extends` field
/// - Detect cycles
/// - Cap depth at 5
/// - Merge from deepest ancestor first
fn resolve_inheritance(
    tokens: ThemeTokens,
    source: &ThemeSource,
) -> Result<ThemeTokens, SilkprintError> {
    if tokens.meta.extends.is_empty() {
        return Ok(tokens);
    }

    // Build the inheritance chain
    let mut chain: Vec<ThemeTokens> = vec![tokens];
    let mut seen_names: HashSet<String> = HashSet::new();

    // Add the root theme name
    let root_name = chain[0].meta.name.clone();
    if !root_name.is_empty() {
        seen_names.insert(root_name);
    }

    loop {
        let current = chain.last().ok_or_else(|| SilkprintError::ThemeCycle {
            chain: "empty chain".to_string(),
        })?;
        let parent_name = current.meta.extends.clone();

        if parent_name.is_empty() {
            break;
        }

        // Cycle detection
        if seen_names.contains(&parent_name) {
            let names: Vec<String> = chain.iter().map(|t| t.meta.name.clone()).collect();
            return Err(SilkprintError::ThemeCycle {
                chain: format!("{} -> {parent_name}", names.join(" -> ")),
            });
        }

        // Depth check
        if chain.len() >= MAX_INHERITANCE_DEPTH {
            let names: Vec<String> = chain.iter().map(|t| t.meta.name.clone()).collect();
            return Err(SilkprintError::ThemeInheritanceDepth {
                chain: names.join(" -> "),
            });
        }

        // Load the parent theme
        let parent_source = ThemeSource::BuiltIn(parent_name.clone());
        let parent_toml = load_toml_source(&parent_source).map_err(|_| {
            SilkprintError::ThemeNotFound {
                name: parent_name.clone(),
                suggestions: find_suggestions(&parent_name, &builtin::list_themes()),
            }
        })?;
        let parent_tokens = parse_theme_toml(&parent_toml, source)?;

        seen_names.insert(parent_name);
        chain.push(parent_tokens);
    }

    // Merge from bottom up (deepest ancestor first)
    // The chain is [child, parent, grandparent, ...]
    // We reverse to get [grandparent, parent, child] and fold
    chain.reverse();
    let mut merged = chain.remove(0);
    for descendant in chain {
        merged = merge_tokens(merged, descendant);
    }

    Ok(merged)
}

/// Merge two theme token sets. The `child` overrides the `base` for any
/// non-default field. Array fields (fallback chains) are REPLACED, not appended.
#[allow(clippy::needless_pass_by_value)]
fn merge_tokens(base: ThemeTokens, child: ThemeTokens) -> ThemeTokens {
    // Re-serialize both to TOML Value, deep-merge, then deserialize back.
    // This handles all field types correctly without manual per-field logic.
    let base_val = toml::Value::try_from(&base);
    let child_val = toml::Value::try_from(&child);

    match (base_val, child_val) {
        (Ok(mut b), Ok(c)) => {
            deep_merge_toml(&mut b, &c);
            b.try_into().unwrap_or(child)
        }
        _ => child, // If serialization fails, child wins
    }
}

/// Deep-merge TOML values. `overlay` values override `base` values.
/// Tables are merged recursively. All other types: overlay replaces base.
/// Empty strings and zero numbers in the overlay are treated as "unset"
/// and do NOT override the base.
fn deep_merge_toml(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_val) in overlay_table {
                if let Some(base_val) = base_table.get_mut(key) {
                    // Recursive merge for nested tables
                    if base_val.is_table() && overlay_val.is_table() {
                        deep_merge_toml(base_val, overlay_val);
                    } else if !is_default_value(overlay_val) {
                        // Non-default overlay replaces base
                        *base_val = overlay_val.clone();
                    }
                } else if !is_default_value(overlay_val) {
                    base_table.insert(key.clone(), overlay_val.clone());
                }
            }
        }
        (base, overlay) => {
            if !is_default_value(overlay) {
                *base = overlay.clone();
            }
        }
    }
}

/// Check if a TOML value is a "default" (empty string, zero number, false bool).
fn is_default_value(val: &toml::Value) -> bool {
    match val {
        toml::Value::String(s) => s.is_empty(),
        toml::Value::Integer(n) => *n == 0,
        toml::Value::Float(f) => *f == 0.0,
        toml::Value::Boolean(b) => !b,
        toml::Value::Array(a) => a.is_empty(),
        toml::Value::Table(t) => t.is_empty(),
        toml::Value::Datetime(_) => false,
    }
}

/// Two-level color resolution.
///
/// Pass 1: Resolve aliases within `[colors]` (one color referencing another).
/// Pass 2: Resolve all semantic/component color fields against the resolved table.
fn resolve_all_colors(mut tokens: ThemeTokens) -> ThemeTokens {
    // Pass 1: Resolve intra-colors references
    let resolved_colors = resolve_color_table(&tokens.colors);
    tokens.colors = resolved_colors;

    // Pass 2: Resolve all color fields throughout the token tree
    resolve_token_colors(&mut tokens);

    tokens
}

/// Resolve aliases within the `[colors]` table itself.
///
/// A color value can reference another color key (e.g., `surface = "cream"`).
/// We iterate until stable (max 10 passes to prevent infinite loops).
fn resolve_color_table(colors: &HashMap<String, String>) -> HashMap<String, String> {
    let mut resolved = colors.clone();
    for _pass in 0..10 {
        let mut changed = false;
        let snapshot = resolved.clone();
        for value in resolved.values_mut() {
            if !value.starts_with('#') && !value.is_empty() {
                if let Some(target) = snapshot.get(value.as_str()) {
                    if target != value {
                        *value = target.clone();
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    resolved
}

/// Resolve a single color reference against the colors table.
/// Returns the hex value if found, or the original string if not.
fn resolve_color_ref(value: &str, colors: &HashMap<String, String>) -> String {
    if value.is_empty() || value.starts_with('#') {
        return value.to_string();
    }
    colors
        .get(value)
        .cloned()
        .unwrap_or_else(|| value.to_string())
}

/// Resolve all color reference fields in the token tree.
///
/// This walks every string field that might hold a color reference
/// and replaces it with the resolved hex value from the `[colors]` table.
#[allow(clippy::too_many_lines)]
fn resolve_token_colors(tokens: &mut ThemeTokens) {
    let colors = tokens.colors.clone();

    // Helper closure to resolve a single field
    let r = |field: &mut String| {
        if !field.is_empty() && !field.starts_with('#') {
            if let Some(hex) = colors.get(field.as_str()) {
                *field = hex.clone();
            }
        }
    };

    // Page
    r(&mut tokens.page.background);

    // Text
    r(&mut tokens.text.color);

    // Headings
    r(&mut tokens.headings.color);

    // Code block
    r(&mut tokens.code_block.background);
    r(&mut tokens.code_block.border_color);
    r(&mut tokens.code_block.left_accent_color);
    r(&mut tokens.code_block.language_label_color);

    // Code inline
    r(&mut tokens.code_inline.background);
    r(&mut tokens.code_inline.border_color);

    // Blockquote
    r(&mut tokens.blockquote.border_color);
    r(&mut tokens.blockquote.background);
    r(&mut tokens.blockquote.text_color);

    // Table
    r(&mut tokens.table.header_background);
    r(&mut tokens.table.header_border_color);
    r(&mut tokens.table.row_border_color);
    r(&mut tokens.table.stripe_background);

    // Horizontal rule
    r(&mut tokens.horizontal_rule.color);

    // Links
    r(&mut tokens.links.color);

    // Images
    r(&mut tokens.images.caption_color);

    // List
    r(&mut tokens.list.bullet_color);
    r(&mut tokens.list.task_checked_color);
    r(&mut tokens.list.task_unchecked_color);

    // Footnotes
    r(&mut tokens.footnotes.separator_color);
    r(&mut tokens.footnotes.number_color);
    r(&mut tokens.footnotes.backref_color);

    // Alerts
    r(&mut tokens.alerts.note_color);
    r(&mut tokens.alerts.tip_color);
    r(&mut tokens.alerts.important_color);
    r(&mut tokens.alerts.warning_color);
    r(&mut tokens.alerts.caution_color);

    // ToC
    r(&mut tokens.toc.entry_color);
    r(&mut tokens.toc.page_number_color);

    // Page numbers
    r(&mut tokens.page_numbers.color);

    // Title page
    r(&mut tokens.title_page.title_color);
    r(&mut tokens.title_page.subtitle_color);
    r(&mut tokens.title_page.author_color);
    r(&mut tokens.title_page.date_color);
    r(&mut tokens.title_page.separator_color);

    // Emphasis
    r(&mut tokens.emphasis.strikethrough_color);

    // Math
    r(&mut tokens.math.color);

    // Highlight
    r(&mut tokens.highlight.fill);
    r(&mut tokens.highlight.text_color);

    // Description list
    r(&mut tokens.description_list.term_color);

    // Syntax tokens
    r(&mut tokens.syntax.background);
    r(&mut tokens.syntax.text.color);
    r(&mut tokens.syntax.keyword.color);
    r(&mut tokens.syntax.string.color);
    r(&mut tokens.syntax.number.color);
    r(&mut tokens.syntax.function.color);
    r(&mut tokens.syntax.type_.color);
    r(&mut tokens.syntax.comment.color);
    r(&mut tokens.syntax.constant.color);
    r(&mut tokens.syntax.boolean.color);
    r(&mut tokens.syntax.operator.color);
    r(&mut tokens.syntax.property.color);
    r(&mut tokens.syntax.tag.color);
    r(&mut tokens.syntax.attribute.color);
    r(&mut tokens.syntax.variable.color);
    r(&mut tokens.syntax.builtin.color);
    r(&mut tokens.syntax.punctuation.color);
    r(&mut tokens.syntax.escape.color);
}

/// Apply base syntax fallback if no syntax tokens were defined in the chain.
///
/// Per SPEC 5.5 step 7: if the final merged theme has no syntax definitions,
/// inherit from `_base-syntax-light` or `_base-syntax-dark` based on variant.
fn apply_syntax_fallback(mut tokens: ThemeTokens) -> Result<ThemeTokens, SilkprintError> {
    if has_syntax_tokens(&tokens.syntax) {
        return Ok(tokens);
    }

    let base_toml = match tokens.meta.variant.as_str() {
        "dark" => builtin::BASE_SYNTAX_DARK_TOML,
        _ => builtin::BASE_SYNTAX_LIGHT_TOML,
    };

    // Parse only the syntax section from the base theme
    let base: ThemeTokens =
        toml::from_str(base_toml).map_err(|e| SilkprintError::ThemeInvalid {
            src: miette::NamedSource::new("base-syntax", base_toml.to_string()),
            span: (0, 1).into(),
            message: format!("Base syntax theme parse error: {e}"),
        })?;

    tokens.syntax = base.syntax;
    Ok(tokens)
}

/// Check if syntax tokens have any meaningful content.
///
/// Returns `true` if at least one syntax token has a non-empty color,
/// indicating the theme (or its ancestors) defined syntax highlighting.
fn has_syntax_tokens(syntax: &SyntaxTokens) -> bool {
    !syntax.keyword.color.is_empty()
        || !syntax.string.color.is_empty()
        || !syntax.function.color.is_empty()
        || !syntax.comment.color.is_empty()
}

/// Run WCAG contrast checks and emit warnings.
///
/// Per SPEC 5.5 step 10: checks ~12 foreground/background pairs.
#[allow(clippy::too_many_lines)]
fn run_contrast_checks(tokens: &ThemeTokens, warnings: &mut WarningCollector) {
    let page_bg = &tokens.page.background;
    let code_bg = &tokens.code_block.background;

    // Pairs: (element name, foreground, background, minimum ratio)
    let checks: &[(&str, &str, &str, f64)] = &[
        // Body text vs page background (AA: 4.5:1)
        ("body text", &tokens.text.color, page_bg, 4.5),
        // Heading text vs page background (3:1 large text)
        ("headings", &tokens.headings.color, page_bg, 3.0),
        // Link color vs page background (4.5:1)
        ("links", &tokens.links.color, page_bg, 4.5),
        // Blockquote text vs page background (4.5:1)
        (
            "blockquote text",
            &tokens.blockquote.text_color,
            page_bg,
            4.5,
        ),
        // Table header text vs header background (4.5:1)
        (
            "table header",
            &tokens.text.color,
            &tokens.table.header_background,
            4.5,
        ),
        // Caption/footnote text vs page background (4.5:1)
        (
            "caption text",
            &tokens.images.caption_color,
            page_bg,
            4.5,
        ),
        (
            "footnote numbers",
            &tokens.footnotes.number_color,
            page_bg,
            4.5,
        ),
        // Page number color vs page background (3:1)
        ("page numbers", &tokens.page_numbers.color, page_bg, 3.0),
    ];

    for (element, fg, bg, minimum) in checks {
        if fg.is_empty() || bg.is_empty() {
            continue;
        }
        if let Some(warning) = contrast::check_contrast(element, fg, bg, *minimum) {
            warnings.push(warning);
        }
    }

    // Syntax token colors vs code block background (4.5:1 each)
    let syntax_checks: &[(&str, &str)] = &[
        ("syntax: text", &tokens.syntax.text.color),
        ("syntax: keyword", &tokens.syntax.keyword.color),
        ("syntax: string", &tokens.syntax.string.color),
        ("syntax: number", &tokens.syntax.number.color),
        ("syntax: function", &tokens.syntax.function.color),
        ("syntax: type", &tokens.syntax.type_.color),
        ("syntax: comment", &tokens.syntax.comment.color),
        ("syntax: constant", &tokens.syntax.constant.color),
        ("syntax: boolean", &tokens.syntax.boolean.color),
        ("syntax: operator", &tokens.syntax.operator.color),
        ("syntax: property", &tokens.syntax.property.color),
        ("syntax: tag", &tokens.syntax.tag.color),
        ("syntax: attribute", &tokens.syntax.attribute.color),
        ("syntax: variable", &tokens.syntax.variable.color),
        ("syntax: builtin", &tokens.syntax.builtin.color),
        ("syntax: punctuation", &tokens.syntax.punctuation.color),
        ("syntax: escape", &tokens.syntax.escape.color),
    ];

    for (element, fg) in syntax_checks {
        if fg.is_empty() || code_bg.is_empty() {
            continue;
        }
        if let Some(warning) = contrast::check_contrast(element, fg, code_bg, 4.5) {
            warnings.push(warning);
        }
    }
}

/// Find similar theme names for error suggestions.
fn find_suggestions(name: &str, themes: &[builtin::ThemeInfo]) -> String {
    let name_lower = name.to_lowercase();
    let mut matches: Vec<&str> = themes
        .iter()
        .filter(|t| {
            let t_lower = t.name.to_lowercase();
            t_lower.contains(&name_lower)
                || name_lower.contains(&t_lower)
                || levenshtein_distance(&name_lower, &t_lower) <= 3
        })
        .map(|t| t.name)
        .collect();

    if matches.is_empty() {
        // Fall back to showing first few themes
        matches = themes.iter().take(5).map(|t| t.name).collect();
    }

    matches.join(", ")
}

/// Simple Levenshtein distance for fuzzy theme name matching.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = usize::from(a_chars[i - 1] != b_chars[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}
