use crate::config::UserConfig;
use crate::service::is_running;
use crate::utils::{failcat, okcat, okcat_with_emoji};
use anyhow::{Context, Result};
use std::env;
use std::time::{Duration, Instant};

/// Default test URLs
const TEST_URLS: &[(&str, &str)] = &[
    ("Google", "http://www.google.com/generate_204"),
    ("YouTube", "http://www.youtube.com"),
    ("GitHub", "http://github.com"),
];

/// Test proxy connectivity
pub fn test_connectivity(
    config: &UserConfig,
    custom_url: Option<String>,
    custom_port: Option<u16>,
) -> Result<()> {
    crate::service::ensure_ready(config)?;

    if !is_running(config)? {
        failcat("代理程序未运行，请先执行 openclash on");
        return Ok(());
    }

    // Determine proxy port
    let port = match custom_port {
        Some(p) => p,
        None => {
            // Try env vars first
            if let Ok(proxy) = env::var("all_proxy").or_else(|_| env::var("ALL_PROXY")) {
                if let Some(port_str) = proxy.rsplit(':').next() {
                    if let Ok(p) = port_str.parse::<u16>() {
                        p
                    } else {
                        crate::service::get_proxy_port(config)?
                    }
                } else {
                    crate::service::get_proxy_port(config)?
                }
            } else if let Ok(proxy) = env::var("http_proxy").or_else(|_| env::var("HTTP_PROXY")) {
                if let Some(port_str) = proxy.rsplit(':').next() {
                    if let Ok(p) = port_str.parse::<u16>() {
                        p
                    } else {
                        crate::service::get_proxy_port(config)?
                    }
                } else {
                    crate::service::get_proxy_port(config)?
                }
            } else {
                crate::service::get_proxy_port(config)?
            }
        }
    };

    okcat_with_emoji("🧪", &format!("测试代理连通性 (端口: {})...", port));
    println!();

    let proxy_url = format!("socks5h://127.0.0.1:{}", port);

    if let Some(url) = custom_url {
        test_single_url(&proxy_url, "Custom", &url)?;
    } else {
        let mut all_passed = true;
        for (name, url) in TEST_URLS {
            match test_single_url(&proxy_url, name, url) {
                Ok(_) => {}
                Err(_) => all_passed = false,
            }
        }

        println!();
        if all_passed {
            okcat("✅ 所有测试通过，代理工作正常");
        } else {
            failcat("⚠️ 部分测试未通过，请检查配置");
        }
    }

    Ok(())
}

fn test_single_url(proxy_url: &str, name: &str, url: &str) -> Result<()> {
    let proxy =
        reqwest::Proxy::all(proxy_url).with_context(|| format!("无法创建代理: {}", proxy_url))?;

    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(8))
        .danger_accept_invalid_certs(true)
        .build()
        .context("无法创建 HTTP 客户端")?;

    let start = Instant::now();
    let result = client.get(url).send();
    let elapsed = start.elapsed();

    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() || status.as_u16() == 204 {
                okcat(&format!(
                    "{:<12} ✅ {:>6}ms  {}",
                    name,
                    elapsed.as_millis(),
                    status
                ));
                Ok(())
            } else {
                failcat(&format!(
                    "{:<12} ❌ {:>6}ms  HTTP {}",
                    name,
                    elapsed.as_millis(),
                    status
                ));
                anyhow::bail!("HTTP {}", status)
            }
        }
        Err(e) => {
            failcat(&format!(
                "{:<12} ❌ {:>6}ms  {}",
                name,
                elapsed.as_millis(),
                e
            ));
            Err(e.into())
        }
    }
}
