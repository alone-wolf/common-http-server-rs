//! CORS configuration helpers and presets.

use crate::core::server::ConfigError;
use axum::http::{HeaderName, HeaderValue, Method};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tower_http::cors::{Any, CorsLayer};

/// CORS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// 允许的源
    pub allowed_origins: Vec<String>,
    /// 允许的方法
    pub allowed_methods: Vec<String>,
    /// 允许的头
    pub allowed_headers: Vec<String>,
    /// 暴露的头
    pub exposed_headers: Vec<String>,
    /// 是否允许凭证
    pub allow_credentials: bool,
    /// 预检请求缓存时间（秒）
    pub max_age: Option<u64>,
    /// 是否在开发模式下允许所有源
    pub dev_mode_allow_all: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
            ],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
                "X-Request-ID".to_string(),
            ],
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
            dev_mode_allow_all: false,
        }
    }
}

impl CorsConfig {
    /// 创建新的 CORS 配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置允许的源
    pub fn allowed_origins(mut self, origins: Vec<&str>) -> Self {
        self.allowed_origins = origins.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加允许的源
    pub fn add_allowed_origin(mut self, origin: &str) -> Self {
        self.allowed_origins.push(origin.to_string());
        self
    }

    /// 设置允许的方法
    pub fn allowed_methods(mut self, methods: Vec<&str>) -> Self {
        self.allowed_methods = methods.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加允许的方法
    pub fn add_allowed_method(mut self, method: &str) -> Self {
        self.allowed_methods.push(method.to_string());
        self
    }

    /// 设置允许的头
    pub fn allowed_headers(mut self, headers: Vec<&str>) -> Self {
        self.allowed_headers = headers.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加允许的头
    pub fn add_allowed_header(mut self, header: &str) -> Self {
        self.allowed_headers.push(header.to_string());
        self
    }

    /// 设置暴露的头
    pub fn exposed_headers(mut self, headers: Vec<&str>) -> Self {
        self.exposed_headers = headers.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加暴露的头
    pub fn add_exposed_header(mut self, header: &str) -> Self {
        self.exposed_headers.push(header.to_string());
        self
    }

    /// 设置是否允许凭证
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    /// 设置预检请求缓存时间
    pub fn max_age(mut self, max_age: u64) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// 设置开发模式（允许所有源）
    pub fn dev_mode(mut self, dev_mode: bool) -> Self {
        self.dev_mode_allow_all = dev_mode;
        self
    }

    /// 构建生产环境 CORS 层
    pub fn build_production_layer(&self) -> CorsLayer {
        let mut cors = CorsLayer::new();

        // 设置允许的源
        if !self.allowed_origins.is_empty() {
            let origins: Vec<HeaderValue> = self
                .allowed_origins
                .iter()
                .filter_map(|s| HeaderValue::from_str(s).ok())
                .collect();

            if !origins.is_empty() {
                cors = cors.allow_origin(origins);
            }
        }

        // 设置允许的方法
        if !self.allowed_methods.is_empty() {
            let methods: Vec<Method> = self
                .allowed_methods
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();

            if !methods.is_empty() {
                cors = cors.allow_methods(methods);
            }
        }

        // 设置允许的头
        if !self.allowed_headers.is_empty() {
            let headers: Vec<HeaderName> = self
                .allowed_headers
                .iter()
                .filter_map(|s| HeaderName::from_str(s).ok())
                .collect();
            if !headers.is_empty() {
                cors = cors.allow_headers(headers);
            }
        }

        // 设置暴露的头
        if !self.exposed_headers.is_empty() {
            let headers: Vec<HeaderName> = self
                .exposed_headers
                .iter()
                .filter_map(|s| HeaderName::from_str(s).ok())
                .collect();
            if !headers.is_empty() {
                cors = cors.expose_headers(headers);
            }
        }

        // 设置凭证
        if self.allow_credentials {
            cors = cors.allow_credentials(true);
        }

        // 设置最大缓存时间
        if let Some(max_age) = self.max_age {
            cors = cors.max_age(std::time::Duration::from_secs(max_age));
        }

        cors
    }

    /// 构建开发环境 CORS 层（允许所有）
    pub fn build_development_layer(&self) -> CorsLayer {
        let mut cors = CorsLayer::new().max_age(std::time::Duration::from_secs(
            self.max_age.unwrap_or(86400),
        ));

        // 如果允许凭证，需要明确指定允许的源、方法和头，而不是使用 Any
        if self.allow_credentials {
            cors = cors
                .allow_credentials(true)
                .allow_origin(vec![
                    HeaderValue::from_static("http://localhost:3000"),
                    HeaderValue::from_static("http://localhost:8080"),
                    HeaderValue::from_static("http://127.0.0.1:3000"),
                    HeaderValue::from_static("http://127.0.0.1:8080"),
                ])
                .allow_methods(vec![
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::PATCH,
                    Method::OPTIONS,
                ])
                .allow_headers(vec![
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("authorization"),
                    HeaderName::from_static("x-requested-with"),
                    HeaderName::from_static("x-request-id"),
                ]);
        } else {
            cors = cors.allow_origin(Any).allow_methods(Any).allow_headers(Any);
        }

        cors
    }

    /// 根据环境配置构建 CORS 层
    pub fn build_layer(&self) -> CorsLayer {
        if self.dev_mode_allow_all {
            tracing::warn!("CORS is running in development mode - allowing all origins");
            self.build_development_layer()
        } else {
            tracing::info!("CORS is running in production mode with restricted origins");
            self.build_production_layer()
        }
    }

    /// 验证 CORS 配置
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证允许的源
        if self.allowed_origins.is_empty() && !self.dev_mode_allow_all {
            return Err(ConfigError::EmptyAllowedOrigins);
        }

        // 验证允许的方法
        if self.allowed_methods.is_empty() {
            return Err(ConfigError::EmptyAllowedMethods);
        }

        // 验证允许的头
        if self.allowed_headers.is_empty() {
            return Err(ConfigError::EmptyAllowedHeaders);
        }

        Ok(())
    }

    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // 从环境变量读取配置
        if let Ok(origins) = std::env::var("CORS_ALLOWED_ORIGINS") {
            config.allowed_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(methods) = std::env::var("CORS_ALLOWED_METHODS") {
            config.allowed_methods = methods.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(headers) = std::env::var("CORS_ALLOWED_HEADERS") {
            config.allowed_headers = headers.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(exposed) = std::env::var("CORS_EXPOSED_HEADERS") {
            config.exposed_headers = exposed.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(creds) = std::env::var("CORS_ALLOW_CREDENTIALS") {
            config.allow_credentials = creds.parse().unwrap_or(false);
        }

        if let Ok(max_age) = std::env::var("CORS_MAX_AGE") {
            config.max_age = max_age.parse().ok();
        }

        if let Ok(dev_mode) = std::env::var("CORS_DEV_MODE") {
            config.dev_mode_allow_all = dev_mode.parse().unwrap_or(false);
        }

        config
    }
}

/// 预定义的 CORS 配置
pub mod presets {
    use super::*;

    /// 严格的 Web API 配置
    pub fn web_api() -> CorsConfig {
        CorsConfig::new()
            .allowed_origins(vec!["https://yourdomain.com"])
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .allow_credentials(true)
            .max_age(3600) // 1 hour
    }

    /// 开发环境配置
    pub fn development() -> CorsConfig {
        CorsConfig::new().dev_mode(true).allow_credentials(true)
    }

    /// 移动应用配置
    pub fn mobile_app() -> CorsConfig {
        CorsConfig::new()
            .allowed_origins(vec![
                "capacitor://localhost",
                "ionic://localhost",
                "http://localhost",
                "https://localhost",
            ])
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization", "X-Requested-With"])
            .allow_credentials(true)
            .max_age(7200) // 2 hours
    }

    /// 多域名配置
    pub fn multi_domain(domains: Vec<&str>) -> CorsConfig {
        CorsConfig::new()
            .allowed_origins(domains)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
            .allowed_headers(vec![
                "Content-Type",
                "Authorization",
                "X-Requested-With",
                "X-Request-ID",
            ])
            .allow_credentials(true)
            .max_age(86400) // 24 hours
    }
}
