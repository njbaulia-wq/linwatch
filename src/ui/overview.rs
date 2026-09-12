use std::collections::VecDeque;

use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Chart, GraphType, LineGauge, Paragraph, Sparkline, Wrap},
    Frame,
};

use super::common::*;
use super::theme;

pub fn overview(frame: &mut Frame, area: Rect, app: &AppState) {
    match breakpoint_for_area(area) {
        Breakpoint::Tiny => render_tiny_layout(frame, area, app),
        Breakpoint::Compact => render_small_layout(frame, area, app),
        // Wide reuses the full layout with rebalanced columns (see render_body).
        Breakpoint::Full | Breakpoint::Wide => render_full_layout(frame, area, app),
    }
}

// ─── TINY layout (width < 64 or height < 16) ─────────────────────────────────
fn render_tiny_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(4),
        ])
        .split(area);

    render_health_bar_inline(frame, chunks[0], app);
    render_kpi_compact_row(frame, chunks[1], app);
    render_alerts_inline(frame, chunks[2], app);
}

// ─── SMALL layout (width < 100 or height < 25) ───────────────────────────────
fn render_small_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Min(4),
        ])
        .split(area);

    render_health_bar_inline(frame, chunks[0], app);
    render_kpi_compact_row(frame, chunks[1], app);
    render_pressure_row(frame, chunks[2], app);
    render_alerts_inline(frame, chunks[3], app);
}

// ─── FULL layout (large terminal) ────────────────────────────────────────────
fn render_full_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let focus_height = if area.height >= 36 { 12 } else { 9 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(focus_height),
            Constraint::Min(8),
        ])
        .split(area);

    render_health_bar_inline(frame, chunks[0], app);
    render_kpi_row(frame, chunks[1], app);
    render_body(frame, chunks[2], app);
}

