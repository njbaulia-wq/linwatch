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
// HEALTH BAR — compact inline health + temp + battery + net + uptime
// ═══════════════════════════════════════════════════════════════════════════════
fn render_health_bar_inline(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let sev = Severity::from_health(app.health_score as f64);

    let temp = app
        .temp_c
        .map(|v| format!("{v:.0}\u{b0}"))
        .unwrap_or_else(|| "\u{2014}".into());
    let bat = app
        .battery_pct
        .map(|v| format!("{v}%"))
        .unwrap_or_else(|| "\u{2014}".into());
    let net = format!(
        "\u{2193}{}/s \u{2191}{}/s",
        format_bytes(app.net_down_bps),
        format_bytes(app.net_up_bps)
    );

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
            .label(Span::styled(
                format!("{} {}%", sev.symbol(), app.health_score),
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, inner);
        return;
    }

    let left_width = 22u16.min(area.width / 3);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_width), Constraint::Min(20)])
        .split(area);

    let gauge = LineGauge::default()
        .gauge_style(
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        )
        .ratio(visual_ratio(app.health_score as f64, chunks[0].width))
        .label(Span::styled(
            format!(" {} {}%", sev.symbol(), app.health_score),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[0]);

    let data_color = sample_status_color(app.sample_status());
    let info = Line::from(vec![
        Span::styled(
            format!("{} ", app.sample_status()),
            Style::default().fg(data_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("\u{2600} {temp}  "),
            Style::default().fg(t.accent_orange),
        ),
        Span::styled(
            format!("\u{26a1} {bat}  "),
            Style::default().fg(t.accent_yellow),
        ),
        Span::styled(format!("{net}  "), Style::default().fg(t.accent_teal)),
        Span::styled(
            format!("Up {}", truncate(&app.uptime, 12)),
            Style::default().fg(t.overlay1),
        ),
    ]);
    frame.render_widget(Paragraph::new(info), chunks[1]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// KPI ROW — 4 visual gauge cards with CPU name + mini sparkline
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

    let cpu = app.cpu_usage;
    let mem = app.mem_pct();
    let dsk = app.disk.pct as f64;
    let gpu = app.primary_gpu();
    let gpu_usage = gpu.and_then(|g| g.usage_pct);

    render_kpi_gauge(
        frame,
        chunks[0],
        "CPU",
        &format!("{}  x{}", app.system.cpu_model, app.system.cpu_count),
        app.system.cpu_count,
        cpu,
        t.accent_teal,
        &app.cpu_history,
        100,
    );
    render_kpi_gauge(
        frame,
        chunks[1],
        "GPU",
        &gpu.map(|g| truncate(&g.model, 20))
            .unwrap_or_else(|| "None".into()),
        0,
        gpu_usage.unwrap_or(0.0),
        t.accent_purple,
        &app.gpu_usage_history,
        100,
    );
    render_kpi_gauge(
        frame,
        chunks[2],
        "MEM",
        &format_memory_detail(app.mem_used, app.mem_total),
        0,
        mem,
        t.accent_yellow,
        &app.mem_history,
        100,
    );
    render_kpi_gauge(
        frame,
        chunks[3],
        "DSK",
        &format!("{:.0} / {:.0} GB", app.disk.used_gb, app.disk.total_gb),
        0,
        dsk,
        t.accent_green,
        &VecDeque::new(),
        100,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_kpi_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    subtitle: &str,
    extra: usize,
    value: f64,
    color: ratatui::style::Color,
    history: &VecDeque<(f64, f64)>,
    max: u64,
) {
    let t = theme::get();
    let sev = Severity::from_usage(value);
    let block = panel_block_severity(format!(" {label} "), sev);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 6 {
        return;
    }

    let gauge_height = if inner.height >= 6 { 2 } else { 1 };
    let spark_height = inner.height.saturating_sub(gauge_height + 2) as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(gauge_height),
            Constraint::Min(spark_height as u16),
        ])
        .split(inner);

    // Subtitle (CPU name, GPU model, etc.)
    let subtitle_color = if extra > 0 { t.overlay0 } else { t.overlay1 };
    let subtitle_text = if extra > 0 {
        format!("{subtitle} \u{00d7}{extra}")
    } else {
        subtitle.to_string()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            truncate(&subtitle_text, inner.width as usize),
            Style::default().fg(subtitle_color),
        )),
        chunks[0],
    );

    // Gauge bar with value
    let gauge = LineGauge::default()
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(visual_ratio(value, chunks[1].width))
        .label(Span::styled(
            format!(" {value:.1}%"),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[1]);

    // Sparkline if enough space and data
    if spark_height >= 2 && history.len() >= 2 {
        let data: Vec<u64> = history
            .iter()
            .rev()
            .take(inner.width as usize)
            .rev()
            .map(|(_, v)| v.max(0.0).round() as u64)
            .collect();
        if data.len() >= 2 {
            frame.render_widget(
                Sparkline::default()
                    .data(&data)
                    .max(max)
                    .style(Style::default().fg(color)),
                chunks[2],
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// KPI COMPACT ROW — for tiny/small layouts
// ═══════════════════════════════════════════════════════════════════════════════
fn render_kpi_compact_row(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let cpu = app.cpu_usage;
    let mem = app.mem_pct();
    let dsk = app.disk.pct as f64;
    let gpu = app.primary_gpu().and_then(|g| g.usage_pct);

    let items = [
        ("CPU", cpu, t.accent_teal),
        ("GPU", gpu.unwrap_or(0.0), t.accent_purple),
        ("MEM", mem, t.accent_yellow),
        ("DSK", dsk, t.accent_green),
    ];

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(34),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
        ])
        .split(area);

    // Below ~48 cols four gauges side-by-side are unreadable: use a 2x2 grid.
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
                render_kpi_compact_gauge(frame, pair_cols[slot], items[*item_idx]);
            }
        }
        return;
    }

    for (i, item) in items.iter().enumerate() {
        render_kpi_compact_gauge(frame, cols[i], *item);
    }
}

