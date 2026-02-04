use anyhow::{Context, Result};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::config::{get_config_path, Config, QueryConfig};
use crate::scoring::{ScoringConfig, SizeBucket, SizeConfig};

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

/// Print text with a typewriter effect, one character at a time.
fn typewriter(text: &str) {
    use std::thread;
    use std::time::Duration;
    for c in text.chars() {
        print!("{}", c);
        std::io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(18));
    }
    println!();
}

/// Run the interactive init wizard to create a config file.
///
/// If `default_path` is Some, uses that as the config file path.
/// Otherwise, prompts the user with the default config path.
pub fn run_init_wizard(default_path: Option<PathBuf>) -> Result<()> {
    println!();
    typewriter("PR Bro Configuration Wizard");
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
        typewriter("The base score is the starting point for every PR. All other factors add to or multiply this number.");
        println!();
        let base_str = prompt_with_default("Base score", "100")?;
        let base_score: f64 = base_str
            .parse()
            .unwrap_or(defaults.base_score.unwrap_or(100.0));

        // Age factor
        typewriter("The age factor rewards older PRs so they don't get forgotten. Format: '+N per DURATION' (e.g., '+1 per 1h' adds 1 point per hour).");
        println!();
        let age = prompt_with_default("Age factor", "+1 per 1h")?;

        // Approvals factor
        typewriter("The approvals factor adjusts score based on how many approvals a PR already has. Format: '+N per 1' or 'xN per 1'.");
        println!();
        let approvals = prompt_with_default("Approvals factor", "+10 per 1")?;

        // Size buckets
        typewriter("Size buckets let you boost or penalize PRs based on how many lines were changed. Small PRs are quicker to review!");
        println!();
        let use_default_size =
            prompt_yes_no("Size buckets - use defaults? (<100: x5, 100-500: x1, >500: x0.5)", true)?;
        let size = if use_default_size {
            defaults.size.clone()
        } else {
            typewriter("Let's define your custom size buckets. You'll set a line-count range and a score effect for each.");
            println!();
            let mut buckets: Vec<SizeBucket> = Vec::new();
            loop {
                let range = prompt("  Line count range (e.g., '<100', '100-500', '>500'): ")?;
                if range.is_empty() {
                    println!("  Range is required.");
                    continue;
                }
                let effect = prompt("  Score effect (e.g., 'x5', 'x1', 'x0.5'): ")?;
                if effect.is_empty() {
                    println!("  Effect is required.");
                    continue;
                }
                buckets.push(SizeBucket { range, effect });
                let add_more = prompt_yes_no("  Add another size bucket?", false)?;
                if !add_more {
                    break;
                }
            }
            if buckets.is_empty() {
                None
            } else {
                Some(SizeConfig {
                    exclude: None,
                    buckets: Some(buckets),
                })
            }
        };

        // Previously reviewed
        typewriter("If you've already reviewed a PR, you can deprioritize it so fresh PRs surface first. Use 'x0.5' to halve the score, or 'none' to skip.");
        println!();
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
    typewriter("Now let's set up your PR queries. These use GitHub's search syntax -- the same one you'd use in the GitHub search bar.");
    println!();
    typewriter("Common patterns:");
    typewriter("  review-requested:@me is:open  -- PRs where you're a reviewer");
    typewriter("  author:@me is:open            -- Your own open PRs");
    typewriter("  repo:owner/name is:open       -- All open PRs in a specific repo");
    println!();

    let mut queries: Vec<QueryConfig> = Vec::new();
    let mut query_count = 0;
    loop {
        query_count += 1;
        let name = format!("Query {}", query_count);

        let query = loop {
            let q = prompt("GitHub search query: ")?;
            if !q.is_empty() {
                break q;
            }
            println!("  Search query is required.");
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
