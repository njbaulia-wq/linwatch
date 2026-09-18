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

pub fn memory_tab(frame: &mut Frame, area: Rect, app: &AppState) {
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
    render_architecture_and_trend(frame, chunks[1], app, area.width < 90);
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

    // RAM Subsystem Card
    let ram_sev = Severity::from_usage(app.mem_pct());
    let ram_block = panel_block(" RAM Subsystem ");
    let ram_inner = ram_block.inner(cards[0]);
    frame.render_widget(ram_block, cards[0]);

    if ram_inner.height >= 2 {
        let ram_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(ram_inner);

        let ram_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(ram_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((app.mem_pct() / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("{} {:>5.1}%", ram_sev.symbol(), app.mem_pct()),
                Style::default()
                    .fg(severity_color(ram_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(ram_gauge, ram_rows[0]);

        let ram_text = Line::from(vec![
            dim("Active: "),
            styled(format!("{:.0} MB", app.mem_used), t.text),
            dim(format!(" / {:.0} MB", app.mem_total)),
            dim("  │  Avail: "),
            styled(format!("{:.0} MB", app.mem_available), t.accent_teal),
        ]);
        frame.render_widget(Paragraph::new(ram_text), ram_rows[1]);
    } else if ram_inner.height == 1 {
        let ram_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(ram_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((app.mem_pct() / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("RAM: {:.1}%", app.mem_pct()),
                Style::default()
                    .fg(severity_color(ram_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(ram_gauge, ram_inner);
    }

    // Swap Space Card
    let swap_sev = Severity::from_usage(app.swap_pct());
    let swap_block = panel_block(" Swap Space ");
    let swap_inner = swap_block.inner(cards[1]);
    frame.render_widget(swap_block, cards[1]);

    if app.swap_total > 0.0 {
        if swap_inner.height >= 2 {
            let swap_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(swap_inner);

            let swap_gauge = LineGauge::default()
                .gauge_style(
                    Style::default()
                        .fg(severity_color(swap_sev))
                        .add_modifier(Modifier::BOLD),
                )
                .line_set(ratatui::symbols::line::THICK)
                .ratio((app.swap_pct() / 100.0).clamp(0.0, 1.0))
                .label(Span::styled(
                    format!("{} {:>5.1}%", swap_sev.symbol(), app.swap_pct()),
                    Style::default()
                        .fg(severity_color(swap_sev))
                        .add_modifier(Modifier::BOLD),
                ));
            frame.render_widget(swap_gauge, swap_rows[0]);

            let swap_free = (app.swap_total - app.swap_used).max(0.0);
            let swap_text = Line::from(vec![
                dim("Used: "),
                styled(format!("{:.0} MB", app.swap_used), t.text),
                dim(format!(" / {:.0} MB", app.swap_total)),
                dim("  │  Free: "),
                styled(format!("{:.0} MB", swap_free), t.subtext0),
            ]);
            frame.render_widget(Paragraph::new(swap_text), swap_rows[1]);
        } else if swap_inner.height == 1 {
            let swap_gauge = LineGauge::default()
                .gauge_style(
                    Style::default()
                        .fg(severity_color(swap_sev))
                        .add_modifier(Modifier::BOLD),
                )
                .line_set(ratatui::symbols::line::THICK)
                .ratio((app.swap_pct() / 100.0).clamp(0.0, 1.0))
                .label(Span::styled(
                    format!("Swap: {:.1}%", app.swap_pct()),
                    Style::default()
                        .fg(severity_color(swap_sev))
                        .add_modifier(Modifier::BOLD),
                ));
            frame.render_widget(swap_gauge, swap_inner);
        }
    } else {
        let msg = vec![
            Line::from(dim("Swap paging is not enabled (0 MB)")),
            Line::from(dim("Host operates with physical memory only")),
        ];
        frame.render_widget(Paragraph::new(msg), swap_inner);
    }
}

fn render_architecture_and_trend(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();

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

    // Left Panel: Architecture & Kernel Cache
    let arch_block = panel_block(" Memory Composition & Cache ");
    let arch_inner = arch_block.inner(panels[0]);
    frame.render_widget(arch_block, panels[0]);

    if arch_inner.height >= 4 && arch_inner.width >= 16 {
        let total = app.mem_total.max(1.0);
        let used = app.mem_used.clamp(0.0, total);
        let cached = app.mem_cached.clamp(0.0, total);
        let buffers = app.mem_buffers.clamp(0.0, total);
        let cache_buf = (cached + buffers).clamp(0.0, (total - used).max(0.0));
        let free = (total - used - cache_buf).max(0.0);

        // Calculate bar segments
        let bar_width = (arch_inner.width as usize).saturating_sub(2);
        let w_used = ((used / total) * bar_width as f64).round() as usize;
        let w_cache = ((cache_buf / total) * bar_width as f64).round() as usize;
        let w_free = bar_width.saturating_sub(w_used + w_cache);

        let mut lines = Vec::new();

        // 1. Horizontal segmented bar
        lines.push(Line::from(vec![
            Span::raw("["),
            Span::styled("█".repeat(w_used), Style::default().fg(t.accent_yellow)),
            Span::styled("▒".repeat(w_cache), Style::default().fg(t.accent_teal)),
            Span::styled("░".repeat(w_free), Style::default().fg(t.overlay1)),
            Span::raw("]"),
        ]));

        // 2. Legend
        lines.push(Line::from(vec![
            Span::styled("■ ", Style::default().fg(t.accent_yellow)),
            dim("Used   "),
            Span::styled("■ ", Style::default().fg(t.accent_teal)),
            dim("Cache/Buf   "),
            Span::styled("□ ", Style::default().fg(t.overlay1)),
            dim("Free"),
        ]));

        // 3-6. Detailed Breakdown Tree
        let p_used = (used / total) * 100.0;
        let p_cached = (cached / total) * 100.0;
        let p_buffers = (buffers / total) * 100.0;
        let p_free = (free / total) * 100.0;

        lines.push(Line::from(vec![
            styled("├─ Active (Apps):    ", t.accent_yellow),
            styled(format!("{:>6.0} MB", used), t.text),
            dim(format!("  ({:>5.1}%)", p_used)),
        ]));
        lines.push(Line::from(vec![
            styled("├─ Page Cache:       ", t.accent_teal),
            styled(format!("{:>6.0} MB", cached), t.text),
            dim(format!("  ({:>5.1}%)", p_cached)),
        ]));
        lines.push(Line::from(vec![
            styled("├─ Kernel Buffers:   ", t.accent_blue),
            styled(format!("{:>6.0} MB", buffers), t.text),
            dim(format!("  ({:>5.1}%)", p_buffers)),
        ]));
        lines.push(Line::from(vec![
            styled("└─ Free (Zeroed):    ", t.subtext0),
            styled(format!("{:>6.0} MB", free), t.text),
            dim(format!("  ({:>5.1}%)", p_free)),
        ]));

        frame.render_widget(Paragraph::new(lines), arch_inner);
    } else if arch_inner.height >= 1 {
        let summary = Line::from(vec![
            styled(format!("Used: {:.0} MB", app.mem_used), t.accent_yellow),
            dim("  Cache: "),
            styled(format!("{:.0} MB", app.mem_cached), t.accent_teal),
            dim("  Free: "),
            styled(format!("{:.0} MB", app.mem_free), t.text),
        ]);
        frame.render_widget(Paragraph::new(summary), arch_inner);
    }

    // Right Panel: Utilization Trend Chart
    render_trend_chart(frame, panels[1], app);
}

fn render_trend_chart(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let current = app.mem_pct();
    let peak = app.mem_history.iter().map(|p| p.1).fold(current, f64::max);
    let avg = if app.mem_history.is_empty() {
        current
    } else {
        app.mem_history.iter().map(|p| p.1).sum::<f64>() / app.mem_history.len() as f64
    };

    let title = format!(
        " RAM Utilization Trend (120s) [Cur: {:.1}% │ Peak: {:.1}% │ Avg: {:.1}%] ",
        current, peak, avg
    );

    if app.mem_history.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting memory metrics...").block(panel_block(title)),
            area,
        );
        return;
    }

    let points: Vec<(f64, f64)> = app.mem_history.iter().copied().collect();
    let x_start = app.mem_history.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = app
        .mem_history
        .back()
        .map(|p| p.0)
        .unwrap_or(1.0)
        .max(x_start + 1.0);

    let line_set = ratatui::widgets::Dataset::default()
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(
            Style::default()
                .fg(t.accent_yellow)
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
                    Span::styled("now", Style::default().fg(t.accent_yellow)),
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
        " Top Memory Consumers ({} detected) ",
        app.top_mem_processes.len()
    );

    if app.top_mem_processes.is_empty() {
        frame.render_widget(
            Paragraph::new("No active process memory data recorded yet.").block(panel_block(title)),
            area,
        );
        return;
    }

    let max_rows = (area.height as usize).saturating_sub(3).max(1);

    let rows = app
        .top_mem_processes
        .iter()
        .take(max_rows)
        .map(|proc| {
            let pct_ram = if app.mem_total > 0.0 {
                (proc.mem_mb / app.mem_total) * 100.0
            } else {
                0.0
            };
            let sev = Severity::from_usage(pct_ram * 2.0);

            let mut cells = vec![
                Cell::from(Span::styled(
                    format!("{:<7}", proc.pid),
                    Style::default()
                        .fg(t.accent_blue)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(format!("{:>7.1} MB", proc.mem_mb)),
                Cell::from(Span::styled(
                    format!("{:>5.1}%", pct_ram),
                    Style::default().fg(severity_color(sev)),
                )),
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
        Constraint::Length(12),
        Constraint::Length(8),
    ];
    let mut headers = vec![
        Cell::from(header_col("PID")),
        Cell::from(header_col("MEM (MB)")),
        Cell::from(header_col("% RAM")),
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

    fn make_test_app() -> AppState {
        let mut mem_history = VecDeque::new();
        mem_history.push_back((0.0, 45.0));
        mem_history.push_back((1.0, 50.0));
        mem_history.push_back((2.0, 55.0));

        let top_mem_processes = vec![
            ProcessInfo {
                pid: 1234,
                ppid: 1,
                name: "rust-analyzer".into(),
                cpu_pct: 12.5,
                mem_mb: 850.0,
                threads: 16,
                state: "S".into(),
                reason: String::new(),
                is_high_risk: false,
                is_dev: true,
            },
            ProcessInfo {
                pid: 5678,
                ppid: 1,
                name: "browser".into(),
                cpu_pct: 4.2,
                mem_mb: 620.0,
                threads: 32,
                state: "S".into(),
                reason: String::new(),
                is_high_risk: false,
                is_dev: false,
            },
        ];

        AppState {
            system: SystemInfo {
                hostname: "test-box".into(),
                os_name: "Linux".into(),
                os_version: "6.8.0".into(),
                kernel: "6.8.0-generic".into(),
                cpu_model: "AMD Ryzen 9".into(),
                cpu_count: 16,
                selinux_mode: "Disabled".into(),
            },
            env: EnvKind::BareMetal,
            cpu_usage: 25.0,
            core_usages: vec![25.0; 16],
            cpu_history: VecDeque::new(),
            previous_cpu: None,
            mem_total: 16384.0,
            mem_used: 8192.0,
            mem_available: 8192.0,
            mem_free: 2048.0,
            mem_cached: 5120.0,
            mem_buffers: 1024.0,
            mem_history,
            swap_total: 8192.0,
            swap_used: 1024.0,
            disk: DiskInfo {
                mount_point: "/".into(),
                total_gb: 500.0,
                used_gb: 200.0,
                free_gb: 300.0,
                pct: 40,
                fs_type: "ext4".into(),
            },
            mounts: Vec::new(),
            uptime: "5h 12m".into(),
            load_avg: ["0.50".into(), "0.40".into(), "0.30".into()],
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
            process_count: 100,
            top_cpu_processes: Vec::new(),
            top_mem_processes,
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: HashMap::new(),
            process_sort: ProcessSort::MemDesc,
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
            active_tab: ViewTab::Memory,
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
    fn memory_renders_at_various_sizes() {
        let app = make_test_app();
        for (w, h) in [(60, 15), (80, 24), (100, 30), (120, 40), (160, 48)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    memory_tab(f, f.size(), &app);
                })
                .unwrap();
        }
    }

    #[test]
    fn memory_renders_with_zero_swap() {
        let mut app = make_test_app();
        app.swap_total = 0.0;
        app.swap_used = 0.0;
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                memory_tab(f, f.size(), &app);
            })
            .unwrap();
    }
}
