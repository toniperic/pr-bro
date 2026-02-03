use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION_CHECK_CACHE_KEY: &str = "version-check:latest";
const DISMISSED_VERSION_CACHE_KEY: &str = "version-check:dismissed";
const CACHE_TTL_SECONDS: u64 = 86400; // 24 hours

/// Cached version information with timestamp
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedVersionInfo {
    pub latest_version: String,
    pub checked_at: u64, // Unix timestamp
}

/// Dismissed version tracking
#[derive(Debug, Serialize, Deserialize)]
struct DismissedVersion {
    version: String,
}

/// Read cached version info from disk
pub fn read_cached_version(cache_path: &Path) -> Option<CachedVersionInfo> {
    let bytes = cacache::read_sync(cache_path, VERSION_CHECK_CACHE_KEY).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Write version info to cache
pub fn write_cached_version(cache_path: &Path, info: &CachedVersionInfo) -> Result<()> {
    let json = serde_json::to_vec(info)?;
    cacache::write_sync(cache_path, VERSION_CHECK_CACHE_KEY, &json)?;
    Ok(())
}

/// Check if cached version info is still fresh (within 24h)
pub fn is_cache_fresh(info: &CachedVersionInfo) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    now - info.checked_at < CACHE_TTL_SECONDS
}

/// Read dismissed version from cache
pub fn read_dismissed_version(cache_path: &Path) -> Option<String> {
    let bytes = cacache::read_sync(cache_path, DISMISSED_VERSION_CACHE_KEY).ok()?;
    let dismissed: DismissedVersion = serde_json::from_slice(&bytes).ok()?;
    Some(dismissed.version)
}

/// Write dismissed version to cache
pub fn write_dismissed_version(cache_path: &Path, version: &str) -> Result<()> {
    let dismissed = DismissedVersion {
        version: version.to_string(),
    };
    let json = serde_json::to_vec(&dismissed)?;
    cacache::write_sync(cache_path, DISMISSED_VERSION_CACHE_KEY, &json)?;
    Ok(())
}
