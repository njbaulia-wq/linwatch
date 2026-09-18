use crate::types::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::process::Command;

#[must_use]
pub fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
}

/// Security-module status across distros: SELinux enforcing/permissive,
/// AppArmor on Debian/Ubuntu family, otherwise Disabled.
pub fn read_security_mode() -> String {
    if let Ok(content) = fs::read_to_string("/sys/fs/selinux/enforce") {
        match content.trim() {
            "1" => return String::from("Enforcing"),
            "0" => return String::from("Permissive"),
            _ => return String::from("Unknown"),
        }
    }
    if Path::new("/sys/kernel/security/apparmor").exists()
        || fs::read_to_string("/sys/module/apparmor/parameters/enabled")
            .is_ok_and(|v| v.trim().starts_with('Y'))
    {
        return String::from("AppArmor");
    }
    String::from("Disabled")
}

/// Runtime environment. Containers/WSL share the host kernel, so uptime,
/// load and network counters there are host-wide — the UI badges this.
pub fn detect_env() -> EnvKind {
    let version = fs::read_to_string("/proc/version").unwrap_or_default();
    let version_lower = version.to_ascii_lowercase();
    if version_lower.contains("microsoft") || version_lower.contains("wsl") {
        return EnvKind::Wsl;
    }
    if read_trimmed("/proc/sys/kernel/osrelease")
        .is_some_and(|k| k.to_ascii_lowercase().contains("microsoft"))
    {
        return EnvKind::Wsl;
    }
    if Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists() {
        return EnvKind::Container;
    }
    if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") {
        if cgroup.contains("docker")
            || cgroup.contains("kubepods")
            || cgroup.contains("lxc")
            || cgroup.contains("containerd")
        {
            return EnvKind::Container;
        }
    }
    if is_virtual_machine(&version) {
        return EnvKind::VirtualMachine;
    }
    EnvKind::BareMetal
}

fn is_virtual_machine(proc_version: &str) -> bool {
    if proc_version.contains("hypervisor") {
        return true;
    }
    for path in [
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/sys_vendor",
    ] {
        if let Ok(value) = fs::read_to_string(path) {
            let lower = value.to_ascii_lowercase();
            if [
                "kvm",
                "qemu",
                "virtualbox",
                "vmware",
                "xen",
                "hyper-v",
                "parallels",
                "bhyve",
            ]
            .iter()
            .any(|sig| lower.contains(sig))
            {
                return true;
            }
        }
    }
    fs::read_to_string("/proc/cpuinfo").is_ok_and(|cpuinfo| cpuinfo.contains("hypervisor"))
}

/// Whether systemd is usable here. Cached: absence means "skip polling",
/// not a per-tick failure (Alpine/OpenRC, containers, WSL1).
pub fn has_systemd() -> bool {
    use std::sync::OnceLock;
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        Path::new("/run/systemd/system").exists()
            && Command::new("systemctl")
                .args(["--version"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
    })
}

#[must_use]
pub fn read_open_ports() -> Option<Vec<OpenPort>> {
    let mut raw_ports = Vec::new();
    if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
        parse_socket_file(&content, "TCP", &mut raw_ports);
    }
    if let Ok(content) = fs::read_to_string("/proc/net/udp") {
        parse_socket_file(&content, "UDP", &mut raw_ports);
    }
    if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
        parse_socket_file(&content, "TCP6", &mut raw_ports);
    }
    if let Ok(content) = fs::read_to_string("/proc/net/udp6") {
        parse_socket_file(&content, "UDP6", &mut raw_ports);
    }
    raw_ports.sort_by_key(|(p, _)| p.port);
    raw_ports.dedup_by(|(a, _), (b, _)| a.port == b.port && a.proto == b.proto);

    let target_inodes: HashSet<u64> = raw_ports
        .iter()
        .map(|(_, inode)| *inode)
        .filter(|&inode| inode > 0)
        .collect();

    let inode_map = build_socket_inode_map(&target_inodes);

    let mut ports = Vec::with_capacity(raw_ports.len());
    for (mut port, inode) in raw_ports {
        if let Some((pid, name)) = inode_map.get(&inode) {
            port.pid = Some(*pid);
            port.process_name = Some(name.clone());
        }
        ports.push(port);
    }
    Some(ports)
}

