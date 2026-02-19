# 认证和授权模块使用指南

本模块提供了灵活的认证和授权解决方案，支持多种认证方式：
生产硬化建议请同时参考 `doc/SECURITY_NOTES.md`。
全部文档索引见 `doc/README.md`。

## 支持的认证方式

### 1. Basic 认证
使用用户名和密码进行认证，密码使用 bcrypt 哈希存储。

### 2. Bearer API Key 认证
使用预定义的 API Key 进行认证，适用于服务间调用。

### 3. Bearer JWT 认证
使用 JSON Web Token 进行无状态认证，支持过期时间控制。

## 快速开始

### 1. 配置认证

```rust
use common_http_server::auth::{auth_presets, BasicUser, SharedAuthConfig};
use jsonwebtoken::Algorithm;
use common_http_server::HttpsPolicy;
use ipnet::IpNet;

// 使用预定义的开发环境配置
let auth_config: SharedAuthConfig = auth_presets::development().shared();

// 或者自定义配置（推荐用 builder，避免字段变更导致示例过期）
let mut auth_config = common_http_server::auth::AuthConfig::default()
    .with_jwt_secret("replace-with-at-least-32-char-random-secret")
    .with_jwt_algorithm(Algorithm::HS256)
    .with_jwt_issuer("your-service")
    .with_jwt_audience("your-clients")
    .with_jwt_leeway_seconds(30)
    .with_https_policy(HttpsPolicy::RequireSecureTransport)
    .with_trusted_proxy("10.0.0.0/8".parse::<IpNet>()?);
auth_config.api_keys = vec!["your-api-key".to_string()];
auth_config.basic_users = vec![
    BasicUser::new("admin", "password123", vec!["admin", "user"])?,
    BasicUser::new("user", "password123", vec!["user"])?,
];
let auth_config = auth_config.shared();
```

### 1.1 生产环境最小安全配置（推荐）

```rust
use common_http_server::{AppBuilder, AppConfig, AuthConfig, HttpsPolicy, Server, ServerConfig};
use ipnet::IpNet;

let jwt_secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set and should be at least 32 characters");

let auth = AuthConfig::default()
    .with_jwt_secret(jwt_secret)
    .with_jwt_issuer("your-service")
    .with_jwt_audience("your-clients")
    .with_https_policy(HttpsPolicy::RequireSecureTransport)
    .with_trusted_proxy("10.0.0.0/8".parse::<IpNet>()?);
let auth = auth.shared();

let app_builder = AppBuilder::new(AppConfig::default())
    .validate_auth_config(auth.clone());

let server = Server::new(ServerConfig::new(3000), app_builder);
```

### 2. 应用认证中间件

```rust
use axum::{middleware, Router};
use common_http_server::auth::{
    auth_presets, basic_auth_middleware, api_key_auth_middleware, jwt_auth_middleware
};

let auth_config = auth_presets::development().shared();

// Basic 认证路由
let basic_routes = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(auth_config.clone(), basic_auth_middleware));

// API Key 认证路由
let api_key_routes = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(auth_config.clone(), api_key_auth_middleware));

// JWT 认证路由
let jwt_routes = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(auth_config.clone(), jwt_auth_middleware));

let app = Router::new()
    .nest("/basic", basic_routes)
    .nest("/api-key", api_key_routes)
    .nest("/jwt", jwt_routes);
```

### 3. 角色和权限控制

```rust
use common_http_server::auth::{require_roles, require_permissions};

// 需要管理员角色的路由
let admin_routes = Router::new()
    .route("/dashboard", get(admin_dashboard))
    .layer(middleware::from_fn(require_roles(vec!["admin"])));

// 需要特定权限的路由
let write_routes = Router::new()
    .route("/create", post(create_resource))
    .layer(middleware::from_fn(require_permissions(vec!["write"])));
```

## API 使用示例

### Basic 认证

```bash
# 使用 curl 测试
curl -H "Authorization: Basic YWRtaW46YWRtaW4xMjM=" \
     http://localhost:8080/basic/protected
```

### API Key 认证

```bash
curl -H "Authorization: Bearer your-api-key" \
     http://localhost:8080/api-key/protected
```

### JWT 认证

```bash
# 先登录获取 token
curl -X POST -H "Content-Type: application/json" \
     -d '{"username":"admin","password":"admin123"}' \
     http://localhost:8080/login

# 使用 token 访问受保护的资源
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:8080/jwt/protected
```

## 高级功能

### 获取当前用户信息

```rust
use common_http_server::auth::get_auth_user;
use axum::extract::Request;

async fn user_info(request: Request) -> Result<Json<ApiResponse<User>>, AuthError> {
    let auth_user = get_auth_user(&request)?;
    Ok(Json(ApiResponse::success(auth_user.user)))
}
```

### 检查用户角色和权限

```rust
use common_http_server::auth::{user_has_role, user_has_permission};

async fn check_permissions(request: Request) -> &'static str {
    if user_has_role(&request, "admin") {
        "Admin access granted"
    } else if user_has_permission(&request, "read") {
        "Read access granted"
    } else {
        "Access denied"
    }
}
```

### JWT 工具函数

```rust
use common_http_server::auth::{auth_presets, JwtUtils, User};

let auth_config = auth_presets::development();

// 生成 JWT token
let user = User {
    id: "123".to_string(),
    username: "john_doe".to_string(),
    roles: vec!["user".to_string()],
    permissions: vec!["read".to_string()],
};

let token = JwtUtils::generate_token(&user, &auth_config)?;

// 验证 JWT token
let claims = JwtUtils::verify_token(&token, &auth_config)?;
```

## 安全最佳实践

1. **生产环境配置**
   - 使用强密钥作为 JWT secret
   - 设置较短的 token 过期时间
   - 配置 `https_policy = RequireSecureTransport`
   - 正确设置 `trusted_proxies`
   - 从安全存储加载敏感配置

2. **密码安全**
   - 使用 bcrypt 进行密码哈希
   - 要求强密码策略
   - 实现密码重置功能

3. **Token 管理**
   - 实现 token 刷新机制
   - 考虑使用 token 黑名单
   - 记录认证失败尝试

## 错误处理

认证失败时会返回相应的 HTTP 状态码：

- `401 Unauthorized`: 认证失败、token 无效、缺少认证头
- `403 Forbidden`: 权限不足
- `426 Upgrade Required`: 启用了 `https_policy = RequireSecureTransport` 但请求不是 HTTPS
- `500 Internal Server Error`: 服务器内部错误（未预期错误）

说明：
- Basic 认证对“用户不存在/密码错误”统一返回 `Invalid credentials`，减少用户枚举风险。

错误响应格式：
```json
{
    "success": false,
    "error": "Invalid credentials",
    "status_code": 401
}
```

## 完整示例

运行示例代码：
```bash
# JWT + 客户端交互示例
cargo run -p common-http-server --example jwt_with_client --features external-health

# 认证 + 保护 + 监控组合示例
cargo run -p common-http-server --example level3_security_and_monitoring
```

示例包含：
- `jwt_with_client`:
  - `/auth/login` - 登录获取 JWT token
  - `/api/me` - JWT 认证保护接口
- `level3_security_and_monitoring`:
  - `/secure/profile` - Basic 认证 + 角色控制
  - `/monitor/metrics` - 指标端点
