use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::traits::{ActionKind, EngineEvent, GameEngine};

#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
pub enum DebateRole {
    Pro,
    Con,
    Judge,
    Over,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LincolnActor {
    pub id: String,
    pub kind: String, // "Human" | "Ai"
    pub role: DebateRole,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub actor_id: String,
    pub role: DebateRole,
    pub content: String,
}

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

    fn step(&mut self, actor_id: &str, action: serde_json::Value) -> Result<Vec<EngineEvent>, String> {
        let content = action
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("动作缺少 content 字段")?
            .to_string();

        if content.is_empty() {
            return Err("发言内容不能为空".to_string());
        }

        // 验证是否轮到该 actor
        let actor = self
            .actors
            .iter()
            .find(|a| a.id == actor_id)
            .ok_or(format!("未注册的 actor: {actor_id}"))?;

        if actor.role != self.cur_role {
            return Err(format!(
                "还没轮到 {:?} 发言，当前轮次: {:?}",
                actor.role, self.cur_role
            ));
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
            "game_type": self.game_type(),
            "room_id": self.room_id,
            "actors": self.actors,
            "active_actor": self.current_actor(),
            "round": self.round,
            "max_round": self.max_round,
            "finished": self.finished,
            "history": self.history,
        })
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