fn build_socket_inode_map(target_inodes: &HashSet<u64>) -> HashMap<u64, (u32, String)> {
    let mut map = HashMap::new();
    if target_inodes.is_empty() {
        return map;
    }

    let Ok(proc_entries) = fs::read_dir("/proc") else {
        return map;
    };

    for entry in proc_entries.flatten() {
        if map.len() == target_inodes.len() {
            break;
        }
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        let Ok(pid) = name_str.parse::<u32>() else {
            continue;
        };

        let fd_path = entry.path().join("fd");
        let Ok(fd_entries) = fs::read_dir(fd_path) else {
            continue;
        };

        let mut comm_cached: Option<String> = None;

        for fd_entry in fd_entries.flatten() {
            if let Ok(link) = fs::read_link(fd_entry.path()) {
                let link_str = link.to_string_lossy();
                if let Some(inode_str) = link_str
                    .strip_prefix("socket:[")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    if let Ok(inode) = inode_str.parse::<u64>() {
                        if target_inodes.contains(&inode) && !map.contains_key(&inode) {
                            let proc_name = match &comm_cached {
                                Some(name) => name.clone(),
                                None => {
                                    let comm = fs::read_to_string(format!("/proc/{pid}/comm"))
                                        .map(|s| s.trim().to_string())
                                        .unwrap_or_else(|_| String::from("unknown"));
                                    comm_cached = Some(comm.clone());
                                    comm
                                }
                            };
                            map.insert(inode, (pid, proc_name));
                            if map.len() == target_inodes.len() {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    map
}

fn identify_service_port(port: u16) -> String {
    match port {
        22 => String::from("SSH"),
        53 => String::from("DNS"),
        80 => String::from("HTTP"),
        443 => String::from("HTTPS"),
        3000 => String::from("Node/React"),
        3306 => String::from("MySQL"),
        5432 => String::from("Postgres"),
        6379 => String::from("Redis"),
        8000 => String::from("FastAPI/Django"),
        8080 => String::from("HTTP Alt/Java"),
        9000 => String::from("PHP-FPM"),
        27017 => String::from("MongoDB"),
        5173 => String::from("Vite Dev"),
        _ => String::from("Other"),
    }
}

fn parse_socket_file(content: &str, proto: &str, ports: &mut Vec<(OpenPort, u64)>) {
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }
        let local_addr = parts[1];
        let state_hex = parts[3];
        let inode: u64 = parts[9].parse().unwrap_or(0);
        let is_tcp = proto == "TCP" || proto == "TCP6";
        let state = match (is_tcp, state_hex) {
            (true, "0A") => "LISTEN",
            (true, "01") => "ESTABLISHED",
            (false, _) => "OPEN",
            _ => continue,
        };
        if is_tcp && state != "LISTEN" {
            continue;
        }
        if let Some((ip_hex, port_hex)) = local_addr.split_once(':') {
            let ip = match ip_hex.len() {
                8 => parse_hex_ip(ip_hex),
                32 => parse_hex_ip6(ip_hex),
                _ => Err(()),
            };
            if let (Ok(port), Ok(ip)) = (u16::from_str_radix(port_hex, 16), ip) {
                let service_name = identify_service_port(port);
                ports.push((
                    OpenPort {
                        port,
                        ip,
                        proto: proto.to_string(),
                        state: state.to_string(),
                        service_name,
                        pid: None,
                        process_name: None,
                    },
                    inode,
                ));
            }
        }
    }
}

fn parse_hex_ip(hex: &str) -> Result<String, ()> {
    if hex.len() != 8 {
        return Err(());
    }
    let bytes = [
        u8::from_str_radix(&hex[6..8], 16).map_err(|_| ())?,
        u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?,
        u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?,
        u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?,
    ];
    Ok(format!(
        "{}.{}.{}.{}",
        bytes[0], bytes[1], bytes[2], bytes[3]
    ))
}

/// Decode the 32-hex IPv6 address from `/proc/net/tcp6|udp6`.
/// The kernel prints each 32-bit word byte-swapped on little-endian hosts
/// (e.g. `...01000000` is `::1`), so every 4-byte group is reversed back.
/// Both linwatch targets (x86_64, aarch64) are little-endian.
fn parse_hex_ip6(hex: &str) -> Result<String, ()> {
    if hex.len() != 32 || !hex.is_ascii() {
        return Err(());
    }
    let mut raw = [0u8; 16];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| ())?;
        raw[i] = u8::from_str_radix(pair, 16).map_err(|_| ())?;
    }
    for word in raw.as_chunks_mut::<4>().0 {
        word.reverse();
    }
    Ok(std::net::Ipv6Addr::from(u128::from_be_bytes(raw)).to_string())
}

pub fn read_system_info() -> SystemInfo {
    // freedesktop os-release spec: /etc takes precedence, /usr/lib is the
    // vendor fallback, containers may expose the host file at /run/host.
    let os_release = read_os_release_content();
    let os_name = os_release_value(&os_release, "NAME")
        .or_else(|| os_release_value(&os_release, "PRETTY_NAME"))
        .unwrap_or_else(|| String::from("Linux"));
    let os_version = os_release_value(&os_release, "VERSION_ID")
        .or_else(|| os_release_value(&os_release, "VERSION"))
        .unwrap_or_else(|| String::from("unknown"));
    let kernel =
        read_trimmed("/proc/sys/kernel/osrelease").unwrap_or_else(|| String::from("unknown"));
    let hostname =
        read_trimmed("/proc/sys/kernel/hostname").unwrap_or_else(|| String::from("localhost"));
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let (cpu_model, cpu_count) = parse_cpuinfo(&cpuinfo);
    let selinux_mode = read_security_mode();

    SystemInfo {
        os_name,
        os_version,
        kernel,
        hostname,
        cpu_model,
        cpu_count,
        selinux_mode,
    }
}

fn read_os_release_content() -> String {
    for path in [
        "/etc/os-release",
        "/usr/lib/os-release",
        "/run/host/os-release",
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            if !content.trim().is_empty() {
                return content;
            }
        }
    }
    String::new()
}

/// Parse CPU model + count across x86 and ARM layouts.
/// x86 uses `model name`; Raspberry Pi / ARM64 use `Model`, `Hardware`
/// (+`Revision`), or `CPU part` instead.
fn parse_cpuinfo(cpuinfo: &str) -> (String, usize) {
    let mut model_x86: Option<String> = None;
    let mut model_pi: Option<String> = None;
    let mut hardware: Option<String> = None;
    let mut revision: Option<String> = None;
    let mut cpu_part: Option<String> = None;
    let mut count = 0;
    let mut mhz_count = 0;

    for line in cpuinfo.lines() {
        if let Some(stripped) = line.strip_prefix("model name") {
            if let Some((_, model)) = stripped.split_once(':') {
                let model = model.trim();
                if !model.is_empty() && model_x86.is_none() {
                    model_x86 = Some(model.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix("Model") {
            if let Some((_, model)) = stripped.split_once(':') {
                let model = model.trim();
                if !model.is_empty() && model_pi.is_none() {
                    model_pi = Some(model.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix("Hardware") {
            if let Some((_, value)) = stripped.split_once(':') {
                let value = value.trim();
                if !value.is_empty() && hardware.is_none() {
                    hardware = Some(value.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix("Revision") {
            if let Some((_, value)) = stripped.split_once(':') {
                let value = value.trim();
                if !value.is_empty() && revision.is_none() {
                    revision = Some(value.to_string());
                }
            }
        } else if line.starts_with("CPU part") {
            if let Some((_, value)) = line.split_once(':') {
                let value = value.trim();
                if !value.is_empty() && cpu_part.is_none() {
                    cpu_part = Some(value.to_string());
                }
            }
        }
        if line.starts_with("processor") {
            count += 1;
        } else if line.starts_with("cpu MHz") {
            mhz_count += 1;
        }
    }

    if count == 0 {
        count = mhz_count;
    }
    let cpu_count = count.max(1);
    let cpu_model = model_x86
        .or(model_pi)
        .or_else(|| match (hardware, revision) {
            (Some(h), Some(r)) => Some(format!("{h} ({r})")),
            (Some(h), None) => Some(h),
            (None, _) => None,
        })
        .or_else(|| cpu_part.map(|p| format!("ARM ({p})")))
        .unwrap_or_else(|| String::from("Unknown CPU"));
    (cpu_model, cpu_count)
}

fn os_release_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k == key).then(|| v.trim_matches('"').to_string())
    })
}

#[must_use]
pub fn read_cpu_snapshot() -> Option<CpuSnapshot> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let mut total = None;
    let mut cores = Vec::with_capacity(64);

    for line in content.lines().filter(|line| line.starts_with("cpu")) {
        let label = line.split_whitespace().next().unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        let sample = parse_cpu_sample(line).unwrap_or(CpuSample { idle: 0, total: 0 });
        if label == "cpu" {
            total = Some(sample);
        } else if label
            .strip_prefix("cpu")
            .is_some_and(|suffix| suffix.chars().all(|c| c.is_ascii_digit()))
        {
            cores.push(sample);
        }
    }

    Some(CpuSnapshot {
        total: total?,
        cores,
    })
}

fn parse_cpu_sample(line: &str) -> Option<CpuSample> {
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse().ok())
        .collect();
    if values.len() < 5 {
        return None;
    }
    let idle = values[3] + values[4];
    let total = values.iter().sum();
    Some(CpuSample { idle, total })
}

#[must_use]
pub fn read_mem_info() -> Option<MemInfo> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    parse_mem_info(&content)
}

fn parse_mem_info(content: &str) -> Option<MemInfo> {
    let mut mem_total = 0.0;
    let mut mem_available = 0.0;
    let mut has_available = false;
    // Pre-3.14 kernels lack MemAvailable: emulate `free` as
    // free + buffers + cache (+ reclaimable slab).
    let mut mem_free = 0.0;
    let mut buffers = 0.0;
    let mut cached = 0.0;
    let mut sreclaimable = 0.0;
    let mut swap_total = 0.0;
    let mut swap_free = 0.0;

    for line in content.lines() {
        let value = line
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        if line.starts_with("MemTotal:") {
            mem_total = value;
        } else if line.starts_with("MemAvailable:") {
            mem_available = value;
            has_available = true;
        } else if line.starts_with("MemFree:") {
            mem_free = value;
        } else if line.starts_with("Buffers:") {
            buffers = value;
        } else if line.starts_with("Cached:") {
            cached = value;
        } else if line.starts_with("SReclaimable:") {
            sreclaimable = value;
        } else if line.starts_with("SwapTotal:") {
            swap_total = value;
        } else if line.starts_with("SwapFree:") {
            swap_free = value;
        }
    }

    let used_kb = if has_available {
        mem_total - mem_available
    } else {
        let emulated_available = mem_free + buffers + cached + sreclaimable;
        if emulated_available > 0.0 {
            mem_total - emulated_available
        } else {
            mem_total - mem_free
        }
    };

    let available_kb = if has_available {
        mem_available
    } else {
        mem_free + buffers + cached + sreclaimable
    };

    (mem_total > 0.0).then_some(MemInfo {
        total_mb: mem_total / 1024.0,
        used_mb: used_kb.max(0.0) / 1024.0,
        available_mb: available_kb.max(0.0) / 1024.0,
        free_mb: mem_free / 1024.0,
        cached_mb: (cached + sreclaimable) / 1024.0,
        buffers_mb: buffers / 1024.0,
        swap_total_mb: swap_total / 1024.0,
        swap_used_mb: (swap_total - swap_free).max(0.0) / 1024.0,
    })
}

#[must_use]
pub fn read_disk_info() -> Option<Vec<DiskInfo>> {
    let mut mounts = important_mounts();
    if !mounts.iter().any(|(m, _)| m == "/") {
        mounts.insert(0, (String::from("/"), String::from("ext4")));
    }

    let mut infos = Vec::with_capacity(mounts.len());
    for (mount, fs_type) in mounts {
        if let Some(info) = read_mount_info(&mount, &fs_type) {
            infos.push(info);
        }
    }

    (!infos.is_empty()).then_some(infos)
}

/// Root-disk entry: prefer `/`, fall back to the first real mount.
pub fn primary_mount(mounts: &[DiskInfo]) -> Option<&DiskInfo> {
    mounts
        .iter()
        .find(|m| m.mount_point == "/")
        .or_else(|| mounts.first())
}

fn important_mounts() -> Vec<(String, String)> {
    // Filter by filesystem type (not an allowlist of paths) so extra data
    // mounts (/data, /mnt/*, NFS, btrfs subvolumes) are monitored too.
    const PSEUDO: &[&str] = &[
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "tmpfs",
        "shm",
        "mqueue",
        "cgroup",
        "cgroup2",
        "overlay",
        "squashfs",
        "nsfs",
        "debugfs",
        "tracefs",
        "fusectl",
        "configfs",
        "securityfs",
        "hugetlbfs",
        "pstore",
        "autofs",
        "bpf",
    ];
    let content = fs::read_to_string("/proc/mounts").unwrap_or_default();
    let mut mounts = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(mount) = parts.get(1) else { continue };
        let Some(fs_type) = parts.get(2) else {
            continue;
        };
        if PSEUDO.contains(fs_type) {
            continue;
        }
        mounts.push(((*mount).to_string(), (*fs_type).to_string()));
    }
    mounts.sort_by(|a, b| a.0.cmp(&b.0));
    mounts.dedup_by(|a, b| a.0 == b.0);
    mounts
}

fn read_mount_info(mount_point: &str, fs_type: &str) -> Option<DiskInfo> {
    let path = CString::new(mount_point).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: statvfs is a POSIX read-only syscall that fills a caller-provided buffer.
    // The MaybeUninit is only written to on success (result == 0).
    let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    // SAFETY: statvfs returned success, so the buffer was fully initialized above.
    let stats = unsafe { stats.assume_init() };
    let block_size = stats.f_frsize as f64;
    let total_bytes = stats.f_blocks as f64 * block_size;
    let free_bytes = stats.f_bfree as f64 * block_size;
    let used_bytes = (total_bytes - free_bytes).max(0.0);
    let pct = if total_bytes > 0.0 {
        ((used_bytes / total_bytes) * 100.0).round() as u16
    } else {
        0
    };

    Some(DiskInfo {
        mount_point: mount_point.to_string(),
        fs_type: fs_type.to_string(),
        used_gb: used_bytes / 1024.0 / 1024.0 / 1024.0,
        total_gb: total_bytes / 1024.0 / 1024.0 / 1024.0,
        free_gb: free_bytes / 1024.0 / 1024.0 / 1024.0,
        pct: pct.min(100),
    })
}

pub fn read_uptime() -> Option<String> {
    let content = fs::read_to_string("/proc/uptime").ok()?;
    let secs = content.split_whitespace().next()?.parse::<f64>().ok()? as u64;
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    Some(if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else {
        format!("{hours:02}h {minutes:02}m")
    })
}

pub fn read_load_average() -> Option<[String; 3]> {
    let content = fs::read_to_string("/proc/loadavg").ok()?;
    let mut values = [
        String::from("0.00"),
        String::from("0.00"),
        String::from("0.00"),
    ];
    for (idx, value) in content.split_whitespace().take(3).enumerate() {
        values[idx] = value.to_string();
    }
    Some(values)
}

pub fn read_battery() -> (Option<u16>, String) {
    let base = Path::new("/sys/class/power_supply");
    let Ok(entries) = fs::read_dir(base) else {
        return (None, String::from("N/A"));
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let supply_type = fs::read_to_string(path.join("type")).unwrap_or_default();
        if supply_type.trim() == "Battery" {
            let capacity = fs::read_to_string(path.join("capacity"))
                .ok()
                .and_then(|value| value.trim().parse::<u16>().ok())
                .map(|v| v.min(100));
            let status = fs::read_to_string(path.join("status"))
                .map(|value| value.trim().to_string())
                .unwrap_or_else(|_| String::from("Unknown"));
            return (capacity, status);
        }
    }
    (None, String::from("No battery"))
}

#[must_use]
pub fn read_network_totals() -> Option<HashMap<String, (u64, u64)>> {
    let content = fs::read_to_string("/proc/net/dev").ok()?;
    Some(parse_network_totals(&content))
}

fn parse_network_totals(content: &str) -> HashMap<String, (u64, u64)> {
    let mut interfaces = HashMap::new();

    for line in content.lines().skip(2) {
        let Some((iface, data)) = line.split_once(':') else {
            continue;
        };
        let iface = iface.trim();
        if iface == "lo" {
            continue;
        }
        let values: Vec<u64> = data
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if values.len() < 9 {
            continue;
        }
        interfaces.insert(iface.to_string(), (values[0], values[8]));
    }

    interfaces
}

#[must_use]
pub fn read_disk_io_totals() -> Option<HashMap<String, (u64, u64)>> {
    let content = fs::read_to_string("/proc/diskstats").ok()?;
    Some(parse_disk_io_totals(&content))
}

fn parse_disk_io_totals(content: &str) -> HashMap<String, (u64, u64)> {
    let mut devices = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // 2.6+: major minor name + 11 stats (2.6) … +discard/flush fields (4.18+).
        // Sectors read/write sit at indices 5 and 9 in both layouts.
        if parts.len() < 14 {
            continue;
        }
        let name = parts[2].to_string();
        if name.starts_with("loop")
            || name.starts_with("dm-")
            || name.starts_with("ram")
            || name.starts_with("zram")
        {
            continue;
        }
        let rd_sectors: u64 = parts[5].parse().unwrap_or(0);
        let wr_sectors: u64 = parts[9].parse().unwrap_or(0);
        if rd_sectors > 0 || wr_sectors > 0 {
            devices.insert(name, (rd_sectors, wr_sectors));
        }
    }

    devices
}

#[must_use]
pub fn read_temperature() -> Option<f64> {
    let base = Path::new("/sys/class/thermal");
    let Ok(entries) = fs::read_dir(base) else {
        return None;
    };
    let mut fallback: Option<f64> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let thermal_type = fs::read_to_string(path.join("type"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        // Never trust battery/charger zones as system temperature.
        if thermal_type.contains("battery") || thermal_type.contains("charger") {
            continue;
        }
        let Ok(temp_str) = fs::read_to_string(path.join("temp")) else {
            continue;
        };
        let Ok(temp_raw) = temp_str.trim().parse::<f64>() else {
            continue;
        };
        let temp_c = temp_raw / 1000.0;
        if !(0.0..=150.0).contains(&temp_c) {
            continue;
        }
        // Pass 1: CPU-ish zones win; pass 2: first sane zone of any type
        // (covers acpitz, soc-thermal, Pi `cpu-thermal`, etc.).
        if thermal_type == "x86_pkg_temp" || thermal_type.contains("cpu") {
            return Some(temp_c);
        }
        if fallback.is_none() {
            fallback = Some(temp_c);
        }
    }
    fallback
}

#[must_use]
pub fn read_gpu_info() -> Option<Vec<GpuInfo>> {
    let entries = fs::read_dir("/sys/class/drm").ok()?;
    let mut gpus = Vec::new();

    for entry in entries.flatten() {
        let card_path = entry.path();
        let Some(card) = card_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_drm_card(card) {
            continue;
        }

        let device_path = card_path.join("device");
        if !device_path.exists() {
            continue;
        }

        let vendor_id = read_trimmed_path(device_path.join("vendor"));
        let device_id = read_trimmed_path(device_path.join("device"));
        let uevent = fs::read_to_string(device_path.join("uevent")).unwrap_or_default();
        let driver = uevent_value(&uevent, "DRIVER").unwrap_or_else(|| String::from("unknown"));
        let pci_slot = uevent_value(&uevent, "PCI_SLOT_NAME").unwrap_or_else(|| {
            card_path
                .canonicalize()
                .ok()
                .and_then(|path| {
                    path.ancestors()
                        .find_map(|part| part.file_name()?.to_str()?.contains(':').then_some(part))
                        .and_then(|part| part.file_name()?.to_str().map(str::to_string))
                })
                .unwrap_or_else(|| String::from("unknown"))
        });
        let vendor = vendor_name(vendor_id.as_deref());
        let model = read_lspci_model(&pci_slot).unwrap_or_else(|| {
            format!(
                "{} {}",
                vendor,
                device_id.unwrap_or_else(|| String::from("unknown device"))
            )
        });
        let kind = gpu_kind(vendor_id.as_deref(), &pci_slot);
        let power_state = read_trimmed_path(device_path.join("power_state"))
            .or_else(|| read_trimmed_path(device_path.join("power/runtime_status")))
            .unwrap_or_else(|| String::from("unknown"));
        let (temp_c, sensor_source) = read_gpu_temperature(card, &device_path, &driver);
        let usage_pct = read_gpu_usage(&device_path);
        let power_w = read_gpu_power_w(&device_path, &driver);
        let frequency_mhz = read_sys_u64_path(card_path.join("gt_cur_freq_mhz"))
            .or_else(|| read_sys_u64_path(card_path.join("gt_act_freq_mhz")));
        let max_frequency_mhz = read_sys_u64_path(card_path.join("gt_max_freq_mhz"))
            .or_else(|| read_sys_u64_path(card_path.join("gt_RP0_freq_mhz")));
        let rc6_residency_ms = read_sys_u64_path(card_path.join("power/rc6_residency_ms"));
        let memory_used_mb = read_sys_u64_path(device_path.join("mem_info_vram_used"))
            .map(|value| value as f64 / 1024.0 / 1024.0);
        let memory_total_mb = read_sys_u64_path(device_path.join("mem_info_vram_total"))
            .map(|value| value as f64 / 1024.0 / 1024.0);

        gpus.push(GpuInfo {
            card: card.to_string(),
            vendor,
            model,
            driver,
            kind,
            pci_slot,
            temp_c,
            usage_pct,
            power_w,
            frequency_mhz,
            max_frequency_mhz,
            rc6_residency_ms,
            memory_used_mb,
            memory_total_mb,
            power_state,
            sensor_source,
        });
    }

    gpus.sort_by(|a, b| a.card.cmp(&b.card));
    Some(gpus)
}

#[must_use]
pub fn read_systemd_failed_units(limit: usize) -> Option<Vec<SystemdUnitIssue>> {
    let output = Command::new("systemctl")
        .args(["list-units", "--state=failed", "--no-legend", "--plain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let content = String::from_utf8_lossy(&output.stdout);
    let mut issues = Vec::new();
    for line in content.lines().take(limit) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            issues.push(SystemdUnitIssue {
                unit: parts[0].to_string(),
                load: parts[1].to_string(),
                active: parts[2].to_string(),
                sub: parts[3].to_string(),
                description: parts[4..].join(" "),
            });
        }
    }
    Some(issues)
}

#[must_use]
pub fn read_storage_health() -> Option<Vec<StorageHealth>> {
    let entries = fs::read_dir("/sys/block").ok()?;
    let mut drives = Vec::new();

    for entry in entries.flatten() {
        let block_path = entry.path();
        let Some(device) = block_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_virtual_block_device(device) {
            continue;
        }

        let device_path = block_path.join("device");
        let model = read_trimmed_path(device_path.join("model"))
            .or_else(|| read_trimmed_path(device_path.join("name")))
            .unwrap_or_else(|| String::from("unknown"));
        let rotational = read_trimmed_path(block_path.join("queue/rotational"))
            .map(|value| value == "1")
            .unwrap_or(false);
        let kind = if device.starts_with("nvme") {
            "NVMe"
        } else if rotational {
            "HDD"
        } else {
            "SSD"
        }
        .to_string();
        let temp_c = read_block_device_temp_c(&device_path);
        let risk = match temp_c {
            Some(temp) if temp >= 85.0 => Severity::Critical,
            Some(temp) if temp >= 70.0 => Severity::Warn,
            _ => Severity::Ok,
        };
        let note = match (risk, temp_c) {
            (Severity::Critical, Some(temp)) => format!("temperature critical at {temp:.0}C"),
            (Severity::Warn, Some(temp)) => format!("temperature elevated at {temp:.0}C"),
            (_, Some(_)) => String::from("temperature sensor OK"),
            _ => String::from("basic sysfs health; no SMART sensor exposed"),
        };

        drives.push(StorageHealth {
            device: device.to_string(),
            model,
            kind,
            temp_c,
            critical_warning: None,
            media_errors: None,
            risk,
            note,
        });
    }

    drives.sort_by(|a, b| a.device.cmp(&b.device));
    Some(drives)
}

fn is_virtual_block_device(device: &str) -> bool {
    device.starts_with("loop")
        || device.starts_with("ram")
        || device.starts_with("dm-")
        || device.starts_with("zram")
}

fn read_block_device_temp_c(device_path: &Path) -> Option<f64> {
    let hwmon_root = device_path.join("hwmon");
    let entries = fs::read_dir(hwmon_root).ok()?;
    for entry in entries.flatten() {
        let hwmon = entry.path();
        for index in 1..=8 {
            let temp_path = hwmon.join(format!("temp{index}_input"));
            if let Some(raw) = read_sys_u64_path(&temp_path) {
                let temp_c = raw as f64 / 1000.0;
                if (0.0..=150.0).contains(&temp_c) {
                    return Some(temp_c);
                }
            }
        }
    }
    None
}

fn is_drm_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn read_trimmed_path(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_sys_u64_path(path: impl AsRef<Path>) -> Option<u64> {
    read_trimmed_path(path)?.parse().ok()
}

fn uevent_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then(|| value.trim().to_string())
    })
}

fn vendor_name(vendor_id: Option<&str>) -> String {
    match vendor_id.unwrap_or_default().to_ascii_lowercase().as_str() {
        "0x8086" => String::from("Intel"),
        "0x1002" | "0x1022" => String::from("AMD"),
        "0x10de" => String::from("NVIDIA"),
        "0x1ed5" => String::from("Moore Threads"),
        id if !id.is_empty() => id.to_string(),
        _ => String::from("Unknown"),
    }
}

fn gpu_kind(vendor_id: Option<&str>, pci_slot: &str) -> String {
    if vendor_id.is_some_and(|id| id.eq_ignore_ascii_case("0x8086"))
        || pci_slot.ends_with(":00:02.0")
    {
        String::from("iGPU")
    } else {
        String::from("dGPU")
    }
}

fn read_lspci_model(pci_slot: &str) -> Option<String> {
    if pci_slot == "unknown" {
        return None;
    }
    let cache = get_pci_cache();
    cache.get(pci_slot).cloned()
}

fn get_pci_cache() -> &'static std::collections::HashMap<String, String> {
    use std::sync::OnceLock;
    static PCI_CACHE: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
    PCI_CACHE.get_or_init(|| {
        let mut cache = std::collections::HashMap::new();
        if let Ok(output) = Command::new("lspci").args(["-mm", "-D"]).output() {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                for line in content.lines() {
                    let fields = quoted_fields(line);
                    if fields.len() >= 3 {
                        let slot = fields[0].clone();
                        let vendor = fields.get(1).cloned().unwrap_or_default();
                        let device = fields.get(2).cloned().unwrap_or_default();
                        cache.insert(slot, format!("{vendor} {device}"));
                    }
                }
            }
        }
        cache
    })
}

fn quoted_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        match ch {
            '"' if in_quote => {
                fields.push(std::mem::take(&mut current));
                in_quote = false;
            }
            '"' => in_quote = true,
            _ if in_quote => current.push(ch),
            _ => {}
        }
    }
    fields
}

