//! Desktop/terminal notifications for PRs newly appearing in the active list.
//!
//! The comparison is purely in-memory: each refresh diffs the incoming PR list
//! against the one it replaces. Nothing is persisted, so a restart simply
//! re-seeds from the first fetch without notifying.

use std::collections::HashSet;

use crate::config::NotificationConfig;

/// Count PRs in `incoming` whose URL was not present in `previous`.
///
/// `previous` is empty on the very first fetch of a session; callers are
/// expected to skip notifying in that case rather than announcing every PR.
pub fn count_new<'a>(previous: &HashSet<String>, incoming: impl Iterator<Item = &'a str>) -> usize {
    incoming.filter(|url| !previous.contains(*url)).count()
}

/// Human-readable notification body for `count` new PRs.
fn message(count: usize) -> String {
    if count == 1 {
        "1 PR needs your review".to_string()
    } else {
        format!("{} PRs need your review", count)
    }
}

/// Fire a single notification for `count` newly-appeared PRs.
///
/// Never fails the caller: delivery problems are reported through the stderr
/// buffer so they surface after the TUI restores the terminal.
pub fn notify(config: &NotificationConfig, count: usize) {
    if !config.enabled || count == 0 {
        return;
    }

    if let Some(command) = &config.command {
        run_command(command, count);
        return;
    }

    if config.sound {
        bell();
    }

    if config.desktop {
        desktop(count);
    }
}

/// Ring the terminal bell.
///
/// BEL renders nothing and does not move the cursor, so writing it mid-draw
/// does not corrupt the TUI's buffer.
fn bell() {
    use std::io::Write;

    let mut stdout = std::io::stdout();
    if write!(stdout, "\x07").is_ok() {
        let _ = stdout.flush();
    }
}

fn desktop(count: usize) {
    let result = notify_rust::Notification::new()
        .summary("PR Bro")
        .body(&message(count))
        .show();

    if let Err(e) = result {
        crate::stderr_buffer::warn(format!("Desktop notification failed: {}", e));
    }
}

/// Run the user's command through `sh -c`, substituting `{count}`.
///
/// Spawned without waiting so a slow command cannot stall the refresh.
fn run_command(command: &str, count: usize) {
    let rendered = command.replace("{count}", &count.to_string());

    let spawned = std::process::Command::new("sh")
        .arg("-c")
        .arg(&rendered)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    if let Err(e) = spawned {
        crate::stderr_buffer::warn(format!("Notification command failed: {}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(urls: &[&str]) -> HashSet<String> {
        urls.iter().map(|u| u.to_string()).collect()
    }

    #[test]
    fn counts_prs_absent_from_previous() {
        let previous = set(&["a", "b"]);
        assert_eq!(count_new(&previous, ["a", "b", "c"].into_iter()), 1);
    }

    #[test]
    fn unchanged_list_is_not_new() {
        let previous = set(&["a", "b"]);
        assert_eq!(count_new(&previous, ["b", "a"].into_iter()), 0);
    }

    #[test]
    fn swapped_pr_counts_despite_unchanged_length() {
        // Same number of PRs in and out, but one was replaced: a length
        // comparison would miss this entirely.
        let previous = set(&["a", "b", "c"]);
        assert_eq!(count_new(&previous, ["a", "b", "d"].into_iter()), 1);
    }

    #[test]
    fn removed_prs_do_not_count() {
        let previous = set(&["a", "b", "c"]);
        assert_eq!(count_new(&previous, ["a"].into_iter()), 0);
    }

    #[test]
    fn empty_previous_counts_everything() {
        // Callers skip the first fetch; the function itself stays honest.
        let previous = HashSet::new();
        assert_eq!(count_new(&previous, ["a", "b"].into_iter()), 2);
    }

    #[test]
    fn message_is_singular_for_one() {
        assert_eq!(message(1), "1 PR needs your review");
        assert_eq!(message(3), "3 PRs need your review");
    }

    /// Manual check for the platform notification backend, which cannot be
    /// asserted on automatically. Run with:
    ///   cargo test --lib notify -- --ignored --nocapture
    #[test]
    #[ignore = "fires a real notification"]
    fn fires_a_real_notification() {
        notify(
            &NotificationConfig {
                enabled: true,
                sound: true,
                desktop: true,
                command: None,
            },
            3,
        );
    }

    #[test]
    fn disabled_config_is_a_no_op() {
        let config = NotificationConfig::default();
        assert!(!config.enabled);
        notify(&config, 5); // must not panic or emit
    }
}
