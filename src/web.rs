use anyhow::{Context, Result};
use axum::{
    extract::Form,
    http::header,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use std::process::Command;

use crate::{
    config::UserConfig, dashboard::Snapshot, i18n, mixin, proxy, service, subscription, tun, utils,
};

pub const DEFAULT_WEB_HOST: &str = "127.0.0.1";
pub const DEFAULT_WEB_PORT: u16 = 3737;

const APP_CSS: &str = r#"
:root {
    --bg: #f4efe7;
    --bg-strong: #efe3d4;
    --surface: rgba(255, 252, 247, 0.86);
    --surface-strong: rgba(255, 252, 247, 0.96);
    --text: #1f2a33;
    --muted: #5d6b76;
    --line: rgba(31, 42, 51, 0.14);
    --line-strong: rgba(31, 42, 51, 0.28);
    --accent: #d9643a;
    --accent-deep: #9f3f22;
    --accent-soft: rgba(217, 100, 58, 0.14);
    --ok: #1f8a62;
    --ok-soft: rgba(31, 138, 98, 0.14);
    --warn: #c07a1f;
    --warn-soft: rgba(192, 122, 31, 0.14);
    --danger: #c84646;
    --danger-soft: rgba(200, 70, 70, 0.14);
    --shadow: 0 18px 50px rgba(31, 42, 51, 0.08);
    --radius-xl: 28px;
    --radius-lg: 20px;
    --radius-md: 14px;
    --transition: 180ms cubic-bezier(0.22, 1, 0.36, 1);
    color-scheme: light;
    font-family: "Avenir Next", "Segoe UI", "PingFang SC", "Noto Sans CJK SC", sans-serif;
}

* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; }
body {
    background:
        radial-gradient(circle at top left, rgba(217, 100, 58, 0.14), transparent 34%),
        radial-gradient(circle at top right, rgba(31, 138, 98, 0.12), transparent 28%),
        linear-gradient(180deg, var(--bg), #f7f3ed 48%, #efe8df 100%);
    color: var(--text);
}
a { color: inherit; text-decoration: none; }
button, input {
    font: inherit;
}

.shell {
    width: min(1220px, calc(100vw - 32px));
    margin: 0 auto;
    padding: 28px 0 48px;
    position: relative;
}
.shell::before,
.shell::after {
    content: "";
    position: fixed;
    inset: auto;
    border: 1px solid rgba(31, 42, 51, 0.08);
    border-radius: 999px;
    pointer-events: none;
    opacity: 0.8;
}
.shell::before {
    width: 420px;
    height: 140px;
    right: -140px;
    top: 76px;
    transform: rotate(-14deg);
}
.shell::after {
    width: 320px;
    height: 110px;
    left: -120px;
    bottom: 88px;
    transform: rotate(18deg);
}

.topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 22px;
    border: 1px solid var(--line);
    border-radius: var(--radius-xl);
    background: rgba(255, 253, 250, 0.78);
    backdrop-filter: blur(20px);
    box-shadow: var(--shadow);
}
.brand {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.eyebrow,
.meta-chip,
.card-label,
.metric-label,
.field label,
.mono {
    font-family: "SFMono-Regular", "JetBrains Mono", "Menlo", monospace;
}
.eyebrow {
    margin: 0;
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--muted);
}
.brand h1,
.hero-copy h2 {
    margin: 0;
    font-family: "Iowan Old Style", "Palatino Linotype", "Songti SC", serif;
    letter-spacing: -0.04em;
}
.brand h1 {
    font-size: clamp(1.7rem, 2.6vw, 2.3rem);
}
.topbar-side {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: flex-end;
}
.meta-chip,
.chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 0 14px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.62);
    font-size: 12px;
}

