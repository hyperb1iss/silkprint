use std::path::Path;

pub(crate) fn code_fence_language(info: &str) -> &str {
    info.split([' ', ',', '\t']).next().unwrap_or("")
}

pub(crate) fn is_known_code_fence_language(lang: &str) -> bool {
    let lower = lang.to_lowercase();
    KNOWN_LANGUAGES.contains(&lower.as_str())
}

pub(crate) fn is_http_url(value: &str) -> bool {
    uri_scheme(value).is_some_and(|scheme| {
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
    })
}

pub(crate) fn wikilink_target(target: &str) -> String {
    let (path, anchor) = target
        .split_once('#')
        .map_or((target, None), |(path, anchor)| (path, Some(anchor)));
    if path.is_empty() || uri_scheme(path).is_some() || Path::new(path).extension().is_some() {
        return target.to_string();
    }
    anchor.map_or_else(
        || format!("{path}.md"),
        |anchor| format!("{path}.md#{anchor}"),
    )
}

pub(crate) fn uri_scheme(value: &str) -> Option<&str> {
    let (scheme, _rest) = value.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
    .then_some(scheme)
}

const KNOWN_LANGUAGES: &[&str] = &[
    "bash",
    "c",
    "clojure",
    "cpp",
    "c++",
    "csharp",
    "c#",
    "cs",
    "csv",
    "css",
    "dart",
    "diff",
    "dockerfile",
    "elixir",
    "elm",
    "erlang",
    "go",
    "graphql",
    "haskell",
    "html",
    "java",
    "javascript",
    "js",
    "json",
    "jsonc",
    "jsx",
    "julia",
    "kotlin",
    "latex",
    "tex",
    "lua",
    "makefile",
    "markdown",
    "md",
    "nix",
    "objc",
    "objective-c",
    "ocaml",
    "perl",
    "php",
    "plain",
    "text",
    "txt",
    "powershell",
    "python",
    "py",
    "r",
    "ruby",
    "rb",
    "rust",
    "rs",
    "scala",
    "scss",
    "sh",
    "shell",
    "sql",
    "swift",
    "toml",
    "ts",
    "tsx",
    "typescript",
    "typst",
    "vim",
    "xml",
    "yaml",
    "yml",
    "zig",
    "zsh",
    "mermaid",
    "math",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_fence_language_reads_first_token() {
        assert_eq!(code_fence_language("rust,ignore"), "rust");
        assert_eq!(code_fence_language("python linenos"), "python");
        assert_eq!(code_fence_language(""), "");
    }

    #[test]
    fn wikilink_target_adds_markdown_extension_to_bare_paths() {
        assert_eq!(wikilink_target("Guide"), "Guide.md");
        assert_eq!(wikilink_target("Docs/Intro#top"), "Docs/Intro.md#top");
        assert_eq!(wikilink_target("Guide.md"), "Guide.md");
        assert_eq!(
            wikilink_target("https://example.com/Guide"),
            "https://example.com/Guide"
        );
    }

    #[test]
    fn is_http_url_uses_case_insensitive_http_scheme() {
        assert!(is_http_url("https://example.com/image.png"));
        assert!(is_http_url("HTTPS://example.com/image.png"));
        assert!(is_http_url("http:nopath"));
        assert!(!is_http_url("ftp://example.com/image.png"));
        assert!(!is_http_url("httpx://example.com/image.png"));
        assert!(!is_http_url("./local.png"));
    }
}
