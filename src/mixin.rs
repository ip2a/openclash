use crate::config::{merge_configs, validate_config, RuntimeConfig, UserConfig};
use crate::i18n::msg;
use crate::service::restart;
use crate::utils::{okcat, read_file_or_empty};
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct UiInfo {
    pub port: u16,
    pub public_address: String,
    pub local_address: String,
    pub common_address: String,
}

#[derive(Debug, Clone)]
pub struct LanSettings {
    pub allow_lan: bool,
    pub mixed_port: u16,
}

/// Merge mixin + raw + mixin and restart service
pub fn merge_and_restart(config: &UserConfig) -> Result<()> {
    let mixin_path = config.config_mixin();
    let raw_path = config.config_raw();
    let runtime_path = config.config_runtime();

    let mixin_content = read_file_or_empty(&mixin_path);
    let raw_content = read_file_or_empty(&raw_path);

    // Merge: mixin + raw + mixin (last mixin overrides)
    let merged = merge_configs(&[&mixin_content, &raw_content, &mixin_content])
        .context("Failed to merge configs")?;

    // Validate before writing
    let temp_runtime = config.clash_base_dir.join("runtime.yaml.tmp");
    merged.to_file(&temp_runtime)?;

    if let Err(e) = validate_config(&config.kernel_path(), &temp_runtime) {
        std::fs::remove_file(&temp_runtime).ok();
        anyhow::bail!("验证失败：请检查 Mixin 配置\n{}", e);
    }

    // Backup current runtime and write new
    if runtime_path.exists() {
        let backup = config.clash_base_dir.join("runtime.yaml.backup");
        std::fs::copy(&runtime_path, &backup).ok();
    }

    std::fs::rename(&temp_runtime, &runtime_path)
        .with_context(|| format!("Failed to write runtime config: {}", runtime_path.display()))?;

    restart(config)?;
    Ok(())
}

/// Edit mixin config with default editor
pub fn edit_mixin(config: &UserConfig) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let status = Command::new(&editor)
        .arg(config.config_mixin())
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if status.success() {
        merge_and_restart(config)?;
        okcat(&msg("config_updated"));
    }

    Ok(())
}

/// View mixin config
pub fn view_mixin(config: &UserConfig) -> Result<()> {
    let path = config.config_mixin();
    let content = read_file_or_empty(&path);
    println!("{}", content);
    Ok(())
}

/// View runtime config
pub fn view_runtime(config: &UserConfig) -> Result<()> {
    let path = config.config_runtime();
    let content = read_file_or_empty(&path);
    println!("{}", content);
    Ok(())
}

/// Set or view secret
pub fn set_secret(config: &mut UserConfig, secret: Option<String>) -> Result<()> {
    match secret {
        Some(s) => {
            let mut mixin = RuntimeConfig::from_file(&config.config_mixin())?;
            mixin.set("secret", serde_yaml::Value::String(s));
            mixin.to_file(&config.config_mixin())?;
            merge_and_restart(config)?;
            okcat(&msg("secret_updated"));
        }
        None => {
            let runtime = RuntimeConfig::from_file(&config.config_runtime())?;
            let current = runtime.get_string("secret").unwrap_or_default();
            okcat(&format!("{}{}", msg("current_secret"), current));
        }
    }
    Ok(())
}

pub fn lan_settings(config: &UserConfig) -> Result<LanSettings> {
    let runtime = RuntimeConfig::from_file(&config.config_runtime())?;
    Ok(LanSettings {
        allow_lan: runtime.get_bool("allow-lan").unwrap_or(false),
        mixed_port: runtime.get_u16("mixed-port").unwrap_or(config.mixed_port),
    })
}

pub fn set_lan_settings(config: &UserConfig, settings: LanSettings) -> Result<()> {
    let mut mixin = RuntimeConfig::from_file(&config.config_mixin())?;
    mixin.set("allow-lan", serde_yaml::Value::Bool(settings.allow_lan));
    mixin.set(
        "mixed-port",
        serde_yaml::Value::Number(settings.mixed_port.into()),
    );
    mixin.to_file(&config.config_mixin())?;
    merge_and_restart(config)?;
    Ok(())
}

pub fn ui_info(config: &UserConfig) -> Result<UiInfo> {
    let port = crate::service::get_ui_port(config)?;
    let public_ip = crate::utils::get_public_ip().unwrap_or_else(|_| "公网".to_string());
    let local_ip = crate::utils::get_local_ip().unwrap_or_else(|_| "127.0.0.1".to_string());

    Ok(UiInfo {
        port,
        public_address: format!("http://{}:{}/ui", public_ip, port),
        local_address: format!("http://{}:{}/ui", local_ip, port),
        common_address: "http://board.zash.run.place".to_string(),
    })
}
