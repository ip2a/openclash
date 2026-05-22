use anyhow::Result;
use std::fs;

use crate::{config::UserConfig, mixin, proxy, service, tun};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub ready: bool,
    pub running: bool,
    pub pid: Option<String>,
    pub kernel_name: String,
    pub base_dir: String,
    pub mixed_port: Option<u16>,
    pub ui_port: Option<u16>,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub subscription_url: String,
    pub secret_preview: String,
    pub local_ui: Option<String>,
    pub public_ui: Option<String>,
    pub common_ui: String,
    pub config_raw_exists: bool,
    pub config_mixin_exists: bool,
    pub config_runtime_exists: bool,
    pub logs: Vec<String>,
}

impl Snapshot {
    pub fn load(config: &UserConfig) -> Result<Self> {
        let ready = config.kernel_path().exists()
            && config.config_mixin().exists()
            && config.config_runtime().exists();
        let running = service::is_running(config).unwrap_or(false);
        let pid = fs::read_to_string(config.pid_file())
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let mixed_port = if config.config_runtime().exists() {
            service::get_proxy_port(config).ok()
        } else {
            None
        };
        let ui_info = if config.config_runtime().exists() {
            mixin::ui_info(config).ok()
        } else {
            None
        };
        let system_proxy_enabled = if config.config_mixin().exists() {
            proxy::is_system_proxy_enabled(config).unwrap_or(false)
        } else {
            false
        };
        let tun_enabled = if config.config_runtime().exists() {
            tun::is_tun_enabled(config).unwrap_or(false)
        } else {
            false
        };
        let secret_preview = if config.secret.is_empty() {
            "not set".to_string()
        } else {
            mask_value(&config.secret)
        };

        let subscription_url = if !config.subscription_url.is_empty() {
            config.subscription_url.clone()
        } else {
            fs::read_to_string(config.config_url_file())
                .unwrap_or_default()
                .trim()
                .to_string()
        };

        Ok(Self {
            ready,
            running,
            pid,
            kernel_name: config.kernel_name.clone(),
            base_dir: config.clash_base_dir.display().to_string(),
            mixed_port,
            ui_port: ui_info.as_ref().map(|info| info.port),
            system_proxy_enabled,
            tun_enabled,
            subscription_url,
            secret_preview,
            local_ui: ui_info.as_ref().map(|info| info.local_address.clone()),
            public_ui: ui_info.as_ref().map(|info| info.public_address.clone()),
            common_ui: ui_info
                .map(|info| info.common_address)
                .unwrap_or_else(|| "http://board.zash.run.place".to_string()),
            config_raw_exists: config.config_raw().exists(),
            config_mixin_exists: config.config_mixin().exists(),
            config_runtime_exists: config.config_runtime().exists(),
            logs: service::read_recent_logs(config, 12),
        })
    }
}

fn mask_value(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        "*".repeat(chars.len())
    } else {
        format!(
            "{}{}",
            chars[..2].iter().collect::<String>(),
            "*".repeat(chars.len().saturating_sub(2))
        )
    }
}
