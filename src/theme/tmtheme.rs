use std::fmt::Write;

use super::syntax::ResolvedSyntaxStyle;

/// Generate a tmTheme XML document from resolved syntax styles.
///
/// Typst uses tmTheme (`TextMate`) format for syntax highlighting.
/// The generated XML is served as a virtual file via `World::file()`
/// at `/__silkprint_theme.tmTheme`.
pub fn generate_tmtheme(
    name: &str,
    background: &str,
    foreground: &str,
    styles: &[ResolvedSyntaxStyle],
) -> String {
    let mut xml = String::with_capacity(2048);

    // XML preamble
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
    xml.push_str("<plist version=\"1.0\">\n");
    xml.push_str("<dict>\n");

    // Theme name
    xml.push_str("  <key>name</key>\n");
    let _ = writeln!(xml, "  <string>{}</string>", escape_xml(name));

    // Settings array
    xml.push_str("  <key>settings</key>\n");
    xml.push_str("  <array>\n");

    // Global settings (first entry has no scope)
    xml.push_str("    <dict>\n");
    xml.push_str("      <key>settings</key>\n");
    xml.push_str("      <dict>\n");
    let _ = writeln!(
        xml,
        "        <key>background</key>\n        <string>{background}</string>"
    );
    let _ = writeln!(
        xml,
        "        <key>foreground</key>\n        <string>{foreground}</string>"
    );
    xml.push_str("      </dict>\n");
    xml.push_str("    </dict>\n");

    // Individual token styles
    for style in styles {
        if style.foreground.is_empty() {
            continue;
        }
        xml.push_str("    <dict>\n");
        let _ = writeln!(
            xml,
            "      <key>name</key>\n      <string>{}</string>",
            escape_xml(&style.name)
        );
        let _ = writeln!(
            xml,
            "      <key>scope</key>\n      <string>{}</string>",
            escape_xml(&style.scope)
        );
        xml.push_str("      <key>settings</key>\n");
        xml.push_str("      <dict>\n");
        let _ = writeln!(
            xml,
            "        <key>foreground</key>\n        <string>{}</string>",
            &style.foreground
        );
        if style.bold || style.italic {
            let font_style = match (style.bold, style.italic) {
                (true, true) => "bold italic",
                (true, false) => "bold",
                (false, true) => "italic",
                (false, false) => unreachable!(),
            };
            let _ = writeln!(
                xml,
                "        <key>fontStyle</key>\n        <string>{font_style}</string>"
            );
        }
        xml.push_str("      </dict>\n");
        xml.push_str("    </dict>\n");
    }

    xml.push_str("  </array>\n");
    xml.push_str("</dict>\n");
    xml.push_str("</plist>\n");

    xml
}

/// Minimal XML escaping for attribute/element values.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_plist_structure() {
        let styles = vec![ResolvedSyntaxStyle {
            name: "keyword".to_string(),
            scope: "keyword, keyword.control".to_string(),
            foreground: "#e135ff".to_string(),
            bold: true,
            italic: false,
        }];

        let xml = generate_tmtheme("test-theme", "#1a1a2e", "#e2e2e8", &styles);
        assert!(xml.contains("<?xml version=\"1.0\""));
        assert!(xml.contains("<key>background</key>"));
        assert!(xml.contains("<string>#1a1a2e</string>"));
        assert!(xml.contains("<string>#e135ff</string>"));
        assert!(xml.contains("<string>bold</string>"));
        assert!(xml.contains("<key>name</key>"));
    }

    #[test]
    fn skips_styles_with_empty_foreground() {
        let styles = vec![ResolvedSyntaxStyle {
            name: "empty".to_string(),
            scope: "source".to_string(),
            foreground: String::new(),
            bold: false,
            italic: false,
        }];

        let xml = generate_tmtheme("test", "#000", "#fff", &styles);
        assert!(!xml.contains("empty"));
    }
}
