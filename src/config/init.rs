use anyhow::{Context, Result};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::config::{get_config_path, Config, QueryConfig};
use crate::scoring::{Effect, LabelEffect, RangeOp, ScoringConfig, SizeBucket, SizeConfig};

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

/// Validate an approvals effect string using the same "per N" trick as validation.rs.
/// Approvals use "per N" to mean "per N approvals", not per time unit.
/// We convert bare numeric per-parts to "per 1sec" for parsing validation.
fn validate_approvals_str(s: &str) -> Result<(), String> {
    let parseable_str = if let Some((effect_part, per_part)) = s.split_once(" per ") {
        if per_part.trim().chars().all(|c| c.is_numeric() || c == '.') {
            format!("{} per 1sec", effect_part)
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    };
    Effect::parse(&parseable_str)
        .map(|_| ())
        .map_err(|e| e.to_string())
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

    // 1. Scoring configuration
    println!();
    let defaults = ScoringConfig::default();
    let configure_scoring = prompt_yes_no("Configure scoring? (n accepts defaults)", true)?;

    let scoring = if configure_scoring {
        println!();

        // Base score
        typewriter("The base score is the starting point for every PR. All other factors add to or multiply this number.");
        let base_score: f64 = loop {
            let base_str = prompt_with_default("Base score", "100")?;
            match base_str.parse::<f64>() {
                Ok(v) if v >= 0.0 => break v,
                Ok(_) => println!("  Invalid: must be non-negative. Try again."),
                Err(_) => println!("  Invalid: must be a non-negative number. Try again."),
            }
        };

        // Age factor
        println!();
        typewriter("The age factor rewards older PRs so they don't get forgotten.");
        typewriter("Format: '+N per DURATION' adds points over time (e.g., '+1 per 1h' adds 1 point per hour).");
        typewriter("Format: 'xN per DURATION' compounds over time (e.g., 'x1.05 per 1d' multiplies score by 1.05 each day).");
        let age = loop {
            let input = prompt_with_default("Age factor", "+1 per 1h")?;
            match Effect::parse(&input) {
                Ok(_) => break input,
                Err(e) => println!("  Invalid: {}. Try again.", e),
            }
        };

        // Approvals factor
        println!();
        typewriter(
            "The approvals factor adjusts score based on how many approvals a PR already has.",
        );
        typewriter("Available formats:");
        typewriter("  +N per 1  -- adds N points per approval (e.g., '+10 per 1')");
        typewriter("  xN per 1  -- multiplies score by N per approval (e.g., 'x0.8 per 1' to deprioritize approved PRs)");
        typewriter("  +N        -- flat add regardless of count (e.g., '+20')");
        typewriter("  xN        -- flat multiply regardless of count (e.g., 'x2')");
        let approvals = loop {
            let input = prompt_with_default("Approvals factor", "+10 per 1")?;
            match validate_approvals_str(&input) {
                Ok(_) => break input,
                Err(e) => println!("  Invalid: {}. Try again.", e),
            }
        };

        // Size buckets
        println!();
        typewriter(
            "Size buckets let you boost or penalize PRs based on how many lines were changed.",
        );
        typewriter("For example, if you prefer reviewing smaller PRs first, you might set:");
        typewriter("  <100 lines  -> x5    (boosted -- review these first)");
        typewriter("  100-500     -> x1    (neutral)");
        typewriter("  >500 lines  -> x0.25 (penalized -- these drop to the bottom)");
        typewriter("Stick with the defaults if you're unsure -- you can always tweak them later in the config file.");
        let use_default_size = prompt_yes_no(
            "Size buckets - use defaults? (<100: x5, 100-500: x1, >500: x0.5)",
            true,
        )?;
        let size = if use_default_size {
            defaults.size.clone()
        } else {
            typewriter("Let's define your custom size buckets. You'll set a line-count range and a score effect for each.");
            println!();
            let mut buckets: Vec<SizeBucket> = Vec::new();
            loop {
                let range = loop {
                    let r = prompt("  Line count range (e.g., '<100', '100-500', '>500'): ")?;
                    if r.is_empty() {
                        println!("  Range is required.");
                        continue;
                    }
                    match RangeOp::parse(&r) {
                        Ok(_) => break r,
                        Err(e) => println!("  Invalid range: {}. Try again.", e),
                    }
                };
                let effect = loop {
                    let e = prompt("  Score effect (e.g., 'x5', 'x1', 'x0.5'): ")?;
                    if e.is_empty() {
                        println!("  Effect is required.");
                        continue;
                    }
                    match Effect::parse(&e) {
                        Ok(_) => break e,
                        Err(err) => println!("  Invalid effect: {}. Try again.", err),
                    }
                };
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
        println!();
        typewriter("If you've already left a review on a PR, you can adjust its score.");
        typewriter("Use 'x2' to prioritize it (e.g., follow up on your feedback), or 'x0.5' to deprioritize it (focus on fresh PRs).");
        typewriter("Use 'none' to skip this factor entirely.");
        let previously_reviewed = loop {
            let input = prompt_with_default(
                "Previously reviewed factor (e.g., x0.5 to deprioritize)",
                "none",
            )?;
            if input == "none" || input.is_empty() {
                break None;
            }
            match Effect::parse(&input) {
                Ok(_) => break Some(input),
                Err(e) => println!("  Invalid: {}. Try again.", e),
            }
        };

        // Labels
        println!();
        typewriter("Labels let you boost or penalize PRs based on GitHub labels.");
        typewriter(
            "Examples: 'high priority' -> '+50', 'low priority' -> 'x0.5', 'release' -> '+100'.",
        );
        let mut label_effects: Vec<LabelEffect> = Vec::new();
        let mut add_label = prompt_yes_no("Add a label rule?", false)?;
        while add_label {
            let name = loop {
                let n = prompt("  Label name: ")?;
                if !n.trim().is_empty() {
                    break n;
                }
                println!("  Label name is required.");
            };
            let effect = loop {
                let e = prompt("  Score effect (e.g., '+50', 'x0.5', 'x2'): ")?;
                if e.is_empty() {
                    println!("  Effect is required.");
                    continue;
                }
                match Effect::parse(&e) {
                    Ok(_) => break e,
                    Err(err) => println!("  Invalid effect: {}. Try again.", err),
                }
            };
            label_effects.push(LabelEffect { name, effect });
            add_label = prompt_yes_no("  Add another label rule?", false)?;
        }
        let labels = if label_effects.is_empty() {
            None
        } else {
            Some(label_effects)
        };

        ScoringConfig {
            base_score: Some(base_score),
            age: Some(age),
            approvals: Some(approvals),
            size,
            labels,
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
    typewriter("  review-requested:@me review:required is:open is:pr repo:owner/name");
    typewriter("                                        -- combine qualifiers for precision");

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

    // 4. Config path
    let default_config_path = default_path.unwrap_or_else(get_config_path);
    println!();
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

    // 5. Write config
    let config = Config {
        scoring: Some(scoring),
        queries,
        auto_refresh_interval: 300,
    };

    let yaml = serde_saphyr::to_string(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

    // Create parent directories
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    std::fs::write(&config_path, &yaml)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

    println!();
    println!("Config written to {}", config_path.display());
    typewriter("Each scoring parameter you configured can also be overridden per query, for more granular results. See the docs for details and the rest of the options.");
    println!("Run `pr-bro` to get started.");

    Ok(())
}
