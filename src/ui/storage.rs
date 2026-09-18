use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    widgets::{LineGauge, Paragraph, Row, Table},
    Frame,
};

use super::common::*;
use super::theme;

pub fn storage_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let has_disk_io = !app.disk_io.is_empty();
    let io_height = if has_disk_io { 6 } else { 0 };
    let health_height = if app.storage_health.is_empty() { 0 } else { 6 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(health_height),
            Constraint::Length(io_height),
            Constraint::Min(6),
        ])
        .split(area);

    let gauge_area = chunks[0].inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });
    render_line_gauge(
        frame,
        gauge_area,
        "Root Filesystem (/) ",
        app.disk.pct as f64,
        t.accent_green,
    );

    if has_disk_io {
        let d_io_rows = app
            .disk_io
            .iter()
            .take(5)
            .map(|dio| {
                Row::new(vec![
                    Cell::from(dio.device.as_str()),
                    Cell::from(format!("{}/s", format_bytes(dio.read_bps))),
                    Cell::from(format!("{}/s", format_bytes(dio.write_bps))),
                ])
                .style(Style::default().fg(t.text))
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            Table::new(
                d_io_rows,
                [
                    Constraint::Length(10),
                    Constraint::Length(14),
                    Constraint::Min(10),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Device")),
                Cell::from(header_col("Read")),
                Cell::from(header_col("Write")),
            ]))
            .block(panel_block(" Disk I/O Throughput "))
            .column_spacing(1),
            chunks[2],
        );
    }

    if !app.storage_health.is_empty() {
        // Narrow: drop Warn/Errors numeric columns, keep Device/Kind/Model/Temp/Risk.
        let compact_health = area.width < 100;
        let health_rows = app
            .storage_health
            .iter()
            .take(5)
            .map(|drive| {
                let color = severity_color(drive.risk);
                let temp = drive
                    .temp_c
                    .map(|value| format!("{value:.0}\u{b0}C"))
                    .unwrap_or_else(|| String::from("N/A"));
                let mut cells = vec![
                    Cell::from(Span::styled(
                        format!("{} {}", drive.risk.symbol(), drive.device),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(drive.kind.as_str()),
                    Cell::from(truncate(&drive.model, 18)),
                    Cell::from(temp),
                ];
                if !compact_health {
                    cells.push(Cell::from(
                        drive
                            .critical_warning
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| String::from("-")),
                    ));
                    cells.push(Cell::from(
                        drive
                            .media_errors
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| String::from("-")),
                    ));
                }
                cells.push(Cell::from(truncate_words(
                    &drive.note,
                    if compact_health { 20 } else { 30 },
                )));
                Row::new(cells).style(Style::default().fg(t.text))
            })
            .collect::<Vec<_>>();

        let mut health_widths = vec![
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(19),
            Constraint::Length(8),
        ];
        if !compact_health {
            health_widths.push(Constraint::Length(7));
            health_widths.push(Constraint::Length(8));
        }
        health_widths.push(Constraint::Min(14));
        let mut health_header = vec![
            Cell::from(header_col("Device")),
            Cell::from(header_col("Kind")),
            Cell::from(header_col("Model")),
            Cell::from(header_col("Temp")),
        ];
        if !compact_health {
            health_header.push(Cell::from(header_col("Warn")));
            health_header.push(Cell::from(header_col("Errors")));
        }
        health_header.push(Cell::from(header_col("Risk")));

        frame.render_widget(
            Table::new(health_rows, health_widths)
                .header(Row::new(health_header))
                .block(panel_block(" Physical Drive Health "))
                .column_spacing(1),
            chunks[1],
        );
    }

    let mount_rows = app
        .mounts
        .iter()
        .map(|m| {
            let sev = Severity::from_usage(m.pct as f64);
            let color = severity_color(sev);
            Row::new(vec![
                Cell::from(truncate(&m.mount_point, 24)),
                Cell::from(Span::styled(
                    format!("{} {:>3}%", sev.symbol(), m.pct),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Cell::from(format!("{:.1}/{:.1} GB", m.used_gb, m.total_gb)),
            ])
        })
        .collect::<Vec<_>>();

    if mount_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No mount data").block(panel_block(" Filesystem Mount Points ")),
            chunks[3],
        );
    } else {
        frame.render_widget(
            Table::new(
                mount_rows,
                [
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Min(14),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Mount")),
                Cell::from(header_col("Usage")),
                Cell::from(header_col("Size")),
            ]))
            .block(panel_block(" Filesystem Mount Points "))
            .column_spacing(1),
            chunks[3],
        );
    }
}

fn render_line_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    color: ratatui::style::Color,
) {
    let sev = Severity::from_usage(value);
    let gauge = LineGauge::default()
        .block(ratatui::widgets::Block::default().title(format!(
            "{} {}{:>5.1}%",
            sev.symbol(),
            label,
            value
        )))
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .line_set(ratatui::symbols::line::THICK)
        .ratio((value / 100.0).clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

use ratatui::{text::Span, widgets::Cell};
