//! Emoji handling is primarily done by comrak's `shortcodes` feature,
//! which translates `:emoji_name:` → Unicode characters at parse time.
//!
//! This module provides any additional emoji-related utilities if needed.

/// Check if a string contains emoji shortcode syntax.
pub fn contains_shortcodes(input: &str) -> bool {
    // Simple heuristic: look for :word: patterns
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
