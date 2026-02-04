use anyhow::{anyhow, Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use octocrab::Octocrab;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::github::types::PullRequest;

/// Search GitHub for pull requests matching the given query.
/// Auth errors (401 / Bad credentials) fail immediately as a typed AuthError.
/// Rate limit and permission errors also fail immediately.
/// Transient/network errors are retried up to 3 times with exponential backoff.
pub async fn search_prs(client: &Octocrab, query: &str) -> Result<Vec<PullRequest>> {
    // Ensure the query only returns PRs, not issues
    let query = if query.contains("is:pr") {
        query.to_string()
    } else {
        format!("{} is:pr", query)
    };

    let max_retries = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;
        match client
            .search()
            .issues_and_pull_requests(&query)
            .send()
            .await
        {
            Ok(results) => {
                let prs: Vec<PullRequest> = results
                    .items
                    .into_iter()
                    .filter(|issue| issue.pull_request.is_some()) // Only PRs, not issues
                    .map(|issue| {
                        // Extract owner/repo from html_url
                        // Format: "https://github.com/owner/repo/pull/123"
                        let path = issue.html_url.path();
                        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                        let repo = if parts.len() >= 2 {
                            format!("{}/{}", parts[0], parts[1])
                        } else {
                            "unknown/unknown".to_string()
                        };

                        PullRequest {
                            title: issue.title,
                            number: issue.number,
                            author: issue.user.login.clone(),
                            repo,
                            url: issue.html_url.to_string(),
                            created_at: issue.created_at,
                            updated_at: issue.updated_at,
                            additions: 0, // Search API doesn't include these
                            deletions: 0, // Will be populated by enrichment
                            approvals: 0, // Requires separate API call
                            draft: false, // Search API doesn't expose draft status reliably
                            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
                            user_has_reviewed: false, // Will be populated by enrichment
                            filtered_size: None, // Will be set by enrich_pr if exclude patterns configured
                        }
                    })
                    .collect();
                return Ok(prs);
            }
            Err(e) => {
                let error_str = format!("{:?}", e);

                // Auth errors: fail immediately with typed AuthError (no retry)
                if error_str.contains("401") || error_str.contains("Bad credentials") {
                    return Err(crate::fetch::AuthError {
                        message:
                            "Authentication failed. Your GitHub token may be invalid or expired."
                                .to_string(),
                    }
                    .into());
                }

                // Rate limit: fail immediately (caller handles differently)
                if error_str.contains("rate limit") || error_str.contains("403") {
                    return Err(anyhow!(
                        "GitHub API rate limit exceeded. Wait a few minutes and try again."
                    ));
                }

                // Permission errors: fail immediately
                if error_str.contains("do not have permission")
                    || error_str.contains("resources do not exist")
                {
                    return Err(anyhow!("Repository not found or no access. Check repo name and token permissions (needs 'repo' scope for private repos)."));
                }

                // Transient errors: retry with backoff
                if attempt >= max_retries {
                    return Err(anyhow!(
                        "GitHub API error after {} attempts: {}",
                        max_retries,
                        e
                    ));
                }

                let delay = std::time::Duration::from_millis(100 * (1 << (attempt - 1))); // 100ms, 200ms, 400ms
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Fetch PR details (additions, deletions) from the GitHub API
async fn fetch_pr_details(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<(u64, u64)> {
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .context("Failed to fetch PR details")?;

    let additions = pr.additions.unwrap_or(0);
    let deletions = pr.deletions.unwrap_or(0);

    Ok((additions, deletions))
}

/// Fetch PR review count (approved reviews) and check if authenticated user has reviewed
async fn fetch_pr_reviews(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
    auth_username: Option<&str>,
) -> Result<(u32, bool)> {
    let reviews = client
        .pulls(owner, repo)
        .list_reviews(number)
        .send()
        .await
        .context("Failed to fetch PR reviews")?;

    let approved_count = reviews
        .items
        .iter()
        .filter(|review| {
            matches!(
                review.state,
                Some(octocrab::models::pulls::ReviewState::Approved)
            )
        })
        .count() as u32;

    // Check if authenticated user has reviewed (any review state counts)
    let user_has_reviewed = auth_username.is_some_and(|username| {
        reviews.items.iter().any(|r| {
            r.user
                .as_ref()
                .is_some_and(|u| u.login.eq_ignore_ascii_case(username))
        })
    });

    Ok((approved_count, user_has_reviewed))
}

/// Fetch per-file diff data for a PR with pagination.
/// Returns a list of (filename, additions, deletions) tuples.
async fn fetch_pr_file_list(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<Vec<(String, u64, u64)>> {
    let page = client
        .pulls(owner, repo)
        .list_files(number)
        .await
        .context("Failed to fetch PR file list")?;

    let all_files = client
        .all_pages(page)
        .await
        .context("Failed to paginate PR file list")?;

    Ok(all_files
        .into_iter()
        .map(|f| (f.filename, f.additions, f.deletions))
        .collect())
}

/// Filter files by basename glob matching and compute total size of non-excluded files.
fn apply_size_exclusions(files: &[(String, u64, u64)], exclude_patterns: &[String]) -> Result<u64> {
    let compiled: Vec<glob::Pattern> = exclude_patterns
        .iter()
        .map(|p| glob::Pattern::new(p).context(format!("Invalid glob pattern: {}", p)))
        .collect::<Result<Vec<_>>>()?;

    let total = files
        .iter()
        .filter(|(filename, _, _)| {
            let basename = std::path::Path::new(filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(filename);
            !compiled.iter().any(|pat| pat.matches(basename))
        })
        .map(|(_, additions, deletions)| additions + deletions)
        .sum();

    Ok(total)
}

/// Enrich a PR with detailed information (size and approvals)
async fn enrich_pr(
    client: &Octocrab,
    pr: &mut PullRequest,
    auth_username: Option<&str>,
    exclude_patterns: &Option<Vec<String>>,
) -> Result<()> {
    // Parse owner/repo from pr.repo field
    let parts: Vec<&str> = pr.repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid repo format: {}", pr.repo));
    }
    let owner = parts[0];
    let repo_name = parts[1];

    // Fetch details and reviews in parallel
    let details_fut = fetch_pr_details(client, owner, repo_name, pr.number);
    let reviews_fut = fetch_pr_reviews(client, owner, repo_name, pr.number, auth_username);

    match tokio::try_join!(details_fut, reviews_fut) {
        Ok(((additions, deletions), (approvals, user_has_reviewed))) => {
            pr.additions = additions;
            pr.deletions = deletions;
            pr.approvals = approvals;
            pr.user_has_reviewed = user_has_reviewed;

            // Conditionally fetch per-file data and apply size exclusions
            if let Some(ref patterns) = exclude_patterns {
                if !patterns.is_empty() {
                    match fetch_pr_file_list(client, owner, repo_name, pr.number).await {
                        Ok(files) => {
                            match apply_size_exclusions(&files, patterns) {
                                Ok(filtered) => pr.filtered_size = Some(filtered),
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to apply size exclusions for PR {}: {}",
                                        pr.number, e
                                    );
                                    // Leave filtered_size as None — fallback to aggregate size
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to fetch file list for PR {}: {}",
                                pr.number, e
                            );
                            // Leave filtered_size as None — fallback to aggregate size
                        }
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            // If enrichment fails, log but don't fail the whole operation
            eprintln!("Warning: Failed to enrich PR {}: {}", pr.number, e);
            Ok(())
        }
    }
}

/// Helper function for concurrent PR enrichment
async fn enrich_pr_with_rate_limit_check(
    client: Octocrab,
    mut pr: PullRequest,
    rate_limited: Arc<AtomicBool>,
    auth_username: Option<String>,
    exclude_patterns: Option<Vec<String>>,
) -> PullRequest {
    if rate_limited.load(Ordering::Relaxed) {
        return pr; // Skip enrichment if rate limited
    }

    match enrich_pr(
        &client,
        &mut pr,
        auth_username.as_deref(),
        &exclude_patterns,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("rate limit") || err_str.contains("403") {
                eprintln!("Warning: Rate limit hit during enrichment. Returning partial results.");
                rate_limited.store(true, Ordering::Relaxed);
            } else {
                eprintln!("Warning: Failed to enrich PR {}: {}", pr.number, e);
            }
        }
    }
    pr
}

/// Search and enrich PRs with full details
pub async fn search_and_enrich_prs(
    client: &Octocrab,
    query: &str,
    auth_username: Option<&str>,
    exclude_patterns: Option<Vec<String>>,
) -> Result<Vec<PullRequest>> {
    let prs = search_prs(client, query).await?;

    // Enrich PRs with bounded concurrency
    const MAX_CONCURRENT_ENRICHMENTS: usize = 10;

    // Rate limit flag shared across concurrent tasks
    let rate_limited = Arc::new(AtomicBool::new(false));

    let mut futures = FuturesUnordered::new();
    let mut prs_iter = prs.into_iter();
    let mut enriched_prs = Vec::new();

    // Fill initial batch
    for _ in 0..MAX_CONCURRENT_ENRICHMENTS {
        if let Some(pr) = prs_iter.next() {
            futures.push(enrich_pr_with_rate_limit_check(
                client.clone(),
                pr,
                rate_limited.clone(),
                auth_username.map(|s| s.to_string()),
                exclude_patterns.clone(),
            ));
        }
    }

    // Process results and feed new tasks
    while let Some(pr) = futures.next().await {
        enriched_prs.push(pr);

        // Add next PR if not rate limited
        if !rate_limited.load(Ordering::Relaxed) {
            if let Some(next_pr) = prs_iter.next() {
                futures.push(enrich_pr_with_rate_limit_check(
                    client.clone(),
                    next_pr,
                    rate_limited.clone(),
                    auth_username.map(|s| s.to_string()),
                    exclude_patterns.clone(),
                ));
            }
        }
    }

    // Add any remaining unenriched PRs (if rate limited, remaining weren't submitted)
    enriched_prs.extend(prs_iter);

    Ok(enriched_prs)
}