// ═══════════════════════════════════════════════════════════════════════════════
// VERDICT BAR — one glanceable answer: health + primary cause.
// Temp/battery/net/uptime live in header/footer; repeating them here was
// redundant ink.
// ═══════════════════════════════════════════════════════════════════════════════
fn render_health_bar_inline(frame: &mut Frame, area: Rect, app: &AppState) {
    let sev = Severity::from_health(app.health_score as f64);

    if area.width < 50 {
        let block = panel_block_severity("", sev);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(visual_ratio(app.health_score as f64, inner.width))
            .label(Line::from(vec![
                severity_chip(sev),
                Span::styled(
                    format!(" {}", app.health_score),
                    Style::default()
                        .fg(severity_color(sev))
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        frame.render_widget(gauge, inner);
        return;
    }

    let left_width = 24u16.min(area.width / 3);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_width), Constraint::Min(20)])
        .split(area);

    let health_word = match sev {
        Severity::Ok => "Healthy",
        Severity::Warn => "Attention",
        Severity::Critical => "Critical",
        Severity::Neutral => "Monitoring",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            severity_chip(sev),
            Span::styled(
                format!(" {} {health_word}", app.health_score),
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        chunks[0],
    );

    let cause_line = match app.root_causes.first() {
        Some(cause) if cause.severity != Severity::Ok => Line::from(vec![
            severity_chip(cause.severity),
            Span::styled(
                format!(" {} ", truncate(&cause.title, 16)),
                Style::default()
                    .fg(severity_color(cause.severity))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(&cause.detail, chunks[1].width as usize),
                Style::default().fg(theme::get().subtext1),
            ),
        ]),
        _ => Line::from(vec![
            severity_chip(Severity::Ok),
            Span::styled(
                " All thresholds nominal",
                Style::default().fg(theme::get().subtext1),
            ),
        ]),
    };
    frame.render_widget(Paragraph::new(cause_line), chunks[1]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// KPI ROW — 4 borderless stat blocks: label, honest value, sparkline.
// Severity rides on the value color (+chip), never on decoration.
// ═══════════════════════════════════════════════════════════════════════════════
fn render_kpi_row(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32),
            Constraint::Percentage(32),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
        ])
        .split(area);

    let gpu = app.primary_gpu();
    let gpu_history = if gpu.is_some() && app.gpu_usage_history.len() >= 2 {
        Some(&app.gpu_usage_history)
    } else {
        None
    };
    render_kpi_stat(
        frame,
        chunks[0],
        "CPU",
        &format!(
            "{} ×{}",
            truncate(&app.system.cpu_model, 20),
            app.system.cpu_count
        ),
        Some(app.cpu_usage),
        Some(&app.cpu_history),
        t.accent_teal,
    );
    render_kpi_stat(
        frame,
        chunks[1],
        "GPU",
        &gpu.map(|g| truncate(&g.model, 20))
            .unwrap_or_else(|| String::from("None")),
        gpu.and_then(|g| g.usage_pct),
        gpu_history,
        t.accent_purple,
    );
    render_kpi_stat(
        frame,
        chunks[2],
        "MEM",
        &format_memory_detail(app.mem_used, app.mem_total),
        Some(app.mem_pct()),
        Some(&app.mem_history),
        t.accent_yellow,
    );
    render_kpi_stat(
        frame,
        chunks[3],
        "DSK",
        &format!("{:.0}/{:.0}GB", app.disk.used_gb, app.disk.total_gb),
        Some(app.disk.pct as f64),
        None,
        t.accent_green,
    );
}

fn render_kpi_stat(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    subtitle: &str,
    value: Option<f64>,
    history: Option<&VecDeque<(f64, f64)>>,
    color: ratatui::style::Color,
) {
    let block = flat_panel("");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 10 {
        // Ultra-compact: single honest line.
        let line = match value {
            Some(v) => format!("{label} {}", fmt_pct0(v)),
            None => format!("{label} N/A"),
        };
        frame.render_widget(Paragraph::new(dim(line)), inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(dim(format!(
            "{label} · {}",
            truncate(
                subtitle,
                (inner.width as usize).saturating_sub(label.len() + 3)
            )
        ))),
        chunks[0],
    );

    match value {
        Some(v) => {
            let sev = Severity::from_usage(v);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    severity_chip(sev),
                    Span::styled(
                        format!(" {}", fmt_pct1(v)),
                        Style::default()
                            .fg(severity_color(sev))
                            .add_modifier(Modifier::BOLD),
                    ),
                ])),
                chunks[1],
            );
        }
        None => {
            frame.render_widget(Paragraph::new(dim("○ N/A")), chunks[1]);
        }
    }

    let points: Vec<u64> = history
        .map(|h| {
            h.iter()
                .rev()
                .take(inner.width as usize)
                .rev()
                .map(|(_, v)| v.max(0.0).round() as u64)
                .collect()
        })
        .unwrap_or_default();
    if points.len() >= 2 {
        frame.render_widget(
            Sparkline::default()
                .data(&points)
                .max(100)
                .style(Style::default().fg(color)),
            chunks[2],
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// KPI COMPACT ROW — for tiny/small layouts
// ═══════════════════════════════════════════════════════════════════════════════
fn render_kpi_compact_row(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let gpu = app.primary_gpu();
    let items = [
        (
            "CPU",
            truncate(&app.system.cpu_model, 14),
            Some(app.cpu_usage),
            Some(&app.cpu_history),
            t.accent_teal,
        ),
        (
            "GPU",
            gpu.map(|g| truncate(&g.model, 12))
                .unwrap_or_else(|| String::from("None")),
            gpu.and_then(|g| g.usage_pct),
            None,
            t.accent_purple,
        ),
        (
            "MEM",
            format!(
                "{:.1}/{:.1}GiB",
                app.mem_used / 1024.0,
                app.mem_total / 1024.0
            ),
            Some(app.mem_pct()),
            Some(&app.mem_history),
            t.accent_yellow,
        ),
        (
            "DSK",
            format!("{:.0}/{:.0}GB", app.disk.used_gb, app.disk.total_gb),
            Some(app.disk.pct as f64),
            None,
            t.accent_green,
        ),
    ];

    // Below ~48 cols four stats side-by-side are unreadable: use a 2x2 grid.
    if area.width < 48 && area.height >= 7 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        for (row, pair) in [(rows[0], [0, 1]), (rows[1], [2, 3])].iter() {
            let pair_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*row);
            for (slot, item_idx) in pair.iter().enumerate() {
                let (label, subtitle, value, history, color) = &items[*item_idx];
                render_kpi_stat(
                    frame,
                    pair_cols[slot],
                    label,
                    subtitle,
                    *value,
                    *history,
                    *color,
                );
            }
        }
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(34),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
        ])
        .split(area);
    for (i, (label, subtitle, value, history, color)) in items.iter().enumerate() {
        render_kpi_stat(frame, cols[i], label, subtitle, *value, *history, *color);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PRESSURE ROW — horizontal gauges for small layout
// ═══════════════════════════════════════════════════════════════════════════════
fn render_pressure_row(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" Pressure ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    render_mini_gauge(frame, rows[0], "CPU ", app.cpu_usage, t.accent_teal);
    render_detail_gauge(
        frame,
        rows[1],
        "MEM ",
        app.mem_pct(),
        &format_memory_detail(app.mem_used, app.mem_total),
        t.accent_yellow,
    );
    render_mini_gauge(frame, rows[2], "SWAP", app.swap_pct(), t.accent_purple);
}

fn render_mini_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    let sev = Severity::from_usage(value);

    // Narrow columns drop the trailing state word; value + bar suffice.
    let wide = area.width >= 30;
    let mut constraints = vec![
        Constraint::Length(5),
        Constraint::Length(6),
        Constraint::Min(6),
    ];
    if wide {
        constraints.push(Constraint::Length(5));
    }
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            label,
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{value:>5.1}%"),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        )),
        cols[1],
    );

    let gauge = LineGauge::default()
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(visual_ratio(value, cols[2].width))
        .label(Span::raw(""));
    frame.render_widget(gauge, cols[2]);

    if wide {
        frame.render_widget(
            Paragraph::new(Span::styled(
                severity_word(sev),
                Style::default().fg(severity_color(sev)),
            ))
            .alignment(Alignment::Right),
            cols[3],
        );
    }
}

