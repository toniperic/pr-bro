pub mod cache;
pub mod checker;

use std::time::{SystemTime, UNIX_EPOCH};

/// Status of version check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionStatus {
    /// A newer version is available
    UpdateAvailable { current: String, latest: String },
    /// Current version is up to date
    UpToDate,
    /// Version check failed or was skipped
    Unknown,
}

/// Check for available updates
///
/// This function checks the GitHub Releases API for newer versions.
/// Results are cached for 24 hours. Dismissed versions are not shown.
/// All errors fail silently and return Unknown.
pub async fn check_version(token: &str, current_version: &str) -> VersionStatus {
    let cache_path = crate::github::cache::get_cache_path();

    // Try to load from cache first
    if let Some(cached) = cache::read_cached_version(&cache_path) {
        if cache::is_cache_fresh(&cached) {
            // Check if this version was dismissed
            if let Some(dismissed) = cache::read_dismissed_version(&cache_path) {
                if dismissed == cached.latest_version {
                    return VersionStatus::UpToDate; // Suppress dismissed version
                }
            }

            // Check if cached version is newer
            if checker::is_newer(current_version, &cached.latest_version) {
                return VersionStatus::UpdateAvailable {
                    current: current_version.to_string(),
                    latest: cached.latest_version,
                };
            } else {
                return VersionStatus::UpToDate;
            }
        }
    }

    // No fresh cache, fetch from API
    let latest_version = match checker::fetch_latest_version(token).await {
        Ok(version) => version,
        Err(_) => return VersionStatus::Unknown, // Fail silently
    };

    // Write to cache
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cached_info = cache::CachedVersionInfo {
        latest_version: latest_version.clone(),
        checked_at: now,
    };

    // Ignore cache write errors
    let _ = cache::write_cached_version(&cache_path, &cached_info);

    // Check if dismissed
    if let Some(dismissed) = cache::read_dismissed_version(&cache_path) {
        if dismissed == latest_version {
            return VersionStatus::UpToDate; // Suppress dismissed version
        }
    }

    // Compare versions
    if checker::is_newer(current_version, &latest_version) {
        VersionStatus::UpdateAvailable {
            current: current_version.to_string(),
            latest: latest_version,
        }
    } else {
        VersionStatus::UpToDate
    }
}

/// Dismiss a specific version so it won't be shown in the update banner
pub fn dismiss_version(version: &str) {
    let cache_path = crate::github::cache::get_cache_path();
    // Ignore errors - this is best-effort
    let _ = cache::write_dismissed_version(&cache_path, version);
}

/// Load cached status without making an API call
///
/// Useful for showing update banner immediately on startup if cache is fresh.
/// Returns Unknown if no fresh cache exists.
pub fn load_cached_status(current_version: &str) -> VersionStatus {
    let cache_path = crate::github::cache::get_cache_path();

    let cached = match cache::read_cached_version(&cache_path) {
        Some(c) if cache::is_cache_fresh(&c) => c,
        _ => return VersionStatus::Unknown,
    };

    // Check if dismissed
    if let Some(dismissed) = cache::read_dismissed_version(&cache_path) {
        if dismissed == cached.latest_version {
            return VersionStatus::UpToDate;
        }
    }

    // Compare versions
    if checker::is_newer(current_version, &cached.latest_version) {
        VersionStatus::UpdateAvailable {
            current: current_version.to_string(),
            latest: cached.latest_version,
        }
    } else {
        VersionStatus::UpToDate
    }
}