.hero {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(300px, 0.85fr);
    gap: 18px;
    margin-top: 18px;
}
.hero-copy,
.hero-side,
.section-card {
    border: 1px solid var(--line);
    border-radius: var(--radius-xl);
    background: var(--surface);
    backdrop-filter: blur(20px);
    box-shadow: var(--shadow);
}
.hero-copy {
    padding: 28px;
}
.hero-copy h2 {
    font-size: clamp(2.6rem, 5vw, 4.5rem);
    line-height: 0.96;
}
.hero-copy p {
    margin: 18px 0 0;
    max-width: 56ch;
    line-height: 1.7;
    color: var(--muted);
}
.hero-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 22px;
}
.hero-side {
    padding: 24px;
    position: relative;
    overflow: hidden;
}
.hero-side::before {
    content: "";
    position: absolute;
    inset: 16px 16px auto auto;
    width: 72px;
    height: 72px;
    border-radius: 18px;
    background: linear-gradient(135deg, rgba(217, 100, 58, 0.16), rgba(31, 138, 98, 0.12));
}
.status-stack {
    position: relative;
    display: grid;
    gap: 12px;
}
.status-stack strong {
    display: block;
    font-size: 1.8rem;
    margin-top: 10px;
}

.flash {
    margin-top: 18px;
    padding: 15px 18px;
    border-radius: var(--radius-lg);
    border: 1px solid transparent;
    box-shadow: var(--shadow);
}
.flash.success {
    background: var(--ok-soft);
    border-color: rgba(31, 138, 98, 0.22);
}
.flash.error {
    background: var(--danger-soft);
    border-color: rgba(200, 70, 70, 0.22);
}

.dashboard {
    display: grid;
    grid-template-columns: 1.1fr 0.9fr;
    gap: 18px;
    margin-top: 18px;
}
.stack {
    display: grid;
    gap: 18px;
}
.section-card {
    padding: 22px;
}
.section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 18px;
}
.section-head h3 {
    margin: 0;
    font-size: 1.08rem;
}
.section-head p {
    margin: 6px 0 0;
    color: var(--muted);
}

.metric-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
}
.metric {
    padding: 16px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line);
    background: var(--surface-strong);
    transition: transform var(--transition), border-color var(--transition), box-shadow var(--transition);
}
.metric:hover {
    transform: translateY(-2px);
    border-color: var(--line-strong);
    box-shadow: 0 12px 30px rgba(31, 42, 51, 0.08);
}
.metric-label {
    display: block;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
}
.metric-value {
    display: block;
    margin-top: 10px;
    font-size: 1.5rem;
    letter-spacing: -0.03em;
}
.metric-note {
    display: block;
    margin-top: 8px;
    color: var(--muted);
    font-size: 0.94rem;
}

.card-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
}
.mini-card {
    padding: 16px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.72);
}
.card-label {
    display: block;
    margin-bottom: 10px;
    font-size: 12px;
    color: var(--muted);
    letter-spacing: 0.08em;
    text-transform: uppercase;
}
.mini-card a {
    color: var(--accent-deep);
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 3px;
}

.actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
}
.inline-form {
    margin: 0;
}
.button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 44px;
    padding: 0 18px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.72);
    color: var(--text);
    cursor: pointer;
    transition: transform var(--transition), border-color var(--transition), background var(--transition), box-shadow var(--transition);
}
.button:hover {
    transform: translateY(-1px);
    border-color: var(--line-strong);
    box-shadow: 0 10px 20px rgba(31, 42, 51, 0.08);
}
.button.primary {
    background: linear-gradient(135deg, var(--accent), #e88c58);
    color: #fffaf3;
    border-color: transparent;
}
.button.ok {
    background: linear-gradient(135deg, var(--ok), #31a073);
    color: #f7fff9;
    border-color: transparent;
}
.button.warn {
    background: linear-gradient(135deg, var(--warn), #de9c38);
    color: #fffaf3;
    border-color: transparent;
}
.button.danger {
    background: linear-gradient(135deg, var(--danger), #db6666);
    color: #fff7f7;
    border-color: transparent;
}

.form-grid {
    display: grid;
    gap: 14px;
}
.field {
    display: grid;
    gap: 8px;
}
.field label {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
}
.field input {
    width: 100%;
    min-height: 46px;
    padding: 0 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: rgba(255, 255, 255, 0.88);
    color: var(--text);
    transition: border-color var(--transition), box-shadow var(--transition), transform var(--transition);
}
.field input:focus {
    outline: none;
    border-color: rgba(217, 100, 58, 0.56);
    box-shadow: 0 0 0 4px rgba(217, 100, 58, 0.12);
}

.detail-grid {
    display: grid;
    gap: 10px;
}
.detail-row {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    padding-bottom: 10px;
    border-bottom: 1px solid rgba(31, 42, 51, 0.08);
}
.detail-row:last-child {
    padding-bottom: 0;
    border-bottom: 0;
}
.detail-key {
    color: var(--muted);
}
.detail-value {
    text-align: right;
    overflow-wrap: anywhere;
}
.detail-value.good { color: var(--ok); }
.detail-value.bad { color: var(--danger); }

.log-panel {
    margin: 0;
    padding: 18px;
    min-height: 280px;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(31, 42, 51, 0.1);
    background: #1f2a33;
    color: #eef5f7;
    overflow: auto;
    font-family: "SFMono-Regular", "JetBrains Mono", "Menlo", monospace;
    font-size: 13px;
    line-height: 1.6;
}

.footer-note {
    margin-top: 20px;
    color: var(--muted);
    font-size: 0.95rem;
    text-align: center;
}

@media (max-width: 980px) {
    .hero,
    .dashboard {
        grid-template-columns: 1fr;
    }
    .card-grid,
    .metric-grid {
        grid-template-columns: 1fr;
    }
}

@media (max-width: 720px) {
    .shell {
        width: min(100vw - 20px, 100%);
        padding-top: 16px;
    }
    .topbar,
    .hero-copy,
    .hero-side,
    .section-card {
        padding: 18px;
        border-radius: 22px;
    }
    .topbar,
    .section-head,
    .detail-row {
        flex-direction: column;
        align-items: flex-start;
    }
    .topbar-side {
        justify-content: flex-start;
    }
    .actions,
    .hero-actions {
        width: 100%;
    }
    .button {
        width: 100%;
    }
    .inline-form {
        width: 100%;
    }
}
"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Zh,
    En,
}

#[derive(Debug, Clone)]
struct Flash {
    message: String,
    is_error: bool,
}

#[derive(Deserialize)]
struct SecretForm {
    secret: String,
}

#[derive(Deserialize)]
struct SubscriptionForm {
    url: String,
}

struct PageData {
    snapshot: Snapshot,
    secret_value: String,
    lang: Lang,
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(home))
        .route("/assets/app.css", get(serve_css))
        .route("/actions/start", post(action_start))
        .route("/actions/stop", post(action_stop))
        .route("/actions/restart", post(action_restart))
        .route("/actions/proxy", post(action_toggle_proxy))
        .route("/actions/tun", post(action_toggle_tun))
        .route("/actions/secret", post(action_update_secret))
        .route("/actions/subscription", post(action_update_subscription))
}

pub async fn run(host: String, port: u16, no_open: bool) -> Result<()> {
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind Web dashboard on {addr}"))?;

    let browser_url = browser_url(&host, port);
    println!("openclash web dashboard: {browser_url}");

    if host == "0.0.0.0" {
        if let Ok(ip) = utils::get_local_ip() {
            println!("openclash web dashboard (LAN): http://{ip}:{port}");
        }
    }

    if !no_open {
        maybe_open_browser(&browser_url);
    }

    axum::serve(listener, router())
        .await
        .context("Web dashboard server exited unexpectedly")?;
    Ok(())
}

async fn home() -> Html<String> {
    render_page(None)
}

async fn action_start() -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let config = UserConfig::load()?;
        service::ensure_ready(&config)?;
        service::start(&config)?;
        Ok(tr(lang, "服务已启动", "Service started").to_string())
    })();
    render_result(result)
}

async fn action_stop() -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let config = UserConfig::load()?;
        service::stop(&config)?;
        Ok(tr(lang, "服务已停止", "Service stopped").to_string())
    })();
    render_result(result)
}

async fn action_restart() -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let config = UserConfig::load()?;
        service::ensure_ready(&config)?;
        service::restart(&config)?;
        Ok(tr(lang, "服务已重启", "Service restarted").to_string())
    })();
    render_result(result)
}

async fn action_toggle_proxy() -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let config = UserConfig::load()?;
        service::ensure_ready(&config)?;
        let snapshot = Snapshot::load(&config)?;
        if snapshot.system_proxy_enabled {
            proxy::unset_system_proxy(&config)?;
            Ok(tr(lang, "系统代理已关闭", "System proxy disabled").to_string())
        } else {
            proxy::set_system_proxy(&config)?;
            Ok(tr(lang, "系统代理已开启", "System proxy enabled").to_string())
        }
    })();
    render_result(result)
}

