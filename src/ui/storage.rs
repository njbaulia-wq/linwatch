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

pub fn storage_tab(frame: &mut Frame, area: Rect, app: &AppState) {
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
    render_io_chart(frame, chunks[1], app);
    render_mounts_and_health(frame, chunks[2], app, area.width < 90);
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

    // Root Filesystem Card
    let disk_pct = app.disk.pct as f64;
    let disk_sev = Severity::from_usage(disk_pct);
    let root_block = panel_block(" Root Filesystem (/) ");
    let root_inner = root_block.inner(cards[0]);
    frame.render_widget(root_block, cards[0]);

    if root_inner.height >= 2 {
        let root_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(root_inner);

        let root_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(disk_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((disk_pct / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("{} {:>3}%", disk_sev.symbol(), app.disk.pct),
                Style::default()
                    .fg(severity_color(disk_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(root_gauge, root_rows[0]);

        let fs_label = if app.disk.fs_type.is_empty() {
            "auto"
        } else {
            &app.disk.fs_type
        };
        let root_text = Line::from(vec![
            dim("Used: "),
            styled(format!("{:.1} GB", app.disk.used_gb), t.text),
            dim(format!(" / {:.1} GB", app.disk.total_gb)),
            dim("  │  Free: "),
            styled(format!("{:.1} GB", app.disk.free_gb), t.accent_teal),
            dim("  │  Type: "),
            styled(fs_label, t.accent_blue),
        ]);
        frame.render_widget(Paragraph::new(root_text), root_rows[1]);
    } else if root_inner.height == 1 {
        let root_gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(disk_sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((disk_pct / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("Root: {}%", app.disk.pct),
                Style::default()
                    .fg(severity_color(disk_sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(root_gauge, root_inner);
    }

    // Disk Activity Card
    let act_block = panel_block(" Real-Time Disk Activity ");
    let act_inner = act_block.inner(cards[1]);
    frame.render_widget(act_block, cards[1]);

    if act_inner.height >= 2 {
        let act_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(act_inner);

        let io_rates = Line::from(vec![
            Span::styled(
                "▲ Read:  ",
                Style::default()
                    .fg(t.accent_teal)
                    .add_modifier(Modifier::BOLD),
            ),
            styled(format!("{}/s", format_bytes(app.disk_read_bps)), t.text),
            dim("    "),
            Span::styled(
                "▼ Write: ",
                Style::default()
                    .fg(t.accent_orange)
                    .add_modifier(Modifier::BOLD),
            ),
            styled(format!("{}/s", format_bytes(app.disk_write_bps)), t.text),
        ]);
        frame.render_widget(Paragraph::new(io_rates), act_rows[0]);

        let total_rate = app.disk_read_bps + app.disk_write_bps;
        let io_meta = Line::from(vec![
            dim("Active Devices: "),
            styled(format!("{} block dev", app.disk_io.len()), t.accent_blue),
            dim("  │  Combined: "),
            styled(format!("{}/s", format_bytes(total_rate)), t.text),
        ]);
        frame.render_widget(Paragraph::new(io_meta), act_rows[1]);
    } else if act_inner.height == 1 {
        let line = Line::from(vec![
            styled(
                format!("R: {}/s", format_bytes(app.disk_read_bps)),
                t.accent_teal,
            ),
            dim(" │ "),
            styled(
                format!("W: {}/s", format_bytes(app.disk_write_bps)),
                t.accent_orange,
            ),
        ]);
        frame.render_widget(Paragraph::new(line), act_inner);
    }
}

fn render_io_chart(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let max_read = app
        .disk_read_history
        .iter()
        .map(|p| p.1)
        .fold(0.0f64, f64::max);
    let max_write = app
        .disk_write_history
        .iter()
        .map(|p| p.1)
        .fold(0.0f64, f64::max);
    let peak_io = max_read
        .max(max_write)
        .max(app.disk_read_bps)
        .max(app.disk_write_bps);

    let title = format!(
        " Disk I/O Throughput Trend (120s) [▲ Read: {}/s │ ▼ Write: {}/s │ Peak: {}/s] ",
        format_bytes(app.disk_read_bps),
        format_bytes(app.disk_write_bps),
        format_bytes(peak_io)
    );

    if app.disk_read_history.is_empty() && app.disk_write_history.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting disk I/O metrics...").block(panel_block(title)),
            area,
        );
        return;
    }

    let y_max = (peak_io * 1.2).max(1024.0 * 1024.0);
    let points_read: Vec<(f64, f64)> = app.disk_read_history.iter().copied().collect();
    let points_write: Vec<(f64, f64)> = app.disk_write_history.iter().copied().collect();

    let x_start = app
        .disk_read_history
        .front()
        .or_else(|| app.disk_write_history.front())
        .map(|p| p.0)
        .unwrap_or(0.0);
    let x_end = app
        .disk_read_history
        .back()
        .or_else(|| app.disk_write_history.back())
        .map(|p| p.0)
        .unwrap_or(1.0)
        .max(x_start + 1.0);

    let mut datasets = Vec::new();
    if !points_read.is_empty() {
        datasets.push(
            ratatui::widgets::Dataset::default()
                .name("Read")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(t.accent_teal)
                        .add_modifier(Modifier::BOLD),
                )
                .data(&points_read),
        );
    }
    if !points_write.is_empty() {
        datasets.push(
            ratatui::widgets::Dataset::default()
                .name("Write")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(t.accent_orange)
                        .add_modifier(Modifier::BOLD),
                )
                .data(&points_write),
        );
    }

    let y_half = y_max * 0.5;
    let chart = Chart::new(datasets)
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
                .bounds([0.0, y_max])
                .labels(vec![
                    Span::styled("0", Style::default().fg(t.overlay1)),
                    Span::styled(
                        format!("{}/s", format_bytes(y_half)),
                        Style::default().fg(t.overlay1),
                    ),
                    Span::styled(
                        format!("{}/s", format_bytes(y_max)),
                        Style::default().fg(t.overlay1),
                    ),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}

fn render_mounts_and_health(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();

    let panels = if narrow {
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

    // Left Panel: Filesystem Mount Points Table
    let mount_rows = app
        .mounts
        .iter()
        .map(|m| {
            let sev = Severity::from_usage(m.pct as f64);
            let color = severity_color(sev);

            let bar_len = 6;
            let filled = ((m.pct as f64 / 100.0).clamp(0.0, 1.0) * bar_len as f64).round() as usize;
            let bar_display = format!(
                "[{}{}] {:>3}%",
                "█".repeat(filled),
                "░".repeat(bar_len - filled),
                m.pct
            );

            let fs = if m.fs_type.is_empty() {
                "-"
            } else {
                &m.fs_type
            };

            Row::new(vec![
                Cell::from(truncate(&m.mount_point, 18)),
                Cell::from(Span::styled(fs, Style::default().fg(t.accent_blue))),
                Cell::from(Span::styled(
                    bar_display,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Cell::from(format!(
                    "{:.1}/{:.1} GB ({:.1}G free)",
                    m.used_gb, m.total_gb, m.free_gb
                )),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    let mount_title = format!(" Filesystem Mount Points ({}) ", app.mounts.len());
    if mount_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No filesystem mount points detected").block(panel_block(mount_title)),
            panels[0],
        );
    } else {
        let table = Table::new(
            mount_rows,
            [
                Constraint::Length(14),
                Constraint::Length(8),
                Constraint::Length(15),
                Constraint::Min(16),
            ],
        )
        .header(Row::new(vec![
            Cell::from(header_col("Mount")),
            Cell::from(header_col("Type")),
            Cell::from(header_col("Usage")),
            Cell::from(header_col("Capacity")),
        ]))
        .block(panel_block(mount_title))
        .column_spacing(1);

        frame.render_widget(table, panels[0]);
    }

    // Right Panel: Drive Health OR Block Device Activity Fallback
    if !app.storage_health.is_empty() {
        render_drive_health_table(frame, panels[1], app);
    } else {
        render_block_device_table(frame, panels[1], app);
    }
}

fn render_drive_health_table(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let compact = area.width < 50;

    let rows = app
        .storage_health
        .iter()
        .map(|drive| {
            let color = severity_color(drive.risk);
            let temp = drive
                .temp_c
                .map(|v| format!("{v:.0}\u{b0}C"))
                .unwrap_or_else(|| String::from("N/A"));

            let mut cells = vec![
                Cell::from(Span::styled(
                    format!("{} {}", drive.risk.symbol(), drive.device),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Cell::from(drive.kind.as_str()),
                Cell::from(truncate(&drive.model, 16)),
                Cell::from(temp),
            ];

            if !compact {
                cells.push(Cell::from(truncate_words(&drive.note, 24)));
            }

            Row::new(cells).style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    let mut widths = vec![
        Constraint::Length(11),
        Constraint::Length(7),
        Constraint::Length(17),
        Constraint::Length(8),
    ];
    let mut headers = vec![
        Cell::from(header_col("Device")),
        Cell::from(header_col("Kind")),
        Cell::from(header_col("Model")),
        Cell::from(header_col("Temp")),
    ];
    if !compact {
        widths.push(Constraint::Min(12));
        headers.push(Cell::from(header_col("Health / Note")));
    }

    let title = format!(" Physical Drive Health ({}) ", app.storage_health.len());
    let table = Table::new(rows, widths)
        .header(Row::new(headers))
        .block(panel_block(title))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_block_device_table(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let title = format!(" Block Device Activity ({}) ", app.disk_io.len());

    if app.disk_io.is_empty() {
        frame.render_widget(
            Paragraph::new("No block device activity detected").block(panel_block(title)),
            area,
        );
        return;
    }

    let rows = app
        .disk_io
        .iter()
        .map(|dio| {
            let active = dio.read_bps > 0.0 || dio.write_bps > 0.0;
            let status_span = if active {
                Span::styled(
                    "● ACTIVE",
                    Style::default()
                        .fg(t.accent_teal)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("○ IDLE", Style::default().fg(t.overlay1))
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    &dio.device,
                    Style::default().fg(t.text).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    format!("{}/s", format_bytes(dio.read_bps)),
                    Style::default().fg(t.accent_teal),
                )),
                Cell::from(Span::styled(
                    format!("{}/s", format_bytes(dio.write_bps)),
                    Style::default().fg(t.accent_orange),
                )),
                Cell::from(status_span),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Min(10),
        ],
    )
    .header(Row::new(vec![
        Cell::from(header_col("Device")),
        Cell::from(header_col("Read")),
        Cell::from(header_col("Write")),
        Cell::from(header_col("Status")),
    ]))
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
        let mut disk_read_history = VecDeque::new();
        let mut disk_write_history = VecDeque::new();
        disk_read_history.push_back((0.0, 1024.0 * 500.0));
        disk_read_history.push_back((1.0, 1024.0 * 1024.0 * 2.0));
        disk_write_history.push_back((0.0, 1024.0 * 200.0));
        disk_write_history.push_back((1.0, 1024.0 * 1024.0 * 5.0));

        let mounts = vec![
            DiskInfo {
                mount_point: "/".into(),
                total_gb: 512.0,
                used_gb: 256.0,
                free_gb: 256.0,
                pct: 50,
                fs_type: "ext4".into(),
            },
            DiskInfo {
                mount_point: "/home".into(),
                total_gb: 1024.0,
                used_gb: 300.0,
                free_gb: 724.0,
                pct: 29,
                fs_type: "btrfs".into(),
            },
            DiskInfo {
                mount_point: "/boot/efi".into(),
                total_gb: 1.0,
                used_gb: 0.1,
                free_gb: 0.9,
                pct: 10,
                fs_type: "vfat".into(),
            },
        ];

        let disk_io = vec![
            DiskIoInfo {
                device: "nvme0n1".into(),
                read_bps: 1024.0 * 1024.0 * 2.0,
                write_bps: 1024.0 * 1024.0 * 5.0,
            },
            DiskIoInfo {
                device: "sda".into(),
                read_bps: 0.0,
                write_bps: 0.0,
            },
        ];

        AppState {
            system: SystemInfo {
                hostname: "storage-box".into(),
                os_name: "Linux".into(),
                os_version: "6.8.0".into(),
                kernel: "6.8.0-generic".into(),
                cpu_model: "AMD Ryzen".into(),
                cpu_count: 8,
                selinux_mode: "Disabled".into(),
            },
            env: EnvKind::BareMetal,
            cpu_usage: 10.0,
            core_usages: vec![10.0; 8],
            cpu_history: VecDeque::new(),
            previous_cpu: None,
            mem_total: 16384.0,
            mem_used: 4096.0,
            mem_available: 12288.0,
            mem_free: 8192.0,
            mem_cached: 3072.0,
            mem_buffers: 1024.0,
            mem_history: VecDeque::new(),
            swap_total: 4096.0,
            swap_used: 0.0,
            disk: DiskInfo {
                mount_point: "/".into(),
                total_gb: 512.0,
                used_gb: 256.0,
                free_gb: 256.0,
                pct: 50,
                fs_type: "ext4".into(),
            },
            mounts,
            uptime: "12h 30m".into(),
            load_avg: ["0.10".into(), "0.20".into(), "0.30".into()],
            battery_pct: None,
            battery_status: "AC".into(),
            net_down_bps: 0.0,
            net_up_bps: 0.0,
            net_down_history: VecDeque::new(),
            net_up_history: VecDeque::new(),
            interfaces: Vec::new(),
            previous_net: None,
            disk_io,
            previous_disk_io: None,
            disk_read_bps: 1024.0 * 1024.0 * 2.0,
            disk_write_bps: 1024.0 * 1024.0 * 5.0,
            disk_read_history,
            disk_write_history,
            gpus: Vec::new(),
            previous_gpu_rc6: None,
            gpu_usage_history: VecDeque::new(),
            gpu_temp_history: VecDeque::new(),
            temp_c: None,
            temp_history: VecDeque::new(),
            process_count: 50,
            top_cpu_processes: Vec::new(),
            top_mem_processes: Vec::new(),
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: HashMap::new(),
            process_sort: ProcessSort::CpuDesc,
            process_history: HashMap::new(),
            process_selected: 0,
            is_tree_view: false,
            health_score: 98,
            alerts: Vec::new(),
            successful_reads: 10,
            failed_reads: 0,
            degraded_sources: Vec::new(),
            last_sample_at: Instant::now(),
            counter: 1,
            show_help: false,
            refresh_index: 1,
            active_tab: ViewTab::Storage,
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
    fn storage_renders_at_various_sizes() {
        let app = make_test_app();
        for (w, h) in [(60, 15), (80, 24), (100, 30), (120, 40), (160, 48)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    storage_tab(f, f.size(), &app);
                })
                .unwrap();
        }
    }

    #[test]
    fn storage_renders_with_smart_health() {
        let mut app = make_test_app();
        app.storage_health = vec![StorageHealth {
            device: "nvme0n1".into(),
            model: "Samsung 980 PRO".into(),
            kind: "NVMe".into(),
            temp_c: Some(42.0),
            critical_warning: Some(0),
            media_errors: Some(0),
            risk: Severity::Ok,
            note: "Healthy (100% life)".into(),
        }];

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                storage_tab(f, f.size(), &app);
            })
            .unwrap();
    }
}
