use serde::{Deserialize, Serialize};

use crate::scoring::ScoringConfig;

fn default_refresh_interval() -> u64 {
    300
}

fn default_theme() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Global scoring configuration (applies to all queries unless overridden)
    #[serde(default)]
    pub scoring: Option<ScoringConfig>,

    pub queries: Vec<QueryConfig>,

    /// Auto-refresh interval in seconds (defaults to 300 = 5 minutes)
    #[serde(default = "default_refresh_interval")]
    pub auto_refresh_interval: u64,

    /// Theme selection: "dark", "light", or "auto" (detects terminal background)
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Notify when new PRs show up in the active list
    #[serde(default)]
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryConfig {
    pub name: Option<String>,
    pub query: String,

    /// Per-query scoring configuration (merges with global scoring — set fields override, unset fields inherit from global)
    #[serde(default)]
    pub scoring: Option<ScoringConfig>,
}

/// Notification settings for new PRs appearing in the active list
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationConfig {
    /// Master switch (defaults to false — opt in)
    #[serde(default)]
    pub enabled: bool,

    /// Ring the terminal bell
    #[serde(default = "default_true")]
    pub sound: bool,

    /// Show a native desktop notification
    #[serde(default = "default_true")]
    pub desktop: bool,

    /// Shell command to run instead of `sound`/`desktop`.
    /// `{count}` is replaced with the number of new PRs.
    #[serde(default)]
    pub command: Option<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sound: true,
            desktop: true,
            command: None,
        }
    }
}
