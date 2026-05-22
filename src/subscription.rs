use crate::config::{validate_config, UserConfig};
use crate::i18n::msg;
use crate::mixin::merge_and_restart;
use crate::utils::{failcat, failcat_with_emoji, okcat, okcat_with_emoji};
use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Update subscription configuration
pub fn update_sync(config: &mut UserConfig, url: Option<String>) -> Result<()> {
    let url = url.or_else(|| {
        if config.subscription_url.is_empty() {
            let url_file = config.config_url_file();
            if url_file.exists() {
                fs::read_to_string(&url_file)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        } else {
            Some(config.subscription_url.clone())
        }
    });

    let url = match url {
        Some(u) if u.starts_with("http") => u,
        _ => {
            failcat(&format!(
                "没有提供有效的订阅链接，使用 {} 进行更新...",
                config.config_raw().display()
            ));
            update_with_file(config)?;
            return Ok(());
        }
    };

    okcat_with_emoji("👌", &msg("update_downloading"));

    // Backup current config
    let raw_path = config.config_raw();
    let bak_path = config.config_raw_bak();
    if raw_path.exists() {
        fs::copy(&raw_path, &bak_path)
            .with_context(|| format!("Failed to backup config: {}", raw_path.display()))?;
    }

    // Download new config
    if let Err(e) = download_raw_config(&raw_path, &url) {
        rollback(config, &format!("下载失败: {}", e))?;
    }

    // Check if downloaded content is base64-encoded URI list
    let raw_content = fs::read_to_string(&raw_path).unwrap_or_default();
    let is_base64_sub = is_base64_encoded(&raw_content);

    if is_base64_sub {
        okcat_with_emoji("🔍", "检测到 base64 编码订阅，直接转换...");
        if let Err(conv_err) = download_convert_config(config, &raw_path, &url) {
            rollback(
                config,
                &format!(
                    "转换失败: {}，日志: {}",
                    conv_err,
                    config.subconverter_dir().join("latest.log").display()
                ),
            )?;
        }
    } else {
        // Validate config
        okcat_with_emoji("🔍", "正在验证配置文件...");
        if let Err(_e) = validate_config(&config.kernel_path(), &raw_path) {
            failcat_with_emoji("⚠️", &format!("配置验证失败，尝试转换..."));
            if let Err(conv_err) = download_convert_config(config, &raw_path, &url) {
                rollback(
                    config,
                    &format!(
                        "转换失败: {}，日志: {}",
                        conv_err,
                        config.subconverter_dir().join("latest.log").display()
                    ),
                )?;
            }
        }
    }

    // Merge and restart
    merge_and_restart(config)?;

    // Save URL
    fs::write(config.config_url_file(), &url)?;
    config.subscription_url = url.clone();
    config.save()?;

    okcat_with_emoji("🍃", &msg("update_success"));

    // Log success
    let log_msg = format!(
        "[{}] 订阅更新成功：{}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        url
    );
    let log_path = config.update_log();
    fs::write(&log_path, format!("{}\n", log_msg)).ok();
    okcat_with_emoji("✅", &log_msg);

    Ok(())
}

/// Update with local file (no URL)
fn update_with_file(config: &mut UserConfig) -> Result<()> {
    merge_and_restart(config)?;
    Ok(())
}

/// Setup auto update via crontab
pub fn setup_auto_update(config: &UserConfig) -> Result<()> {
    let user =
        std::env::var("SUDO_USER").unwrap_or_else(|_| std::env::var("USER").unwrap_or_default());

    // Determine crontab path based on OS
    let os_info = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let crontab_path =
        if os_info.to_lowercase().contains("rhel") || os_info.to_lowercase().contains("centos") {
            format!("/var/spool/cron/{}", user)
        } else {
            format!("/var/spool/cron/crontabs/{}", user)
        };

    let cron_entry = format!(
        "0 0 */{} * * bash -i -c 'openclash update sync'",
        config.auto_update_interval
    );

    let content = fs::read_to_string(&crontab_path).unwrap_or_default();
    if !content.contains("openclash update") {
        let mut new_content = content;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&cron_entry);
        new_content.push('\n');

        fs::write(&crontab_path, new_content)
            .with_context(|| format!("Failed to write crontab: {}", crontab_path))?;
    }

    okcat(&msg("auto_update_set"));
    Ok(())
}

