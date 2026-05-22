use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;

static MSG_ZH: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("proxy_on", "😼 已开启代理环境");
    m.insert("proxy_off", "😼 已关闭代理环境");
    m.insert("proxy_enabled", "😼 系统代理：开启");
    m.insert("proxy_disabled", "😼 系统代理：关闭");
    m.insert("tun_enabled", "😼 Tun 模式已开启");
    m.insert("tun_disabled", "😼 Tun 模式已关闭");
    m.insert("tun_status_on", "😼 Tun 状态：开启");
    m.insert("tun_status_off", "😾 Tun 状态：关闭");
    m.insert("secret_updated", "😼 密钥更新成功，已重启生效");
    m.insert("current_secret", "😼 当前密钥：");
    m.insert("update_success", "🍃 订阅更新成功");
    m.insert("update_downloading", "👌 正在下载：原配置已备份...");
    m.insert("update_validating", "🍃 下载成功：内核验证配置...");
    m.insert("auto_update_set", "😼 已设置定时更新订阅");
    m.insert("web_console", "😼 Web 控制台");
    m.insert("lang_switched", "语言已切换为中文");
    m.insert("current_lang", "当前语言：中文 (zh)");
    m.insert("lang_usage", "用法: openclash lang [zh|en]");
    m.insert("config_updated", "配置更新成功，已重启生效");
    m.insert(
        "service_not_ready",
        "❌ 内核未就绪，请重新运行当前命令以自动准备资源",
    );
    m.insert("enjoy", "🎉 enjoy 🎉");
    m
});

static MSG_EN: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("proxy_on", "😼 Proxy environment enabled");
    m.insert("proxy_off", "😼 Proxy environment disabled");
    m.insert("proxy_enabled", "😼 System proxy: enabled");
    m.insert("proxy_disabled", "😼 System proxy: disabled");
    m.insert("tun_enabled", "😼 Tun mode enabled");
    m.insert("tun_disabled", "😼 Tun mode disabled");
    m.insert("tun_status_on", "😼 Tun status: enabled");
    m.insert("tun_status_off", "😾 Tun status: disabled");
    m.insert(
        "secret_updated",
        "😼 Secret updated successfully, restarted",
    );
    m.insert("current_secret", "😼 Current secret: ");
    m.insert("update_success", "🍃 Subscription updated successfully");
    m.insert(
        "update_downloading",
        "👌 Downloading: Original config backed up...",
    );
    m.insert(
        "update_validating",
        "🍃 Download successful: Kernel validating config...",
    );
    m.insert("auto_update_set", "😼 Scheduled subscription update set");
    m.insert("web_console", "😼 Web Console");
    m.insert("lang_switched", "Language switched to English");
    m.insert("current_lang", "Current language: English (en)");
    m.insert("lang_usage", "Usage: openclash lang [zh|en]");
    m.insert(
        "config_updated",
        "Configuration updated successfully, restarted",
    );
    m.insert(
        "service_not_ready",
        "❌ Kernel not ready, rerun the command to prepare embedded resources",
    );
    m.insert("enjoy", "🎉 enjoy 🎉");
    m
});

/// Get current language setting
pub fn get_current_lang() -> String {
    let lang_file = crate::utils::openclash_dir()
        .map(|d| d.join("lang.conf"))
        .unwrap_or_default();

    if lang_file.exists() {
        let content = fs::read_to_string(&lang_file).unwrap_or_default();
        for line in content.lines() {
            if line.starts_with("LANG=") {
                return line.trim_start_matches("LANG=").trim().to_string();
            }
        }
    }

    env_locale()
}

/// Set language
pub fn set_language(lang: &str) -> anyhow::Result<()> {
    let dir = crate::utils::openclash_dir()?;
    let lang_file = dir.join("lang.conf");

    if lang == "zh" || lang == "en" {
        fs::write(&lang_file, format!("LANG={}\n", lang))?;
        Ok(())
    } else {
        anyhow::bail!("Unsupported language: {}", lang)
    }
}

/// Translate a message key
pub fn msg(key: &str) -> String {
    let lang = get_current_lang();
    if lang == "en" {
        MSG_EN.get(key).unwrap_or(&key).to_string()
    } else {
        MSG_ZH.get(key).unwrap_or(&key).to_string()
    }
}

/// Get locale from environment
fn env_locale() -> String {
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_default()
        .to_lowercase();

    if locale.starts_with("en") {
        "en".to_string()
    } else {
        "zh".to_string()
    }
}
