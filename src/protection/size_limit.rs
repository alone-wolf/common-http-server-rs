//! Request size-limiting middleware.

use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::BytesMut;
use http_body_util::BodyExt;
use std::sync::Arc;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct SizeLimitConfig {
    pub max_body_size: usize,
    pub max_header_size: usize,
    pub max_url_length: usize,
    pub check_content_length: bool,
    pub log_violations: bool,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB
            max_header_size: 8 * 1024,       // 8KB
            max_url_length: 2048,            // 2048 characters
            check_content_length: true,
            log_violations: true,
        }
    }
}

impl SizeLimitConfig {
    pub fn new(max_body_size: usize) -> Self {
        Self {
            max_body_size,
            ..Default::default()
        }
    }

    pub fn max_header_size(mut self, size: usize) -> Self {
        self.max_header_size = size;
        self
    }

    pub fn max_url_length(mut self, length: usize) -> Self {
        self.max_url_length = length;
        self
    }

    pub fn check_content_length(mut self, check: bool) -> Self {
        self.check_content_length = check;
        self
    }

    pub fn log_violations(mut self, log: bool) -> Self {
        self.log_violations = log;
        self
    }

    pub fn build(self) -> Arc<SizeLimitService> {
        Arc::new(SizeLimitService::new(self))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.max_body_size == 0 {
            return Err("max_body_size must be greater than 0".to_string());
        }
        if self.max_header_size == 0 {
            return Err("max_header_size must be greater than 0".to_string());
        }
        if self.max_url_length == 0 {
            return Err("max_url_length must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SizeLimitService {
    config: SizeLimitConfig,
}

impl SizeLimitService {
    pub fn new(config: SizeLimitConfig) -> Self {
        Self { config }
    }

    pub fn check_request_size(&self, request: &Request) -> Result<(), SizeLimitError> {
        // Check URL length
        let uri = request.uri();
        if uri
            .path_and_query()
            .map(|pq| pq.as_str().len())
            .unwrap_or(0)
            > self.config.max_url_length
        {
            if self.config.log_violations {
                warn!(
                    "URL length {} exceeds limit {}",
                    uri.path_and_query()
                        .map(|pq| pq.as_str().len())
                        .unwrap_or(0),
                    self.config.max_url_length
                );
            }
            return Err(SizeLimitError::UrlTooLong {
                actual: uri
                    .path_and_query()
                    .map(|pq| pq.as_str().len())
                    .unwrap_or(0),
                limit: self.config.max_url_length,
            });
        }

        // Check header size
        let header_size = self.calculate_header_size(request.headers());
        if header_size > self.config.max_header_size {
            if self.config.log_violations {
                warn!(
                    "Header size {} exceeds limit {}",
                    header_size, self.config.max_header_size
                );
            }
            return Err(SizeLimitError::HeadersTooLarge {
                actual: header_size,
                limit: self.config.max_header_size,
            });
        }

        // Check Content-Length header if enabled
        if self.config.check_content_length
            && let Some(content_length) = request.headers().get("content-length")
            && let Ok(length) = content_length.to_str()
            && let Ok(size) = length.parse::<usize>()
            && size > self.config.max_body_size
        {
            if self.config.log_violations {
                warn!(
                    "Content-Length {} exceeds limit {}",
                    size, self.config.max_body_size
                );
            }
            return Err(SizeLimitError::BodyTooLarge {
                actual: size,
                limit: self.config.max_body_size,
            });
        }

        Ok(())
    }

    fn calculate_header_size(&self, headers: &HeaderMap) -> usize {
        let mut size = 0;
        for (name, value) in headers {
            size += name.as_str().len();
            size += value.len();
            size += 4; // ": " + "\r\n"
        }
        size + 2 // Final "\r\n"
    }

    pub async fn read_body_with_limit<B>(&self, body: B) -> Result<Bytes, SizeLimitError>
    where
        B: axum::body::HttpBody + std::marker::Unpin,
        B::Data: AsRef<[u8]>,
        B::Error: std::fmt::Display,
    {
        let mut collected = BytesMut::new();
        let mut size = 0;

        let mut stream = Box::pin(body.into_data_stream());
        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| SizeLimitError::BodyReadError(e.to_string()))?;
            let chunk_bytes = chunk.as_ref();
            size += chunk_bytes.len();

            if size > self.config.max_body_size {
                if self.config.log_violations {
                    warn!(
                        "Body size {} exceeds limit {}",
                        size, self.config.max_body_size
                    );
                }
                return Err(SizeLimitError::BodyTooLarge {
                    actual: size,
                    limit: self.config.max_body_size,
                });
            }

            collected.extend_from_slice(chunk_bytes);
        }

        Ok(collected.freeze())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SizeLimitError {
    #[error("Request body too large: {actual} bytes (limit: {limit} bytes)")]
    BodyTooLarge { actual: usize, limit: usize },

    #[error("Request headers too large: {actual} bytes (limit: {limit} bytes)")]
    HeadersTooLarge { actual: usize, limit: usize },

    #[error("Request URL too long: {actual} characters (limit: {limit} characters)")]
    UrlTooLong { actual: usize, limit: usize },

    #[error("Failed to read request body: {0}")]
    BodyReadError(String),
}

impl IntoResponse for SizeLimitError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            SizeLimitError::BodyTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            SizeLimitError::HeadersTooLarge { .. } => StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            SizeLimitError::UrlTooLong { .. } => StatusCode::URI_TOO_LONG,
            SizeLimitError::BodyReadError(_) => StatusCode::BAD_REQUEST,
        };

        crate::core::response::ApiResponse::<()>::error_with_status(self.to_string(), status)
            .into_response()
    }
}

