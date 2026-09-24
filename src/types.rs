use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const HISTORY_LIMIT: usize = 120;
pub const SPARKLINE_LIMIT: usize = 30;
pub const EVENT_LIMIT: usize = 100;
pub const REFRESH_INTERVALS_MS: [u64; 5] = [500, 750, 1_000, 2_000, 5_000];
pub const BATTERY_READ_EVERY: u64 = 5;
pub const THERMAL_READ_EVERY: u64 = 3;
pub const SYSTEMD_READ_EVERY: u64 = 8;
pub const STORAGE_HEALTH_READ_EVERY: u64 = 8;
pub const GPU_READ_EVERY: u64 = 4;

pub type NetworkCounters = HashMap<String, (u64, u64)>;
pub type DiskIoCounters = HashMap<String, (u64, u64)>;
pub type GpuRc6Counters = HashMap<String, u64>;

#[derive(Clone, Copy)]
pub struct CpuSample {
    pub idle: u64,
    pub total: u64,
}

#[derive(Clone)]
pub struct CpuSnapshot {
    pub total: CpuSample,
    pub cores: Vec<CpuSample>,
}

pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu_model: String,
    pub cpu_count: usize,
    pub selinux_mode: String,
}

#[derive(Clone)]
pub struct OpenPort {
    pub port: u16,
    pub ip: String,
    pub proto: String,
    pub state: String,
    pub service_name: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Clone)]
pub struct DiskInfo {
    pub mount_point: String,
    pub fs_type: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub free_gb: f64,
    pub pct: u16,
}

pub struct MemInfo {
    pub total_mb: f64,
    pub used_mb: f64,
    pub available_mb: f64,
    pub free_mb: f64,
    pub cached_mb: f64,
    pub buffers_mb: f64,
    pub swap_total_mb: f64,
    pub swap_used_mb: f64,
}

#[derive(Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cpu_pct: f64,
    pub mem_mb: f64,
    pub io_read_bps: f64,
    pub io_write_bps: f64,
    pub threads: u32,
    pub state: String,
    pub reason: String,
    pub is_high_risk: bool,
    pub is_dev: bool,
}

#[derive(Clone, Default, Debug)]
pub struct ProcessDetail {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub state: String,
    pub threads: u32,
    pub uid: u32,
    pub gid: u32,
    pub vm_peak_mb: f64,
    pub vm_size_mb: f64,
    pub vm_rss_mb: f64,
    pub rss_anon_mb: f64,
    pub rss_file_mb: f64,
    pub rss_shmem_mb: f64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub cancelled_write_bytes: u64,
    pub io_read_bps: f64,
    pub io_write_bps: f64,
    pub open_fds: usize,
    pub cwd: String,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
pub struct PsiValues {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
    pub total_us: u64,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
pub struct PsiMetric {
    pub some: PsiValues,
    pub full: Option<PsiValues>,
}

#[derive(Clone, Copy, Default, Debug, Serialize)]
pub struct SystemPsi {
    pub cpu: PsiMetric,
    pub memory: PsiMetric,
    pub io: PsiMetric,
}

#[derive(Clone, Default, Debug, Serialize)]
pub struct HwSensor {
    pub name: String,
    pub label: String,
    pub temp_c: Option<f64>,
    pub fan_rpm: Option<u64>,
    pub crit_c: Option<f64>,
}

pub struct NetInterface {
    pub name: String,
    pub down_bps: f64,
    pub up_bps: f64,
}

pub struct DiskIoInfo {
    pub device: String,
    pub read_bps: f64,
    pub write_bps: f64,
}

#[derive(Clone)]
pub struct GpuInfo {
    pub card: String,
    pub vendor: String,
    pub model: String,
    pub driver: String,
    pub kind: String,
    pub pci_slot: String,
    pub temp_c: Option<f64>,
    pub usage_pct: Option<f64>,
    pub power_w: Option<f64>,
    pub frequency_mhz: Option<u64>,
    pub max_frequency_mhz: Option<u64>,
    pub rc6_residency_ms: Option<u64>,
    pub memory_used_mb: Option<f64>,
    pub memory_total_mb: Option<f64>,
    pub power_state: String,
    pub sensor_source: String,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ProcessPrevStat {
    pub cpu_time: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub struct ProcessSummary {
    pub count: usize,
    pub top_cpu: Vec<ProcessInfo>,
    pub top_mem: Vec<ProcessInfo>,
    pub top_io: Vec<ProcessInfo>,
    pub current_totals: HashMap<u32, ProcessPrevStat>,
    pub zombie_count: usize,
}

#[derive(Clone, Serialize)]
pub struct RootCause {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Serialize)]
pub struct MonitorEvent {
    pub tick: u64,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
}

#[derive(Clone)]
pub struct SystemdUnitIssue {
    pub unit: String,
    #[allow(dead_code)]
    pub load: String,
    pub active: String,
    pub sub: String,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Clone)]
pub struct StorageHealth {
    pub device: String,
    pub model: String,
    pub kind: String,
    pub temp_c: Option<f64>,
    #[allow(dead_code)]
    pub critical_warning: Option<u64>,
    #[allow(dead_code)]
    pub media_errors: Option<u64>,
    pub risk: Severity,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvKind {
    #[default]
    BareMetal,
    VirtualMachine,
    Container,
    Wsl,
}

impl EnvKind {
    pub fn label(&self) -> &'static str {
        match self {
            EnvKind::BareMetal => "Bare metal",
            EnvKind::VirtualMachine => "VM",
            EnvKind::Container => "Container",
            EnvKind::Wsl => "WSL",
        }
    }