fn read_gpu_temperature(card: &str, device_path: &Path, driver: &str) -> (Option<f64>, String) {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return (None, String::from("No hwmon"));
    };
    let device_real = device_path.canonicalize().ok();
    let mut fallback = None;

    for entry in entries.flatten() {
        let hwmon_path = entry.path();
        let name = read_trimmed_path(hwmon_path.join("name")).unwrap_or_default();
        let hwmon_real = hwmon_path.canonicalize().ok();
        let is_same_device = device_real
            .as_ref()
            .zip(hwmon_real.as_ref())
            .is_some_and(|(device, hwmon)| hwmon.starts_with(device));
        let name_matches = matches!(
            name.as_str(),
            "amdgpu" | "nouveau" | "nvidia" | "i915" | "xe"
        ) || (!driver.is_empty() && name.eq_ignore_ascii_case(driver));

        if !is_same_device && !name_matches {
            continue;
        }

        if let Some(temp) = read_labeled_hwmon_temp(&hwmon_path, true) {
            return (Some(temp), format!("{card}/{name}"));
        }
        fallback = fallback.or_else(|| read_labeled_hwmon_temp(&hwmon_path, false));
    }

    if let Some(temp) = fallback {
        (Some(temp), format!("{card}/hwmon"))
    } else if driver == "i915" || driver == "xe" {
        (None, String::from("No dedicated iGPU temp"))
    } else {
        (None, String::from("Sensor N/A"))
    }
}

