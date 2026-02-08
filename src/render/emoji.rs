//! Emoji shortcode detection utilities.
//!
//! Emoji translation is handled by comrak's `shortcodes` extension, which
//! converts `:emoji_name:` syntax into Unicode characters at parse time.
//! This module provides a pre-parse heuristic for detecting whether input
//! contains shortcode patterns, useful for diagnostics or progress hints.

/// Check whether a string likely contains emoji shortcode syntax (`:word:` patterns).
///
/// This is a lightweight heuristic -- it looks for colon-delimited identifiers
/// containing only alphanumeric characters, underscores, or hyphens.
/// False positives are possible (e.g. in URLs or clock times like `10:30:00`),
/// but that's acceptable since this is only used for informational purposes.
///
/// # Examples
///
/// ```
/// use silkprint::render::emoji::contains_shortcodes;
///
/// assert!(contains_shortcodes("Hello :wave: world"));
/// assert!(contains_shortcodes(":rocket:"));
/// assert!(!contains_shortcodes("no shortcodes here"));
/// assert!(!contains_shortcodes("just a : colon"));
/// ```
pub fn contains_shortcodes(input: &str) -> bool {
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ':' {
            let mut found_word = false;
            for next in chars.by_ref() {
                if next == ':' && found_word {
                    return true;
                }
                if next.is_alphanumeric() || next == '_' || next == '-' {
                    found_word = true;
                } else {
                    break;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_simple_shortcode() {
        assert!(contains_shortcodes(":wave:"));
    }

    #[test]
    fn detects_shortcode_in_text() {
        assert!(contains_shortcodes("Hello :rocket: world!"));
    }

    #[test]
    fn detects_shortcode_with_hyphens() {
        assert!(contains_shortcodes(":thumbs-up:"));
    }

    #[test]
    fn detects_shortcode_with_underscores() {
        assert!(contains_shortcodes(":heavy_check_mark:"));
    }

    #[test]
    fn rejects_plain_text() {
        assert!(!contains_shortcodes("no shortcodes here"));
    }

    #[test]
    fn rejects_lone_colons() {
        assert!(!contains_shortcodes("just a : colon : pair"));
    }

    #[test]
    fn rejects_empty_shortcode() {
        assert!(!contains_shortcodes("::"));
    }

    #[test]
    fn rejects_colon_with_spaces() {
        assert!(!contains_shortcodes(":not a shortcode:"));
    }
}
