#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("房间不存在")]
    NotFound,
    #[error("角色槽位已被占用")]
    SlotOccupied,
    #[error("你不是房间的主人")]
    NotOwner,
    #[error("不支持的游戏类型: {0}")]
    UnsupportedGameType(String),
    #[error("{0}")]
    EngineError(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("数据序列化错误: {0}")]
    Json(#[from] serde_json::Error),
}
