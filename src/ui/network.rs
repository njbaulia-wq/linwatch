use crate::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Row, Sparkline, Table},
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
            Constraint::Length(7),
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
        Paragraph::new(summary).block(panel_block("\u{2194} Network Summary")),
        chunks[0],
    );

    let graph_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_network_sparkline(
        frame,
        graph_chunks[0],
        "\u{2193} Download trend",
        &app.net_down_history,
        app.net_down_bps,
        t.accent_teal,
    );
    render_network_sparkline(
        frame,
        graph_chunks[1],
        "\u{2191} Upload trend",
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
            Paragraph::new("No interface activity")
                .block(panel_block("\u{2194} Interface Details")),
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
            .block(panel_block("\u{2194} Interface Details"))
            .column_spacing(1),
            bottom_chunks[0],
        );
    }

    let port_rows = app
        .open_ports
        .iter()
        .map(|p| {
            let service_color = if p.service_name != "Other" {
                t.accent_teal
            } else {
                t.overlay1
            };
            Row::new(vec![
                Cell::from(p.proto.as_str()),
                Cell::from(p.ip.as_str()),
                Cell::from(p.port.to_string()),
                Cell::from(Span::styled(
                    p.service_name.as_str(),
                    Style::default().fg(service_color),
                )),
                Cell::from(p.state.as_str()),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    // Narrow: drop Service/State columns so Proto/IP/Port always fit.
    let compact_ports = area.width < 100;

    if port_rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No active open ports detected").block(panel_block("🔓 Open Ports")),
            bottom_chunks[1],
        );
    } else if compact_ports {
        let compact_rows = app
            .open_ports
            .iter()
            .map(|p| {
                Row::new(vec![
                    Cell::from(p.proto.as_str()),
                    Cell::from(p.ip.as_str()),
                    Cell::from(p.port.to_string()),
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
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Proto")),
                Cell::from(header_col("IP Address")),
                Cell::from(header_col("Port")),
            ]))
            .block(panel_block("🔓 Open Ports"))
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
                    Constraint::Length(15),
                    Constraint::Length(8),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Proto")),
                Cell::from(header_col("IP Address")),
                Cell::from(header_col("Port")),
                Cell::from(header_col("Service")),
                Cell::from(header_col("State")),
            ]))
            .block(panel_block("🔓 Open Ports"))
            .column_spacing(1),
            bottom_chunks[1],
        );
    }
}

fn render_network_sparkline(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    history: &std::collections::VecDeque<(f64, f64)>,
    current: f64,
    color: ratatui::style::Color,
) {
    let t = theme::get();
    let data = history
        .iter()
        .rev()
        .take(50)
        .rev()
        .map(|(_, value)| value.max(0.0).round() as u64)
        .collect::<Vec<_>>();
    let max = data.iter().copied().max().unwrap_or(1).max(1);
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

    let block_title = format!(
        "{}  now {}/s  avg {}/s",
        title,
        format_bytes(current),
        format_bytes(avg)
    );

    let sparkline = if data.len() >= 2 {
        Sparkline::default()
            .data(&data)
            .max(max)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .block(panel_block(block_title))
    } else {
        Sparkline::default()
            .data(&[0])
            .max(1)
            .style(Style::default().fg(t.overlay1))
            .block(panel_block(block_title))
    };
    frame.render_widget(sparkline, area);
}

use ratatui::widgets::Cell;
