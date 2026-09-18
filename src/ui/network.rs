use crate::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Row, Table},
    Frame,
};

use super::common::*;
use super::theme;

pub fn network_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(6),
        ])
        .split(area);

    let summary = Line::from(vec![
        Span::styled("Download: ", Style::default().fg(t.accent_teal)),
        Span::styled(
            format!("{}/s", format_bytes(app.net_down_bps)),
            Style::default()
                .fg(t.accent_teal)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(t.overlay1)),
        Span::styled("Upload: ", Style::default().fg(t.accent_orange)),
        Span::styled(
            format!("{}/s", format_bytes(app.net_up_bps)),
            Style::default()
                .fg(t.accent_orange)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(summary).block(panel_block(" Network Throughput Summary ")),
        chunks[0],
    );

    let graph_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_network_braille_chart(
        frame,
        graph_chunks[0],
        "↓ Download Activity",
        &app.net_down_history,
        app.net_down_bps,
        t.accent_teal,
    );
    render_network_braille_chart(
        frame,
        graph_chunks[1],
        "↑ Upload Activity",
        &app.net_up_history,
        app.net_up_bps,
        t.accent_orange,
    );

    let bottom_chunks = if area.width < 90 {
        // Narrow: stack interface and port tables vertically.
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2])
            .to_vec()
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2])
            .to_vec()
    };

    let iface_rows = app
        .interfaces
        .iter()
        .map(|iface| {
            Row::new(vec![
                Cell::from(iface.name.as_str()),
                Cell::from(format!("{}/s", format_bytes(iface.down_bps))),
                Cell::from(format!("{}/s", format_bytes(iface.up_bps))),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    if iface_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No interface activity").block(panel_block(" Network Interfaces ")),
            bottom_chunks[0],
        );
    } else {
        frame.render_widget(
            Table::new(
                iface_rows,
                [
                    Constraint::Length(12),
                    Constraint::Min(14),
                    Constraint::Min(12),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Interface")),
                Cell::from(header_col("Download")),
                Cell::from(header_col("Upload")),
            ]))
            .block(panel_block(" Network Interfaces "))
            .column_spacing(1),
            bottom_chunks[0],
        );
    }

    let port_rows = app
        .open_ports
        .iter()
        .map(|p| {
            let (proc_service, has_proc) = match (&p.process_name, p.pid) {
                (Some(name), Some(pid)) => (format!("{name} ({pid})"), true),
                _ if p.service_name != "Other" => (p.service_name.clone(), true),
                _ => (String::from("-"), false),
            };
            let proc_color = if has_proc { t.accent_teal } else { t.overlay1 };
            Row::new(vec![
                Cell::from(p.proto.as_str()),
                Cell::from(p.ip.as_str()),
                Cell::from(p.port.to_string()),
                Cell::from(Span::styled(proc_service, Style::default().fg(proc_color))),
                Cell::from(p.state.as_str()),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    // Narrow: drop State column and compact Process so Proto/IP/Port/Process fit.
    let compact_ports = area.width < 100;

    if port_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No active open ports detected").block(panel_block(" Listening Ports ")),
            bottom_chunks[1],
        );
    } else if compact_ports {
        let compact_rows = app
            .open_ports
            .iter()
            .map(|p| {
                let proc_service = match (&p.process_name, p.pid) {
                    (Some(name), Some(pid)) => format!("{name} ({pid})"),
                    _ if p.service_name != "Other" => p.service_name.clone(),
                    _ => String::from("-"),
                };
                Row::new(vec![
                    Cell::from(p.proto.as_str()),
                    Cell::from(p.ip.as_str()),
                    Cell::from(p.port.to_string()),
                    Cell::from(proc_service),
                ])
                .style(Style::default().fg(t.text))
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Table::new(
                compact_rows,
                [
                    Constraint::Length(6),
                    Constraint::Min(13),
                    Constraint::Length(6),
                    Constraint::Length(18),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Proto")),
                Cell::from(header_col("IP Address")),
                Cell::from(header_col("Port")),
                Cell::from(header_col("Process / Service")),
            ]))
            .block(panel_block(" Listening Ports "))
            .column_spacing(1),
            bottom_chunks[1],
        );
    } else {
        frame.render_widget(
            Table::new(
                port_rows,
                [
                    Constraint::Length(6),
                    Constraint::Min(13),
                    Constraint::Length(6),
                    Constraint::Length(22),
                    Constraint::Length(8),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Proto")),
                Cell::from(header_col("IP Address")),
                Cell::from(header_col("Port")),
                Cell::from(header_col("Process / Service")),
                Cell::from(header_col("State")),
            ]))
            .block(panel_block(" Listening Ports "))
            .column_spacing(1),
            bottom_chunks[1],
        );
    }
}

fn render_network_braille_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    history: &std::collections::VecDeque<(f64, f64)>,
    current: f64,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    if history.is_empty() {
        frame.render_widget(
            Paragraph::new("Collecting throughput data...").block(panel_block(title)),
            area,
        );
        return;
    }

    let points: Vec<(f64, f64)> = history.iter().copied().collect();
    let x_start = history.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = history
        .back()
        .map(|p| p.0)
        .unwrap_or(1.0)
        .max(x_start + 1.0);

    let peak = history.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);
    let avg = if history.is_empty() {
        0.0
    } else {
        history
            .iter()
            .rev()
            .take(10)
            .map(|(_, value)| *value)
            .sum::<f64>()
            / history.len().min(10) as f64
    };

    let y_max = (peak * 1.15).max(1024.0);
    let half_str = format!("{}/s", format_bytes(y_max * 0.5));
    let max_str = format!("{}/s", format_bytes(y_max));

    let block_title = format!(
        "{title} · now {}/s · peak {}/s · avg {}/s",
        format_bytes(current),
        format_bytes(peak),
        format_bytes(avg)
    );

    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .data(&points);

    let chart = Chart::new(vec![dataset])
        .block(panel_block(block_title))
        .x_axis(
            Axis::default()
                .bounds([x_start, x_end])
                .labels(vec![
                    Span::styled("-120s", Style::default().fg(t.overlay1)),
                    Span::styled("now", Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(vec![
                    Span::styled("0", Style::default().fg(t.overlay1)),
                    Span::styled(half_str, Style::default().fg(t.overlay1)),
                    Span::styled(max_str, Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}

use ratatui::widgets::Cell;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::VecDeque;

    #[test]
    fn network_tab_renders_with_process_mapped_ports() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new(AppConfig::default());
        app.open_ports = vec![
            OpenPort {
                port: 22,
                ip: "0.0.0.0".to_string(),
                proto: "TCP".to_string(),
                state: "LISTEN".to_string(),
                service_name: "SSH".to_string(),
                pid: Some(512),
                process_name: Some("sshd".to_string()),
            },
            OpenPort {
                port: 8080,
                ip: "127.0.0.1".to_string(),
                proto: "TCP".to_string(),
                state: "LISTEN".to_string(),
                service_name: "HTTP Alt/Java".to_string(),
                pid: None,
                process_name: None,
            },
        ];

        terminal
            .draw(|frame| {
                network_tab(frame, frame.size(), &app);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content_str: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("sshd (512)"));
        assert!(content_str.contains("HTTP Alt/Java"));
    }

    #[test]
    fn network_tab_renders_braille_chart_history() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new(AppConfig::default());
        let mut history = VecDeque::new();
        for i in 0..60 {
            history.push_back((i as f64, (i * 1024) as f64));
        }
        app.net_down_history = history.clone();
        app.net_up_history = history;
        app.net_down_bps = 50_000.0;
        app.net_up_bps = 25_000.0;

        terminal
            .draw(|frame| {
                network_tab(frame, frame.size(), &app);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content_str: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(content_str.contains("Download Activity"));
        assert!(content_str.contains("Upload Activity"));
    }
}
