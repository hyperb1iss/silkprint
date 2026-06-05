use crate::cli_ui::{color_enabled, coral, cyan, dim, green, purple};

pub(crate) fn handle_list_themes() {
    let mut themes = silkprint::theme::builtin::list_themes();
    themes.sort_by(|a, b| b.print_safe.cmp(&a.print_safe).then(a.name.cmp(b.name)));

    let name_w = 22;
    let variant_w = 6;
    let table_w = 4 + 2 + name_w + 2 + variant_w + 3 + 45;
    let wide_sep = dim(&"\u{2500}".repeat(table_w));
    let thin_sep = dim(&"\u{2508}".repeat(table_w));

    println!();
    println!(
        "  {} {}{}",
        purple("\u{1f48e}"),
        purple("SilkPrint Themes"),
        dim(&format!(
            "{:>width$}",
            format!("{} themes", themes.len()),
            width = table_w - 17
        )),
    );
    println!("  {wide_sep}");
    println!(
        "  {}  {}  {}     {}",
        purple("    "),
        purple(&format!("{:<name_w$}", "Theme")),
        purple(&format!("{:<variant_w$}", "Variant")),
        purple("Description"),
    );
    println!("  {wide_sep}");

    let mut prev_print_safe = true;
    for t in &themes {
        if prev_print_safe && !t.print_safe {
            println!("  {thin_sep}");
        }
        prev_print_safe = t.print_safe;
        let swatch = theme_swatch(t.name);
        let is_default = t.name == "silkcircuit-dawn";

        let name_plain = if is_default {
            format!("{} \u{2605}", t.name)
        } else {
            t.name.to_string()
        };
        let name_padded = format!("{name_plain:<name_w$}");
        let name = if is_default {
            coral(&name_padded)
        } else {
            cyan(&name_padded)
        };

        let variant_padded = format!("{:<variant_w$}", t.variant);
        let variant = dim(&variant_padded);

        let badge = if t.print_safe {
            green("\u{25cf}")
        } else {
            " ".to_string()
        };

        println!(
            "  {swatch}  {name}  {variant}  {badge}  {}",
            dim(t.description)
        );
    }

    println!("  {wide_sep}");
    println!(
        "  {} = print-safe   {} = default",
        green("\u{25cf}"),
        coral("\u{2605}"),
    );
    println!();
}

fn theme_swatch(name: &str) -> String {
    let [bg, heading, text, link] = extract_swatch_colors(name);

    if !color_enabled() {
        return "\u{2588}\u{2588}\u{2588}\u{2588}".to_string();
    }

    let render_block = |hex: &str| -> String {
        let (r, g, b) = hex_to_rgb(hex);
        format!("\x1b[38;2;{r};{g};{b}m\u{2588}\x1b[0m")
    };

    format!(
        "{}{}{}{}",
        render_block(&bg),
        render_block(&heading),
        render_block(&text),
        render_block(&link),
    )
}

fn extract_swatch_colors(name: &str) -> [String; 4] {
    let fallback = || {
        [
            "#888888".to_string(),
            "#888888".to_string(),
            "#888888".to_string(),
            "#888888".to_string(),
        ]
    };

    let Some(toml_str) = silkprint::theme::builtin::get_builtin_theme(name) else {
        return fallback();
    };

    let Ok(table) = toml_str.parse::<toml::Table>() else {
        return fallback();
    };

    let colors = table
        .get("colors")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();

    let resolve_field = |section: &str, field: &str, default: &str| -> String {
        let val = table
            .get(section)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(field))
            .and_then(|v| v.as_str())
            .unwrap_or(default);
        resolve_color_ref(val, &colors)
    };

    [
        resolve_field("page", "background", "#ffffff"),
        resolve_field("headings", "color", "#333333"),
        resolve_field("text", "color", "#1a1a1a"),
        resolve_field("links", "color", "#4a5dbd"),
    ]
}

fn resolve_color_ref(value: &str, colors: &toml::Table) -> String {
    if value.starts_with('#') {
        return value.to_string();
    }
    if let Some(resolved) = colors.get(value).and_then(|v| v.as_str()) {
        if resolved.starts_with('#') {
            resolved.to_string()
        } else {
            colors
                .get(resolved)
                .and_then(|v| v.as_str())
                .unwrap_or("#888888")
                .to_string()
        }
    } else {
        "#888888".to_string()
    }
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return (136, 136, 136);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(136);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(136);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(136);
    (r, g, b)
}
