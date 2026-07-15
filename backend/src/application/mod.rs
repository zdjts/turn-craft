//! 应用层 — Use Case 编排
//!
//! 职责：
//! - 登录/注册
//! - 房间创建/加入/离开
//! - AI 调度编排
//! - 回放编排
//! - 权限检查
//! - 状态恢复
//!
//! 当前状态：模块已声明，代码逐步从 `room::RoomService`、`auth::AuthService` 迁移至此。
//! 参见 docs/architecture_handover.md 第 5.2 节。
