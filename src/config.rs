use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub show_distro: Option<bool>,
    pub show_distro_id: Option<bool>,
    pub show_kernel: Option<bool>,
    pub show_cpu: Option<bool>,
    pub show_gpu: Option<bool>,
    pub show_memory: Option<bool>,
    pub show_swap: Option<bool>,
    pub show_local_ip: Option<bool>,
    pub show_battery: Option<bool>,
    pub show_storage: Option<bool>,
    pub show_uptime: Option<bool>,
    pub logo_color: Option<String>,
    pub color: Option<String>,
    pub show_user_host: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_distro: Some(true),
            show_distro_id: Some(true),
            show_kernel: Some(true),
            show_cpu: Some(true),
            show_gpu: Some(true),
            show_memory: Some(true),
            show_swap: Some(true),
            show_local_ip: Some(true),
            show_battery: Some(true),
            show_storage: Some(true),
            show_uptime: Some(true),
            logo_color: Some("#00FFFF #FF00FF #FFFF00 #FFFFFF".to_string()), 
            color: Some("#FFFFFF".to_string()),
            show_user_host: Some(true),
        }
    }
}

impl Config {
    #[allow(dead_code)]
    pub fn from_file(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn from_exe_dir() -> Option<Self> {
        use std::env;
        use std::path::PathBuf;
        let exe_path = env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;
        let config_path: PathBuf = exe_dir.join("config.json");
        let content = std::fs::read_to_string(config_path).ok()?;
        serde_json::from_str(&content).ok()
    }
}