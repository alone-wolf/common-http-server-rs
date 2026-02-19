use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    RestartService,
    ShutdownService,
}

impl ActionKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::RestartService => "Restart Service",
            Self::ShutdownService => "Shutdown Service",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionEvent {
    pub kind: ActionKind,
    pub requested_at: DateTime<Utc>,
}

impl ActionEvent {
    pub fn new(kind: ActionKind) -> Self {
        Self {
            kind,
            requested_at: Utc::now(),
        }
    }
}

pub const ACTION_ITEMS: [ActionKind; 2] = [ActionKind::RestartService, ActionKind::ShutdownService];
