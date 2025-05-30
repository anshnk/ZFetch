use crate::config::Config;
use os_info;
use battery;
use tokio::process::Command;
use std::future::Future;
use std::pin::Pin;
use sysinfo::{System, Disk};

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
}

pub async fn get_system_info(config: &Config) -> SystemInfo {
    let os_task = tokio::task::spawn_blocking(|| os_info::get());
    let sys_task = tokio::task::spawn_blocking(|| {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        sys
    });
    let kernel_task = tokio::task::spawn_blocking(|| sysinfo::System::kernel_version().unwrap_or_default());
    let uptime_task = if config.show_uptime.unwrap_or(true) {
        Some(tokio::task::spawn_blocking(|| sysinfo::System::uptime()))
    } else {
        None
    };

    let gpu_task: Option<Pin<Box<dyn Future<Output = String> + Send>>> = if config.show_gpu.unwrap_or(true) {
        Some(
            if cfg!(target_os = "macos") {
                Box::pin(async {
                    if let Ok(output) = Command::new("ioreg")
                        .args(&["-r", "-c", "IOPCIDevice"])
                        .output()
                        .await
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Some(model) = stdout
                            .lines()
                            .find_map(|line| {
                                if line.contains("model") && (line.contains("Apple") || line.contains("display") || line.contains("GPU")) {
                                    let parts: Vec<&str> = line.split('=').collect();
                                    if parts.len() > 1 {
                                        Some(parts[1].trim().replace("\"", ""))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                        {
                            return model;
                        }
                    }
                    if let Ok(output) = Command::new("system_profiler")
                        .args(&["SPDisplaysDataType", "-json"])
                        .output()
                        .await
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                            if let Some(gpus) = json.get("SPDisplaysDataType").and_then(|v| v.as_array()) {
                                if let Some(gpu) = gpus.get(0) {
                                    let model = gpu.get("sppci_model").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                    let cores = gpu.get("spdisplays_gpu_core_count").and_then(|v| v.as_u64());
                                    let freq = gpu.get("spdisplays_gpu_core_clock").and_then(|v| v.as_str());
                                    let mut details = model.to_string();
                                    if let Some(cores) = cores {
                                        details.push_str(&format!(" ({} cores", cores));
                                        if let Some(freq) = freq {
                                            details.push_str(&format!(", {})", freq));
                                        } else {
                                            details.push(')');
                                        }
                                    }
                                    return details;
                                }
                            }
                        }
                        "Unknown".to_string()
                    } else {
                        "Unknown".to_string()
                    }
                })
            } else if cfg!(target_os = "windows") {
                Box::pin(async {
                    if let Some(gl_gpu) = detect_gpu_opengl() {
                        return gl_gpu;
                    }
                    if let Some(vk_gpu) = detect_gpu_vulkan() {
                        return vk_gpu;
                    }
                    if let Ok(output) = Command::new("wmic")
                        .args(&["path", "win32_VideoController", "get", "name"])
                        .output()
                        .await
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let gpus: Vec<_> = stdout
                            .lines()
                            .skip(1)
                            .map(str::trim)
                            .filter(|line| !line.is_empty())
                            .collect();
                        if !gpus.is_empty() {
                            return gpus.join(", ");
                        }
                    }
                    // Fallback to dxdiag aka worst fallback, didnt work when i tried lmao
                    if let Ok(_output) = Command::new("dxdiag")
                        .args(&["/t", "dxdiag.txt"])
                        .output()
                        .await
                    {
                        if std::path::Path::new("dxdiag.txt").exists() {
                            if let Ok(contents) = std::fs::read_to_string("dxdiag.txt") {
                                if let Some(line) = contents.lines().find(|l| l.contains("Card name:")) {
                                    return line.replace("Card name:", "").trim().to_string();
                                }
                            }
                            let _ = std::fs::remove_file("dxdiag.txt");
                        }
                    }
                    "Unknown".to_string()
                })
                // yo i havent tried linux yet, but just report to me
            } else if cfg!(target_os = "linux") {
                Box::pin(async {
                    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
                        let mut gpus = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with("card") && !name.contains("-") {
                                    let device_path = path.join("device");
                                    let vendor_path = device_path.join("vendor");
                                    let device_name_path = device_path.join("device");
                                    if let (Ok(vendor), Ok(device)) = (
                                        std::fs::read_to_string(&vendor_path),
                                        std::fs::read_to_string(&device_name_path),
                                    ) {
                                        gpus.push(format!("PCI {}:{}", vendor.trim(), device.trim()));
                                    }
                                }
                            }
                        }
                        if !gpus.is_empty() {
                            return gpus.join(", ");
                        }
                    }
                    if let Ok(output) = Command::new("lspci")
                        .output()
                        .await
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let gpus: Vec<_> = stdout
                            .lines()
                            .filter(|line| line.contains(" VGA ") || line.contains("3D controller"))
                            .map(|line| line.split(':').last().unwrap_or("").trim().to_string())
                            .collect();
                        if !gpus.is_empty() {
                            return gpus.join(", ");
                        }
                    }
                    if let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") {
                        let mut gpus = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let class_path = path.join("class");
                            if let Ok(class) = std::fs::read_to_string(&class_path) {
                                if class.trim().starts_with("0x03") {
                                    let vendor_path = path.join("vendor");
                                    let device_path = path.join("device");
                                    if let (Ok(vendor), Ok(device)) = (
                                        std::fs::read_to_string(&vendor_path),
                                        std::fs::read_to_string(&device_path),
                                    ) {
                                        gpus.push(format!("PCI {}:{}", vendor.trim(), device.trim()));
                                    }
                                }
                            }
                        }
                        if !gpus.is_empty() {
                            return gpus.join(", ");
                        }
                    }
                    if let Some(gl_gpu) = detect_gpu_opengl() {
                        return gl_gpu;
                    }
                    if let Some(vk_gpu) = detect_gpu_vulkan() {
                        return vk_gpu;
                    }
                    "Unknown".to_string()
                })
            } else {
                Box::pin(async {
                    tokio::task::spawn_blocking(|| {
                        if let Some(gl_gpu) = detect_gpu_opengl() {
                            return gl_gpu;
                        }
                        if let Some(vk_gpu) = detect_gpu_vulkan() {
                            return vk_gpu;
                        }
                        "Unknown".to_string()
                    })
                    .await
                    .unwrap_or_else(|_| "Unknown".to_string())
                })
            }
        )
    } else {
        None
    };

    // Wait for all the stuff to finish and deal with those Option<bool>s
    let (os, sys, kernel, uptime_secs, gpu) = match (uptime_task, gpu_task) {
        (Some(uptime_task), Some(gpu_task)) => {
            let (os, sys, kernel, uptime_secs, gpu) = tokio::join!(
                os_task,
                sys_task,
                kernel_task,
                uptime_task,
                gpu_task
            );
            (os, sys, kernel, Some(uptime_secs), Some(gpu))
        }
        (Some(uptime_task), None) => {
            let (os, sys, kernel, uptime_secs) = tokio::join!(
                os_task,
                sys_task,
                kernel_task,
                uptime_task
            );
            (os, sys, kernel, Some(uptime_secs), None)
        }
        (None, Some(gpu_task)) => {
            let (os, sys, kernel, gpu) = tokio::join!(
                os_task,
                sys_task,
                kernel_task,
                gpu_task
            );
            (os, sys, kernel, None, Some(gpu))
        }
        (None, None) => {
            let (os, sys, kernel) = tokio::join!(
                os_task,
                sys_task,
                kernel_task
            );
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
        raw_os_type.to_lowercase().replace(' ', "").chars().take(16).collect()
    };

    let cpu = if config.show_cpu.unwrap_or(true) {
        let cpu_brand = sys.cpus().get(0).map_or("Unknown".to_string(), |c| c.brand().to_string());
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
            Some(format_bytes(sys.total_memory() / 1024)), // Convert the memory from KiB to MiB
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
        gpu.map(|g| g)
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
    }
}

