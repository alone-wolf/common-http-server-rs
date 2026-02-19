#[cfg(not(feature = "external-health"))]
fn main() {
    eprintln!(
        "This sample needs the optional reqwest client.\nRun with:\n\
         cargo run -p common-http-server --example jwt_with_client --features external-health"
    );
}

#[cfg(feature = "external-health")]
mod enabled {
    use axum::{
        Json, Router,
        extract::{Extension, State},
        http::StatusCode,
        middleware,
        routing::{get, post},
    };
    use common_http_server::auth::User;
    use common_http_server::{
        AuthConfig, AuthUser, BasicUser, JwtUtils, SharedAuthConfig, jwt_auth_middleware,
    };
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;
    use tokio::{net::TcpListener, sync::oneshot, time::Duration};

    #[derive(Debug, Deserialize, Serialize)]
    struct LoginRequest {
        username: String,
        password: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct LoginResponse {
        access_token: String,
        token_type: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct MeResponse {
        username: String,
        roles: Vec<String>,
        auth_type: String,
    }

    async fn login(
        State(config): State<SharedAuthConfig>,
        Json(payload): Json<LoginRequest>,
    ) -> Result<Json<LoginResponse>, StatusCode> {
        let basic_user = config
            .basic_users
            .iter()
            .find(|user| user.username == payload.username)
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let password_ok = basic_user
            .verify_password(&payload.password)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !password_ok {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let user = User {
            id: payload.username.clone(),
            username: payload.username,
            roles: basic_user.roles.clone(),
            permissions: vec!["profile:read".to_string()],
        };

        let token = JwtUtils::generate_token(&user, &config)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
        }))
    }

    async fn me(Extension(auth_user): Extension<AuthUser>) -> Json<MeResponse> {
        Json(MeResponse {
            username: auth_user.user.username,
            roles: auth_user.user.roles,
            auth_type: format!("{:?}", auth_user.auth_type),
        })
    }

    async fn run_client(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let login_response = client
            .post(format!("{base_url}/auth/login"))
            .json(&LoginRequest {
                username: "demo".to_string(),
                password: "demo123".to_string(),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<LoginResponse>()
            .await?;

        println!("login success, got access token");

        let me_response = client
            .get(format!("{base_url}/api/me"))
            .bearer_auth(&login_response.access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<MeResponse>()
            .await?;

        println!("protected endpoint response: {:?}", me_response);
        Ok(())
    }

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = AuthConfig::default()
            .with_jwt_secret("replace-with-your-own-very-strong-secret-32-plus-chars")
            .with_jwt_issuer("jwt-sample-service")
            .with_jwt_audience("jwt-sample-client");
        config.basic_users = vec![BasicUser::new("demo", "demo123", vec!["user"])?];
        let shared_auth = config.shared();

        let protected_api =
            Router::new()
                .route("/me", get(me))
                .layer(middleware::from_fn_with_state(
                    shared_auth.clone(),
                    jwt_auth_middleware,
                ));

        let app = Router::new()
            .route("/auth/login", post(login))
            .nest("/api", protected_api)
            .with_state(shared_auth);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let server_addr = listener.local_addr()?;
        let base_url = format!("http://{server_addr}");

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server_task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
        });

        tokio::time::sleep(Duration::from_millis(120)).await;
        let client_result = run_client(&base_url).await;

        let _ = shutdown_tx.send(());
        let server_result = server_task.await?;
        server_result?;
        client_result
    }
}

#[cfg(feature = "external-health")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enabled::run().await
}
