use crate::config::UserConfig;
use crate::i18n::msg;
use crate::mixin::merge_and_restart;
use crate::utils::{failcat, okcat};
use anyhow::Result;
use std::fs;

/// Check if TUN mode is enabled
pub fn is_tun_enabled(config: &UserConfig) -> Result<bool> {
    let runtime = crate::config::RuntimeConfig::from_file(&config.config_runtime())?;
    Ok(runtime.get_bool("tun.enable").unwrap_or(false))
}

/// Enable TUN mode
pub fn tun_on(config: &UserConfig) -> Result<()> {
    if is_tun_enabled(config)? {
        okcat(&msg("tun_status_on"));
        return Ok(());
    }

    let mut mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
    mixin.set("tun.enable", serde_yaml::Value::Bool(true));
    mixin.to_file(&config.config_mixin())?;

    merge_and_restart(config)?;

    // Brief wait then check kernel log for TUN errors
    std::thread::sleep(std::time::Duration::from_millis(500));

    let log = fs::read_to_string(config.kernel_log()).unwrap_or_default();
    if log.contains("unsupported kernel version") || log.contains("Start TUN listening error") {
        // Rollback
        let mut mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
        mixin.set("tun.enable", serde_yaml::Value::Bool(false));
        mixin.to_file(&config.config_mixin())?;
        merge_and_restart(config).ok();

        anyhow::bail!("不支持的内核版本");
    }

    okcat(&msg("tun_enabled"));
    Ok(())
}

/// Disable TUN mode
pub fn tun_off(config: &UserConfig) -> Result<()> {
    if !is_tun_enabled(config)? {
        okcat(&msg("tun_status_off"));
        return Ok(());
    }

    let mut mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
    mixin.set("tun.enable", serde_yaml::Value::Bool(false));
    mixin.to_file(&config.config_mixin())?;

    merge_and_restart(config)?;
    okcat(&msg("tun_disabled"));
    Ok(())
}

/// Show TUN status
pub fn tun_status(config: &UserConfig) -> Result<()> {
    if is_tun_enabled(config)? {
        okcat(&msg("tun_status_on"));
    } else {
        failcat(&msg("tun_status_off"));
    }
    Ok(())
}
