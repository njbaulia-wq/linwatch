use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Cell, Chart, GraphType, LineGauge, Paragraph, Row, Table},
    Frame,
};

use crate::state::AppState;
use crate::types::{GpuInfo, Severity};

use super::common::*;
use super::theme;

pub fn gpu_tab(frame: &mut Frame, area: Rect, app: &AppState) {
    if area.width < 10 || area.height < 5 {
        return;
    }

    if app.gpus.is_empty() {
        render_no_gpu_fallback(frame, area, app);
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
    render_gpu_charts(frame, chunks[1], app, area.width < 90);
    render_gpu_table(frame, chunks[2], app, area.width < 110);
}

fn render_no_gpu_fallback(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();
    let block = panel_block(" Graphics & Display Subsystem ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        frame.render_widget(
            Paragraph::new("No DRM graphics adapters found.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(t.overlay1)),
            inner,
        );
        return;
    }

    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        styled("Direct Rendering Manager (DRM): ", t.overlay1),
        styled(
            "No hardware GPU nodes exposed (/sys/class/drm)",
            t.accent_orange,
        ),
    ]));

    lines.push(Line::from(vec![
        styled("Host Environment: ", t.overlay1),
        styled(app.env.label(), t.accent_teal),
        dim("  │  Kernel: "),
        styled(truncate(&app.system.kernel, 24), t.text),
        dim("  │  Display: "),
        styled("Headless / Virtual Console", t.subtext0),
    ]));

    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        styled("├─ Acceleration Mode:  ", t.accent_blue),
        styled(
            "Software Rasterizer (llvmpipe / swiftshader fallback)",
            t.text,
        ),
    ]));

    lines.push(Line::from(vec![
        styled("├─ DRM Subsystem Path:  ", t.subtext0),
        styled("/sys/class/drm (0 devices detected)", t.text),
    ]));

    lines.push(Line::from(vec![
        styled("├─ Virtualization:      ", t.subtext0),
        styled(
            if app.env.is_host_scoped() {
                "Container / VM environment (GPU pass-through not configured)"
            } else {
                "Bare metal host without active discrete or integrated GPU DRM driver"
            },
            t.text,
        ),
    ]));

    lines.push(Line::from(vec![
        styled("└─ Diagnostics Note:    ", t.accent_green),
        styled(
            "To monitor GPUs inside containers, bind /dev/dri and /sys/class/drm.",
            t.text,
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_vitals(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();
    let Some(gpu) = app.primary_gpu() else {
        return;
    };

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

    // Card 1: GPU Engine & Acceleration
    let eng_block = panel_block(" GPU Engine & Acceleration ");
    let eng_inner = eng_block.inner(cards[0]);
    frame.render_widget(eng_block, cards[0]);

    if eng_inner.height >= 2 {
        let eng_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(eng_inner);

        if let Some(usage) = gpu.usage_pct {
            let sev = Severity::from_usage(usage);
            let gauge = LineGauge::default()
                .gauge_style(
                    Style::default()
                        .fg(severity_color(sev))
                        .add_modifier(Modifier::BOLD),
                )
                .line_set(ratatui::symbols::line::THICK)
                .ratio((usage / 100.0).clamp(0.0, 1.0))
                .label(Span::styled(
                    format!("{} Load {:>5.1}%", sev.symbol(), usage),
                    Style::default()
                        .fg(severity_color(sev))
                        .add_modifier(Modifier::BOLD),
                ));
            frame.render_widget(gauge, eng_rows[0]);
        } else {
            let status_line = Line::from(vec![
                styled("● ", t.accent_purple),
                styled("Hardware Accelerated Engine Active", t.text),
            ]);
            frame.render_widget(Paragraph::new(status_line), eng_rows[0]);
        }

        let line2 = Line::from(vec![
            styled(format!("{} ", gpu.kind), t.accent_purple),
            Span::styled(
                truncate(&gpu.model, 26),
                Style::default().fg(t.text).add_modifier(Modifier::BOLD),
            ),
            dim("  │  Driver: "),
            styled(truncate(&gpu.driver, 12), t.accent_blue),
            dim("  │  Slot: "),
            styled(truncate(&gpu.pci_slot, 12), t.subtext0),
        ]);
        frame.render_widget(Paragraph::new(line2), eng_rows[1]);
    } else if eng_inner.height == 1 {
        let line = Line::from(vec![
            styled(format!("{}: ", gpu.card), t.accent_purple),
            styled(truncate(&gpu.model, 28), t.text),
        ]);
        frame.render_widget(Paragraph::new(line), eng_inner);
    }

    // Card 2: VRAM & Thermal Vitals
    let vram_block = panel_block(" VRAM & Thermal Vitals ");
    let vram_inner = vram_block.inner(cards[1]);
    frame.render_widget(vram_block, cards[1]);

    if vram_inner.height >= 2 {
        let vram_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(vram_inner);

        if let (Some(used), Some(total)) = (gpu.memory_used_mb, gpu.memory_total_mb) {
            if total > 0.0 {
                let mem_pct = (used / total) * 100.0;
                let sev = Severity::from_usage(mem_pct);
                let gauge = LineGauge::default()
                    .gauge_style(
                        Style::default()
                            .fg(severity_color(sev))
                            .add_modifier(Modifier::BOLD),
                    )
                    .line_set(ratatui::symbols::line::THICK)
                    .ratio((mem_pct / 100.0).clamp(0.0, 1.0))
                    .label(Span::styled(
                        format!("{} VRAM {:>5.1}%", sev.symbol(), mem_pct),
                        Style::default()
                            .fg(severity_color(sev))
                            .add_modifier(Modifier::BOLD),
                    ));
                frame.render_widget(gauge, vram_rows[0]);
            } else {
                render_temp_or_clock_gauge(frame, vram_rows[0], gpu);
            }
        } else {
            render_temp_or_clock_gauge(frame, vram_rows[0], gpu);
        }

        let power_str = gpu
            .power_w
            .map(|w| format!("{w:.1} W"))
            .unwrap_or_else(|| String::from("N/A"));
        let temp_str = gpu
            .temp_c
            .map(|c| format!("{c:.0}\u{b0}C"))
            .unwrap_or_else(|| String::from("N/A"));

        let line2 = Line::from(vec![
            dim("Temp: "),
            styled(
                temp_str,
                if gpu.temp_c.is_some() {
                    t.accent_orange
                } else {
                    t.subtext0
                },
            ),
            dim("  │  Power: "),
            styled(power_str, t.accent_yellow),
            dim("  │  Clock: "),
            styled(clock_text(gpu), t.accent_teal),
            dim("  │  State: "),
            styled(&gpu.power_state, t.text),
        ]);
        frame.render_widget(Paragraph::new(line2), vram_rows[1]);
    } else if vram_inner.height == 1 {
        let temp_str = gpu
            .temp_c
            .map(|c| format!("{c:.0}\u{b0}C"))
            .unwrap_or_else(|| String::from("N/A"));
        let line = Line::from(vec![
            dim("Temp: "),
            styled(temp_str, t.accent_orange),
            dim(" │ Power: "),
            styled(
                gpu.power_w
                    .map(|w| format!("{w:.0}W"))
                    .unwrap_or_else(|| "N/A".into()),
                t.accent_yellow,
            ),
        ]);
        frame.render_widget(Paragraph::new(line), vram_inner);
    }
}

fn render_temp_or_clock_gauge(frame: &mut Frame, area: Rect, gpu: &GpuInfo) {
    let t = theme::get();
    if let Some(temp) = gpu.temp_c {
        let sev = Severity::from_usage(temp);
        let gauge = LineGauge::default()
            .gauge_style(
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            )
            .line_set(ratatui::symbols::line::THICK)
            .ratio((temp / 100.0).clamp(0.0, 1.0))
            .label(Span::styled(
                format!("{} Temp {:>3.0}\u{b0}C", sev.symbol(), temp),
                Style::default()
                    .fg(severity_color(sev))
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, area);
    } else {
        let memory_label = if gpu.kind == "iGPU" {
            "Shared System Memory (UMA)"
        } else {
            "VRAM Telemetry N/A"
        };
        let line = Line::from(vec![
            styled("■ ", t.accent_teal),
            styled(memory_label, t.subtext0),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }
}

fn render_gpu_charts(frame: &mut Frame, area: Rect, app: &AppState, narrow: bool) {
    let t = theme::get();

    if narrow {
        // Narrow: combined dual-trace telemetry chart
        render_combined_gpu_chart(frame, area, app);
    } else {
        // Wide: side-by-side Load Trend & Temperature Trend
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_single_gpu_chart(
            frame,
            panels[0],
            " GPU Load Trend (120s) ",
            &app.gpu_usage_history,
            t.accent_purple,
            "%",
        );
        render_single_gpu_chart(
            frame,
            panels[1],
            " GPU Temperature Trend (120s) ",
            &app.gpu_temp_history,
            t.accent_orange,
            "\u{b0}C",
        );
    }
}

fn render_single_gpu_chart(
    frame: &mut Frame,
    area: Rect,
    title_prefix: &str,
    data: &std::collections::VecDeque<(f64, f64)>,
    color: ratatui::style::Color,
    unit: &str,
) {
    let t = theme::get();

    if data.len() < 2 {
        frame.render_widget(
            Paragraph::new("Sensor telemetry collecting or not exposed")
                .block(panel_block(title_prefix))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let current = data.back().map(|p| p.1).unwrap_or(0.0);
    let peak = data.iter().map(|p| p.1).fold(current, f64::max);
    let avg = data.iter().map(|p| p.1).sum::<f64>() / data.len() as f64;

    let title = format!(
        "{title_prefix}[Cur: {current:.0}{unit} │ Peak: {peak:.0}{unit} │ Avg: {avg:.0}{unit}] "
    );

    let points: Vec<(f64, f64)> = data.iter().copied().collect();
    let x_start = data.front().map(|p| p.0).unwrap_or(0.0);
    let x_end = data.back().map(|p| p.0).unwrap_or(1.0).max(x_start + 1.0);
    let max_y = peak.max(100.0);

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
                .labels(vec![
                    Span::styled("-120s", Style::default().fg(t.overlay1)),
                    Span::styled("now", Style::default().fg(color)),
                ])
                .style(Style::default().fg(t.overlay1)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_y])
                .labels(vec![
                    Span::styled("0", Style::default().fg(t.overlay1)),
                    Span::styled(
                        format!("{:.0}{unit}", max_y * 0.5),
                        Style::default().fg(t.overlay1),
                    ),
                    Span::styled(format!("{max_y:.0}{unit}"), Style::default().fg(t.overlay1)),
                ])
                .style(Style::default().fg(t.overlay1)),
        );
    frame.render_widget(chart, area);
}

fn render_combined_gpu_chart(frame: &mut Frame, area: Rect, app: &AppState) {
    let t = theme::get();

    let has_usage = app.gpu_usage_history.len() >= 2;
    let has_temp = app.gpu_temp_history.len() >= 2;

    let title = " GPU Telemetry Trend (120s) [▲ Load │ ▼ Temp] ";

    if !has_usage && !has_temp {
        frame.render_widget(
            Paragraph::new("GPU telemetry collecting or not exposed")
                .block(panel_block(title))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let points_usage: Vec<(f64, f64)> = app.gpu_usage_history.iter().copied().collect();
    let points_temp: Vec<(f64, f64)> = app.gpu_temp_history.iter().copied().collect();

    let x_start = app
        .gpu_usage_history
        .front()
        .or_else(|| app.gpu_temp_history.front())
        .map(|p| p.0)
        .unwrap_or(0.0);
    let x_end = app
        .gpu_usage_history
        .back()
        .or_else(|| app.gpu_temp_history.back())
        .map(|p| p.0)
        .unwrap_or(1.0)
        .max(x_start + 1.0);

    let mut datasets = Vec::new();
    if has_usage {
        datasets.push(
            ratatui::widgets::Dataset::default()
                .name("Load")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(t.accent_purple)
                        .add_modifier(Modifier::BOLD),
                )
                .data(&points_usage),
        );
    }
    if has_temp {
        datasets.push(
            ratatui::widgets::Dataset::default()
                .name("Temp")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(t.accent_orange)
                        .add_modifier(Modifier::BOLD),
                )
                .data(&points_temp),
        );
    }

    let chart = Chart::new(datasets)
        .block(panel_block(title))
        .x_axis(
            Axis::default()
                .bounds([x_start, x_end])
                .labels(vec![
                    Span::styled("-120s", Style::default().fg(t.overlay1)),
                    Span::styled("now", Style::default().fg(t.accent_purple)),
                ])
                .style(Style::default().fg(t.overlay1)),
        )
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

fn render_gpu_table(frame: &mut Frame, area: Rect, app: &AppState, compact: bool) {
    let t = theme::get();
    let title = format!(" Detected Graphics Adapters ({}) ", app.gpus.len());

    let rows = app
        .gpus
        .iter()
        .map(|gpu| {
            let temp_str = gpu
                .temp_c
                .map(|v| format!("{v:.0}\u{b0}C"))
                .unwrap_or_else(|| String::from("N/A"));
            let usage_str = gpu
                .usage_pct
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| String::from("N/A"));
            let power_str = gpu
                .power_w
                .map(|v| format!("{v:.1} W"))
                .unwrap_or_else(|| String::from("N/A"));
            let memory_str = match (gpu.memory_used_mb, gpu.memory_total_mb) {
                (Some(used), Some(total)) if total > 0.0 => {
                    format!("{used:.0}/{total:.0} MB")
                }
                _ if gpu.kind == "iGPU" => String::from("Shared"),
                _ => String::from("N/A"),
            };

            let mut cells = vec![
                Cell::from(Span::styled(
                    &gpu.card,
                    Style::default()
                        .fg(t.accent_purple)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(&gpu.kind, Style::default().fg(t.accent_teal))),
                Cell::from(truncate(&gpu.model, 28)),
                Cell::from(gpu.driver.as_str()),
                Cell::from(usage_str),
                Cell::from(clock_text(gpu)),
                Cell::from(temp_str),
            ];

            if !compact {
                cells.push(Cell::from(power_str));
                cells.push(Cell::from(memory_str));
            }

            Row::new(cells).style(Style::default().fg(t.text))
        })
        .collect::<Vec<_>>();

    let mut widths = vec![
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Min(16),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(8),
    ];
    let mut headers = vec![
        Cell::from(header_col("Card")),
        Cell::from(header_col("Kind")),
        Cell::from(header_col("Model")),
        Cell::from(header_col("Driver")),
        Cell::from(header_col("Load")),
        Cell::from(header_col("Clock")),
        Cell::from(header_col("Temp")),
    ];

    if !compact {
        widths.push(Constraint::Length(9));
        widths.push(Constraint::Length(14));
        headers.push(Cell::from(header_col("Power")));
        headers.push(Cell::from(header_col("Memory")));
    }

    let table = Table::new(rows, widths)
        .header(Row::new(headers))
        .block(panel_block(title))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn clock_text(gpu: &GpuInfo) -> String {
    match (gpu.frequency_mhz, gpu.max_frequency_mhz) {
        (Some(cur), Some(max)) if max >= 1000 => format!("{cur}/{:.2}G", max as f64 / 1000.0),
        (Some(cur), Some(max)) => format!("{cur}/{max}M"),
        (Some(cur), None) => format!("{cur} MHz"),
        _ => String::from("N/A"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::types::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::VecDeque;

    fn make_test_app(gpus: Vec<GpuInfo>) -> AppState {
        let mut gpu_usage_history = VecDeque::new();
        let mut gpu_temp_history = VecDeque::new();
        if !gpus.is_empty() {
            gpu_usage_history.push_back((0.0, 30.0));
            gpu_usage_history.push_back((1.0, 45.0));
            gpu_temp_history.push_back((0.0, 52.0));
            gpu_temp_history.push_back((1.0, 56.0));
        }

        let mut app = AppState::test_state();
        app.system.hostname = "gpu-test".into();
        app.system.os_name = "Linux".into();
        app.system.os_version = "6.8.0".into();
        app.system.kernel = "6.8.0-generic".into();
        app.system.cpu_model = "AMD Ryzen 9".into();
        app.system.cpu_count = 8;
        app.system.selinux_mode = "Disabled".into();
        app.cpu_usage = 15.0;
        app.core_usages = vec![15.0; 8];
        app.gpus = gpus;
        app.gpu_usage_history = gpu_usage_history;
        app.gpu_temp_history = gpu_temp_history;
        app.active_tab = ViewTab::Gpu;
        app
    }

    #[test]
    fn gpu_renders_fallback_when_no_gpu() {
        let app = make_test_app(Vec::new());
        for (w, h) in [(60, 15), (80, 24), (100, 30), (120, 40), (160, 48)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    gpu_tab(f, f.size(), &app);
                })
                .unwrap();
        }
    }

    #[test]
    fn gpu_renders_with_gpu_present() {
        let gpus = vec![GpuInfo {
            card: "card0".into(),
            vendor: "NVIDIA".into(),
            model: "GeForce RTX 4090".into(),
            driver: "nvidia".into(),
            kind: "dGPU".into(),
            pci_slot: "0000:01:00.0".into(),
            temp_c: Some(58.0),
            usage_pct: Some(65.0),
            power_w: Some(220.0),
            frequency_mhz: Some(2520),
            max_frequency_mhz: Some(2520),
            rc6_residency_ms: None,
            memory_used_mb: Some(8192.0),
            memory_total_mb: Some(24576.0),
            power_state: "D0".into(),
            sensor_source: "nvidia-smi".into(),
        }];

        let app = make_test_app(gpus);
        for (w, h) in [(70, 20), (90, 25), (120, 35), (160, 45)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    gpu_tab(f, f.size(), &app);
                })
                .unwrap();
        }
    }
}
