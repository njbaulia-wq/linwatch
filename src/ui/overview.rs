use std::collections::VecDeque;

use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Chart, GraphType, Paragraph, Sparkline, Wrap},
    Frame,
};

use super::common::*;
use super::theme;

pub fn overview(frame: &mut Frame, area: Rect, app: &AppState) {
    match breakpoint_for_area(area) {
        Breakpoint::Tiny => render_tiny_layout(frame, area, app),
        Breakpoint::Compact => render_compact_layout(frame, area, app),
        Breakpoint::Full | Breakpoint::Wide => render_full_layout(frame, area, app),
    }
}

// ─── TINY layout (width < 64 or height < 16) ─────────────────────────────────
fn render_tiny_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(6)])
        .split(area);

    render_kpi_grid_2x2(frame, chunks[0], app);
    render_diagnostics_and_events(frame, chunks[1], app);
}

// ─── COMPACT layout (width < 100 or height < 25) ─────────────────────────────
fn render_compact_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let top_height = if area.width < 100 { 8 } else { 5 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_height), Constraint::Min(8)])
        .split(area);

    if area.width < 100 {
        render_kpi_grid_2x2(frame, chunks[0], app);
    } else {
        render_kpi_row_4(frame, chunks[0], app);
    }

    render_body(frame, chunks[1], app);
}

// ─── FULL layout (width >= 100 and height >= 25) ────────────────────────────
fn render_full_layout(frame: &mut Frame, area: Rect, app: &AppState) {
    let kpi_height = if area.height >= 34 { 6 } else { 5 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(kpi_height), Constraint::Min(12)])
        .split(area);

    render_kpi_row_4(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
}

// ═══════════════════════════════════════════════════════════════════════════════
// KPI CARDS — Non-overlapping, rich core metric indicators
// ═══════════════════════════════════════════════════════════════════════════════

fn render_kpi_row_4(frame: &mut Frame, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    render_cpu_card(frame, cols[0], app);
    render_mem_card(frame, cols[1], app);
    render_storage_card(frame, cols[2], app);
    render_net_gpu_card(frame, cols[3], app);
}

fn render_kpi_grid_2x2(frame: &mut Frame, area: Rect, app: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_cpu_card(frame, top_cols[0], app);
    render_mem_card(frame, top_cols[1], app);
    render_storage_card(frame, bottom_cols[0], app);
    render_net_gpu_card(frame, bottom_cols[1], app);
}

fn render_cpu_card(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let sev = Severity::from_usage(app.cpu_usage);
    let block = panel_block(" CPU ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let sub = if let Some(temp) = app.temp_c {
        format!("{}c · {:.0}\u{b0}C", app.system.cpu_count, temp)
    } else {
        format!("{} cores", app.system.cpu_count)
    };

    let line1 = Line::from(vec![
        severity_chip(sev),
        Span::styled(
            format!(" {:>4.1}% ", app.cpu_usage),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            truncate(&sub, chunks[0].width.saturating_sub(12) as usize),
            Style::default().fg(t.overlay1),
        ),
    ]);
    frame.render_widget(Paragraph::new(line1), chunks[0]);

    if chunks[1].height > 0 && app.cpu_history.len() >= 2 {
        let points = sparkline_points(&app.cpu_history, chunks[1].width as usize);
        frame.render_widget(
            Sparkline::default()
                .data(&points)
                .max(100)
                .style(Style::default().fg(t.accent_teal)),
            chunks[1],
        );
    }
}

fn render_mem_card(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let pct = app.mem_pct();
    let sev = Severity::from_usage(pct);
    let block = panel_block(" Memory ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let sub = format!(
        "{:.1}/{:.1} GB",
        app.mem_used / 1024.0,
        app.mem_total / 1024.0
    );

    let line1 = Line::from(vec![
        severity_chip(sev),
        Span::styled(
            format!(" {:>4.1}% ", pct),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            truncate(&sub, chunks[0].width.saturating_sub(12) as usize),
            Style::default().fg(t.overlay1),
        ),
    ]);
    frame.render_widget(Paragraph::new(line1), chunks[0]);

    if chunks[1].height > 0 && app.mem_history.len() >= 2 {
        let points = sparkline_points(&app.mem_history, chunks[1].width as usize);
        frame.render_widget(
            Sparkline::default()
                .data(&points)
                .max(100)
                .style(Style::default().fg(t.accent_yellow)),
            chunks[1],
        );
    }
}

