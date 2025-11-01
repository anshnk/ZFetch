use crate::config::Config;
use std::cmp::Ordering;
use std::future::Future;
use std::pin::Pin;
use tokio::task::JoinHandle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageInfo {
    pub name: String,
    pub total: String,
    pub used: String,
    pub percent: u8,
    pub fs_type: String,
    pub readonly: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemInfo {
    pub distro: String,
    pub distro_id: String,
    pub kernel: String,
    pub cpu: Option<String>,
    pub cpu_max_frequency: Option<String>,
    pub gpus: Vec<GpuInfo>,
    pub total_memory: Option<String>,
    pub used_memory: Option<String>,
    pub total_swap: Option<String>,
    pub used_swap: Option<String>,
    pub uptime: Option<String>,
    pub local_ip: Option<String>,
    pub battery: Option<String>,
    pub storage: Vec<StorageInfo>,
    pub username: Option<String>,
    pub hostname: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistroInfo {
    pub name: String,
    pub id: String,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryStats {
    pub total: Option<String>,
    pub used: Option<String>,
    pub swap_total: Option<String>,
    pub swap_used: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuInfo {
    pub name: String,
    pub kind: GpuKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuKind {
    Integrated,
    Discrete,
}

impl GpuKind {
    pub fn label(&self) -> &'static str {
        match self {
            GpuKind::Integrated => "Integrated",
            GpuKind::Discrete => "Discrete",
        }
    }
}

pub fn classify_gpu_name(name: &str) -> GpuKind {
    let lower = name.to_lowercase();

    if lower.contains("intel")
        || lower.contains("uhd")
        || lower.contains("iris")
        || lower.contains("igpu")
        || lower.contains("applesilicon")
        || lower.contains("apple m")
        || lower.contains("apu")
        || lower.contains("integrated")
        || lower.contains("graphics controller")
    {
        return GpuKind::Integrated;
    }

    if lower.contains("radeon") {
        if lower.contains("rx") || lower.contains("xt") || lower.contains("pro") {
            return GpuKind::Discrete;
        }
        if lower.contains("vega") || lower.contains("graphics") {
            return GpuKind::Integrated;
        }
    }

    if lower.contains("nvidia")
        || lower.contains("geforce")
        || lower.contains("quadro")
        || lower.contains("tesla")
        || lower.contains("rtx")
        || lower.contains("gtx")
    {
        return GpuKind::Discrete;
    }

    if lower.contains("apple")
        && (lower.contains("gpu") || lower.contains("graphics") || lower.contains("m1"))
    {
        return GpuKind::Integrated;
    }

    GpuKind::Discrete
}

pub fn normalize_gpu_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown".to_string();
    }

    let without_revision = trimmed.split(" (rev").next().unwrap_or(trimmed).trim();
    let without_subsystem = without_revision
        .split(" [Subsystem")
        .next()
        .unwrap_or(without_revision)
        .trim();

    let vendor = detect_gpu_vendor(without_subsystem);

    if let Some(inner) = extract_bracket_name(without_subsystem) {
        let inner_trimmed = inner.trim();
        if let Some(vendor) = vendor {
            if inner_trimmed
                .to_lowercase()
                .starts_with(&vendor.to_lowercase())
            {
                return inner_trimmed.to_string();
            }
            return format!("{} {}", vendor, inner_trimmed);
        }
        return inner_trimmed.to_string();
    }

    let mut cleaned = without_subsystem
        .replace("Corporation", "")
        .replace("Corp.", "")
        .replace("Inc.", "")
        .replace("Ltd.", "")
        .replace("Company", "")
        .replace("Device", "")
        .replace("Technologies", "");

    cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if let Some(vendor) = vendor {
        let lower_cleaned = cleaned.to_lowercase();
        if lower_cleaned.starts_with(&vendor.to_lowercase()) {
            cleaned
        } else if cleaned.is_empty() {
            vendor.to_string()
        } else {
            format!("{} {}", vendor, cleaned)
        }
    } else {
        cleaned
    }
}

fn detect_gpu_vendor(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    let has_token = |needle: &str| tokens.iter().any(|token| *token == needle);

    if has_token("nvidia") || has_token("geforce") || lower.contains("geforce ") {
        Some("NVIDIA")
    } else if has_token("intel") {
        Some("Intel")
    } else if lower.contains("advanced micro devices") || has_token("amd") || has_token("ati") {
        Some("AMD")
    } else if has_token("apple") {
        Some("Apple")
    } else if has_token("qualcomm") {
        Some("Qualcomm")
    } else if has_token("arm") {
        Some("ARM")
    } else {
        None
    }
}

fn extract_bracket_name(text: &str) -> Option<&str> {
    let start = text.rfind('[')?;
    let end = text[start + 1..].find(']')?;
    Some(text[start + 1..start + 1 + end].trim())
}

#[cfg(target_os = "linux")]
use crate::linux::system as platform;
#[cfg(target_os = "macos")]
use crate::macos::system as platform;
#[cfg(target_os = "windows")]
use crate::windows::system as platform;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::{DistroInfo, MemoryStats};
    use crate::common::storage::format_bytes;
    use std::net::UdpSocket;
    use sysinfo::System;

    pub fn detect_distro() -> DistroInfo {
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

        let id = os_type.to_lowercase().replace(' ', "_");

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

    pub fn detect_cpu_max_frequency(sys: &System) -> Option<String> {
        let freq = sys.global_cpu_info().frequency();
        if freq == 0 {
            None
        } else {
            Some(format!("{:.2}GHz", freq as f64 / 1000.0))
        }
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
        None
    }

    pub fn detect_username() -> Option<String> {
        Some(whoami::username())
    }

    pub fn detect_hostname() -> Option<String> {
        whoami::fallible::hostname().ok()
    }
}

pub async fn get_system_info(config: &Config) -> SystemInfo {
    let show_cpu = config.show_cpu.unwrap_or(true);
    let show_memory = config.show_memory.unwrap_or(true);
    let show_swap = config.show_swap.unwrap_or(true);
    let show_uptime = config.show_uptime.unwrap_or(true);
    let show_gpu = config.show_gpu.unwrap_or(true);
    let show_local_ip = config.show_local_ip.unwrap_or(true);
    let show_battery = config.show_battery.unwrap_or(true);
    let show_storage = config.show_storage.unwrap_or(true);

    let sys_task = tokio::task::spawn_blocking(|| {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu();
        sys.refresh_memory();
        sys
    });

    let uptime_handle: Option<JoinHandle<u64>> = if show_uptime {
        Some(tokio::task::spawn_blocking(|| sysinfo::System::uptime()))
    } else {
        None
    };

    let gpu_handle: Option<JoinHandle<Vec<GpuInfo>>> = if show_gpu {
        build_platform_gpu_task().map(|future| {
            tokio::spawn(async move {
                let names = future.await;
                names
                    .into_iter()
                    .map(|name| {
                        let normalized = normalize_gpu_name(&name);
                        let kind = classify_gpu_name(&normalized);
                        GpuInfo {
                            kind,
                            name: normalized,
                        }
                    })
                    .collect()
            })
        })
    } else {
        None
    };

    let sys = sys_task
        .await
        .expect("refreshing system information task failed");

    let distro_info = platform::detect_distro();
    let kernel = platform::detect_kernel();

    let cpu = if show_cpu {
        platform::detect_cpu(&sys)
    } else {
        None
    };
    let cpu_max_frequency = if show_cpu {
        platform::detect_cpu_max_frequency(&sys)
    } else {
        None
    };

    let memory_stats = if show_memory || show_swap {
        Some(platform::detect_memory(&sys))
    } else {
        None
    };

    let total_memory = if show_memory {
        memory_stats.as_ref().and_then(|stats| stats.total.clone())
    } else {
        None
    };

    let used_memory = if show_memory {
        memory_stats.as_ref().and_then(|stats| stats.used.clone())
    } else {
        None
    };

    let total_swap = if show_swap {
        memory_stats
            .as_ref()
            .and_then(|stats| stats.swap_total.clone())
    } else {
        None
    };

    let used_swap = if show_swap {
        memory_stats
            .as_ref()
            .and_then(|stats| stats.swap_used.clone())
    } else {
        None
    };

    let uptime = if let Some(handle) = uptime_handle {
        match handle.await {
            Ok(secs) => platform::format_uptime(secs),
            Err(_) => None,
        }
    } else {
        None
    };

    let mut gpus = if let Some(handle) = gpu_handle {
        match handle.await {
            Ok(list) => list,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    if gpus.len() > 1 {
        gpus.sort_by(|a, b| match (&a.kind, &b.kind) {
            (GpuKind::Discrete, GpuKind::Integrated) => Ordering::Less,
            (GpuKind::Integrated, GpuKind::Discrete) => Ordering::Greater,
            _ => Ordering::Equal,
        });
    }

    let local_ip = if show_local_ip {
        platform::detect_local_ip()
    } else {
        None
    };

    let battery = if show_battery {
        platform::detect_battery()
    } else {
        None
    };

    let storage = if show_storage {
        get_storage_info().await
    } else {
        Vec::new()
    };

    let username = platform::detect_username();
    let hostname = platform::detect_hostname();

    SystemInfo {
        distro: distro_info.name,
        distro_id: distro_info.id,
        kernel,
        cpu,
        cpu_max_frequency,
        gpus,
        total_memory,
        used_memory,
        total_swap,
        used_swap,
        uptime,
        local_ip,
        battery,
        storage,
        username,
        hostname,
    }
}

#[cfg(target_os = "macos")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    crate::macos::gpu::build_gpu_task()
}

#[cfg(target_os = "windows")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    crate::windows::gpu::build_gpu_task()
}

#[cfg(target_os = "linux")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    crate::linux::gpu::build_gpu_task()
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    use crate::common::gpu::{detect_opengl_renderer, detect_vulkan_gpu};

    Some(Box::pin(async move {
        if let Some(gl_gpu) = detect_opengl_renderer() {
            return vec![gl_gpu];
        }
        if let Some(vk_gpu) = detect_vulkan_gpu() {
            return vec![vk_gpu];
        }
        Vec::new()
    }))
}

async fn get_storage_info() -> Vec<StorageInfo> {
    #[cfg(target_os = "windows")]
    {
        return crate::windows::storage::collect().await;
    }

    #[cfg(target_os = "macos")]
    {
        return crate::macos::storage::collect().await;
    }

    #[cfg(target_os = "linux")]
    {
        return crate::linux::storage::collect().await;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}
