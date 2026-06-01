use url::{Host, Url};

use super::origin::DocumentOrigin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDocument {
    pub body: String,
    pub origin: DocumentOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteInput {
    Url(Url),
    GitHub(GitHubReference),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubReference {
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub reference: String,
}

pub fn parse_remote_input(value: &str) -> Result<Option<RemoteInput>, String> {
    if let Some(spec) = value.strip_prefix("gh:") {
        return parse_github_reference(spec)
            .map(RemoteInput::GitHub)
            .map(Some);
    }

    let lower = value.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        let url = Url::parse(value).map_err(|err| format!("invalid URL: {err}"))?;
        return Ok(Some(RemoteInput::Url(url)));
    }

    Ok(None)
}

impl RemoteInput {
    pub fn url(&self) -> Result<Url, String> {
        match self {
            Self::Url(url) => Ok(url.clone()),
            Self::GitHub(reference) => reference.to_raw_url(),
        }
    }
}

impl GitHubReference {
    fn to_raw_url(&self) -> Result<Url, String> {
        let mut url = Url::parse("https://raw.githubusercontent.com/")
            .map_err(|err| format!("invalid GitHub raw base URL: {err}"))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|()| "invalid GitHub raw base URL".to_string())?;
            segments.push(&self.owner).push(&self.repo);
            for segment in self.reference.split('/') {
                segments.push(segment);
            }
            for segment in self.path.split('/') {
                segments.push(segment);
            }
        }
        Ok(url)
    }
}

fn parse_github_reference(spec: &str) -> Result<GitHubReference, String> {
    let (owner, rest) = spec
        .split_once('/')
        .ok_or_else(|| "expected gh:owner/repo[/path][@ref]".to_string())?;
    validate_github_segment(owner, "owner")?;

    let repo_end = rest.find(['/', '@']).unwrap_or(rest.len());
    let repo = &rest[..repo_end];
    validate_github_segment(repo, "repo")?;

    let tail = &rest[repo_end..];
    let (path, reference) = if let Some(reference) = tail.strip_prefix('@') {
        ("README.md", reference)
    } else if let Some(path_ref) = tail.strip_prefix('/') {
        split_path_and_ref(path_ref)
    } else {
        ("README.md", "HEAD")
    };

    validate_github_path(path)?;
    validate_github_ref(reference)?;

    Ok(GitHubReference {
        owner: owner.to_string(),
        repo: repo.to_string(),
        path: path.to_string(),
        reference: reference.to_string(),
    })
}

fn split_path_and_ref(path_ref: &str) -> (&str, &str) {
    if let Some((path, reference)) = path_ref.rsplit_once('@') {
        (path, reference)
    } else {
        (path_ref, "HEAD")
    }
}

fn validate_github_segment(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('.')
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(format!("invalid GitHub {label}"));
    }
    Ok(())
}

fn validate_github_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err("invalid GitHub path".to_string());
    }
    Ok(())
}

