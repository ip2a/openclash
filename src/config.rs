use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_CLASH_BASE_DIR: &str = "/opt/clash";

/// User-level configuration in ~/.openclash/config.toml
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserConfig {
    /// Language: zh or en
    #[serde(default = "default_lang")]
    pub language: String,

    /// Clash runtime base directory (default: ~/.openclash)
    #[serde(default = "default_clash_base_dir")]
    pub clash_base_dir: PathBuf,

    /// Kernel binary name (clash or mihomo)
    #[serde(default = "default_kernel_name")]
    pub kernel_name: String,

    /// Mixed proxy port
    #[serde(default = "default_mixed_port")]
    pub mixed_port: u16,

    /// External controller (Web UI) port
    #[serde(default = "default_ui_port")]
    pub ui_port: u16,

    /// External controller bind address
    #[serde(default = "default_ui_bind")]
    pub ui_bind: String,

    /// Secret for Web UI
    #[serde(default)]
    pub secret: String,

    /// Tun mode enabled
    #[serde(default)]
    pub tun_enable: bool,

    /// System proxy enabled
    #[serde(default = "default_true")]
    pub system_proxy_enable: bool,

    /// Subscription URL
    #[serde(default)]
    pub subscription_url: String,

    /// Auto update interval in hours
    #[serde(default = "default_auto_update_interval")]
    pub auto_update_interval: u64,

    /// Custom paths for binaries (optional)
    #[serde(default)]
    pub bin_paths: HashMap<String, PathBuf>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language: default_lang(),
            clash_base_dir: default_clash_base_dir(),
            kernel_name: default_kernel_name(),
            mixed_port: default_mixed_port(),
            ui_port: default_ui_port(),
            ui_bind: default_ui_bind(),
            secret: String::new(),
            tun_enable: false,
            system_proxy_enable: true,
            subscription_url: String::new(),
            auto_update_interval: default_auto_update_interval(),
            bin_paths: HashMap::new(),
        }
    }
}

fn default_lang() -> String {
    "zh".to_string()
}

fn default_clash_base_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".openclash")
}

fn default_kernel_name() -> String {
    "mihomo".to_string()
}
fn default_mixed_port() -> u16 {
    7890
}
fn default_ui_port() -> u16 {
    9090
}
fn default_ui_bind() -> String {
    "0.0.0.0".to_string()
}
fn default_auto_update_interval() -> u64 {
    48
}
fn default_true() -> bool {
    true
}

impl UserConfig {
    /// Load user config from ~/.openclash/config.toml
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
            let mut config: UserConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;
            if config.normalize_legacy_defaults() {
                config.save()?;
            }
            Ok(config)
        } else {
            let config = UserConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save user config to ~/.openclash/config.toml
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
        Ok(())
    }

    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let dir = crate::utils::openclash_dir()?;
        Ok(dir.join("config.toml"))
    }

    /// Get binary directory
    pub fn bin_dir(&self) -> PathBuf {
        self.clash_base_dir.join("bin")
    }

    /// Get kernel binary path
    pub fn kernel_path(&self) -> PathBuf {
        self.bin_dir().join(platform_binary_name(&self.kernel_name))
    }

    /// Get subconverter directory
    pub fn subconverter_dir(&self) -> PathBuf {
        self.bin_dir().join("subconverter")
    }

    /// Get subconverter binary path
    pub fn subconverter_path(&self) -> PathBuf {
        self.subconverter_dir().join(platform_binary_name("subconverter"))
    }

    /// Get raw config path
    pub fn config_raw(&self) -> PathBuf {
        self.clash_base_dir.join("config.yaml")
    }

    /// Get mixin config path
    pub fn config_mixin(&self) -> PathBuf {
        self.clash_base_dir.join("mixin.yaml")
    }

    /// Get runtime config path
    pub fn config_runtime(&self) -> PathBuf {
        self.clash_base_dir.join("runtime.yaml")
    }

    /// Get config backup path
    pub fn config_raw_bak(&self) -> PathBuf {
        self.clash_base_dir.join("config.yaml.bak")
    }

    /// Get subscription URL file
    pub fn config_url_file(&self) -> PathBuf {
        self.clash_base_dir.join("url")
    }

    /// Get update log path
    pub fn update_log(&self) -> PathBuf {
        self.clash_base_dir.join("openclash-update.log")
    }

    /// Get TUI log path
    pub fn tui_log(&self) -> PathBuf {
        self.clash_base_dir.join("openclash-tui.log")
    }

    /// Get PID file path
    pub fn pid_file(&self) -> PathBuf {
        self.clash_base_dir.join("run/mihomo.pid")
    }

    /// Get kernel log path
    pub fn kernel_log(&self) -> PathBuf {
        self.clash_base_dir.join("run/mihomo.log")
    }

    /// Get run directory
    pub fn run_dir(&self) -> PathBuf {
        self.clash_base_dir.join("run")
    }

    /// Get public (web UI) directory
    pub fn public_dir(&self) -> PathBuf {
        self.clash_base_dir.join("public")
    }

    fn normalize_legacy_defaults(&mut self) -> bool {
        if self.clash_base_dir == PathBuf::from(LEGACY_CLASH_BASE_DIR) {
            self.clash_base_dir = default_clash_base_dir();
            return true;
        }
        false
    }
}

