pub mod app;
pub mod event;
pub mod theme;
pub mod ui;

pub use app::App;
pub use theme::{resolve_theme, Theme, ThemeColors};

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use event::{Event, EventHandler};

pub async fn run_tui(mut app: App, mut client: octocrab::Octocrab) -> anyhow::Result<()> {
    // Buffer stderr while TUI is active to prevent output corrupting the display
    crate::stderr_buffer::activate();

    // Init terminal (sets up panic hooks automatically)
    let mut terminal = ratatui::init();

    // Create event handler with tick rate and auto-refresh interval
    let refresh_secs = app.config.auto_refresh_interval;
    let mut events = EventHandler::new(250, refresh_secs); // 250ms tick, N-second refresh

    // Spawn initial fetch as background task
    let client_clone = client.clone();
    let config_clone = app.config.clone();
    let snooze_clone = app.snooze_state.clone();
    let cache_config_clone = app.cache_config.clone();
    let verbose = app.verbose;
    let auth_username_clone = app.auth_username.clone();

    let mut pending_fetch: Option<tokio::task::JoinHandle<_>> = Some(tokio::spawn(async move {
        tokio::time::timeout(
            Duration::from_secs(20),
            crate::fetch::fetch_and_score_prs(
                &client_clone,
                &config_clone,
                &snooze_clone,
                &cache_config_clone,
                verbose,
                auth_username_clone.as_deref(),
            ),
        )
        .await
    }));
    app.is_loading = true;

    // Spawn background version check (after TUI renders, non-blocking)
    let mut pending_version_check: Option<tokio::task::JoinHandle<_>> = if !app.no_version_check {
        // First, check if we have a fresh cached result (synchronous, instant)
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let cached_status = crate::version_check::load_cached_status(&current_version);
        match &cached_status {
            crate::version_check::VersionStatus::UpdateAvailable { .. } => {
                app.set_version_status(cached_status);
                None // No need to fetch, cache is fresh
            }
            _ => {
                // Spawn background check
                let token = std::env::var("PR_BRO_GH_TOKEN").ok();
                token.map(|t| {
                    tokio::spawn(async move {
                        crate::version_check::check_version(&t, &current_version).await
                    })
                })
            }
        }
    } else {
        None
    };

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        // Handle events
        match events.next().await {
            Event::Key(key) => {
                app.last_interaction = std::time::Instant::now();
                handle_key_event(&mut app, key);
            }
            Event::Tick => {
                app.update_flash();
                app.advance_spinner();
            }
            Event::Refresh => {
                app.needs_refresh = true;
            }
        }

        // Check if background fetch has completed
        if let Some(handle) = &mut pending_fetch {
            if handle.is_finished() {
                let handle = pending_fetch.take().unwrap();
                match handle.await {
                    Ok(Ok(Ok((active, snoozed, rate_limit)))) => {
                        app.update_prs(active, snoozed, rate_limit);
                    }
                    Ok(Ok(Err(e))) => {
                        if e.downcast_ref::<crate::fetch::AuthError>().is_some() {
                            // Auth failure: restore terminal, re-prompt, re-init
                            ratatui::restore();

                            match crate::credentials::reprompt_for_token() {
                                Ok(new_token) => {
                                    // Recreate client with new token
                                    match crate::github::create_client(
                                        &new_token,
                                        &app.cache_config,
                                    ) {
                                        Ok((new_client, new_cache_handle)) => {
                                            client = new_client.clone();
                                            if new_cache_handle.is_some() {
                                                app.cache_handle = new_cache_handle;
                                            }

                                            // Re-fetch authenticated username
                                            let new_username = new_client
                                                .current()
                                                .user()
                                                .await
                                                .ok()
                                                .map(|u| u.login);
                                            app.auth_username = new_username;

                                            // Re-init terminal
                                            terminal = ratatui::init();

                                            // Trigger immediate refresh with new client
                                            app.needs_refresh = true;
                                            app.show_flash(
                                                "Re-authenticated. Refreshing...".to_string(),
                                            );
                                        }
                                        Err(ce) => {
                                            // Re-init terminal even on failure (must restore TUI)
                                            terminal = ratatui::init();
                                            app.show_flash(format!("Re-auth failed: {}", ce));
                                        }
                                    }
                                }
                                Err(pe) => {
                                    // User cancelled or error during prompt
                                    // Re-init terminal (must restore TUI)
                                    terminal = ratatui::init();
                                    app.show_flash(format!("Re-auth cancelled: {}", pe));
                                }
                            }
                        } else {
                            app.show_flash(format!("Refresh failed: {}", e));
                        }
                    }
                    Ok(Err(_elapsed)) => {
                        // Timeout: fetch took longer than 20 seconds
                        app.show_flash(
                            "Refresh timed out (20s). Will retry on next refresh.".to_string(),
                        );
                    }
                    Err(e) => {
                        app.show_flash(format!("Refresh task panicked: {}", e));
                    }
                }
                app.is_loading = false;
            }
        }

        // Check if background version check completed
        if let Some(handle) = &mut pending_version_check {
            if handle.is_finished() {
                let handle = pending_version_check.take().unwrap();
                if let Ok(status) = handle.await {
                    app.set_version_status(status);
                }
                // Silently ignore join errors
            }
        }

        // Spawn new refresh if needed and no fetch is pending
        if app.needs_refresh && pending_fetch.is_none() {
            // Check if this is a manual refresh (force_refresh) or auto-refresh
            let is_manual = app.force_refresh;
            let modal_open = app.input_mode != app::InputMode::Normal;
            let recent_interaction = app.last_interaction.elapsed() < Duration::from_secs(10);

            // Suppress auto-refresh if modal is open or user interacted recently.
            // Manual refresh ('r' key) always proceeds.
            // When suppressed, needs_refresh stays true so it retries on the next tick.
            if is_manual || (!modal_open && !recent_interaction) {
                app.needs_refresh = false;

                if is_manual {
                    if let Some(cache) = &app.cache_handle {
                        cache.clear_memory();
                    }
                    app.force_refresh = false;
                }

                // Spawn background fetch
                let client_clone = client.clone();
                let config_clone = app.config.clone();
                let snooze_clone = app.snooze_state.clone();
                let cache_config_clone = app.cache_config.clone();
                let verbose = app.verbose;
                let auth_username_clone = app.auth_username.clone();

                pending_fetch = Some(tokio::spawn(async move {
                    tokio::time::timeout(
                        Duration::from_secs(20),
                        crate::fetch::fetch_and_score_prs(
                            &client_clone,
                            &config_clone,
                            &snooze_clone,
                            &cache_config_clone,
                            verbose,
                            auth_username_clone.as_deref(),
                        ),
                    )
                    .await
                }));
                app.is_loading = true;
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    ratatui::restore();

    // Flush buffered stderr messages now that the terminal is restored
    for msg in crate::stderr_buffer::drain() {
        eprintln!("{}", msg);
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        app::InputMode::Normal => {
            match key.code {
                // Quit
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true
                }

                // Navigation
                KeyCode::Char('j') | KeyCode::Down => app.next_row(),
                KeyCode::Char('k') | KeyCode::Up => app.previous_row(),

                // Open PR in browser
                KeyCode::Enter | KeyCode::Char('o') => {
                    if let Some(pr) = app.selected_pr() {
                        let title = pr.title.clone();
                        if let Err(e) = app.open_selected() {
                            app.show_flash(format!("Failed to open browser: {}", e));
                        } else {
                            app.show_flash(format!("Opened: {}", title));
                        }
                    }
                }

                // Snooze
                KeyCode::Char('s') => app.start_snooze_input(),

                // Unsnooze
                KeyCode::Char('u') => app.unsnooze_selected(),

                // Undo
                KeyCode::Char('z') => app.undo_last(),

                // Tab switching
                KeyCode::Tab => app.toggle_view(),

                // Refresh (manual = force fresh data)
                KeyCode::Char('r') => {
                    app.needs_refresh = true;
                    app.force_refresh = true;
                    app.show_flash("Refreshing (fresh data)...".to_string());
                }

                // Help
                KeyCode::Char('?') => app.show_help(),

                // Score breakdown
                KeyCode::Char('b') => app.show_score_breakdown(),

                // Dismiss update banner
                KeyCode::Char('x') => {
                    if app.has_update_banner() {
                        app.dismiss_update_banner();
                    }
                }

                _ => {}
            }
        }
        app::InputMode::SnoozeInput => {
            match key.code {
                // Confirm snooze
                KeyCode::Enter => app.confirm_snooze_input(),

                // Cancel snooze
                KeyCode::Esc => app.cancel_snooze_input(),

                // Backspace
                KeyCode::Backspace => {
                    app.snooze_input.pop();
                }

                // Character input (alphanumeric + space)
                KeyCode::Char(c) if c.is_alphanumeric() || c == ' ' => {
                    app.snooze_input.push(c);
                }

                // Ignore all other keys (don't propagate to Normal mode)
                _ => {}
            }
        }
        app::InputMode::ScoreBreakdown => match key.code {
            KeyCode::Esc | KeyCode::Char('b') => app.dismiss_score_breakdown(),
            KeyCode::Char('j') | KeyCode::Down => app.next_row(),
            KeyCode::Char('k') | KeyCode::Up => app.previous_row(),
            _ => {}
        },
        app::InputMode::Help => {
            // Any key exits help
            app.dismiss_help();
        }
    }
}
