use std::collections::VecDeque;

use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, GraphType, LineGauge, Paragraph},
    Frame,
};

use super::common::*;
use super::theme;

pub fn cpu_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let info_height: usize = 3;
    let bar_area_height = (area.height as usize)
        .saturating_sub(info_height + 8)
        .clamp(3, 8);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(info_height as u16),
            Constraint::Length(bar_area_height as u16),
            Constraint::Min(8),
        ])
        .split(area);

    let info_line = Line::from(vec![
        Span::styled(
            format!("\u{2699} {}  ", truncate(&app.system.cpu_model, 32)),
            Style::default().fg(t.text),
        ),
        Span::styled(
            format!("Cores: {}  ", app.system.cpu_count),
            Style::default().fg(t.overlay0),
        ),
        Span::styled(
            format!(
                "\u{25a0} Load: {} {} {}",
                app.load_avg[0], app.load_avg[1], app.load_avg[2]
            ),
            Style::default().fg(t.overlay0),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(info_line).block(panel_block("\u{2699} CPU Info")),
        chunks[0],
    );

    let bar_area = chunks[1].inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });
    if bar_area.height == 0 || bar_area.width < 12 {
        // Too small for gauges: the trend chart below still carries the signal.
    } else {
        let total_cores = app.core_usages.len();
        let cores_to_show = total_cores.min((bar_area.height as usize) / 2 * 2).max(1);
        let hidden = total_cores.saturating_sub(cores_to_show);
        let bar_height = (bar_area.height / cores_to_show.max(1) as u16).max(1);

        let bars = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                (0..cores_to_show)
                    .map(|_| Constraint::Length(bar_height))
                    .collect::<Vec<_>>(),
            )
            .split(bar_area);

        for (i, &usage) in app.core_usages.iter().enumerate().take(bars.len()) {
            let sev = Severity::from_usage(usage);
            let color = severity_color(sev);
            let more = if hidden > 0 && i + 1 == bars.len() {
                format!(" (+{hidden} more)")
            } else {
                String::new()
            };
            let gauge = LineGauge::default()
                .block(ratatui::widgets::Block::default().title(format!(
                    "{} Core {i} {:>5.1}%{more}",
                    sev.symbol(),
                    usage
                )))
                .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
                .line_set(ratatui::symbols::line::THICK)
                .ratio((usage / 100.0).clamp(0.0, 1.0));
            frame.render_widget(gauge, bars[i]);
        }
    }

    render_chart(
        frame,
        chunks[2],
        "\u{2699} CPU Trend (120s)",
        &app.cpu_history,
        t.accent_teal,
    );
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
        .name("line")
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
