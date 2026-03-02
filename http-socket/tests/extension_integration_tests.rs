use std::collections::HashMap;
use std::sync::Mutex;

use http_socket::error::{AuthError, StoreError};
use http_socket::extension::auth::{AuthContext, Authenticator};
use http_socket::extension::store::{SessionSnapshot, SessionStore};
use http_socket::protocol::handshake::CapabilityContract;
use http_socket::{CapabilityMap, ProtocolVersion, SessionId, TransportKind};

#[derive(Debug)]
struct DenyAuthenticator;

impl Authenticator for DenyAuthenticator {
    fn authenticate(
        &self,
        _context: &AuthContext,
    ) -> Result<http_socket::extension::auth::AuthDecision, AuthError> {
        Err(AuthError::AccessDenied("blocked".to_string()))
    }
}

#[derive(Default)]
struct InMemoryStore {
    items: Mutex<HashMap<u64, SessionSnapshot>>,
}

impl SessionStore for InMemoryStore {
    fn save(&self, snapshot: SessionSnapshot) -> Result<(), StoreError> {
        self.items
            .lock()
            .expect("store mutex poisoned")
            .insert(snapshot.session_id.as_u64(), snapshot);
        Ok(())
    }

    fn load(&self, session_id: SessionId) -> Result<Option<SessionSnapshot>, StoreError> {
        Ok(self
            .items
            .lock()
            .expect("store mutex poisoned")
            .get(&session_id.as_u64())
            .cloned())
    }

    fn delete(&self, session_id: SessionId) -> Result<(), StoreError> {
        self.items
            .lock()
            .expect("store mutex poisoned")
            .remove(&session_id.as_u64());
        Ok(())
    }
}

#[test]
fn authenticator_can_fail_with_classified_error() {
    let auth = DenyAuthenticator;
    let context = AuthContext {
        session_id: SessionId::new(301),
        principal: "alice".to_string(),
        token: "bad".to_string(),
    };

    let err = auth
        .authenticate(&context)
        .expect_err("auth should be rejected");
    assert_eq!(err, AuthError::AccessDenied("blocked".to_string()));
}

#[test]
fn session_store_is_pluggable() {
    let store = InMemoryStore::default();
    let snapshot = SessionSnapshot {
        session_id: SessionId::new(302),
        contract: CapabilityContract {
            transport: TransportKind::Ws,
            version: ProtocolVersion::new(1),
            enabled_capabilities: CapabilityMap::new(),
        },
    };

    store.save(snapshot.clone()).expect("save should succeed");
    let loaded = store
        .load(SessionId::new(302))
        .expect("load should succeed")
        .expect("snapshot should exist");
    assert_eq!(loaded, snapshot);
}
