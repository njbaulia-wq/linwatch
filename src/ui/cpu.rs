use std::collections::VecDeque;

use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, GraphType, Paragraph},
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
            format!("Model: {}  ", truncate(&app.system.cpu_model, 48)),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Cores: {}  ", app.system.cpu_count),
            Style::default().fg(t.accent_blue),
        ),
        Span::styled(
            format!(
                "Load: {} {} {}",
                app.load_avg[0], app.load_avg[1], app.load_avg[2]
            ),
            Style::default().fg(t.overlay1),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(info_line).block(panel_block(" CPU Specification ")),
        chunks[0],
    );

    // One bordered panel, one compact row per core — N boxed gauges was
    // mostly border ink. Bars use block characters scaled to the row width.
    let block = panel_block(" CPU Cores ");
    let bar_area = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    let total_cores = app.core_usages.len();
    if total_cores == 0 {
        frame.render_widget(
            Paragraph::new(dim("No per-core data yet")),
            bar_area.inner(&Margin {
                horizontal: 2,
                vertical: 1,
            }),
        );
    } else if bar_area.height == 0 || bar_area.width < 14 {
        // Too small for bars: the trend chart below still carries the signal.
    } else {
        let mut shown = total_cores.min(bar_area.height as usize).max(1);
        let hidden = total_cores.saturating_sub(shown);
        // Reserve the last row for the overflow note when clamped.
        let overflow_row = hidden > 0 && shown > 1;
        if overflow_row {
            shown -= 1;
        }
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                (0..shown + usize::from(overflow_row))
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<_>>(),
            )
            .split(bar_area);

        for (i, &usage) in app.core_usages.iter().enumerate().take(shown) {
            let sev = Severity::from_usage(usage);
            let row_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(12), Constraint::Min(6)])
                .split(rows[i]);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("C{i:<3}{usage:>5.1}%"),
                    Style::default().fg(severity_color(sev)),
                )),
                row_cols[0],
            );
            let bar_w = row_cols[1].width as usize;
            let filled = ((usage / 100.0).clamp(0.0, 1.0) * bar_w as f64).round() as usize;
            let filled = filled.min(bar_w);
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_w.saturating_sub(filled));
            frame.render_widget(
                Paragraph::new(Span::styled(bar, Style::default().fg(severity_color(sev)))),
                row_cols[1],
            );
        }
        if overflow_row {
            frame.render_widget(
                Paragraph::new(dim(format!("+{hidden} more cores"))),
                rows[shown],
            );
        }
    }

    render_chart(
        frame,
        chunks[2],
        " CPU Utilization Trend (120s) ",
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
