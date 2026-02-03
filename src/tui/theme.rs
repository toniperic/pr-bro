//! Centralized theme module for TUI color constants and styles

use ratatui::prelude::*;

// Score-based colors (based on relative score percentage 0-100%)
pub const SCORE_HIGH: Color = Color::Red; // >= 70% of max — most urgent
pub const SCORE_MID: Color = Color::Yellow; // >= 40% of max
pub const SCORE_LOW: Color = Color::Green; // < 40% of max — less urgent

/// Returns the appropriate color for a score based on its percentage of max score
pub fn score_color(score: f64, max_score: f64) -> Color {
    let percentage = if max_score > 0.0 {
        (score / max_score) * 100.0
    } else {
        0.0
    };

    if percentage >= 70.0 {
        SCORE_HIGH
    } else if percentage >= 40.0 {
        SCORE_MID
    } else {
        SCORE_LOW
    }
}

// Score bar colors (same thresholds as score text)
pub const BAR_FILLED_HIGH: Color = Color::Red;
pub const BAR_FILLED_MID: Color = Color::Yellow;
pub const BAR_FILLED_LOW: Color = Color::Green;
pub const BAR_EMPTY: Color = Color::DarkGray;

// Table colors
pub const ROW_ALT_BG: Color = Color::Indexed(235); // 256-color dark gray for alternating rows
pub const INDEX_COLOR: Color = Color::DarkGray; // Index column color

// Styles
pub const TITLE_STYLE: Style = Style::new().bold();
pub const HEADER_STYLE: Style = Style::new().bold();
pub const TAB_ACTIVE: Style = Style::new().reversed();
pub const ROW_SELECTED: Style = Style::new().reversed();

// General colors
pub const MUTED: Color = Color::DarkGray; // For secondary text

// Title bar colors
pub const TITLE_COLOR: Color = Color::Cyan; // Accent color for app name

// Tab colors
pub const TAB_ACTIVE_STYLE: Style = Style::new().fg(Color::Cyan).bold();
pub const TAB_INACTIVE_STYLE: Style = Style::new().fg(Color::DarkGray);

// Status bar colors
pub const STATUS_BAR_BG: Color = Color::Indexed(236); // Subtle dark background
pub const STATUS_KEY_COLOR: Color = Color::Cyan; // Keyboard shortcut hints
pub const FLASH_SUCCESS: Color = Color::Green; // Positive flash messages
pub const FLASH_ERROR: Color = Color::Red; // Error flash messages

// Divider and separator colors
pub const DIVIDER_COLOR: Color = Color::Indexed(238); // Subtle line color

// Popup overlay colors
pub const POPUP_BORDER: Color = Color::Cyan; // Accent color for popup borders
pub const POPUP_TITLE: Style = Style::new().fg(Color::Cyan).bold();
pub const POPUP_BG: Color = Color::Indexed(234); // Dark background for popup content

// Scrollbar colors
pub const SCROLLBAR_THUMB: Color = Color::Indexed(244); // Medium gray for scrollbar thumb
pub const SCROLLBAR_TRACK: Color = Color::Indexed(236); // Dark gray for scrollbar track

// Update banner colors
pub const BANNER_BG: Color = Color::Rgb(50, 50, 120); // Dark blue-purple accent
pub const BANNER_FG: Color = Color::White;
pub const BANNER_KEY: Color = Color::Yellow; // Highlight for dismiss key hint
