use crate::tui::app::{App, InputMode, View};
use crate::tui::theme::ThemeColors;
use crate::version_check::VersionStatus;
use chrono::{Datelike, Local};
use ratatui::layout::Margin;
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    Tabs,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Handle very small terminal sizes gracefully
    if area.height < 6 || area.width < 30 {
        let msg = Paragraph::new("Terminal too small").alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

    // Layout: conditionally include banner row
    if app.has_update_banner() {
        let chunks = Layout::vertical([
            Constraint::Length(1), // Title bar
            Constraint::Length(1), // Update banner
            Constraint::Length(1), // Tab bar
            Constraint::Fill(1),   // PR table
            Constraint::Length(1), // Status bar
        ])
        .split(area);

        render_title(frame, chunks[0], app);
        render_update_banner(frame, chunks[1], app);
        render_tabs(frame, chunks[2], app);
        render_table(frame, chunks[3], app);
        render_status_bar(frame, chunks[4], app);
    } else {
        // Layout: Title(1) + Tabs(1) + Table(fill) + Status(1)
        let chunks = Layout::vertical([
            Constraint::Length(1), // Title bar
            Constraint::Length(1), // Tab bar
            Constraint::Fill(1),   // PR table
            Constraint::Length(1), // Status bar
        ])
        .split(area);

        render_title(frame, chunks[0], app);
        render_tabs(frame, chunks[1], app);
        render_table(frame, chunks[2], app);
        render_status_bar(frame, chunks[3], app);
    }

    // Render overlays based on input mode
    match app.input_mode {
        InputMode::SnoozeInput => render_snooze_popup(frame, app),
        InputMode::Help => render_help_popup(frame, app),
        InputMode::ScoreBreakdown => render_score_breakdown_popup(frame, app),
        InputMode::Normal => {}
    }

    // Render loading overlay if loading (appears on top of everything)
    if app.is_loading {
        render_loading_overlay(frame, app);
    }
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    // Build title with rate limit on the right
    let mut spans = vec![Span::styled(
        "PR Bro",
        Style::default().fg(app.theme_colors.title_color).bold(),
    )];

    // Add rate limit info on the right if available
    if let Some(remaining) = app.rate_limit_remaining {
        let rate_limit_text = format!("API: {} remaining", remaining);
        let left_len = "PR Bro".len();
        let right_len = rate_limit_text.len();
        let padding_len = (area.width as usize).saturating_sub(left_len + right_len);

        // Add padding and rate limit text
        spans.push(Span::raw(" ".repeat(padding_len)));
        spans.push(Span::styled(
            rate_limit_text,
            Style::default().fg(app.theme_colors.muted),
        ));
    }

    let title = Line::from(spans);
    frame.render_widget(Paragraph::new(title), area);
}

