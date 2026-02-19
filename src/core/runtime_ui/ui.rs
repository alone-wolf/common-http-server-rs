use crate::core::runtime_ui::actions::ACTION_ITEMS;
use crate::core::runtime_ui::app::RuntimeUiConfig;
use crate::core::runtime_ui::state::{AppState, RuntimeTab};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap,
};

pub fn render(frame: &mut Frame, state: &AppState, config: &RuntimeUiConfig) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_tabs(frame, areas[0], state);
    match state.active_tab {
        RuntimeTab::Status => render_status_tab(frame, areas[1], state),
        RuntimeTab::Logs => render_logs_tab(frame, areas[1], state),
        RuntimeTab::Actions => render_actions_tab(frame, areas[1], state),
        RuntimeTab::About => render_about_tab(frame, areas[1], state),
    }
    render_footer(frame, areas[2], state, config);
}

fn render_tabs(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected = RuntimeTab::ALL
        .iter()
        .position(|tab| *tab == state.active_tab)
        .unwrap_or(0);
    let titles: Vec<Line<'_>> = RuntimeTab::ALL
        .iter()
        .map(|tab| Line::from(format!(" {} ", tab.title())))
        .collect();

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::ALL).title("Runtime UI"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_status_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let status = &state.status;
    let uptime = if status.uptime.is_zero() {
        state.started_at.elapsed()
    } else {
        status.uptime
    };

    let rows = vec![
        Row::new(vec![
            Cell::from("System CPU"),
            Cell::from(format!("{:.2}%", status.system_cpu_percent)),
        ]),
        Row::new(vec![
            Cell::from("System Memory"),
            Cell::from(format!("{:.2}%", status.system_memory_percent)),
        ]),
        Row::new(vec![
            Cell::from("Process CPU"),
            Cell::from(format!("{:.2}%", status.process_cpu_percent)),
        ]),
        Row::new(vec![
            Cell::from("Process Memory"),
            Cell::from(format!(
                "{:.2} MiB",
                bytes_to_mib(status.process_memory_bytes)
            )),
        ]),
        Row::new(vec![
            Cell::from("Uptime"),
            Cell::from(format!("{:.1}s", uptime.as_secs_f64())),
        ]),
        Row::new(vec![
            Cell::from("Total Requests"),
            Cell::from(status.total_requests.to_string()),
        ]),
        Row::new(vec![
            Cell::from("Success Requests"),
            Cell::from(status.success_requests.to_string()),
        ]),
        Row::new(vec![
            Cell::from("Failed Requests"),
            Cell::from(status.failed_requests.to_string()),
        ]),
        Row::new(vec![
            Cell::from("Dropped Logs"),
            Cell::from(status.dropped_logs.to_string()),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(22), Constraint::Min(10)])
        .header(
            Row::new(vec!["Metric", "Value"]).style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Service Status")
                .borders(Borders::ALL),
        );

    frame.render_widget(table, area);
}

fn render_logs_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let tag_filter = state.logs.tag_filter.as_deref().unwrap_or("ALL");
    let level_filter = state
        .logs
        .level_filter
        .map(|level| level.to_string())
        .unwrap_or_else(|| "ALL".to_string());
    let auto_scroll_status = if state.logs.paused_auto_scroll {
        "PAUSED"
    } else {
        "RUNNING"
    };

    let meta = Paragraph::new(format!(
        "Tag: {} | Level: {} | AutoScroll: {} | Keys: ↑/↓ scroll, p pause, t tag, l level",
        tag_filter, level_filter, auto_scroll_status
    ))
    .block(Block::default().borders(Borders::ALL).title("Filters"));
    frame.render_widget(meta, chunks[0]);

    let filtered = state.logs.filtered_entries();
    let list_height = chunks[1].height.saturating_sub(2) as usize;
    let total = filtered.len();
    let base_offset = if state.logs.paused_auto_scroll {
        state.logs.scroll_offset
    } else {
        0
    };
    let start = total.saturating_sub(list_height.saturating_add(base_offset));
    let end = (start + list_height).min(total);

    let items: Vec<ListItem<'_>> = if start < end {
        filtered[start..end]
            .iter()
            .map(|entry| {
                ListItem::new(format!(
                    "{} [{}] [{}] {}",
                    entry.timestamp.format("%H:%M:%S"),
                    entry.level,
                    entry.tag,
                    entry.message
                ))
            })
            .collect()
    } else {
        vec![ListItem::new("No logs yet")]
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Logs"));
    frame.render_widget(list, chunks[1]);
}

fn render_actions_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let items: Vec<ListItem<'_>> = ACTION_ITEMS
        .iter()
        .map(|action| ListItem::new(Line::from(action.label())))
        .collect();
    let mut list_state = ListState::default();
    list_state.select(Some(state.actions.selected));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ")
        .block(
            Block::default()
                .title("Actions (Use ↑/↓ and Enter)")
                .borders(Borders::ALL),
        );
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let prompt = if let Some(action) = state.actions.confirmation {
        format!(
            "Confirm {} ? Press Enter again. (Esc to cancel)",
            action.label()
        )
    } else if let Some(feedback) = &state.actions.last_feedback {
        feedback.clone()
    } else {
        "Select an action and press Enter".to_string()
    };

    let prompt_widget =
        Paragraph::new(prompt).block(Block::default().title("Confirmation").borders(Borders::ALL));
    frame.render_widget(prompt_widget, chunks[1]);
}

fn render_about_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let about = &state.about;
    let lines = vec![
        Line::from(vec![
            Span::styled("App: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&about.app_name),
        ]),
        Line::from(vec![
            Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&about.version),
        ]),
        Line::from(vec![
            Span::styled("Developer: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&about.developer),
        ]),
        Line::from(vec![
            Span::styled(
                "Build Time: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(&about.build_time),
        ]),
        Line::from(vec![
            Span::styled(
                "Git Commit: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(about.git_commit.as_deref().unwrap_or("N/A")),
        ]),
    ];

    let about_widget = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().title("About").borders(Borders::ALL));
    frame.render_widget(about_widget, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState, config: &RuntimeUiConfig) {
    let hints = match state.active_tab {
        RuntimeTab::Status => "←/→ switch tab | q quit ui",
        RuntimeTab::Logs => "←/→ switch tab | ↑/↓ scroll | p pause | t tag | l level | q quit ui",
        RuntimeTab::Actions => "←/→ switch tab | ↑/↓ select | Enter confirm | q quit ui",
        RuntimeTab::About => "←/→ switch tab | q quit ui",
    };
    let line = Line::from(vec![
        Span::raw(format!("{} | ", config.title)),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn bytes_to_mib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}