    /// Short badge for narrow headers.
    pub fn badge(&self) -> &'static str {
        match self {
            EnvKind::BareMetal => "HW",
            EnvKind::VirtualMachine => "VM",
            EnvKind::Container => "CTR",
            EnvKind::Wsl => "WSL",
        }
    }

    /// Whether uptime/load/net counters describe the host, not us.
    #[allow(dead_code)]
    pub fn is_host_scoped(&self) -> bool {
        matches!(self, EnvKind::Container | EnvKind::Wsl)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewTab {
    Overview,
    Cpu,
    Gpu,
    Memory,
    Storage,
    Network,
    Processes,
}

impl ViewTab {
    pub fn label(&self) -> &'static str {
        match self {
            ViewTab::Overview => "Overview",
            ViewTab::Cpu => "CPU",
            ViewTab::Gpu => "GPU",
            ViewTab::Memory => "Memory",
            ViewTab::Storage => "Storage",
            ViewTab::Network => "Network",
            ViewTab::Processes => "Processes",
        }
    }

    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            ViewTab::Overview => "\u{25a0}",
            ViewTab::Cpu => "\u{2699}",
            ViewTab::Gpu => "\u{25a6}",
            ViewTab::Memory => "\u{25a3}",
            ViewTab::Storage => "\u{25c6}",
            ViewTab::Network => "\u{2194}",
            ViewTab::Processes => "\u{2630}",
        }
    }