fn validate_github_ref(reference: &str) -> Result<(), String> {
    if reference.is_empty()
        || reference.starts_with('/')
        || reference
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err("invalid GitHub ref".to_string());
    }
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "terminal"))]
pub fn fetch_remote_document(input: &RemoteInput) -> Result<RemoteDocument, String> {
    let url = input.url()?;
    let bytes = fetch_remote_bytes_cached(&url)?;
    let body =
        String::from_utf8(bytes).map_err(|err| format!("remote document is not UTF-8: {err}"))?;
    Ok(RemoteDocument {
        body,
        origin: DocumentOrigin::remote(url),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn fetch_remote_bytes(url: &str) -> Result<Vec<u8>, String> {
    use std::time::Duration;

    let parsed = Url::parse(url).map_err(|err| format!("invalid URL: {err}"))?;
    validate_fetch_url(&parsed)?;

    let config = hardened_fetch_config(Duration::from_secs(20));
    let agent = ureq::Agent::with_parts(
        config,
        ureq::unversioned::transport::DefaultConnector::default(),
        PublicResolver::default(),
    );

    let mut response = agent
        .get(parsed.as_str())
        .header(
            "User-Agent",
            concat!("silkprint/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(fetch_error_message)?;

    response
        .body_mut()
        .with_config()
        .limit(MAX_REMOTE_BYTES)
        .read_to_vec()
        .map_err(|err| err.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn validate_remote_link(url: &str) -> Result<(), String> {
    use std::time::Duration;

    let parsed = Url::parse(url).map_err(|err| format!("invalid URL: {err}"))?;
    validate_fetch_url(&parsed)?;

    let config = hardened_fetch_config(Duration::from_secs(10));
    let agent = ureq::Agent::with_parts(
        config,
        ureq::unversioned::transport::DefaultConnector::default(),
        PublicResolver::default(),
    );
    agent
        .head(parsed.as_str())
        .header(
            "User-Agent",
            concat!("silkprint/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map(|_| ())
        .map_err(fetch_error_message)
}

#[cfg(not(target_arch = "wasm32"))]
fn hardened_fetch_config(timeout: std::time::Duration) -> ureq::config::Config {
    ureq::config::Config::builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        // Proxy CONNECT would let the proxy resolve the original host again.
        .proxy(None)
        .build()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "terminal"))]
fn fetch_remote_bytes_cached(url: &Url) -> Result<Vec<u8>, String> {
    let Some(path) = download_cache_path(url) else {
        return fetch_remote_bytes(url.as_str());
    };
    if let Ok(bytes) = std::fs::read(&path) {
        return Ok(bytes);
    }

    let bytes = fetch_remote_bytes(url.as_str())?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, &bytes);
    Ok(bytes)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "terminal"))]
fn download_cache_path(url: &Url) -> Option<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.as_str().hash(&mut hasher);
    let key = hasher.finish();
    directories::ProjectDirs::from("tech", "hyperbliss", "silkprint").map(|dirs| {
        dirs.cache_dir()
            .join("downloads")
            .join(format!("{key:016x}.md"))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_error_message(err: ureq::Error) -> String {
    match err {
        ureq::Error::HostNotFound => "blocked non-public or unresolvable host".to_string(),
        other => other.to_string(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_fetch_url(url: &Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("only http and https URLs are supported".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URLs with credentials are not supported".to_string());
    }
    let Some(host) = url.host() else {
        return Err("URL is missing a host".to_string());
    };
    if !host_is_public(&host) {
        return Err("blocked non-public or unresolvable host".to_string());
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
struct PublicResolver {
    inner: ureq::unversioned::resolver::DefaultResolver,
}

#[cfg(not(target_arch = "wasm32"))]
impl ureq::unversioned::resolver::Resolver for PublicResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> Result<ureq::unversioned::resolver::ResolvedSocketAddrs, ureq::Error> {
        let addrs = self.inner.resolve(uri, config, timeout)?;
        if all_addrs_are_global(addrs.iter()) {
            Ok(addrs)
        } else {
            Err(ureq::Error::HostNotFound)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn all_addrs_are_global<'a>(addrs: impl IntoIterator<Item = &'a std::net::SocketAddr>) -> bool {
    addrs.into_iter().all(|addr| is_global_ip(addr.ip()))
}

#[cfg(not(target_arch = "wasm32"))]
fn host_is_public(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(domain) => {
            let lower = domain.to_ascii_lowercase();
            let lower = lower.trim_end_matches('.');
            let last_label = lower.rsplit('.').next().unwrap_or(lower);
            !lower.is_empty() && last_label != "localhost" && last_label != "local"
        }
        Host::Ipv4(ip) => is_global_ip((*ip).into()),
        Host::Ipv6(ip) => is_global_ip((*ip).into()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn is_global_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || o[0] == 0
                || (o[0] == 100 && (o[1] & 0xc0) == 64))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_global_ip(IpAddr::V4(v4));
            }
            let segments = v6.segments();
            if let Some(v4) = embedded_ipv4(segments) {
                return is_global_ip(IpAddr::V4(v4));
            }
            let first = segments[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (segments[0] == 0x2001 && segments[1] == 0x0db8)
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn embedded_ipv4(segments: [u16; 8]) -> Option<std::net::Ipv4Addr> {
    let tail = || {
        let [a, b] = segments[6].to_be_bytes();
        let [c, d] = segments[7].to_be_bytes();
        std::net::Ipv4Addr::new(a, b, c, d)
    };

    if segments[..6] == [0, 0, 0, 0, 0, 0]
        || segments[..6] == [0, 0, 0, 0, 0xffff, 0]
        || segments[..6] == [0x0064, 0xff9b, 0, 0, 0, 0]
    {
        return Some(tail());
    }

    if segments[0] == 0x2002 {
        let [a, b] = segments[1].to_be_bytes();
        let [c, d] = segments[2].to_be_bytes();
        return Some(std::net::Ipv4Addr::new(a, b, c, d));
    }

    None
}

#[cfg(not(target_arch = "wasm32"))]
const MAX_REMOTE_BYTES: u64 = 20 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_reference_defaults() {
        let input = parse_remote_input("gh:hyperb1iss/silkprint")
            .expect("parse")
            .expect("remote");
        let RemoteInput::GitHub(reference) = input else {
            panic!("expected gh reference");
        };

        assert_eq!(reference.owner, "hyperb1iss");
        assert_eq!(reference.repo, "silkprint");
        assert_eq!(reference.path, "README.md");
        assert_eq!(reference.reference, "HEAD");
        assert_eq!(
            reference.to_raw_url().expect("raw").as_str(),
            "https://raw.githubusercontent.com/hyperb1iss/silkprint/HEAD/README.md"
        );
    }

    #[test]
    fn parses_uppercase_http_urls() {
        let input = parse_remote_input("HTTPS://example.com/README.md")
            .expect("parse")
            .expect("remote");

        assert!(
            matches!(input, RemoteInput::Url(url) if url.as_str() == "https://example.com/README.md")
        );
    }

    #[test]
    fn parses_github_reference_with_path_and_ref() {
        let input = parse_remote_input("gh:owner/repo/docs/guide.md@feature/remote-docs")
            .expect("parse")
            .expect("remote");
        let RemoteInput::GitHub(reference) = input else {
            panic!("expected gh reference");
        };

        assert_eq!(reference.path, "docs/guide.md");
        assert_eq!(reference.reference, "feature/remote-docs");
        assert_eq!(
            reference.to_raw_url().expect("raw").as_str(),
            "https://raw.githubusercontent.com/owner/repo/feature/remote-docs/docs/guide.md"
        );
    }

    #[test]
    fn parses_github_reference_with_repo_ref_only() {
        let input = parse_remote_input("gh:owner/repo@release/2026")
            .expect("parse")
            .expect("remote");
        let RemoteInput::GitHub(reference) = input else {
            panic!("expected gh reference");
        };

        assert_eq!(reference.path, "README.md");
        assert_eq!(reference.reference, "release/2026");
    }

    #[test]
    fn rejects_bad_github_references() {
        for value in [
            "gh:owner",
            "gh:/repo",
            "gh:owner/repo/../secret.md",
            "gh:owner/repo/docs/@main",
            "gh:owner/repo@../main",
        ] {
            assert!(parse_remote_input(value).is_err(), "{value}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ssrf_guard_blocks_local_and_private_hosts() {
        for value in [
            "http://localhost/x.png",
            "http://127.0.0.1/x.png",
            "http://169.254.169.254/latest/meta-data",
            "http://10.0.0.5/x",
            "http://192.168.1.1/x",
            "http://[::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://[::ffff:169.254.169.254]/x",
            "http://[::ffff:0:127.0.0.1]/x",
            "http://[::192.168.1.1]/x",
            "http://[64:ff9b::127.0.0.1]/x",
            "http://[2002:0a00:0001::]/x",
            "https://printer.local/x",
            "https://printer.local./x",
            "ftp://example.com/x",
        ] {
            let url = Url::parse(value).expect("url");
            assert!(validate_fetch_url(&url).is_err(), "{value}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ssrf_resolver_rejects_dns_rebinding_address_sets() {
        let public: std::net::SocketAddr = "93.184.216.34:443".parse().expect("public");
        let private: std::net::SocketAddr = "127.0.0.1:443".parse().expect("private");

        assert!(all_addrs_are_global([public].iter()));
        assert!(!all_addrs_are_global([public, private].iter()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn hardened_fetch_config_ignores_env_proxies() {
        let config = hardened_fetch_config(std::time::Duration::from_secs(20));

        assert!(config.proxy().is_none());
        assert_eq!(config.max_redirects(), 0);
    }
}
