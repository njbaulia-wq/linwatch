mod collector;
mod state;
mod types;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use std::{
    env, fs, io,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};
use types::{AppConfig, MonitorConfig};

fn main() -> Result<(), io::Error> {
    let (config, config_warnings) = load_config();
    let Some(app_config) = parse_args(config, &config_warnings)? else {
        return Ok(());
    };

    if app_config.json_once {
        return json_output(&app_config);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let result = catch_unwind(AssertUnwindSafe(|| ui::run_app(&mut terminal, app_config)));

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn json_output(config: &AppConfig) -> Result<(), io::Error> {
    use state::AppState;

    if config.watch_json || config.json_lines {
        let mut app = AppState::new_raw(config);
        let mut stdout = io::stdout().lock();
        let mut emitted = 0_u64;
        loop {
            write_snapshot(&mut stdout, &app.to_snapshot(), config.json_pretty)?;
            use std::io::Write;
            writeln!(stdout)?;
            emitted += 1;
            let target_samples =
                config
                    .samples
                    .unwrap_or(if config.watch_json { u64::MAX } else { 1 });
            if emitted >= target_samples {
                return Ok(());
            }
            std::thread::sleep(app.refresh_rate());
            app.update();
        }
    }

    let app = AppState::new_raw(config);
    write_snapshot(
        &mut io::stdout().lock(),
        &app.to_snapshot(),
        config.json_pretty,
    )
}

fn write_snapshot<W: io::Write>(
    writer: &mut W,
    snapshot: &types::MonitorSnapshot,
    pretty: bool,
) -> Result<(), io::Error> {
    if pretty {
        serde_json::to_writer_pretty(writer, snapshot)
    } else {
        serde_json::to_writer(writer, snapshot)
    }
    .map_err(io::Error::other)
}

fn load_config() -> (MonitorConfig, Vec<String>) {
    let config_dir = dirs_config_path();
    let config_path = config_dir.join("linwatch").join("config.toml");

    if !config_path.exists() {
        return (MonitorConfig::default(), Vec::new());
    }

    match fs::read_to_string(&config_path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => validate_config(config),
            Err(e) => {
                let mut warnings = Vec::new();
                warnings.push(format!("Config parse error: {e}"));
                (MonitorConfig::default(), warnings)
            }
        },
        Err(_) => (MonitorConfig::default(), Vec::new()),
    }
}

fn validate_config(mut config: MonitorConfig) -> (MonitorConfig, Vec<String>) {
    let mut warnings = Vec::new();

    if let Some(theme) = config.theme.as_deref() {
        if parse_theme_name(theme).is_err() {
            warnings.push(format!("unsupported theme '{theme}'; using default"));
            config.theme = None;
        }
    }

    if let Some(tab) = config.default_tab.as_deref() {
        if !matches!(
            tab,
            "overview"
                | "cpu"
                | "gpu"
                | "igpu"
                | "memory"
                | "mem"
                | "storage"
                | "disk"
                | "network"
                | "net"
                | "processes"
                | "proc"
        ) {
            warnings.push(format!("unsupported default_tab '{tab}'; using overview"));
            config.default_tab = None;
        }
    }

    clamp_f64(
        &mut config.cpu_alert,
        1.0,
        100.0,
        "cpu_alert",
        &mut warnings,
    );
    clamp_f64(
        &mut config.mem_alert,
        1.0,
        100.0,
        "mem_alert",
        &mut warnings,
    );
    clamp_f64(
        &mut config.temp_alert,
        1.0,
        120.0,
        "temp_alert",
        &mut warnings,
    );
    clamp_f64(
        &mut config.swap_alert,
        0.0,
        100.0,
        "swap_alert",
        &mut warnings,
    );
    clamp_u16(&mut config.disk_alert, 1, 100, "disk_alert", &mut warnings);
    clamp_u16(
        &mut config.battery_alert,
        1,
        100,
        "battery_alert",
        &mut warnings,
    );
    (config, warnings)
}

fn clamp_f64(value: &mut Option<f64>, min: f64, max: f64, name: &str, warnings: &mut Vec<String>) {
    if let Some(current) = *value {
        let clamped = current.clamp(min, max);
        if (clamped - current).abs() > f64::EPSILON {
            warnings.push(format!("{name} out of range; clamped to {clamped}"));
            *value = Some(clamped);
        }
    }
}

fn clamp_u16(value: &mut Option<u16>, min: u16, max: u16, name: &str, warnings: &mut Vec<String>) {
    if let Some(current) = *value {
        let clamped = current.clamp(min, max);
        if clamped != current {
            warnings.push(format!("{name} out of range; clamped to {clamped}"));
            *value = Some(clamped);
        }
    }
}

fn dirs_config_path() -> std::path::PathBuf {
    if let Ok(home) = env::var("HOME") {
        Path::new(&home).join(".config")
    } else {
        Path::new("/etc").to_path_buf()
    }
}

fn parse_args(
    config: MonitorConfig,
    config_warnings: &[String],
) -> Result<Option<AppConfig>, io::Error> {
    let mut app_config = AppConfig {
        refresh_index: resolve_interval_index(config.refresh_interval.as_deref()),
        config,
        json_once: false,
        json_pretty: false,
        json_lines: false,
        watch_json: false,
        samples: None,
    };
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_cli_help();
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("linwatch {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--check-config" => {
                let config_dir = dirs_config_path();
                let config_path = config_dir.join("linwatch").join("config.toml");
                if config_path.exists() {
                    println!("Config file: {}", config_path.display());
                } else {
                    println!("No config file found; using defaults");
                }
                if config_warnings.is_empty() {
                    println!("Config OK — no issues found");
                } else {
                    println!("Config warnings ({}):", config_warnings.len());
                    for warning in config_warnings {
                        println!("  - {warning}");
                    }
                }
                return Ok(None);
            }
            "-i" | "--interval" => {
                let Some(value) = args.next() else {
                    return Err(invalid_arg("--interval requires a value"));
                };
                app_config.refresh_index = parse_interval_index(&value)?;
            }
            _ if arg.starts_with("--interval=") => {
                let value = arg.trim_start_matches("--interval=");
                app_config.refresh_index = parse_interval_index(value)?;
            }
            "--theme" => {
                let Some(value) = args.next() else {
                    return Err(invalid_arg("--theme requires a value"));
                };
                app_config.config.theme = Some(parse_theme_name(&value)?.to_string());
            }
            _ if arg.starts_with("--theme=") => {
                let value = arg.trim_start_matches("--theme=");
                app_config.config.theme = Some(parse_theme_name(value)?.to_string());
            }
            "--json" => {
                app_config.json_once = true;
            }
            "--json-pretty" => {
                app_config.json_once = true;
                app_config.json_pretty = true;
            }
            "--json-lines" => {
                app_config.json_once = true;
                app_config.json_lines = true;
            }
            "--watch-json" => {
                app_config.json_once = true;
                app_config.watch_json = true;
                app_config.json_lines = true;
            }
            "--samples" => {
                let Some(value) = args.next() else {
                    return Err(invalid_arg("--samples requires a value"));
                };
                app_config.samples = Some(parse_sample_count(&value)?);
            }
            _ if arg.starts_with("--samples=") => {
                let value = arg.trim_start_matches("--samples=");
                app_config.samples = Some(parse_sample_count(value)?);
            }
            _ => return Err(invalid_arg(format!("unknown argument: {arg}"))),
        }
    }

    if app_config.samples.is_some() && !app_config.json_once {
        app_config.json_once = true;
        app_config.json_lines = true;
    }

    Ok(Some(app_config))
}

