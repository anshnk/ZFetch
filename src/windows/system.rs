use std::net::UdpSocket;
use std::process::Command;

use battery::Manager;
use sysinfo::System;

use crate::common::storage::format_bytes;
use crate::system::{DistroInfo, MemoryStats};

pub fn detect_distro() -> DistroInfo {
    let info = os_info::get();
    let raw_type = info.os_type().to_string();
    let mut version = info.version().to_string();

    if version.eq_ignore_ascii_case("unknown") {
        version.clear();
    }

    let name = if version.is_empty() {
        raw_type.clone()
    } else {
        format!("{} {}", raw_type, version)
    };

    let raw_type_lower = raw_type.to_lowercase();
    let id = if raw_type_lower.contains("windows") {
        if version.starts_with("10.0.22") || version.contains("11") {
            "windows_11".to_string()
        } else if version.contains("2025") {
            "windows_2025".to_string()
        } else {
            "windows".to_string()
        }
    } else {
        raw_type_lower.replace(' ', "_")
    };

    DistroInfo { name, id }
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

pub fn detect_cpu_max_frequency(sys: &System) -> Option<String> {
    if let Some(freq_mhz) = query_wmic_max_clock_speed() {
        return Some(format!("{:.2}GHz", freq_mhz as f64 / 1000.0));
    }

    let freq = sys.global_cpu_info().frequency();
    if freq == 0 {
        None
    } else {
        Some(format!("{:.2}GHz", freq as f64 / 1000.0))
    }
}

fn query_wmic_max_clock_speed() -> Option<u64> {
    let output = Command::new("wmic")
        .args(&["cpu", "get", "MaxClockSpeed", "/value"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(value) = line.trim().strip_prefix("MaxClockSpeed=") {
            if let Ok(freq) = value.trim().parse::<u64>() {
                return Some(freq);
            }
        }
    }

    None
}