fn render_detail_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    detail: &str,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    let sev = Severity::from_usage(value);
    let detail_width = (area.width / 2).clamp(10, 22);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(detail_width),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            label,
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )),
        cols[0],
    );

    let gauge = LineGauge::default()
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(visual_ratio(value, cols[1].width))
        .label(Span::styled(
            format!("{value:.1}%"),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, cols[1]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            truncate(detail, detail_width as usize),
            Style::default().fg(t.overlay1),
        ))
        .alignment(Alignment::Right),
        cols[2],
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// BODY — charts + resource bars + alerts (full layout)
// ═══════════════════════════════════════════════════════════════════════════════
fn render_body(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.height < 10 {
        render_alerts_visual(frame, area, app);
        return;
    }

    // Wide (≥160): four balanced columns — charts get room plus a second
    // chart column instead of squeezing into 56/22/22.
    if area.width >= 160 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(46),
                Constraint::Percentage(20),
                Constraint::Percentage(17),
                Constraint::Percentage(17),
            ])
            .split(area);

        render_charts(frame, chunks[0], app);
        render_resource_bars(frame, chunks[1], app);
        render_alerts_visual(frame, chunks[2], app);
        render_process_bars(frame, chunks[3], app);
        return;
    }

    if area.width < 120 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        render_charts(frame, rows[0], app);
        render_resource_bars(frame, bottom[0], app);
        render_alerts_visual(frame, bottom[1], app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(56),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
        ])
        .split(area);

    render_charts(frame, chunks[0], app);
    render_resource_bars(frame, chunks[1], app);
    render_alerts_visual(frame, chunks[2], app);
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHARTS — CPU + GPU are the primary glance targets.
// ═══════════════════════════════════════════════════════════════════════════════
fn render_charts(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let horizontal = area.width >= 96 && area.height < 16;
    let chunks = Layout::default()
        .direction(if horizontal {
            Direction::Horizontal
        } else {
            Direction::Vertical
        })
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let cpu_avg = moving_average(&app.cpu_history, 5);
    let cpu_peak = app.cpu_history.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let cpu_title = format!(
        "CPU {} · AVG {:.0} PEAK {:.0}  {}",
        fmt_pct1(app.cpu_usage),
        cpu_avg,
        cpu_peak,
        truncate(&app.system.cpu_model, 20)
    );
    render_chart(
        frame,
        chunks[0],
        &cpu_title,
        &app.cpu_history,
        t.accent_teal,
    );

    let gpu = app.primary_gpu();
    let gpu_usage = gpu.and_then(|g| g.usage_pct).unwrap_or(0.0);
    let gpu_avg = moving_average(&app.gpu_usage_history, 5);
    let gpu_peak = app
        .gpu_usage_history
        .iter()
        .map(|(_, v)| *v)
        .fold(gpu_usage, f64::max);
    let gpu_name = gpu
        .map(|g| truncate(&g.model, 22))
        .unwrap_or_else(|| String::from("No GPU sensor"));
    let gpu_title = format!(
        "GPU {} · AVG {:.0} PEAK {:.0}  {gpu_name}",
        fmt_pct1(gpu_usage),
        gpu_avg,
        gpu_peak
    );
    render_chart(
        frame,
        chunks[1],
        &gpu_title,
        &app.gpu_usage_history,
        t.accent_purple,
    );
}

