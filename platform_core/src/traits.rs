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

// ═══════════════════════════════════════════════════════
//  身份映射 — 统一 slot_name / actor_id / peer_id
// ═══════════════════════════════════════════════════════

/// SlotId：房间创建时前端指定的槽位名称（如 "Judge"、"Player1"）
pub type SlotId = String;

/// ActorId：引擎内部 actor 的唯一标识
pub type ActorId = String;

/// PeerId：WebSocket 连接的唯一标识（当前等于 slot_name）
pub type PeerId = String;

/// 身份映射表：房间创建时生成并冻结
///
/// 确保 SlotId / ActorId / PeerId 三者之间的稳定映射，
/// 解决当前多人对局中身份漂移问题。
#[derive(Debug, Clone)]
pub struct IdentityMap {
    /// slot → actor（通常 1:1，slot_name == actor_id）
    slot_to_actor: std::collections::HashMap<SlotId, ActorId>,
    /// actor → slot（反向查找）
    actor_to_slot: std::collections::HashMap<ActorId, SlotId>,
}

impl IdentityMap {
    pub fn new(slot_names: &[String]) -> Self {
        let mut slot_to_actor = std::collections::HashMap::new();
        let mut actor_to_slot = std::collections::HashMap::new();
        for name in slot_names {
            // 默认 slot_name == actor_id，工厂也可覆盖
            slot_to_actor.insert(name.clone(), name.clone());
            actor_to_slot.insert(name.clone(), name.clone());
        }
        Self { slot_to_actor, actor_to_slot }
    }

    pub fn register_mapping(&mut self, slot: SlotId, actor: ActorId) {
        self.actor_to_slot.remove(&actor);
        self.slot_to_actor.remove(&slot);
        self.slot_to_actor.insert(slot.clone(), actor.clone());
        self.actor_to_slot.insert(actor, slot);
    }

    pub fn actor_for_slot(&self, slot: &str) -> Option<&str> {
        self.slot_to_actor.get(slot).map(|s| s.as_str())
    }

    pub fn slot_for_actor(&self, actor: &str) -> Option<&str> {
        self.actor_to_slot.get(actor).map(|s| s.as_str())
    }

    pub fn slots(&self) -> impl Iterator<Item = &str> {
        self.slot_to_actor.keys().map(|s| s.as_str())
    }

    pub fn actors(&self) -> impl Iterator<Item = &str> {
        self.actor_to_slot.keys().map(|s| s.as_str())
    }
}

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
    /// 玩家加入槽位（引擎如需感知可处理）
    PlayerJoined(String),
    /// 玩家离开槽位
    PlayerLeft(String),
}

/// 游戏元数据 — 供 API 端点返回，前端从端点获取描述信息
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GameMeta {
    pub game_type: String,
    pub name: String,
    pub description: String,
    pub min_players: usize,
    pub max_players: usize,
    pub slot_names: Vec<String>,
    pub config_schema: Option<serde_json::Value>,
}

/// 房间循环成为纯粹的"收取 Action → 喂给引擎 step → 广播引擎 to_json 快照"的失忆中转站。
pub trait GameEngine: Send + Sync + 'static {
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
