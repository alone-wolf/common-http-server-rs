use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_ID_ALLOCATOR: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(u64);

impl SessionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next() -> Self {
        let value = SESSION_ID_ALLOCATOR.fetch_add(1, Ordering::Relaxed);
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