fn platform_binary_name(name: &str) -> String {
    if cfg!(windows) && !name.to_ascii_lowercase().ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_opt_default_dir() {
        let mut config = UserConfig {
            clash_base_dir: PathBuf::from(LEGACY_CLASH_BASE_DIR),
            ..UserConfig::default()
        };

        assert!(config.normalize_legacy_defaults());
        assert_eq!(config.clash_base_dir, default_clash_base_dir());
    }

    #[test]
    fn keeps_custom_runtime_dir() {
        let custom_dir = PathBuf::from("/tmp/openclash-custom");
        let mut config = UserConfig {
            clash_base_dir: custom_dir.clone(),
            ..UserConfig::default()
        };

        assert!(!config.normalize_legacy_defaults());
        assert_eq!(config.clash_base_dir, custom_dir);
    }
}

/// Runtime YAML config manipulation (serde_yaml based, no yq dependency)
pub struct RuntimeConfig {
    pub data: serde_yaml::Value,
}

impl RuntimeConfig {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read runtime config: {}", path.display()))?;
        let data: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse runtime config: {}", path.display()))?;
        Ok(Self { data })
    }

    #[allow(dead_code)]
    pub fn from_str(content: &str) -> Result<Self> {
        let data: serde_yaml::Value =
            serde_yaml::from_str(content).context("Failed to parse runtime config")?;
        Ok(Self { data })
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content =
            serde_yaml::to_string(&self.data).context("Failed to serialize runtime config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write runtime config: {}", path.display()))?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&serde_yaml::Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &self.data;
        for k in keys {
            current = current.get(k)?;
        }
        Some(current)
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_u16(&self, key: &str) -> Option<u16> {
        self.get(key)
            .and_then(|v| v.as_u64())
            .and_then(|n| n.try_into().ok())
    }

    pub fn set(&mut self, key: &str, value: serde_yaml::Value) {
        let keys: Vec<&str> = key.split('.').collect();
        if keys.is_empty() {
            return;
        }

        let mut current = &mut self.data;
        for (i, k) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                if let serde_yaml::Value::Mapping(ref mut map) = current {
                    map.insert(serde_yaml::Value::String(k.to_string()), value);
                }
                return;
            }
            if let serde_yaml::Value::Mapping(ref mut map) = current {
                current = map
                    .entry(serde_yaml::Value::String(k.to_string()))
                    .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
            }
        }
    }
}

/// Merge multiple YAML configs: mixin + raw + mixin (later overrides earlier)
/// Sequences are deduplicated
pub fn merge_configs(configs: &[&str]) -> Result<RuntimeConfig> {
    let mut result = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());

    for content in configs {
        let doc: serde_yaml::Value =
            serde_yaml::from_str(content).context("Failed to parse config for merge")?;
        deep_merge(&mut result, &doc);
    }

    // Deduplicate sequences recursively
    dedup_sequences(&mut result);

    Ok(RuntimeConfig { data: result })
}

fn deep_merge(base: &mut serde_yaml::Value, overlay: &serde_yaml::Value) {
    match (base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, val) in overlay_map.iter() {
                if let Some(existing) = base_map.get_mut(key) {
                    deep_merge(existing, val);
                } else {
                    base_map.insert(key.clone(), val.clone());
                }
            }
        }
        (base_val, overlay_val) => {
            *base_val = overlay_val.clone();
        }
    }
}

fn dedup_sequences(value: &mut serde_yaml::Value) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (_, val) in map.iter_mut() {
                dedup_sequences(val);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            let mut seen = std::collections::HashSet::new();
            seq.retain(|item| {
                let key = serde_yaml::to_string(item).unwrap_or_default();
                let is_new = !seen.contains(&key);
                seen.insert(key);
                is_new
            });
        }
        _ => {}
    }
}

/// Validate config using kernel test mode
pub fn validate_config(kernel_path: &Path, config_path: &Path) -> Result<()> {
    let output = std::process::Command::new(kernel_path)
        .args([
            "-d",
            &config_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_string_lossy(),
            "-f",
            &config_path.to_string_lossy(),
            "-t",
        ])
        .output()
        .with_context(|| format!("Failed to run kernel validation: {}", kernel_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unsupport proxy type") {
            anyhow::bail!("配置包含不支持的代理类型，请使用 mihomo 内核");
        }
        anyhow::bail!("配置验证失败: {}", stderr);
    }

    Ok(())
}