    pub fn all() -> &'static [ViewTab] {
        &[
            ViewTab::Overview,
            ViewTab::Cpu,
            ViewTab::Gpu,
            ViewTab::Memory,
            ViewTab::Storage,
            ViewTab::Network,
            ViewTab::Processes,
        ]
    }

    pub fn index(&self) -> usize {
        ViewTab::all().iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn prev(&self) -> Self {
        match self {
            ViewTab::Overview => ViewTab::Processes,
            ViewTab::Cpu => ViewTab::Overview,
            ViewTab::Gpu => ViewTab::Cpu,
            ViewTab::Memory => ViewTab::Gpu,
            ViewTab::Storage => ViewTab::Memory,
            ViewTab::Network => ViewTab::Storage,
            ViewTab::Processes => ViewTab::Network,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ViewTab::Overview => ViewTab::Cpu,
            ViewTab::Cpu => ViewTab::Gpu,
            ViewTab::Gpu => ViewTab::Memory,
            ViewTab::Memory => ViewTab::Storage,
            ViewTab::Storage => ViewTab::Network,
            ViewTab::Network => ViewTab::Processes,
            ViewTab::Processes => ViewTab::Overview,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessSort {
    CpuDesc,
    CpuAsc,
    MemDesc,
    MemAsc,
    IoDesc,
    IoAsc,
    PidAsc,
    PidDesc,
}

impl ProcessSort {
    pub fn cycle(&mut self) {
        *self = match self {
            ProcessSort::CpuDesc => ProcessSort::CpuAsc,
            ProcessSort::CpuAsc => ProcessSort::MemDesc,
            ProcessSort::MemDesc => ProcessSort::MemAsc,
            ProcessSort::MemAsc => ProcessSort::IoDesc,
            ProcessSort::IoDesc => ProcessSort::IoAsc,
            ProcessSort::IoAsc => ProcessSort::PidAsc,
            ProcessSort::PidAsc => ProcessSort::PidDesc,
            ProcessSort::PidDesc => ProcessSort::CpuDesc,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ProcessSort::CpuDesc => "CPU \u{2193}",
            ProcessSort::CpuAsc => "CPU \u{2191}",
            ProcessSort::MemDesc => "MEM \u{2193}",
            ProcessSort::MemAsc => "MEM \u{2191}",
            ProcessSort::IoDesc => "DISK I/O \u{2193}",
            ProcessSort::IoAsc => "DISK I/O \u{2191}",
            ProcessSort::PidAsc => "PID \u{2191}",
            ProcessSort::PidDesc => "PID \u{2193}",
        }
    }
}

#[derive(Deserialize, Default)]
pub struct MonitorConfig {
    pub refresh_interval: Option<String>,
    pub default_tab: Option<String>,
    pub theme: Option<String>,
    pub cpu_alert: Option<f64>,
    pub mem_alert: Option<f64>,
    pub disk_alert: Option<u16>,
    pub temp_alert: Option<f64>,
    pub battery_alert: Option<u16>,
    pub swap_alert: Option<f64>,
}

pub struct AppConfig {
    pub refresh_index: usize,
    pub config: MonitorConfig,
    pub json_once: bool,
    pub json_pretty: bool,
    pub json_lines: bool,
    pub watch_json: bool,
    pub samples: Option<u64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_index: 1,
            config: MonitorConfig::default(),
            json_once: false,
            json_pretty: false,
            json_lines: false,
            watch_json: false,
            samples: None,
        }
    }
}

#[derive(Serialize)]
pub struct MonitorSnapshot {
    pub timestamp: String,
    pub hostname: String,
    pub kernel: String,
    pub os: String,
    pub uptime: String,
    pub cpu_usage_pct: f64,
    pub core_count: usize,
    pub memory_used_mb: f64,
    pub memory_total_mb: f64,
    pub memory_used_pct: f64,
    pub swap_used_mb: f64,
    pub swap_total_mb: f64,
    pub disk_used_pct: u16,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub net_down_bps: f64,
    pub net_up_bps: f64,
    pub battery_pct: Option<u16>,
    pub temperature_c: Option<f64>,
    pub health_score: u16,
    pub health_status: String,
    pub process_count: usize,
    pub top_cpu_processes: Vec<ProcessInfo>,
    pub top_mem_processes: Vec<ProcessInfo>,
    pub top_io_processes: Vec<ProcessInfo>,
    pub alerts: Vec<String>,
    pub root_causes: Vec<RootCause>,
    pub recent_events: Vec<MonitorEvent>,
    pub sample_status: String,
    pub environment: String,
    pub security_mode: String,
    pub psi: Option<SystemPsi>,
}

/// Severity level used for color + symbol cues (accessibility-friendly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Ok,
    Warn,
    Critical,
    #[allow(dead_code)]
    Neutral,
}

impl Severity {
    pub fn from_usage(pct: f64) -> Self {
        if pct >= 85.0 {
            Severity::Critical
        } else if pct >= 70.0 {
            Severity::Warn
        } else {
            Severity::Ok
        }
    }

    pub fn from_health(score: f64) -> Self {
        if score >= 80.0 {
            Severity::Ok
        } else if score >= 55.0 {
            Severity::Warn
        } else {
            Severity::Critical
        }
    }

    /// Non-color glyph cue so color-blind users still see severity.
    pub fn symbol(&self) -> &'static str {
        match self {
            Severity::Ok => "\u{25cf}",       // ●
            Severity::Warn => "\u{25b2}",     // ▲
            Severity::Critical => "\u{25a0}", // ■
            Severity::Neutral => "\u{25cb}",  // ○
        }
    }
}
