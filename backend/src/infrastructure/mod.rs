//! 基础设施层 — 外部依赖适配
//!
//! 职责：
//! - SQLite 数据库适配
//! - JWT 签发/验证
//! - Tracing/日志
//! - WebSocket 连接管理
//! - AI API 客户端
//! - 事件存储
//!
//! 参见 docs/architecture_handover.md 第 5.3 节。

pub use crate::event_store::{EventStore, SqliteEventStore, GameEvent};