fn read_labeled_hwmon_temp(hwmon_path: &Path, prefer_gpu_label: bool) -> Option<f64> {
    let mut best = None;
    for idx in 1..=16 {
        let input = hwmon_path.join(format!("temp{idx}_input"));
        let Some(raw) = read_sys_u64_path(input) else {
            continue;
        };
        let temp = raw as f64 / 1000.0;
        if !(5.0..=125.0).contains(&temp) {
            continue;
        }

        let label = read_trimmed_path(hwmon_path.join(format!("temp{idx}_label")))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let is_gpu_label = label.contains("gpu")
            || label.contains("edge")
            || label.contains("junction")
            || label.contains("hotspot")
            || label.contains("mem");

        if prefer_gpu_label && is_gpu_label {
            return Some(temp);
        }
        if !prefer_gpu_label {
            best = best.or(Some(temp));
        }
    }
    best
}

fn read_gpu_usage(device_path: &Path) -> Option<f64> {
    let usage = read_sys_u64_path(device_path.join("gpu_busy_percent"))?;
    Some((usage as f64).clamp(0.0, 100.0))
}

fn read_gpu_power_w(device_path: &Path, driver: &str) -> Option<f64> {
    let entries = fs::read_dir("/sys/class/hwmon").ok()?;
    let device_real = device_path.canonicalize().ok();
    for entry in entries.flatten() {
        let hwmon_path = entry.path();
        let name = read_trimmed_path(hwmon_path.join("name")).unwrap_or_default();
        let hwmon_real = hwmon_path.canonicalize().ok();
        let is_same_device = device_real
            .as_ref()
            .zip(hwmon_real.as_ref())
            .is_some_and(|(device, hwmon)| hwmon.starts_with(device));
        let name_matches = matches!(name.as_str(), "amdgpu" | "nvidia")
            || (!driver.is_empty() && name.eq_ignore_ascii_case(driver));
        if !is_same_device && !name_matches {
            continue;
        }
        for file_name in ["power1_average", "power1_input"] {
            if let Some(value) = read_sys_u64_path(hwmon_path.join(file_name)) {
                return Some(value as f64 / 1_000_000.0);
            }
        }
    }
    None
}

