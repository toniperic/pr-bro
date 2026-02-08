use anyhow::{Context, Result};
use http::{HeaderMap, Uri};
use octocrab::service::middleware::cache::{CacheKey, CacheStorage, CacheWriter, CachedResponse};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Configuration for HTTP response caching
#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub enabled: bool, // false when --no-cache
}

/// Get the platform-appropriate cache directory for pr-bro
pub fn get_cache_path() -> PathBuf {
    dirs::cache_dir()
        .map(|p| p.join("pr-bro/http-cache"))
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "{}/.cache/pr-bro/http-cache",
                std::env::var("HOME").unwrap_or_default()
            ))
        })
}

/// Clear the HTTP cache directory
pub fn clear_cache() -> Result<()> {
    let cache_path = get_cache_path();
    match std::fs::remove_dir_all(&cache_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("Failed to remove cache directory"),
    }
}

/// Evict cache entries older than 7 days. Returns number of entries removed.
/// Best-effort: errors during listing or removal are silently ignored.
pub fn evict_stale_entries() -> usize {
    let cache_path = get_cache_path();
    let threshold = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    // 7 days in milliseconds
    let max_age_ms: u128 = 7 * 24 * 60 * 60 * 1000;
    let cutoff = threshold.saturating_sub(max_age_ms);

    let mut removed = 0usize;
    for entry in cacache::list_sync(&cache_path).flatten() {
        if entry.time < cutoff {
            let _ = cacache::remove_sync(&cache_path, &entry.key);
            removed += 1;
        }
    }
    removed
}

/// Disk-persistent cache implementing octocrab's CacheStorage trait
///
/// Uses cacache for disk persistence and in-memory HashMap for fast access.
/// Responses are cached by URI with ETag/Last-Modified headers for conditional requests.
#[derive(Clone)]
pub struct DiskCache {
    inner: Arc<Mutex<CacheData>>,
    cache_path: PathBuf,
}

struct CacheData {
    keys: HashMap<String, CacheKey>,            // URI string -> CacheKey
    responses: HashMap<String, CachedResponse>, // URI string -> cached response
}

/// Serializable representation of a cache entry for disk storage
#[derive(serde::Serialize, serde::Deserialize)]
struct DiskCacheEntry {
    etag: Option<String>,
    last_modified: Option<String>,
    headers: Vec<(String, Vec<u8>)>, // header name -> value bytes
    body: Vec<u8>,
}

impl DiskCacheEntry {
    /// Create a DiskCacheEntry from CacheKey and CachedResponse
    fn from_parts(key: &CacheKey, response: &CachedResponse) -> Self {
        let (etag, last_modified) = match key {
            CacheKey::ETag(etag) => (Some(etag.clone()), None),
            CacheKey::LastModified(lm) => (None, Some(lm.clone())),
            _ => (None, None), // Handle non-exhaustive enum
        };

        let headers: Vec<(String, Vec<u8>)> = response
            .headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
            .collect();

        Self {
            etag,
            last_modified,
            headers,
            body: response.body.clone(),
        }
    }

    /// Convert back to CacheKey and CachedResponse
    fn to_parts(&self) -> Result<(CacheKey, CachedResponse)> {
        let key = if let Some(etag) = &self.etag {
            CacheKey::ETag(etag.clone())
        } else if let Some(lm) = &self.last_modified {
            CacheKey::LastModified(lm.clone())
        } else {
            anyhow::bail!("Invalid cache entry: no ETag or Last-Modified");
        };

        let mut headers = HeaderMap::new();
        for (name, value) in &self.headers {
            let header_name: http::HeaderName = name.parse().context("Invalid header name")?;
            let header_value =
                http::HeaderValue::from_bytes(value).context("Invalid header value")?;
            headers.insert(header_name, header_value);
        }

        let response = CachedResponse {
            headers,
            body: self.body.clone(),
        };

        Ok((key, response))
    }
}

impl DiskCache {
    pub fn new(cache_path: PathBuf) -> Self {
        // Don't pre-load disk cache - entries are loaded on demand
        Self {
            inner: Arc::new(Mutex::new(CacheData {
                keys: HashMap::new(),
                responses: HashMap::new(),
            })),
            cache_path,
        }
    }

    /// Clear the in-memory cache to force fresh requests on next fetch
    pub fn clear_memory(&self) {
        let mut data = self.inner.lock().unwrap();
        data.keys.clear();
        data.responses.clear();
    }

    /// Try to load a cache entry from disk
    fn load_from_disk(&self, uri_key: &str) -> Option<CacheKey> {
        // Try to read from disk
        let bytes = cacache::read_sync(&self.cache_path, uri_key).ok()?;

        // Deserialize
        let entry: DiskCacheEntry = serde_json::from_slice(&bytes).ok()?;

        // Convert to CacheKey and CachedResponse
        let (key, response) = entry.to_parts().ok()?;

        // Populate in-memory cache for subsequent hits
        let mut data = self.inner.lock().unwrap();
        data.keys.insert(uri_key.to_string(), key.clone());
        data.responses.insert(uri_key.to_string(), response);

        Some(key)
    }
}