fn format_bytes(kb: u64) -> String {
    let gb = kb as f64 / 1024.0 / 1024.0;
    format!("{:.2} GB", gb)
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_gpu_opengl() -> Option<String> {
    use glutin::prelude::*;
    use glutin::config::ConfigTemplateBuilder;
    use glutin::context::{ContextApi, ContextAttributesBuilder};
    use glutin::display::GetGlDisplay;
    use glutin_winit::{DisplayBuilder, GlWindow};
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;
    use raw_window_handle::HasRawWindowHandle;
    use std::ffi::CString;

    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new();
    let template = ConfigTemplateBuilder::new();
    let display_builder = DisplayBuilder::new().with_window_builder(Some(wb));
    let (window, gl_config) = display_builder
        .build(&event_loop, template, |mut configs| configs.next().expect("No GL config found"))
        .ok()?;

    let raw_window_handle = window.as_ref().map(|w| w.raw_window_handle());
    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version::new(3, 3))));
    let not_current_gl_context = unsafe {
        gl_display.create_context(&gl_config, &context_attributes.build(raw_window_handle)).ok()?
    };
    let window = window?;
    let attrs = window.build_surface_attributes(<_>::default());
    let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs).ok()? };
    let _gl_context = not_current_gl_context.make_current(&gl_surface).ok()?;

    gl::load_with(|symbol| {
        let c_str = CString::new(symbol).unwrap();
        gl_display.get_proc_address(&c_str) as *const _
    });

    let renderer = unsafe {
        let ptr = gl::GetString(gl::RENDERER);
        if ptr.is_null() {
            return None;
        }
        std::ffi::CStr::from_ptr(ptr as *const i8)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    };
    renderer
}

