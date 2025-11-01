use std::collections::HashMap;
use std::fs;
use std::net::UdpSocket;
use std::path::Path;
use std::process::Command;

use battery::Manager;
use sysinfo::System;

use crate::common::storage::format_bytes;
use crate::system::{DistroInfo, MemoryStats};

const OS_RELEASE_PATHS: &[&str] = &["/etc/os-release", "/usr/lib/os-release"];

pub fn detect_distro() -> DistroInfo {
    if let Some(release) = read_os_release() {
        if let Some(proxmox) = detect_proxmox(&release) {
            return proxmox;
        }

        return build_distro_from_release(&release);
    }

    fallback_distro_from_os_info()
}

pub fn detect_kernel() -> String {
    System::kernel_version().unwrap_or_default()
}

pub fn detect_cpu(sys: &System) -> Option<String> {
    let cpu_brand = sys
        .cpus()
        .get(0)
        .map(|cpu| cpu.brand().to_string())
        .filter(|brand| !brand.trim().is_empty())
        .unwrap_or_else(|| "Unknown".to_string());

    let cpu_cores = sys
        .physical_core_count()
        .unwrap_or_else(|| sys.cpus().len());

    Some(format!("{} ({} cores)", cpu_brand, cpu_cores))
}

pub fn detect_memory(sys: &System) -> MemoryStats {
    MemoryStats {
        total: Some(format_bytes(sys.total_memory() / 1024)),
        used: Some(format_bytes(sys.used_memory() / 1024)),
        swap_total: Some(format_bytes(sys.total_swap() / 1024)),
        swap_used: Some(format_bytes(sys.used_swap() / 1024)),
    }
}

pub fn format_uptime(uptime_secs: u64) -> Option<String> {
    let days = uptime_secs / 86_400;
    let hours = (uptime_secs % 86_400) / 3_600;
    let minutes = (uptime_secs % 3_600) / 60;
    Some(format!("{}d {}h {}m", days, hours, minutes))
}

pub fn detect_local_ip() -> Option<String> {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .ok()
        .map(|addr| addr.ip().to_string())
}

pub fn detect_battery() -> Option<String> {
    let manager = Manager::new().ok()?;
    let mut batteries = manager.batteries().ok()?;
    let battery = batteries.next()?.ok()?;

    let percent = battery
        .state_of_charge()
        .get::<battery::units::ratio::percent>();

    let plugged = if matches!(
        battery.state(),
        battery::State::Charging | battery::State::Full
    ) {
        "[AC Connected]"
    } else {
        "[Discharging]"
    };

    Some(format!("{}% {}", percent.round(), plugged))
}

pub fn detect_username() -> Option<String> {
    Some(whoami::username())
}

pub fn detect_hostname() -> Option<String> {
    whoami::fallible::hostname().ok()
}

pub fn detect_cpu_max_frequency(_sys: &System) -> Option<String> {
    if let Some(freq_khz) = read_cpufreq_max_khz() {
        return Some(format!("{:.2}GHz", freq_khz as f64 / 1_000_000.0));
    }

    if let Some(freq_mhz) = read_lscpu_max_mhz() {
        return Some(format!("{:.2}GHz", freq_mhz as f64 / 1_000.0));
    }

    None
}

fn read_os_release() -> Option<HashMap<String, String>> {
    let path = OS_RELEASE_PATHS
        .iter()
        .map(Path::new)
        .find(|candidate| candidate.exists())?;

    parse_key_values(fs::read_to_string(path).ok()?)
}

fn parse_key_values(content: String) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };

        let value = trimmed_value(value);
        map.insert(key.to_string(), value);
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn trimmed_value(value: &str) -> String {
    let mut trimmed = value.trim();

    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed = &trimmed[1..trimmed.len() - 1];
    }

    trimmed.to_string()
}

fn detect_proxmox(release: &HashMap<String, String>) -> Option<DistroInfo> {
    let id = release.get("ID").map(String::as_str);
    let id_like = release.get("ID_LIKE").map(String::as_str).unwrap_or("");

    let looks_like_proxmox = id == Some("pve")
        || id == Some("proxmox")
        || id_like
            .split_whitespace()
            .any(|token| token == "proxmox" || token == "pve")
        || Path::new("/usr/bin/pveversion").exists()
        || Path::new("/usr/sbin/pveversion").exists()
        || Path::new("/etc/pve").exists();

    if !looks_like_proxmox {
        return None;
    }

    let version = proxmox_version(release);
    let name = version
        .map(|v| format!("Proxmox VE {}", v))
        .unwrap_or_else(|| "Proxmox VE".to_string());

    Some(DistroInfo {
        name,
        id: "proxmox".to_string(),
    })
}

fn proxmox_version(release: &HashMap<String, String>) -> Option<String> {
    if let Some(version) = release.get("VERSION_ID").map(String::as_str) {
        let trimmed = version.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    Command::new("pveversion").output().ok().and_then(|output| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().find_map(|line| {
            let head = line.split_whitespace().next()?;
            let version = head.split('/').nth(1)?;
            let trimmed = version.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    })
}

fn build_distro_from_release(release: &HashMap<String, String>) -> DistroInfo {
    let mut name = release
        .get("PRETTY_NAME")
        .cloned()
        .or_else(|| release.get("NAME").cloned())
        .unwrap_or_else(|| "Linux".to_string());

    if let Some(variant) = release.get("VARIANT") {
        let variant = variant.trim();
        if !variant.is_empty() && !name.to_lowercase().contains(&variant.to_lowercase()) {
            name.push_str(" (");
            name.push_str(variant);
            name.push(')');
        }
    }

    let id = release
        .get("ID")
        .map(|raw| normalize_logo_id(raw))
        .filter(|id| !id.is_empty())
        .or_else(|| {
            release.get("ID_LIKE").and_then(|raw| {
                raw.split_whitespace()
                    .map(|candidate| normalize_logo_id(candidate))
                    .find(|candidate| !candidate.is_empty())
            })
        })
        .unwrap_or_else(|| normalize_logo_id(&name));

    DistroInfo { name, id }
}

fn fallback_distro_from_os_info() -> DistroInfo {
    let info = os_info::get();
    let os_type = info.os_type().to_string();
    let mut version = info.version().to_string();
    if version.eq_ignore_ascii_case("unknown") {
        version.clear();
    }

    let name = if version.is_empty() {
        os_type.clone()
    } else {
        format!("{} {}", os_type, version)
    };

    let id = normalize_logo_id(&os_type);

    DistroInfo { name, id }
}

fn normalize_logo_id(raw: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            last_was_separator = false;
        } else if matches!(ch, ' ' | '-' | '_' | '.') {
            if !last_was_separator && !normalized.is_empty() {
                normalized.push('_');
                last_was_separator = true;
            }
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized
    }
}

fn read_cpufreq_max_khz() -> Option<u64> {
    let contents =
        fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq").ok()?;
    contents.trim().parse::<u64>().ok()
}

fn read_lscpu_max_mhz() -> Option<u64> {
    let output = Command::new("lscpu").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(value) = line.trim().strip_prefix("CPU max MHz:") {
            if let Ok(freq) = value.trim().parse::<f64>() {
                return Some(freq.round() as u64);
            }
        }
    }
    None
}
