use serde::{Deserialize, Serialize};

use crate::scoring::ScoringConfig;
use crate::tui::theme::ThemeVariant;

fn default_refresh_interval() -> u64 {
    300
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

    /// TUI theme (dark or light, defaults to dark)
    #[serde(default)]
    pub theme: ThemeVariant,
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