async fn action_toggle_tun() -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let config = UserConfig::load()?;
        service::ensure_ready(&config)?;
        let snapshot = Snapshot::load(&config)?;
        if snapshot.tun_enabled {
            tun::tun_off(&config)?;
            Ok(tr(lang, "TUN 已关闭", "TUN disabled").to_string())
        } else {
            tun::tun_on(&config)?;
            Ok(tr(lang, "TUN 已开启", "TUN enabled").to_string())
        }
    })();
    render_result(result)
}

async fn action_update_secret(Form(payload): Form<SecretForm>) -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let mut config = UserConfig::load()?;
        mixin::set_secret(&mut config, Some(payload.secret))?;
        Ok(tr(lang, "控制密钥已更新", "Controller secret updated").to_string())
    })();
    render_result(result)
}

async fn action_update_subscription(Form(payload): Form<SubscriptionForm>) -> Html<String> {
    let lang = current_lang();
    let result = (|| -> Result<String> {
        let mut config = UserConfig::load()?;
        let url = if payload.url.trim().is_empty() {
            None
        } else {
            Some(payload.url.trim().to_string())
        };
        subscription::update_sync(&mut config, url)?;
        Ok(tr(lang, "订阅已更新", "Subscription updated").to_string())
    })();
    render_result(result)
}

async fn serve_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS)
}

fn render_result(result: Result<String>) -> Html<String> {
    match result {
        Ok(message) => render_page(Some(Flash {
            message,
            is_error: false,
        })),
        Err(err) => render_page(Some(Flash {
            message: err.to_string(),
            is_error: true,
        })),
    }
}

fn render_page(flash: Option<Flash>) -> Html<String> {
    let lang = current_lang();
    let body = match load_page_data(lang) {
        Ok(data) => layout(&data, flash).into_string(),
        Err(err) => error_layout(lang, &err.to_string()).into_string(),
    };
    Html(body)
}

fn load_page_data(lang: Lang) -> Result<PageData> {
    let config = UserConfig::load()?;
    let snapshot = Snapshot::load(&config)?;
    let secret_value = if config.config_runtime().exists() {
        crate::config::RuntimeConfig::from_file(&config.config_runtime())?
            .get_string("secret")
            .unwrap_or_default()
    } else {
        String::new()
    };

    Ok(PageData {
        snapshot,
        secret_value,
        lang,
    })
}

