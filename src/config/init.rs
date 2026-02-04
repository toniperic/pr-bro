use anyhow::{Context, Result};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::config::{get_config_path, Config, QueryConfig};
use crate::scoring::ScoringConfig;

/// Prompt user with a message and return their trimmed input.
fn prompt(message: &str) -> Result<String> {
    print!("{}", message);
    std::io::stdout()
        .flush()
        .context("Failed to flush stdout")?;
    let mut input = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

/// Prompt user with a message and a default value. Returns default if input is empty.
fn prompt_with_default(message: &str, default: &str) -> Result<String> {
    let input = prompt(&format!("{} [{}]: ", message, default))?;
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input)
    }
}

/// Prompt user with a yes/no question. Returns bool based on input and default.
fn prompt_yes_no(message: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    let input = prompt(&format!("{} [{}]: ", message, hint))?;
    let input = input.to_lowercase();
    if input.is_empty() {
        Ok(default_yes)
    } else {
        Ok(input == "y" || input == "yes")
    }
}

/// Run the interactive init wizard to create a config file.
///
/// If `default_path` is Some, uses that as the config file path.
/// Otherwise, prompts the user with the default config path.
pub fn run_init_wizard(default_path: Option<PathBuf>) -> Result<()> {
    println!();
    println!("PR Bro Configuration Wizard");
    println!("===========================");
    println!();

    // 1. Config path
    let default_config_path = default_path.unwrap_or_else(get_config_path);
    let path_str = prompt_with_default(
        "Where should the config be saved?",
        &default_config_path.display().to_string(),
    )?;
    let config_path = PathBuf::from(&path_str);

    // Check if file already exists
    if config_path.exists() {
        let overwrite = prompt_yes_no(
            &format!(
                "Config already exists at {}. Overwrite?",
                config_path.display()
            ),
            false,
        )?;
        if !overwrite {
            println!("Aborted.");
            return Ok(());
        }
    }

    // 2. Scoring configuration
    println!();
    let defaults = ScoringConfig::default();
    let configure_scoring = prompt_yes_no("Configure scoring? (Enter accepts defaults)", true)?;

    let scoring = if configure_scoring {
        println!();

        // Base score
        let base_str = prompt_with_default("Base score", "100")?;
        let base_score: f64 = base_str
            .parse()
            .unwrap_or(defaults.base_score.unwrap_or(100.0));

        // Age factor
        let age = prompt_with_default("Age factor", "+1 per 1h")?;

        // Approvals factor
        let approvals = prompt_with_default("Approvals factor", "+10 per 1")?;

        // Size buckets
        let use_default_size =
            prompt_yes_no("Size buckets - use defaults? (<100: x5, 100-500: x1, >500: x0.5)", true)?;
        let size = if use_default_size {
            defaults.size.clone()
        } else {
            None
        };

        // Previously reviewed
        let prev_reviewed = prompt_with_default(
            "Previously reviewed factor (e.g., x0.5 to deprioritize)",
            "none",
        )?;
        let previously_reviewed = if prev_reviewed == "none" || prev_reviewed.is_empty() {
            None
        } else {
            Some(prev_reviewed)
        };

        ScoringConfig {
            base_score: Some(base_score),
            age: Some(age),
            approvals: Some(approvals),
            size,
            labels: None,
            previously_reviewed,
        }
    } else {
        ScoringConfig::default()
    };

    // 3. Queries (at least one required)
    println!();
    println!("Add at least one query to search for PRs.");
    println!();

    let mut queries: Vec<QueryConfig> = Vec::new();
    loop {
        let name = loop {
            let n = prompt("Query name (e.g., 'my-reviews'): ")?;
            if !n.is_empty() {
                break n;
            }
            println!("  Query name is required.");
        };

        let query = loop {
            let q = prompt("GitHub search query (e.g., 'is:pr review-requested:@me is:open'): ")?;
            if !q.is_empty() {
                break q;
            }
            println!("  GitHub search query is required.");
        };

        queries.push(QueryConfig {
            name: Some(name),
            query,
            scoring: None,
        });

        let add_another = prompt_yes_no("Add another query?", false)?;
        if !add_another {
            break;
        }
        println!();
    }

    // 4. Write config
    let config = Config {
        scoring: Some(scoring),
        queries,
        auto_refresh_interval: 300,
    };

    let yaml = serde_saphyr::to_string(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

    // Create parent directories
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create directory {}",
                parent.display()
            )
        })?;
    }

    std::fs::write(&config_path, &yaml).with_context(|| {
        format!(
            "Failed to write config to {}",
            config_path.display()
        )
    })?;

    println!();
    println!("Config written to {}", config_path.display());
    println!("Set PR_BRO_GH_TOKEN env var with your GitHub token to get started.");

    Ok(())
}
