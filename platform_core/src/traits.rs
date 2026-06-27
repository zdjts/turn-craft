/// 动作来源类型：AI 或人类玩家
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ActionKind {
    Ai,
    Human,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, Debug)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cached_tokens: u64,
}

impl TokenUsage {
    pub fn accumulate(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.cached_tokens += other.cached_tokens;
    }
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

// ═══════════════════════════════════════════════════════
//  GameEngine — 类型擦除后的泛型游戏引擎合约
// ═══════════════════════════════════════════════════════

/// 引擎侧事件：房间循环消费这些副作用来代为执行网络 IO
pub enum EngineEvent {
    /// 触发 AI 决策，值为目标 actor_id
    TriggerAi(String),
    /// 游戏结束
    GameOver,
    /// 向特定玩家发送私密消息（如手牌）
    PrivateMessage {
        actor_id: String,
        payload: serde_json::Value,
    },
}

/// 通用游戏引擎接口
///
/// 所有游戏实现此 trait，房间循环完全通过此 trait 与游戏交互。
/// 房间循环成为纯粹的"收取 Action → 喂给引擎 step → 广播引擎 to_json 快照"的失忆中转站。
pub trait GameEngine: Send + 'static {
    /// 游戏类型标识（如 "lincoln"、"werewolf"）
    fn game_type(&self) -> &str;

    /// 执行一步动作，返回副作用事件列表
    fn step(
        &mut self,
        actor_id: &str,
        action: serde_json::Value,
    ) -> Result<Vec<EngineEvent>, crate::error::EngineError>;

    /// 导出当前全量状态快照为 JSON
    fn to_json(&self) -> serde_json::Value;

    /// 导出针对特定玩家的状态快照（默认调用 to_json）
    fn to_json_for_player(&self, _actor_id: &str) -> serde_json::Value {
        self.to_json()
    }

    /// 专门为大模型生成的、经过 Prompt Caching 优化的快照格式
    /// 默认实现为回退到 to_json_for_player
    fn to_ai_prompt(&self, actor_id: &str) -> String {
        self.to_json_for_player(actor_id).to_string()
    }

    /// 当前应该行动的 actor_id
    fn current_actor(&self) -> Option<String>;

    /// 游戏是否已结束
    fn is_finished(&self) -> bool;

    /// AI 使用的工具定义（OpenAI function calling 格式）
    /// 默认返回 None，表示不使用 tool use
    /// 需要使用 tool use 的游戏（如德州扑克）应覆写此方法
    fn tools(&self) -> Option<serde_json::Value> {
        None
    }
}
