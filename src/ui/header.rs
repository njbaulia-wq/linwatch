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
    let width = area.width;
    let compact = width < 90;
    let ultra_compact = width < 70;

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let left_w = layout[0].width.saturating_sub(1) as usize;
    let _right_w = layout[1].width.saturating_sub(1) as usize;

    // Line 1 Left: Brand, Version, Environment badge, OS name
    let mut left_l1_spans = vec![
        Span::styled(
            "LINWATCH ",
            Style::default()
                .fg(t.accent_blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(t.overlay1),
        ),
        Span::styled(
            format!("[{}]", app.env.badge()),
            Style::default()
                .fg(t.accent_teal)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if !compact {
        left_l1_spans.push(Span::styled(
            format!(" · {} {}", app.system.os_name, app.system.os_version),
            Style::default().fg(t.subtext0),
        ));
    }

    // Line 2 Left: Hostname, Uptime, Cores, Procs
    let left_l2_str = if ultra_compact {
        format!("{} · Up {}", app.system.hostname, app.uptime)
    } else if compact {
        format!(
            "{} · Up {} · {}c",
            app.system.hostname, app.uptime, app.system.cpu_count
        )
    } else {
        format!(
            "Host: {} · Up {} · {} cores · {} procs",
            app.system.hostname, app.uptime, app.system.cpu_count, app.process_count
        )
    };
    let left_l2_spans = vec![Span::styled(
        common::truncate(&left_l2_str, left_w.max(8)),
        Style::default().fg(t.subtext1),
    )];

    let header_block = common::panel_block("").borders(ratatui::widgets::Borders::NONE);
    frame.render_widget(
        Paragraph::new(vec![Line::from(left_l1_spans), Line::from(left_l2_spans)])
            .block(header_block.clone()),
        layout[0],
    );

    // Right Side
    let health_sev = crate::types::Severity::from_health(app.health_score as f64);
    let health_color = common::severity_color(health_sev);
    let sample_color = common::sample_status_color(app.sample_status());
    let status_label = match health_sev {
        crate::types::Severity::Ok => "Healthy",
        crate::types::Severity::Warn => "Attention",
        crate::types::Severity::Critical => "Critical",
        crate::types::Severity::Neutral => "Monitoring",
    };

    // Line 1 Right: Health Score, Data Quality, Battery
    let mut right_l1_spans = vec![
        common::severity_chip(health_sev),
        Span::styled(
            format!(" {status_label} {}%  ", app.health_score),
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        ),
        common::dim("Data "),
        Span::styled(
            app.sample_status(),
            Style::default()
                .fg(sample_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(bat) = app.battery_pct {
        let bat_color = if bat <= app.battery_alert {
            t.accent_red
        } else if bat <= 35 {
            t.accent_yellow
        } else {
            t.accent_green
        };

        let info = &app.battery_info;
        let psys_w = app.rapl_power.psys_w;
        let cpu_w = app
            .rapl_power
            .pkg_w
            .or_else(|| app.hw_sensors.iter().find_map(|s| s.power_w));

        let bat_str = match (
            info.power_w,
            info.time_to_empty_mins,
            info.time_to_full_mins,
            info.status.as_str(),
        ) {
            (Some(w), Some(mins), _, "Discharging") if w > 0.05 => {
                let h = mins / 60;
                let m = mins % 60;
                if h > 0 {
                    format!(" · Bat {bat}% · {:.1}W ({h}h {m:02}m)", w)
                } else {
                    format!(" · Bat {bat}% · {:.1}W ({m}m)", w)
                }
            }
            (Some(w), _, Some(mins), "Charging") if w > 0.05 => {
                let h = mins / 60;
                let m = mins % 60;
                if h > 0 {
                    format!(" · Bat {bat}% · +{:.1}W ({h}h {m:02}m chg)", w)
                } else {
                    format!(" · Bat {bat}% · +{:.1}W ({m}m chg)", w)
                }
            }
            (Some(w), _, _, "Discharging") if w > 0.05 => format!(" · Bat {bat}% · {:.1}W", w),
            (Some(w), _, _, "Charging") if w > 0.05 => format!(" · Bat {bat}% · +{:.1}W", w),
            _ => {
                if let Some(p) = psys_w.filter(|&w| w > 0.1) {
                    format!(" · Bat {bat}% · {:.1}W (Sys AC)", p)
                } else if let Some(c) = cpu_w.filter(|&w| w > 0.1) {
                    format!(" · Bat {bat}% · {:.1}W (AC)", c)
                } else if info.ac_online == Some(true) || info.status == "Full" {
                    format!(" · Bat {bat}% · AC Online")
                } else {
                    format!(" · Bat {bat}%")
                }
            }
        };

        right_l1_spans.push(Span::styled(bat_str, Style::default().fg(bat_color)));
    } else {
        let psys_w = app.rapl_power.psys_w;
        let cpu_w = app
            .rapl_power
            .pkg_w
            .or_else(|| app.hw_sensors.iter().find_map(|s| s.power_w));
        if let Some(p) = psys_w.filter(|&w| w > 0.1) {
            right_l1_spans.push(Span::styled(
                format!(" · Sys: {:.1}W (AC)", p),
                Style::default().fg(t.accent_teal),
            ));
        } else if let Some(w) = cpu_w.filter(|&w| w > 0.1) {
            right_l1_spans.push(Span::styled(
                format!(" · Pwr: {:.1}W (AC)", w),
                Style::default().fg(t.accent_teal),
            ));
        } else if let Some(true) = app.battery_info.ac_online {
            right_l1_spans.push(Span::styled(
                " · AC Online",
                Style::default().fg(t.accent_teal),
            ));
        } else {
            right_l1_spans.push(Span::styled(" · AC Power", Style::default().fg(t.overlay1)));
        }
    }

    // Line 2 Right: Load averages & CPU Temperature
    let mut right_l2_spans = vec![
        Span::styled("Load: ", Style::default().fg(t.overlay1)),
        Span::styled(
            format!(
                "{} {} {}",
                app.load_avg[0], app.load_avg[1], app.load_avg[2]
            ),
            Style::default().fg(t.subtext0),
        ),
    ];
    if let Some(temp) = app.temp_c {
        let temp_color = if temp >= app.temp_alert {
            t.accent_red
        } else if temp >= 70.0 {
            t.accent_orange
        } else {
            t.accent_teal
        };
        right_l2_spans.push(Span::styled(
            format!(" · CPU {:.0}\u{b0}C", temp),
            Style::default().fg(temp_color),
        ));
    }

    frame.render_widget(
        Paragraph::new(vec![Line::from(right_l1_spans), Line::from(right_l2_spans)])
            .block(header_block)
            .alignment(Alignment::Right),
        layout[1],
    );
}
