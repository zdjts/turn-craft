#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("用户名已存在")]
    UsernameTaken,
    #[error("用户不存在")]
    NotFound,

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
}