#[must_use]
pub fn read_process_summary(
    limit: usize,
    prev_totals: &HashMap<u32, u64>,
    cpu_delta: u64,
) -> Option<ProcessSummary> {
    let entries = fs::read_dir("/proc").ok()?;
    // Pre-size from the previous tick (validated: HashMap::with_capacity + clear
    // reuse keeps allocation across ticks instead of reallocating).
    let mut processes = Vec::with_capacity(prev_totals.len().max(64));
    let mut current_totals = HashMap::with_capacity(prev_totals.len().max(64));
    let mut zombie_count = 0;
    let page_size = page_size_bytes();

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(pid_str) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };

        let Ok(stat_content) = fs::read_to_string(path.join("stat")) else {
            continue;
        };
        let Some(process) =
            parse_process_stat(pid, &stat_content, prev_totals, cpu_delta, page_size)
        else {
            continue;
        };

        if process.info.state == "Z" {
            zombie_count += 1;
        }
        let total_time = process.total_time;
        let info = process.info;
        current_totals.insert(pid, total_time);
        processes.push(info);
    }

    let count = current_totals.len();
    let limit = limit.max(1).min(processes.len());
    let (top_cpu, top_mem) = if processes.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let mut top_cpu = top_by(processes.as_mut_slice(), limit, compare_cpu_desc).to_vec();
        top_cpu.sort_unstable_by(compare_cpu_desc);

        let mut top_mem = top_by(processes.as_mut_slice(), limit, compare_mem_desc).to_vec();
        top_mem.sort_unstable_by(compare_mem_desc);
        (top_cpu, top_mem)
    };

    Some(ProcessSummary {
        count,
        top_cpu,
        top_mem,
        current_totals,
        zombie_count,
    })
}