fn render_update_banner(frame: &mut Frame, area: Rect, app: &App) {
    if let VersionStatus::UpdateAvailable { current, latest } = &app.version_status {
        // Build banner text with styling
        let banner_text = Line::from(vec![
            Span::styled(
                format!("  Update available: v{} -> v{}  ", current, latest),
                Style::default().fg(app.theme_colors.banner_fg),
            ),
            Span::raw("["),
            Span::styled("x", Style::default().fg(app.theme_colors.banner_key).bold()),
            Span::styled(
                " to dismiss]",
                Style::default().fg(app.theme_colors.banner_fg),
            ),
        ]);

        let banner =
            Paragraph::new(banner_text).style(Style::default().bg(app.theme_colors.banner_bg));
        frame.render_widget(banner, area);
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec!["Active", "Snoozed"];
    let selected = match app.current_view {
        View::Active => 0,
        View::Snoozed => 1,
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(app.theme_colors.muted))
        .highlight_style(
            Style::default()
                .fg(app.theme_colors.title_color)
                .bold()
                .reversed(),
        )
        .divider(" | ")
        .padding("  ", "  ");

    frame.render_widget(tabs, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &mut App) {
    let prs = app.current_prs();

    if prs.is_empty() {
        let empty_msg = Paragraph::new("No PRs to review")
            .alignment(Alignment::Center)
            .block(Block::default());
        frame.render_widget(empty_msg, area);
        return;
    }

    // Calculate max score for bar scaling
    let max_score = prs
        .iter()
        .map(|(_, result)| result.score)
        .fold(0.0_f64, f64::max);

    // Store PR count and selected position for scrollbar (before borrowing table_state)
    let pr_count = prs.len();
    let selected_pos = app.table_state.selected().unwrap_or(0);

    // Build rows, widths, and header based on current view
    let (rows, widths, header_cells): (Vec<Row>, Vec<Constraint>, Vec<&str>) =
        if matches!(app.current_view, View::Snoozed) {
            // Snoozed view: 5 columns with Duration
            let rows: Vec<Row> = prs
                .iter()
                .enumerate()
                .map(|(idx, (pr, score_result))| {
                    let index = format!("{}.", idx + 1);
                    let score_str = format_score(score_result.score, score_result.incomplete);
                    let bar_line = score_bar(score_result.score, max_score, 8, &app.theme_colors);

                    // Build score cell with colored text and bar
                    let score_color = app.theme_colors.score_color(score_result.score, max_score);
                    let mut score_spans = vec![Span::styled(
                        format!("{:>5} ", score_str),
                        Style::default().fg(score_color),
                    )];
                    score_spans.extend(bar_line.spans);
                    let score_line = Line::from(score_spans);

                    let title = pr.title.clone();

                    // Get duration from snooze entry
                    let duration = app
                        .snooze_state
                        .snoozed_entries()
                        .get(&pr.url)
                        .map(|entry| entry.format_remaining())
                        .unwrap_or_else(|| "unknown".to_string());

                    // Alternating row background (odd rows get subtle background)
                    let row_style = if idx % 2 == 1 {
                        Style::default().bg(app.theme_colors.row_alt_bg)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(index).style(Style::default().fg(app.theme_colors.index_color)),
                        Cell::from(score_line),
                        Cell::from(title),
                        Cell::from(duration).style(Style::default().fg(app.theme_colors.muted)),
                        Cell::from(pr.short_ref()),
                    ])
                    .style(row_style)
                })
                .collect();

            let widths = vec![
                Constraint::Length(4),  // Index
                Constraint::Length(16), // Score + bar
                Constraint::Fill(1),    // Title
                Constraint::Length(12), // Duration: "indefinite" = 10 chars + padding
                Constraint::Length(40), // PR ref
            ];

            let header = vec!["#", "Score", "Title", "Duration", "PR"];

            (rows, widths, header)
        } else {
            // Active view: 4 columns without Duration
            let rows: Vec<Row> = prs
                .iter()
                .enumerate()
                .map(|(idx, (pr, score_result))| {
                    let index = format!("{}.", idx + 1);
                    let score_str = format_score(score_result.score, score_result.incomplete);
                    let bar_line = score_bar(score_result.score, max_score, 8, &app.theme_colors);

                    // Build score cell with colored text and bar
                    let score_color = app.theme_colors.score_color(score_result.score, max_score);
                    let mut score_spans = vec![Span::styled(
                        format!("{:>5} ", score_str),
                        Style::default().fg(score_color),
                    )];
                    score_spans.extend(bar_line.spans);
                    let score_line = Line::from(score_spans);

                    let title = pr.title.clone();

                    // Alternating row background (odd rows get subtle background)
                    let row_style = if idx % 2 == 1 {
                        Style::default().bg(app.theme_colors.row_alt_bg)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(index).style(Style::default().fg(app.theme_colors.index_color)),
                        Cell::from(score_line),
                        Cell::from(title),
                        Cell::from(pr.short_ref()),
                    ])
                    .style(row_style)
                })
                .collect();

            let widths = vec![
                Constraint::Length(4),  // Index: "99."
                Constraint::Length(16), // Score + bar: "12.3k ████░░░░"
                Constraint::Fill(1),    // Title
                Constraint::Length(40), // PR: "owner/repo-name#12345"
            ];

            let header = vec!["#", "Score", "Title", "PR"];

            (rows, widths, header)
        };

    let table = Table::new(rows, widths)
        .header(
            Row::new(header_cells)
                .style(app.theme_colors.header_style)
                .bottom_margin(1),
        )
        .row_highlight_style(app.theme_colors.row_selected);

    frame.render_stateful_widget(table, area, &mut app.table_state);

    // Render scrollbar if PR list exceeds visible area
    let visible_rows = area.height.saturating_sub(2) as usize; // Subtract header and margin
    if pr_count > visible_rows {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(app.theme_colors.scrollbar_thumb))
            .track_style(Style::default().fg(app.theme_colors.scrollbar_track));

        let mut scrollbar_state = ScrollbarState::new(pr_count).position(selected_pos);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some((ref msg, _)) = app.flash_message {
        // Show flash message with color based on message type
        let msg_color =
            if msg.starts_with("Failed") || msg.starts_with("Error") || msg.contains("cancelled") {
                app.theme_colors.flash_error
            } else if msg.starts_with("Snoozed:")
                || msg.starts_with("Unsnoozed:")
                || msg.starts_with("Re-snoozed:")
                || msg.starts_with("Undid")
                || msg.starts_with("Refreshed")
                || msg.starts_with("Opened:")
            {
                app.theme_colors.flash_success
            } else {
                Color::Reset // Default for unknown message types
            };
        Line::from(Span::styled(msg.clone(), Style::default().fg(msg_color)))
    } else {
        // Show normal status
        let prs = app.current_prs();
        let count = format!("{} PRs", prs.len());

        let view_mode = match app.current_view {
            View::Active => "Active",
            View::Snoozed => "Snoozed",
        };

        let elapsed = app.last_refresh.elapsed();
        let refresh_time = if elapsed.as_secs() < 60 {
            format!("refreshed {}s ago", elapsed.as_secs())
        } else {
            format!("refreshed {}m ago", elapsed.as_secs() / 60)
        };

        // Build hints with colored shortcut keys
        let mut hint_spans = Vec::new();
        let hints = match app.current_view {
            View::Active => vec![
                ("j", "/", "k", ":nav "),
                ("Enter", "", "", ":open "),
                ("b", "", "", ":breakdown "),
                ("s", "", "", ":snooze "),
                ("r", "", "", ":refresh "),
                ("Tab", "", "", ":snoozed "),
                ("?", "", "", ":help "),
                ("q", "", "", ":quit"),
            ],
            View::Snoozed => vec![
                ("j", "/", "k", ":nav "),
                ("Enter", "", "", ":open "),
                ("b", "", "", ":breakdown "),
                ("s", "", "", ":resnooze "),
                ("u", "", "", ":unsnooze "),
                ("r", "", "", ":refresh "),
                ("Tab", "", "", ":active "),
                ("?", "", "", ":help "),
                ("q", "", "", ":quit"),
            ],
        };

        for (i, (key1, sep, key2, label)) in hints.iter().enumerate() {
            if i > 0 {
                hint_spans.push(Span::raw(" "));
            }
            hint_spans.push(Span::styled(
                *key1,
                Style::default().fg(app.theme_colors.status_key_color),
            ));
            if !sep.is_empty() {
                hint_spans.push(Span::raw(*sep));
                hint_spans.push(Span::styled(
                    *key2,
                    Style::default().fg(app.theme_colors.status_key_color),
                ));
            }
            hint_spans.push(Span::raw(*label));
        }

        let mut spans = vec![
            Span::styled(count, Style::default().fg(app.theme_colors.muted)),
            Span::raw(" "),
            Span::styled(view_mode, Style::default().fg(app.theme_colors.muted)),
            Span::raw(" "),
            Span::styled(refresh_time, Style::default().fg(app.theme_colors.muted)),
            Span::raw("  "),
        ];
        spans.extend(hint_spans);
        Line::from(spans)
    };

    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(app.theme_colors.status_bar_bg)),
        area,
    );
}

