use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::{
    app::{App, InputState, LanInputState, Modal, NodePickerState},
    theme::{truncate, Theme},
};

pub fn draw(frame: &mut Frame, app: &App) {
    let theme = Theme::default();
    let area = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(theme.background).fg(theme.text)),
        area,
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, app, layout[0], &theme);
    draw_main(frame, app, layout[1], &theme);
    draw_footer(frame, app, layout[2], &theme);

    if let Some(modal) = app.modal.as_ref() {
        match modal {
            Modal::Help => draw_help_modal(frame, &theme),
            Modal::Confirm { prompt, .. } => draw_confirm_modal(frame, prompt, &theme),
            Modal::Input(state) => draw_input_modal(frame, state, &theme),
            Modal::Lan(state) => draw_lan_modal(frame, state, &theme),
            Modal::Nodes(state) => draw_nodes_modal(frame, state, &theme),
            Modal::Info { title, lines } => draw_info_modal(frame, title, lines, &theme),
        }
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new("openclash terminal control center")
        .style(theme.title())
        .alignment(Alignment::Center);
    frame.render_widget(title, rows[0]);

    let meta = format!(
        "{}  |  kernel={}  |  base={}",
        app.status_line(),
        app.snapshot.kernel_name,
        truncate(
            &app.snapshot.base_dir,
            rows[1].width.saturating_sub(28) as usize
        ),
    );
    let meta_widget = Paragraph::new(meta)
        .style(Style::default().fg(theme.text_dim))
        .alignment(Alignment::Center);
    frame.render_widget(meta_widget, rows[1]);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    draw_summary(frame, app, columns[0], theme);
    draw_actions_and_logs(frame, app, columns[1], theme);
}