fn render_storage_card(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let pct = app.disk.pct as f64;
    let sev = Severity::from_usage(pct);
    let block = panel_block(" Storage ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let sub = format!("{:.0}/{:.0} GB", app.disk.used_gb, app.disk.total_gb);
    let line1 = Line::from(vec![
        severity_chip(sev),
        Span::styled(
            format!(" {:>3.0}% ", pct),
            Style::default()
                .fg(severity_color(sev))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            truncate(&sub, chunks[0].width.saturating_sub(10) as usize),
            Style::default().fg(t.overlay1),
        ),
    ]);
    frame.render_widget(Paragraph::new(line1), chunks[0]);

    // Show active disk I/O rates
    let total_read: f64 = app.disk_io.iter().map(|d| d.read_bps).sum();
    let total_write: f64 = app.disk_io.iter().map(|d| d.write_bps).sum();
    let io_text = if total_read > 0.0 || total_write > 0.0 {
        format!(
            "R: {}/s  W: {}/s",
            format_bytes(total_read),
            format_bytes(total_write)
        )
    } else {
        format!("Mount: {}", app.disk.mount_point)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            truncate(&io_text, chunks[1].width as usize),
            Style::default().fg(t.subtext0),
        )),
        chunks[1],
    );
}

fn render_net_gpu_card(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" Network & GPU ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Network line
    let net_line = Line::from(vec![
        Span::styled(
            "↓",
            Style::default()
                .fg(t.accent_teal)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", format_bytes(app.net_down_bps)),
            Style::default().fg(t.text),
        ),
        Span::styled(
            "↑",
            Style::default()
                .fg(t.accent_orange)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format_bytes(app.net_up_bps), Style::default().fg(t.text)),
    ]);
    frame.render_widget(Paragraph::new(net_line), chunks[0]);

    // GPU line or connection stats
    let gpu_text = if let Some(gpu) = app.primary_gpu() {
        let usage = gpu
            .usage_pct
            .map(|u| format!("{u:.0}%"))
            .unwrap_or_else(|| String::from("N/A"));
        let temp = gpu
            .temp_c
            .map(|c| format!("{c:.0}\u{b0}C"))
            .unwrap_or_default();
        format!("GPU {} {} {}", gpu.kind, usage, temp)
    } else {
        format!("Ports: {} listening", app.open_ports.len())
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            truncate(&gpu_text, chunks[1].width as usize),
            Style::default().fg(t.subtext0),
        )),
        chunks[1],
    );
}

fn sparkline_points(history: &VecDeque<(f64, f64)>, width: usize) -> Vec<u64> {
    history
        .iter()
        .rev()
        .take(width)
        .rev()
        .map(|(_, v)| v.max(0.0).round() as u64)
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// BODY — Middle trends and bottom intelligence
// ═══════════════════════════════════════════════════════════════════════════════

fn render_body(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.height < 10 {
        render_diagnostics_and_events(frame, area, app);
        return;
    }

    // Two vertical sections: Top is Trends + Top Consumers; Bottom is Diagnostics + Events
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_activity_section(frame, rows[0], app);
    render_diagnostics_and_events(frame, rows[1], app);
}

fn render_activity_section(frame: &mut Frame, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_trend_charts(frame, cols[0], app);
    render_top_consumer_list(frame, cols[1], app);
}

fn render_trend_charts(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" Real-time Activity Trends (120s) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let cpu_peak = app.cpu_history.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let cpu_title = format!("CPU · {:.1}% (Peak {:.0}%)", app.cpu_usage, cpu_peak);
    render_braille_chart(
        frame,
        chunks[0],
        &cpu_title,
        &app.cpu_history,
        t.accent_teal,
    );

    let mem_peak = app.mem_history.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let mem_title = format!("Memory · {:.1}% (Peak {:.0}%)", app.mem_pct(), mem_peak);
    render_braille_chart(
        frame,
        chunks[1],
        &mem_title,
        &app.mem_history,
        t.accent_yellow,
    );
}

fn render_braille_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    if area.height < 3 || area.width < 10 {
        return;
    }

    let title_line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]);

    if data.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting telemetries...").style(Style::default().fg(t.overlay1)),
            area,
        );
        return;
    }

    let points: Vec<(f64, f64)> = data.iter().copied().collect();
    let dataset = ratatui::widgets::Dataset::default()
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .data(&points);

    let chart = Chart::new(vec![dataset])
        .block(Block::default().title(title_line))
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

