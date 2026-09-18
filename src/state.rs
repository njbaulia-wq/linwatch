use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::collector;
use crate::types::*;

pub struct AppState {
    pub system: SystemInfo,
    pub env: EnvKind,
    pub cpu_usage: f64,
    pub core_usages: Vec<f64>,
    pub cpu_history: VecDeque<(f64, f64)>,
    pub previous_cpu: Option<CpuSnapshot>,
    pub mem_total: f64,
    pub mem_used: f64,
    pub mem_available: f64,
    pub mem_free: f64,
    pub mem_cached: f64,
    pub mem_buffers: f64,
    pub mem_history: VecDeque<(f64, f64)>,
    pub swap_total: f64,
    pub swap_used: f64,
    pub disk: DiskInfo,
    pub mounts: Vec<DiskInfo>,
    pub uptime: String,
    pub load_avg: [String; 3],
    pub battery_pct: Option<u16>,
    pub battery_status: String,
    pub net_down_bps: f64,
    pub net_up_bps: f64,
    pub net_down_history: VecDeque<(f64, f64)>,
    pub net_up_history: VecDeque<(f64, f64)>,
    pub interfaces: Vec<NetInterface>,
    pub previous_net: Option<(NetworkCounters, Instant)>,
    pub disk_io: Vec<DiskIoInfo>,
    pub previous_disk_io: Option<(DiskIoCounters, Instant)>,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
    pub disk_read_history: VecDeque<(f64, f64)>,
    pub disk_write_history: VecDeque<(f64, f64)>,
    pub gpus: Vec<GpuInfo>,
    pub previous_gpu_rc6: Option<(GpuRc6Counters, Instant)>,
    pub gpu_usage_history: VecDeque<(f64, f64)>,
    pub gpu_temp_history: VecDeque<(f64, f64)>,
    pub temp_c: Option<f64>,
    pub temp_history: VecDeque<(f64, f64)>,
    pub process_count: usize,
    pub top_cpu_processes: Vec<ProcessInfo>,
    pub top_mem_processes: Vec<ProcessInfo>,
    pub root_causes: Vec<RootCause>,
    pub failed_units: Vec<SystemdUnitIssue>,
    pub storage_health: Vec<StorageHealth>,
    pub previous_process_totals: HashMap<u32, u64>,
    pub process_sort: ProcessSort,
    pub process_history: HashMap<u32, VecDeque<f64>>,
    pub process_selected: usize,
    pub is_tree_view: bool,
    pub health_score: u16,
    pub alerts: Vec<String>,
    pub successful_reads: u64,
    pub failed_reads: u64,
    pub degraded_sources: Vec<String>,
    pub last_sample_at: Instant,
    pub counter: u64,
    pub show_help: bool,
    pub refresh_index: usize,
    pub active_tab: ViewTab,
    pub tick_count: u64,
    pub terminal_width: u16,
    pub cpu_alert: f64,
    pub mem_alert: f64,
    pub disk_alert: u16,
    pub temp_alert: f64,
    pub battery_alert: u16,
    pub swap_alert: f64,
    pub process_search: String,
    pub is_search_mode: bool,
    pub open_ports: Vec<OpenPort>,
    pub zombie_count: usize,
    pub confirm_kill_pid: Option<u32>,
    pub confirm_kill_name: Option<String>,
    pub process_action_message: Option<String>,
    pub events: VecDeque<MonitorEvent>,
    pub cpu_pressure_ticks: u8,
    pub mem_pressure_ticks: u8,
    pub thermal_pressure_ticks: u8,
    pub previous_sample_status_label: String,
    pub previous_health_band: Severity,
    pub inspect_process_pid: Option<u32>,
    pub inspect_process_detail: Option<ProcessDetail>,
    pub psi: Option<SystemPsi>,
    pub hw_sensors: Vec<HwSensor>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let cfg = &config.config;
        let default_tab = match cfg.default_tab.as_deref() {
            Some("cpu") => ViewTab::Cpu,
            Some("gpu") | Some("igpu") => ViewTab::Gpu,
            Some("memory") | Some("mem") => ViewTab::Memory,
            Some("storage") | Some("disk") => ViewTab::Storage,
            Some("network") | Some("net") => ViewTab::Network,
            Some("processes") | Some("proc") => ViewTab::Processes,
            _ => ViewTab::Overview,
        };

        let mut state = Self {
            system: collector::read_system_info(),
            env: collector::detect_env(),
            cpu_usage: 0.0,
            core_usages: Vec::new(),
            cpu_history: VecDeque::with_capacity(HISTORY_LIMIT),
            previous_cpu: None,
            mem_total: 1.0,
            mem_used: 0.0,
            mem_available: 1.0,
            mem_free: 1.0,
            mem_cached: 0.0,
            mem_buffers: 0.0,
            mem_history: VecDeque::with_capacity(HISTORY_LIMIT),
            swap_total: 0.0,
            swap_used: 0.0,
            disk: DiskInfo {
                mount_point: String::from("/"),
                fs_type: String::from("ext4"),
                used_gb: 0.0,
                total_gb: 0.0,
                free_gb: 0.0,
                pct: 0,
            },
            mounts: Vec::new(),
            uptime: String::from("00:00:00"),
            load_avg: [
                String::from("0.00"),
                String::from("0.00"),
                String::from("0.00"),
            ],
            battery_pct: None,
            battery_status: String::from("N/A"),
            net_down_bps: 0.0,
            net_up_bps: 0.0,
            net_down_history: VecDeque::with_capacity(HISTORY_LIMIT),
            net_up_history: VecDeque::with_capacity(HISTORY_LIMIT),
            interfaces: Vec::new(),
            previous_net: None,
            disk_io: Vec::new(),
            previous_disk_io: None,
            disk_read_bps: 0.0,
            disk_write_bps: 0.0,
            disk_read_history: VecDeque::with_capacity(HISTORY_LIMIT),
            disk_write_history: VecDeque::with_capacity(HISTORY_LIMIT),
            gpus: Vec::new(),
            previous_gpu_rc6: None,
            gpu_usage_history: VecDeque::with_capacity(HISTORY_LIMIT),
            gpu_temp_history: VecDeque::with_capacity(HISTORY_LIMIT),
            temp_c: None,
            temp_history: VecDeque::with_capacity(HISTORY_LIMIT),
            process_count: 0,
            top_cpu_processes: Vec::new(),
            top_mem_processes: Vec::new(),
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: HashMap::new(),
            process_sort: ProcessSort::CpuDesc,
            process_history: HashMap::new(),
            process_selected: 0,
            is_tree_view: false,
            health_score: 100,
            alerts: Vec::new(),
            successful_reads: 0,
            failed_reads: 0,
            degraded_sources: Vec::new(),
            last_sample_at: Instant::now(),
            counter: 0,
            show_help: false,
            refresh_index: config.refresh_index,
            active_tab: default_tab,
            tick_count: 0,
            terminal_width: 120,
            cpu_alert: cfg.cpu_alert.unwrap_or(85.0),
            mem_alert: cfg.mem_alert.unwrap_or(85.0),
            disk_alert: cfg.disk_alert.unwrap_or(85),
            temp_alert: cfg.temp_alert.unwrap_or(80.0),
            battery_alert: cfg.battery_alert.unwrap_or(20),
            swap_alert: cfg.swap_alert.unwrap_or(35.0),
            process_search: String::new(),
            is_search_mode: false,
            open_ports: Vec::new(),
            zombie_count: 0,
            confirm_kill_pid: None,
            confirm_kill_name: None,
            process_action_message: None,
            events: VecDeque::with_capacity(EVENT_LIMIT),
            cpu_pressure_ticks: 0,
            mem_pressure_ticks: 0,
            thermal_pressure_ticks: 0,
            previous_sample_status_label: String::from("OK"),
            previous_health_band: Severity::Ok,
            inspect_process_pid: None,
            inspect_process_detail: None,
            psi: None,
            hw_sensors: Vec::new(),
        };