pub async fn size_limit_middleware(
    State(service): State<Arc<SizeLimitService>>,
    request: Request,
    next: Next,
) -> Result<Response, SizeLimitError> {
    // Check request size before processing
    service.check_request_size(&request)?;

    // Read and validate body size if there's a body
    let (parts, body) = request.into_parts();
    let limited_body = service.read_body_with_limit(body).await?;
    let limited_body = Body::from(limited_body);

    let request = Request::from_parts(parts, limited_body);

    Ok(next.run(request).await)
}

// A simpler version that only checks Content-Length header (more efficient for most cases)
pub async fn content_length_middleware(
    State(service): State<Arc<SizeLimitService>>,
    request: Request,
    next: Next,
) -> Result<Response, SizeLimitError> {
    // Only check Content-Length header, don't read the entire body
    if let Some(content_length) = request.headers().get("content-length")
        && let Ok(length_str) = content_length.to_str()
        && let Ok(size) = length_str.parse::<usize>()
        && size > service.config.max_body_size
    {
        if service.config.log_violations {
            warn!(
                "Content-Length {} exceeds limit {}",
                size, service.config.max_body_size
            );
        }
        return Err(SizeLimitError::BodyTooLarge {
            actual: size,
            limit: service.config.max_body_size,
        });
    }

    Ok(next.run(request).await)
}

pub mod presets {
    use super::*;

    pub fn minimal() -> SizeLimitConfig {
        SizeLimitConfig::new(1024 * 1024) // 1MB
            .max_header_size(4 * 1024) // 4KB
            .max_url_length(1024) // 1KB
    }

    pub fn moderate() -> SizeLimitConfig {
        SizeLimitConfig::new(10 * 1024 * 1024) // 10MB
            .max_header_size(8 * 1024) // 8KB
            .max_url_length(2048) // 2KB
    }

    pub fn generous() -> SizeLimitConfig {
        SizeLimitConfig::new(100 * 1024 * 1024) // 100MB
            .max_header_size(16 * 1024) // 16KB
            .max_url_length(4096) // 4KB
    }

    pub fn api() -> SizeLimitConfig {
        SizeLimitConfig::new(5 * 1024 * 1024) // 5MB
            .max_header_size(8 * 1024) // 8KB
            .max_url_length(2048) // 2KB
            .check_content_length(true)
    }

    pub fn file_upload() -> SizeLimitConfig {
        SizeLimitConfig::new(500 * 1024 * 1024) // 500MB
            .max_header_size(16 * 1024) // 16KB
            .max_url_length(4096) // 4KB
            .check_content_length(false) // Don't rely on Content-Length for uploads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_limit_config() {
        let config = SizeLimitConfig::new(1024);
        assert_eq!(config.max_body_size, 1024);
    }

    #[test]
    fn test_header_size_calculation() {
        let service = SizeLimitService::new(SizeLimitConfig::default());

        // Create a mock request with some headers
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("authorization", "Bearer token".parse().unwrap());

        let size = service.calculate_header_size(&headers);
        assert!(size > 0);
    }

    #[test]
    fn test_presets() {
        let minimal = presets::minimal();
        assert_eq!(minimal.max_body_size, 1024 * 1024);

        let generous = presets::generous();
        assert_eq!(generous.max_body_size, 100 * 1024 * 1024);
    }

    #[test]
    fn test_size_limit_config_validation() {
        let config = SizeLimitConfig::new(0);
        assert!(config.validate().is_err());
    }
}
