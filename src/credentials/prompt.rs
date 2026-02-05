use anyhow::{Context, Result};

/// Prompts user to enter GitHub personal access token
pub fn prompt_for_token() -> Result<String> {
    println!("GitHub personal access token required.");
    println!("Create one at: https://github.com/settings/tokens");
    println!("Required scopes: repo (for private repos) or public_repo (for public only)");
    println!();

    let token = rpassword::prompt_password("Enter token: ")
        .context("Failed to read token from stdin")?;

    let token = token.trim();

    if token.is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    Ok(token.to_string())
}

/// Re-prompts for token when the existing one is rejected by GitHub
pub fn reprompt_for_token() -> Result<String> {
    eprintln!();
    eprintln!("Your GitHub token was rejected (invalid or expired).");
    eprintln!("Please provide a new token.");
    eprintln!();

    let token = prompt_for_token()?;

    eprintln!("New token provided. Set PR_BRO_GH_TOKEN in your shell profile to persist it.");

    Ok(token)
}

/// Setup token if missing - checks env var, then prompts interactively
/// Returns the token (either from env var or newly prompted)
pub fn setup_token_if_missing() -> Result<String> {
    // Check env var first
    if let Some(token) = super::get_token_from_env() {
        return Ok(token);
    }

    // No env var set, prompt for token
    let token = prompt_for_token()?;

    eprintln!(
        "Token accepted for this session. To persist, set PR_BRO_GH_TOKEN in your shell profile:\n  \
         export PR_BRO_GH_TOKEN=\"your_token_here\""
    );

    Ok(token)
}
