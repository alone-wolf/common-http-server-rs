# CORS 配置指南

这个 HTTP 服务器框架提供了细粒度的 CORS 配置功能，支持多种使用场景。
生产安全建议请同时参考 `doc/SECURITY_NOTES.md`。
全部文档索引见 `doc/README.md`。

## 🚀 快速开始

### 基本使用

```rust
use common_http_server_rs::{AppBuilder, AppConfig, CorsConfig, Server, ServerConfig};

let cors_config = CorsConfig::new()
    .allowed_origins(vec!["http://localhost:3000"])
    .allowed_methods(vec!["GET", "POST"])
    .allowed_headers(vec!["Content-Type", "Authorization"])
    .allow_credentials(true);

let server_config = ServerConfig::new(3000);
let app_config = AppConfig::new().with_cors_config(cors_config);
let app_builder = AppBuilder::new(app_config);

let server = Server::new(server_config, app_builder);
server.start().await?;
```

## 📋 配置选项

### 源配置 (Origins)
```rust
.allowed_origins(vec![
    "http://localhost:3000",
    "https://yourdomain.com",
    "https://app.yourdomain.com"
])
```

### 方法配置 (Methods)
```rust
.allowed_methods(vec![
    "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"
])
```

### 头部配置 (Headers)
```rust
// 允许的请求头
.allowed_headers(vec![
    "Content-Type",
    "Authorization",
    "X-Requested-With",
    "X-Request-ID"
])

// 暴露给客户端的响应头
.exposed_headers(vec![
    "X-Total-Count",
    "X-Request-ID"
])
```

### 凭证配置
```rust
.allow_credentials(true)  // 允许发送 cookies
```

### 缓存配置
```rust
.max_age(7200)  // 预检请求缓存 2 小时
```

## 🎯 预设配置

### 1. 开发环境
```rust
use common_http_server_rs::presets;

let cors_config = presets::development();
```
- 允许所有源
- 允许所有方法和头部
- 适合本地开发

### 2. 生产环境 Web API
```rust
let cors_config = presets::web_api()
    .allowed_origins(vec!["https://yourdomain.com"])
    .allow_credentials(true);
```
- 严格的源控制
- 仅允许必要的 HTTP 方法
- 适合生产环境

### 3. 移动应用
```rust
let cors_config = presets::mobile_app();
```
- 支持 Capacitor/Ionic 应用
- 允许 localhost 和 HTTPS
- 适合混合移动应用

### 4. 多域名配置
```rust
let cors_config = presets::multi_domain(vec![
    "https://app1.example.com",
    "https://app2.example.com",
    "https://admin.example.com"
]);
```
- 支持多个前端应用
- 统一的 API 后端

## 🔧 环境变量配置

可以通过环境变量配置 CORS：

```bash
export CORS_ALLOWED_ORIGINS="http://localhost:3000,https://yourdomain.com"
export CORS_ALLOWED_METHODS="GET,POST,PUT,DELETE"
export CORS_ALLOWED_HEADERS="Content-Type,Authorization"
export CORS_ALLOW_CREDENTIALS="true"
export CORS_MAX_AGE="7200"
export CORS_DEV_MODE="false"
```

然后在代码中使用：

```rust
let cors_config = CorsConfig::from_env();
```

## ⚠️ 重要注意事项

### 1. 凭证与通配符冲突
当 `allow_credentials(true)` 时，不应把允许源设置为 `*`：

```rust
// ❌ 不推荐
let cors_config = CorsConfig::new()
    .allowed_origins(vec!["*"])
    .allow_credentials(true);

// ✅ 正确
let cors_config = CorsConfig::new()
    .allowed_origins(vec!["http://localhost:3000"])
    .allow_credentials(true);
```

### 2. 开发模式自动处理
在开发模式下，框架会自动处理凭证与通配符的冲突：

```rust
let cors_config = CorsConfig::new()
    .dev_mode(true)  // 开发模式
    .allow_credentials(true);  // 会自动使用具体的源而不是通配符
```

### 3. 预检请求缓存
合理设置 `max_age` 可以减少预检请求的频率：

```rust
.max_age(86400)  // 24 小时，适合生产环境
.max_age(300)    // 5 分钟，适合开发环境
```

## 🧪 测试 CORS

### 测试预检请求
```bash
curl -v -X OPTIONS \
  -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: Content-Type,Authorization" \
  http://localhost:3000/test
```

### 测试实际请求
```bash
curl -v \
  -H "Origin: http://localhost:3000" \
  -H "Content-Type: application/json" \
  http://localhost:3000/test
```

## 📝 示例项目

查看 `src/main.rs` 了解当前 CORS 配置示例（包含 `CorsConfig` 的链式配置）：

```bash
# 运行 common-http-server-rs 内置示例应用
cargo run -p common-http-server-rs
```

## 🔍 调试 CORS

启用日志记录来调试 CORS 问题：

```bash
RUST_LOG=debug cargo run -p common-http-server-rs
```

日志会显示：
- CORS 配置模式
- 预检请求处理
- 实际请求的 CORS 头

## 🛡️ 安全最佳实践

1. **生产环境不要使用通配符**
2. **仅允许必要的 HTTP 方法**
3. **仅允许必要的请求头**
4. **合理设置缓存时间**
5. **定期审查允许的源列表**

## 📚 更多资源

- [MDN CORS 文档](https://developer.mozilla.org/zh-CN/docs/Web/HTTP/CORS)
- [Axum CORS 中间件](https://docs.rs/tower-http/latest/tower_http/cors/index.html)
- [HTTP 访问控制](https://fetch.spec.whatwg.org/#http-access-control)