fn layout(data: &PageData, flash: Option<Flash>) -> Markup {
    let running_label = if data.snapshot.running {
        tr(data.lang, "运行中", "Running")
    } else {
        tr(data.lang, "已停止", "Stopped")
    };
    let proxy_label = if data.snapshot.system_proxy_enabled {
        tr(data.lang, "代理已开启", "Proxy enabled")
    } else {
        tr(data.lang, "代理已关闭", "Proxy disabled")
    };
    let tun_label = if data.snapshot.tun_enabled {
        tr(data.lang, "TUN 已开启", "TUN enabled")
    } else {
        tr(data.lang, "TUN 已关闭", "TUN disabled")
    };
    let controller_link = data.snapshot.local_ui.as_deref();

    html! {
        (DOCTYPE)
        html lang=(html_lang(data.lang)) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "openclash web" }
                link rel="stylesheet" href="/assets/app.css";
            }
            body {
                div.shell {
                    header.topbar {
                        div.brand {
                            p.eyebrow { (tr(data.lang, "Rust 托管控制台", "Rust-hosted control surface")) }
                            h1 { "openclash web" }
                        }
                        div.topbar-side {
                            span.meta-chip { (format!("v{}", env!("CARGO_PKG_VERSION"))) }
                            span.meta-chip { (format!("{} · {}", running_label, proxy_label)) }
                            a.button href="/" { (tr(data.lang, "刷新页面", "Refresh")) }
                        }
                    }

                    section.hero {
                        div.hero-copy {
                            p.eyebrow { (tr(data.lang, "参考 memorph 的 Web 结构", "Inspired by memorph web")) }
                            h2 { (tr(data.lang, "把 TUI 的控制力，搬进可直接访问的页面里。", "Bring the TUI control surface into a directly accessible page.")) }
                            p {
                                (tr(
                                    data.lang,
                                    "这个页面由 Rust 直接托管，保留了现有 Mihomo controller 的入口，同时把服务状态、网络开关、订阅更新和日志整合成一个统一的 Web Dashboard。",
                                    "This page is hosted directly by Rust, keeps the existing Mihomo controller reachable, and folds service state, network toggles, subscription refresh, and logs into one web dashboard.",
                                ))
                            }
                            div.hero-actions {
                                @if data.snapshot.running {
                                    form.inline-form method="post" action="/actions/stop" onsubmit=(confirm_attr(data.lang, "停止当前服务？", "Stop the current service?")) {
                                        button.button.danger type="submit" { (tr(data.lang, "停止服务", "Stop service")) }
                                    }
                                    form.inline-form method="post" action="/actions/restart" onsubmit=(confirm_attr(data.lang, "重启服务并重载配置？", "Restart the service and reload config?")) {
                                        button.button.warn type="submit" { (tr(data.lang, "重启服务", "Restart service")) }
                                    }
                                } @else {
                                    form.inline-form method="post" action="/actions/start" onsubmit=(confirm_attr(data.lang, "准备资源并启动服务？", "Prepare resources and start the service?")) {
                                        button.button.primary type="submit" { (tr(data.lang, "启动服务", "Start service")) }
                                    }
                                }
                                @if let Some(url) = controller_link {
                                    a.button.ok href=(url) target="_blank" rel="noreferrer" {
                                        (tr(data.lang, "打开 Mihomo 控制台", "Open Mihomo controller"))
                                    }
                                }
                            }
                        }
                        div.hero-side {
                            div.status-stack {
                                span.card-label { (tr(data.lang, "当前总览", "Current overview")) }
                                strong { (running_label) }
                                p {
                                    (tr(data.lang, "系统代理：", "System proxy: ")) (proxy_label)
                                    br;
                                    (tr(data.lang, "TUN：", "TUN: ")) (tun_label)
                                    br;
                                    (tr(data.lang, "Dashboard 命令：", "Dashboard command: ")) code.mono { "openclash web" }
                                }
                            }
                        }
                    }

                    @if let Some(flash) = flash {
                        div class=(if flash.is_error { "flash error" } else { "flash success" }) {
                            (flash.message)
                        }
                    }

                    section.dashboard {
                        div.stack {
                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "运行状态", "Runtime status")) }
                                        p { (tr(data.lang, "从现有 TUI 状态抽成的 Web 视图。", "A web view extracted from the existing TUI status model.")) }
                                    }
                                }
                                div.metric-grid {
                                    (metric_card(data.lang, tr(data.lang, "Ready", "Ready"), yes_no(data.snapshot.ready), tr(data.lang, "资源、mixin、runtime 是否齐备", "Whether resources, mixin, and runtime are all ready")))
                                    (metric_card(data.lang, tr(data.lang, "Service", "Service"), yes_no(data.snapshot.running), tr(data.lang, "当前内核进程状态", "Current kernel process state")))
                                    (metric_card(data.lang, tr(data.lang, "Mixed Port", "Mixed Port"), data.snapshot.mixed_port.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()), tr(data.lang, "系统代理和测试流量使用的入口", "Entry used by system proxy and test traffic")))
                                    (metric_card(data.lang, tr(data.lang, "Controller Port", "Controller Port"), data.snapshot.ui_port.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()), tr(data.lang, "Mihomo external controller 端口", "Port exposed by Mihomo external controller")))
                                    (metric_card(data.lang, tr(data.lang, "System Proxy", "System Proxy"), yes_no(data.snapshot.system_proxy_enabled), tr(data.lang, "当前系统级代理开关", "Current system-level proxy toggle")))
                                    (metric_card(data.lang, tr(data.lang, "TUN", "TUN"), yes_no(data.snapshot.tun_enabled), tr(data.lang, "当前 TUN 模式状态", "Current TUN mode status")))
                                }
                            }

                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "访问入口", "Access points")) }
                                        p { (tr(data.lang, "Dashboard 自己托管，底层 controller 继续保留。", "The dashboard is self-hosted while the underlying controller stays available.")) }
                                    }
                                }
                                div.card-grid {
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Dashboard 命令", "Dashboard command")) }
                                        div.mono { (format!("openclash web --host {} --port {}", DEFAULT_WEB_HOST, DEFAULT_WEB_PORT)) }
                                    }
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Kernel", "Kernel")) }
                                        div { (data.snapshot.kernel_name.clone()) }
                                    }
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Local Controller", "Local Controller")) }
                                        (access_value(data.snapshot.local_ui.as_deref()))
                                    }
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Public Controller", "Public Controller")) }
                                        (access_value(data.snapshot.public_ui.as_deref()))
                                    }
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Shared Address", "Shared Address")) }
                                        (access_value(Some(&data.snapshot.common_ui)))
                                    }
                                    div.mini-card {
                                        span.card-label { (tr(data.lang, "Base Directory", "Base Directory")) }
                                        div.mono { (&data.snapshot.base_dir) }
                                    }
                                }
                            }

                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "实时日志", "Recent logs")) }
                                        p { (tr(data.lang, "沿用现有 run/mihomo.log 的最近输出。", "Uses the latest output from the existing run/mihomo.log.")) }
                                    }
                                }
                                pre.log-panel {
                                    @if data.snapshot.logs.is_empty() {
                                        (tr(data.lang, "还没有内核日志。", "No kernel log yet."))
                                    } @else {
                                        (data.snapshot.logs.join("\n"))
                                    }
                                }
                            }
                        }

                        div.stack {
                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "快捷操作", "Quick actions")) }
                                        p { (tr(data.lang, "所有动作都直接复用现有 Rust 服务层。", "Each action directly reuses the existing Rust service layer.")) }
                                    }
                                }
                                div.actions {
                                    @if data.snapshot.running {
                                        form.inline-form method="post" action="/actions/stop" onsubmit=(confirm_attr(data.lang, "停止当前服务？", "Stop the current service?")) {
                                            button.button.danger type="submit" { (tr(data.lang, "停止服务", "Stop service")) }
                                        }
                                        form.inline-form method="post" action="/actions/restart" onsubmit=(confirm_attr(data.lang, "重启服务并重载配置？", "Restart the service and reload config?")) {
                                            button.button.warn type="submit" { (tr(data.lang, "重启服务", "Restart service")) }
                                        }
                                    } @else {
                                        form.inline-form method="post" action="/actions/start" onsubmit=(confirm_attr(data.lang, "准备资源并启动服务？", "Prepare resources and start the service?")) {
                                            button.button.primary type="submit" { (tr(data.lang, "启动服务", "Start service")) }
                                        }
                                    }
                                    form.inline-form method="post" action="/actions/proxy" onsubmit=(confirm_attr(data.lang, "切换系统代理状态？", "Toggle system proxy?")) {
                                        button class=(if data.snapshot.system_proxy_enabled { "button danger" } else { "button ok" }) type="submit" {
                                            @if data.snapshot.system_proxy_enabled {
                                                (tr(data.lang, "关闭系统代理", "Disable system proxy"))
                                            } @else {
                                                (tr(data.lang, "开启系统代理", "Enable system proxy"))
                                            }
                                        }
                                    }
                                    form.inline-form method="post" action="/actions/tun" onsubmit=(confirm_attr(data.lang, "切换 TUN 状态并重启服务？", "Toggle TUN and restart the service?")) {
                                        button class=(if data.snapshot.tun_enabled { "button danger" } else { "button ok" }) type="submit" {
                                            @if data.snapshot.tun_enabled {
                                                (tr(data.lang, "关闭 TUN", "Disable TUN"))
                                            } @else {
                                                (tr(data.lang, "开启 TUN", "Enable TUN"))
                                            }
                                        }
                                    }
                                    a.button href="/" { (tr(data.lang, "刷新状态", "Refresh state")) }
                                }
                            }

                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "配置入口", "Configuration")) }
                                        p { (tr(data.lang, "Secret 与订阅沿用现有配置写入逻辑。", "Secret and subscription reuse the existing config write path.")) }
                                    }
                                }
                                div.form-grid {
                                    form method="post" action="/actions/secret" {
                                        div.field {
                                            label for="secret" { (tr(data.lang, "Controller Secret", "Controller Secret")) }
                                            input id="secret" name="secret" type="text" value=(&data.secret_value) placeholder=(tr(data.lang, "留空表示清空 secret", "Leave empty to clear secret"));
                                        }
                                        button.button.primary type="submit" style="margin-top: 12px;" { (tr(data.lang, "保存 Secret", "Save secret")) }
                                    }
                                    form method="post" action="/actions/subscription" {
                                        div.field {
                                            label for="subscription" { (tr(data.lang, "Subscription URL", "Subscription URL")) }
                                            input id="subscription" name="url" type="url" value=(&data.snapshot.subscription_url) placeholder="https://...";
                                        }
                                        button.button.primary type="submit" style="margin-top: 12px;" { (tr(data.lang, "更新订阅", "Update subscription")) }
                                    }
                                }
                            }

                            section.section-card {
                                div.section-head {
                                    div {
                                        h3 { (tr(data.lang, "细节与文件", "Details & files")) }
                                        p { (tr(data.lang, "保留当前工程里最重要的运行数据。", "Keeps the most important runtime details from the current project.")) }
                                    }
                                }
                                div.detail-grid {
                                    (detail_row(tr(data.lang, "PID", "PID"), data.snapshot.pid.as_deref().unwrap_or("-"), None))
                                    (detail_row(tr(data.lang, "Secret Preview", "Secret Preview"), &data.snapshot.secret_preview, None))
                                    (detail_row(tr(data.lang, "raw", "raw"), exists_text(data.snapshot.config_raw_exists), Some(data.snapshot.config_raw_exists)))
                                    (detail_row(tr(data.lang, "mixin", "mixin"), exists_text(data.snapshot.config_mixin_exists), Some(data.snapshot.config_mixin_exists)))
                                    (detail_row(tr(data.lang, "runtime", "runtime"), exists_text(data.snapshot.config_runtime_exists), Some(data.snapshot.config_runtime_exists)))
                                }
                            }
                        }
                    }

                    p.footer-note {
                        (tr(
                            data.lang,
                            "如果你还想把连接、代理组、规则切换也继续搬进这个页面，下一步就该接 Mihomo controller API，而不是继续依赖 yacd。",
                            "If you want connections, proxy groups, and rule switching inside this page too, the next step is to wire Mihomo controller APIs directly instead of continuing to depend on yacd.",
                        ))
                    }
                }
            }
        }
    }
}

