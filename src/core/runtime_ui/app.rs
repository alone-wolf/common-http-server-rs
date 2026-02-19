use crate::core::logging::enter_ui_log_mode;
use crate::core::runtime_ui::actions::ActionEvent;
use crate::core::runtime_ui::event::{apply_input_event, map_key_event};
use crate::core::runtime_ui::state::{
    AboutInfo, AppState, LogEntry, LogLevel, StatusSnapshot, UiStateUpdate,
};
use crate::core::runtime_ui::ui;
use crate::monitoring::MonitoringState;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{Event as CEvent, EventStream};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RuntimeUiConfig {
    pub enabled: bool,
    pub title: String,
    pub tick_rate: Duration,
    pub max_log_entries: usize,
    pub update_channel_capacity: usize,
}

impl Default for RuntimeUiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            title: "common-http-server".to_string(),
            tick_rate: Duration::from_millis(500),
            max_log_entries: 1000,
            update_channel_capacity: 1024,
        }
    }
}

impl RuntimeUiConfig {
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn tick_rate(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn max_log_entries(mut self, max_log_entries: usize) -> Self {
        self.max_log_entries = max_log_entries.max(10);
        self
    }

    pub fn update_channel_capacity(mut self, update_channel_capacity: usize) -> Self {
        self.update_channel_capacity = update_channel_capacity.max(1);
        self
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeUiHandle {
    updates_tx: mpsc::Sender<UiStateUpdate>,
    pending_status: Arc<Mutex<Option<StatusSnapshot>>>,
    dropped_logs: Arc<AtomicU64>,
}

impl RuntimeUiHandle {
    pub fn send_update(&self, update: UiStateUpdate) -> Result<(), String> {
        match update {
            UiStateUpdate::Status(status) => self.send_status(status),
            UiStateUpdate::Log(entry) => self.send_log(entry),
            UiStateUpdate::About(about) => self.send_about(about),
        }
    }

    pub fn send_status(&self, status: StatusSnapshot) -> Result<(), String> {
        if self.updates_tx.is_closed() {
            return Err("runtime ui update channel is closed".to_string());
        }

        match self.pending_status.lock() {
            Ok(mut guard) => {
                *guard = Some(status);
                Ok(())
            }
            Err(_) => Err("runtime ui pending status lock is poisoned".to_string()),
        }
    }

    pub fn send_log(&self, entry: LogEntry) -> Result<(), String> {
        self.try_send(UiStateUpdate::Log(entry), true)
    }

    pub fn send_about(&self, about: AboutInfo) -> Result<(), String> {
        self.try_send(UiStateUpdate::About(about), false)
    }

    pub fn send_log_with_level(
        &self,
        level: LogLevel,
        tag: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<(), String> {
        self.send_log(LogEntry::new(level, tag, message))
    }

    pub fn send_info_log(
        &self,
        tag: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<(), String> {
        self.send_log_with_level(LogLevel::Info, tag, message)
    }

    fn try_send(&self, update: UiStateUpdate, drop_when_full: bool) -> Result<(), String> {
        match self.updates_tx.try_send(update) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) if drop_when_full => {
                self.dropped_logs.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Full(_)) => Err("runtime ui update channel is full".to_string()),
            Err(TrySendError::Closed(_)) => Err("runtime ui update channel is closed".to_string()),
        }
    }

    pub fn dropped_log_count(&self) -> u64 {
        self.dropped_logs.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    fn new_for_test(capacity: usize) -> (Self, mpsc::Receiver<UiStateUpdate>) {
        let (updates_tx, updates_rx) = mpsc::channel(capacity.max(1));
        (
            Self {
                updates_tx,
                pending_status: Arc::new(Mutex::new(None)),
                dropped_logs: Arc::new(AtomicU64::new(0)),
            },
            updates_rx,
        )
    }
}

#[derive(Debug)]
pub struct RuntimeUiActionStream {
    actions_rx: mpsc::UnboundedReceiver<ActionEvent>,
}

impl RuntimeUiActionStream {
    pub async fn recv(&mut self) -> Option<ActionEvent> {
        self.actions_rx.recv().await
    }
}

#[derive(Debug)]
pub struct RuntimeUiRuntime {
    pub handle: RuntimeUiHandle,
    pub actions: RuntimeUiActionStream,
    pub task: Option<JoinHandle<()>>,
}

pub type RuntimeUiActionHandler =
    Arc<dyn Fn(ActionEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct RuntimeUiServiceConfig {
    pub ui_config: RuntimeUiConfig,
    pub about: AboutInfo,
    pub status_tick_rate: Duration,
    pub action_handler: Option<RuntimeUiActionHandler>,
}

impl Default for RuntimeUiServiceConfig {
    fn default() -> Self {
        Self {
            ui_config: RuntimeUiConfig::default(),
            about: AboutInfo::default(),
            status_tick_rate: Duration::from_millis(500),
            action_handler: None,
        }
    }
}

impl RuntimeUiServiceConfig {
    pub fn with_ui_config(mut self, ui_config: RuntimeUiConfig) -> Self {
        self.ui_config = ui_config;
        self
    }

    pub fn with_about(mut self, about: AboutInfo) -> Self {
        self.about = about;
        self
    }

    pub fn with_status_tick_rate(mut self, status_tick_rate: Duration) -> Self {
        self.status_tick_rate = status_tick_rate;
        self
    }

    pub fn with_action_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(ActionEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.action_handler = Some(Arc::new(move |event| Box::pin(handler(event))));
        self
    }
}

#[derive(Debug)]
pub struct RuntimeUiService {
    pub handle: RuntimeUiHandle,
    pub actions: Option<RuntimeUiActionStream>,
    pub ui_task: Option<JoinHandle<()>>,
    pub status_collector_task: Option<JoinHandle<()>>,
    pub action_dispatch_task: Option<JoinHandle<()>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeUiError {
    #[error("terminal IO failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn spawn_runtime_ui(config: RuntimeUiConfig, about: AboutInfo) -> RuntimeUiRuntime {
    let (updates_tx, updates_rx) = mpsc::channel(config.update_channel_capacity.max(1));
    let (actions_tx, actions_rx) = mpsc::unbounded_channel();
    let pending_status = Arc::new(Mutex::new(None));
    let dropped_logs = Arc::new(AtomicU64::new(0));
    let handle = RuntimeUiHandle {
        updates_tx,
        pending_status: pending_status.clone(),
        dropped_logs,
    };
    let actions = RuntimeUiActionStream { actions_rx };

    if !config.enabled {
        return RuntimeUiRuntime {
            handle,
            actions,
            task: None,
        };
    }

    let ui_log_handle = handle.clone();
    let task = tokio::spawn(async move {
        let _ui_log_mode_guard = {
            let ui_handle = ui_log_handle;
            enter_ui_log_mode(Arc::new(move |level, target, message| {
                let _ = ui_handle.send_log_with_level(level.into(), target, message);
            }))
        };
        info!("runtime terminal UI started");
        if let Err(err) = run_ui_loop(config, about, updates_rx, actions_tx, pending_status).await {
            warn!(error = %err, "runtime terminal UI stopped with error");
        }
        info!("runtime terminal UI stopped");
    });

    RuntimeUiRuntime {
        handle,
        actions,
        task: Some(task),
    }
}

pub fn start_terminal_ui_with_monitoring(
    monitoring: MonitoringState,
    config: RuntimeUiServiceConfig,
) -> RuntimeUiService {
    let runtime = spawn_runtime_ui(config.ui_config.clone(), config.about.clone());
    let _ = runtime.handle.send_about(config.about);

    let status_collector_task = if config.ui_config.enabled {
        let handle = runtime.handle.clone();
        Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(config.status_tick_rate);
            let mut system = System::new_all();
            let current_pid = sysinfo::get_current_pid().ok();

            loop {
                ticker.tick().await;
                system.refresh_cpu_all();
                system.refresh_memory();
                if let Some(pid) = current_pid {
                    let targets = [pid];
                    let _ = system.refresh_processes_specifics(
                        ProcessesToUpdate::Some(&targets),
                        true,
                        ProcessRefreshKind::nothing().with_cpu().with_memory(),
                    );
                }

                let total_memory = system.total_memory();
                let used_memory = system.used_memory();
                let system_memory_percent = if total_memory == 0 {
                    0.0
                } else {
                    (used_memory as f32 / total_memory as f32) * 100.0
                };

                let (process_cpu_percent, process_memory_bytes) = current_pid
                    .and_then(|pid| system.process(pid))
                    .map(|process| (process.cpu_usage(), process.memory()))
                    .unwrap_or((0.0, 0));

                let (total_requests, failed_requests, success_requests, uptime) = {
                    let stats = monitoring.stats.read().await;
                    (
                        stats.total_requests(),
                        stats.error_requests(),
                        stats.success_requests(),
                        stats.uptime(),
                    )
                };

                let snapshot = StatusSnapshot {
                    system_cpu_percent: system.global_cpu_usage(),
                    system_memory_percent,
                    process_cpu_percent,
                    process_memory_bytes,
                    uptime,
                    total_requests,
                    success_requests,
                    failed_requests,
                    dropped_logs: handle.dropped_log_count(),
                };

                if handle.send_status(snapshot).is_err() {
                    break;
                }
            }
        }))
    } else {
        None
    };

    let mut actions = Some(runtime.actions);
    let action_dispatch_task = if config.ui_config.enabled {
        if let Some(handler) = config.action_handler {
            let mut stream = actions.take().expect("actions stream must exist");
            Some(tokio::spawn(async move {
                while let Some(action) = stream.recv().await {
                    (handler)(action).await;
                }
            }))
        } else {
            None
        }
    } else {
        None
    };

    RuntimeUiService {
        handle: runtime.handle,
        actions,
        ui_task: runtime.task,
        status_collector_task,
        action_dispatch_task,
    }
}

async fn run_ui_loop(
    config: RuntimeUiConfig,
    about: AboutInfo,
    mut updates_rx: mpsc::Receiver<UiStateUpdate>,
    actions_tx: mpsc::UnboundedSender<ActionEvent>,
    pending_status: Arc<Mutex<Option<StatusSnapshot>>>,
) -> Result<(), RuntimeUiError> {
    let _terminal_guard = TerminalSession::enter()?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = AppState::new(config.max_log_entries, about);
    let mut key_events = EventStream::new();
    let mut ticker = tokio::time::interval(config.tick_rate);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;

    let mut redraw = true;
    let mut updates_open = true;

    loop {
        if let Some(status) = take_pending_status(&pending_status) {
            state.apply_update(UiStateUpdate::Status(status));
            redraw = true;
        }

        if redraw {
            terminal.draw(|frame| ui::render(frame, &state, &config))?;
            redraw = false;
        }

        tokio::select! {
            _ = ticker.tick() => {
                if let Some(status) = take_pending_status(&pending_status) {
                    state.apply_update(UiStateUpdate::Status(status));
                }
                redraw = true;
            }
            maybe_event = key_events.next() => {
                match maybe_event {
                    Some(Ok(CEvent::Key(key))) => {
                        if let Some(event) = map_key_event(key) {
                            if let Some(action) = apply_input_event(&mut state, event) {
                                let _ = actions_tx.send(action);
                            }
                            redraw = true;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(RuntimeUiError::Io(err)),
                    None => break,
                }
            }
            maybe_update = updates_rx.recv(), if updates_open => {
                match maybe_update {
                    Some(update) => {
                        state.apply_update(update);
                        redraw = true;
                    }
                    None => {
                        updates_open = false;
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    terminal.clear()?;
    Ok(())
}

fn take_pending_status(
    pending_status: &Arc<Mutex<Option<StatusSnapshot>>>,
) -> Option<StatusSnapshot> {
    match pending_status.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    }
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> Result<Self, std::io::Error> {
        enable_raw_mode()?;
        if let Err(err) = crossterm::execute!(std::io::stdout(), EnterAlternateScreen, Hide) {
            // Best-effort rollback so a partial init does not leave the terminal
            // in raw mode after startup errors.
            let _ = crossterm::execute!(std::io::stdout(), Show, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dropped_log_counter_increments_when_queue_is_full() {
        let (handle, mut updates_rx) = RuntimeUiHandle::new_for_test(1);

        handle.send_about(AboutInfo::default()).expect("fill queue");
        handle
            .send_info_log("runtime", "first dropped")
            .expect("drop #1");
        handle
            .send_info_log("runtime", "second dropped")
            .expect("drop #2");

        assert_eq!(handle.dropped_log_count(), 2);
        assert!(updates_rx.recv().await.is_some());
    }
}
