use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};

use super::model::RoomCommand;

/// 保活超时时间
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(600);

/// 房间保活管理器
///
/// 当房间所有玩家离开时，调用 `track()` 开始计时。
/// 当玩家重连时，调用 `release()` 取消计时。
/// 超过 `RECONNECT_TIMEOUT` 后自动发送 `Shutdown` 给房间 actor。
#[derive(Clone)]
pub struct RoomSupervisor {
    inner: Arc<Mutex<SupervisorInner>>,
}

struct SupervisorInner {
    /// room_id → (开始等待的时间, 向 actor 发送命令的 channel)
    pending: HashMap<String, (Instant, mpsc::Sender<RoomCommand>)>,
}

impl RoomSupervisor {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SupervisorInner {
                pending: HashMap::new(),
            })),
        }
    }

    /// 开始跟踪房间的保活状态
    pub async fn track(&self, room_id: String, cmd_tx: mpsc::Sender<RoomCommand>) {
        let mut inner = self.inner.lock().await;
        inner.pending.insert(room_id, (Instant::now(), cmd_tx));
    }

    /// 取消房间的保活跟踪（玩家重连时调用）
    pub async fn release(&self, room_id: &str) {
        let mut inner = self.inner.lock().await;
        inner.pending.remove(room_id);
    }

    /// 启动保活检查循环，在后台运行
    pub async fn run(self) {
        info!("房间保活管理器已启动");

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let expired: Vec<String> = {
                let inner = self.inner.lock().await;
                inner
                    .pending
                    .iter()
                    .filter(|(_, (since, _))| since.elapsed() >= RECONNECT_TIMEOUT)
                    .map(|(id, _)| id.clone())
                    .collect()
            };

            for room_id in expired {
                let cmd_tx = {
                    let mut inner = self.inner.lock().await;
                    inner.pending.remove(&room_id).map(|(_, tx)| tx)
                };

                if let Some(tx) = cmd_tx {
                    warn!(room_id = %room_id, "保活超时，关闭房间");
                    if tx.send(RoomCommand::Shutdown).await.is_err() {
                        warn!(room_id = %room_id, "房间 actor 已关闭");
                    }
                }
            }
        }
    }
}
