use crate::core::runtime_ui::actions::ActionEvent;
use crate::core::runtime_ui::state::{AppState, RuntimeTab};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiInputEvent {
    Quit,
    NextTab,
    PreviousTab,
    Up,
    Down,
    Enter,
    Escape,
    ToggleLogsAutoScroll,
    CycleLogTagFilter,
    CycleLogLevelFilter,
}

pub fn map_key_event(key: KeyEvent) -> Option<UiInputEvent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    let event = match key.code {
        KeyCode::Char('q') => UiInputEvent::Quit,
        KeyCode::Right => UiInputEvent::NextTab,
        KeyCode::Left => UiInputEvent::PreviousTab,
        KeyCode::Up => UiInputEvent::Up,
        KeyCode::Down => UiInputEvent::Down,
        KeyCode::Enter => UiInputEvent::Enter,
        KeyCode::Esc => UiInputEvent::Escape,
        KeyCode::Char('p') => UiInputEvent::ToggleLogsAutoScroll,
        KeyCode::Char('t') => UiInputEvent::CycleLogTagFilter,
        KeyCode::Char('l') => UiInputEvent::CycleLogLevelFilter,
        _ => return None,
    };

    Some(event)
}

pub fn apply_input_event(state: &mut AppState, event: UiInputEvent) -> Option<ActionEvent> {
    match event {
        UiInputEvent::Quit => {
            state.should_quit = true;
            None
        }
        UiInputEvent::NextTab => {
            state.next_tab();
            None
        }
        UiInputEvent::PreviousTab => {
            state.previous_tab();
            None
        }
        UiInputEvent::Up => {
            state.up();
            None
        }
        UiInputEvent::Down => {
            state.down();
            None
        }
        UiInputEvent::Enter => state.request_action(),
        UiInputEvent::Escape => {
            state.actions.confirmation = None;
            None
        }
        UiInputEvent::ToggleLogsAutoScroll => {
            if state.active_tab == RuntimeTab::Logs {
                state.toggle_logs_auto_scroll();
            }
            None
        }
        UiInputEvent::CycleLogTagFilter => {
            if state.active_tab == RuntimeTab::Logs {
                state.logs.cycle_tag_filter();
            }
            None
        }
        UiInputEvent::CycleLogLevelFilter => {
            if state.active_tab == RuntimeTab::Logs {
                state.logs.cycle_level_filter();
            }
            None
        }
    }
}
