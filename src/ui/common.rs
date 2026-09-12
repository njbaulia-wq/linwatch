use crate::types::Severity;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders},
};

use super::theme;

pub fn panel_block(title: impl Into<String>) -> Block<'static> {
    let t = theme::get();
    Block::default()
        .title(title.into())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .style(Style::default().bg(t.bg_panel))
}

pub fn panel_block_severity(title: impl Into<String>, severity: Severity) -> Block<'static> {
    let t = theme::get();
    let border_color = severity_color(severity);
    Block::default()
        .title(title.into())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(t.bg_panel))
}

/// Borderless group container. Prefer this for metric groups so ink goes to
/// data, not chrome; reserve `panel_block` for verdict/alerts/focus panels.
pub fn flat_panel(title: impl Into<String>) -> Block<'static> {
    let t = theme::get();
    Block::default()
        .title(title.into())
        .borders(Borders::NONE)
        .style(Style::default().bg(t.bg_panel))
}

pub fn styled(text: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(color))
}

/// Dim supporting text. Uses overlay1 (AA-passing); overlay0 is
/// decorative-only and must not carry information.
pub fn dim(text: impl Into<String>) -> Span<'static> {
    styled(text, theme::get().overlay1)
}

/// One-decimal percent for body values: `45.2%`.
pub fn fmt_pct1(value: f64) -> String {
    format!("{value:.1}%")
}

/// Whole percent for tiny gauges: `45%`.
pub fn fmt_pct0(value: f64) -> String {
    format!("{value:.0}%")
}

/// Temperature or honest N/A — never a fake `0.0°`.
pub fn fmt_temp(temp: Option<f64>) -> String {
    temp.map(|v| format!("{v:.0}\u{b0}"))
        .unwrap_or_else(|| String::from("N/A"))
}

/// Single-source severity mark: symbol in severity color, bold.
/// Always pair with `severity_word`, never color alone.
pub fn severity_chip(severity: Severity) -> Span<'static> {
    Span::styled(
        severity.symbol(),
        Style::default()
            .fg(severity_color(severity))
            .add_modifier(Modifier::BOLD),
    )
}

pub fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "OK",
        Severity::Warn => "WARN",
        Severity::Critical => "CRIT",
        Severity::Neutral => "N/A",
    }
}

pub fn severity_color(severity: Severity) -> Color {
    let t = theme::get();
    match severity {
        Severity::Ok => t.accent_green,
        Severity::Warn => t.accent_orange,
        Severity::Critical => t.accent_red,
        Severity::Neutral => t.accent_teal,
    }
}

pub fn sample_status_color(status: &str) -> Color {
    let t = theme::get();
    match status {
        "OK" => t.accent_green,
        "Partial" => t.accent_orange,
        "Warming up" => t.accent_blue,
        _ => t.accent_red,
    }
}

pub fn format_bytes(bytes_per_second: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes_per_second = bytes_per_second.max(0.0);

    if bytes_per_second >= GB {
        format!("{:.1} GB", bytes_per_second / GB)
    } else if bytes_per_second >= MB {
        format!("{:.1} MB", bytes_per_second / MB)
    } else if bytes_per_second >= KB {
        format!("{:.1} KB", bytes_per_second / KB)
    } else {
        format!("{:.0} B", bytes_per_second)
    }
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let mut output: String = value.chars().take(max_chars.saturating_sub(1)).collect();
        output.push('\u{2026}');
        output
    }
}

/// Truncate at a word boundary when possible so titles never read as
/// mid-word fragments (`Sample quality…` instead of `Sample qualit…`).
/// Falls back to hard truncation for single long words.
pub fn truncate_words(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut end = 0usize;
    let mut last_space: Option<usize> = None;
    for (idx, ch) in value.char_indices() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        if ch.is_whitespace() {
            last_space = Some(idx);
        }
        end = idx + ch.len_utf8();
    }
    let cut = last_space.unwrap_or(end);
    if cut == 0 {
        return String::from("\u{2026}");
    }
    format!("{}\u{2026}", value[..cut].trim_end())
}

/// Unified responsive breakpoints (validated: ratatui `Percentage` is
/// relative to total space, so text columns must self-truncate per width).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Breakpoint {
    /// <64 cols or <16 rows: essentials only.
    Tiny,
    /// <100 cols or <25 rows: compact rows, reduced columns.
    Compact,
    /// Standard desktop terminal.
    Full,
    /// ≥160 cols: extra columns/panels allowed.
    Wide,
}

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 15;

pub fn breakpoint(width: u16, height: u16) -> Breakpoint {
    if width < 64 || height < 16 {
        Breakpoint::Tiny
    } else if width < 100 || height < 25 {
        Breakpoint::Compact
    } else if width >= 160 {
        Breakpoint::Wide
    } else {
        Breakpoint::Full
    }
}

pub fn breakpoint_for_area(area: Rect) -> Breakpoint {
    breakpoint(area.width, area.height)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

pub fn sparkline_chars(value: f64) -> &'static str {
    if value >= 90.0 {
        "\u{2588}"
    } else if value >= 75.0 {
        "\u{2593}"
    } else if value >= 50.0 {
        "\u{2592}"
    } else if value >= 25.0 {
        "\u{2591}"
    } else {
        " "
    }
}

pub fn header_col(text: &str) -> Span<'static> {
    // Bold only: underlines double the ink on every table for no new meaning.
    Span::styled(
        text.to_string(),
        Style::default()
            .fg(theme::get().overlay1)
            .add_modifier(Modifier::BOLD),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints_cover_all_sizes() {
        assert_eq!(breakpoint(50, 40), Breakpoint::Tiny);
        assert_eq!(breakpoint(120, 10), Breakpoint::Tiny);
        assert_eq!(breakpoint(80, 24), Breakpoint::Compact);
        assert_eq!(breakpoint(120, 20), Breakpoint::Compact);
        assert_eq!(breakpoint(120, 40), Breakpoint::Full);
        assert_eq!(breakpoint(159, 40), Breakpoint::Full);
        assert_eq!(breakpoint(160, 40), Breakpoint::Wide);
        assert_eq!(breakpoint(200, 60), Breakpoint::Wide);
        // Compact never outranks Tiny, Wide only by width at Full height.
        assert!(Breakpoint::Tiny < Breakpoint::Compact);
        assert!(Breakpoint::Compact < Breakpoint::Full);
        assert!(Breakpoint::Full < Breakpoint::Wide);
    }
}