struct ParsedProcess {
    info: ProcessInfo,
    total_time: u64,
}

fn parse_process_stat(
    pid: u32,
    content: &str,
    prev_totals: &HashMap<u32, u64>,
    cpu_delta: u64,
    page_size: f64,
) -> Option<ParsedProcess> {
    let open = content.find('(')?;
    let close = content.rfind(')')?;
    if close <= open {
        return None;
    }

    let name = content[open + 1..close].to_owned();
    // Walk fields without collecting into a Vec (one fewer heap alloc per process).
    // Layout after comm: 0 state, 1 ppid, 2 pgrp, 3 session, 4 tty, 5 tpgid, 6 flags,
    // 7 minflt, 8 cminflt, 9 majflt, 10 cmajflt, 11 utime, 12 stime, 13 cutime,
    // 14 cstime, 15 priority, 16 nice, 17 threads, 18 itreal, 19 starttime,
    // 20 vsize, 21 rss.
    let mut fields = content[close + 1..].split_whitespace();
    let state = fields.next()?.to_owned();
    let ppid: u32 = fields.next()?.parse().unwrap_or(0);
    for _ in 0..9 {
        fields.next()?;
    }
    let utime: u64 = fields.next()?.parse().unwrap_or(0);
    let stime: u64 = fields.next()?.parse().unwrap_or(0);
    let total_time = utime + stime;
    let cpu_pct = if cpu_delta > 0 {
        let prev = prev_totals.get(&pid).unwrap_or(&total_time);
        let diff = total_time.saturating_sub(*prev);
        (diff as f64 / cpu_delta as f64) * 100.0
    } else {
        0.0
    };
    for _ in 0..4 {
        fields.next()?;
    }
    let threads: u32 = fields.next()?.parse().unwrap_or(1);
    for _ in 0..3 {
        fields.next()?;
    }
    let rss_pages: f64 = fields.next()?.parse().unwrap_or(0.0);
    let mem_mb = (rss_pages * page_size) / 1024.0 / 1024.0;

    Some(ParsedProcess {
        info: ProcessInfo {
            pid,
            ppid,
            name,
            cpu_pct,
            mem_mb,
            threads,
            state,
            reason: String::new(),
            is_high_risk: false,
            is_dev: false,
        },
        total_time,
    })
}

