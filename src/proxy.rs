use crate::config::UserConfig;
use crate::i18n::msg;
use crate::service::is_running;
use crate::utils::{failcat, okcat};
use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct ProxyEndpoints {
    pub http: String,
    pub socks5: String,
    pub no_proxy: String,
}

/// Set system proxy environment variables in current shell
pub fn set_system_proxy(config: &UserConfig) -> Result<()> {
    if !is_running(config)? {
        anyhow::bail!("代理程序未运行，请执行 openclash on 开启代理环境");
    }

    let endpoints = proxy_endpoints(config)?;

    env::set_var("http_proxy", &endpoints.http);
    env::set_var("https_proxy", &endpoints.http);
    env::set_var("HTTP_PROXY", &endpoints.http);
    env::set_var("HTTPS_PROXY", &endpoints.http);
    env::set_var("all_proxy", &endpoints.socks5);
    env::set_var("ALL_PROXY", &endpoints.socks5);
    env::set_var("no_proxy", &endpoints.no_proxy);
    env::set_var("NO_PROXY", &endpoints.no_proxy);

    // Update mixin config to persist system-proxy setting (no restart needed)
    let mut mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
    mixin.set("system-proxy.enable", serde_yaml::Value::Bool(true));
    mixin.to_file(&config.config_mixin())?;

    okcat(&msg("proxy_enabled"));
    Ok(())
}

/// Unset system proxy environment variables
pub fn unset_system_proxy(config: &UserConfig) -> Result<()> {
    env::remove_var("http_proxy");
    env::remove_var("https_proxy");
    env::remove_var("HTTP_PROXY");
    env::remove_var("HTTPS_PROXY");
    env::remove_var("all_proxy");
    env::remove_var("ALL_PROXY");
    env::remove_var("no_proxy");
    env::remove_var("NO_PROXY");

    // Update mixin config (no restart needed)
    let mut mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
    mixin.set("system-proxy.enable", serde_yaml::Value::Bool(false));
    mixin.to_file(&config.config_mixin())?;

    okcat(&msg("proxy_disabled"));
    Ok(())
}

/// Show current proxy status
pub fn show_status(config: &UserConfig) -> Result<()> {
    let enabled = is_system_proxy_enabled(config)?;

    if enabled {
        let endpoints = proxy_endpoints(config)?;
        okcat(&format!(
            "系统代理：开启\nhttp_proxy： {}\nsocks_proxy：{}",
            endpoints.http, endpoints.socks5
        ));
    } else {
        failcat("系统代理：关闭");
    }

    Ok(())
}

pub fn is_system_proxy_enabled(config: &UserConfig) -> Result<bool> {
    let mixin = crate::config::RuntimeConfig::from_file(&config.config_mixin())?;
    Ok(mixin.get_bool("system-proxy.enable").unwrap_or(false))
}

pub fn proxy_endpoints(config: &UserConfig) -> Result<ProxyEndpoints> {
    let port = crate::service::get_proxy_port(config)?;

    let auth = get_auth(config).unwrap_or_default();
    let auth_prefix = if auth.is_empty() {
        String::new()
    } else {
        format!("{}@", auth)
    };

    Ok(ProxyEndpoints {
        http: format!("http://{}{}:{}", auth_prefix, "127.0.0.1", port),
        socks5: format!("socks5h://{}{}:{}", auth_prefix, "127.0.0.1", port),
        no_proxy: "localhost,127.0.0.1,::1".to_string(),
    })
}

fn get_auth(config: &UserConfig) -> Result<String> {
    let runtime = crate::config::RuntimeConfig::from_file(&config.config_runtime())?;
    if let Some(auth_list) = runtime.get("authentication") {
        if let Some(arr) = auth_list.as_sequence() {
            if let Some(first) = arr.first() {
                if let Some(s) = first.as_str() {
                    return Ok(s.to_string());
                }
            }
        }
    }
    Ok(String::new())
}
