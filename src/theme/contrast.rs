use crate::warnings::SilkprintWarning;

/// Calculate the WCAG 2.1 relative luminance of an sRGB color.
///
/// Input: hex color string (e.g., "#1a1a2e").
/// Output: luminance value between 0.0 (black) and 1.0 (white).
pub fn relative_luminance(hex: &str) -> Option<f64> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    let to_linear = |c: u8| -> f64 {
        let s = f64::from(c) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };

    let r_lin = to_linear(r);
    let g_lin = to_linear(g);
    let b_lin = to_linear(b);

    Some(0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin)
}

/// Calculate the WCAG 2.1 contrast ratio between two colors.
///
/// Returns a ratio >= 1.0 (e.g., 4.5 for AA compliance).
pub fn contrast_ratio(fg_hex: &str, bg_hex: &str) -> Option<f64> {
    let l1 = relative_luminance(fg_hex)?;
    let l2 = relative_luminance(bg_hex)?;

    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    Some((lighter + 0.05) / (darker + 0.05))
}

/// Check a foreground/background pair and emit a warning if contrast is insufficient.
pub fn check_contrast(
    element: &str,
    fg_hex: &str,
    bg_hex: &str,
    minimum: f64,
) -> Option<SilkprintWarning> {
    let ratio = contrast_ratio(fg_hex, bg_hex)?;
    if ratio < minimum {
        Some(SilkprintWarning::ContrastRatio {
            element: element.to_string(),
            ratio,
            minimum,
        })
    } else {
        None
    }
}
