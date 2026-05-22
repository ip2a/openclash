use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::config::{RuntimeConfig, UserConfig};

#[derive(Debug, Clone)]
pub struct ProxyGroup {
    pub name: String,
    pub now: String,
    pub nodes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProxiesResponse {
    proxies: HashMap<String, ProxyInfo>,
}

#[derive(Debug, Deserialize)]
struct ProxyInfo {
    #[serde(default)]
    all: Vec<String>,
    #[serde(default)]
    now: String,
}

pub fn proxy_groups(config: &UserConfig) -> Result<Vec<ProxyGroup>> {
    let client = controller_client()?;
    let url = format!("{}/proxies", controller_base_url(config)?);
    let mut request = client.get(url);
    if let Some(secret) = controller_secret(config)? {
        request = request.header(AUTHORIZATION, format!("Bearer {}", secret));
    }

    let body = request.send()?.error_for_status()?.text()?;
    let response: ProxiesResponse =
        serde_json::from_str(&body).context("Failed to parse Mihomo proxies response")?;
    let mut groups: Vec<ProxyGroup> = response
        .proxies
        .into_iter()
        .filter_map(|(name, proxy)| {
            if proxy.all.is_empty() {
                return None;
            }
            Some(ProxyGroup {
                name,
                now: proxy.now,
                nodes: proxy.all,
            })
        })
        .collect();

    groups.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(groups)
}

pub fn switch_proxy(config: &UserConfig, group: &str, node: &str) -> Result<()> {
    let client = controller_client()?;
    let url = format!(
        "{}/proxies/{}",
        controller_base_url(config)?,
        encode_path_segment(group)
    );
    let body = serde_json::json!({ "name": node }).to_string();
    let mut request = client.put(url).header(CONTENT_TYPE, "application/json").body(body);
    if let Some(secret) = controller_secret(config)? {
        request = request.header(AUTHORIZATION, format!("Bearer {}", secret));
    }

    request
        .send()
        .with_context(|| format!("Failed to switch proxy group: {}", group))?
        .error_for_status()?;
    Ok(())
}

fn controller_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("Failed to build Mihomo controller client")
}

fn controller_base_url(config: &UserConfig) -> Result<String> {
    let runtime = RuntimeConfig::from_file(&config.config_runtime())?;
    let controller = runtime
        .get_string("external-controller")
        .unwrap_or_else(|| format!("{}:{}", config.ui_bind, config.ui_port));
    let controller = controller
        .strip_prefix("http://")
        .or_else(|| controller.strip_prefix("https://"))
        .unwrap_or(&controller);
    let host_port = if controller.starts_with("0.0.0.0:") {
        controller.replacen("0.0.0.0", "127.0.0.1", 1)
    } else if controller.starts_with("[::]:") {
        controller.replacen("[::]", "127.0.0.1", 1)
    } else {
        controller.to_string()
    };
    Ok(format!("http://{}", host_port))
}

fn controller_secret(config: &UserConfig) -> Result<Option<String>> {
    let runtime = RuntimeConfig::from_file(&config.config_runtime())?;
    Ok(runtime
        .get_string("secret")
        .filter(|secret| !secret.trim().is_empty()))
}

fn encode_path_segment(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}
