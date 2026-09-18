use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::common::*;
use super::theme;

pub fn help(frame: &mut Frame, area: Rect) {
    let t = theme::get();

    let dim_overlay = Block::default().style(Style::default().bg(t.surface0));
    frame.render_widget(dim_overlay, area);

    let modal = centered_rect(66, 46, area);
    let lines = vec![
        Line::from(Span::styled(
            " Keyboard Controls ",
            Style::default()
                .fg(t.accent_blue)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Q / Esc       ", Style::default().fg(t.accent_teal)),
            Span::styled("Exit monitor", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" R             ", Style::default().fg(t.accent_teal)),
            Span::styled("Refresh immediately", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" H             ", Style::default().fg(t.accent_teal)),
            Span::styled("Toggle this panel", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" 1-7           ", Style::default().fg(t.accent_teal)),
            Span::styled("Switch tab", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" Tab / BackTab ", Style::default().fg(t.accent_teal)),
            Span::styled("Next/previous tab", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" S             ", Style::default().fg(t.accent_teal)),
            Span::styled("Cycle process sort", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" T             ", Style::default().fg(t.accent_teal)),
            Span::styled(
                "Toggle process tree hierarchy",
                Style::default().fg(t.subtext0),
            ),
        ]),
        Line::from(vec![
            Span::styled(" K             ", Style::default().fg(t.accent_teal)),
            Span::styled(
                "Terminate process (SIGTERM / SIGKILL)",
                Style::default().fg(t.subtext0),
            ),
        ]),
        Line::from(vec![
            Span::styled(" /             ", Style::default().fg(t.accent_teal)),
            Span::styled(
                "Search processes; Esc clears",
                Style::default().fg(t.subtext0),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " \u{2191}/\u{2193}         ",
                Style::default().fg(t.accent_teal),
            ),
            Span::styled("Select process row", Style::default().fg(t.subtext0)),
        ]),
        Line::from(vec![
            Span::styled(" + / -         ", Style::default().fg(t.accent_teal)),
            Span::styled("Faster/slower refresh", Style::default().fg(t.subtext0)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Status Indicators ",
            Style::default()
                .fg(t.accent_blue)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" \u{25cf} ", Style::default().fg(t.accent_green)),
            Span::styled("OK    ", Style::default().fg(t.subtext1)),
            Span::styled("\u{25b2} ", Style::default().fg(t.accent_orange)),
            Span::styled("Warn    ", Style::default().fg(t.subtext1)),
            Span::styled("\u{25a0} ", Style::default().fg(t.accent_red)),
            Span::styled("Critical", Style::default().fg(t.subtext1)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Direct Kernel Telemetry ",
            Style::default()
                .fg(t.accent_blue)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "CPU, memory, disk, network, processes, disk I/O, and",
            Style::default().fg(t.subtext0),
        )),
        Line::from(Span::styled(
            "platform data read from /proc, /sys, and statvfs.",
            Style::default().fg(t.subtext0),
        )),
        Line::from(Span::styled(
            "No daemon, no database, no outbound network calls.",
            Style::default().fg(t.subtext0),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" Help & Keyboard Controls ")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.accent_blue))
                .style(Style::default().bg(t.base)),
        ),
        modal,
    );
}