fn detect_gpu_vulkan() -> Option<String> {
    use ash::vk;
    use ash::Entry;

    let entry = unsafe { Entry::load().ok()? };
    let app_name = std::ffi::CString::new("ZFetch").unwrap();
    let engine_name = std::ffi::CString::new("No Engine").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(app_name.as_c_str())
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(engine_name.as_c_str())
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::API_VERSION_1_0);
    let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&create_info, None).ok()? };
    let physical_devices = unsafe { instance.enumerate_physical_devices().ok()? };
    let props = unsafe { instance.get_physical_device_properties(physical_devices[0]) };
    let name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) };
    Some(name.to_string_lossy().to_string())
}

// Dummy implementations for local_ip and battery
// (these are just placeholders and don't do anything real)
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
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

        let percent = battery.state_of_charge().get::<battery::units::ratio::percent>();
        let _status = match battery.state() {
            battery::State::Charging => "Charging",
            battery::State::Full => "Full",
            battery::State::Discharging => "Discharging",
            battery::State::Empty => "Empty",
            battery::State::Unknown => "Unknown",
            _ => "Unknown",
        };

        let plugged = if matches!(battery.state(), battery::State::Charging | battery::State::Full) {
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
    let mut storage_info = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use tokio::process::Command;
        if let Ok(output) = Command::new("df")
            .arg("-k")
            .arg("/")
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let columns: Vec<&str> = line.split_whitespace().collect();
                if columns.len() >= 6 {
                    let total_kb: u64 = columns[1].parse().unwrap_or(0);
                    let avail_kb: u64 = columns[3].parse().unwrap_or(0);
                    let used_kb = total_kb.saturating_sub(avail_kb);
                    let percent = columns[4].trim_end_matches('%').parse().unwrap_or(0);

                    storage_info.push(StorageInfo {
                        name: "/".to_string(),
                        total: format_bytes(total_kb),
                        used: format_bytes(used_kb),
                        percent,
                        fs_type: "apfs".to_string(),
                        readonly: false,
                    });
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use tokio::process::Command;
        if let Ok(output) = Command::new("df")
            .arg("-k")
            .arg("/")
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let columns: Vec<&str> = line.split_whitespace().collect();
                if columns.len() >= 6 && columns[columns.len() - 1] == "/" {
                    let total_kb: u64 = columns[1].parse().unwrap_or(0);
                    let avail_kb: u64 = columns[3].parse().unwrap_or(0);
                    let used_kb = total_kb.saturating_sub(avail_kb);
                    let percent = columns[4].trim_end_matches('%').parse().unwrap_or(0);

                    storage_info.push(StorageInfo {
                        name: "/".to_string(),
                        total: format_bytes(total_kb),
                        used: format_bytes(used_kb),
                        percent,
                        fs_type: "apfs".to_string(),
                        readonly: false,
                    });
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use tokio::process::Command;
        if let Ok(output) = Command::new("wmic")
            .args(&["logicaldisk", "get", "name,size,freespace,filesystem,drivetype"])
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let columns: Vec<&str> = line.split_whitespace().collect();
                // Only show C:\ (im too lazy to figure out windows stuff)
                if columns.len() >= 4 && columns[0].to_ascii_uppercase().starts_with("C:") {
                    let total: u64 = columns[1].parse().unwrap_or(0);
                    let free: u64 = columns[2].parse().unwrap_or(0);
                    let used = total.saturating_sub(free);
                    let percent = if total > 0 {
                        ((used as f64 / total as f64) * 100.0).round() as u8
                    } else {
                        0
                    };
                    let fs_type = columns[3].to_string();

                    storage_info.push(StorageInfo {
                        name: "C:/".to_string(),
                        total: format_bytes(total / 1024),
                        used: format_bytes(used / 1024),
                        percent,
                        fs_type,
                        readonly: false,
                    });
                }
            }
        }
    }

    storage_info
}