fn moving_average(data: &VecDeque<(f64, f64)>, window: usize) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let vals: Vec<f64> = data.iter().rev().take(window).map(|(_, v)| *v).collect();
    vals.iter().sum::<f64>() / vals.len() as f64
}

fn render_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    if area.height < 8 || area.width < 52 {
        render_compact_wave_chart(frame, area, title, data, color);
        return;
    }

    if data.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting...")
                .block(chart_block(title, color))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

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
        .block(chart_block(title, color))
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

fn render_compact_wave_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
) {
    let block = chart_block(title, color);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let latest = data.back().map(|(_, v)| *v).unwrap_or(0.0);
    if inner.height == 1 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:>3.0}%", latest),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            inner,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:>3.0}% ", latest),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                severity_word(Severity::from_usage(latest)),
                Style::default().fg(severity_color(Severity::from_usage(latest))),
            ),
        ])),
        chunks[0],
    );

    if data.len() >= 2 {
        let values: Vec<u64> = data
            .iter()
            .rev()
            .take(inner.width as usize)
            .rev()
            .map(|(_, v)| v.max(0.0).round() as u64)
            .collect();
        frame.render_widget(
            Sparkline::default()
                .data(&values)
                .max(100)
                .style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
            chunks[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new("waiting").alignment(Alignment::Center),
            chunks[1],
        );
    }
}

