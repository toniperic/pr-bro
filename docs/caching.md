# Caching

PR Bro uses ETag-based HTTP caching to reduce GitHub API rate limit consumption. For quick-start, see the [README](../README.md).

## Cache Location

Platform-specific cache directory:
- **macOS**: `~/Library/Caches/pr-bro`
- **Linux**: `~/.cache/pr-bro`
- **Windows**: `%LOCALAPPDATA%\pr-bro\cache`

## Cache Behavior

- **In-memory cache**: Fast access to recently fetched data
- **Disk cache**: Persistent storage using ETags for validation
- **Manual refresh** (`r` key in TUI): Bypasses in-memory cache for fresh data
- **Auto-refresh**: Uses cache (only fetches if data changed on server)

## Cache Management

```bash
# Disable caching for one run
pr-bro --no-cache

# Clear all cached responses
pr-bro --clear-cache
```

Clearing cache removes all stored API responses but preserves configuration and snooze state.