fn parse_theme_name(value: &str) -> Result<&str, io::Error> {
    let normalized = value.trim();
    match normalized {
        "default" | "high_contrast" | "high-contrast" | "colorblind" | "colorblind-safe" => {
            Ok(normalized)
        }
        _ => Err(invalid_arg(format!(
            "unsupported theme: {value}. Supported: default, high_contrast, colorblind"
        ))),
    }
}

fn parse_sample_count(value: &str) -> Result<u64, io::Error> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| invalid_arg(format!("invalid sample count: {value}")))
}

fn resolve_interval_index(value: Option<&str>) -> usize {
    match value {
        Some(v) => parse_interval_index(v).unwrap_or(1),
        None => 1,
    }
}

fn parse_interval_index(value: &str) -> Result<usize, io::Error> {
    let normalized = value.trim().to_ascii_lowercase();
    let millis = if let Some(value) = normalized.strip_suffix("ms") {
        value.trim().parse::<u64>().ok()
    } else if let Some(value) = normalized.strip_suffix('s') {
        value
            .trim()
            .parse::<f64>()
            .ok()
            .map(|seconds| (seconds * 1_000.0).round() as u64)
    } else {
        normalized.parse::<u64>().ok()
    };

    let Some(millis) = millis else {
        return Err(invalid_arg(format!("invalid interval value: {value}")));
    };

    types::REFRESH_INTERVALS_MS
        .iter()
        .position(|supported| *supported == millis)
        .ok_or_else(|| {
            invalid_arg(format!(
                "unsupported interval: {value}. Supported: 500ms, 750ms, 1s, 2s, 5s"
            ))
        })
}