fn chart_block(title: &str, color: ratatui::style::Color) -> Block<'static> {
    panel_block("")
        .title(Line::from(vec![Span::styled(
            format!(" {title} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]))
        .border_style(Style::default().fg(color))
}

// ═══════════════════════════════════════════════════════════════════════════════
// RESOURCE BARS — true-percent gauges only. NET (bps) and TEMP (°C) are not
// percents, so they render as honest text lines, never fake gauges.
// ═══════════════════════════════════════════════════════════════════════════════
fn render_resource_bars(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" System ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 {
        return;
    }

    // Tall panels get separate NET + TEMP lines; short ones share a line.
    let tall = inner.height >= 10;
    let rows_len = if tall { 5 } else { 4 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(rows_len), Constraint::Min(3)])
        .split(inner);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            (0..rows_len)
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(chunks[0]);

    render_detail_gauge(
        frame,
        rows[0],
        "MEM ",
        app.mem_pct(),
        &format_memory_detail(app.mem_used, app.mem_total),
        t.accent_yellow,
    );
    render_mini_gauge(frame, rows[1], "DSK ", app.disk.pct as f64, t.accent_green);
    render_mini_gauge(frame, rows[2], "SWAP", app.swap_pct(), t.accent_purple);

    let net_text = format!(
        "↓{}/s ↑{}/s",
        format_bytes(app.net_down_bps),
        format_bytes(app.net_up_bps)
    );
    let temp_state = match app.temp_c {
        Some(temp) if temp >= 80.0 => "Hot",
        Some(temp) if temp >= 70.0 => "Warm",
        Some(_) => "Normal",
        None => "No sensor",
    };
    let temp_text = format!("{} {}", fmt_temp(app.temp_c), temp_state);
    if tall {
        render_stat_line(frame, rows[3], "NET ", &net_text, t.accent_teal);
        render_stat_line(
            frame,
            rows[4],
            "TEMP ",
            &temp_text,
            severity_color(Severity::from_usage(app.temp_c.unwrap_or(0.0))),
        );
    } else {
        let combined = format!("{net_text} · {temp_text}");
        render_stat_line(frame, rows[3], "", &combined, t.overlay1);
    }
    render_process_bars(frame, chunks[1], app);
}

fn render_stat_line(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    text: &str,
    color: ratatui::style::Color,
) {
    let mut line = String::from(label);
    line.push_str(&truncate(
        text,
        (area.width as usize).saturating_sub(label.len()).max(4),
    ));
    frame.render_widget(
        Paragraph::new(Span::styled(line, Style::default().fg(color))),
        area,
    );
}

fn render_process_bars(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    if area.height == 0 || area.width < 12 {
        return;
    }

    let max_rows = area.height as usize;
    let max_cpu = app
        .top_cpu_processes
        .iter()
        .take(max_rows)
        .map(|p| p.cpu_pct)
        .fold(1.0, f64::max);

    for (idx, process) in app.top_cpu_processes.iter().take(max_rows).enumerate() {
        let row = Rect {
            x: area.x,
            y: area.y + idx as u16,
            width: area.width,
            height: 1,
        };
        let name_width = (area.width / 3).clamp(6, 14);
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(name_width),
                Constraint::Min(5),
                Constraint::Length(5),
            ])
            .split(row);
        let sev = Severity::from_usage(process.cpu_pct);
        frame.render_widget(
            Paragraph::new(Span::styled(
                truncate(&process.name, name_width as usize),
                Style::default().fg(t.overlay1),
            )),
            cols[0],
        );

        let ratio = visual_ratio(process.cpu_pct / max_cpu * 100.0, cols[1].width);
        let gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(ratio)
            .label(Span::raw(""));
        frame.render_widget(gauge, cols[1]);
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:>4.0}%", process.cpu_pct),
                Style::default().fg(severity_color(sev)),
            ))
            .alignment(Alignment::Right),
            cols[2],
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ALERTS — visual severity indicators, compact
// ═══════════════════════════════════════════════════════════════════════════════
fn render_alerts_visual(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let health_sev = Severity::from_health(app.health_score as f64);
    let mut lines: Vec<Line> = Vec::new();
    // Narrow panels show titles only; details would wrap mid-word.
    let show_detail = area.width >= 32;

    if let Some(cause) = app.root_causes.first() {
        let cause_sev = cause.severity;
        let mut spans = vec![
            severity_chip(cause_sev),
            Span::styled(
                format!(" {}", truncate(&cause.title, 14)),
                Style::default()
                    .fg(severity_color(cause_sev))
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if show_detail {
            spans.push(Span::styled(
                format!(" — {}", truncate(&cause.detail, 30)),
                Style::default().fg(t.text),
            ));
        }
        lines.push(Line::from(spans));
    }

    if let Some(unit) = app.failed_units.first() {
        lines.push(Line::from(vec![
            severity_chip(Severity::Critical),
            Span::styled(
                format!(" {}", truncate(&unit.unit, 14)),
                Style::default()
                    .fg(severity_color(Severity::Critical))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}/{}", unit.active, unit.sub),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    if let Some(drive) = app.storage_health.iter().find(|d| d.risk != Severity::Ok) {
        let temp = fmt_temp(drive.temp_c);
        lines.push(Line::from(vec![
            severity_chip(drive.risk),
            Span::styled(
                format!(" {}", truncate(&drive.device, 10)),
                Style::default().fg(t.text),
            ),
            Span::styled(
                format!(" {temp} {}", truncate(&drive.model, 12)),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    // GPU context lives on the GPU tab + KPI: only surface it here when hot.
    if let Some(gpu) = app.primary_gpu() {
        let hot = gpu.temp_c.is_some_and(|temp| temp >= 85.0)
            || gpu.usage_pct.is_some_and(|usage| usage >= 90.0);
        if hot {
            lines.push(Line::from(vec![
                severity_chip(Severity::Warn),
                Span::styled(
                    format!(" {} {}", gpu.kind, truncate(&gpu.model, 16)),
                    Style::default()
                        .fg(severity_color(Severity::Warn))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        " {} {}",
                        fmt_pct0(gpu.usage_pct.unwrap_or(0.0)),
                        fmt_temp(gpu.temp_c)
                    ),
                    Style::default().fg(t.overlay1),
                ),
            ]));
        }
    }

    for event in app.events.iter().rev().take(2) {
        let mut spans = vec![
            severity_chip(event.severity),
            Span::styled(
                format!(" {}", truncate(&event.title, 14)),
                Style::default().fg(severity_color(event.severity)),
            ),
        ];
        if show_detail {
            spans.push(Span::styled(
                format!(" — {}", truncate(&event.detail, 28)),
                Style::default().fg(t.overlay1),
            ));
        }
        lines.push(Line::from(spans));
    }

    let alert_budget = (area.height as usize)
        .saturating_sub(lines.len() + 2)
        .min(3);
    for alert in app.alerts.iter().take(alert_budget) {
        let sev = if alert.starts_with('\u{26a1}') {
            Severity::Critical
        } else if alert.starts_with('\u{26a0}') {
            Severity::Warn
        } else {
            Severity::Ok
        };
        lines.push(Line::from(vec![
            severity_chip(sev),
            Span::styled(
                format!(" {}", truncate(alert, 38)),
                Style::default().fg(t.text),
            ),
        ]));
    }

    let border_sev = if app.alerts.iter().any(|a| a.starts_with('\u{26a1}')) {
        Severity::Critical
    } else if app.alerts.iter().any(|a| a.starts_with('\u{26a0}')) {
        Severity::Warn
    } else {
        health_sev
    };

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        area.inner(&ratatui::layout::Margin {
            horizontal: 1,
            vertical: 0,
        }),
    );
    let block = panel_block_severity(" Status ", border_sev);
    frame.render_widget(block, area);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ALERTS INLINE — for tiny/small layouts
// ═══════════════════════════════════════════════════════════════════════════════
fn render_alerts_inline(frame: &mut Frame, area: Rect, app: &AppState) {
    render_alerts_visual(frame, area, app);
}

fn visual_ratio(value: f64, width: u16) -> f64 {
    let ratio = (value / 100.0).clamp(0.0, 1.0);
    if value > 0.0 && width > 0 {
        ratio.max(1.0 / f64::from(width))
    } else {
        ratio
    }
}

fn format_memory_detail(used_mb: f64, total_mb: f64) -> String {
    format!(
        "{}/{}GiB",
        format_truncated_gib(used_mb),
        format_truncated_gib(total_mb)
    )
}

fn format_truncated_gib(value_mb: f64) -> String {
    let gib = (value_mb.max(0.0) / 1024.0 * 100.0).floor() / 100.0;
    format!("{gib:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn make_app() -> AppState {
        use crate::types::*;
        AppState {
            system: SystemInfo {
                os_name: String::new(),
                os_version: String::new(),
                kernel: String::new(),
                hostname: String::new(),
                cpu_model: String::new(),
                cpu_count: 1,
                selinux_mode: String::new(),
                cpu_vulnerabilities: String::new(),
            },
            env: EnvKind::BareMetal,
            cpu_usage: 45.0,
            core_usages: vec![40.0, 50.0],
            cpu_history: VecDeque::from([(0.0, 40.0), (1.0, 45.0), (2.0, 50.0)]),
            previous_cpu: None,
            mem_total: 16000.0,
            mem_used: 8000.0,
            mem_history: VecDeque::from([(0.0, 50.0), (1.0, 51.0), (2.0, 50.5)]),
            swap_total: 4096.0,
            swap_used: 0.0,
            disk: DiskInfo {
                mount_point: "/".into(),
                used_gb: 50.0,
                total_gb: 200.0,
                pct: 25,
            },
            mounts: Vec::new(),
            uptime: String::from("1h 23m"),
            load_avg: [
                String::from("0.50"),
                String::from("0.60"),
                String::from("0.70"),
            ],
            battery_pct: Some(85),
            battery_status: String::from("Discharging"),
            net_down_bps: 1024.0,
            net_up_bps: 512.0,
            net_down_history: VecDeque::new(),
            net_up_history: VecDeque::new(),
            interfaces: Vec::new(),
            previous_net: None,
            disk_io: Vec::new(),
            previous_disk_io: None,
            gpus: Vec::new(),
            previous_gpu_rc6: None,
            gpu_usage_history: VecDeque::new(),
            gpu_temp_history: VecDeque::new(),
            temp_c: Some(55.0),
            temp_history: VecDeque::new(),
            process_count: 128,
            top_cpu_processes: Vec::new(),
            top_mem_processes: Vec::new(),
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: std::collections::HashMap::new(),
            process_sort: ProcessSort::CpuDesc,
            process_history: std::collections::HashMap::new(),
            process_selected: 0,
            health_score: 85,
            alerts: vec!["System stable \u{2713} All thresholds nominal.".into()],
            successful_reads: 100,
            failed_reads: 2,
            degraded_sources: Vec::new(),
            last_sample_at: std::time::Instant::now(),
            counter: 3,
            show_help: false,
            refresh_index: 1,
            active_tab: ViewTab::Overview,
            tick_count: 5,
            terminal_width: 120,
            cpu_alert: 85.0,
            mem_alert: 85.0,
            disk_alert: 85,
            temp_alert: 80.0,
            battery_alert: 20,
            swap_alert: 35.0,
            process_search: String::new(),
            is_search_mode: false,
            open_ports: Vec::new(),
            git_modified_files: 0,
            git_fail_streak: 0,
            zombie_count: 0,
            confirm_kill_pid: None,
            confirm_kill_name: None,
            process_action_message: None,
            events: VecDeque::new(),
            cpu_pressure_ticks: 0,
            mem_pressure_ticks: 0,
            thermal_pressure_ticks: 0,
            previous_sample_status_label: String::from("OK"),
            previous_health_band: Severity::Ok,
        }
    }

    #[test]
    fn overview_renders_at_common_sizes() {
        use ratatui::{backend::TestBackend, Terminal};
        for (w, h) in [
            (60, 15),
            (70, 18),
            (80, 24),
            (100, 30),
            (120, 40),
            (160, 48),
            (180, 50),
        ] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = make_app();
            terminal
                .draw(|frame| {
                    app.terminal_width = frame.size().width;
                    overview(frame, frame.size(), &app);
                })
                .unwrap();
            let buf = terminal.backend().buffer();
            assert!(
                buf.content.iter().any(|c| c.symbol() != " "),
                "blank render at {w}x{h}"
            );
        }
    }
}
