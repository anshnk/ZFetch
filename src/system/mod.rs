use crate::common::storage::format_bytes;
use crate::config::Config;
use std::future::Future;
use std::net::UdpSocket;
use std::pin::Pin;

pub struct StorageInfo {
    pub name: String,
    pub total: String,
    pub used: String,
    pub percent: u8,
    pub fs_type: String,
    pub readonly: bool,
}

pub struct SystemInfo {
    pub distro: String,
    pub distro_id: String,
    pub kernel: String,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
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

pub async fn get_system_info(config: &Config) -> SystemInfo {
    let os_task = tokio::task::spawn_blocking(|| os_info::get());
    let sys_task = tokio::task::spawn_blocking(|| {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu();
        sys.refresh_memory();
        sys
    });

    let kernel_task =
        tokio::task::spawn_blocking(|| sysinfo::System::kernel_version().unwrap_or_default());
    let uptime_task = if config.show_uptime.unwrap_or(true) {
        Some(tokio::task::spawn_blocking(|| sysinfo::System::uptime()))
    } else {
        None
    };

    let gpu_task: Option<Pin<Box<dyn Future<Output = String> + Send>>> =
        if config.show_gpu.unwrap_or(true) {
            build_platform_gpu_task()
        } else {
            None
        };

    let (os, sys, kernel, uptime_secs, gpu) = match (uptime_task, gpu_task) {
        (Some(uptime_task), Some(gpu_task)) => {
            let (os, sys, kernel, uptime_secs, gpu) =
                tokio::join!(os_task, sys_task, kernel_task, uptime_task, gpu_task);
            (os, sys, kernel, Some(uptime_secs), Some(gpu))
        }
        (Some(uptime_task), None) => {
            let (os, sys, kernel, uptime_secs) =
                tokio::join!(os_task, sys_task, kernel_task, uptime_task);
            (os, sys, kernel, Some(uptime_secs), None)
        }
        (None, Some(gpu_task)) => {
            let (os, sys, kernel, gpu) = tokio::join!(os_task, sys_task, kernel_task, gpu_task);
            (os, sys, kernel, None, Some(gpu))
        }
        (None, None) => {
            let (os, sys, kernel) = tokio::join!(os_task, sys_task, kernel_task);
            (os, sys, kernel, None, None)
        }
    };

    let os = os.unwrap();
    let sys = sys.unwrap();
    let kernel = kernel.unwrap();

    let raw_os_type = os.os_type().to_string();
    let version = os.version().to_string();
    let distro = match os.os_type() {
        os_info::Type::Macos => format!("Mac OS ({})", os.version()),
        os_info::Type::Windows => format!("Windows ({})", os.version()),
        _ => format!("{} ({})", raw_os_type, os.version()),
    };

    let distro_id = if raw_os_type.to_lowercase().contains("windows") {
        if version.starts_with("10.0.22") || version.contains("Windows 11") {
            "windows_11".to_string()
        } else {
            "windows".to_string()
        }
    } else if raw_os_type.to_lowercase().contains("macos") {
        "macos".to_string()
    } else {
        raw_os_type
            .to_lowercase()
            .replace(' ', "")
            .chars()
            .take(16)
            .collect()
    };

    let cpu = if config.show_cpu.unwrap_or(true) {
        let cpu_brand = sys
            .cpus()
            .get(0)
            .map_or("Unknown".to_string(), |c| c.brand().to_string());
        let cpu_cores = sys.physical_core_count().unwrap_or(sys.cpus().len());
        let cpu_freq = sys.cpus().get(0).map_or(0, |c| c.frequency());
        Some(format!(
            "{} ({} cores) ({:.2} GHz)",
            cpu_brand,
            cpu_cores,
            cpu_freq as f64 / 1000.0
        ))
    } else {
        None
    };

    let (total_memory, used_memory, total_swap, used_swap) = if config.show_memory.unwrap_or(true) {
        (
            Some(format_bytes(sys.total_memory() / 1024)),
            Some(format_bytes(sys.used_memory() / 1024)),
            Some(format_bytes(sys.total_swap() / 1024)),
            Some(format_bytes(sys.used_swap() / 1024)),
        )
    } else {
        (None, None, None, None)
    };

    let uptime = if config.show_uptime.unwrap_or(true) {
        if let Some(uptime_secs) = uptime_secs {
            let uptime_secs = uptime_secs.unwrap();
            let days = uptime_secs / 86400;
            let hours = (uptime_secs % 86400) / 3600;
            let minutes = (uptime_secs % 3600) / 60;
            Some(format!("{}d {}h {}m", days, hours, minutes))
        } else {
            None
        }
    } else {
        None
    };

    let gpu = if config.show_gpu.unwrap_or(true) {
        gpu
    } else {
        None
    };

    let local_ip = if config.show_local_ip.unwrap_or(true) {
        get_local_ip()
    } else {
        None
    };

    let battery = if config.show_battery.unwrap_or(true) {
        get_battery_status()
    } else {
        None
    };

    let storage = if config.show_storage.unwrap_or(true) {
        get_storage_info().await
    } else {
        Vec::new()
    };

    let username = whoami::username();
    let hostname = whoami::fallible::hostname().ok();

    SystemInfo {
        distro,
        distro_id,
        kernel,
        cpu,
        gpu,
        total_memory,
        used_memory,
        total_swap,
        used_swap,
        uptime,
        local_ip,
        battery,
        storage,
        username: Some(username),
        hostname,
    }
}

#[cfg(target_os = "macos")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    crate::macos::gpu::build_gpu_task()
}

#[cfg(target_os = "windows")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    crate::windows::gpu::build_gpu_task()
}

#[cfg(target_os = "linux")]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    crate::linux::gpu::build_gpu_task()
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn build_platform_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    use crate::common::gpu::{detect_opengl_renderer, detect_vulkan_gpu};

    Some(Box::pin(async move {
        if let Some(gl_gpu) = detect_opengl_renderer() {
            return gl_gpu;
        }
        if let Some(vk_gpu) = detect_vulkan_gpu() {
            return vk_gpu;
        }
        "Unknown".to_string()
    }))
}

fn get_local_ip() -> Option<String> {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|sock| {
            sock.connect("8.8.8.8:80")?;
            sock.local_addr()
        })
        .ok()
        .map(|addr| addr.ip().to_string())
}

fn get_battery_status() -> Option<String> {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        use battery::Manager;

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
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
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
