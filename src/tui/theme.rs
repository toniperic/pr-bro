//! Centralized theme module for TUI color constants and styles

use ratatui::prelude::*;

/// Complete color palette for the TUI
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Score-based colors (traffic light pattern)
    pub score_high: Color,
    pub score_mid: Color,
    pub score_low: Color,

    // Score bar colors
    pub bar_filled_high: Color,
    pub bar_filled_mid: Color,
    pub bar_filled_low: Color,
    pub bar_empty: Color,

    // Table colors
    pub row_alt_bg: Color,
    pub index_color: Color,

    // Styles
    pub title_style: Style,
    pub header_style: Style,
    pub tab_active: Style,
    pub row_selected: Style,

    // General colors
    pub muted: Color,
    pub title_color: Color,

    // Tab colors
    pub tab_active_style: Style,
    pub tab_inactive_style: Style,

    // Status bar colors
    pub status_bar_bg: Color,
    pub status_key_color: Color,
    pub flash_success: Color,
    pub flash_error: Color,

    // Divider and separator colors
    pub divider_color: Color,

    // Popup overlay colors
    pub popup_border: Color,
    pub popup_title: Style,
    pub popup_bg: Color,

    // Scrollbar colors
    pub scrollbar_thumb: Color,
    pub scrollbar_track: Color,

    // Update banner colors
    pub banner_bg: Color,
    pub banner_fg: Color,
    pub banner_key: Color,
}

impl ThemeColors {
    /// Dark theme palette (reproduces original constants exactly)
    pub fn dark() -> Self {
        Self {
            score_high: Color::Red,
            score_mid: Color::Yellow,
            score_low: Color::Green,
            bar_filled_high: Color::Red,
            bar_filled_mid: Color::Yellow,
            bar_filled_low: Color::Green,
            bar_empty: Color::DarkGray,
            row_alt_bg: Color::Indexed(235),
            index_color: Color::DarkGray,
            title_style: Style::new().bold(),
            header_style: Style::new().bold(),
            tab_active: Style::new().reversed(),
            row_selected: Style::new().reversed(),
            muted: Color::Gray,
            title_color: Color::Cyan,
            tab_active_style: Style::new().fg(Color::Cyan).bold(),
            tab_inactive_style: Style::new().fg(Color::DarkGray),
            status_bar_bg: Color::Indexed(236),
            status_key_color: Color::Cyan,
            flash_success: Color::Green,
            flash_error: Color::Red,
            divider_color: Color::Indexed(238),
            popup_border: Color::Cyan,
            popup_title: Style::new().fg(Color::Cyan).bold(),
            popup_bg: Color::Indexed(234),
            scrollbar_thumb: Color::Indexed(244),
            scrollbar_track: Color::Indexed(236),
            banner_bg: Color::Rgb(50, 50, 120),
            banner_fg: Color::White,
            banner_key: Color::Yellow,
        }
    }

    /// Returns the appropriate color for a score based on its percentage of max score
    pub fn score_color(&self, score: f64, max_score: f64) -> Color {
        let percentage = if max_score > 0.0 {
            (score / max_score) * 100.0
        } else {
            0.0
        };

        if percentage >= 70.0 {
            self.score_high
        } else if percentage >= 40.0 {
            self.score_mid
        } else {
            self.score_low
        }
    }
}