        state.update();
        state
    }

    pub fn new_raw(config: &AppConfig) -> Self {
        let mut state = Self::new(AppConfig {
            refresh_index: config.refresh_index,
            config: MonitorConfig {
                refresh_interval: None,
                default_tab: None,
                theme: config.config.theme.clone(),
                cpu_alert: config.config.cpu_alert,
                mem_alert: config.config.mem_alert,
                disk_alert: config.config.disk_alert,
                temp_alert: config.config.temp_alert,
                battery_alert: config.config.battery_alert,
                swap_alert: config.config.swap_alert,
            },
            json_once: false,
            json_pretty: false,
            json_lines: false,
            watch_json: false,
            samples: None,
        });
        std::thread::sleep(state.refresh_rate().min(Duration::from_millis(750)));
        state.update();
        state
    }

    pub fn to_snapshot(&self) -> MonitorSnapshot {
        use std::time::UNIX_EPOCH;
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        MonitorSnapshot {
            timestamp: now.to_string(),
            hostname: self.system.hostname.clone(),
            kernel: self.system.kernel.clone(),
            os: format!("{} {}", self.system.os_name, self.system.os_version),
            uptime: self.uptime.clone(),
            cpu_usage_pct: self.cpu_usage,
            core_count: self.system.cpu_count,
            memory_used_mb: self.mem_used,
            memory_total_mb: self.mem_total,
            memory_used_pct: self.mem_pct(),
            swap_used_mb: self.swap_used,
            swap_total_mb: self.swap_total,
            disk_used_pct: self.disk.pct,
            disk_used_gb: self.disk.used_gb,
            disk_total_gb: self.disk.total_gb,
            net_down_bps: self.net_down_bps,
            net_up_bps: self.net_up_bps,
            battery_pct: self.battery_pct,
            temperature_c: self.temp_c,
            health_score: self.health_score,
            health_status: match crate::types::Severity::from_health(self.health_score as f64) {
                Severity::Ok => "Healthy",
                Severity::Warn => "Attention",
                Severity::Critical => "Critical",
                Severity::Neutral => "Monitoring",
            }
            .to_string(),
            process_count: self.process_count,
            top_cpu_processes: self.top_cpu_processes.clone(),
            top_mem_processes: self.top_mem_processes.clone(),
            alerts: self.alerts.clone(),
            root_causes: self.root_causes.clone(),
            recent_events: self.events.iter().rev().take(20).cloned().collect(),
            sample_status: self.sample_status().to_string(),
            environment: self.env.label().to_string(),
            security_mode: self.system.selinux_mode.clone(),
            psi: self.psi,
        }
    }

    #[allow(dead_code)]
    pub fn platform_summary(&self) -> String {
        if self.env.is_host_scoped() {
            format!(
                "{} ({} scope: host)",
                self.env.label(),
                match self.env {
                    EnvKind::Container => "ctr",
                    _ => "wsl",
                }
            )
        } else {
            self.env.label().to_string()
        }
    }

    pub fn refresh_rate(&self) -> Duration {
        Duration::from_millis(REFRESH_INTERVALS_MS[self.refresh_index])
    }

    pub fn refresh_label(&self) -> String {
        let ms = REFRESH_INTERVALS_MS[self.refresh_index];
        if ms >= 1_000 {
            format!("{:.1}s", ms as f64 / 1_000.0)
        } else {
            format!("{ms}ms")
        }
    }

    pub fn faster_refresh(&mut self) {
        self.refresh_index = self.refresh_index.saturating_sub(1);
    }

    pub fn slower_refresh(&mut self) {
        self.refresh_index = (self.refresh_index + 1).min(REFRESH_INTERVALS_MS.len() - 1);
    }

    pub fn sample_status(&self) -> &'static str {
        if self.tick_count <= 1 {
            return "Warming up";
        }
        match self.degraded_sources.len() {
            0 => "OK",
            1 | 2 => "Partial",
            _ => "Degraded",
        }
    }

    pub fn mem_pct(&self) -> f64 {
        (self.mem_used / self.mem_total * 100.0).clamp(0.0, 100.0)
    }

    pub fn swap_pct(&self) -> f64 {
        if self.swap_total <= 0.0 {
            0.0
        } else {
            (self.swap_used / self.swap_total * 100.0).clamp(0.0, 100.0)
        }
    }

    pub fn process_select_next(&mut self) {
        let len = self.filtered_processes().len();
        if len == 0 {
            self.process_selected = 0;
        } else {
            self.process_selected = (self.process_selected + 1) % len;
        }
    }

    pub fn process_select_prev(&mut self) {
        let len = self.filtered_processes().len();
        if len == 0 {
            self.process_selected = 0;
        } else if self.process_selected == 0 {
            self.process_selected = len - 1;
        } else {
            self.process_selected -= 1;
        }
    }

    pub fn toggle_tree_view(&mut self) {
        self.is_tree_view = !self.is_tree_view;
        self.process_selected = 0;
    }

    pub fn filtered_process_tree(&self) -> Vec<(&ProcessInfo, String)> {
        if !self.is_tree_view {
            let mut procs: Vec<&ProcessInfo> = match self.process_sort {
                ProcessSort::CpuDesc | ProcessSort::CpuAsc => {
                    let mut v = Vec::with_capacity(self.top_cpu_processes.len());
                    v.extend(self.top_cpu_processes.iter());
                    v
                }
                ProcessSort::MemDesc | ProcessSort::MemAsc => {
                    let mut v = Vec::with_capacity(self.top_mem_processes.len());
                    v.extend(self.top_mem_processes.iter());
                    v
                }
                ProcessSort::PidAsc | ProcessSort::PidDesc => {
                    let mut seen = HashSet::with_capacity(
                        self.top_cpu_processes.len() + self.top_mem_processes.len(),
                    );
                    let mut v = Vec::with_capacity(seen.capacity());
                    v.extend(
                        self.top_cpu_processes
                            .iter()
                            .chain(self.top_mem_processes.iter())
                            .filter(|p| seen.insert(p.pid)),
                    );
                    v
                }
            };

            if !self.process_search.is_empty() {
                let search = self.process_search.to_lowercase();
                let search_lower = search.as_str();
                procs.retain(|p| {
                    contains_case_insensitive(&p.name, search_lower)
                        || pid_contains(p.pid, search_lower)
                });
            }
            self.sort_processes(&mut procs);
            return procs.into_iter().map(|p| (p, String::new())).collect();
        }

        let mut seen =
            HashSet::with_capacity(self.top_cpu_processes.len() + self.top_mem_processes.len());
        let mut procs: Vec<&ProcessInfo> = Vec::with_capacity(seen.capacity());
        procs.extend(
            self.top_cpu_processes
                .iter()
                .chain(self.top_mem_processes.iter())
                .filter(|p| seen.insert(p.pid)),
        );

        if !self.process_search.is_empty() {
            let search = self.process_search.to_lowercase();
            let search_lower = search.as_str();
            procs.retain(|p| {
                contains_case_insensitive(&p.name, search_lower)
                    || pid_contains(p.pid, search_lower)
            });
        }
        self.sort_processes(&mut procs);

        let pid_set: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
        let mut children_map: HashMap<u32, Vec<&ProcessInfo>> = HashMap::new();
        let mut roots: Vec<&ProcessInfo> = Vec::new();

        for p in &procs {
            if p.ppid == 0 || p.ppid == p.pid || !pid_set.contains(&p.ppid) {
                roots.push(*p);
            } else {
                children_map.entry(p.ppid).or_default().push(*p);
            }
        }

        let mut result = Vec::with_capacity(procs.len());
        let mut visited = HashSet::with_capacity(procs.len());

        for root in roots {
            dfs_tree(
                root,
                "",
                false,
                true,
                &children_map,
                &mut visited,
                &mut result,
            );
        }

        for p in procs {
            if visited.insert(p.pid) {
                result.push((p, String::new()));
            }
        }

        result
    }

    fn sort_processes(&self, procs: &mut [&ProcessInfo]) {
        match self.process_sort {
            ProcessSort::CpuDesc => procs.sort_unstable_by(|a, b| {
                b.cpu_pct
                    .partial_cmp(&a.cpu_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ProcessSort::CpuAsc => procs.sort_unstable_by(|a, b| {
                a.cpu_pct
                    .partial_cmp(&b.cpu_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ProcessSort::MemDesc => procs.sort_unstable_by(|a, b| {
                b.mem_mb
                    .partial_cmp(&a.mem_mb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ProcessSort::MemAsc => procs.sort_unstable_by(|a, b| {
                a.mem_mb
                    .partial_cmp(&b.mem_mb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ProcessSort::PidAsc => procs.sort_unstable_by_key(|p| p.pid),
            ProcessSort::PidDesc => procs.sort_unstable_by_key(|p| std::cmp::Reverse(p.pid)),
        }
    }

    pub fn filtered_processes(&self) -> Vec<&ProcessInfo> {
        self.filtered_process_tree()
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    }

    pub fn update(&mut self) {
        self.counter += 1;
        self.tick_count += 1;
        self.degraded_sources.clear();

        let mut cpu_delta = None;
        if let Some(current) = self.capture("cpu", collector::read_cpu_snapshot()) {
            if let Some(previous) = self.previous_cpu.as_ref() {
                self.cpu_usage = calculate_cpu_usage(previous.total, current.total);
                self.core_usages = previous
                    .cores
                    .iter()
                    .zip(current.cores.iter())
                    .map(|(p, c)| calculate_cpu_usage(*p, *c))
                    .collect();
                cpu_delta = Some(current.total.total.saturating_sub(previous.total.total));
            }
            self.previous_cpu = Some(current);
        }
        push_history(&mut self.cpu_history, self.counter, self.cpu_usage);

        if let Some(mem) = self.capture("memory", collector::read_mem_info()) {
            self.mem_total = mem.total_mb.max(1.0);
            self.mem_used = mem.used_mb.max(0.0);
            self.mem_available = mem.available_mb.max(0.0);
            self.mem_free = mem.free_mb.max(0.0);
            self.mem_cached = mem.cached_mb.max(0.0);
            self.mem_buffers = mem.buffers_mb.max(0.0);
            self.swap_total = mem.swap_total_mb.max(0.0);
            self.swap_used = mem.swap_used_mb.max(0.0);
        }
        let mem_pct_val = self.mem_pct();
        push_history(&mut self.mem_history, self.counter, mem_pct_val);

        if let Some(mounts) = self.capture("disk", collector::read_disk_info()) {
            self.disk = collector::primary_mount(&mounts)
                .cloned()
                .unwrap_or_else(empty_disk_info);
            self.mounts = mounts;
        }

        self.update_disk_io_rates();

        if let Some(uptime) = self.capture("uptime", collector::read_uptime()) {
            self.uptime = uptime;
        }
        if let Some(load_avg) = self.capture("loadavg", collector::read_load_average()) {
            self.load_avg = load_avg;
        }

        if self.tick_count == 1 || self.tick_count.is_multiple_of(BATTERY_READ_EVERY) {
            let (battery_pct, battery_status) = collector::read_battery();
            self.battery_pct = battery_pct;
            self.battery_status = battery_status;
        }
        if self.tick_count == 1 || self.tick_count.is_multiple_of(THERMAL_READ_EVERY) {
            self.temp_c = collector::read_temperature();
        }
        if let Some(temp) = self.temp_c {
            push_history(&mut self.temp_history, self.counter, temp);
        }
        if self.tick_count == 1 || self.tick_count.is_multiple_of(GPU_READ_EVERY) {
            self.gpus = self
                .capture("gpu", collector::read_gpu_info())
                .unwrap_or_default();
            self.update_gpu_usage_from_rc6();
        }
        let primary_gpu_sample = self.primary_gpu().map(|gpu| (gpu.usage_pct, gpu.temp_c));
        if let Some((usage_pct, temp_c)) = primary_gpu_sample {
            if let Some(usage) = usage_pct {
                push_history(&mut self.gpu_usage_history, self.counter, usage);
            }
            if let Some(temp) = temp_c {
                push_history(&mut self.gpu_temp_history, self.counter, temp);
            }
        }
        if self.tick_count == 1 || self.tick_count.is_multiple_of(SYSTEMD_READ_EVERY) {
            // Best-effort: absence of systemd (Alpine/OpenRC, containers,
            // WSL1) means "skip", never a per-tick sample failure.
            if collector::has_systemd() {
                self.failed_units = self
                    .capture("systemd", collector::read_systemd_failed_units(5))
                    .unwrap_or_default();
            } else {
                self.failed_units.clear();
            }
            self.open_ports = collector::read_open_ports().unwrap_or_default();
        }
        if self.tick_count == 1 || self.tick_count.is_multiple_of(STORAGE_HEALTH_READ_EVERY) {
            self.storage_health = self
                .capture("storage_health", collector::read_storage_health())
                .unwrap_or_default();
        }

        self.psi = collector::read_psi();
        if self.tick_count == 1 || self.tick_count.is_multiple_of(THERMAL_READ_EVERY) {
            self.hw_sensors = collector::read_hw_sensors();
        }
        if let Some(pid) = self.inspect_process_pid {
            self.refresh_inspected_process(pid);
        }

        if let Some(mut summary) = self.capture(
            "processes",
            collector::read_process_summary(
                30,
                &self.previous_process_totals,
                cpu_delta.unwrap_or(0),
            ),
        ) {
            self.process_count = summary.count;
            self.zombie_count = summary.zombie_count;

            for p in &mut summary.top_cpu {
                p.is_high_risk = p.cpu_pct > 90.0;
                let hist = self
                    .process_history
                    .entry(p.pid)
                    .or_insert_with(|| VecDeque::with_capacity(SPARKLINE_LIMIT));
                hist.push_back(p.cpu_pct);
                while hist.len() > SPARKLINE_LIMIT {
                    hist.pop_front();
                }
            }
            for p in &mut summary.top_mem {
                p.is_high_risk = p.cpu_pct > 90.0;
            }

            self.sort_and_truncate_processes(&mut summary.top_cpu, &mut summary.top_mem, 10);
            // Reuse the previous map's allocation instead of dropping it every
            // tick (validated: HashMap::clear retains capacity for reuse).
            self.previous_process_totals.clear();
            self.previous_process_totals
                .extend(summary.current_totals.drain());
        }

        // Proactively evict sparkline history for PIDs no longer in any top list.
        // Prevents unbounded HashMap growth on systems with many short-lived processes.
        {
            let active: HashSet<u32> = self
                .top_cpu_processes
                .iter()
                .chain(self.top_mem_processes.iter())
                .map(|p| p.pid)
                .collect();
            self.process_history.retain(|pid, _| active.contains(pid));
        }

        self.update_network_rates();
        self.update_sustained_pressure();
        self.alerts = build_alerts(self);
        self.root_causes = build_root_causes(self);
        self.health_score = calculate_health_score(self);
        self.record_state_events();
        self.last_sample_at = Instant::now();

        // Clamp selected process index to current list.
        let filtered_len = self.filtered_processes().len();
        if self.process_selected >= filtered_len {
            self.process_selected = filtered_len.saturating_sub(1);
        }
    }

    fn sort_and_truncate_processes(
        &mut self,
        top_cpu: &mut Vec<ProcessInfo>,
        top_mem: &mut Vec<ProcessInfo>,
        limit: usize,
    ) {
        top_cpu.sort_unstable_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_cpu.truncate(limit);
        top_mem.sort_unstable_by(|a, b| {
            b.mem_mb
                .partial_cmp(&a.mem_mb)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_mem.truncate(limit);
        self.top_cpu_processes = std::mem::take(top_cpu);
        self.top_mem_processes = std::mem::take(top_mem);
    }

    fn capture<T>(&mut self, source: &str, value: Option<T>) -> Option<T> {
        if value.is_some() {
            self.successful_reads += 1;
        } else {
            self.failed_reads += 1;
            self.degraded_sources.push(source.to_string());
        }
        value
    }

    fn update_network_rates(&mut self) {
        if let Some(current) = self.capture("network", collector::read_network_totals()) {
            let now = Instant::now();
            if let Some((previous, prev_time)) = &self.previous_net {
                let elapsed = now.duration_since(*prev_time).as_secs_f64().max(0.1);
                self.interfaces = current
                    .iter()
                    .map(|(name, &(rx, tx))| {
                        let (prev_rx, prev_tx) = previous.get(name).copied().unwrap_or((rx, tx));
                        NetInterface {
                            name: name.clone(),
                            down_bps: normalize_rate(
                                ((rx.saturating_sub(prev_rx)) as f64) / elapsed,
                            ),
                            up_bps: normalize_rate(((tx.saturating_sub(prev_tx)) as f64) / elapsed),
                        }
                    })
                    .collect();
            }
            self.net_down_bps = normalize_rate(self.interfaces.iter().map(|i| i.down_bps).sum());
            self.net_up_bps = normalize_rate(self.interfaces.iter().map(|i| i.up_bps).sum());
            push_history(&mut self.net_down_history, self.counter, self.net_down_bps);
            push_history(&mut self.net_up_history, self.counter, self.net_up_bps);
            self.previous_net = Some((current, now));
        }
    }

    fn update_disk_io_rates(&mut self) {
        if let Some(current) = self.capture("disk_io", collector::read_disk_io_totals()) {
            let now = Instant::now();
            if let Some((previous, prev_time)) = &self.previous_disk_io {
                let elapsed = now.duration_since(*prev_time).as_secs_f64().max(0.1);
                self.disk_io = current
                    .iter()
                    .map(|(name, &(rd, wr))| {
                        let (prev_rd, prev_wr) = previous.get(name).copied().unwrap_or((rd, wr));
                        DiskIoInfo {
                            device: name.clone(),
                            read_bps: normalize_rate(
                                ((rd.saturating_sub(prev_rd)) as f64 * 512.0) / elapsed,
                            ),
                            write_bps: normalize_rate(
                                ((wr.saturating_sub(prev_wr)) as f64 * 512.0) / elapsed,
                            ),
                        }
                    })
                    .collect();

                let total_read: f64 = self.disk_io.iter().map(|d| d.read_bps).sum();
                let total_write: f64 = self.disk_io.iter().map(|d| d.write_bps).sum();
                self.disk_read_bps = total_read;
                self.disk_write_bps = total_write;
                push_history(&mut self.disk_read_history, self.counter, total_read);
                push_history(&mut self.disk_write_history, self.counter, total_write);
            }
            self.previous_disk_io = Some((current, now));
        }
    }

    fn update_gpu_usage_from_rc6(&mut self) {
        let now = Instant::now();
        let current: GpuRc6Counters = self
            .gpus
            .iter()
            .filter_map(|gpu| gpu.rc6_residency_ms.map(|rc6| (gpu.card.clone(), rc6)))
            .collect();

        if let Some((previous, prev_time)) = &self.previous_gpu_rc6 {
            let elapsed_ms = now.duration_since(*prev_time).as_secs_f64() * 1000.0;
            if elapsed_ms > 0.0 {
                for gpu in &mut self.gpus {
                    let Some(current_rc6) = gpu.rc6_residency_ms else {
                        continue;
                    };
                    let Some(previous_rc6) = previous.get(&gpu.card) else {
                        continue;
                    };
                    let rc6_delta = current_rc6.saturating_sub(*previous_rc6) as f64;
                    gpu.usage_pct =
                        Some((100.0 - (rc6_delta / elapsed_ms * 100.0)).clamp(0.0, 100.0));
                }
            }
        }

        if !current.is_empty() {
            self.previous_gpu_rc6 = Some((current, now));
        }
    }

    pub fn request_kill(&mut self) {
        self.request_kill_signal(libc::SIGTERM);
    }

    pub fn request_kill_signal(&mut self, signal: i32) {
        let own_pid = std::process::id();
        let procs = self.filtered_processes();
        if let Some(p) = procs
            .get(self.process_selected)
            .map(|p| (p.pid, p.name.clone()))
        {
            // Refuse to kill PID 1 (init/systemd) or our own process.
            if p.0 <= 1 {
                self.process_action_message =
                    Some(String::from("Refusing to kill PID 1 (init/systemd)"));
                return;
            }
            if p.0 == own_pid {
                self.process_action_message =
                    Some(String::from("Refusing to kill own process (linwatch)"));
                return;
            }

            match self.confirm_kill_pid {
                Some(pid) if pid == p.0 => {
                    let sig_label = if signal == libc::SIGKILL {
                        "SIGKILL (9)"
                    } else {
                        "SIGTERM (15)"
                    };
                    match self.execute_kill(p.0, signal) {
                        Ok(()) => {
                            self.process_action_message =
                                Some(format!("Sent {} to PID {} ({})", sig_label, p.0, p.1));
                        }
                        Err(err) => {
                            self.process_action_message =
                                Some(format!("Could not terminate PID {}: {}", p.0, err));
                        }
                    }
                    self.confirm_kill_pid = None;
                    self.confirm_kill_name = None;
                }
                _ => {
                    self.confirm_kill_pid = Some(p.0);
                    self.confirm_kill_name = Some(p.1);
                    self.process_action_message = Some(format!(
                        "Terminate PID {}? [K/Enter] SIGTERM, [9] SIGKILL, [Esc] Cancel",
                        p.0
                    ));
                }
            }
        }
    }

    pub fn cancel_kill(&mut self) {
        self.confirm_kill_pid = None;
        self.confirm_kill_name = None;
        self.process_action_message = Some(String::from("Process action cancelled"));
    }

    fn execute_kill(&mut self, pid: u32, signal: i32) -> std::io::Result<()> {
        // SAFETY: libc::kill is a POSIX syscall that sends a signal to a process.
        // The pid is validated before calling (must be > 1 and not our own PID).
        // A return value of 0 means success; non-zero means error, caught by last_os_error.
        let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn open_inspector(&mut self, pid: u32) {
        let name = self
            .top_cpu_processes
            .iter()
            .chain(self.top_mem_processes.iter())
            .find(|p| p.pid == pid)
            .map(|p| p.name.as_str())
            .unwrap_or("process");
        self.inspect_process_detail = collector::read_process_detail(pid, name);
        self.inspect_process_pid = Some(pid);
    }

    pub fn close_inspector(&mut self) {
        self.inspect_process_pid = None;
        self.inspect_process_detail = None;
    }

    pub fn refresh_inspected_process(&mut self, pid: u32) {
        let name = self
            .inspect_process_detail
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_default();
        if let Some(detail) = collector::read_process_detail(pid, &name) {
            self.inspect_process_detail = Some(detail);
        }
    }

    pub fn send_inspected_signal(&mut self, signal: i32) {
        if let Some(pid) = self.inspect_process_pid {
            let res = self.execute_kill(pid, signal);
            match res {
                Ok(()) => {
                    let sig_name = match signal {
                        libc::SIGTERM => "SIGTERM",
                        libc::SIGKILL => "SIGKILL",
                        libc::SIGSTOP => "SIGSTOP",
                        libc::SIGCONT => "SIGCONT",
                        _ => "Signal",
                    };
                    self.process_action_message = Some(format!("{sig_name} sent to PID {pid}"));
                    if signal == libc::SIGKILL || signal == libc::SIGTERM {
                        self.close_inspector();
                    }
                }
                Err(err) => {
                    self.process_action_message =
                        Some(format!("Failed to signal PID {pid}: {err}"));
                }
            }
        }
    }

    pub fn primary_gpu(&self) -> Option<&GpuInfo> {
        self.gpus
            .iter()
            .find(|gpu| gpu.kind == "iGPU")
            .or_else(|| self.gpus.first())
    }

    #[cfg(test)]
    #[must_use]
    pub fn test_state() -> Self {
        Self {
            system: SystemInfo {
                os_name: String::new(),
                os_version: String::new(),
                kernel: String::new(),
                hostname: String::new(),
                cpu_model: String::new(),
                cpu_count: 1,
                selinux_mode: String::new(),
            },
            env: EnvKind::BareMetal,
            cpu_usage: 0.0,
            core_usages: Vec::new(),
            cpu_history: std::collections::VecDeque::new(),
            previous_cpu: None,
            mem_total: 1.0,
            mem_used: 0.0,
            mem_available: 1.0,
            mem_free: 1.0,
            mem_cached: 0.0,
            mem_buffers: 0.0,
            mem_history: std::collections::VecDeque::new(),
            swap_total: 0.0,
            swap_used: 0.0,
            disk: DiskInfo {
                mount_point: "/".into(),
                fs_type: "ext4".into(),
                used_gb: 0.0,
                total_gb: 1.0,
                free_gb: 1.0,
                pct: 0,
            },
            mounts: Vec::new(),
            uptime: String::new(),
            load_avg: [String::new(), String::new(), String::new()],
            battery_pct: None,
            battery_status: String::new(),
            net_down_bps: 0.0,
            net_up_bps: 0.0,
            net_down_history: std::collections::VecDeque::new(),
            net_up_history: std::collections::VecDeque::new(),
            interfaces: Vec::new(),
            previous_net: None,
            disk_io: Vec::new(),
            previous_disk_io: None,
            disk_read_bps: 0.0,
            disk_write_bps: 0.0,
            disk_read_history: std::collections::VecDeque::new(),
            disk_write_history: std::collections::VecDeque::new(),
            gpus: Vec::new(),
            previous_gpu_rc6: None,
            gpu_usage_history: std::collections::VecDeque::new(),
            gpu_temp_history: std::collections::VecDeque::new(),
            temp_c: None,
            temp_history: std::collections::VecDeque::new(),
            process_count: 0,
            top_cpu_processes: Vec::new(),
            top_mem_processes: Vec::new(),
            root_causes: Vec::new(),
            failed_units: Vec::new(),
            storage_health: Vec::new(),
            previous_process_totals: std::collections::HashMap::new(),
            process_sort: ProcessSort::CpuDesc,
            process_history: std::collections::HashMap::new(),
            process_selected: 0,
            is_tree_view: false,
            health_score: 100,
            alerts: Vec::new(),
            successful_reads: 0,
            failed_reads: 0,
            degraded_sources: Vec::new(),
            last_sample_at: std::time::Instant::now(),
            counter: 0,
            show_help: false,
            refresh_index: 1,
            active_tab: ViewTab::Overview,
            tick_count: 0,
            terminal_width: 120,
            cpu_alert: 85.0,
            mem_alert: 85.0,
            disk_alert: 85,
            temp_alert: 80.0,
            battery_alert: 20,
            swap_alert: 35.0,
            process_search: String::new(),
            is_search_mode: false,
            open_ports: Vec::new(),
            zombie_count: 0,
            confirm_kill_pid: None,
            confirm_kill_name: None,
            process_action_message: None,
            events: std::collections::VecDeque::new(),
            cpu_pressure_ticks: 0,
            mem_pressure_ticks: 0,
            thermal_pressure_ticks: 0,
            previous_sample_status_label: String::from("OK"),
            previous_health_band: Severity::Ok,
            inspect_process_pid: None,
            inspect_process_detail: None,
            psi: None,
            hw_sensors: Vec::new(),
        }
    }

    fn update_sustained_pressure(&mut self) {
        self.cpu_pressure_ticks =
            pressure_tick(self.cpu_pressure_ticks, self.cpu_usage >= self.cpu_alert);
        self.mem_pressure_ticks =
            pressure_tick(self.mem_pressure_ticks, self.mem_pct() >= self.mem_alert);
        self.thermal_pressure_ticks = pressure_tick(
            self.thermal_pressure_ticks,
            self.temp_c.is_some_and(|temp| temp >= self.temp_alert),
        );
    }

    fn record_state_events(&mut self) {
        if self.cpu_pressure_ticks == 3 {
            let detail = self
                .top_cpu_processes
                .first()
                .map(|p| {
                    format!(
                        "CPU stayed above threshold; top process {} PID {} at {:.1}%",
                        p.name, p.pid, p.cpu_pct
                    )
                })
                .unwrap_or_else(|| String::from("CPU stayed above threshold"));
            self.push_event(Severity::Critical, "Sustained CPU pressure", detail);
        }

        if self.mem_pressure_ticks == 3 {
            let detail = self
                .top_mem_processes
                .first()
                .map(|p| {
                    format!(
                        "Memory stayed above threshold; top process {} PID {} uses {:.1} MB",
                        p.name, p.pid, p.mem_mb
                    )
                })
                .unwrap_or_else(|| String::from("Memory stayed above threshold"));
            self.push_event(Severity::Critical, "Sustained memory pressure", detail);
        }

        if self.thermal_pressure_ticks == 3 {
            self.push_event(
                Severity::Critical,
                "Sustained thermal pressure",
                format!(
                    "Temperature stayed above threshold at {}",
                    self.temp_c
                        .map(|temp| format!("{temp:.1}C"))
                        .unwrap_or_else(|| String::from("unknown"))
                ),
            );
        }

        if self.zombie_count > 0 && self.tick_count == 1 {
            self.push_event(
                Severity::Warn,
                "Zombie processes detected",
                format!("{} zombie processes are present", self.zombie_count),
            );
        }

        if !self.failed_units.is_empty()
            && (self.tick_count == 1 || self.tick_count.is_multiple_of(SYSTEMD_READ_EVERY))
        {
            self.push_event(
                Severity::Critical,
                "Failed systemd units",
                format!(
                    "{} failed unit(s), first: {}",
                    self.failed_units.len(),
                    self.failed_units[0].unit
                ),
            );
        }

        if let Some(drive) = self
            .storage_health
            .iter()
            .find(|drive| drive.risk != Severity::Ok)
        {
            if self.tick_count == 1 || self.tick_count.is_multiple_of(STORAGE_HEALTH_READ_EVERY) {
                self.push_event(
                    drive.risk,
                    "Storage health warning",
                    format!("{}: {}", drive.device, drive.note),
                );
            }
        }

        let sample_status = self.sample_status().to_string();
        if sample_status != self.previous_sample_status_label {
            let severity = match sample_status.as_str() {
                "OK" => Severity::Ok,
                "Partial" => Severity::Warn,
                _ => Severity::Critical,
            };
            let detail = if self.degraded_sources.is_empty() {
                format!("{} -> {}", self.previous_sample_status_label, sample_status)
            } else {
                format!(
                    "{} -> {} ({})",
                    self.previous_sample_status_label,
                    sample_status,
                    self.degraded_sources.join(", ")
                )
            };
            self.push_event(severity, "Sample quality changed", detail);
            self.previous_sample_status_label = sample_status;
        }

        let health_band = Severity::from_health(self.health_score as f64);
        if health_band != self.previous_health_band {
            self.push_event(
                health_band,
                "Health band changed",
                format!(
                    "{} -> {} at {}%",
                    severity_label_for_event(self.previous_health_band),
                    severity_label_for_event(health_band),
                    self.health_score
                ),
            );
            self.previous_health_band = health_band;
        }
    }

    fn push_event(
        &mut self,
        severity: Severity,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let title = title.into();
        let detail = detail.into();
        if self
            .events
            .back()
            .is_some_and(|event| event.title == title && event.detail == detail)
        {
            return;
        }
        if self.events.len() >= EVENT_LIMIT {
            self.events.pop_front();
        }
        self.events.push_back(MonitorEvent {
            tick: self.tick_count,
            severity,
            title,
            detail,
        });
    }
}

fn dfs_tree<'a>(
    node: &'a ProcessInfo,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    children: &HashMap<u32, Vec<&'a ProcessInfo>>,
    visited: &mut HashSet<u32>,
    result: &mut Vec<(&'a ProcessInfo, String)>,
) {
    if !visited.insert(node.pid) {
        return;
    }

    let branch = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}└─ ")
    } else {
        format!("{prefix}├─ ")
    };

    result.push((node, branch));

    if let Some(kids) = children.get(&node.pid) {
        let child_prefix = if is_root {
            String::new()
        } else if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        let count = kids.len();
        for (i, &child) in kids.iter().enumerate() {
            let last = i == count - 1;
            dfs_tree(child, &child_prefix, last, false, children, visited, result);
        }
    }
}

fn pressure_tick(current: u8, active: bool) -> u8 {
    if active {
        current.saturating_add(1)
    } else {
        0
    }
}

fn severity_label_for_event(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "Healthy",
        Severity::Warn => "Attention",
        Severity::Critical => "Critical",
        Severity::Neutral => "Monitoring",
    }
}

fn sustained_suffix(ticks: u8) -> String {
    if ticks >= 3 {
        format!(" (sustained {ticks} samples)")
    } else {
        String::new()
    }
}

fn empty_disk_info() -> DiskInfo {
    DiskInfo {
        mount_point: String::from("/"),
        fs_type: String::from("ext4"),
        used_gb: 0.0,
        total_gb: 0.0,
        free_gb: 0.0,
        pct: 0,
    }
}

fn calculate_cpu_usage(prev: CpuSample, curr: CpuSample) -> f64 {
    let idle = curr.idle.saturating_sub(prev.idle);
    let total = curr.total.saturating_sub(prev.total);
    if total == 0 {
        0.0
    } else {
        (1.0 - (idle as f64 / total as f64)) * 100.0
    }
}

fn push_history(history: &mut VecDeque<(f64, f64)>, counter: u64, value: f64) {
    if history.len() >= HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back((counter as f64, value));
}

/// ASCII fast-path substring search against an already-lowercased needle.
/// Avoids `haystack.to_lowercase()` (one heap alloc per process per frame).
fn contains_case_insensitive(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.len() < needle_lower.len() {
        return false;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let h = haystack.as_bytes();
        let n = needle_lower.as_bytes();
        return h.len() >= n.len()
            && (0..=(h.len() - n.len())).any(|i| {
                h[i..i + n.len()]
                    .iter()
                    .zip(n.iter())
                    .all(|(a, b)| a.to_ascii_lowercase() == *b)
            });
    }
    haystack.to_lowercase().contains(needle_lower)
}

/// Decimal substring check for PIDs without `pid.to_string()` allocation.
fn pid_contains(pid: u32, needle: &str) -> bool {
    if needle.is_empty() || !needle.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if needle.len() > 10 {
        return false;
    }
    let needle_val: u32 = match needle.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    // Exact match is the common case (`/1234`); substring fallback via
    // fixed stack buffer instead of a heap String.
    if pid == needle_val {
        return true;
    }
    let mut buf = [0u8; 10];
    let mut v = pid;
    let len: usize = if v == 0 {
        buf[0] = b'0';
        1
    } else {
        let mut tmp = [0u8; 10];
        let mut n = 0;
        while v > 0 {
            tmp[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
        for i in 0..n {
            buf[i] = tmp[n - 1 - i];
        }
        n
    };
    let hay = &buf[..len];
    let nd = needle.as_bytes();
    hay.len() >= nd.len() && (0..=(hay.len() - nd.len())).any(|i| &hay[i..i + nd.len()] == nd)
}

fn normalize_rate(value: f64) -> f64 {
    if value.abs() < f64::EPSILON {
        0.0
    } else {
        value
    }
}

pub fn build_alerts(app: &AppState) -> Vec<String> {
    let mut alerts = Vec::with_capacity(8);
    if app.cpu_usage >= app.cpu_alert {
        alerts.push(format!("\u{26a1} CPU alert: {:.1}% load", app.cpu_usage));
    }
    if app.mem_pct() >= app.mem_alert {
        alerts.push(format!("\u{26a1} Memory alert: {:.1}% used", app.mem_pct()));
    }
    if let Some(temp) = app.temp_c {
        if temp >= app.temp_alert {
            alerts.push(format!("\u{26a1} Thermal alert: {:.1}\u{b0}C", temp));
        }
    }
    if let Some(gpu) = app.primary_gpu() {
        if let Some(temp) = gpu.temp_c {
            if temp >= 95.0 {
                alerts.push(format!(
                    "\u{26a1} GPU thermal alert: {} at {:.1}\u{b0}C",
                    gpu.card, temp
                ));
            } else if temp >= 85.0 {
                alerts.push(format!(
                    "\u{26a0} GPU warm: {} at {:.1}\u{b0}C",
                    gpu.card, temp
                ));
            }
        }
    }
    if app.disk.pct >= app.disk_alert {
        alerts.push(format!("\u{26a0} Disk space: {}% full", app.disk.pct));
    }
    if !app.failed_units.is_empty() {
        alerts.push(format!(
            "\u{26a0} Systemd: {} failed units",
            app.failed_units.len()
        ));
    }
    if app.zombie_count > 0 {
        alerts.push(format!("\u{2639} Zombie procs: {}", app.zombie_count));
    }

    if let Some(pct) = app.battery_pct {
        if pct <= app.battery_alert {
            alerts.push(format!("\u{26a0} Battery low: {}%", pct));
        }
    }

    if alerts.is_empty() {
        alerts.push(String::from(
            "System stable \u{2713} All thresholds nominal.",
        ));
    }
    alerts
}

pub fn build_root_causes(app: &AppState) -> Vec<RootCause> {
    let mut causes = Vec::with_capacity(8);
    let top_cpu = app.top_cpu_processes.first();
    let top_mem = app.top_mem_processes.first();
    let hot_drive = app
        .storage_health
        .iter()
        .find(|drive| drive.risk != Severity::Ok);

    if app.cpu_usage >= app.cpu_alert {
        let detail = if let Some(proc_info) = top_cpu {
            format!(
                "{} (PID {}) is the top CPU consumer at {:.1}%{}",
                proc_info.name,
                proc_info.pid,
                proc_info.cpu_pct,
                sustained_suffix(app.cpu_pressure_ticks)
            )
        } else {
            String::from("System-wide CPU pressure is high, process sample unavailable")
        };
        causes.push(RootCause {
            severity: Severity::Critical,
            title: String::from("CPU pressure"),
            detail,
        });
    }

    if app.mem_pct() >= app.mem_alert {
        let detail = if let Some(proc_info) = top_mem {
            format!(
                "{} (PID {}) uses {:.1} MB RAM{}",
                proc_info.name,
                proc_info.pid,
                proc_info.mem_mb,
                sustained_suffix(app.mem_pressure_ticks)
            )
        } else {
            String::from("Memory pressure is high, top memory process unavailable")
        };
        causes.push(RootCause {
            severity: Severity::Critical,
            title: String::from("Memory pressure"),
            detail,
        });
    }

    if app.swap_pct() >= app.swap_alert {
        causes.push(RootCause {
            severity: Severity::Warn,
            title: String::from("Swap pressure"),
            detail: format!(
                "Swap usage at {:.1}%; memory pressure may affect responsiveness",
                app.swap_pct()
            ),
        });
    }

    if let Some(temp) = app.temp_c {
        if temp >= app.temp_alert {
            causes.push(RootCause {
                severity: Severity::Critical,
                title: String::from("Thermal pressure"),
                detail: format!(
                    "CPU temperature is {:.0}\u{b0}C; cooling or workload may need attention{}",
                    temp,
                    sustained_suffix(app.thermal_pressure_ticks)
                ),
            });
        } else if temp >= 70.0 {
            causes.push(RootCause {
                severity: Severity::Warn,
                title: String::from("CPU warm"),
                detail: format!(
                    "CPU temperature is {:.0}\u{b0}C; watch sustained load",
                    temp
                ),
            });
        }
    }

    if let Some(gpu) = app.primary_gpu() {
        if let Some(temp) = gpu.temp_c {
            if temp >= 95.0 {
                causes.push(RootCause {
                    severity: Severity::Critical,
                    title: String::from("GPU hot"),
                    detail: format!(
                        "{} temperature is {:.0}\u{b0}C from {}",
                        gpu.card, temp, gpu.sensor_source
                    ),
                });
            } else if temp >= 85.0 {
                causes.push(RootCause {
                    severity: Severity::Warn,
                    title: String::from("GPU warm"),
                    detail: format!("{} temperature is {:.0}\u{b0}C", gpu.card, temp),
                });
            }
        }
    }

    if app.disk.pct >= app.disk_alert {
        causes.push(RootCause {
            severity: Severity::Critical,
            title: String::from("Disk capacity"),
            detail: format!("Root disk is {}% full", app.disk.pct),
        });
    }

    if let Some(disk_io) = app
        .disk_io
        .iter()
        .max_by(|a, b| {
            (a.read_bps + a.write_bps)
                .partial_cmp(&(b.read_bps + b.write_bps))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|io| io.read_bps + io.write_bps >= 50.0 * 1024.0 * 1024.0)
    {
        causes.push(RootCause {
            severity: Severity::Warn,
            title: String::from("Disk I/O"),
            detail: format!(
                "{} active: read {} / write {} per second",
                disk_io.device,
                format_rate(disk_io.read_bps),
                format_rate(disk_io.write_bps)
            ),
        });
    }

    if let Some(unit) = app.failed_units.first() {
        causes.push(RootCause {
            severity: Severity::Critical,
            title: String::from("Failed service"),
            detail: format!("{} is {} / {}", unit.unit, unit.active, unit.sub),
        });
    }

    if let Some(drive) = hot_drive {
        causes.push(RootCause {
            severity: drive.risk,
            title: String::from("Storage risk"),
            detail: format!("{}: {}", drive.device, drive.note),
        });
    }

    if causes.is_empty() {
        causes.push(RootCause {
            severity: Severity::Ok,
            title: String::from("No root cause"),
            detail: String::from("No dominant pressure source detected"),
        });
    }

    causes.truncate(5);
    causes
}

pub fn calculate_health_score(app: &AppState) -> u16 {
    let mut score = 100.0;
    score -= over_threshold_penalty(app.cpu_usage, 70.0, 25.0);
    score -= over_threshold_penalty(app.mem_pct(), 70.0, 25.0);
    score -= over_threshold_penalty(app.disk.pct as f64, 75.0, 20.0);
    score -= over_threshold_penalty(app.swap_pct(), 20.0, 15.0);
    if let Some(temp) = app.temp_c {
        score -= over_threshold_penalty(temp, 70.0, 15.0);
    }
    if let Some(gpu) = app.primary_gpu() {
        if let Some(temp) = gpu.temp_c {
            score -= over_threshold_penalty(temp, 85.0, 10.0);
        }
    }
    for p in app.top_cpu_processes.iter() {
        if p.cpu_pct > 90.0 {
            score -= 2.0;
        }
    }
    score -= app.failed_units.len().min(5) as f64 * 4.0;
    for drive in &app.storage_health {
        score -= match drive.risk {
            Severity::Critical => 10.0,
            Severity::Warn => 5.0,
            Severity::Ok | Severity::Neutral => 0.0,
        };
    }
    score.clamp(0.0, 100.0).round() as u16
}

fn over_threshold_penalty(value: f64, threshold: f64, max_penalty: f64) -> f64 {
    if value <= threshold {
        0.0
    } else {
        ((value - threshold) / (100.0 - threshold) * max_penalty).clamp(0.0, max_penalty)
    }
}

fn format_rate(bytes_per_second: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes_per_second = bytes_per_second.max(0.0);

    if bytes_per_second >= GB {
        format!("{:.1} GB/s", bytes_per_second / GB)
    } else if bytes_per_second >= MB {
        format!("{:.1} MB/s", bytes_per_second / MB)
    } else if bytes_per_second >= KB {
        format!("{:.1} KB/s", bytes_per_second / KB)
    } else {
        format!("{:.0} B/s", bytes_per_second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare_state() -> AppState {
        AppState::test_state()
    }

    fn make_process(
        pid: u32,
        name: &str,
        cpu: f64,
        mem: f64,
        threads: u32,
        state: &str,
    ) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: 1,
            name: name.to_string(),
            cpu_pct: cpu,
            mem_mb: mem,
            threads,
            state: state.to_string(),
            reason: "Normal".to_string(),
            is_high_risk: cpu > 90.0,
            is_dev: false,
        }
    }

    #[test]
    fn health_score_perfect() {
        let mut app = bare_state();
        app.cpu_usage = 10.0;
        app.mem_total = 16000.0;
        app.mem_used = 4000.0;
        app.swap_total = 8000.0;
        app.swap_used = 0.0;
        app.disk = DiskInfo {
            mount_point: "/".into(),
            fs_type: "ext4".into(),
            used_gb: 50.0,
            total_gb: 500.0,
            free_gb: 450.0,
            pct: 10,
        };
        app.temp_c = Some(45.0);
        app.health_score = calculate_health_score(&app);
        assert_eq!(app.health_score, 100);
    }

    #[test]
    fn health_score_high_cpu_penalty() {
        let mut app = bare_state();
        app.cpu_usage = 95.0;
        app.mem_total = 16000.0;
        app.mem_used = 4000.0;
        app.swap_total = 8000.0;
        app.swap_used = 0.0;
        app.disk = DiskInfo {
            mount_point: "/".into(),
            fs_type: "ext4".into(),
            used_gb: 50.0,
            total_gb: 500.0,
            free_gb: 450.0,
            pct: 10,
        };
        app.temp_c = Some(45.0);
        app.health_score = calculate_health_score(&app);
        assert!(app.health_score < 85, "score={}", app.health_score);
    }

    #[test]
    fn health_score_high_mem_penalty() {
        let mut app = bare_state();
        app.cpu_usage = 10.0;
        app.mem_total = 16000.0;
        app.mem_used = 15200.0; // 95%
        app.swap_total = 8000.0;
        app.swap_used = 500.0;
        app.disk = DiskInfo {
            mount_point: "/".into(),
            fs_type: "ext4".into(),
            used_gb: 50.0,
            total_gb: 500.0,
            free_gb: 450.0,
            pct: 10,
        };
        app.temp_c = Some(45.0);
        app.health_score = calculate_health_score(&app);
        assert!(app.health_score < 85);
    }

    #[test]
    fn health_score_high_risk_processes_penalty() {
        let mut app = bare_state();
        app.top_cpu_processes = vec![
            make_process(1, "bad", 99.0, 100.0, 1, "R"),
            make_process(2, "ok", 50.0, 100.0, 1, "S"),
        ];
        app.health_score = calculate_health_score(&app);
        // pre-penalty state: cpu_usage=0, mem=0, so baseline = 100
        // penalty: over_threshold for near-0 values = 0
        // only penalty is -2 for high-risk process
        assert_eq!(app.health_score, 98);
    }

    #[test]
    fn health_score_full_degradation() {
        let mut app = bare_state();
        app.cpu_usage = 100.0;
        app.mem_total = 100.0;
        app.mem_used = 100.0;
        app.swap_total = 100.0;
        app.swap_used = 100.0;
        app.disk = DiskInfo {
            mount_point: "/".into(),
            fs_type: "ext4".into(),
            used_gb: 90.0,
            total_gb: 100.0,
            free_gb: 10.0,
            pct: 90,
        };
        app.temp_c = Some(100.0);
        app.failed_units = vec![SystemdUnitIssue {
            unit: "bad.service".into(),
            load: "loaded".into(),
            active: "failed".into(),
            sub: "failed".into(),
            description: "".into(),
        }];
        app.health_score = calculate_health_score(&app);
        assert!(app.health_score < 10, "score={}", app.health_score);
    }

    #[test]
    fn filtered_processes_sorts_by_cpu_desc() {
        let mut app = bare_state();
        app.process_sort = ProcessSort::CpuDesc;
        app.top_cpu_processes = vec![
            make_process(1, "a", 50.0, 100.0, 1, "R"),
            make_process(2, "b", 90.0, 100.0, 1, "R"),
            make_process(3, "c", 10.0, 100.0, 1, "S"),
        ];
        let result = app.filtered_processes();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].pid, 2);
        assert_eq!(result[1].pid, 1);
        assert_eq!(result[2].pid, 3);
    }

    #[test]
    fn filtered_processes_sorts_by_pid_asc() {
        let mut app = bare_state();
        app.process_sort = ProcessSort::PidAsc;
        app.top_cpu_processes = vec![
            make_process(100, "a", 50.0, 100.0, 1, "R"),
            make_process(10, "b", 90.0, 100.0, 1, "R"),
            make_process(1, "c", 10.0, 100.0, 1, "S"),
        ];
        app.top_mem_processes = vec![make_process(50, "d", 5.0, 100.0, 1, "S")];
        let result = app.filtered_processes();
        assert_eq!(result[0].pid, 1);
        assert_eq!(result[1].pid, 10);
        assert_eq!(result[2].pid, 50);
        assert_eq!(result[3].pid, 100);
    }

    #[test]
    fn filtered_processes_search_filter() {
        let mut app = bare_state();
        app.process_sort = ProcessSort::CpuDesc;
        app.top_cpu_processes = vec![
            make_process(1, "firefox", 50.0, 100.0, 1, "R"),
            make_process(2, "systemd", 10.0, 100.0, 1, "S"),
            make_process(3, "firefox-esr", 20.0, 100.0, 1, "S"),
            make_process(4, "bash", 5.0, 100.0, 1, "S"),
        ];
        app.process_search = "firefox".to_string();
        let result = app.filtered_processes();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|p| p.name.contains("firefox")));
    }

    #[test]
    fn build_alerts_empty_when_nominal() {
        let app = bare_state();
        let alerts = build_alerts(&app);
        assert!(!alerts.is_empty());
        assert!(alerts[0].contains("stable") || alerts[0].contains("nominal"));
    }

    #[test]
    fn build_alerts_cpu_alert_triggers() {
        let mut app = bare_state();
        app.cpu_usage = 90.0;
        app.cpu_alert = 50.0;
        let alerts = build_alerts(&app);
        assert!(alerts.iter().any(|a| a.contains("CPU")));
    }

    #[test]
    fn build_alerts_mem_alert_triggers() {
        let mut app = bare_state();
        app.mem_total = 1000.0;
        app.mem_used = 950.0;
        app.mem_alert = 50.0;
        let alerts = build_alerts(&app);
        assert!(alerts.iter().any(|a| a.contains("Memory")));
    }

    #[test]
    fn build_alerts_disk_alert_triggers() {
        let mut app = bare_state();
        app.disk.pct = 95;
        app.disk_alert = 50;
        let alerts = build_alerts(&app);
        assert!(alerts.iter().any(|a| a.contains("Disk")));
    }

    #[test]
    fn build_root_causes_cpu_pressure() {
        let mut app = bare_state();
        app.cpu_usage = 90.0;
        app.cpu_alert = 50.0;
        app.top_cpu_processes = vec![make_process(42, "stressed", 95.0, 100.0, 1, "R")];
        let causes = build_root_causes(&app);
        assert!(causes.iter().any(|c| c.title.contains("CPU")));
        assert!(causes.iter().any(|c| c.detail.contains("stressed")));
        assert!(causes.iter().any(|c| c.detail.contains("PID 42")));
    }

    #[test]
    fn build_root_causes_memory_pressure() {
        let mut app = bare_state();
        app.mem_total = 1000.0;
        app.mem_used = 950.0;
        app.mem_alert = 50.0;
        app.top_mem_processes = vec![make_process(99, "mem-hog", 10.0, 800.0, 1, "R")];
        let causes = build_root_causes(&app);
        assert!(causes.iter().any(|c| c.title.contains("Memory")));
    }

    #[test]
    fn build_root_causes_empty_when_healthy() {
        let app = bare_state();
        let causes = build_root_causes(&app);
        assert!(causes.iter().any(|c| c.title == "No root cause"));
    }

    #[test]
    fn severity_from_usage() {
        assert_eq!(Severity::from_usage(90.0), Severity::Critical);
        assert_eq!(Severity::from_usage(75.0), Severity::Warn);
        assert_eq!(Severity::from_usage(50.0), Severity::Ok);
    }

    #[test]
    fn severity_from_health() {
        assert_eq!(Severity::from_health(90.0), Severity::Ok);
        assert_eq!(Severity::from_health(65.0), Severity::Warn);
        assert_eq!(Severity::from_health(30.0), Severity::Critical);
    }

    #[test]
    fn calculate_cpu_usage_basic() {
        let prev = CpuSample {
            idle: 100,
            total: 500,
        };
        let curr = CpuSample {
            idle: 150,
            total: 1000,
        };
        // idle delta = 50, total delta = 500
        // usage = (1 - 50/500) * 100 = 90%
        let usage = calculate_cpu_usage(prev, curr);
        assert!((usage - 90.0).abs() < 0.01);
    }

    #[test]
    fn calculate_cpu_usage_no_delta() {
        let prev = CpuSample {
            idle: 100,
            total: 500,
        };
        let curr = CpuSample {
            idle: 100,
            total: 500,
        };
        assert_eq!(calculate_cpu_usage(prev, curr), 0.0);
    }

    #[test]
    fn calculate_cpu_usage_all_busy() {
        let prev = CpuSample {
            idle: 100,
            total: 500,
        };
        let curr = CpuSample {
            idle: 100,
            total: 1000,
        };
        // idle delta = 0, total delta = 500
        // usage = (1 - 0/500) * 100 = 100%
        let usage = calculate_cpu_usage(prev, curr);
        assert!((usage - 100.0).abs() < 0.01);
    }

    #[test]
    fn process_sort_cycles() {
        let mut sort = ProcessSort::CpuDesc;
        assert_eq!(sort.label(), "CPU \u{2193}");
        sort.cycle();
        assert_eq!(sort, ProcessSort::CpuAsc);
        sort.cycle();
        assert_eq!(sort, ProcessSort::MemDesc);
        sort.cycle();
        assert_eq!(sort, ProcessSort::MemAsc);
        sort.cycle();
        assert_eq!(sort, ProcessSort::PidAsc);
        sort.cycle();
        assert_eq!(sort, ProcessSort::PidDesc);
        sort.cycle();
        assert_eq!(sort, ProcessSort::CpuDesc);
    }

    #[test]
    fn mem_pct_calculation() {
        let mut app = bare_state();
        app.mem_total = 16000.0;
        app.mem_used = 8000.0;
        assert!((app.mem_pct() - 50.0).abs() < 0.01);
        app.mem_used = 16000.0;
        assert!((app.mem_pct() - 100.0).abs() < 0.01);
        app.mem_used = 0.0;
        assert!((app.mem_pct() - 0.0).abs() < 0.01);
    }

    #[test]
    fn swap_pct_calculation() {
        let mut app = bare_state();
        app.swap_total = 8000.0;
        app.swap_used = 2000.0;
        assert!((app.swap_pct() - 25.0).abs() < 0.01);
        // No swap
        app.swap_total = 0.0;
        app.swap_used = 0.0;
        assert_eq!(app.swap_pct(), 0.0);
    }

    #[test]
    fn sample_status_tracking() {
        let mut app = bare_state();
        // First tick is "Warming up" — advance past it.
        app.tick_count = 2;
        app.degraded_sources.clear();
        assert_eq!(app.sample_status(), "OK");
        app.degraded_sources.push("cpu".into());
        assert_eq!(app.sample_status(), "Partial");
        app.degraded_sources.push("mem".into());
        assert_eq!(app.sample_status(), "Partial");
        app.degraded_sources.push("disk".into());
        assert_eq!(app.sample_status(), "Degraded");
    }

    #[test]
    fn platform_summary_marks_host_scoped_envs() {
        let mut app = bare_state();
        app.env = EnvKind::Container;
        assert!(
            app.platform_summary().contains("host"),
            "{}",
            app.platform_summary()
        );
        app.env = EnvKind::Wsl;
        assert!(
            app.platform_summary().contains("host"),
            "{}",
            app.platform_summary()
        );
        app.env = EnvKind::BareMetal;
        assert!(!app.platform_summary().contains("host"));
        assert_eq!(EnvKind::Container.badge(), "CTR");
    }

    #[test]
    fn search_matches_case_insensitively() {
        assert!(contains_case_insensitive("Firefox", "fire"));
        assert!(contains_case_insensitive("FIREFOX-ESR", "firefox"));
        assert!(!contains_case_insensitive("bash", "fire"));
    }

    #[test]
    fn pid_contains_matches_without_allocation() {
        assert!(pid_contains(1234, "23"));
        assert!(pid_contains(1234, "1234"));
        assert!(!pid_contains(1234, "99"));
        assert!(!pid_contains(1234, "fire"));
        assert!(!pid_contains(12, "12345"));
    }

    #[test]
    fn refresh_interval_cycling() {
        let mut app = bare_state();
        app.refresh_index = 0; // 500ms
        assert_eq!(app.refresh_label(), "500ms");
        app.faster_refresh();
        assert_eq!(app.refresh_index, 0); // can't go below 0
        app.slower_refresh();
        assert_eq!(app.refresh_index, 1);
        assert_eq!(app.refresh_label(), "750ms");
        app.refresh_index = 4; // last (5s)
        app.slower_refresh();
        assert_eq!(app.refresh_index, 4); // can't go above max
    }

    #[test]
    fn view_tab_navigation() {
        assert_eq!(ViewTab::Overview.next(), ViewTab::Cpu);
        assert_eq!(ViewTab::Processes.next(), ViewTab::Overview);
        assert_eq!(ViewTab::Overview.prev(), ViewTab::Processes);
        assert_eq!(ViewTab::Cpu.prev(), ViewTab::Overview);
        assert_eq!(ViewTab::Overview.index(), 0);
        assert_eq!(ViewTab::Processes.index(), 6);
    }

    #[test]
    fn format_rate_variants() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(500.0), "500 B/s");
        assert_eq!(format_rate(1500.0), "1.5 KB/s");
        assert_eq!(format_rate(1_500_000.0), "1.4 MB/s");
        assert_eq!(format_rate(2_000_000_000.0), "1.9 GB/s");
    }

    #[test]
    fn push_history_respects_limit() {
        let mut history = VecDeque::with_capacity(HISTORY_LIMIT);
        for i in 0..HISTORY_LIMIT + 50 {
            push_history(&mut history, i as u64, i as f64);
        }
        assert_eq!(history.len(), HISTORY_LIMIT);
        // first entry should be 50 (after 50 evictions), last entry should be the max
        assert_eq!(history.front().map(|(_, v)| *v), Some(50.0));
        assert_eq!(
            history.back().map(|(_, v)| *v),
            Some((HISTORY_LIMIT + 50 - 1) as f64)
        );
    }

    #[test]
    fn over_threshold_penalty_basics() {
        assert_eq!(over_threshold_penalty(50.0, 70.0, 25.0), 0.0); // below threshold
        assert_eq!(over_threshold_penalty(70.0, 70.0, 25.0), 0.0); // at threshold
        assert!(over_threshold_penalty(85.0, 70.0, 25.0) > 0.0); // above threshold
        assert_eq!(over_threshold_penalty(100.0, 70.0, 25.0), 25.0); // max penalty
    }

    #[test]
    fn snapshot_contains_expected_fields() {
        let mut app = bare_state();
        app.tick_count = 2;
        let snap = app.to_snapshot();
        assert_eq!(snap.health_status, "Healthy");
        assert_eq!(snap.sample_status, "OK");
        assert_eq!(snap.core_count, 1);
    }

    #[test]
    fn filtered_processes_empty_when_no_data() {
        let app = bare_state();
        assert!(app.filtered_processes().is_empty());
    }

    #[test]
    fn toggle_tree_view_switches_mode_and_resets_selection() {
        let mut app = bare_state();
        app.process_selected = 5;
        assert!(!app.is_tree_view);
        app.toggle_tree_view();
        assert!(app.is_tree_view);
        assert_eq!(app.process_selected, 0);
        app.toggle_tree_view();
        assert!(!app.is_tree_view);
    }

    #[test]
    fn tree_view_formats_hierarchy_and_branch_prefixes() {
        let mut app = bare_state();
        let mut p1 = make_process(100, "parent", 10.0, 100.0, 1, "S");
        p1.ppid = 1;
        let mut p2 = make_process(101, "child_a", 5.0, 50.0, 1, "S");
        p2.ppid = 100;
        let mut p3 = make_process(102, "child_b", 2.0, 30.0, 1, "S");
        p3.ppid = 100;
        let mut p4 = make_process(103, "grandchild", 1.0, 20.0, 1, "S");
        p4.ppid = 101;

        app.top_cpu_processes = vec![p1, p2, p3, p4];
        app.is_tree_view = true;

        let tree = app.filtered_process_tree();
        assert_eq!(tree.len(), 4);
        assert_eq!(tree[0].0.pid, 100);
        assert_eq!(tree[0].1, ""); // root has empty prefix
        assert_eq!(tree[1].0.pid, 101);
        assert_eq!(tree[1].1, "├─ "); // first child
        assert_eq!(tree[2].0.pid, 103);
        assert_eq!(tree[2].1, "│  └─ "); // grandchild under first child
        assert_eq!(tree[3].0.pid, 102);
        assert_eq!(tree[3].1, "└─ "); // last child
    }

    #[test]
    fn request_kill_signal_workflow() {
        let mut app = bare_state();
        let p = make_process(99999, "test_proc", 10.0, 100.0, 1, "S");
        app.top_cpu_processes = vec![p];

        // First press: enters confirmation
        app.request_kill();
        assert_eq!(app.confirm_kill_pid, Some(99999));
        assert!(app
            .process_action_message
            .as_ref()
            .unwrap()
            .contains("Terminate PID 99999"));

        // Cancel test
        app.cancel_kill();
        assert_eq!(app.confirm_kill_pid, None);

        // First press again: enters confirmation
        app.request_kill();
        assert_eq!(app.confirm_kill_pid, Some(99999));

        // Signal 9 press (will fail gracefully because 99999 is non-existent process in test)
        app.request_kill_signal(libc::SIGKILL);
        assert_eq!(app.confirm_kill_pid, None);
        assert!(app
            .process_action_message
            .as_ref()
            .unwrap()
            .contains("PID 99999"));
    }

    #[test]
    fn inspector_workflow_and_signal_handling() {
        let mut app = bare_state();
        let own_pid = std::process::id();
        app.open_inspector(own_pid);
        assert_eq!(app.inspect_process_pid, Some(own_pid));
        assert!(app.inspect_process_detail.is_some());
        let detail = app.inspect_process_detail.as_ref().unwrap();
        assert_eq!(detail.pid, own_pid);

        // Refresh
        app.refresh_inspected_process(own_pid);
        assert_eq!(app.inspect_process_pid, Some(own_pid));

        // Close
        app.close_inspector();
        assert_eq!(app.inspect_process_pid, None);
        assert!(app.inspect_process_detail.is_none());

        // Signal to non-existent PID
        app.open_inspector(99999);
        app.send_inspected_signal(libc::SIGSTOP);
        assert!(app.process_action_message.is_some());
    }
}
