use crate::state::AppState;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::common::*;
use super::theme;

pub fn footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let t = theme::get();
    let help_label = if app.show_help {
        " H-hide "
    } else {
        " H-help "
    };

    // Three density levels so the bar never clips mid-word on narrow screens.
    let text = if app.terminal_width < 64 {
        Line::from(vec![
            Span::styled("Q ", Style::default().fg(t.overlay0)),
            Span::styled("| R ", Style::default().fg(t.overlay0)),
            Span::styled(format!("|{help_label}"), Style::default().fg(t.overlay0)),
            Span::styled(
                app.refresh_label(),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" |{}", app.sample_status()),
                Style::default().fg(sample_status_color(app.sample_status())),
            ),
        ])
    } else if app.terminal_width < 100 {
        Line::from(vec![
            Span::styled("Q Exit ", Style::default().fg(t.overlay0)),
            Span::styled("| R Refresh ", Style::default().fg(t.overlay0)),
            Span::styled("| Tab View ", Style::default().fg(t.overlay0)),
            Span::styled(format!("|{help_label}"), Style::default().fg(t.overlay0)),
            Span::styled("| Interval ", Style::default().fg(t.overlay0)),
            Span::styled(
                app.refresh_label(),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    " | \u{2193}{}/s \u{2191}{}/s",
                    format_bytes(app.net_down_bps),
                    format_bytes(app.net_up_bps)
                ),
                Style::default().fg(t.accent_teal),
            ),
            Span::styled(
                format!(" | Data {}", app.sample_status()),
                Style::default().fg(sample_status_color(app.sample_status())),
            ),
        ])
    } else {
        let mut spans = vec![
            Span::styled("Q Exit ", Style::default().fg(t.overlay0)),
            Span::styled("| R Refresh now ", Style::default().fg(t.overlay0)),
            Span::styled("| Tab Switch view ", Style::default().fg(t.overlay0)),
            Span::styled(format!("|{help_label}"), Style::default().fg(t.overlay0)),
            Span::styled("| S Sort process ", Style::default().fg(t.overlay0)),
            Span::styled("| Up/Down Select ", Style::default().fg(t.overlay0)),
            Span::styled("| Interval ", Style::default().fg(t.overlay1)),
            Span::styled(
                format!("{} ", app.refresh_label()),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("| Network ", Style::default().fg(t.overlay1)),
            Span::styled(
                format!("Down {}/s ", format_bytes(app.net_down_bps)),
                Style::default().fg(t.accent_teal),
            ),
            Span::styled(
                format!("Up {}/s", format_bytes(app.net_up_bps)),
                Style::default().fg(t.accent_orange),
            ),
        ];
        if app.env.is_host_scoped() {
            spans.push(Span::styled(
                format!(" | {} host-scope", app.env.label()),
                Style::default().fg(t.accent_yellow),
            ));
        }
        Line::from(spans)
    };

    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border)),
        ),
        area,
    );
}
