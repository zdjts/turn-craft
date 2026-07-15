
use crate::user::error::UserError;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("用户名或密码错误")]
    InvalidCredentials,
    #[error("Token 无效")]
    Unauthorized,
    #[error("Token 已过期，请重新登录")]
    TokenExpired,
    #[error("用户名不能为空")]
    EmptyUsername,
    #[error("密码长度不能少于6位")]
    WeakPassword,
    #[error("用户系统错误: {0}")]
    User(#[from] UserError),
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("密码哈希错误: {0}")]
    CryptoError(String),
}
