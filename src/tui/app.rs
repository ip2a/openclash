use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::{
    config::UserConfig,
    controller::{self, ProxyGroup},
    dashboard::Snapshot,
    mixin::{self, LanSettings},
    proxy, service, subscription, tun, web,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ToggleProxy,
    SelectNode,
    ConfigureLan,
    ToggleTun,
    UpdateSubscription,
    ShowUi,
    Refresh,
    Quit,
}

impl Action {
    pub fn description(self, snapshot: &Snapshot) -> String {
        match self {
            Self::ToggleProxy => {
                if snapshot.system_proxy_enabled {
                    "关闭系统代理".to_string()
                } else {
                    "开启系统代理，服务未运行时会自动启动".to_string()
                }
            }
            Self::SelectNode => "查看代理组节点并切换当前节点".to_string(),
            Self::ConfigureLan => "配置局域网访问开关和 mixed-port".to_string(),
            Self::ToggleTun => {
                if snapshot.tun_enabled {
                    "关闭 TUN 模式".to_string()
                } else {
                    "开启 TUN 模式并重载配置".to_string()
                }
            }
            Self::UpdateSubscription => "更新订阅以刷新节点列表".to_string(),
            Self::ShowUi => "打开 Web dashboard 或 Mihomo controller".to_string(),
            Self::Refresh => "重新读取状态、配置和日志".to_string(),
            Self::Quit => "退出终端控制台".to_string(),
        }
    }

