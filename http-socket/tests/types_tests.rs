use http_socket::{ProtocolVersion, SessionId, TransportKind};

#[test]
fn session_id_next_is_monotonic() {
    let first = SessionId::next();
    let second = SessionId::next();
    assert!(second.as_u64() > first.as_u64());
}

#[test]
fn protocol_version_orders_correctly() {
    let v1 = ProtocolVersion::new(1);
    let v2 = ProtocolVersion::new(2);
    assert!(v2 > v1);
    assert_eq!(v1.to_string(), "v1");
}

#[test]
fn transport_kind_display_is_stable() {
    assert_eq!(TransportKind::Ws.to_string(), "ws");
    assert_eq!(TransportKind::Sse.to_string(), "sse");
    assert_eq!(TransportKind::Poll.to_string(), "poll");
}