fn error_layout(lang: Lang, message: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang=(html_lang(lang)) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "openclash web" }
                link rel="stylesheet" href="/assets/app.css";
            }
            body {
                div.shell {
                    div.section-card {
                        div.section-head {
                            div {
                                h3 { (tr(lang, "页面加载失败", "Page failed to load")) }
                                p { (tr(lang, "无法构建 dashboard。", "Unable to build the dashboard.")) }
                            }
                        }
                        div class="flash error" { (message) }
                    }
                }
            }
        }
    }
}

fn metric_card(_lang: Lang, label: &str, value: String, note: &str) -> Markup {
    html! {
        div.metric {
            span.metric-label { (label) }
            span.metric-value { (value) }
            span.metric-note { (note) }
        }
    }
}

fn detail_row(label: &str, value: &str, status: Option<bool>) -> Markup {
    let class = match status {
        Some(true) => "detail-value good",
        Some(false) => "detail-value bad",
        None => "detail-value",
    };

    html! {
        div.detail-row {
            span.detail-key { (label) }
            span class=(class) { (value) }
        }
    }
}

fn access_value(value: Option<&str>) -> Markup {
    let Some(value) = value else {
        return html! { div { "-" } };
    };

    if value.contains("公网") {
        html! { div { (value) } }
    } else {
        html! {
            a href=(value) target="_blank" rel="noreferrer" { (value) }
        }
    }
}

fn confirm_attr(lang: Lang, zh: &'static str, en: &'static str) -> String {
    format!("return confirm({:?})", tr(lang, zh, en))
}

fn browser_url(host: &str, port: u16) -> String {
    if host == "0.0.0.0" {
        format!("http://127.0.0.1:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

fn maybe_open_browser(url: &str) {
    if which::which("xdg-open").is_ok() {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

fn current_lang() -> Lang {
    if i18n::get_current_lang() == "en" {
        Lang::En
    } else {
        Lang::Zh
    }
}

fn tr(lang: Lang, zh: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

fn html_lang(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "zh-CN",
        Lang::En => "en",
    }
}

fn yes_no(value: bool) -> String {
    if value {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

fn exists_text(value: bool) -> &'static str {
    if value {
        "present"
    } else {
        "missing"
    }
}
