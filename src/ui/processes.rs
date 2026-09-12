use crate::state::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Row, Table, TableState},
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

    let summary_line = Line::from(vec![
        styled(format!("\u{2630} {} processes", app.process_count), t.text),
        styled("  Sort: ", t.overlay1),
        styled(app.process_sort.label(), t.accent_blue).add_modifier(Modifier::BOLD),
        styled(
            "  [S] sort  [/] search  [K] terminate  [\u{2191}/\u{2193}] select",
            t.overlay1,
        ),
    ]);
    frame.render_widget(
        Paragraph::new(summary_line).block(panel_block("\u{2630} Process Overview")),
        header_chunks[0],
    );

    if show_search {
        let cursor = if app.is_search_mode { "█" } else { "" };
        let search_line = Line::from(vec![
            Span::styled(
                "  🔎 Search: ",
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

    let sorted = app.filtered_processes();

    // Progressive column disclosure: hide low-priority columns first so the
    // table never overflows narrow terminals.
    let show_spark = area.width >= 110;
    let show_why = area.width >= 100;
    let show_state = area.width >= 88;
    let show_thr = area.width >= 80;

    let p_rows = sorted
        .iter()
        .map(|p| {
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
            let name_width = if area.width < 100 { 12 } else { 18 };

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
                format!(
                    "{}  {}{}",
                    sev.symbol(),
                    name_prefix,
                    truncate(&p.name, name_width)
                ),
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
            Paragraph::new("No process data").block(panel_block("☰ Process List")),
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
                .block(panel_block("☰ Process List"))
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
        let popup_w = chunks[1].width.saturating_sub(4).clamp(24, 56);
        let popup_h = 5.min(chunks[1].height.max(3));
        let popup = Rect {
            x: chunks[1].x + (chunks[1].width.saturating_sub(popup_w)) / 2,
            y: chunks[1].y + (chunks[1].height.saturating_sub(popup_h)) / 2,
            width: popup_w,
            height: popup_h,
        };
        let confirm_text = vec![
            Line::from(Span::styled(
                format!(" Send SIGTERM to PID {pid} ({name})?"),
                Style::default()
                    .fg(t.accent_red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[K] confirm  ", Style::default().fg(t.accent_red)),
                Span::styled("[Esc] cancel", Style::default().fg(t.overlay1)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(confirm_text)
                .alignment(Alignment::Center)
                .block(panel_block_severity(
                    "⚠ Confirm Terminate",
                    crate::types::Severity::Critical,
                )),
            popup,
        );
    }
}

use ratatui::widgets::Cell;
