#[cfg(feature = "axum")]
mod enabled {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Extension, Path, State};
    use axum::http::{Method, StatusCode};
    use axum::routing::{get, post};
    use http_socket::core::session::attach::attach_transport;
    use http_socket::core::session::core::SessionCore;
    use http_socket::core::session::resume::AllowAllResumeTokenValidator;
    use http_socket::core::session::swap::swap_transport;
    use http_socket::error::ExtensionError;
    use http_socket::extension::middleware::MiddlewareHook;
    use http_socket::protocol::handshake::{
        CapabilityContract, ClientAdvertise, ServerPreferencePolicy, negotiate,
    };
    use http_socket::transport::handle::MockTransportHandle;
    use http_socket::{
        AxumMiddlewareError, AxumRequestContext, CapabilityKey, CapabilityMap,
        CapabilityRequirement, CapabilityRequirementMap, CapabilityValue, HttpSocketAxumLayer,
        ProtocolVersion, RouterHttpSocketExt, ServerBuilder, SessionBuilder, TransportKind,
    };

    #[derive(Debug)]
    struct SocketMethodGuard;

    impl MiddlewareHook<AxumRequestContext> for SocketMethodGuard {
        fn before(&self, request: &AxumRequestContext) -> Result<(), ExtensionError> {
            if request.path.starts_with("/socket/connect") && request.method != Method::POST {
                return Err(ExtensionError::MiddlewareRejected(
                    "connect only accepts POST".to_string(),
                ));
            }
            if request.path.starts_with("/socket/send") && request.method != Method::POST {
                return Err(ExtensionError::MiddlewareRejected(
                    "send only accepts POST".to_string(),
                ));
            }
            if request.path.starts_with("/socket/swap") && request.method != Method::POST {
                return Err(ExtensionError::MiddlewareRejected(
                    "swap only accepts POST".to_string(),
                ));
            }
            if request.path.starts_with("/socket/session") && request.method != Method::GET {
                return Err(ExtensionError::MiddlewareRejected(
                    "session status only accepts GET".to_string(),
                ));
            }
            if request.path.starts_with("/socket/clients") && request.method != Method::GET {
                return Err(ExtensionError::MiddlewareRejected(
                    "clients count only accepts GET".to_string(),
                ));
            }
            Ok(())
        }
    }

    #[derive(Debug)]
    struct SessionRecord {
        session: SessionCore,
        messages: Vec<String>,
    }

    #[derive(Debug)]
    struct AppState {
        supported_versions: Vec<ProtocolVersion>,
        supported_transports: Vec<TransportKind>,
        sessions: Mutex<HashMap<u64, SessionRecord>>,
        next_transport_id: AtomicU64,
    }

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut server_caps = CapabilityMap::new();
        server_caps.insert(CapabilityKey::new("resume"), CapabilityValue::Bool(true));
        server_caps.insert(
            CapabilityKey::new("codec.json"),
            CapabilityValue::Bool(true),
        );
        server_caps.insert(
            CapabilityKey::new("compression"),
            CapabilityValue::Text("gzip".to_string()),
        );

        let server = ServerBuilder::new()
            .supported_versions(vec![ProtocolVersion::new(1)])
            .supported_transports(vec![
                TransportKind::Ws,
                TransportKind::Sse,
                TransportKind::Poll,
            ])
            .capabilities(server_caps)
            .build()?;

        let state = Arc::new(AppState {
            supported_versions: server.supported_versions,
            supported_transports: server.supported_transports,
            sessions: Mutex::new(HashMap::new()),
            next_transport_id: AtomicU64::new(1),
        });

        let layer = HttpSocketAxumLayer::new().with_hook(Arc::new(SocketMethodGuard));
        let app = Router::new()
            .route("/health", get(health))
            .route("/socket/clients", get(clients_count))
            .route("/socket/connect", post(connect))
            .route("/socket/send/{session_id}", post(send_message))
            .route("/socket/swap/{session_id}", post(swap_session_transport))
            .route("/socket/session/{session_id}", get(session_status))
            .with_state(state)
            .with_http_socket(layer);

        let bind = std::env::var("HTTP_SOCKET_BIND")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "127.0.0.1:4000".to_string());
        let listener = tokio::net::TcpListener::bind(&bind).await?;

        println!("http-socket axum server running at http://{bind}");
        println!("health: GET /health");
        println!("connected clients: GET /socket/clients");
        println!("connect payload format:");
        println!(
            "  transports=ws,poll;versions=1;caps=resume:true,codec.json:true;required=resume:true"
        );

        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn health(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
        let connected = match state.sessions.lock() {
            Ok(sessions) => sessions.len(),
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };

        (StatusCode::OK, format!("ok;connected_clients={connected}"))
    }

    async fn clients_count(
        State(state): State<Arc<AppState>>,
        error: Option<Extension<AxumMiddlewareError>>,
    ) -> (StatusCode, String) {
        if let Some(Extension(err)) = error {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                format!("middleware rejected: {}", err.reason),
            );
        }

        let connected = match state.sessions.lock() {
            Ok(sessions) => sessions.len(),
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };

        (StatusCode::OK, format!("ok;connected_clients={connected}"))
    }

    async fn connect(
        State(state): State<Arc<AppState>>,
        error: Option<Extension<AxumMiddlewareError>>,
        body: String,
    ) -> (StatusCode, String) {
        if let Some(Extension(err)) = error {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                format!("middleware rejected: {}", err.reason),
            );
        }

        let advertise = match parse_advertise(&body) {
            Ok(value) => value,
            Err(reason) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("invalid connect payload: {reason}"),
                );
            }
        };

        let policy = ServerPreferencePolicy::new(
            state.supported_versions.clone(),
            state.supported_transports.clone(),
        )
        .with_required_capability(CapabilityKey::new("resume"));

        let selected = match negotiate(&advertise, &policy) {
            Ok(value) => value,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("negotiation failed: {err}"),
                );
            }
        };
        let contract = CapabilityContract::from(selected.clone());

        let transport_id = state.next_transport_id.fetch_add(1, Ordering::Relaxed);
        let mut session = SessionBuilder::new().build();
        if let Err(err) = attach_transport(
            &mut session,
            contract,
            Arc::new(MockTransportHandle::new(transport_id, selected.transport)),
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("attach transport failed: {err}"),
            );
        }

        let session_id = session.session_id().as_u64();
        let mut sessions = match state.sessions.lock() {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };
        sessions.insert(
            session_id,
            SessionRecord {
                session,
                messages: Vec::new(),
            },
        );
        let connected_clients = sessions.len();
        println!("new client connected: session_id={session_id}, total={connected_clients}");

        (
            StatusCode::OK,
            format!(
                "ok;session_id={session_id};transport={};version={};caps={};connected_clients={connected_clients}",
                selected.transport,
                selected.version,
                format_capability_keys(&selected.enabled_capabilities),
            ),
        )
    }

    async fn send_message(
        Path(session_id): Path<u64>,
        State(state): State<Arc<AppState>>,
        error: Option<Extension<AxumMiddlewareError>>,
        body: String,
    ) -> (StatusCode, String) {
        if let Some(Extension(err)) = error {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                format!("middleware rejected: {}", err.reason),
            );
        }
        if body.trim().is_empty() {
            return (StatusCode::BAD_REQUEST, "empty payload".to_string());
        }

        let mut sessions = match state.sessions.lock() {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };
        let connected_clients = sessions.len();
        let record = match sessions.get_mut(&session_id) {
            Some(value) => value,
            None => return (StatusCode::NOT_FOUND, "session not found".to_string()),
        };

        record.messages.push(body.clone());
        record.session.push_outbound(body.into_bytes());

        (
            StatusCode::OK,
            format!(
                "ok;session_id={session_id};stored={};outbound={};state={:?};connected_clients={connected_clients}",
                record.messages.len(),
                record.session.outbound_len(),
                record.session.state(),
            ),
        )
    }

    async fn swap_session_transport(
        Path(session_id): Path<u64>,
        State(state): State<Arc<AppState>>,
        error: Option<Extension<AxumMiddlewareError>>,
        body: String,
    ) -> (StatusCode, String) {
        if let Some(Extension(err)) = error {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                format!("middleware rejected: {}", err.reason),
            );
        }

        let resume_token = body.trim();
        if resume_token.is_empty() {
            return (StatusCode::BAD_REQUEST, "resume token is empty".to_string());
        }

        let mut sessions = match state.sessions.lock() {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };
        let connected_clients = sessions.len();
        let record = match sessions.get_mut(&session_id) {
            Some(value) => value,
            None => return (StatusCode::NOT_FOUND, "session not found".to_string()),
        };

        let contract = match record.session.contract() {
            Some(value) => value.clone(),
            None => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session missing capability contract".to_string(),
                );
            }
        };

        let transport_id = state.next_transport_id.fetch_add(1, Ordering::Relaxed);
        if let Err(err) = swap_transport(
            &mut record.session,
            contract.clone(),
            Arc::new(MockTransportHandle::new(transport_id, contract.transport)),
            resume_token,
            &AllowAllResumeTokenValidator,
        ) {
            return (
                StatusCode::BAD_REQUEST,
                format!("swap transport failed: {err}"),
            );
        }

        (
            StatusCode::OK,
            format!(
                "ok;session_id={session_id};transport_id={transport_id};transport={};connected_clients={connected_clients}",
                contract.transport,
            ),
        )
    }

    async fn session_status(
        Path(session_id): Path<u64>,
        State(state): State<Arc<AppState>>,
        error: Option<Extension<AxumMiddlewareError>>,
    ) -> (StatusCode, String) {
        if let Some(Extension(err)) = error {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                format!("middleware rejected: {}", err.reason),
            );
        }

        let sessions = match state.sessions.lock() {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session store lock poisoned".to_string(),
                );
            }
        };
        let connected_clients = sessions.len();
        let record = match sessions.get(&session_id) {
            Some(value) => value,
            None => return (StatusCode::NOT_FOUND, "session not found".to_string()),
        };

        let (transport, version, caps) = match record.session.contract() {
            Some(contract) => (
                contract.transport.to_string(),
                contract.version.to_string(),
                format_capability_keys(&contract.enabled_capabilities),
            ),
            None => ("none".to_string(), "none".to_string(), String::new()),
        };
        let transport_id = record
            .session
            .transport()
            .map(|transport| transport.id())
            .unwrap_or(0);

        (
            StatusCode::OK,
            format!(
                "ok;session_id={session_id};state={:?};transport={transport};transport_id={transport_id};version={version};caps={caps};stored={};outbound={};connected_clients={connected_clients}",
                record.session.state(),
                record.messages.len(),
                record.session.outbound_len(),
            ),
        )
    }

    fn parse_advertise(raw: &str) -> Result<ClientAdvertise, String> {
        let mut transports = None;
        let mut versions = None;
        let mut capabilities = CapabilityMap::new();
        let mut required = CapabilityRequirementMap::new();

        for segment in raw.split(';') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            let (key, value) = segment
                .split_once('=')
                .ok_or_else(|| format!("invalid segment `{segment}`"))?;
            let key = key.trim();
            let value = value.trim();

            match key {
                "transports" => transports = Some(parse_transports(value)?),
                "versions" => versions = Some(parse_versions(value)?),
                "caps" => capabilities = parse_capabilities(value)?,
                "required" => required = parse_required_capabilities(value),
                unknown => return Err(format!("unsupported key `{unknown}`")),
            }
        }

        let transports = transports.ok_or_else(|| "missing transports".to_string())?;
        let versions = versions.ok_or_else(|| "missing versions".to_string())?;
        Ok(ClientAdvertise::new(
            transports,
            versions,
            capabilities,
            required,
        ))
    }

    fn parse_transports(raw: &str) -> Result<Vec<TransportKind>, String> {
        let mut values = Vec::new();
        for item in raw.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            values.push(parse_transport(item)?);
        }
        if values.is_empty() {
            return Err("transports is empty".to_string());
        }
        Ok(values)
    }

    fn parse_transport(raw: &str) -> Result<TransportKind, String> {
        match raw {
            "ws" => Ok(TransportKind::Ws),
            "sse" => Ok(TransportKind::Sse),
            "poll" => Ok(TransportKind::Poll),
            _ => Err(format!("unsupported transport `{raw}`")),
        }
    }

    fn parse_versions(raw: &str) -> Result<Vec<ProtocolVersion>, String> {
        let mut values = Vec::new();
        for item in raw.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let value = item
                .parse::<u16>()
                .map_err(|_| format!("invalid version `{item}`"))?;
            values.push(ProtocolVersion::new(value));
        }
        if values.is_empty() {
            return Err("versions is empty".to_string());
        }
        Ok(values)
    }

    fn parse_capabilities(raw: &str) -> Result<CapabilityMap, String> {
        let mut values = CapabilityMap::new();
        if raw.trim().is_empty() {
            return Ok(values);
        }

        for item in raw.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }

            let (key, value) = item
                .split_once(':')
                .ok_or_else(|| format!("invalid capability `{item}`"))?;
            values.insert(
                CapabilityKey::new(key.trim()),
                parse_capability_value(value.trim()),
            );
        }

        Ok(values)
    }

    fn parse_capability_value(raw: &str) -> CapabilityValue {
        if raw.eq_ignore_ascii_case("true") {
            return CapabilityValue::Bool(true);
        }
        if raw.eq_ignore_ascii_case("false") {
            return CapabilityValue::Bool(false);
        }
        if let Ok(number) = raw.parse::<i64>() {
            return CapabilityValue::Number(number);
        }
        CapabilityValue::Text(raw.to_string())
    }

    fn parse_required_capabilities(raw: &str) -> CapabilityRequirementMap {
        let mut values = CapabilityRequirementMap::new();
        for item in raw.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }

            if let Some((key, value)) = item.split_once(':') {
                values.insert(
                    CapabilityKey::new(key.trim()),
                    parse_capability_requirement(value.trim()),
                );
                continue;
            }
            values.insert(CapabilityKey::new(item), CapabilityRequirement::BoolTrue);
        }
        values
    }

    fn parse_capability_requirement(raw: &str) -> CapabilityRequirement {
        if raw.eq_ignore_ascii_case("present") {
            return CapabilityRequirement::Present;
        }
        if raw.eq_ignore_ascii_case("true") {
            return CapabilityRequirement::BoolTrue;
        }
        CapabilityRequirement::Equals(parse_capability_value(raw))
    }

    fn format_capability_keys(capabilities: &CapabilityMap) -> String {
        capabilities
            .keys()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[cfg(feature = "axum")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enabled::run().await
}

#[cfg(not(feature = "axum"))]
fn main() {
    println!("axum_http_socket_server requires `axum` feature");
}
