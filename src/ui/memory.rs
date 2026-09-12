use std::collections::VecDeque;

use crate::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, GraphType, LineGauge, Paragraph},
    Frame,
};

use super::common::*;
use super::theme;

pub fn memory_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(8)])
        .split(area);

    let bar_area = chunks[0].inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });
    let bars = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(bar_area);

    render_line_gauge(
        frame,
        bars[0],
        "\u{25a3} RAM",
        app.mem_pct(),
        t.accent_yellow,
    );
    render_line_gauge(
        frame,
        bars[1],
        "\u{21c4} SWAP",
        app.swap_pct(),
        t.accent_purple,
    );

    // Narrow: stack details above the trend chart instead of squeezing 50/50.
    let (detail_panel, chart_panel) = if area.width < 80 {
        let stacked = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Min(6)])
            .split(chunks[1]);
        (stacked[0], stacked[1])
    } else {
        let side = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        (side[0], side[1])
    };

    let details = vec![
        Line::from(vec![
            styled("\u{25a3} RAM:  ", t.accent_yellow),
            styled(
                format!(
                    "{:.0} MB / {:.0} MB ({:.1}%)",
                    app.mem_used,
                    app.mem_total,
                    app.mem_pct()
                ),
                t.text,
            ),
        ]),
        Line::from(vec![
            styled("\u{21c4} Swap: ", t.accent_purple),
            styled(
                format!(
                    "{:.0} MB / {:.0} MB ({:.1}%)",
                    app.swap_used,
                    app.swap_total,
                    app.swap_pct()
                ),
                t.overlay1,
            ),
        ]),
        Line::from(vec![
            styled("\u{2603} Temp: ", t.overlay1),
            styled(
                app.temp_c
                    .map(|temp| format!("{temp:.1}\u{b0}C"))
                    .unwrap_or_else(|| String::from("N/A")),
                t.text,
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(details).block(panel_block("\u{25a3} Memory Details")),
        detail_panel,
    );

    render_chart(
        frame,
        chart_panel,
        "\u{25a3} Memory Trend (120s)",
        &app.mem_history,
        t.accent_yellow,
    );
}

fn render_line_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    color: ratatui::style::Color,
) {
    use crate::types::Severity;
    let sev = Severity::from_usage(value);
    let gauge = LineGauge::default()
        .block(ratatui::widgets::Block::default().title(format!(
            "{} {}{:>5.1}%",
            sev.symbol(),
            label,
            value
        )))
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .line_set(ratatui::symbols::line::THICK)
        .ratio((value / 100.0).clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

fn render_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
) {
    if data.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting...").block(panel_block(title)),
            area,
        );
        return;
    }

    let t = theme::get();
    let points: Vec<(f64, f64)> = data.iter().copied().collect();
    let x_start = data.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = data.back().map(|p| p.0).unwrap_or(1.0).max(x_start + 1.0);

    let line_set = ratatui::widgets::Dataset::default()
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .data(&points);

    let chart = Chart::new(vec![line_set])
        .block(panel_block(title))
        .x_axis(
            Axis::default()
                .bounds([x_start, x_end])
                .style(Style::default().fg(t.overlay1)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(vec![
                    Span::styled("0", Style::default().fg(t.overlay1)),
                    Span::styled("50", Style::default().fg(t.overlay1)),
                    Span::styled("100", Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}
