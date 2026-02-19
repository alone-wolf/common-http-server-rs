pub mod config;
pub mod helpers;
pub mod jwt;
pub mod middleware;
pub mod types;

// Re-export all public types and functions
pub use config::{AuthConfig, HttpsPolicy, SharedAuthConfig, presets};
pub use helpers::{get_auth_user, user_has_permission, user_has_role};
pub use jwt::JwtUtils;
pub use middleware::{
    api_key_auth_middleware, basic_auth_middleware, jwt_auth_middleware, require_permissions,
    require_roles,
};
pub use types::{AuthError, AuthType, AuthUser, BasicUser, Claims, User};