fn render_top_consumer_list(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" Top Resource Consumers ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 14 {
        return;
    }

    let header_line = Line::from(vec![
        Span::styled(
            " PID    ",
            Style::default().fg(t.overlay1).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "CPU%  ",
            Style::default().fg(t.overlay1).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "MEM     ",
            Style::default().fg(t.overlay1).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "COMMAND",
            Style::default().fg(t.overlay1).add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut lines = vec![header_line];
    let max_rows = inner.height.saturating_sub(1) as usize;

    for p in app.top_cpu_processes.iter().take(max_rows) {
        let is_risk = p.is_high_risk;
        let sev = if is_risk {
            Severity::Critical
        } else {
            Severity::Ok
        };
        let name_w = (inner.width as usize).saturating_sub(24).max(10);

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<7} ", p.pid),
                Style::default().fg(if is_risk { t.accent_red } else { t.text }),
            ),
            Span::styled(
                format!("{:>4.1}% ", p.cpu_pct),
                Style::default()
                    .fg(if is_risk { t.accent_red } else { t.accent_teal })
                    .add_modifier(if is_risk {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!("{:>6.1}M ", p.mem_mb),
                Style::default().fg(t.overlay1),
            ),
            severity_chip(sev),
            Span::styled(
                format!(" {}", truncate(&p.name, name_w)),
                Style::default().fg(if is_risk { t.accent_red } else { t.text }),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DIAGNOSTICS & EVENT STREAM
// ═══════════════════════════════════════════════════════════════════════════════

fn render_diagnostics_and_events(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.height < 4 {
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_root_cause_panel(frame, cols[0], app);
    render_events_alerts_panel(frame, cols[1], app);
}

fn render_root_cause_panel(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let primary_cause = app.root_causes.first();
    let is_critical = primary_cause.is_some_and(|c| c.severity == Severity::Critical);
    let border_sev = if is_critical {
        Severity::Critical
    } else {
        Severity::Ok
    };

    let block = panel_block_severity(" Root-Cause Diagnostics ", border_sev);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines = Vec::new();
    let active_causes: Vec<_> = app
        .root_causes
        .iter()
        .filter(|c| c.severity != Severity::Ok)
        .collect();

    if active_causes.is_empty() {
        lines.push(Line::from(vec![
            severity_chip(Severity::Ok),
            Span::styled(
                " System Nominal ",
                Style::default()
                    .fg(t.accent_green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "All telemetry indicators are operating within standard thresholds.",
            Style::default().fg(t.subtext0),
        )));
        lines.push(Line::from(Span::styled(
            "No CPU throttling, memory starvation, or failed services detected.",
            Style::default().fg(t.overlay1),
        )));
    } else {
        for cause in active_causes.iter().take(inner.height as usize) {
            lines.push(Line::from(vec![
                severity_chip(cause.severity),
                Span::styled(
                    format!(" {}: ", cause.title),
                    Style::default()
                        .fg(severity_color(cause.severity))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    truncate(
                        &cause.detail,
                        inner.width.saturating_sub(cause.title.len() as u16 + 8) as usize,
                    ),
                    Style::default().fg(t.text),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn render_events_alerts_panel(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" System Events & Timeline ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines = Vec::new();

    // Show failed systemd units if any
    for unit in app.failed_units.iter().take(2) {
        lines.push(Line::from(vec![
            severity_chip(Severity::Critical),
            Span::styled(
                format!(" Failed: {}", truncate(&unit.unit, 18)),
                Style::default()
                    .fg(t.accent_red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}/{})", unit.active, unit.sub),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    // Show recent events
    for event in app
        .events
        .iter()
        .rev()
        .take(inner.height.saturating_sub(lines.len() as u16) as usize)
    {
        lines.push(Line::from(vec![
            severity_chip(event.severity),
            Span::styled(
                format!(" {}", truncate_words(&event.title, 18)),
                Style::default().fg(severity_color(event.severity)),
            ),
            Span::styled(
                format!(
                    " · {}",
                    truncate(&event.detail, inner.width.saturating_sub(22) as usize)
                ),
                Style::default().fg(t.overlay1),
            ),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "● Event stream idle — no anomalous state transitions recorded.",
            Style::default().fg(t.overlay1),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
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