/// View update log
pub fn view_log(config: &UserConfig) -> Result<()> {
    let log_path = config.update_log();
    if log_path.exists() {
        let content = fs::read_to_string(&log_path)?;
        println!("{}", content);
    } else {
        failcat("暂无更新日志");
    }
    Ok(())
}

/// Download raw config from URL
pub(crate) fn download_raw_config(dest: &Path, url: &str) -> Result<()> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--insecure",
            "--connect-timeout",
            "4",
            "--retry",
            "1",
            "--user-agent",
            "clash-verge/v2.0.4",
            "--output",
            &dest.to_string_lossy(),
            url,
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => {
            // Fallback to wget
            let output = Command::new("wget")
                .args([
                    "--no-verbose",
                    "--no-check-certificate",
                    "--timeout=3",
                    "--tries=1",
                    "--user-agent=clash-verge/v2.0.4",
                    &format!("--output-document={}", dest.display()),
                    url,
                ])
                .output()?;

            if output.status.success() {
                Ok(())
            } else {
                anyhow::bail!("下载失败: curl 和 wget 均不可用或请求失败");
            }
        }
    }
}

/// Download config using subconverter
pub(crate) fn download_convert_config(config: &UserConfig, dest: &Path, url: &str) -> Result<()> {
    let subconverter_port = start_subconverter(config)?;

    let convert_url = format!(
        "http://127.0.0.1:{}/sub?target=clash&url={}",
        subconverter_port,
        url.replace("&", "%26")
            .replace("=", "%3D")
            .replace("?", "%3F")
    );

    let result = download_raw_config(dest, &convert_url);
    stop_subconverter(config);

    result
}

/// Start subconverter service
pub(crate) fn start_subconverter(config: &UserConfig) -> Result<u16> {
    let mut port: u16 = 25500;

    // Check if default port is in use
    if crate::utils::is_already_in_use(port, "subconverter")? {
        port = crate::utils::get_random_port()?;
        failcat_with_emoji("⚠️", &format!("转换端口 25500 被占用，已更换为 {}", port));

        // Update subconverter config port
        let pref_path = config.subconverter_dir().join("pref.yml");
        let example_path = config.subconverter_dir().join("pref.example.yml");
        if !pref_path.exists() && example_path.exists() {
            fs::copy(&example_path, &pref_path)?;
        }

        if pref_path.exists() {
            let content = fs::read_to_string(&pref_path)?;
            let updated = content.replace("port: 25500", &format!("port: {}", port));
            fs::write(&pref_path, updated)?;
        }
    }

    let log_path = config.subconverter_dir().join("latest.log");
    let _ = Command::new("nohup")
        .arg(config.subconverter_path())
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&log_path)?)
        .spawn()?;

    // Wait for subconverter to start
    let start = std::time::Instant::now();
    while !crate::utils::is_bind(port)? {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if start.elapsed().as_secs() > 5 {
            stop_subconverter(config);
            anyhow::bail!("订阅转换服务启动失败，查看日志: {}", log_path.display());
        }
    }

    Ok(port)
}

/// Stop subconverter service
pub(crate) fn stop_subconverter(config: &UserConfig) {
    let _ = Command::new("pkill")
        .args(["-9", "-f", &config.subconverter_path().to_string_lossy()])
        .output();
}

/// Rollback to backup config
fn rollback(config: &UserConfig, reason: &str) -> Result<()> {
    failcat_with_emoji("🍂", reason);

    let bak_path = config.config_raw_bak();
    let raw_path = config.config_raw();

    if bak_path.exists() {
        fs::copy(&bak_path, &raw_path)?;
    }

    let log_msg = format!(
        "[{}] 订阅更新失败：{}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        reason
    );
    fs::write(config.update_log(), format!("{}\n", log_msg)).ok();

    anyhow::bail!("{}", reason)
}

/// Detect if subscription content is base64-encoded URI list
pub(crate) fn is_base64_encoded(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.len() < 20 {
        return false;
    }

    // Base64 charset
    let base64_chars: std::collections::HashSet<char> =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="
            .chars()
            .collect();

    let total = trimmed.chars().count();
    let valid = trimmed.chars().filter(|c| base64_chars.contains(c)).count();

    // Most chars are base64-valid and no YAML markers
    valid > total * 95 / 100 && !trimmed.contains(':')
}
