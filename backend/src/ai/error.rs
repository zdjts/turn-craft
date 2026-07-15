
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("AI 配置不存在")]
    ConfigNotFound,

    #[error("AI 服务不可用: {0}")]
    #[allow(dead_code)]
    ProviderError(String),

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("数据序列化错误: {0}")]
    Json(#[from] serde_json::Error),
}
