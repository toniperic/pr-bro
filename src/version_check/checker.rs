use anyhow::{Context, Result};

/// Fetch the latest version from GitHub Releases API
pub async fn fetch_latest_version(token: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://api.github.com/repos/toniperic/pr-bro/releases")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "pr-bro")
        .send()
        .await
        .context("Failed to fetch releases from GitHub")?;

    let releases: Vec<serde_json::Value> = response
        .json()
        .await
        .context("Failed to parse releases JSON")?;

    // Find first non-draft release
    for release in releases {
        if let Some(draft) = release.get("draft").and_then(|v| v.as_bool()) {
            if draft {
                continue; // Skip drafts
            }
        }

        if let Some(tag_name) = release.get("tag_name").and_then(|v| v.as_str()) {
            // Strip leading 'v' if present
            let version = tag_name.strip_prefix('v').unwrap_or(tag_name);
            return Ok(version.to_string());
        }
    }

    anyhow::bail!("No non-draft releases found")
}

/// Check if latest version is newer than current version using semver comparison
pub fn is_newer(current: &str, latest: &str) -> bool {
    let Ok(current_ver) = semver::Version::parse(current) else {
        return false;
    };
    let Ok(latest_ver) = semver::Version::parse(latest) else {
        return false;
    };

    latest_ver > current_ver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
    }

    #[test]
    fn test_is_newer_prerelease_to_stable() {
        // Pre-release to stable of same version
        assert!(is_newer("0.2.0-rc.1", "0.2.0"));
    }

    #[test]
    fn test_is_newer_stable_to_prerelease_of_newer() {
        // Stable to pre-release of newer version
        assert!(is_newer("0.2.0", "0.3.0-rc.1"));
    }

    #[test]
    fn test_is_newer_invalid_versions() {
        assert!(!is_newer("invalid", "0.2.0"));
        assert!(!is_newer("0.2.0", "invalid"));
        assert!(!is_newer("invalid", "also-invalid"));
    }

    #[test]
    fn test_is_newer_prerelease_ordering() {
        // Pre-releases should be ordered correctly
        assert!(is_newer("0.2.0-rc.1", "0.2.0-rc.2"));
        assert!(is_newer("0.2.0-beta.1", "0.2.0-rc.1"));
        assert!(is_newer("0.2.0-alpha.1", "0.2.0-beta.1"));
    }
}
