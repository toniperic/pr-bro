use crate::config::Config;
use crate::github::cache::CacheConfig;
use crate::github::types::PullRequest;
use crate::scoring::{calculate_score, merge_scoring_configs, ScoreResult};
use crate::snooze::{filter_active_prs, filter_snoozed_prs, SnoozeState};
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Typed error for GitHub authentication failures (401 / Bad credentials).
/// Callers can downcast `anyhow::Error` to this type to distinguish auth
/// errors from transient network errors and trigger a token re-prompt.
#[derive(Debug)]
pub struct AuthError {
    pub message: String,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthError {}

/// Fetch PRs from all configured queries, deduplicate, score, and split into
/// active and snoozed lists. Both lists are sorted by score descending.
///
/// This function is called from main.rs for initial load and from the TUI
/// event loop for manual/auto refresh.
pub async fn fetch_and_score_prs(
    client: &octocrab::Octocrab,
    config: &Config,
    snooze_state: &SnoozeState,
    cache_config: &CacheConfig,
    verbose: bool,
    auth_username: Option<&str>,
) -> Result<(
    Vec<(PullRequest, ScoreResult)>,
    Vec<(PullRequest, ScoreResult)>,
    Option<u64>,
)> {
    if verbose {
        let cache_status = if cache_config.enabled {
            "enabled"
        } else {
            "disabled (--no-cache)"
        };
        eprintln!("Cache: {}", cache_status);
    }

    // Resolve global scoring config once (fallback for queries without per-query scoring)
    let global_scoring = config.scoring.clone().unwrap_or_default();

    // Search PRs for each query in parallel
    let mut all_prs = Vec::new();
    let mut any_succeeded = false;

    let mut futures = FuturesUnordered::new();
    let auth_username_owned = auth_username.map(|s| s.to_string());
    for (query_index, query_config) in config.queries.iter().enumerate() {
        let client = client.clone();
        let query = query_config.query.clone();
        let query_name = query_config.name.clone();
        let auth_username_clone = auth_username_owned.clone();
        // Merge scoring config for this query to get the effective exclude patterns
        let merged_scoring = merge_scoring_configs(&global_scoring, query_config.scoring.as_ref());
        let exclude_patterns = merged_scoring.size.and_then(|s| s.exclude);
        futures.push(async move {
            let result = crate::github::search_and_enrich_prs(
                &client,
                &query,
                auth_username_clone.as_deref(),
                exclude_patterns,
                verbose,
            )
            .await;
            (query_name, query, query_index, result)
        });
    }

    while let Some((name, query, query_index, result)) = futures.next().await {
        match result {
            Ok(prs) => {
                if verbose {
                    eprintln!(
                        "  Found {} PRs for {}",
                        prs.len(),
                        name.as_deref().unwrap_or(&query)
                    );
                }
                // Extend with (pr, query_index) pairs to track which query each PR came from
                all_prs.extend(prs.into_iter().map(|pr| (pr, query_index)));
                any_succeeded = true;
            }
            Err(e) => {
                // If it's an auth error, bail immediately (all queries will fail)
                if e.downcast_ref::<AuthError>().is_some() {
                    return Err(e);
                }
                if verbose {
                    eprintln!(
                        "Query failed: {} - {}",
                        name.as_deref().unwrap_or(&query),
                        e
                    );
                }
            }
        }
    }

    // If all queries failed, return error
    if !any_succeeded && !config.queries.is_empty() {
        anyhow::bail!("All queries failed. Check your network connection and GitHub token.");
    }

    // Deduplicate PRs by URL (same PR may appear in multiple queries)
    // First match wins: track both unique PRs and their query index
    let mut seen_urls = HashSet::new();
    let mut pr_to_query_index = HashMap::new();
    let unique_prs: Vec<_> = all_prs
        .into_iter()
        .filter_map(|(pr, query_idx)| {
            if seen_urls.insert(pr.url.clone()) {
                pr_to_query_index.insert(pr.url.clone(), query_idx);
                Some(pr)
            } else {
                None
            }
        })
        .collect();

    if verbose {
        eprintln!("After deduplication: {} unique PRs", unique_prs.len());
    }

    // Split into active and snoozed
    let active_prs = filter_active_prs(unique_prs.clone(), snooze_state);
    let snoozed_prs = filter_snoozed_prs(unique_prs, snooze_state);

    if verbose {
        eprintln!(
            "After filter: {} active, {} snoozed",
            active_prs.len(),
            snoozed_prs.len()
        );
    }

    // Score active PRs (merge per-query scoring config with global for each PR)
    let mut active_scored: Vec<_> = active_prs
        .into_iter()
        .map(|pr| {
            // Look up which query this PR came from and merge its scoring config
            let query_idx = pr_to_query_index.get(&pr.url).copied().unwrap_or(0);
            let scoring =
                merge_scoring_configs(&global_scoring, config.queries[query_idx].scoring.as_ref());
            let result = calculate_score(&pr, &scoring);
            (pr, result)
        })
        .collect();

    // Score snoozed PRs (merge per-query scoring config with global for each PR)
    let mut snoozed_scored: Vec<_> = snoozed_prs
        .into_iter()
        .map(|pr| {
            // Look up which query this PR came from and merge its scoring config
            let query_idx = pr_to_query_index.get(&pr.url).copied().unwrap_or(0);
            let scoring =
                merge_scoring_configs(&global_scoring, config.queries[query_idx].scoring.as_ref());
            let result = calculate_score(&pr, &scoring);
            (pr, result)
        })
        .collect();

    // Sort both lists by score descending, then by age ascending (older first for ties)
    let sort_fn = |a: &(PullRequest, ScoreResult), b: &(PullRequest, ScoreResult)| {
        // Primary: score descending
        let score_cmp =
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }
        // Tie-breaker: age ascending (older first = smaller created_at)
        a.0.created_at.cmp(&b.0.created_at)
    };

    active_scored.sort_by(sort_fn);
    snoozed_scored.sort_by(sort_fn);

    // Fetch rate limit info (best-effort, don't fail the whole fetch if unavailable)
    let rate_limit_remaining = match client.ratelimit().get().await {
        Ok(rate_limit) => Some(rate_limit.resources.core.remaining as u64),
        Err(_) => None,
    };

    Ok((active_scored, snoozed_scored, rate_limit_remaining))
}