fn draw_summary(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(area);

    let mixed_port = app
        .snapshot
        .mixed_port
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let ui_port = app
        .snapshot
        .ui_port
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let subscription = if app.snapshot.subscription_url.is_empty() {
        "-".to_string()
    } else {
        truncate(&app.snapshot.subscription_url, 30)
    };

    let status_lines = vec![
        status_line("Service", yes_no(app.snapshot.running)),
        status_line("Ready", yes_no(app.snapshot.ready)),
        status_line("PID", app.snapshot.pid.as_deref().unwrap_or("-")),
        status_line("Mixed Port", &mixed_port),
        status_line("Controller Port", &ui_port),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(status_lines))
            .block(section_block("Status", theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        sections[0],
    );

    let network_lines = vec![
        status_line("System Proxy", yes_no(app.snapshot.system_proxy_enabled)),
        status_line("TUN", yes_no(app.snapshot.tun_enabled)),
        status_line("Subscription", &subscription),
        status_line("Dashboard", "openclash web"),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(network_lines))
            .block(section_block("Daily Controls", theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        sections[1],
    );

    let access_lines = vec![
        status_line(
            "local ctrl",
            app.snapshot.local_ui.as_deref().unwrap_or("-"),
        ),
        status_line(
            "public ctrl",
            app.snapshot.public_ui.as_deref().unwrap_or("-"),
        ),
        status_line("shared ctrl", &app.snapshot.common_ui),
        status_line("raw", exists_text(app.snapshot.config_raw_exists)),
        status_line("runtime", exists_text(app.snapshot.config_runtime_exists)),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(access_lines))
            .block(section_block("Access", theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        sections[2],
    );
}

fn draw_actions_and_logs(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(13), Constraint::Min(8)])
        .split(area);

    let actions = app.actions();
    let items: Vec<ListItem> = actions
        .iter()
        .map(|action| {
            let label = action.label(&app.snapshot);
            let description = action.description(&app.snapshot);
            ListItem::new(vec![
                Line::from(Span::styled(
                    label,
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("  {}", description),
                    Style::default().fg(theme.text_dim),
                )),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(
        app.selected_action.min(actions.len().saturating_sub(1)),
    ));
    let actions_widget = List::new(items)
        .block(section_block("Actions", theme))
        .highlight_style(
            Style::default()
                .fg(theme.primary)
                .bg(theme.highlight)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(actions_widget, sections[0], &mut state);

    let mut log_lines = Vec::new();
    if app.snapshot.logs.is_empty() {
        log_lines.push(Line::from(Span::styled(
            "No kernel log yet.",
            theme.muted(),
        )));
    } else {
        for line in &app.snapshot.logs {
            log_lines.push(Line::from(Span::raw(line.clone())));
        }
    }
    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .block(section_block("Recent Logs", theme))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        sections[1],
    );
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let hint = if let Some(toast) = app.toast.as_ref() {
        Span::styled(
            toast.message.clone(),
            Style::default().fg(if toast.is_error {
                theme.error
            } else {
                theme.success
            }),
        )
    } else {
        Span::styled(
            "↑↓ select  Enter open  r refresh  ? help  q quit",
            Style::default().fg(theme.text_dim),
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(hint))
            .alignment(Alignment::Center)
            .style(Style::default().bg(theme.background)),
        area,
    );
}

fn draw_help_modal(frame: &mut Frame, theme: &Theme) {
    let area = centered_rect(58, 58, frame.area());
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(Span::styled("Keyboard", theme.title())),
        Line::from(""),
        Line::from("↑ / ↓ / j / k   Move selection"),
        Line::from("Enter           Confirm / open action"),
        Line::from("Esc             Close popup / quit"),
        Line::from("Left / Right    Switch proxy group in node picker"),
        Line::from("Space / A       Toggle LAN access in LAN popup"),
        Line::from("r               Refresh state"),
        Line::from("?               Toggle help"),
        Line::from("q / Ctrl+C      Quit TUI"),
        Line::from(""),
        Line::from(Span::styled(
            "Main tasks",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("System proxy, node switching, LAN port, and TUN are first-class actions."),
        Line::from("Subscription and Web access stay available as secondary actions."),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(modal_block("Help", theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        area,
    );
}

fn draw_confirm_modal(frame: &mut Frame, prompt: &str, theme: &Theme) {
    let area = centered_rect(46, 26, frame.area());
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            prompt,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter 确认  •  Esc 取消",
            Style::default().fg(theme.text_dim),
        )),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(modal_block("Confirm", theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        area,
    );
}

fn draw_input_modal(frame: &mut Frame, state: &InputState, theme: &Theme) {
    let area = centered_rect(72, 34, frame.area());
    frame.render_widget(Clear, area);

    let body = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    frame.render_widget(modal_block(&state.title, theme), area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(body);

    let value = Paragraph::new(state.value.as_str())
        .block(section_block("Value", theme))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.primary).bg(theme.highlight));
    frame.render_widget(value, rows[0]);

    let hint = Paragraph::new(state.hint.as_str())
        .block(section_block("Hint", theme))
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(theme.text_dim).bg(theme.surface));
    frame.render_widget(hint, rows[1]);

    let footer = Paragraph::new("Enter 保存  •  Esc 取消  •  Backspace 删除")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_dim).bg(theme.surface));
    frame.render_widget(footer, rows[2]);
}

fn draw_lan_modal(frame: &mut Frame, state: &LanInputState, theme: &Theme) {
    let area = centered_rect(58, 34, frame.area());
    frame.render_widget(Clear, area);

    let body = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    frame.render_widget(modal_block("LAN Port", theme), area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(body);

    let allow_lan = if state.allow_lan { "enabled" } else { "disabled" };
    frame.render_widget(
        Paragraph::new(format!("Allow LAN: {}", allow_lan))
            .block(section_block("LAN Access", theme))
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        rows[0],
    );

    frame.render_widget(
        Paragraph::new(state.port_value.as_str())
            .block(section_block("Mixed Port", theme))
            .style(Style::default().fg(theme.primary).bg(theme.highlight)),
        rows[1],
    );

    frame.render_widget(
        Paragraph::new("Space/A toggle LAN  Enter save  Esc cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_dim).bg(theme.surface)),
        rows[2],
    );
}

fn draw_nodes_modal(frame: &mut Frame, state: &NodePickerState, theme: &Theme) {
    let area = centered_rect(84, 76, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(modal_block("Proxy Nodes", theme), area);

    let body = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(2)])
        .split(body);

    let group = state.groups.get(state.group_index);
    let group_title = group
        .map(|group| {
            format!(
                "{} ({}/{})  current: {}",
                group.name,
                state.group_index + 1,
                state.groups.len(),
                if group.now.is_empty() { "-" } else { &group.now }
            )
        })
        .unwrap_or_else(|| "No proxy groups".to_string());
    frame.render_widget(
        Paragraph::new(group_title)
            .block(section_block("Group", theme))
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        rows[0],
    );

    let nodes = group
        .map(|group| group.nodes.as_slice())
        .unwrap_or(&[] as &[String]);
    let items: Vec<ListItem> = nodes
        .iter()
        .map(|node| {
            let marker = if group.map(|group| &group.now == node).unwrap_or(false) {
                "* "
            } else {
                "  "
            };
            ListItem::new(Line::from(format!("{}{}", marker, node)))
        })
        .collect();
    let mut list_state =
        ListState::default().with_selected(Some(state.node_index.min(nodes.len().saturating_sub(1))));
    frame.render_stateful_widget(
        List::new(items)
            .block(section_block("Nodes", theme))
            .highlight_style(
                Style::default()
                    .fg(theme.primary)
                    .bg(theme.highlight)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› "),
        rows[1],
        &mut list_state,
    );

    frame.render_widget(
        Paragraph::new("←→ group  ↑↓ node  Enter switch  Esc close")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_dim).bg(theme.surface)),
        rows[2],
    );
}

fn draw_info_modal(frame: &mut Frame, title: &str, lines: &[String], theme: &Theme) {
    let area = centered_rect(74, 36, frame.area());
    frame.render_widget(Clear, area);
    let mut content = vec![Line::from("")];
    for line in lines {
        content.push(Line::from(line.as_str()));
        content.push(Line::from(""));
    }
    content.push(Line::from(Span::styled(
        "Enter / Esc 关闭",
        Style::default().fg(theme.text_dim),
    )));
    frame.render_widget(
        Paragraph::new(Text::from(content))
            .block(modal_block(title, theme))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.surface)),
        area,
    );
}

fn section_block(title: &str, theme: &Theme) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface))
}

fn modal_block(title: &str, theme: &Theme) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.surface))
}

fn status_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label:<12}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn exists_text(value: bool) -> &'static str {
    if value {
        "exists"
    } else {
        "missing"
    }
}
