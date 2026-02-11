//! Centralized theme module for TUI color constants and styles

use ratatui::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    Dark,
    Light,
}

impl Default for ThemeVariant {
    fn default() -> Self {
        Self::Dark
    }
}

/// Theme holds all colors and styles for the TUI
#[derive(Debug, Clone)]
pub struct Theme {
    variant: ThemeVariant,
}

impl Theme {
    pub fn new(variant: ThemeVariant) -> Self {
        Self { variant }
    }

    pub fn variant(&self) -> ThemeVariant {
        self.variant
    }

    // Score-based colors (based on relative score percentage 0-100%)
    pub fn score_high(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Red,
            ThemeVariant::Light => Color::Red,
        }
    }

    pub fn score_mid(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Yellow,
            ThemeVariant::Light => Color::Rgb(180, 120, 0), // Darker yellow for light theme
        }
    }

    pub fn score_low(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Green,
            ThemeVariant::Light => Color::Green,
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
            self.score_high()
        } else if percentage >= 40.0 {
            self.score_mid()
        } else {
            self.score_low()
        }
    }

    // Score bar colors (same thresholds as score text)
    pub fn bar_filled_high(&self) -> Color {
        self.score_high()
    }

    pub fn bar_filled_mid(&self) -> Color {
        self.score_mid()
    }

    pub fn bar_filled_low(&self) -> Color {
        self.score_low()
    }

    pub fn bar_empty(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::DarkGray,
            ThemeVariant::Light => Color::Indexed(250), // Light gray
        }
    }

    // Table colors
    pub fn row_alt_bg(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(235), // Dark gray for alternating rows
            ThemeVariant::Light => Color::Indexed(254), // Very light gray for alternating rows
        }
    }

    pub fn index_color(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::DarkGray,
            ThemeVariant::Light => Color::Gray,
        }
    }

    // Styles
    pub fn title_style(&self) -> Style {
        Style::new().bold()
    }

    pub fn header_style(&self) -> Style {
        Style::new().bold()
    }

    pub fn tab_active(&self) -> Style {
        Style::new().reversed()
    }

    pub fn row_selected(&self) -> Style {
        Style::new().reversed()
    }

    // General colors
    pub fn muted(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Gray,
            ThemeVariant::Light => Color::DarkGray,
        }
    }

    pub fn text(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Reset, // Use terminal default
            ThemeVariant::Light => Color::Black,
        }
    }

    pub fn background(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Reset, // Use terminal default
            ThemeVariant::Light => Color::White,
        }
    }

    // Title bar colors
    pub fn title_color(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Cyan,
            ThemeVariant::Light => Color::Blue,
        }
    }

    // Tab colors
    pub fn tab_active_style(&self) -> Style {
        match self.variant {
            ThemeVariant::Dark => Style::new().fg(Color::Cyan).bold(),
            ThemeVariant::Light => Style::new().fg(Color::Blue).bold(),
        }
    }

    pub fn tab_inactive_style(&self) -> Style {
        match self.variant {
            ThemeVariant::Dark => Style::new().fg(Color::DarkGray),
            ThemeVariant::Light => Style::new().fg(Color::Gray),
        }
    }

    // Status bar colors
    pub fn status_bar_bg(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(236), // Subtle dark background
            ThemeVariant::Light => Color::Indexed(252), // Light gray background
        }
    }

    pub fn status_key_color(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Cyan,
            ThemeVariant::Light => Color::Blue,
        }
    }

    pub fn flash_success(&self) -> Color {
        Color::Green
    }

    pub fn flash_error(&self) -> Color {
        Color::Red
    }

    // Divider and separator colors
    pub fn divider_color(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(238), // Subtle line color
            ThemeVariant::Light => Color::Indexed(248), // Light gray line
        }
    }

    // Popup overlay colors
    pub fn popup_border(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Cyan,
            ThemeVariant::Light => Color::Blue,
        }
    }

    pub fn popup_title(&self) -> Style {
        match self.variant {
            ThemeVariant::Dark => Style::new().fg(Color::Cyan).bold(),
            ThemeVariant::Light => Style::new().fg(Color::Blue).bold(),
        }
    }

    pub fn popup_bg(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(234), // Dark background for popup content
            ThemeVariant::Light => Color::Indexed(255), // White background
        }
    }

    // Scrollbar colors
    pub fn scrollbar_thumb(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(244), // Medium gray
            ThemeVariant::Light => Color::Indexed(240), // Darker gray for visibility
        }
    }

    pub fn scrollbar_track(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Indexed(236), // Dark gray
            ThemeVariant::Light => Color::Indexed(252), // Light gray
        }
    }

    // Update banner colors
    pub fn banner_bg(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Rgb(50, 50, 120), // Dark blue-purple accent
            ThemeVariant::Light => Color::Rgb(200, 220, 255), // Light blue accent
        }
    }

    pub fn banner_fg(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::White,
            ThemeVariant::Light => Color::Black,
        }
    }

    pub fn banner_key(&self) -> Color {
        match self.variant {
            ThemeVariant::Dark => Color::Yellow,
            ThemeVariant::Light => Color::Rgb(180, 120, 0), // Darker yellow
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeVariant::default())
    }
}
