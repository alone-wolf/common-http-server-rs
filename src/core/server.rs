//! Server bootstrap and runtime configuration.
//!
//! The startup flow is intentionally single-path:
//! `AppBuilder -> Server::new(...) -> Server::start()`.

use crate::core::{
    app::AppBuilder, cors::CorsConfig, logging::LoggingConfig, logging::init_logging,
    logging::structured_logging_middleware,
};
use axum::{Router, middleware};
use std::net::SocketAddr;
use tracing::{error, info};

pub(crate) type StartupValidation =
    Box<dyn Fn() -> Result<(), ConfigError> + Send + Sync + 'static>;

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub enable_cors: bool,
    pub enable_tracing: bool,
    pub enable_logging: bool,
    pub cors_config: Option<CorsConfig>,
    pub logging_config: LoggingConfig,
}

impl ServerConfig {
    /// 创建新的服务器配置
    pub fn new(port: u16) -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port,
        }
    }

    /// 设置主机地址
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// 获取 SocketAddr
    pub fn address(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.host, self.port).parse().map_err(|_| {
            ConfigError::InvalidSocketAddress {
                host: self.host.clone(),
                port: self.port,
            }
        })
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 端口 0 会触发系统随机端口分配，这里要求显式配置固定端口
        if self.port == 0 {
            return Err(ConfigError::InvalidPort {
                port: self.port,
                min: 1,
                max: u16::MAX,
            });
        }

        // 验证主机地址
        if self.host.is_empty() {
            return Err(ConfigError::EmptyHost);
        }

        self.address()?;

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::new(3000)
    }
}

impl AppConfig {
    /// 创建新的应用配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 CORS
    pub fn with_cors(mut self, enable: bool) -> Self {
        self.enable_cors = enable;
        self
    }

    /// 设置 CORS 配置
    pub fn with_cors_config(mut self, config: CorsConfig) -> Self {
        self.cors_config = Some(config);
        self.enable_cors = true;
        self
    }

    /// 设置追踪
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// 设置日志
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// 设置日志配置
    pub fn with_logging_config(mut self, config: LoggingConfig) -> Self {
        self.logging_config = config;
        self
    }

    /// 获取 CORS 配置
    pub fn get_cors_config(&self) -> Option<CorsConfig> {
        if self.enable_cors {
            Some(self.cors_config.clone().unwrap_or_else(|| {
                if cfg!(debug_assertions) {
                    CorsConfig::new().dev_mode(true)
                } else {
                    CorsConfig::new()
                }
            }))
        } else {
            None
        }
    }

    /// 验证应用配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证 CORS 配置
        if let Some(cors_config) = &self.cors_config {
            cors_config
                .validate()
                .map_err(|e| ConfigError::InvalidCors(e.to_string()))?;
        }

        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enable_cors: true,
            enable_tracing: true,
            enable_logging: true,
            cors_config: None,
            logging_config: LoggingConfig::default(),
        }
    }
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid port {port}: must be between {min} and {max}")]
    InvalidPort { port: u16, min: u16, max: u16 },
    #[error("Empty host address")]
    EmptyHost,
    #[error("Invalid CORS configuration: {0}")]
    InvalidCors(String),
    #[error("Invalid socket address: {host}:{port}")]
    InvalidSocketAddress { host: String, port: u16 },
    #[error("Empty allowed origins")]
    EmptyAllowedOrigins,
    #[error("Empty allowed methods")]
    EmptyAllowedMethods,
    #[error("Empty allowed headers")]
    EmptyAllowedHeaders,
    #[error("Credentials enabled without exposed headers")]
    CredentialsWithoutExposedHeaders,
    #[error("Invalid auth configuration: {0}")]
    InvalidAuth(String),
    #[error("Invalid protection configuration: {0}")]
    InvalidProtection(String),
}

/// HTTP 服务器
pub struct Server {
    server_config: ServerConfig,
    app_builder: AppBuilder,
}

impl Server {
    /// 创建新的服务器
    pub fn new(server_config: ServerConfig, app_builder: AppBuilder) -> Self {
        Self {
            server_config,
            app_builder,
        }
    }

    /// 启动服务器
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let (app, app_config, startup_validations) = self.app_builder.into_parts();

        // 初始化日志系统
        if app_config.enable_logging {
            init_logging(&app_config.logging_config)?;
        }

        // 验证服务器配置
        self.server_config
            .validate()
            .map_err(|e| format!("Invalid server configuration: {}", e))?;

        // 验证应用配置
        app_config
            .validate()
            .map_err(|e| format!("Invalid app configuration: {}", e))?;

        for startup_validation in startup_validations {
            startup_validation().map_err(|e| format!("Invalid startup configuration: {}", e))?;
        }

        // 构建应用
        let app = apply_app_layers(app, &app_config);

        let addr = self
            .server_config
            .address()
            .map_err(|e| format!("Invalid server configuration: {}", e))?;

        info!(
            host = %self.server_config.host,
            port = %self.server_config.port,
            cors_enabled = app_config.enable_cors,
            "🚀 Server starting on http://{}",
            addr
        );
        print_default_endpoints(addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await?;

        Ok(())
    }
}

fn apply_app_layers(mut router: Router, config: &AppConfig) -> Router {
    // Keep middleware assembly centralized so route composition and runtime
    // concerns stay separated.
    if config.enable_logging {
        router = router.layer(middleware::from_fn(structured_logging_middleware));
    }

    if config.enable_tracing {
        router = router.layer(tower_http::trace::TraceLayer::new_for_http());
    }

    if let Some(cors_config) = config.get_cors_config() {
        router = router.layer(cors_config.build_layer());
    }

    router
}

fn print_default_endpoints(addr: SocketAddr) {
    let base_url = format!("http://{}", addr);
    println!("Default endpoints:");
    println!("  - {}/health", base_url);
    println!("  - {}/health/detailed", base_url);
    println!("  - {}/api/v1/status", base_url);
}

async fn shutdown_signal() {
    // `Ctrl+C` is supported on every platform.
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(error = %err, "Failed to install Ctrl+C signal handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                error!(error = %err, "Failed to install SIGTERM signal handler");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    let signal = tokio::select! {
        _ = ctrl_c => "Ctrl+C",
        _ = terminate => "SIGTERM",
    };

    println!("Received {signal}, starting graceful shutdown...");
    info!(
        signal,
        "Shutdown signal received, starting graceful shutdown"
    );
}
