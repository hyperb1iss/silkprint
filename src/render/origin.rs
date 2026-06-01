use std::path::{Path, PathBuf};

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentOrigin {
    Local(PathBuf),
    Remote(Url),
}

impl DocumentOrigin {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    pub fn remote(url: Url) -> Self {
        Self::Remote(url)
    }

    pub fn local_base_dir(&self) -> Option<PathBuf> {
        match self {
            Self::Local(path) => local_base_dir(path),
            Self::Remote(_) => None,
        }
    }

    pub fn remote_url(&self) -> Option<&Url> {
        match self {
            Self::Remote(url) => Some(url),
            Self::Local(_) => None,
        }
    }

    pub fn resolve_reference(&self, target: &str) -> String {
        if target.starts_with('#') {
            return target.to_string();
        }
        match self {
            Self::Remote(base) => base
                .join(target)
                .map_or_else(|_| target.to_string(), |url| url.to_string()),
            Self::Local(_) => target.to_string(),
        }
    }
}

pub fn local_base_dir(path: &Path) -> Option<PathBuf> {
    path.canonicalize()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .or_else(|| path.parent().map(Path::to_path_buf))
}

pub fn same_remote_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

pub fn is_markdown_url(url: &Url) -> bool {
    std::path::Path::new(url.path())
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd" | "mdwn" | "mkdn"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_origin_resolves_relative_references() {
        let origin = DocumentOrigin::remote(
            Url::parse("https://raw.githubusercontent.com/o/r/HEAD/docs/README.md").expect("url"),
        );

        assert_eq!(
            origin.resolve_reference("../img/logo.png"),
            "https://raw.githubusercontent.com/o/r/HEAD/img/logo.png"
        );
        assert_eq!(origin.resolve_reference("#intro"), "#intro");
    }

    #[test]
    fn same_origin_compares_scheme_host_and_effective_port() {
        let a = Url::parse("https://example.com/a.md").expect("a");
        let b = Url::parse("https://example.com:443/b.md").expect("b");
        let c = Url::parse("http://example.com/b.md").expect("c");

        assert!(same_remote_origin(&a, &b));
        assert!(!same_remote_origin(&a, &c));
    }
}
