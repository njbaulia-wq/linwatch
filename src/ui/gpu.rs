use std::collections::VecDeque;

use crate::state::AppState;
use crate::types::Severity;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, GraphType, LineGauge, Paragraph, Row, Table},
    Frame,
};

use super::common::*;
use super::theme;

pub fn gpu_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    if app.gpus.is_empty() {
        frame.render_widget(
            Paragraph::new("No GPU found in /sys/class/drm")
                .block(panel_block(" GPU / iGPU "))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let summary_height = if area.width < 100 { 7 } else { 5 };
    let table_height = if area.width < 100 { 6 } else { 7 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Length(table_height),
            Constraint::Min(8),
        ])
        .split(area);

    render_gpu_summary(frame, chunks[0], app);
    render_gpu_table(frame, chunks[1], app);
    render_gpu_charts(frame, chunks[2], app);
}

fn render_gpu_summary(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let Some(gpu) = app.primary_gpu() else {
        return;
    };

    let inner = area.inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });
    if area.width < 100 {
        let usage_text = gpu
            .usage_pct
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| String::from("N/A"));
        let temp_text = gpu
            .temp_c
            .map(|value| format!("{value:.0}\u{b0}C"))
            .unwrap_or_else(|| String::from("Not exposed"));
        let power_text = gpu
            .power_w
            .map(|value| format!("{value:.1} W"))
            .unwrap_or_else(|| String::from("N/A"));
        let clock_text = clock_text(gpu);
        let lines = vec![
            Line::from(vec![
                styled("Primary: ", t.overlay1),
                Span::styled(
                    format!("{} {}", gpu.kind, truncate(&gpu.model, 42)),
                    Style::default().fg(t.text).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                styled("Driver: ", t.overlay1),
                styled(gpu.driver.clone(), t.text),
                styled(" | Slot: ", t.overlay1),
                styled(gpu.pci_slot.clone(), t.text),
                styled(" | State: ", t.overlay1),
                styled(gpu.power_state.clone(), t.text),
            ]),
            Line::from(vec![
                styled("Load: ", t.overlay1),
                styled(usage_text, t.text),
                styled(" | Temp: ", t.overlay1),
                styled(temp_text, t.text),
                styled(" | Clock: ", t.overlay1),
                styled(clock_text, t.text),
            ]),
            Line::from(vec![
                styled("Power: ", t.overlay1),
                styled(power_text, t.text),
                styled(" | ", t.overlay1),
                styled("Sensor: ", t.overlay1),
                styled(gpu.sensor_source.clone(), t.overlay1),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(panel_block(" GPU Overview ")),
            area,
        );
        return;
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    let usage_text = gpu
        .usage_pct
        .map(|value| format!("{value:.0}%"))
        .unwrap_or_else(|| String::from("N/A"));
    let temp_text = gpu
        .temp_c
        .map(|value| format!("{value:.0}\u{b0}C"))
        .unwrap_or_else(|| String::from("N/A"));
    let power_text = gpu
        .power_w
        .map(|value| format!("{value:.1} W"))
        .unwrap_or_else(|| String::from("N/A"));

    let details = vec![
        Line::from(vec![
            styled("Primary: ", t.overlay1),
            Span::styled(
                format!("{} {}", gpu.kind, truncate(&gpu.model, 44)),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            styled("Driver: ", t.overlay1),
            styled(gpu.driver.clone(), t.text),
            styled("  |  Slot: ", t.overlay1),
            styled(gpu.pci_slot.clone(), t.text),
            styled("  |  Power: ", t.overlay1),
            styled(gpu.power_state.clone(), t.text),
        ]),
        Line::from(vec![
            styled("Sensors: ", t.overlay1),
            styled(gpu.sensor_source.clone(), t.overlay1),
            styled("  |  Usage: ", t.overlay1),
            styled(usage_text, t.text),
            styled("  |  Temp: ", t.overlay1),
            styled(temp_text, t.text),
            styled("  |  Draw: ", t.overlay1),
            styled(power_text, t.text),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(details).block(panel_block(" GPU Overview ")),
        columns[0],
    );

    render_optional_gauge(
        frame,
        columns[1],
        "GPU Load",
        gpu.usage_pct,
        t.accent_purple,
        "%",
    );
    render_optional_gauge(
        frame,
        columns[2],
        "GPU Temp",
        gpu.temp_c,
        t.accent_orange,
        "\u{b0}C",
    );
}

fn render_optional_gauge(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: Option<f64>,
    color: ratatui::style::Color,
    suffix: &str,
) {
    let display = value
        .map(|v| format!("{v:.0}{suffix}"))
        .unwrap_or_else(|| String::from("N/A"));
    let ratio = value.unwrap_or(0.0) / 100.0;
    let sev = value.map(Severity::from_usage).unwrap_or(Severity::Neutral);
    let gauge = LineGauge::default()
        .block(
            ratatui::widgets::Block::default().title(format!("{} {label} {display}", sev.symbol())),
        )
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .line_set(ratatui::symbols::line::THICK)
        .ratio(ratio.clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

fn clock_text(gpu: &crate::types::GpuInfo) -> String {
    match (gpu.frequency_mhz, gpu.max_frequency_mhz) {
        (Some(cur), Some(max)) if max >= 1000 => format!("{cur}/{:.2}G", max as f64 / 1000.0),
        (Some(cur), Some(max)) => format!("{cur}/{max}M"),
        (Some(cur), None) => format!("{cur} MHz"),
        _ => String::from("N/A"),
    }
}

fn render_gpu_table(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    if area.width < 100 {
        let rows = app
            .gpus
            .iter()
            .map(|gpu| {
                let temp = gpu
                    .temp_c
                    .map(|value| format!("{value:.0}\u{b0}C"))
                    .unwrap_or_else(|| String::from("N/A"));
                let usage = gpu
                    .usage_pct
                    .map(|value| format!("{value:.0}%"))
                    .unwrap_or_else(|| String::from("N/A"));
                Row::new(vec![
                    Cell::from(Span::styled(
                        format!("{} {}", gpu.kind, gpu.card),
                        Style::default()
                            .fg(t.accent_purple)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(truncate(&gpu.model, 26)),
                    Cell::from(gpu.driver.as_str()),
                    Cell::from(usage),
                    Cell::from(clock_text(gpu)),
                    Cell::from(temp),
                    Cell::from(truncate(&gpu.sensor_source, 16)),
                ])
                .style(Style::default().fg(t.text))
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(12),
                    Constraint::Min(18),
                    Constraint::Length(8),
                    Constraint::Length(7),
                    Constraint::Length(12),
                    Constraint::Length(7),
                    Constraint::Length(17),
                ],
            )
            .header(Row::new(vec![
                Cell::from(header_col("Card")),
                Cell::from(header_col("Model")),
                Cell::from(header_col("Driver")),
                Cell::from(header_col("Load")),
                Cell::from(header_col("Clock")),
                Cell::from(header_col("Temp")),
                Cell::from(header_col("Sensor")),
            ]))
            .block(panel_block(" Detected Graphics Devices "))
            .column_spacing(1),
            area,
        );
        return;
    }

    let rows = app
        .gpus
        .iter()
        .map(|gpu| {
            let temp = gpu
                .temp_c
                .map(|value| format!("{value:.0}\u{b0}C"))
                .unwrap_or_else(|| String::from("N/A"));
            let usage = gpu
                .usage_pct
                .map(|value| format!("{value:.0}%"))
                .unwrap_or_else(|| String::from("N/A"));
            let power = gpu
                .power_w
                .map(|value| format!("{value:.1} W"))
                .unwrap_or_else(|| String::from("N/A"));
            let memory = match (gpu.memory_used_mb, gpu.memory_total_mb) {
                (Some(used), Some(total)) if total > 0.0 => {
                    format!("{used:.0}/{total:.0} MB")
                }
                _ if gpu.kind == "iGPU" => String::from("Shared"),
                _ => String::from("N/A"),
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    format!("{} {}", gpu.kind, gpu.card),
                    Style::default()
                        .fg(t.accent_purple)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(truncate(&gpu.vendor, 10)),
                Cell::from(truncate(&gpu.model, 30)),
                Cell::from(gpu.driver.as_str()),
                Cell::from(usage),
                Cell::from(clock_text(gpu)),
                Cell::from(temp),
                Cell::from(power),
                Cell::from(memory),
                Cell::from(truncate(&gpu.sensor_source, 18)),
            ])
            .style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(11),
                Constraint::Length(11),
                Constraint::Min(18),
                Constraint::Length(9),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Length(7),
                Constraint::Length(9),
                Constraint::Length(14),
                Constraint::Length(19),
            ],
        )
        .header(Row::new(vec![
            Cell::from(header_col("Card")),
            Cell::from(header_col("Vendor")),
            Cell::from(header_col("Model")),
            Cell::from(header_col("Driver")),
            Cell::from(header_col("Load")),
            Cell::from(header_col("Clock")),
            Cell::from(header_col("Temp")),
            Cell::from(header_col("Power")),
            Cell::from(header_col("Memory")),
            Cell::from(header_col("Sensor")),
        ]))
        .block(panel_block(" Detected Graphics Devices "))
        .column_spacing(1),
        area,
    );
}

fn render_gpu_charts(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let chunks = Layout::default()
        .direction(if area.width < 100 {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_history_chart(
        frame,
        chunks[0],
        "GPU load trend",
        &app.gpu_usage_history,
        t.accent_purple,
    );
    render_history_chart(
        frame,
        chunks[1],
        "GPU temperature trend",
        &app.gpu_temp_history,
        t.accent_orange,
    );
}

fn render_history_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
) {
    if data.len() < 2 {
        frame.render_widget(
            Paragraph::new("Sensor not exposed or still collecting")
                .block(panel_block(title))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let t = theme::get();
    let points: Vec<(f64, f64)> = data.iter().copied().collect();
    let x_start = data.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = data.back().map(|p| p.0).unwrap_or(1.0).max(x_start + 1.0);
    let max_y = data
        .iter()
        .map(|(_, value)| *value)
        .fold(0.0, f64::max)
        .max(100.0);

    let dataset = ratatui::widgets::Dataset::default()
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .data(&points);

    let chart = Chart::new(vec![dataset])
        .block(panel_block(title))
        .x_axis(
            Axis::default()
                .bounds([x_start, x_end])
                .style(Style::default().fg(t.overlay1)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_y])
                .labels(vec![
                    Span::styled("0", Style::default().fg(t.overlay1)),
                    Span::styled(format!("{max_y:.0}"), Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}

use ratatui::widgets::Cell;