    pub fn label(self, snapshot: &Snapshot) -> String {
        match self {
            Self::ToggleProxy => {
                if snapshot.system_proxy_enabled {
                    "关闭系统代理".to_string()
                } else {
                    "开启系统代理".to_string()
                }
            }
            Self::SelectNode => "节点列表 / 切换节点".to_string(),
            Self::ConfigureLan => "配置局域网端口".to_string(),
            Self::ToggleTun => {
                if snapshot.tun_enabled {
                    "关闭 TUN 模式".to_string()
                } else {
                    "打开 TUN 模式".to_string()
                }
            }
            Self::UpdateSubscription => "更新订阅".to_string(),
            Self::ShowUi => "Web 入口".to_string(),
            Self::Refresh => "刷新状态".to_string(),
            Self::Quit => "退出".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    Confirm { action: Action, prompt: String },
    Input(InputState),
    Lan(LanInputState),
    Nodes(NodePickerState),
    Info { title: String, lines: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    SubscriptionUrl,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub mode: InputMode,
    pub title: String,
    pub hint: String,
    pub value: String,
    pub cursor: usize,
}

#[derive(Debug, Clone)]
pub struct LanInputState {
    pub allow_lan: bool,
    pub port_value: String,
    pub cursor: usize,
}

#[derive(Debug, Clone)]
pub struct NodePickerState {
    pub groups: Vec<ProxyGroup>,
    pub group_index: usize,
    pub node_index: usize,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub expires_at: Instant,
}

pub struct App {
    pub config: UserConfig,
    pub snapshot: Snapshot,
    pub selected_action: usize,
    pub modal: Option<Modal>,
    pub toast: Option<Toast>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = UserConfig::load()?;
        service::ensure_ready(&config)?;
        let snapshot = Snapshot::load(&config)?;
        Ok(Self {
            config,
            snapshot,
            selected_action: 0,
            modal: None,
            toast: None,
        })
    }

    pub fn actions(&self) -> Vec<Action> {
        vec![
            Action::ToggleProxy,
            Action::SelectNode,
            Action::ConfigureLan,
            Action::ToggleTun,
            Action::UpdateSubscription,
            Action::ShowUi,
            Action::Refresh,
            Action::Quit,
        ]
    }

    pub fn selected_action(&self) -> Action {
        let actions = self.actions();
        actions[self.selected_action.min(actions.len().saturating_sub(1))]
    }

    pub fn status_line(&self) -> String {
        let service = if self.snapshot.running {
            "Running"
        } else {
            "Stopped"
        };
        let proxy = if self.snapshot.system_proxy_enabled {
            "Proxy On"
        } else {
            "Proxy Off"
        };
        let tun = if self.snapshot.tun_enabled {
            "TUN On"
        } else {
            "TUN Off"
        };
        format!("{}  |  {}  |  {}", service, proxy, tun)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }

        match self.modal.as_mut() {
            Some(Modal::Help) => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                    self.modal = None;
                }
                return Ok(false);
            }
            Some(Modal::Info { .. }) => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                    self.modal = None;
                }
                return Ok(false);
            }
            Some(Modal::Confirm { action, .. }) => {
                match key.code {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Enter => {
                        let action = *action;
                        self.modal = None;
                        if action == Action::Quit {
                            return Ok(true);
                        }
                        self.execute_action(action)?;
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Some(Modal::Input(state)) => {
                match key.code {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Enter => {
                        let state = state.clone();
                        self.modal = None;
                        self.submit_input(state)?;
                    }
                    KeyCode::Backspace => {
                        if state.cursor > 0 {
                            state.cursor -= 1;
                            state.value.remove(state.cursor);
                        }
                    }
                    KeyCode::Left => state.cursor = state.cursor.saturating_sub(1),
                    KeyCode::Right => {
                        state.cursor = (state.cursor + 1).min(state.value.chars().count());
                    }
                    KeyCode::Char(ch)
                        if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                    {
                        let byte_idx = char_to_byte_index(&state.value, state.cursor);
                        state.value.insert(byte_idx, ch);
                        state.cursor += 1;
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Some(Modal::Lan(state)) => {
                match key.code {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Enter => {
                        let state = state.clone();
                        self.modal = None;
                        self.submit_lan_settings(state)?;
                    }
                    KeyCode::Char(' ') | KeyCode::Char('a') | KeyCode::Char('A') => {
                        state.allow_lan = !state.allow_lan;
                    }
                    KeyCode::Backspace => {
                        if state.cursor > 0 {
                            state.cursor -= 1;
                            state.port_value.remove(state.cursor);
                        }
                    }
                    KeyCode::Left => state.cursor = state.cursor.saturating_sub(1),
                    KeyCode::Right => {
                        state.cursor = (state.cursor + 1).min(state.port_value.chars().count());
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        let byte_idx = char_to_byte_index(&state.port_value, state.cursor);
                        state.port_value.insert(byte_idx, ch);
                        state.cursor += 1;
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Some(Modal::Nodes(state)) => {
                match key.code {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Left => {
                        state.group_index = state.group_index.saturating_sub(1);
                        state.node_index = current_node_index(state);
                    }
                    KeyCode::Right => {
                        state.group_index =
                            (state.group_index + 1).min(state.groups.len().saturating_sub(1));
                        state.node_index = current_node_index(state);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.node_index = state.node_index.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let last = state
                            .groups
                            .get(state.group_index)
                            .map(|group| group.nodes.len().saturating_sub(1))
                            .unwrap_or(0);
                        state.node_index = (state.node_index + 1).min(last);
                    }
                    KeyCode::Enter => {
                        let state = state.clone();
                        self.switch_selected_node(state)?;
                    }
                    _ => {}
                }
                return Ok(false);
            }
            None => {}
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_action = self.selected_action.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let last = self.actions().len().saturating_sub(1);
                self.selected_action = (self.selected_action + 1).min(last);
            }
            KeyCode::Char('r') => {
                self.execute_action(Action::Refresh)?;
            }
            KeyCode::Enter => {
                let action = self.selected_action();
                self.open_action(action);
            }
            _ => {}
        }

        Ok(false)
    }

    pub fn on_tick(&mut self) {
        if let Some(toast) = self.toast.as_ref() {
            if Instant::now() >= toast.expires_at {
                self.toast = None;
            }
        }
    }

    fn open_action(&mut self, action: Action) {
        match action {
            Action::SelectNode => {
                if let Err(err) = self.open_node_picker() {
                    self.set_error(err.to_string());
                }
            }
            Action::ConfigureLan => match mixin::lan_settings(&self.config) {
                Ok(settings) => {
                    let port_value = settings.mixed_port.to_string();
                    self.modal = Some(Modal::Lan(LanInputState {
                        allow_lan: settings.allow_lan,
                        cursor: port_value.len(),
                        port_value,
                    }));
                }
                Err(err) => self.set_error(err.to_string()),
            },
            Action::ShowUi => {
                let mut lines = vec![format!(
                    "Dashboard: openclash web --port {}",
                    web::DEFAULT_WEB_PORT
                )];

                match mixin::ui_info(&self.config) {
                    Ok(info) => {
                        lines.push(format!("Controller Port: {}", info.port));
                        lines.push(format!("Controller Local: {}", info.local_address));
                        lines.push(format!("Controller Public: {}", info.public_address));
                        lines.push(format!("Shared: {}", info.common_address));
                    }
                    Err(err) => {
                        lines.push(format!("Controller: {}", err));
                    }
                }

                self.modal = Some(Modal::Info {
                    title: "Web Access".to_string(),
                    lines,
                });
            }
            Action::UpdateSubscription => {
                self.modal = Some(Modal::Input(InputState {
                    mode: InputMode::SubscriptionUrl,
                    title: "Update Subscription".to_string(),
                    hint: "输入新的订阅 URL，留空则使用当前配置里的 subscription_url。".to_string(),
                    value: self.snapshot.subscription_url.clone(),
                    cursor: self.snapshot.subscription_url.chars().count(),
                }));
            }
            Action::Refresh => {
                let _ = self.execute_action(Action::Refresh);
            }
            Action::Quit => {
                self.modal = Some(Modal::Confirm {
                    action,
                    prompt: "退出当前 TUI 控制台？".to_string(),
                });
            }
            _ => {
                self.modal = Some(Modal::Confirm {
                    action,
                    prompt: confirm_prompt(action, &self.snapshot),
                });
            }
        }
    }

    fn submit_input(&mut self, input: InputState) -> Result<()> {
        match input.mode {
            InputMode::SubscriptionUrl => {
                let url = if input.value.trim().is_empty() {
                    None
                } else {
                    Some(input.value.trim().to_string())
                };
                subscription::update_sync(&mut self.config, url)?;
                self.refresh_with_notice("Subscription updated", false)?;
            }
        }
        Ok(())
    }

    fn submit_lan_settings(&mut self, input: LanInputState) -> Result<()> {
        let port: u16 = input
            .port_value
            .parse()
            .map_err(|_| anyhow::anyhow!("请输入有效端口号"))?;
        if port == 0 {
            anyhow::bail!("端口号必须大于 0");
        }

        mixin::set_lan_settings(
            &self.config,
            LanSettings {
                allow_lan: input.allow_lan,
                mixed_port: port,
            },
        )?;
        self.refresh_with_notice("LAN settings updated", false)?;
        Ok(())
    }

    fn open_node_picker(&mut self) -> Result<()> {
        service::ensure_ready(&self.config)?;
        if !self.snapshot.running {
            service::start(&self.config)?;
            self.snapshot = Snapshot::load(&self.config)?;
        }

        let groups = controller::proxy_groups(&self.config)?;
        if groups.is_empty() {
            anyhow::bail!("没有可切换的代理组，请先更新订阅");
        }

        self.modal = Some(Modal::Nodes(NodePickerState {
            node_index: current_node_index_for_group(&groups[0]),
            groups,
            group_index: 0,
        }));
        Ok(())
    }

    fn switch_selected_node(&mut self, state: NodePickerState) -> Result<()> {
        let Some(group) = state.groups.get(state.group_index) else {
            return Ok(());
        };
        let Some(node) = group.nodes.get(state.node_index) else {
            return Ok(());
        };
        controller::switch_proxy(&self.config, &group.name, node)?;
        self.modal = None;
        self.refresh_with_notice(&format!("{} -> {}", group.name, node), false)?;
        Ok(())
    }

    fn execute_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::ToggleProxy => {
                service::ensure_ready(&self.config)?;
                if self.snapshot.system_proxy_enabled {
                    proxy::unset_system_proxy(&self.config)?;
                    self.refresh_with_notice("System proxy disabled", false)?;
                } else {
                    if !self.snapshot.running {
                        service::start(&self.config)?;
                    }
                    proxy::set_system_proxy(&self.config)?;
                    self.refresh_with_notice("System proxy enabled", false)?;
                }
            }
            Action::ToggleTun => {
                service::ensure_ready(&self.config)?;
                if self.snapshot.tun_enabled {
                    tun::tun_off(&self.config)?;
                    self.refresh_with_notice("TUN disabled", false)?;
                } else {
                    tun::tun_on(&self.config)?;
                    self.refresh_with_notice("TUN enabled", false)?;
                }
            }
            Action::SelectNode
            | Action::ConfigureLan
            | Action::UpdateSubscription
            | Action::ShowUi => {}
            Action::Refresh => {
                self.snapshot = Snapshot::load(&self.config)?;
                self.selected_action = self
                    .selected_action
                    .min(self.actions().len().saturating_sub(1));
                self.set_notice("State refreshed");
            }
            Action::Quit => {}
        }
        Ok(())
    }

    fn refresh_with_notice(&mut self, message: &str, is_error: bool) -> Result<()> {
        self.config = UserConfig::load()?;
        self.snapshot = Snapshot::load(&self.config)?;
        self.selected_action = self
            .selected_action
            .min(self.actions().len().saturating_sub(1));
        self.toast = Some(Toast {
            message: message.to_string(),
            is_error,
            expires_at: Instant::now() + Duration::from_secs(4),
        });
        Ok(())
    }

    pub fn set_error(&mut self, message: String) {
        self.write_log("ERROR", &message);
        self.toast = Some(Toast {
            message,
            is_error: true,
            expires_at: Instant::now() + Duration::from_secs(5),
        });
    }

    fn set_notice(&mut self, message: &str) {
        self.write_log("INFO", message);
        self.toast = Some(Toast {
            message: message.to_string(),
            is_error: false,
            expires_at: Instant::now() + Duration::from_secs(3),
        });
    }

    fn write_log(&self, level: &str, message: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.config.tui_log())
        {
            let _ = writeln!(
                file,
                "[{}] {} {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                level,
                message
            );
        }
    }
}

fn confirm_prompt(action: Action, snapshot: &Snapshot) -> String {
    match action {
        Action::ToggleProxy => {
            if snapshot.system_proxy_enabled {
                "关闭系统代理？".to_string()
            } else {
                "开启系统代理？服务未运行时会自动启动。".to_string()
            }
        }
        Action::ToggleTun => {
            if snapshot.tun_enabled {
                "关闭 TUN 并重载配置？".to_string()
            } else {
                "开启 TUN 并重载配置？".to_string()
            }
        }
        Action::Quit => "退出当前 TUI 控制台？".to_string(),
        _ => "执行该操作？".to_string(),
    }
}

fn current_node_index(state: &NodePickerState) -> usize {
    state
        .groups
        .get(state.group_index)
        .map(current_node_index_for_group)
        .unwrap_or(0)
}

fn current_node_index_for_group(group: &ProxyGroup) -> usize {
    group
        .nodes
        .iter()
        .position(|node| node == &group.now)
        .unwrap_or(0)
}

fn char_to_byte_index(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(input.len())
}
