use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransportKind {
    Ws,
    Sse,
    Poll,
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TransportKind::Ws => "ws",
            TransportKind::Sse => "sse",
            TransportKind::Poll => "poll",
        };

        write!(f, "{}", value)
    }
}