fn top_by(
    processes: &mut [ProcessInfo],
    limit: usize,
    compare: fn(&ProcessInfo, &ProcessInfo) -> Ordering,
) -> &[ProcessInfo] {
    if limit >= processes.len() {
        return processes;
    }
    let (top, _, _) = processes.select_nth_unstable_by(limit, compare);
    top
}

fn compare_cpu_desc(a: &ProcessInfo, b: &ProcessInfo) -> Ordering {
    b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(Ordering::Equal)
}

fn compare_mem_desc(a: &ProcessInfo, b: &ProcessInfo) -> Ordering {
    b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(Ordering::Equal)
}

fn page_size_bytes() -> f64 {
    // SAFETY: sysconf(_SC_PAGESIZE) is a POSIX read-only query that returns
    // the system page size. It never writes memory and is safe to call.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size > 0 {
        page_size as f64
    } else {
        4096.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_sample_idle_and_total_jiffies() {
        let sample = parse_cpu_sample("cpu  4705 0 2253 105934 136 0 45 0 0 0").unwrap();

        assert_eq!(sample.idle, 105_934 + 136);
        assert_eq!(sample.total, 4705 + 2253 + 105_934 + 136 + 45);
    }

    #[test]
    fn parses_meminfo_using_memavailable_and_swapfree() {
        let mem = parse_mem_info(
            "MemTotal:       8192000 kB\n\
             MemFree:        1000000 kB\n\
             MemAvailable:   6144000 kB\n\
             SwapTotal:      2097152 kB\n\
             SwapFree:       1048576 kB\n",
        )
        .unwrap();

        assert_eq!(mem.total_mb, 8000.0);
        assert_eq!(mem.used_mb, 2000.0);
        assert_eq!(mem.swap_total_mb, 2048.0);
        assert_eq!(mem.swap_used_mb, 1024.0);
    }

    #[test]
    fn parses_tcp_listen_socket_and_ignores_established() {
        let content = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
                       0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 1\n\
                       1: 0100007F:0050 0200007F:CAFE 01 00000000:00000000 00:00000000 00000000 1000 0 2\n";
        let mut ports = Vec::new();

        parse_socket_file(content, "TCP", &mut ports);

        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].0.ip, "127.0.0.1");
        assert_eq!(ports[0].0.port, 8080);
        assert_eq!(ports[0].0.state, "LISTEN");
        assert_eq!(ports[0].0.service_name, "HTTP Alt/Java");
        assert_eq!(ports[0].1, 1);
    }

    #[test]
    fn parses_tcp6_listen_socket_and_ignores_established() {
        let content = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
                       0: 00000000000000000000000001000000:1F90 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 1\n\
                       1: 00000000000000000000000000000000:0050 00000000000000000000000000000000:0000 01 00000000:00000000 00:00000000 00000000 1000 0 2\n";
        let mut ports = Vec::new();

        parse_socket_file(content, "TCP6", &mut ports);

        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].0.ip, "::1");
        assert_eq!(ports[0].0.port, 8080);
        assert_eq!(ports[0].0.state, "LISTEN");
        assert_eq!(ports[0].1, 1);
    }

    #[test]
    fn parses_tcp6_wildcard_address() {
        assert_eq!(
            parse_hex_ip6("00000000000000000000000000000000").unwrap(),
            "::"
        );
        assert_eq!(
            parse_hex_ip6("00000000000000000000000001000000").unwrap(),
            "::1"
        );
        assert!(parse_hex_ip6("0100007F").is_err());
        assert!(parse_hex_ip6("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }

    #[test]
    fn parses_network_totals_and_skips_loopback() {
        let totals = parse_network_totals(
            "Inter-|   Receive                                                |  Transmit\n\
             face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n\
                lo: 100 1 0 0 0 0 0 0 200 2 0 0 0 0 0 0\n\
              eth0: 1234 10 0 0 0 0 0 0 5678 20 0 0 0 0 0 0\n",
        );

        assert_eq!(totals.len(), 1);
        assert_eq!(totals.get("eth0"), Some(&(1234, 5678)));
    }

    #[test]
    fn parses_diskstats_and_filters_virtual_devices() {
        let totals = parse_disk_io_totals(
            "   8       0 sda 1 2 300 4 5 6 700 8 9 10 11\n\
                7       0 loop0 1 2 300 4 5 6 700 8 9 10 11\n\
              253       0 dm-0 1 2 300 4 5 6 700 8 9 10 11\n",
        );

        assert_eq!(totals.len(), 1);
        assert_eq!(totals.get("sda"), Some(&(300, 700)));
    }

    #[test]
    fn parses_process_stat_with_spaces_in_command_name() {
        let mut previous = HashMap::new();
        previous.insert(123, 100);
        let parsed = parse_process_stat(
            123,
            "123 (worker process) S 0 0 0 0 0 0 0 0 0 0 100 50 0 0 20 0 4 0 0 0 256",
            &previous,
            200,
            4096.0,
        )
        .unwrap();

        assert_eq!(parsed.info.pid, 123);
        assert_eq!(parsed.info.ppid, 0);
        assert_eq!(parsed.info.name, "worker process");
        assert_eq!(parsed.info.state, "S");
        assert_eq!(parsed.info.threads, 4);
        assert_eq!(parsed.total_time, 150);
        assert_eq!(parsed.info.cpu_pct, 25.0);
        assert_eq!(parsed.info.mem_mb, 1.0);
    }

    #[test]
    fn meminfo_falls_back_without_memavailable() {
        // Pre-3.14 kernels lack MemAvailable: emulate `free` as
        // free + buffers + cache.
        let mem = parse_mem_info(
            "MemTotal:       8192000 kB\n\
             MemFree:        1024000 kB\n\
             Buffers:         204800 kB\n\
             Cached:         3072000 kB\n\
             SwapTotal:      2097152 kB\n\
             SwapFree:       2097152 kB\n",
        )
        .unwrap();

        assert_eq!(mem.total_mb, 8000.0);
        // used = 8000 - (1000 + 200 + 3000) = 3800 MB, never 100%.
        assert!((mem.used_mb - 3800.0).abs() < 1.0, "used={}", mem.used_mb);
    }

    #[test]
    fn cpuinfo_parses_raspberry_pi_arm() {
        let (model, count) = parse_cpuinfo(
            "processor\t: 0\n\
             Hardware\t: BCM2711\n\
             Revision\t: c03111\n\
             Serial\t\t: 10000000179dda37\n\
             Model\t\t: Raspberry Pi 4 Model B Rev 1.1\n",
        );

        assert!(model.contains("Raspberry Pi 4"), "model={model}");
        assert_eq!(count, 1);
    }

    #[test]
    fn cpuinfo_parses_x86_and_counts_cores() {
        let (model, count) = parse_cpuinfo(
            "processor\t: 0\n\
             model name\t: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz\n\
             processor\t: 1\n\
             model name\t: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz\n",
        );

        assert!(model.contains("i7-9700K"), "model={model}");
        assert_eq!(count, 2);
    }

    #[test]
    fn os_release_prefers_pretty_name_fallback() {
        let content = "PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\nID=debian\n";
        let name = os_release_value(content, "NAME")
            .or_else(|| os_release_value(content, "PRETTY_NAME"))
            .unwrap();
        assert!(name.contains("Debian"), "name={name}");
    }

    #[test]
    fn primary_mount_prefers_root_over_data() {
        let mounts = vec![
            DiskInfo {
                mount_point: String::from("/data"),
                fs_type: String::from("ext4"),
                used_gb: 1.0,
                total_gb: 10.0,
                free_gb: 9.0,
                pct: 10,
            },
            DiskInfo {
                mount_point: String::from("/"),
                fs_type: String::from("ext4"),
                used_gb: 5.0,
                total_gb: 50.0,
                free_gb: 45.0,
                pct: 10,
            },
        ];

        assert_eq!(primary_mount(&mounts).unwrap().mount_point, "/");
        assert!(primary_mount(&[]).is_none());
    }

    #[test]
    fn parses_process_stat_extracts_ppid() {
        let mut previous = HashMap::new();
        previous.insert(123, 100);
        let parsed = parse_process_stat(
            123,
            "123 (child) S 999 0 0 0 0 0 0 0 0 0 100 50 0 0 20 0 4 0 0 0 256",
            &previous,
            200,
            4096.0,
        )
        .unwrap();

        assert_eq!(parsed.info.pid, 123);
        assert_eq!(parsed.info.ppid, 999);
        assert_eq!(parsed.info.name, "child");
    }

    #[test]
    fn build_socket_inode_map_empty_set_returns_empty() {
        let empty_set = HashSet::new();
        let map = build_socket_inode_map(&empty_set);
        assert!(map.is_empty());
    }
}
