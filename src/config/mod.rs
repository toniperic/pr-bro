mod init;
mod schema;

pub use init::run_init_wizard;
pub use schema::{Config, QueryConfig};

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get the config directory path (~/.config/pr-bro/)
pub fn get_config_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".config").join("pr-bro")
}

/// Get the default config file path (~/.config/pr-bro/config.yaml)
pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.yaml")
}

/// Ensure the config directory exists
pub fn ensure_config_dir() -> Result<()> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory at {}",
                config_dir.display()
            )
        })?;
    }
    Ok(())
}

/// Load configuration from a YAML file
///
/// # Arguments
///
/// * `path` - Optional path to config file. If None, uses default path (~/.config/pr-bro/config.yaml)
///
/// # Errors
///
/// Returns an error if:
/// - The config file does not exist
/// - The config file cannot be read
/// - The YAML cannot be parsed
pub fn load_config(path: Option<PathBuf>) -> Result<Config> {
    let config_path = path.unwrap_or_else(get_config_path);

    if !config_path.exists() {
        anyhow::bail!(
            "Config file not found at {}. Create ~/.config/pr-bro/config.yaml",
            config_path.display()
        );
    }

    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;

    let config: Config = serde_saphyr::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse config {}: {}", config_path.display(), e))?;

    Ok(config)
}