fn render_kpi_compact_gauge(
    frame: &mut Frame,
    area: Rect,
    item: (&str, f64, ratatui::style::Color),
) {
    let (label, value, color) = item;
    let sev = Severity::from_usage(value);
    let block = panel_block_severity(format!(" {label} "), sev);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 1 || inner.width < 4 {
        return;
    }

    let gauge = LineGauge::default()
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(visual_ratio(value, inner.width))
        .label(Span::styled(
            format!("{value:.0}%"),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, inner);
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

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Min(6),
            Constraint::Length(5),
        ])
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

    frame.render_widget(
        Paragraph::new(Span::styled(
            severity_label(sev),
            Style::default().fg(severity_color(sev)),
        ))
        .alignment(Alignment::Right),
        cols[3],
    );
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
        "CPU {:>3.0}%  \u{2205}{cpu_avg:.0} \u{2191}{cpu_peak:.0}  {}",
        app.cpu_usage, app.system.cpu_model
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
        .unwrap_or_else(|| "No GPU sensor".into());
    let gpu_title = format!(
        "GPU {:>3.0}%  \u{2205}{gpu_avg:.0} \u{2191}{gpu_peak:.0}  {gpu_name}",
        gpu_usage
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
    let t = theme::get();
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
            Span::styled(status_dot(latest), Style::default().fg(t.overlay1)),
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
// RESOURCE BARS — pressure gauges
// ═══════════════════════════════════════════════════════════════════════════════
fn render_resource_bars(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" System ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 {
        return;
    }

    if inner.height >= 9 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(4)])
            .split(inner);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
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
        render_mini_gauge(
            frame,
            rows[2],
            "NET\u{2193}",
            app.net_down_bps.min(10_000_000.0) / 100_000.0,
            t.accent_blue,
        );
        render_mini_gauge(
            frame,
            rows[3],
            "TEMP",
            app.temp_c.unwrap_or(0.0),
            t.accent_orange,
        );
        render_process_bars(frame, chunks[1], app);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

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
    render_mini_gauge(
        frame,
        rows[3],
        "TEMP",
        app.temp_c.unwrap_or(0.0),
        t.accent_orange,
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
                Style::default().fg(t.overlay0),
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

    if let Some(cause) = app.root_causes.first() {
        let cause_sev = cause.severity;
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", cause_sev.symbol()),
                Style::default()
                    .fg(severity_color(cause_sev))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(&cause.title, 14),
                Style::default()
                    .fg(severity_color(cause_sev))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(truncate(&cause.detail, 30), Style::default().fg(t.text)),
        ]));
    }

    if let Some(unit) = app.failed_units.first() {
        lines.push(Line::from(vec![
            Span::styled(
                "\u{26a0} ",
                Style::default()
                    .fg(t.accent_orange)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(&unit.unit, 14),
                Style::default()
                    .fg(t.accent_orange)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}/{}", unit.active, unit.sub),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    if let Some(drive) = app.storage_health.iter().find(|d| d.risk != Severity::Ok) {
        let temp = drive
            .temp_c
            .map(|v| format!("{v:.0}\u{b0}"))
            .unwrap_or_else(|| "\u{2014}".into());
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", drive.risk.symbol()),
                Style::default()
                    .fg(severity_color(drive.risk))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(truncate(&drive.device, 10), Style::default().fg(t.text)),
            Span::styled(
                format!(" {temp} {}", truncate(&drive.model, 12)),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    if let Some(gpu) = app.primary_gpu() {
        let usage = gpu
            .usage_pct
            .map(|v| format!("{v:.0}%"))
            .unwrap_or_else(|| "\u{2014}".into());
        let temp = gpu
            .temp_c
            .map(|v| format!("{v:.0}\u{b0}"))
            .unwrap_or_else(|| "\u{2014}".into());
        lines.push(Line::from(vec![
            Span::styled(
                "\u{25a6} ",
                Style::default()
                    .fg(t.accent_purple)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} {}", gpu.kind, truncate(&gpu.model, 16)),
                Style::default()
                    .fg(t.accent_purple)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {usage} {temp}"), Style::default().fg(t.overlay1)),
        ]));
    }

    for event in app.events.iter().rev().take(2) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", event.severity.symbol()),
                Style::default()
                    .fg(severity_color(event.severity))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                truncate(&event.title, 14),
                Style::default().fg(severity_color(event.severity)),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(truncate(&event.detail, 28), Style::default().fg(t.overlay1)),
        ]));
    }

    let alert_budget = (area.height as usize)
        .saturating_sub(lines.len() + 2)
        .min(3);
    for alert in app.alerts.iter().take(alert_budget) {
        let (icon, color) = if alert.starts_with('\u{26a1}') {
            ("\u{26a1}", t.accent_red)
        } else if alert.starts_with('\u{26a0}') {
            ("\u{26a0}", t.accent_orange)
        } else {
            ("\u{2713}", t.accent_green)
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{icon} "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(truncate(alert, 38), Style::default().fg(color)),
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

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "OK",
        Severity::Warn => "WARN",
        Severity::Critical => "CRIT",
        Severity::Neutral => "INFO",
    }
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
        "{} / {} GiB",
        format_truncated_gib(used_mb),
        format_truncated_gib(total_mb)
    )
}

fn format_truncated_gib(value_mb: f64) -> String {
    let gib = (value_mb.max(0.0) / 1024.0 * 100.0).floor() / 100.0;
    format!("{gib:.2}")
}

fn status_dot(value: f64) -> &'static str {
    if value >= 85.0 {
        "spike"
    } else if value >= 60.0 {
        "busy"
    } else if value > 0.0 {
        "live"
    } else {
        "idle"
    }
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
