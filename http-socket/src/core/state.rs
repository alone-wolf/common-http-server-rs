#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Connecting,
    Negotiating,
    Active,
    Resuming,
    Draining,
    Closed,
}

impl SessionState {
    pub fn can_transition_to(self, next: SessionState) -> bool {
        use SessionState::*;

        matches!(
            (self, next),
            (Connecting, Negotiating)
                | (Negotiating, Active)
                | (Active, Resuming)
                | (Resuming, Active)
                | (Active, Draining)
                | (Negotiating, Closed)
                | (Connecting, Closed)
                | (Resuming, Closed)
                | (Draining, Closed)
                | (Active, Closed)
        )
    }
}
