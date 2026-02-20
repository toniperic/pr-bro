use crate::config::Config;
use crate::github::cache::{CacheConfig, DiskCache};
use crate::github::types::PullRequest;
use crate::scoring::ScoreResult;
use crate::snooze::SnoozeState;
use crate::tui::theme::{Theme, ThemeColors};
use crate::version_check::VersionStatus;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const MAX_UNDO: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Active,
    Snoozed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    SnoozeInput,
    Help,
    ScoreBreakdown,
}

#[derive(Debug, Clone)]
pub enum UndoAction {
    Snoozed {
        url: String,
        title: String,
    },
    Unsnoozed {
        url: String,
        title: String,
        until: Option<DateTime<Utc>>,
    },
    Resnooze {
        url: String,
        title: String,
        previous_until: Option<DateTime<Utc>>,
    },
}

pub struct App {
    pub active_prs: Vec<(PullRequest, ScoreResult)>,
    pub snoozed_prs: Vec<(PullRequest, ScoreResult)>,
    pub table_state: ratatui::widgets::TableState,
    pub current_view: View,
    pub snooze_state: SnoozeState,
    pub snooze_path: PathBuf,
    pub input_mode: InputMode,
    pub snooze_input: String,
    pub flash_message: Option<(String, Instant)>,
    pub undo_stack: VecDeque<UndoAction>,
    pub last_refresh: Instant,
    pub needs_refresh: bool,
    pub force_refresh: bool,
    pub should_quit: bool,
    pub config: Config,
    pub cache_config: CacheConfig,
    pub cache_handle: Option<Arc<DiskCache>>,
    pub verbose: bool,
    pub is_loading: bool,
    pub spinner_frame: usize,
    pub rate_limit_remaining: Option<u64>,
    pub auth_username: Option<String>,
    pub version_status: VersionStatus,
    pub no_version_check: bool,
    pub theme: Theme,
    pub theme_colors: ThemeColors,
    pub last_interaction: Instant,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        active_prs: Vec<(PullRequest, ScoreResult)>,
        snoozed_prs: Vec<(PullRequest, ScoreResult)>,
        snooze_state: SnoozeState,
        snooze_path: PathBuf,
        config: Config,
        cache_config: CacheConfig,
        cache_handle: Option<Arc<DiskCache>>,
        verbose: bool,
        auth_username: Option<String>,
        no_version_check: bool,
        theme: Theme,
    ) -> Self {
        let mut table_state = ratatui::widgets::TableState::default();
        if !active_prs.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            active_prs,
            snoozed_prs,
            table_state,
            current_view: View::Active,
            snooze_state,
            snooze_path,
            input_mode: InputMode::Normal,
            snooze_input: String::new(),
            flash_message: None,
            undo_stack: VecDeque::new(),
            last_refresh: Instant::now(),
            needs_refresh: false,
            force_refresh: false,
            should_quit: false,
            config,
            cache_config,
            cache_handle,
            verbose,
            is_loading: false,
            spinner_frame: 0,
            rate_limit_remaining: None,
            auth_username,
            version_status: VersionStatus::Unknown,
            no_version_check,
            theme,
            theme_colors: ThemeColors::new(theme),
            last_interaction: Instant::now(),
        }
    }

    /// Create a new App with empty PR lists in loading state
    /// Used for launching TUI before data arrives
    #[allow(clippy::too_many_arguments)]
    pub fn new_loading(
        snooze_state: SnoozeState,
        snooze_path: PathBuf,
        config: Config,
        cache_config: CacheConfig,
        cache_handle: Option<Arc<DiskCache>>,
        verbose: bool,
        auth_username: Option<String>,
        no_version_check: bool,
        theme: Theme,
    ) -> Self {
        Self {
            active_prs: Vec::new(),
            snoozed_prs: Vec::new(),
            table_state: ratatui::widgets::TableState::default(),
            current_view: View::Active,
            snooze_state,
            snooze_path,
            input_mode: InputMode::Normal,
            snooze_input: String::new(),
            flash_message: None,
            undo_stack: VecDeque::new(),
            last_refresh: Instant::now(),
            needs_refresh: false,
            force_refresh: false,
            should_quit: false,
            config,
            cache_config,
            cache_handle,
            verbose,
            is_loading: true,
            spinner_frame: 0,
            rate_limit_remaining: None,
            auth_username,
            version_status: VersionStatus::Unknown,
            no_version_check,
            theme,
            theme_colors: ThemeColors::new(theme),
            last_interaction: Instant::now(),
        }
    }

    pub fn current_prs(&self) -> &[(PullRequest, ScoreResult)] {
        match self.current_view {
            View::Active => &self.active_prs,
            View::Snoozed => &self.snoozed_prs,
        }
    }

    pub fn next_row(&mut self) {
        let prs = self.current_prs();
        if prs.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= prs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous_row(&mut self) {
        let prs = self.current_prs();
        if prs.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    prs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn selected_pr(&self) -> Option<&PullRequest> {
        let prs = self.current_prs();
        self.table_state
            .selected()
            .and_then(|i| prs.get(i).map(|(pr, _)| pr))
    }

    pub fn push_undo(&mut self, action: UndoAction) {
        self.undo_stack.push_front(action);
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.pop_back();
        }
    }

    pub fn update_flash(&mut self) {
        if let Some((_, timestamp)) = self.flash_message {
            if timestamp.elapsed().as_secs() >= 3 {
                self.flash_message = None;
            }
        }
    }

    pub fn show_flash(&mut self, msg: String) {
        self.flash_message = Some((msg, Instant::now()));
    }

    pub fn auto_refresh_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.auto_refresh_interval)
    }

    /// Open the selected PR in the browser
    pub fn open_selected(&self) -> anyhow::Result<()> {
        if let Some(pr) = self.selected_pr() {
            crate::browser::open_url(&pr.url)?;
        }
        Ok(())
    }

    /// Start snooze input mode (works on both Active and Snoozed views)
    pub fn start_snooze_input(&mut self) {
        if self.selected_pr().is_some() {
            self.input_mode = InputMode::SnoozeInput;
            self.snooze_input.clear();
        }
    }

    /// Confirm and apply the snooze input
    pub fn confirm_snooze_input(&mut self) {
        // Get selected PR info before mutating
        let (url, title) = match self.selected_pr() {
            Some(pr) => (pr.url.clone(), pr.title.clone()),
            None => {
                self.input_mode = InputMode::Normal;
                return;
            }
        };

        // Parse duration from input
        let computed_until = if self.snooze_input.trim().is_empty() {
            // Empty string = indefinite snooze
            None
        } else {
            // Parse duration string
            match humantime::parse_duration(&self.snooze_input) {
                Ok(duration) => {
                    let until =
                        Utc::now() + chrono::Duration::from_std(duration).unwrap_or_default();
                    Some(until)
                }
                Err(_) => {
                    self.show_flash(format!("Invalid duration: '{}'", self.snooze_input));
                    self.input_mode = InputMode::Normal;
                    self.snooze_input.clear();
                    return;
                }
            }
        };

        // Capture old snooze_until before overwriting (needed for undo on re-snooze)
        let old_until = self
            .snooze_state
            .snoozed_entries()
            .get(&url)
            .and_then(|entry| entry.snooze_until);

        // Apply snooze
        self.snooze_state.snooze(url.clone(), computed_until);

        // Save to disk
        if let Err(e) = crate::snooze::save_snooze_state(&self.snooze_path, &self.snooze_state) {
            self.show_flash(format!("Failed to save snooze state: {}", e));
            self.input_mode = InputMode::Normal;
            return;
        }

        // Branch behavior based on current view
        match self.current_view {
            View::Active => {
                // Push to undo stack
                self.push_undo(UndoAction::Snoozed {
                    url: url.clone(),
                    title: title.clone(),
                });

                // Move PR from active to snoozed
                self.move_pr_between_lists(&url, true);

                // Show flash message
                self.show_flash(format!("Snoozed: {} (z to undo)", title));
            }
            View::Snoozed => {
                // Push re-snooze to undo stack with previous duration
                self.push_undo(UndoAction::Resnooze {
                    url: url.clone(),
                    title: title.clone(),
                    previous_until: old_until,
                });

                // PR stays in snoozed list -- no move needed
                self.show_flash(format!("Re-snoozed: {} (z to undo)", title));
            }
        }

        // Return to normal mode
        self.input_mode = InputMode::Normal;
        self.snooze_input.clear();
    }

    /// Cancel snooze input
    pub fn cancel_snooze_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.snooze_input.clear();
    }

    /// Unsnooze the selected PR (only works in Snoozed view)
    pub fn unsnooze_selected(&mut self) {
        if !matches!(self.current_view, View::Snoozed) {
            return;
        }

        let (url, title, until) = match self.selected_pr() {
            Some(pr) => {
                let url = pr.url.clone();
                let title = pr.title.clone();
                // Look up snooze entry to get the until time for undo
                let until = self
                    .snooze_state
                    .snoozed_entries()
                    .get(&url)
                    .and_then(|entry| entry.snooze_until);
                (url, title, until)
            }
            None => return,
        };

        // Unsnooze
        self.snooze_state.unsnooze(&url);

        // Save to disk
        if let Err(e) = crate::snooze::save_snooze_state(&self.snooze_path, &self.snooze_state) {
            self.show_flash(format!("Failed to save snooze state: {}", e));
            return;
        }

        // Push to undo stack
        self.push_undo(UndoAction::Unsnoozed {
            url: url.clone(),
            title: title.clone(),
            until,
        });

        // Move PR from snoozed to active
        self.move_pr_between_lists(&url, false);

        // Show flash message
        self.show_flash(format!("Unsnoozed: {} (z to undo)", title));
    }

    /// Undo the last snooze or unsnooze action
    pub fn undo_last(&mut self) {
        let action = match self.undo_stack.pop_front() {
            Some(action) => action,
            None => {
                self.show_flash("Nothing to undo".to_string());
                return;
            }
        };

        match action {
            UndoAction::Snoozed { url, title } => {
                // Undo a snooze: unsnooze the PR
                self.snooze_state.unsnooze(&url);

                // Save to disk
                if let Err(e) =
                    crate::snooze::save_snooze_state(&self.snooze_path, &self.snooze_state)
                {
                    self.show_flash(format!("Failed to save snooze state: {}", e));
                    return;
                }

                // Move PR back from snoozed to active
                self.move_pr_between_lists(&url, false);

                self.show_flash(format!("Undid snooze: {}", title));
            }
            UndoAction::Unsnoozed { url, title, until } => {
                // Undo an unsnooze: re-snooze the PR
                self.snooze_state.snooze(url.clone(), until);

                // Save to disk
                if let Err(e) =
                    crate::snooze::save_snooze_state(&self.snooze_path, &self.snooze_state)
                {
                    self.show_flash(format!("Failed to save snooze state: {}", e));
                    return;
                }

                // Move PR back from active to snoozed
                self.move_pr_between_lists(&url, true);

                self.show_flash(format!("Undid unsnooze: {}", title));
            }
            UndoAction::Resnooze {
                url,
                title,
                previous_until,
            } => {
                // Undo a re-snooze: restore the previous snooze duration
                self.snooze_state.snooze(url.clone(), previous_until);

                // Save to disk
                if let Err(e) =
                    crate::snooze::save_snooze_state(&self.snooze_path, &self.snooze_state)
                {
                    self.show_flash(format!("Failed to save snooze state: {}", e));
                    return;
                }

                // PR stays in snoozed list -- no move needed
                self.show_flash(format!("Undid re-snooze: {}", title));
            }
        }
    }

    /// Move a PR between active and snoozed lists
    ///
    /// # Arguments
    /// * `url` - The URL of the PR to move
    /// * `from_active_to_snoozed` - true to move from active to snoozed, false for the reverse
    fn move_pr_between_lists(&mut self, url: &str, from_active_to_snoozed: bool) {
        let (source_list, dest_list) = if from_active_to_snoozed {
            (&mut self.active_prs, &mut self.snoozed_prs)
        } else {
            (&mut self.snoozed_prs, &mut self.active_prs)
        };

        // Find and remove PR from source list
        if let Some(pos) = source_list.iter().position(|(pr, _)| pr.url == url) {
            let pr_entry = source_list.remove(pos);

            // Insert into destination list, maintaining score-descending sort
            let insert_pos = dest_list
                .iter()
                .position(|(_, score)| score.score < pr_entry.1.score)
                .unwrap_or(dest_list.len());
            dest_list.insert(insert_pos, pr_entry);

            // Fix table selection to stay valid
            let current_list = self.current_prs();
            if current_list.is_empty() {
                self.table_state.select(None);
            } else if let Some(selected) = self.table_state.selected() {
                if selected >= current_list.len() {
                    self.table_state.select(Some(current_list.len() - 1));
                }
            }
        }
    }

    /// Toggle between Active and Snoozed views
    pub fn toggle_view(&mut self) {
        self.current_view = match self.current_view {
            View::Active => View::Snoozed,
            View::Snoozed => View::Active,
        };

        // Reset selection to first item in the new view, or None if empty
        let prs = self.current_prs();
        if prs.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    /// Show help overlay
    pub fn show_help(&mut self) {
        self.input_mode = InputMode::Help;
    }

    /// Dismiss help overlay
    pub fn dismiss_help(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Show score breakdown overlay
    pub fn show_score_breakdown(&mut self) {
        if self.selected_pr().is_some() {
            self.input_mode = InputMode::ScoreBreakdown;
        }
    }

    /// Dismiss score breakdown overlay
    pub fn dismiss_score_breakdown(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Get the selected PR's ScoreResult
    pub fn selected_score_result(&self) -> Option<&crate::scoring::ScoreResult> {
        let prs = self.current_prs();
        self.table_state
            .selected()
            .and_then(|i| prs.get(i).map(|(_, sr)| sr))
    }

    /// Update PRs with fresh data from fetch
    pub fn update_prs(
        &mut self,
        active: Vec<(PullRequest, ScoreResult)>,
        snoozed: Vec<(PullRequest, ScoreResult)>,
        rate_limit_remaining: Option<u64>,
    ) {
        // Replace PR lists
        self.active_prs = active;
        self.snoozed_prs = snoozed;

        // Update rate limit info
        self.rate_limit_remaining = rate_limit_remaining;

        // Preserve selection if possible
        let current_list = self.current_prs();
        if current_list.is_empty() {
            self.table_state.select(None);
        } else if let Some(selected) = self.table_state.selected() {
            // Clamp to new list length
            if selected >= current_list.len() {
                self.table_state.select(Some(current_list.len() - 1));
            }
        } else {
            // No selection before, select first if list is non-empty
            self.table_state.select(Some(0));
        }

        // Reload snooze state from disk (in case it was modified externally)
        if let Ok(loaded_state) = crate::snooze::load_snooze_state(&self.snooze_path) {
            self.snooze_state = loaded_state;
        }

        // Update refresh timestamp
        self.last_refresh = Instant::now();

        // Show flash message
        let active_count = self.active_prs.len();
        let snoozed_count = self.snoozed_prs.len();
        self.show_flash(format!(
            "Refreshed ({} active, {} snoozed)",
            active_count, snoozed_count
        ));
    }

    /// Advance the loading spinner animation frame
    pub fn advance_spinner(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Set the version check status
    pub fn set_version_status(&mut self, status: VersionStatus) {
        self.version_status = status;
    }

    /// Dismiss the update banner and persist the dismissal
    pub fn dismiss_update_banner(&mut self) {
        if let VersionStatus::UpdateAvailable { latest, .. } = &self.version_status {
            crate::version_check::dismiss_version(latest);
            self.version_status = VersionStatus::UpToDate;
            self.show_flash("Update notice dismissed".to_string());
        }
    }

    /// Check if the update banner should be shown
    pub fn has_update_banner(&self) -> bool {
        matches!(self.version_status, VersionStatus::UpdateAvailable { .. })
    }
}