fn invalid_arg<T: Into<String>>(message: T) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn print_cli_help() {
    println!(
        "\
linwatch {}

Usage:
  linwatch [OPTIONS]

Options:
  -i, --interval <VALUE>  Refresh interval: 500ms, 750ms, 1s, 2s, 5s
      --theme <NAME>       Theme: default, high_contrast, colorblind
      --json              Single-shot JSON snapshot to stdout
      --json-pretty       Single-shot pretty-printed JSON
      --json-lines        Emit one JSON snapshot as a line
      --watch-json        Stream JSON Lines until interrupted
      --samples <N>       Limit JSON Lines samples
      --check-config      Validate configuration and exit
  -h, --help              Show this help
  -V, --version           Show version

Config:
  ~/.config/linwatch/config.toml

Keys:
  Q/Esc  Exit
  R      Refresh now
  H      Toggle help
  1-7    Switch tab
  Tab    Next tab
  S      Cycle process sort
  T      Toggle process tree hierarchy
  K      Terminate process ([K/Enter] SIGTERM, [9] SIGKILL)
  /      Search process
  +/-    Change interval",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_index_supported_values() {
        assert_eq!(parse_interval_index("500ms").unwrap(), 0);
        assert_eq!(parse_interval_index("750ms").unwrap(), 1);
        assert_eq!(parse_interval_index("1s").unwrap(), 2);
        assert_eq!(parse_interval_index("2s").unwrap(), 3);
        assert_eq!(parse_interval_index("5s").unwrap(), 4);
        assert_eq!(parse_interval_index("1000ms").unwrap(), 2);
    }

    #[test]
    fn parse_interval_index_unsupported_fails() {
        assert!(parse_interval_index("100ms").is_err());
        assert!(parse_interval_index("invalid").is_err());
        assert!(parse_interval_index("-1s").is_err());
    }

    #[test]
    fn parse_theme_name_supported_and_unsupported() {
        assert_eq!(parse_theme_name("default").unwrap(), "default");
        assert_eq!(parse_theme_name("high_contrast").unwrap(), "high_contrast");
        assert_eq!(parse_theme_name("colorblind").unwrap(), "colorblind");
        assert!(parse_theme_name("unknown_theme").is_err());
    }

    #[test]
    fn parse_sample_count_valid_and_invalid() {
        assert_eq!(parse_sample_count("1").unwrap(), 1);
        assert_eq!(parse_sample_count("100").unwrap(), 100);
        assert!(parse_sample_count("0").is_err());
        assert!(parse_sample_count("-5").is_err());
        assert!(parse_sample_count("abc").is_err());
    }

    #[test]
    fn validate_config_clamps_and_warns() {
        let bad_config = MonitorConfig {
            theme: Some(String::from("neon_glow")),
            default_tab: Some(String::from("settings")),
            cpu_alert: Some(250.0),
            mem_alert: Some(-10.0),
            disk_alert: Some(150),
            ..Default::default()
        };
        let (validated, warnings) = validate_config(bad_config);
        assert!(validated.theme.is_none());
        assert!(validated.default_tab.is_none());
        assert_eq!(validated.cpu_alert, Some(100.0));
        assert_eq!(validated.mem_alert, Some(1.0));
        assert_eq!(validated.disk_alert, Some(100));
        assert_eq!(warnings.len(), 5);
    }
}
