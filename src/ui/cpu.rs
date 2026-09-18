use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Cell, Chart, GraphType, LineGauge, Paragraph, Row, Table},
    Frame,
};

use crate::state::AppState;
use crate::types::Severity;

use super::common::*;
use super::theme;

pub fn cpu_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.width < 10 || area.height < 5 {
        return;
    }

    let top_height = if area.height < 24 { 3 } else { 5 };
    let middle_height = if area.height < 28 { 7 } else { 9 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_height),
            Constraint::Length(middle_height),
            Constraint::Min(6),
        ])
        .split(area);

    render_vitals(frame, chunks[0], app, area.width < 80);
    render_cores_and_trend(frame, chunks[1], app, area.width < 90);
    render_top_consumers(frame, chunks[2], app, area.width < 80);
}

fn render_vitals(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();

    let cards = if narrow {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
    };

    // CPU Subsystem & Vitals Card
    let cpu_sev = Severity::from_usage(app.cpu_usage);
    let vitals_block = panel_block(" CPU Subsystem & Vitals ");
    let vitals_inner = vitals_block.inner(cards[0]);
    frame.render_widget(vitals_block, cards[0]);

    if vitals_inner.height >= 2 {
        let vitals_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(vitals_inner);

        let cpu_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(cpu_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((app.cpu_usage / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("{} {:>5.1}%", cpu_sev.symbol(), app.cpu_usage),
                Style::default()
                    .fg(severity_color(cpu_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(cpu_gauge, vitals_rows[0]);

        let line2 = Line::from(vec![
            dim("Load: "),
            styled(
                format!(
                    "{} {} {}",
                    app.load_avg[0], app.load_avg[1], app.load_avg[2]
                ),
                t.text,
            ),
            dim("  │  Tasks: "),
            styled(format!("{} proc", app.process_count), t.accent_blue),
            dim("  │  Status: "),
            styled(severity_word(cpu_sev), severity_color(cpu_sev)),
        ]);
        frame.render_widget(Paragraph::new(line2), vitals_rows[1]);
    } else if vitals_inner.height == 1 {
        let cpu_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(cpu_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((app.cpu_usage / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("CPU: {:.1}%", app.cpu_usage),
                Style::default()
                    .fg(severity_color(cpu_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(cpu_gauge, vitals_inner);
    }

    // Processor Specification Card
    let spec_block = panel_block(" Processor Specification ");
    let spec_inner = spec_block.inner(cards[1]);
    frame.render_widget(spec_block, cards[1]);

    if spec_inner.height >= 2 {
        let spec_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(spec_inner);

        let model_text = Line::from(vec![Span::styled(
            truncate(&app.system.cpu_model, 44),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        )]);
        frame.render_widget(Paragraph::new(model_text), spec_rows[0]);

        let arch_text = Line::from(vec![
            dim("Cores: "),
            styled(format!("{}", app.system.cpu_count), t.accent_blue),
            dim(" logical  │  Env: "),
            styled(app.env.label(), t.accent_teal),
            dim("  │  Kernel: "),
            styled(truncate(&app.system.kernel, 18), t.subtext0),
        ]);
        frame.render_widget(Paragraph::new(arch_text), spec_rows[1]);
    } else if spec_inner.height == 1 {
        let text = Line::from(vec![
            styled(truncate(&app.system.cpu_model, 26), t.text),
            dim(" ("),
            styled(format!("{} cores", app.system.cpu_count), t.accent_blue),
            dim(")"),
        ]);
        frame.render_widget(Paragraph::new(text), spec_inner);
    }
}

fn render_cores_and_trend(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let panels = if narrow {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area)
    };

    render_cores_matrix(frame, panels[0], app);
    render_trend_chart(frame, panels[1], app);
}

fn render_cores_matrix(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let total_cores = app.core_usages.len();
    let title = format!(" CPU Cores Activity ({total_cores}) ");
    let block = panel_block(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if total_cores == 0 {
        frame.render_widget(
            Paragraph::new(dim("No per-core telemetry recorded yet.")),
            inner,
        );
        return;
    }

    if inner.height == 0 || inner.width < 10 {
        return;
    }

    let avail_w = inner.width as usize;
    let avail_h = inner.height as usize;

    // Determine grid columns:
    // When width is >= 54: 3 columns, >= 34: 2 columns, otherwise 1 column.
    let cols_count = if avail_w >= 54 {
        3
    } else if avail_w >= 34 {
        2
    } else {
        1
    };

    let col_w = (avail_w / cols_count).saturating_sub(1);
    let rows_count = total_cores.div_ceil(cols_count);
    let display_rows = rows_count.min(avail_h);
    let overflow = total_cores.saturating_sub(display_rows * cols_count);

    let mut lines = Vec::with_capacity(display_rows);

    for r in 0..display_rows {
        // If this is the last row and there is overflow, show overflow summary
        if r == display_rows - 1 && overflow > 0 {
            lines.push(Line::from(vec![dim(format!(
                "+ {overflow} more cores actively running"
            ))]));
            continue;
        }

        let mut spans = Vec::new();
        for c in 0..cols_count {
            let core_idx = r * cols_count + c;
            if core_idx < total_cores {
                let usage = app.core_usages[core_idx];
                let sev = Severity::from_usage(usage);
                let color = severity_color(sev);

                let bar_len = col_w.saturating_sub(10).clamp(2, 6);
                let filled = ((usage / 100.0).clamp(0.0, 1.0) * bar_len as f64).round() as usize;
                let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

                spans.push(styled(format!("C{:<2} ", core_idx), t.subtext0));
                spans.push(styled(format!("{:>3.0}% ", usage), color));
                spans.push(styled(bar, color));

                if c + 1 < cols_count && (core_idx + 1) < total_cores {
                    spans.push(dim(" │ "));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_trend_chart(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let current = app.cpu_usage;
    let peak = app.cpu_history.iter().map(|p| p.1).fold(current, f64::max);
    let avg = if app.cpu_history.is_empty() {
        current
    } else {
        app.cpu_history.iter().map(|p| p.1).sum::<f64>() / app.cpu_history.len() as f64
    };

    let title = format!(
        " CPU Utilization Trend (120s) [Cur: {:.1}% │ Peak: {:.1}% │ Avg: {:.1}%] ",
        current, peak, avg
    );

    if app.cpu_history.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting CPU metrics...").block(panel_block(title)),
            area,
        );
        return;
    }

    let points: Vec<(f64, f64)> = app.cpu_history.iter().copied().collect();
    let x_start = app.cpu_history.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = app
        .cpu_history
        .back()
        .map(|p| p.0)
        .unwrap_or(1.0)
        .max(x_start + 1.0);

    let line_set = ratatui::widgets::Dataset::default()
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(
            Style::default()
                .fg(t.accent_teal)
                .add_modifier(Modifier::BOLD),
        )
        .data(&points);

    let chart = Chart::new(vec![line_set])
        .block(panel_block(title))
        .x_axis(
            Axis::default()
                .bounds([x_start, x_end])
                .labels(vec![
                    Span::styled("-120s", Style::default().fg(t.overlay1)),
                    Span::styled("now", Style::default().fg(t.accent_teal)),
                ])
                .style(Style::default().fg(t.overlay1)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(vec![
                    Span::styled("0%", Style::default().fg(t.overlay1)),
                    Span::styled("50%", Style::default().fg(t.overlay1)),
                    Span::styled("100%", Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}

fn render_top_consumers(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();

    let title = format!(
        " Top CPU Consumers ({} detected) ",
        app.top_cpu_processes.len()
    );

    if app.top_cpu_processes.is_empty() {
        frame.render_widget(
            Paragraph::new("No active process telemetry recorded yet.").block(panel_block(title)),
            area,
        );
        return;
    }

    let max_rows = (area.height as usize).saturating_sub(3).max(1);

    let rows = app
        .top_cpu_processes
        .iter()
        .take(max_rows)
        .map(|proc| {
            let sev = Severity::from_usage(proc.cpu_pct);

            let mut cells = vec![
                Cell::from(Span::styled(
                    format!("{:<7}", proc.pid),
                    Style::default()
                        .fg(t.accent_blue)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    format!("{:>5.1}%", proc.cpu_pct),
                    Style::default()
                        .fg(severity_color(sev))
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(format!("{:>7.1} MB", proc.mem_mb)),
            ];

            if !narrow {
                cells.push(Cell::from(proc.state.as_str()));
                cells.push(Cell::from(proc.threads.to_string()));
            }

            let name_col = if proc.is_high_risk {
                Span::styled(
                    format!("! {}", proc.name),
                    Style::default()
                        .fg(t.accent_red)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(&proc.name, Style::default().fg(t.text))
            };
            cells.push(Cell::from(name_col));

            Row::new(cells).style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    let mut widths = vec![
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Length(12),
    ];
    let mut headers = vec![
        Cell::from(header_col("PID")),
        Cell::from(header_col("CPU %")),
        Cell::from(header_col("MEM (MB)")),
    ];

    if !narrow {
        widths.push(Constraint::Length(7));
        widths.push(Constraint::Length(8));
        headers.push(Cell::from(header_col("State")));
        headers.push(Cell::from(header_col("Threads")));
    }
    widths.push(Constraint::Min(14));
    headers.push(Cell::from(header_col("Command / Name")));

    let table = Table::new(rows, widths)
        .header(Row::new(headers))
        .block(panel_block(title))
        .column_spacing(1);

    frame.render_widget(table, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::types::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::{HashMap, VecDeque};
    use std::time::Instant;

    fn make_test_app(core_count: usize) -> AppState {
        let mut cpu_history = VecDeque::new();
        cpu_history.push_back((0.0, 15.0));
        cpu_history.push_back((1.0, 45.0));
        cpu_history.push_back((2.0, 60.0));

        let top_cpu_processes = vec![
            ProcessInfo {
                pid: 1234,
                ppid: 1,
                name: "cargo-build".into(),
                cpu_pct: 78.5,
                mem_mb: 512.0,
                threads: 16,
                state: "R".into(),
                reason: String::new(),
                is_high_risk: false,
                is_dev: true,
            },
            ProcessInfo {
                pid: 5678,
                ppid: 1,
                name: "rust-analyzer".into(),
                cpu_pct: 22.0,
                mem_mb: 850.0,
                threads: 12,
                state: "S".into(),
                reason: String::new(),
                is_high_risk: false,
                is_dev: false,
            },
        ];

        AppState {
            system: SystemInfo {
                hostname: "cpu-test".into(),
                os_name: "Linux".into(),
                os_version: "6.8.0".into(),
                kernel: "6.8.0-generic".into(),
                cpu_model: "AMD Ryzen 9 7950X".into(),
                cpu_count: core_count,
                selinux_mode: "Disabled".into(),
            },
            env: EnvKind::BareMetal,
            cpu_usage: 42.5,
            core_usages: vec![40.0; core_count],
            cpu_history,
            previous_cpu: None,
            mem_total: 32768.0,
            mem_used: 16384.0,
            mem_available: 16384.0,
            mem_free: 8192.0,
            mem_cached: 6144.0,
            mem_buffers: 2048.0,
            mem_history: VecDeque::new(),
            swap_total: 16384.0,
            swap_used: 512.0,
            disk: DiskInfo {
                mount_point: "/".into(),
                total_gb: 512.0,
                used_gb: 200.0,
                free_gb: 312.0,
                pct: 39,
                fs_type: "ext4".into(),
            },
            mounts: Vec::new(),
            uptime: "8h 15m".into(),
            load_avg: ["1.20".into(), "0.85".into(), "0.50".into()],
            battery_pct: None,
            battery_status: "AC".into(),
            net_down_bps: 0.0,
            net_up_bps: 0.0,
            net_down_history: VecDeque::new(),
            net_up_history: VecDeque::new(),
            interfaces: Vec::new(),
            previous_net: None,
            disk_io: Vec::new(),
            previous_disk_io: None,
            disk_read_bps: 0.0,
            disk_write_bps: 0.0,
            disk_read_history: VecDeque::new(),
            disk_write_history: VecDeque::new(),
            gpus: Vec::new(),
            previous_gpu_rc6: None,
            gpu_usage_history: VecDeque::new(),
            gpu_temp_history: VecDeque::new(),
            temp_c: None,
            temp_history: VecDeque::new(),
            process_count: 142,
            top_cpu_processes,
            top_mem_processes: Vec::new(),
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: HashMap::new(),
            process_sort: ProcessSort::CpuDesc,
            process_history: HashMap::new(),
            process_selected: 0,
            is_tree_view: false,
            health_score: 95,
            alerts: Vec::new(),
            successful_reads: 10,
            failed_reads: 0,
            degraded_sources: Vec::new(),
            last_sample_at: Instant::now(),
            counter: 1,
            show_help: false,
            refresh_index: 1,
            active_tab: ViewTab::Cpu,
            tick_count: 1,
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
            previous_sample_status_label: "OK".into(),
            previous_health_band: Severity::Ok,
        }
    }

    #[test]
    fn cpu_renders_at_various_sizes() {
        let app = make_test_app(8);
        for (w, h) in [(60, 15), (80, 24), (100, 30), (120, 40), (160, 48)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    cpu_tab(f, f.size(), &app);
                })
                .unwrap();
        }
    }

    #[test]
    fn cpu_renders_with_many_cores() {
        let app = make_test_app(32);
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                cpu_tab(f, f.size(), &app);
            })
            .unwrap();
    }
}
