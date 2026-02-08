/// Escape special Typst markup characters in content text.
///
/// Characters that have special meaning in Typst body content (`#`, `*`, `_`,
/// `@`, `<`, `>`, `$`, `\`, `~`) are prefixed with a backslash.
pub(crate) fn escape_typst_content(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '#' | '*' | '_' | '@' | '<' | '>' | '$' | '\\' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape special characters in Typst string literals (inside `"`).
pub(crate) fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_content_special_chars() {
        let result = escape_typst_content("# Hello *world* _foo_");
        assert_eq!(result, "\\# Hello \\*world\\* \\_foo\\_");
    }

    #[test]
    fn escape_content_no_special_chars() {
        let result = escape_typst_content("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn escape_string_quotes_and_backslashes() {
        let result = escape_typst_string(r#"path\to\"file""#);
        assert!(result.contains("\\\\"));
        assert!(result.contains("\\\""));
    }
}