impl CacheStorage for DiskCache {
    fn try_hit(&self, uri: &Uri) -> Option<CacheKey> {
        let uri_key = uri.to_string();

        // Check in-memory first
        {
            let data = self.inner.lock().unwrap();
            if let Some(cache_key) = data.keys.get(&uri_key) {
                return Some(cache_key.clone());
            }
        }

        // Try loading from disk
        self.load_from_disk(&uri_key)
    }

    fn load(&self, uri: &Uri) -> Option<CachedResponse> {
        let data = self.inner.lock().unwrap();
        data.responses.get(&uri.to_string()).cloned()
    }

    fn writer(&self, uri: &Uri, key: CacheKey, headers: HeaderMap) -> Box<dyn CacheWriter> {
        Box::new(DiskCacheWriter {
            cache: self.inner.clone(),
            cache_path: self.cache_path.clone(),
            uri_key: uri.to_string(),
            key,
            response: CachedResponse {
                body: Vec::new(),
                headers,
            },
        })
    }
}

/// Writer that persists cache entries to both memory and disk
struct DiskCacheWriter {
    cache: Arc<Mutex<CacheData>>,
    cache_path: PathBuf,
    uri_key: String,
    key: CacheKey,
    response: CachedResponse,
}

impl CacheWriter for DiskCacheWriter {
    fn write_body(&mut self, data: &[u8]) {
        self.response.body.extend_from_slice(data);
    }
}

impl Drop for DiskCacheWriter {
    fn drop(&mut self) {
        let uri_key = self.uri_key.clone();
        let key = self.key.clone();
        let response = CachedResponse {
            body: std::mem::take(&mut self.response.body),
            headers: self.response.headers.clone(),
        };

        // Validate that the response body is valid JSON before caching
        // Truncated/incomplete responses from network failures should not be persisted
        if serde_json::from_slice::<serde_json::Value>(&response.body).is_err() {
            // Skip caching - body is empty or invalid JSON
            return;
        }

        // Write to in-memory cache
        {
            let mut data = self.cache.lock().unwrap();
            data.keys.insert(uri_key.clone(), key.clone());
            data.responses.insert(uri_key.clone(), response.clone());
        }

        // Write to disk (fire-and-forget, don't block on disk errors)
        let entry = DiskCacheEntry::from_parts(&key, &response);
        if let Ok(serialized) = serde_json::to_vec(&entry) {
            let _ = cacache::write_sync(&self.cache_path, &uri_key, &serialized);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, Uri};
    use octocrab::service::middleware::cache::{CacheKey, CacheStorage};

    fn unique_cache_path(test_name: &str) -> PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pr-bro-test-cache-{}-{}", test_name, timestamp))
    }

    #[test]
    fn test_valid_json_is_cached() {
        let cache_path = unique_cache_path("valid");
        let cache = DiskCache::new(cache_path.clone());

        let uri = Uri::from_static("https://api.github.com/repos/test/test/pulls/1");
        let key = CacheKey::ETag("test-etag".to_string());
        let headers = HeaderMap::new();

        // Write valid JSON body
        let mut writer = cache.writer(&uri, key, headers);
        writer.write_body(br#"{"login":"test","id":1}"#);
        drop(writer);

        // Verify cache hit
        assert!(cache.try_hit(&uri).is_some());
        assert!(cache.load(&uri).is_some());

        // Cleanup
        let _ = std::fs::remove_dir_all(&cache_path);
    }

    #[test]
    fn test_truncated_json_is_not_cached() {
        let cache_path = unique_cache_path("truncated");
        let cache = DiskCache::new(cache_path.clone());

        let uri = Uri::from_static("https://api.github.com/repos/test/test/pulls/2");
        let key = CacheKey::ETag("test-etag-2".to_string());
        let headers = HeaderMap::new();

        // Write truncated JSON body (missing closing brace and value)
        let mut writer = cache.writer(&uri, key, headers);
        writer.write_body(br#"{"login":"test","id":"#);
        drop(writer);

        // Verify cache miss - truncated JSON should not be cached
        assert!(cache.try_hit(&uri).is_none());
        assert!(cache.load(&uri).is_none());

        // Cleanup
        let _ = std::fs::remove_dir_all(&cache_path);
    }

    #[test]
    fn test_empty_body_is_not_cached() {
        let cache_path = unique_cache_path("empty");
        let cache = DiskCache::new(cache_path.clone());

        let uri = Uri::from_static("https://api.github.com/repos/test/test/pulls/3");
        let key = CacheKey::ETag("test-etag-3".to_string());
        let headers = HeaderMap::new();

        // Write empty body
        let mut writer = cache.writer(&uri, key, headers);
        writer.write_body(b"");
        drop(writer);

        // Verify cache miss - empty body should not be cached
        assert!(cache.try_hit(&uri).is_none());
        assert!(cache.load(&uri).is_none());

        // Cleanup
        let _ = std::fs::remove_dir_all(&cache_path);
    }
}
