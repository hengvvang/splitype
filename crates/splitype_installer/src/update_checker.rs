//! Online update checks against the public project manifest.
//!
//! The app compares its compiled package version with the version published in
//! the main branch Cargo manifest. Gitee is used only as a timeout fallback for
//! GitHub so ordinary HTTP, parsing, or version errors stay visible.

use std::fmt;
use std::time::Duration;

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;

pub const GITHUB_CARGO_TOML_URL: &str =
    "https://raw.githubusercontent.com/hengvvang/splitype/refs/heads/main/Cargo.toml";
pub const GITEE_CARGO_TOML_URL: &str =
    "https://raw.giteeusercontent.com/hengvvang/splitype/raw/main/Cargo.toml";
pub const RELEASES_URL: &str = "https://github.com/hengvvang/splitype/releases";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const UPDATE_ACCEPT: &str = "text/plain,application/toml,*/*;q=0.8";
const UPDATE_USER_AGENT: &str = concat!(
    "splitype/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/hengvvang/splitype)"
);

/// Remote endpoint used to retrieve the published splitype manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateSource {
    /// GitHub raw content endpoint.
    GitHub,
    /// Gitee mirror endpoint, used only when GitHub times out.
    Gitee,
}

impl UpdateSource {
    fn url(self) -> &'static str {
        match self {
            Self::GitHub => GITHUB_CARGO_TOML_URL,
            Self::Gitee => GITEE_CARGO_TOML_URL,
        }
    }
}

impl fmt::Display for UpdateSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitHub => f.write_str("GitHub"),
            Self::Gitee => f.write_str("Gitee"),
        }
    }
}

/// Coarse failure reason for a manifest fetch attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteFetchFailureKind {
    /// Request exceeded the configured timeout.
    Timeout,
    /// The server returned a non-success HTTP status.
    HttpStatus,
    /// Request setup or transport failed before a response was usable.
    Network,
    /// The response body could not be read as text.
    Body,
}

/// Error produced while fetching one remote manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteFetchFailure {
    pub source: UpdateSource,
    pub kind: RemoteFetchFailureKind,
    detail: String,
}

impl RemoteFetchFailure {
    fn new(source: UpdateSource, kind: RemoteFetchFailureKind, detail: impl Into<String>) -> Self {
        Self {
            source,
            kind,
            detail: detail.into(),
        }
    }

    fn timeout(source: UpdateSource, detail: impl Into<String>) -> Self {
        Self::new(source, RemoteFetchFailureKind::Timeout, detail)
    }

    fn is_timeout(&self) -> bool {
        self.kind == RemoteFetchFailureKind::Timeout
    }
}

impl fmt::Display for RemoteFetchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} update manifest fetch failed: {}",
            self.source, self.detail
        )
    }
}

impl std::error::Error for RemoteFetchFailure {}

/// Error returned by the full update-check pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheckError {
    /// No usable remote manifest could be fetched.
    Fetch(RemoteFetchFailure),
    /// The manifest was fetched but could not produce a valid package version.
    ParseVersion(String),
}

impl fmt::Display for UpdateCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(error) => write!(f, "{error}"),
            Self::ParseVersion(detail) => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for UpdateCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fetch(err) => Some(err),
            _ => None,
        }
    }
}

/// Version comparison result used by the editor UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheckResult {
    /// The remote version is newer than the running build.
    UpdateAvailable(UpdateVersionInfo),
    /// The running build is at least as new as the remote version.
    UpToDate(UpdateVersionInfo),
}

/// Version data shown in localized update prompts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateVersionInfo {
    pub current_version: String,
    pub latest_version: String,
    pub source: UpdateSource,
}

pub fn check_latest_version(current_version: &str) -> Result<UpdateCheckResult, UpdateCheckError> {
    check_latest_version_with(current_version, fetch_remote_cargo_toml)
}

fn check_latest_version_with<F>(
    current_version: &str,
    mut fetch: F,
) -> Result<UpdateCheckResult, UpdateCheckError>
where
    F: FnMut(UpdateSource) -> Result<String, RemoteFetchFailure>,
{
    match fetch(UpdateSource::GitHub) {
        Ok(manifest) => compare_manifest_version(current_version, &manifest, UpdateSource::GitHub),
        Err(error) if error.is_timeout() => {
            let manifest = fetch(UpdateSource::Gitee).map_err(UpdateCheckError::Fetch)?;
            compare_manifest_version(current_version, &manifest, UpdateSource::Gitee)
        }
        Err(error) => Err(UpdateCheckError::Fetch(error)),
    }
}

fn compare_manifest_version(
    current_version: &str,
    manifest: &str,
    source: UpdateSource,
) -> Result<UpdateCheckResult, UpdateCheckError> {
    let current = parse_semver(current_version, "current app version")?;
    let latest_text = extract_package_version(manifest)?;
    let latest = parse_semver(&latest_text, "remote Cargo.toml version")?;
    let info = UpdateVersionInfo {
        current_version: current_version.to_string(),
        latest_version: latest_text,
        source,
    };

    if latest > current {
        Ok(UpdateCheckResult::UpdateAvailable(info))
    } else {
        Ok(UpdateCheckResult::UpToDate(info))
    }
}

fn parse_semver(version: &str, label: &str) -> Result<Version, UpdateCheckError> {
    Version::parse(version).map_err(|err| {
        UpdateCheckError::ParseVersion(format!("{label} '{version}' is not valid SemVer: {err}"))
    })
}

pub fn extract_package_version(manifest: &str) -> Result<String, UpdateCheckError> {
    let parsed: toml::Value = toml::from_str(manifest).map_err(|err| {
        UpdateCheckError::ParseVersion(format!("failed to parse remote Cargo.toml: {err}"))
    })?;
    parsed
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            UpdateCheckError::ParseVersion(
                "remote Cargo.toml does not contain [package].version".to_string(),
            )
        })
}

fn fetch_remote_cargo_toml(source: UpdateSource) -> Result<String, RemoteFetchFailure> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .default_headers(update_request_headers())
        .build()
        .map_err(|err| {
            RemoteFetchFailure::new(
                source,
                RemoteFetchFailureKind::Network,
                format!("failed to build HTTP client: {err}"),
            )
        })?;

    let response = client.get(source.url()).send().map_err(|err| {
        if err.is_timeout() {
            RemoteFetchFailure::timeout(source, "request timed out after 5 seconds".to_string())
        } else {
            RemoteFetchFailure::new(source, RemoteFetchFailureKind::Network, err.to_string())
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(RemoteFetchFailure::new(
            source,
            RemoteFetchFailureKind::HttpStatus,
            format!("server returned HTTP {status}"),
        ));
    }

    response.text().map_err(|err| {
        RemoteFetchFailure::new(source, RemoteFetchFailureKind::Body, err.to_string())
    })
}

fn update_request_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(UPDATE_USER_AGENT));
    headers.insert(ACCEPT, HeaderValue::from_static(UPDATE_ACCEPT));
    headers
}
