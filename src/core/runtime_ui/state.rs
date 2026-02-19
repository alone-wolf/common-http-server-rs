use crate::core::runtime_ui::actions::{ACTION_ITEMS, ActionEvent, ActionKind};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTab {
    Status,
    Logs,
    Actions,
    About,
}

impl RuntimeTab {
    pub const ALL: [RuntimeTab; 4] = [
        RuntimeTab::Status,
        RuntimeTab::Logs,
        RuntimeTab::Actions,
        RuntimeTab::About,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Logs => "Logs",
            Self::Actions => "Actions",
            Self::About => "About",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Status => Self::Logs,
            Self::Logs => Self::Actions,
            Self::Actions => Self::About,
            Self::About => Self::Status,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Status => Self::About,
            Self::Logs => Self::Status,
            Self::Actions => Self::Logs,
            Self::About => Self::Actions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn next_filter(self) -> Option<Self> {
        match self {
            Self::Trace => Some(Self::Debug),
            Self::Debug => Some(Self::Info),
            Self::Info => Some(Self::Warn),
            Self::Warn => Some(Self::Error),
            Self::Error => None,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, tag: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            tag: tag.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub system_cpu_percent: f32,
    pub system_memory_percent: f32,
    pub process_cpu_percent: f32,
    pub process_memory_bytes: u64,
    pub uptime: Duration,
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub dropped_logs: u64,
}

impl Default for StatusSnapshot {
    fn default() -> Self {
        Self {
            system_cpu_percent: 0.0,
            system_memory_percent: 0.0,
            process_cpu_percent: 0.0,
            process_memory_bytes: 0,
            uptime: Duration::ZERO,
            total_requests: 0,
            success_requests: 0,
            failed_requests: 0,
            dropped_logs: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AboutInfo {
    pub app_name: String,
    pub version: String,
    pub developer: String,
    pub build_time: String,
    pub git_commit: Option<String>,
}

impl Default for AboutInfo {
    fn default() -> Self {
        Self {
            app_name: "common-http-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            developer: "unknown".to_string(),
            build_time: "unknown".to_string(),
            git_commit: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UiStateUpdate {
    Status(StatusSnapshot),
    Log(LogEntry),
    About(AboutInfo),
}

#[derive(Debug, Clone)]
pub struct LogsState {
    pub entries: VecDeque<LogEntry>,
    pub max_entries: usize,
    pub tag_filter: Option<String>,
    pub level_filter: Option<LogLevel>,
    pub paused_auto_scroll: bool,
    pub scroll_offset: usize,
    pub known_tags: BTreeMap<String, usize>,
}

impl LogsState {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
            tag_filter: None,
            level_filter: None,
            paused_auto_scroll: false,
            scroll_offset: 0,
            known_tags: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        *self.known_tags.entry(entry.tag.clone()).or_insert(0) += 1;
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries {
            if let Some(removed) = self.entries.pop_front() {
                self.decrement_tag_count(&removed.tag);
            }
        }
        if !self.paused_auto_scroll {
            self.scroll_offset = 0;
        }
    }

    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                let level_match = self.level_filter.is_none_or(|filter| entry.level == filter);
                let tag_match = self
                    .tag_filter
                    .as_ref()
                    .is_none_or(|filter| &entry.tag == filter);
                level_match && tag_match
            })
            .collect()
    }

    pub fn cycle_tag_filter(&mut self) {
        if self.known_tags.is_empty() {
            self.tag_filter = None;
            return;
        }

        let tags: Vec<&String> = self.known_tags.keys().collect();
        self.tag_filter = match &self.tag_filter {
            None => Some(tags[0].clone().to_string()),
            Some(current) => {
                if let Some(index) = tags.iter().position(|tag| *tag == current) {
                    if index + 1 < tags.len() {
                        Some(tags[index + 1].clone().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        self.scroll_offset = 0;
    }

    pub fn cycle_level_filter(&mut self) {
        self.level_filter = match self.level_filter {
            None => Some(LogLevel::Trace),
            Some(level) => level.next_filter(),
        };
        self.scroll_offset = 0;
    }

    fn decrement_tag_count(&mut self, tag: &str) {
        if let Some(count) = self.known_tags.get_mut(tag)
            && *count > 1
        {
            *count -= 1;
            return;
        }
        let _ = self.known_tags.remove(tag);
        if self.tag_filter.as_deref() == Some(tag) {
            self.tag_filter = None;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActionsState {
    pub selected: usize,
    pub confirmation: Option<ActionKind>,
    pub last_feedback: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub started_at: Instant,
    pub should_quit: bool,
    pub active_tab: RuntimeTab,
    pub status: StatusSnapshot,
    pub logs: LogsState,
    pub actions: ActionsState,
    pub about: AboutInfo,
}

impl AppState {
    pub fn new(max_logs: usize, about: AboutInfo) -> Self {
        Self {
            started_at: Instant::now(),
            should_quit: false,
            active_tab: RuntimeTab::Status,
            status: StatusSnapshot::default(),
            logs: LogsState::new(max_logs),
            actions: ActionsState::default(),
            about,
        }
    }

    pub fn apply_update(&mut self, update: UiStateUpdate) {
        match update {
            UiStateUpdate::Status(status) => {
                self.status = status;
            }
            UiStateUpdate::Log(entry) => {
                self.logs.push(entry);
            }
            UiStateUpdate::About(about) => {
                self.about = about;
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.actions.confirmation = None;
    }

    pub fn previous_tab(&mut self) {
        self.active_tab = self.active_tab.previous();
        self.actions.confirmation = None;
    }

    pub fn up(&mut self) {
        match self.active_tab {
            RuntimeTab::Logs => {
                self.logs.paused_auto_scroll = true;
                self.logs.scroll_offset = self.logs.scroll_offset.saturating_add(1);
            }
            RuntimeTab::Actions => {
                let new_selected = self.actions.selected.saturating_sub(1);
                if new_selected != self.actions.selected {
                    self.actions.confirmation = None;
                }
                self.actions.selected = new_selected;
            }
            _ => {}
        }
    }

    pub fn down(&mut self) {
        match self.active_tab {
            RuntimeTab::Logs => {
                self.logs.scroll_offset = self.logs.scroll_offset.saturating_sub(1);
            }
            RuntimeTab::Actions => {
                let max_index = ACTION_ITEMS.len().saturating_sub(1);
                let new_selected = (self.actions.selected + 1).min(max_index);
                if new_selected != self.actions.selected {
                    self.actions.confirmation = None;
                }
                self.actions.selected = new_selected;
            }
            _ => {}
        }
    }

    pub fn toggle_logs_auto_scroll(&mut self) {
        self.logs.paused_auto_scroll = !self.logs.paused_auto_scroll;
        if !self.logs.paused_auto_scroll {
            self.logs.scroll_offset = 0;
        }
    }

    pub fn request_action(&mut self) -> Option<ActionEvent> {
        if self.active_tab != RuntimeTab::Actions {
            return None;
        }

        if let Some(pending) = self.actions.confirmation.take() {
            if pending == ActionKind::ShutdownService {
                // Shutdown action should close the UI loop so the terminal
                // returns to normal immediately.
                self.should_quit = true;
            } else {
                self.actions.last_feedback = Some("Restart action event sent".to_string());
            }
            return Some(ActionEvent::new(pending));
        }

        let selected = ACTION_ITEMS
            .get(self.actions.selected)
            .copied()
            .unwrap_or(ActionKind::RestartService);
        self.actions.last_feedback = None;
        self.actions.confirmation = Some(selected);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_action_confirms_and_requests_ui_quit() {
        let mut state = AppState::new(100, AboutInfo::default());
        state.active_tab = RuntimeTab::Actions;
        state.actions.selected = 1; // ShutdownService

        assert!(state.request_action().is_none());
        assert!(!state.should_quit);

        let event = state.request_action();
        assert!(event.is_some());
        assert_eq!(
            event.expect("expected shutdown action").kind,
            ActionKind::ShutdownService
        );
        assert!(state.should_quit);
    }

    #[test]
    fn restart_action_confirms_without_quitting_ui() {
        let mut state = AppState::new(100, AboutInfo::default());
        state.active_tab = RuntimeTab::Actions;
        state.actions.selected = 0; // RestartService

        assert!(state.request_action().is_none());
        let event = state.request_action().expect("expected restart action");

        assert_eq!(event.kind, ActionKind::RestartService);
        assert!(!state.should_quit);
        assert_eq!(
            state.actions.last_feedback.as_deref(),
            Some("Restart action event sent")
        );
    }

    #[test]
    fn logs_state_removes_stale_tags_when_ring_buffer_evicts_entries() {
        let mut logs = LogsState::new(2);

        logs.push(LogEntry::new(LogLevel::Info, "auth", "a"));
        logs.push(LogEntry::new(LogLevel::Info, "core", "b"));
        logs.push(LogEntry::new(LogLevel::Info, "core", "c"));

        assert!(!logs.known_tags.contains_key("auth"));
        assert!(logs.known_tags.contains_key("core"));
    }

    #[test]
    fn logs_state_clears_tag_filter_if_filtered_tag_is_evicted() {
        let mut logs = LogsState::new(1);
        logs.push(LogEntry::new(LogLevel::Info, "auth", "a"));
        logs.tag_filter = Some("auth".to_string());

        logs.push(LogEntry::new(LogLevel::Info, "core", "b"));

        assert_eq!(logs.tag_filter, None);
    }
}
