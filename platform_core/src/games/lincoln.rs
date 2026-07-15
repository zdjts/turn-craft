use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::traits::{ActionKind, EngineEvent, GameEngine};

/// 林肯辩论角色：正方、反方、裁判、结束
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
pub enum DebateRole {
    Pro,
    Con,
    Judge,
    Over,
}

/// 林肯辩论参与者
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LincolnActor {
    pub id: String,
    pub kind: String, // "Human" | "Ai"
    pub role: DebateRole,
}

/// 辩论历史条目：记录每次发言
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub actor_id: String,
    pub role: DebateRole,
    pub content: String,
}

/// 林肯辩论引擎：管理辩论状态机
pub struct LincolnEngine {
    pub room_id: String,
    pub max_round: usize,
    pub round: usize,
    pub cur_role: DebateRole,
    pub actors: Vec<LincolnActor>,
    pub history: Vec<HistoryEntry>,
    pub finished: bool,
    pub opening_done: bool,
}

impl LincolnEngine {
    /// 创建新的林肯辩论引擎
    pub fn new(room_id: String, max_round: usize) -> Self {
        Self {
            room_id,
            max_round,
            round: 0,
            cur_role: DebateRole::Judge,
            actors: Vec::new(),
            history: Vec::new(),
            finished: false,
            opening_done: false,
        }
    }

    /// 添加参与者到辩论
    pub fn add_actor(&mut self, id: String, kind: ActionKind, role: DebateRole) {
        self.actors.push(LincolnActor {
            id,
            kind: match kind {
                ActionKind::Ai => "Ai".to_string(),
                ActionKind::Human => "Human".to_string(),
            },
            role,
        });
    }
}

impl GameEngine for LincolnEngine {
    fn game_type(&self) -> &str {
        "lincoln"
    }

    fn step(
        &mut self,
        actor_id: &str,
        action: serde_json::Value,
    ) -> Result<Vec<EngineEvent>, crate::error::EngineError> {
        // 支持两种格式：
        // 1. 直接内容: {"content": "text"}
        // 2. 完整消息: {"role": "assistant", "content": "text"}
        let content = action
            .get("content")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or("动作缺少 content 字段（可能是 tool_calls 响应）")?
            .to_string();

        // 验证是否轮到该 actor
        let actor = self
            .actors
            .iter()
            .find(|a| a.id == actor_id)
            .ok_or(format!("未注册的 actor: {actor_id}"))?;

        if actor.role != self.cur_role {
            return Err(crate::error::EngineError(format!(
                "还没轮到 {:?} 发言，当前轮次: {:?}",
                actor.role, self.cur_role
            )));
        }

        // 写入历史
        self.history.push(HistoryEntry {
            actor_id: actor_id.to_string(),
            role: self.cur_role,
            content,
        });

        // 推进状态机
        match self.cur_role {
            DebateRole::Pro | DebateRole::Con => {
                self.round += 1;
                if self.round >= self.max_round {
                    self.cur_role = DebateRole::Judge;
                } else {
                    self.cur_role = match self.cur_role {
                        DebateRole::Pro => DebateRole::Con,
                        _ => DebateRole::Pro,
                    };
                }
            }
            DebateRole::Judge => {
                if self.opening_done {
                    // 裁判总结陈词 → 游戏结束
                    self.cur_role = DebateRole::Over;
                } else {
                    // 裁判开题 → 正方先发言
                    self.opening_done = true;
                    self.cur_role = DebateRole::Pro;
                }
            }
            DebateRole::Over => {}
        }

        let mut events = Vec::new();

        // 检查是否结束
        if self.cur_role == DebateRole::Over {
            self.finished = true;
            events.push(EngineEvent::GameOver);
            return Ok(events);
        }

        // 检查下一个 actor 是否是 AI
        if let Some(next_id) = self.current_actor() {
            if let Some(next_actor) = self.actors.iter().find(|a| a.id == next_id) {
                if next_actor.kind == "Ai" {
                    events.push(EngineEvent::TriggerAi(next_id));
                }
            }
        }

        Ok(events)
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "state",
            "game_type": self.game_type(),
            "room_id": self.room_id,
            "actors": self.actors,
            "active_actor": self.current_actor(),
            "cur_role": self.cur_role,
            "round": self.round,
            "max_round": self.max_round,
            "finished": self.finished,
            "opening_done": self.opening_done,
            "history": self.history,
        })
    }

    fn to_ai_prompt(&self, actor_id: &str) -> String {
        let mut safe_state = self.to_json_for_player(actor_id);
        if let Some(obj) = safe_state.as_object_mut() {
            let history = obj.remove("history").unwrap_or(serde_json::Value::Null);
            let actors = obj.remove("actors").unwrap_or(serde_json::Value::Null);

            let history_str = if let serde_json::Value::Array(arr) = history {
                arr.into_iter()
                    .filter_map(|v| serde_json::from_value::<HistoryEntry>(v).ok())
                    .map(|evt| {
                        let role_name = match evt.role {
                            DebateRole::Judge => "裁判",
                            DebateRole::Pro => "正方",
                            DebateRole::Con => "反方",
                            DebateRole::Over => "系统",
                        };
                        format!("[{} | {}] \"{}\"", role_name, evt.actor_id, evt.content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                "".to_string()
            };

            obj.insert(
                "your_id".to_string(),
                serde_json::Value::String(actor_id.to_string()),
            );

            // 为 Lincoln 游戏生成私人指引
            if let Some(actor) = self.actors.iter().find(|a| a.id == actor_id) {
                let role_instruction = match actor.role {
                    DebateRole::Judge => {
                        "你是【裁判】(Judge)。请给出辩题，听取双方论点后做出最终裁决。字数控制在300字以内。"
                    }
                    DebateRole::Pro => {
                        "你是激进的立论家【正方】(Pro)。请针对裁判给出的辩题，发表具有说服力的论点。字数控制在200字以内。"
                    }
                    DebateRole::Con => {
                        "你是沉稳的驳论家【反方】(Con)。请严密审视正方的发言，并进行针锋相对的反驳。字数控制在200字以内。"
                    }
                    DebateRole::Over => "游戏已结束。",
                };
                obj.insert(
                    "your_role_instruction".to_string(),
                    serde_json::Value::String(role_instruction.to_string()),
                );
            }

            return format!(
                "=== PUBLIC HISTORY ===\n{}\n\n=== ACTORS ===\n{}\n\n=== PRIVATE STATE ===\n{}",
                history_str,
                serde_json::to_string(&actors).unwrap_or_default(),
                serde_json::to_string(obj).unwrap_or_default()
            );
        }
        safe_state.to_string()
    }

    fn current_actor(&self) -> Option<String> {
        self.actors
            .iter()
            .find(|a| a.role == self.cur_role)
            .map(|a| a.id.clone())
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}