fn format_score(score: f64, incomplete: bool) -> String {
    let formatted = if score >= 1_000_000.0 {
        format!("{:.1}M", score / 1_000_000.0)
    } else if score >= 1_000.0 {
        format!("{:.1}k", score / 1_000.0)
    } else {
        format!("{:.0}", score)
    };

    // Trim trailing .0
    let trimmed = formatted.replace(".0M", "M").replace(".0k", "k");

    if incomplete {
        format!("{}*", trimmed)
    } else {
        trimmed
    }
}

fn score_bar(
    score: f64,
    max_score: f64,
    width: usize,
    theme_colors: &ThemeColors,
) -> Line<'static> {
    let ratio = if max_score > 0.0 {
        (score / max_score).min(1.0)
    } else {
        0.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    // Get color based on score
    let bar_color = theme_colors.score_color(score, max_score);

    let mut spans = Vec::new();
    if filled > 0 {
        spans.push(Span::styled(
            "█".repeat(filled),
            Style::default().fg(bar_color),
        ));
    }
    if empty > 0 {
        spans.push(Span::styled(
            "░".repeat(empty),
            Style::default().fg(theme_colors.bar_empty),
        ));
    }

    Line::from(spans)
}

/// Render the snooze duration input popup
fn render_snooze_popup(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect_fixed(40, 7, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border with accent color
    let block = Block::bordered()
        .title("Snooze Duration")
        .border_style(Style::default().fg(app.theme_colors.popup_border))
        .title_style(app.theme_colors.popup_title)
        .style(Style::default().bg(app.theme_colors.popup_bg));
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Split inner area for input, duration preview, end time, and help text
    let chunks = Layout::vertical([
        Constraint::Length(1), // Input line
        Constraint::Length(1), // Duration preview
        Constraint::Length(1), // End time preview
        Constraint::Length(1), // Help text (Enter/Esc)
    ])
    .split(inner);

    // Render input with cursor (cursor in cyan for visibility)
    let input_line = Line::from(vec![
        Span::raw(&app.snooze_input),
        Span::styled("|", Style::default().fg(Color::Cyan)),
    ]);
    let input = Paragraph::new(input_line);
    frame.render_widget(input, chunks[0]);

    // Render live duration preview
    let parse_result = if app.snooze_input.trim().is_empty() {
        None // Not an error, just empty = indefinite
    } else {
        Some(humantime::parse_duration(app.snooze_input.trim()))
    };

    let preview_text = match &parse_result {
        None => "indefinite".to_string(),
        Some(Ok(d)) => humantime::format_duration(*d).to_string(),
        Some(Err(_)) => "invalid duration".to_string(),
    };

    let preview_color = match &parse_result {
        None => app.theme_colors.muted,
        Some(Ok(_)) => Color::Green,
        Some(Err(_)) => Color::Red,
    };

    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Duration: ", Style::default().fg(app.theme_colors.muted)),
        Span::styled(preview_text, Style::default().fg(preview_color)),
    ]));
    frame.render_widget(preview, chunks[1]);

    // Render end time preview
    let end_time_text = match &parse_result {
        None => "Ends: never".to_string(),
        Some(Ok(d)) => {
            let now = Local::now();
            let end = now + chrono::Duration::from_std(*d).unwrap_or_default();
            let days_away = (end.date_naive() - now.date_naive()).num_days();
            let time = end.format("%H:%M");
            // Days until end of current ISO week (Mon=1..Sun=7)
            // e.g. Tuesday(2): days_to_week_end = 7 - 2 = 5 (Wed,Thu,Fri,Sat,Sun)
            let days_to_week_end = 7 - now.weekday().number_from_monday() as i64;
            let date_part = if days_away == 0 {
                "today".to_string()
            } else if days_away == 1 {
                "tomorrow".to_string()
            } else if days_away <= days_to_week_end {
                format!("this {}", end.format("%A"))
            } else if days_away <= days_to_week_end + 7 {
                format!("next {}", end.format("%A"))
            } else if end.year() == now.year() {
                end.format("%b %-d").to_string()
            } else {
                end.format("%b %-d, %Y").to_string()
            };
            format!("Ends: {} {}", date_part, time)
        }
        Some(Err(_)) => String::new(),
    };
    let end_time = Paragraph::new(end_time_text).style(Style::default().fg(app.theme_colors.muted));
    frame.render_widget(end_time, chunks[2]);

    // Render help text
    let help = Paragraph::new("Enter: confirm | Esc: cancel")
        .style(Style::default().fg(app.theme_colors.muted));
    frame.render_widget(help, chunks[3]);
}

