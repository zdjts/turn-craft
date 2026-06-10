use serde::Serialize;

/// 动作来源类型：AI 或人类玩家
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ActionKind {
    Ai,
    Human,
}

/// 游戏自定义载荷约束：可序列化 + 线程安全
pub trait Payload: Serialize + Send + 'static {}
/// # 游戏状态机副作用事件
///
/// 引擎 `step` 计算完毕后向外吐出的"待办事项清单"。
/// 通用房间容器（Container）通过消费此队列，来代为执行网络 IO 或线程隔离。
pub enum GameEvent<R: GameRole, P: Payload> {
    /// 全场纯文本广播（如玩家发言、公共牌翻开）
    Broadcast(String),

    /// 触发 AI 决策。容器拦截后会把快照打包扔给后台 AI 线程池，避免大模型请求卡死房间循环
    TriggerAi(String),

    /// 游戏达到结束条件。容器收到后会通知全场并退出 Actor 循环，平稳回收房间内存
    GameOver,

    /// 游戏特化高级业务载荷。通用外壳看不懂（如德州筹码变更），但会自动将其转为 JSON 广播给特定前端
    Custom(P),

    /// 战争迷雾定向单播。容器会根据 `role` 找到对应的 `actor_id` 进行精准网络单播（如发底牌）
    NotifyRole {
        /// 接收数据的目标游戏角色
        role: R,
        /// 针对该角色特化序列化后的字符串载荷
        payload: String,
    },
}

/// 游戏角色约束：可序列化 + 可比较 + 线程安全
pub trait GameRole: serde::Serialize + Clone + Send + Sync + Eq + PartialEq {}

/// 游戏动作约束：可序列化 + 可调试 + 线程安全
pub trait GameAction: serde::Serialize + Send + Sync + std::fmt::Debug {}

/// 参与者：包含唯一标识、来源类型和角色
pub struct Actor<R: GameRole> {
    pub id: String,
    pub kind: ActionKind,
    pub role: R,
}

/// 房间状态：管理参与者列表和动作历史
pub struct RoomState<R: GameRole, A: GameAction> {
    pub room_id: String,
    pub game_type: String,
    pub actors: Vec<Actor<R>>,
    pub history: Vec<A>,
}

impl<R: GameRole, A: GameAction> RoomState<R, A> {
    pub fn new(room_id: String, game_type: String) -> Self {
        Self {
            room_id,
            game_type,
            actors: Vec::new(),
            history: Vec::new(),
        }
    }
    pub fn find_actor(&self, actor_id: &str) -> Option<&Actor<R>> {
        self.actors.iter().find(|&x| x.id == *actor_id)
    }
}

/// 可玩游戏接口：解析动作、执行步进、获取快照
pub trait Playable<R: GameRole, A: GameAction, P: Payload, E: std::fmt::Debug>:
    Send + Sync + 'static
{
    /// 将原始内容解析为游戏动作
    fn parse_action(&self, actor_id: &str, raw_content: &str) -> Result<A, E>;
    /// 执行一步动作，返回副作用事件列表
    fn step(&mut self, state: &mut RoomState<R, A>, action: A) -> Result<Vec<GameEvent<R, P>>, E>;
    /// 获取指定角色视角的状态快照（支持战争迷雾）
    fn get_snapshot(&self, state: &RoomState<R, A>, role: &R) -> String;
}

// ═══════════════════════════════════════════════════════
//  GameEngine — 类型擦除后的泛型游戏引擎合约
// ═══════════════════════════════════════════════════════

/// 引擎侧事件：房间循环消费这些副作用来代为执行网络 IO
pub enum EngineEvent {
    /// 触发 AI 决策，值为目标 actor_id
    TriggerAi(String),
    /// 游戏结束
    GameOver,
}

/// 通用游戏引擎接口
///
/// 所有游戏实现此 trait，房间循环完全通过此 trait 与游戏交互。
/// 房间循环成为纯粹的"收取 Action → 喂给引擎 step → 广播引擎 to_json 快照"的失忆中转站。
pub trait GameEngine: Send + 'static {
    /// 游戏类型标识（如 "lincoln"、"werewolf"）
    fn game_type(&self) -> &str;

    /// 执行一步动作，返回副作用事件列表
    fn step(&mut self, actor_id: &str, action: serde_json::Value) -> Result<Vec<EngineEvent>, String>;

    /// 导出当前全量状态快照为 JSON
    fn to_json(&self) -> serde_json::Value;

    /// 当前应该行动的 actor_id
    fn current_actor(&self) -> Option<String>;

    /// 游戏是否已结束
    fn is_finished(&self) -> bool;
}
