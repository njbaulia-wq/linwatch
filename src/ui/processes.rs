use crate::state::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Row, Table, TableState},
    Frame,
};

use super::common::*;
use super::theme;

pub fn processes_tab(frame: &mut Frame, area: Rect, app: &AppState, table_state: &mut TableState) {
    let t = theme::get();

    let show_search = app.is_search_mode || !app.process_search.is_empty();
    let show_message = app.process_action_message.is_some();
    let header_height = 3 + if show_search { 2 } else { 0 } + if show_message { 2 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(6)])
        .split(area);

    let mut header_constraints = vec![Constraint::Length(3)];
    if show_search {
        header_constraints.push(Constraint::Length(2));
    }
    if show_message {
        header_constraints.push(Constraint::Length(2));
    }
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(header_constraints)
        .split(chunks[0])
        .to_vec();

    let mode_label = if app.is_tree_view { "Tree" } else { "Flat" };
    let summary_line = Line::from(vec![
        styled(format!("{} processes", app.process_count), t.text),
        styled("  View: ", t.overlay1),
        styled(mode_label, t.accent_teal).add_modifier(Modifier::BOLD),
        styled("  Sort: ", t.overlay1),
        styled(app.process_sort.label(), t.accent_blue).add_modifier(Modifier::BOLD),
        styled(
            "  [Enter/I] inspect  [T] tree  [S] sort  [/] search  [K] terminate  [\u{2191}/\u{2193}] select",
            t.overlay1,
        ),
    ]);
    frame.render_widget(
        Paragraph::new(summary_line).block(panel_block("Process Overview")),
        header_chunks[0],
    );

    if show_search {
        let cursor = if app.is_search_mode { "█" } else { "" };
        let search_line = Line::from(vec![
            Span::styled(
                "  Search: ",
                Style::default()
                    .fg(t.accent_blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}{}", app.process_search, cursor),
                Style::default().fg(t.text),
            ),
            Span::styled(
                "  (Enter closes, Esc clears)",
                Style::default().fg(t.overlay1),
            ),
        ]);
        frame.render_widget(Paragraph::new(search_line), header_chunks[1]);
    }

    if let Some(message) = &app.process_action_message {
        let area = header_chunks[if show_search { 2 } else { 1 }];
        let color = if message.starts_with("Could not") {
            t.accent_red
        } else {
            t.accent_yellow
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(t.overlay1)),
                Span::styled(
                    message.clone(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])),
            area,
        );
    }

    let sorted = app.filtered_process_tree();

    // Progressive column disclosure: hide low-priority columns first so the
    // table never overflows narrow terminals.
    let show_spark = area.width >= 120;
    let show_why = area.width >= 105;
    let show_io = area.width >= 95;
    let show_thr = area.width >= 90;
    let show_state = area.width >= 75;

    let fixed_width = 8
        + 8
        + 9
        + if show_io { 15 } else { 0 }
        + if show_thr { 5 } else { 0 }
        + if show_state { 11 } else { 0 }
        + if show_why { 17 } else { 0 }
        + if show_spark { 12 } else { 0 };
    let cols_count = 4
        + usize::from(show_io)
        + usize::from(show_thr)
        + usize::from(show_state)
        + usize::from(show_why)
        + usize::from(show_spark);
    let spacing = cols_count.saturating_sub(1) as u16;
    let available_name_w = (area.width.saturating_sub(fixed_width + spacing) as usize).max(18);

    let p_rows = sorted
        .iter()
        .map(|(p, prefix)| {
            let is_risk = p.is_high_risk;
            let is_dev = p.is_dev;
            let pid_color = if is_risk { t.accent_red } else { t.text };
            let cpu_color = if is_risk { t.accent_red } else { t.accent_teal };
            let spark = app
                .process_history
                .get(&p.pid)
                .map(|h| {
                    h.iter()
                        .rev()
                        .take(10)
                        .rev()
                        .map(|&v| sparkline_chars(v))
                        .collect::<String>()
                })
                .unwrap_or_default();

            let sev = if is_risk {
                crate::types::Severity::Critical
            } else {
                crate::types::Severity::Ok
            };

            let name_color = if is_risk {
                t.accent_red
            } else if is_dev {
                t.accent_teal
            } else {
                t.text
            };
            let name_prefix = if is_dev { "[Dev] " } else { "" };
            let display_name = truncate(
                &p.name,
                available_name_w.saturating_sub(prefix.len()).max(8),
            );

            let mut cells = vec![
                Cell::from(Span::styled(
                    format!("{:<7}", p.pid),
                    Style::default().fg(pid_color),
                )),
                Cell::from(Span::styled(
                    format!("{:>5.1}% ", p.cpu_pct),
                    Style::default().fg(cpu_color).add_modifier(if is_risk {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                )),
                Cell::from(Span::styled(
                    format!("{:>7.1}M", p.mem_mb),
                    Style::default().fg(t.overlay1),
                )),
            ];
            if show_io {
                let total_io = p.io_read_bps + p.io_write_bps;
                let io_style = if total_io > 50.0 * 1024.0 * 1024.0 {
                    Style::default()
                        .fg(t.accent_red)
                        .add_modifier(Modifier::BOLD)
                } else if total_io > 5.0 * 1024.0 * 1024.0 {
                    Style::default()
                        .fg(t.accent_yellow)
                        .add_modifier(Modifier::BOLD)
                } else if total_io > 10.0 * 1024.0 {
                    Style::default().fg(t.accent_blue)
                } else if total_io > 0.0 {
                    Style::default().fg(t.subtext0)
                } else {
                    Style::default().fg(t.overlay0)
                };

                let io_str = if total_io <= f64::EPSILON {
                    String::from("       -       ")
                } else {
                    format!(
                        "R:{} W:{}",
                        format_compact_rate(p.io_read_bps),
                        format_compact_rate(p.io_write_bps)
                    )
                };
                cells.push(Cell::from(Span::styled(io_str, io_style)));
            }
            if show_thr {
                cells.push(Cell::from(Span::styled(
                    format!("{:>4}", p.threads),
                    Style::default().fg(t.overlay1),
                )));
            }
            if show_state {
                cells.push(Cell::from(Span::styled(
                    truncate(&p.state, 10),
                    Style::default().fg(t.overlay1),
                )));
            }
            cells.push(Cell::from(Span::styled(
                format!("{} {}{}{}", sev.symbol(), name_prefix, prefix, display_name),
                Style::default().fg(name_color).add_modifier(if is_dev {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            )));
            if show_why {
                cells.push(Cell::from(Span::styled(
                    truncate(&p.reason, 16),
                    Style::default().fg(if p.reason == "Normal" {
                        t.overlay1
                    } else if is_dev {
                        t.accent_teal
                    } else {
                        severity_color(sev)
                    }),
                )));
            }
            if show_spark {
                cells.push(Cell::from(Span::styled(
                    spark,
                    Style::default().fg(t.accent_teal),
                )));
            }

            Row::new(cells).style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    if p_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No process data").block(panel_block("Process List")),
            chunks[1],
        );
    } else {
        *table_state.offset_mut() = table_state.offset().min(p_rows.len().saturating_sub(1));
        let highlight_style = Style::default()
            .fg(Color::Rgb(24, 24, 37))
            .bg(t.accent_blue);

        let mut widths = vec![
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(9),
        ];
        if show_io {
            widths.push(Constraint::Length(15));
        }
        if show_thr {
            widths.push(Constraint::Length(5));
        }
        if show_state {
            widths.push(Constraint::Length(11));
        }
        widths.push(Constraint::Min(16));
        if show_why {
            widths.push(Constraint::Length(17));
        }
        if show_spark {
            widths.push(Constraint::Length(12));
        }
        let mut header_cells = vec![
            Cell::from(header_col("PID")),
            Cell::from(header_col("CPU%")),
            Cell::from(header_col("MEM")),
        ];
        if show_io {
            header_cells.push(Cell::from(header_col("DISK R/W")));
        }
        if show_thr {
            header_cells.push(Cell::from(header_col("THR")));
        }
        if show_state {
            header_cells.push(Cell::from(header_col("State")));
        }
        header_cells.push(Cell::from(header_col("Command")));
        if show_why {
            header_cells.push(Cell::from(header_col("Why")));
        }
        if show_spark {
            header_cells.push(Cell::from(header_col("Spark")));
        }

        frame.render_stateful_widget(
            Table::new(p_rows, widths)
                .header(Row::new(header_cells))
                .block(panel_block("Process List"))
                .column_spacing(1)
                .highlight_style(highlight_style)
                .highlight_symbol("  \u{25b6} "),
            chunks[1],
            table_state,
        );
    }

    // Confirm dialog floats over the table so selection context stays visible.
    if let Some(pid) = app.confirm_kill_pid {
        let name = app.confirm_kill_name.as_deref().unwrap_or("unknown");
        let popup_w = chunks[1].width.saturating_sub(4).clamp(28, 62);
        let popup_h = 5.min(chunks[1].height.max(3));
        let popup = Rect {
            x: chunks[1].x + (chunks[1].width.saturating_sub(popup_w)) / 2,
            y: chunks[1].y + (chunks[1].height.saturating_sub(popup_h)) / 2,
            width: popup_w,
            height: popup_h,
        };
        let confirm_text = vec![
            Line::from(Span::styled(
                format!(" Terminate PID {pid} ({name})?"),
                Style::default()
                    .fg(t.accent_red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[K/Enter] SIGTERM  ", Style::default().fg(t.accent_yellow)),
                Span::styled("[9] SIGKILL  ", Style::default().fg(t.accent_red)),
                Span::styled("[Esc] cancel", Style::default().fg(t.overlay1)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(confirm_text)
                .alignment(Alignment::Center)
                .block(panel_block_severity(
                    "Confirm Terminate",
                    crate::types::Severity::Critical,
                )),
            popup,
        );
    }

    // Process Inspector Modal
    if let Some(detail) = &app.inspect_process_detail {
        render_process_inspector(frame, area, detail, app.process_action_message.as_deref());
    }
}

fn render_process_inspector(
    frame: &mut Frame,
    area: Rect,
    detail: &crate::types::ProcessDetail,
    action_message: Option<&str>,
) {
    let t = theme::get();
    let popup_w = area.width.saturating_sub(4).clamp(52, 80);
    let popup_h = area.height.saturating_sub(2).clamp(13, 16);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };

    frame.render_widget(Clear, popup);

    let title = format!(
        " Process Deep-Dive Inspector: {} (PID {}) ",
        truncate(&detail.name, 28),
        detail.pid
    );
    let block = panel_block(title);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let max_w = inner.width as usize;
    let mut lines = Vec::new();

    // 1. Core vitals: PID, PPID, State, Threads, UID:GID
    lines.push(Line::from(vec![
        dim(" PID: "),
        styled(format!("{}", detail.pid), t.accent_blue).add_modifier(Modifier::BOLD),
        dim("  PPID: "),
        styled(format!("{}", detail.ppid), t.overlay1),
        dim("  State: "),
        styled(&detail.state, t.accent_teal).add_modifier(Modifier::BOLD),
        dim("  Threads: "),
        styled(format!("{}", detail.threads), t.accent_purple),
        dim("  User: "),
        styled(format!("{}:{}", detail.uid, detail.gid), t.subtext0),
    ]));

    // 2. Execution Context
    lines.push(Line::from(styled("─".repeat(max_w.min(76)), t.surface1)));
    let cmd_w = max_w.saturating_sub(11).max(10);
    lines.push(Line::from(vec![
        dim(" Command: "),
        styled(truncate(&detail.cmdline, cmd_w), t.text),
    ]));
    let cwd_w = max_w.saturating_sub(11).max(10);
    lines.push(Line::from(vec![
        dim(" WorkDir: "),
        styled(truncate(&detail.cwd, cwd_w), t.subtext0),
    ]));

    // 3. Memory Footprint
    lines.push(Line::from(styled("─".repeat(max_w.min(76)), t.surface1)));
    lines.push(Line::from(vec![
        dim(" VmPeak: "),
        styled(format!("{:>6.1} MB", detail.vm_peak_mb), t.text),
        dim("  │ VmSize: "),
        styled(format!("{:>6.1} MB", detail.vm_size_mb), t.text),
        dim("  │ VmRSS: "),
        styled(format!("{:>6.1} MB", detail.vm_rss_mb), t.accent_yellow)
            .add_modifier(Modifier::BOLD),
    ]));
    lines.push(Line::from(vec![
        dim(" RssAnon: "),
        styled(format!("{:>5.1} MB", detail.rss_anon_mb), t.subtext0),
        dim("  │ RssFile: "),
        styled(format!("{:>5.1} MB", detail.rss_file_mb), t.subtext0),
        dim("  │ RssShmem: "),
        styled(format!("{:>5.1} MB", detail.rss_shmem_mb), t.subtext0),
    ]));

    // 4. I/O & Descriptors
    lines.push(Line::from(styled("─".repeat(max_w.min(76)), t.surface1)));
    let io_rate_info = if detail.io_read_bps + detail.io_write_bps > 0.0 {
        format!(
            " ({} rd, {} wr)",
            format_rate(detail.io_read_bps),
            format_rate(detail.io_write_bps)
        )
    } else {
        String::new()
    };
    lines.push(Line::from(vec![
        dim(" Open FDs: "),
        styled(format!("{}", detail.open_fds), t.accent_teal),
        dim("  │ Read: "),
        styled(format_bytes(detail.read_bytes as f64), t.text),
        dim("  │ Write: "),
        styled(format_bytes(detail.write_bytes as f64), t.text),
        styled(io_rate_info, t.accent_blue),
    ]));

    // 5. Action message or status
    if let Some(msg) = action_message {
        lines.push(Line::from(vec![
            dim(" Action: "),
            styled(truncate(msg, max_w.saturating_sub(10)), t.accent_yellow)
                .add_modifier(Modifier::BOLD),
        ]));
    } else {
        lines.push(Line::from(""));
    }

    // 6. Action Hotkeys
    lines.push(Line::from(styled("─".repeat(max_w.min(76)), t.surface1)));
    lines.push(Line::from(vec![
        styled(" [T] ", t.accent_yellow).add_modifier(Modifier::BOLD),
        dim("SIGTERM  "),
        styled("[9] ", t.accent_red).add_modifier(Modifier::BOLD),
        dim("SIGKILL  "),
        styled("[P] ", t.accent_blue).add_modifier(Modifier::BOLD),
        dim("SIGSTOP  "),
        styled("[C] ", t.accent_teal).add_modifier(Modifier::BOLD),
        dim("SIGCONT  "),
        styled("[Esc/Enter] ", t.overlay1),
        dim("Close"),
    ]));

    let visible: Vec<Line> = lines.into_iter().take(inner.height as usize).collect();
    frame.render_widget(Paragraph::new(visible), inner);
}

use ratatui::widgets::Cell;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn processes_tab_renders_without_crashing() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new(AppConfig::default());
        app.top_cpu_processes = vec![ProcessInfo {
            pid: 1234,
            ppid: 1,
            name: "long-running-worker-process-daemon".to_string(),
            cpu_pct: 15.5,
            mem_mb: 256.0,
            threads: 4,
            state: "S".to_string(),
            reason: "Normal".to_string(),
            is_high_risk: false,
            is_dev: false,
            io_read_bps: 1024.0 * 1024.0 * 2.5,
            io_write_bps: 1024.0 * 512.0,
        }];

        let mut table_state = TableState::default();
        terminal
            .draw(|frame| {
                processes_tab(frame, frame.size(), &app, &mut table_state);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // Verify buffer has content and shows part of the process name
        let content_str: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("long-running-worker"));
        assert!(content_str.contains("DISK R/W"));
    }

    #[test]
    fn processes_tab_renders_tree_view() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new(AppConfig::default());
        app.is_tree_view = true;
        app.top_cpu_processes = vec![
            ProcessInfo {
                pid: 100,
                ppid: 1,
                name: "system-master".to_string(),
                cpu_pct: 10.0,
                mem_mb: 100.0,
                threads: 2,
                state: "S".to_string(),
                reason: "Normal".to_string(),
                is_high_risk: false,
                is_dev: false,
                io_read_bps: 0.0,
                io_write_bps: 0.0,
            },
            ProcessInfo {
                pid: 101,
                ppid: 100,
                name: "worker-child".to_string(),
                cpu_pct: 5.0,
                mem_mb: 50.0,
                threads: 1,
                state: "S".to_string(),
                reason: "Normal".to_string(),
                is_high_risk: false,
                is_dev: false,
                io_read_bps: 0.0,
                io_write_bps: 0.0,
            },
        ];

        let mut table_state = TableState::default();
        terminal
            .draw(|frame| {
                processes_tab(frame, frame.size(), &app, &mut table_state);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content_str: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("system-master"));
        assert!(content_str.contains("worker-child"));
    }

    #[test]
    fn processes_tab_renders_inspector_modal() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new(AppConfig::default());
        app.inspect_process_pid = Some(4242);
        app.inspect_process_detail = Some(ProcessDetail {
            pid: 4242,
            ppid: 1,
            name: "heavy-database-engine".into(),
            cmdline: "/usr/bin/heavy-database-engine --config /etc/db.conf".into(),
            state: "S".into(),
            threads: 16,
            uid: 1000,
            gid: 1000,
            vm_peak_mb: 4096.0,
            vm_size_mb: 3500.0,
            vm_rss_mb: 2048.0,
            rss_anon_mb: 1500.0,
            rss_file_mb: 500.0,
            rss_shmem_mb: 48.0,
            read_bytes: 1024 * 1024 * 250,
            write_bytes: 1024 * 1024 * 500,
            cancelled_write_bytes: 0,
            open_fds: 64,
            cwd: "/var/lib/db".into(),
            io_read_bps: 1024.0 * 1024.0 * 5.0,
            io_write_bps: 1024.0 * 1024.0 * 2.0,
        });

        let mut table_state = TableState::default();
        terminal
            .draw(|frame| {
                processes_tab(frame, frame.size(), &app, &mut table_state);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content_str: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Deep-Dive Inspector"));
        assert!(content_str.contains("heavy-database-engine"));
        assert!(content_str.contains("4242"));
        assert!(content_str.contains("SIGTERM"));
        assert!(content_str.contains("SIGSTOP"));
        assert!(content_str.contains("rd"));
        assert!(content_str.contains("wr"));
    }

    #[test]
    fn processes_tab_responsive_io_column() {
        let mut app = AppState::new(AppConfig::default());
        app.top_cpu_processes = vec![ProcessInfo {
            pid: 999,
            ppid: 1,
            name: "test-proc".to_string(),
            cpu_pct: 1.0,
            mem_mb: 10.0,
            threads: 1,
            state: "S".to_string(),
            reason: "Normal".to_string(),
            is_high_risk: false,
            is_dev: false,
            io_read_bps: 1024.0 * 100.0,
            io_write_bps: 1024.0 * 50.0,
        }];

        // At width 100 (>= 95): DISK R/W column should be visible
        let backend_wide = TestBackend::new(100, 30);
        let mut term_wide = Terminal::new(backend_wide).unwrap();
        let mut table_state = TableState::default();
        term_wide
            .draw(|frame| {
                processes_tab(frame, frame.size(), &app, &mut table_state);
            })
            .unwrap();
        let buf_wide = term_wide.backend().buffer();
        let wide_str: String = buf_wide.content.iter().map(|c| c.symbol()).collect();
        assert!(wide_str.contains("DISK R/W"));

        // At width 80 (< 95): DISK R/W column should be hidden
        let backend_narrow = TestBackend::new(80, 30);
        let mut term_narrow = Terminal::new(backend_narrow).unwrap();
        let mut table_state = TableState::default();
        term_narrow
            .draw(|frame| {
                processes_tab(frame, frame.size(), &app, &mut table_state);
            })
            .unwrap();
        let buf_narrow = term_narrow.backend().buffer();
        let narrow_str: String = buf_narrow.content.iter().map(|c| c.symbol()).collect();
        assert!(!narrow_str.contains("DISK R/W"));
    }
}