/// Create a centered rectangle with fixed width and height
fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    // Clamp dimensions to area bounds
    let width = width.min(area.width);
    let height = height.min(area.height);

    // Calculate centered position
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Render the help overlay popup
fn render_help_popup(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect_fixed(50, 17, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border with accent color
    let block = Block::bordered()
        .title(" Keyboard Shortcuts ")
        .border_style(Style::default().fg(app.theme_colors.popup_border))
        .title_style(app.theme_colors.popup_title)
        .style(Style::default().bg(app.theme_colors.popup_bg));
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Build help text with two-column layout
    let help_lines = vec![
        Line::from(vec![
            Span::styled(
                "j / Down      ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled(
                "k / Up        ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled(
                "Enter / o     ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Open PR in browser"),
        ]),
        Line::from(vec![
            Span::styled(
                "b             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Score breakdown"),
        ]),
        Line::from(vec![
            Span::styled(
                "s             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Snooze / re-snooze PR"),
        ]),
        Line::from(vec![
            Span::styled(
                "u             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Unsnooze PR"),
        ]),
        Line::from(vec![
            Span::styled(
                "z             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Undo last action"),
        ]),
        Line::from(vec![
            Span::styled(
                "Tab           ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Toggle Active/Snoozed"),
        ]),
        Line::from(vec![
            Span::styled(
                "r             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Refresh PRs (bypasses cache)"),
        ]),
        Line::from(vec![
            Span::styled(
                "?             ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Show/hide this help"),
        ]),
        Line::from(vec![
            Span::styled(
                "q / Ctrl-c    ",
                Style::default()
                    .fg(app.theme_colors.status_key_color)
                    .bold(),
            ),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(app.theme_colors.muted),
        )),
    ];

    let help_text = Paragraph::new(help_lines);
    frame.render_widget(help_text, inner);
}

/// Render the loading spinner overlay
fn render_loading_overlay(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect_fixed(30, 3, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border with accent color
    let block = Block::bordered()
        .border_style(Style::default().fg(app.theme_colors.popup_border))
        .style(Style::default().bg(app.theme_colors.popup_bg));
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Braille spinner animation
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[app.spinner_frame % 10];

    // Display different text based on whether this is initial load or refresh
    let text = if app.active_prs.is_empty() && app.snoozed_prs.is_empty() {
        format!("{} Loading PRs...", spinner)
    } else {
        format!("{} Refreshing...", spinner)
    };

    let loading_text = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(app.theme_colors.title_color));

    frame.render_widget(loading_text, inner);
}

/// Render the score breakdown detail popup
fn render_score_breakdown_popup(frame: &mut Frame, app: &App) {
    // Get selected PR and score result
    let (pr, score_result) = match app.selected_pr().zip(app.selected_score_result()) {
        Some(pair) => pair,
        None => return, // Defensive: shouldn't happen but exit gracefully
    };

    let breakdown = &score_result.breakdown;

    // Calculate dynamic height: 4 header lines + 2 lines per factor + 3 footer lines
    // Each factor uses 2 lines: values + indented description
    let num_factors = breakdown.factors.len();
    let factor_lines = if num_factors == 0 { 2 } else { num_factors * 2 };
    let content_height = 4 + factor_lines + 3;
    let popup_height = (content_height as u16).min(frame.area().height.saturating_sub(2));

    let popup_area = centered_rect_fixed(57, popup_height + 2, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border with accent color
    let block = Block::bordered()
        .title(" Score Breakdown ")
        .border_style(Style::default().fg(app.theme_colors.popup_border))
        .title_style(app.theme_colors.popup_title)
        .style(Style::default().bg(app.theme_colors.popup_bg));
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);
    let inner = inner.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Build content lines
    let mut lines = Vec::new();

    // Line 1: PR reference in muted text
    lines.push(Line::from(Span::styled(
        pr.short_ref(),
        Style::default().fg(app.theme_colors.muted),
    )));

    // Line 2: PR title (truncate if needed)
    let max_title_width = (inner.width as usize).saturating_sub(2);
    let title = if pr.title.len() > max_title_width {
        format!("{}...", &pr.title[..max_title_width.saturating_sub(3)])
    } else {
        pr.title.clone()
    };
    lines.push(Line::from(title));

    // Line 3: Empty separator
    lines.push(Line::from(""));

    // Line 4: Base score
    lines.push(Line::from(vec![
        Span::raw("Base score:  "),
        Span::styled(
            format!("{:.1}", breakdown.base_score),
            Style::default().bold(),
        ),
    ]));

    // Lines 5+: Factor contributions
    if breakdown.factors.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No scoring factors configured",
            Style::default().fg(app.theme_colors.muted),
        )));
    } else {
        for factor in &breakdown.factors {
            // Determine color for after value based on change
            let after_color = if factor.after > factor.before {
                Color::Green
            } else if factor.after < factor.before {
                Color::Red
            } else {
                Color::Reset
            };

            // Line 1: label + before -> after
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", factor.label),
                    Style::default().fg(app.theme_colors.title_color).bold(),
                ),
                Span::styled(
                    format!("{:.1}", factor.before),
                    Style::default().fg(app.theme_colors.muted),
                ),
                Span::raw(" -> "),
                Span::styled(
                    format!("{:.1}", factor.after),
                    Style::default().fg(after_color).bold(),
                ),
            ]));

            // Line 2: indented description
            lines.push(Line::from(Span::styled(
                format!("  {}", factor.description),
                Style::default().fg(app.theme_colors.muted),
            )));
        }
    }

    // Line N-2: Empty separator
    lines.push(Line::from(""));

    // Line N-1: Final score with color
    let max_score = app
        .current_prs()
        .iter()
        .map(|(_, sr)| sr.score)
        .fold(0.0_f64, f64::max);
    let score_color = app.theme_colors.score_color(score_result.score, max_score);

    lines.push(Line::from(vec![
        Span::raw("Final score: "),
        Span::styled(
            format!("{:.1}", score_result.score),
            Style::default().fg(score_color).bold(),
        ),
    ]));

    // Line N: Help text
    lines.push(Line::from(Span::styled(
        "Esc or d to close",
        Style::default().fg(app.theme_colors.muted),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
