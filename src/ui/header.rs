use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::common;
use super::theme;

pub fn header(frame: &mut Frame, area: Rect, app: &crate::state::AppState) {
    let t = theme::get();
    let wide = app.terminal_width >= 110;
    let compact = app.terminal_width < 100;

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);
    let left_w = layout[0].width.saturating_sub(1) as usize;
    let right_w = layout[1].width.saturating_sub(1) as usize;

    // Title always carries the environment badge (HW/VM/CTR/WSL) so
    // container/WSL users know counters are host-scoped.
    let title = Span::styled(
        format!(
            "LinWatch v{} [{}]",
            env!("CARGO_PKG_VERSION"),
            app.env.badge()
        ),
        Style::default()
            .fg(t.accent_blue)
            .add_modifier(Modifier::BOLD),
    );

    let mut meta = if compact {
        format!(
            "Host: {} | {} {}",
            app.system.hostname, app.system.os_name, app.system.os_version
        )
    } else {
        format!(
            "System health dashboard | Host: {} | {} {} | Sec: {}",
            app.system.hostname, app.system.os_name, app.system.os_version, app.system.selinux_mode
        )
    };
    if app.git_modified_files > 0 {
        meta.push_str(&format!(" | Git: {} modified", app.git_modified_files));
    }
    if wide {
        meta.push_str(&format!(
            " | Kernel: {} | {}",
            common::truncate(&app.system.kernel, 14),
            app.system.cpu_vulnerabilities
        ));
    }
    if app.env.is_host_scoped() {
        meta.push_str(&format!(" | {}", app.platform_summary()));
    }
    let meta_line = Line::from(Span::styled(
        common::truncate(&meta, left_w.max(8)),
        Style::default().fg(t.subtext1),
    ));

    let health_sev = crate::types::Severity::from_health(app.health_score as f64);
    let health_color = common::severity_color(health_sev);
    let sample_color = common::sample_status_color(app.sample_status());
    let temp_value = app
        .temp_c
        .map(|temp| format!("{temp:.0}\u{b0}C"))
        .unwrap_or_else(|| String::from("N/A"));
    let temp_state = match app.temp_c {
        Some(temp) if temp >= 80.0 => "Hot",
        Some(temp) if temp >= 70.0 => "Warm",
        Some(_) => "Normal",
        None => "No sensor",
    };
    let gpu_status = app
        .primary_gpu()
        .map(|gpu| {
            let usage = gpu
                .usage_pct
                .map(|value| format!("{value:.0}%"))
                .unwrap_or_else(|| String::from("N/A"));
            let temp = gpu
                .temp_c
                .map(|value| format!("{value:.0}\u{b0}C"))
                .unwrap_or_else(|| String::from("N/A"));
            format!("GPU {} {usage} {temp}", gpu.kind)
        })
        .unwrap_or_else(|| String::from("GPU N/A"));
    let status_label = match health_sev {
        crate::types::Severity::Ok => "Healthy",
        crate::types::Severity::Warn => "Attention",
        crate::types::Severity::Critical => "Critical",
        crate::types::Severity::Neutral => "Monitoring",
    };

    let mut health_spans = vec![
        Span::styled(
            format!(
                "{} {status_label} {}%  ",
                health_sev.symbol(),
                app.health_score
            ),
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Data ", Style::default().fg(t.overlay0)),
        Span::styled(
            app.sample_status(),
            Style::default()
                .fg(sample_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !compact {
        health_spans.push(Span::styled("  Updated ", Style::default().fg(t.overlay1)));
        health_spans.push(Span::styled(
            format!("{:.1}s", app.last_sample_at.elapsed().as_secs_f64()),
            Style::default().fg(t.overlay0),
        ));
    }
    let health_line = Line::from(health_spans);

    let context = if compact {
        format!(
            "CPU {temp_value} | {}",
            app.primary_gpu()
                .map(|gpu| format!("GPU {}", gpu.kind))
                .unwrap_or_else(|| String::from("GPU N/A"))
        )
    } else if let Some(bat) = app.battery_pct {
        format!(
            "Battery: {}% {} | CPU Temp: {} {} | {} | Load: {} {} {}",
            bat,
            app.battery_status,
            temp_value,
            temp_state,
            gpu_status,
            app.load_avg[0],
            app.load_avg[1],
            app.load_avg[2]
        )
    } else {
        format!(
            "CPU Temp: {} {} | {} | Load: {} {} {}",
            temp_value, temp_state, gpu_status, app.load_avg[0], app.load_avg[1], app.load_avg[2]
        )
    };

    let header_block = common::panel_block("").borders(ratatui::widgets::Borders::NONE);
    let content = vec![Line::from(title), meta_line];
    frame.render_widget(
        Paragraph::new(content).block(header_block.clone()),
        layout[0],
    );

    let right_block = common::panel_block("").borders(ratatui::widgets::Borders::NONE);
    let right_lines = vec![
        Line::from(Span::styled(
            common::truncate(&context, right_w.max(8)),
            Style::default().fg(t.overlay1),
        )),
        health_line,
        Line::from(Span::styled(
            common::truncate(
                &format!(
                    "CPU: {} cores | Processes: {} | Uptime: {}",
                    app.system.cpu_count, app.process_count, app.uptime
                ),
                right_w.max(8),
            ),
            Style::default().fg(t.overlay0),
        )),
    ];
    frame.render_widget(
        Paragraph::new(right_lines)
            .block(right_block)
            .alignment(Alignment::Right),
        layout[1],
    );
}
