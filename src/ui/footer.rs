use crate::state::AppState;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::theme;

pub fn footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let t = theme::get();

    let key = |k: &str| {
        Span::styled(
            format!("[{k}]"),
            Style::default()
                .fg(t.accent_blue)
                .add_modifier(Modifier::BOLD),
        )
    };
    let desc = |s: &str| Span::styled(format!(" {s} "), Style::default().fg(t.subtext0));
    let sep = || Span::styled(" ", Style::default().fg(t.border));

    // If search mode is active, show search input banner
    let text = if app.is_search_mode {
        Line::from(vec![
            key("/"),
            Span::styled(
                format!(" Search: {}█ ", app.process_search),
                Style::default()
                    .fg(t.accent_yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "(Enter confirms, Esc cancels)",
                Style::default().fg(t.overlay1),
            ),
        ])
    } else if let Some(msg) = &app.process_action_message {
        Line::from(vec![
            Span::styled(
                format!(" {msg} "),
                Style::default()
                    .fg(if msg.starts_with("Could not") {
                        t.accent_red
                    } else {
                        t.accent_teal
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " · Press any key to dismiss",
                Style::default().fg(t.overlay1),
            ),
        ])
    } else if app.terminal_width < 64 {
        Line::from(vec![
            key("Q"),
            sep(),
            key("Tab"),
            sep(),
            key("1-7"),
            sep(),
            key("R"),
            sep(),
            key("S"),
            sep(),
            key("K"),
            sep(),
            key("/"),
            sep(),
            key("H"),
        ])
    } else if app.terminal_width < 96 {
        Line::from(vec![
            key("Q"),
            desc("Exit"),
            key("Tab"),
            desc("View"),
            key("1-7"),
            desc("Tab"),
            key("R"),
            desc("Ref"),
            key("S"),
            desc("Sort"),
            key("K"),
            desc("Kill"),
            key("/"),
            desc("Search"),
            key("H"),
            desc(if app.show_help { "Close" } else { "Help" }),
        ])
    } else {
        Line::from(vec![
            key("Q"),
            desc("Exit"),
            key("Tab"),
            desc("Views"),
            key("1-7"),
            desc("Direct"),
            key("R"),
            desc("Refresh"),
            key("S"),
            desc("Sort"),
            key("K"),
            desc("Terminate"),
            key("/"),
            desc("Search"),
            key("+/-"),
            desc(&format!("Rate:{}", app.refresh_label())),
            key("H"),
            desc(if app.show_help { "Close Help" } else { "Help" }),
        ])
    };

    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border))
                .style(Style::default().bg(t.bg_panel)),
        ),
        area,
    );
}
