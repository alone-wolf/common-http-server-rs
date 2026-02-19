use crate::auth::types::{AuthError, AuthUser};

/// 从请求中获取认证用户
pub fn get_auth_user(request: &axum::extract::Request) -> Result<&AuthUser, AuthError> {
    request
        .extensions()
        .get::<AuthUser>()
        .ok_or(AuthError::MissingAuthHeader)
}

/// 检查用户是否有指定角色
pub fn user_has_role(request: &axum::extract::Request, role: &str) -> bool {
    get_auth_user(request)
        .map(|auth_user| auth_user.user.roles.contains(&role.to_string()))
        .unwrap_or(false)
}

/// 检查用户是否有指定权限
pub fn user_has_permission(request: &axum::extract::Request, permission: &str) -> bool {
    get_auth_user(request)
        .map(|auth_user| auth_user.user.permissions.contains(&permission.to_string()))
        .unwrap_or(false)
}
