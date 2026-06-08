use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::traits::{ActionKind, GameAction, GameEvent, GameRole, Payload, Playable, RoomState};
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize)]
pub enum DebateRole {
    Pro,
    Con,
    Judge,
    Over,
}
impl GameRole for DebateRole {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DebateAction {
    Speech { action_id: String, content: String },
}
impl GameAction for DebateAction {}
pub type DebateRoomState = RoomState<DebateRole, DebateAction>;

pub struct LincolnGame {
    pub max_round: usize,
    // pub first_role: DebateRole,
    pub round: usize,
    pub cur_role: DebateRole,
}
#[derive(Serialize)]
pub enum LincolnPayload {}
impl Payload for LincolnPayload {}
#[derive(Debug)]
pub enum LincolnErr {
    NotYourTurn,
    EmptyContent,
    NotActor,
    InvalidProtocol,
    SpeechTooLong { max: usize, current: usize },
}

#[derive(serde::Serialize, Debug)]
pub struct LincolnSnapshot {
    pub cur_role: DebateRole,
    pub round: usize,
    pub max_round: usize,
    pub history_logs: Vec<String>,
}

impl Playable<DebateRole, DebateAction, LincolnPayload, LincolnErr> for LincolnGame {
    fn parse_action(&self, actor_id: &str, raw_content: &str) -> Result<DebateAction, LincolnErr> {
        Ok(DebateAction::Speech {
            action_id: actor_id.to_string(),
            content: raw_content.to_string(),
        })
    }

    fn step(
        &mut self,
        state: &mut RoomState<DebateRole, DebateAction>,
        action: DebateAction,
    ) -> Result<Vec<crate::traits::GameEvent<DebateRole, LincolnPayload>>, LincolnErr> {
        let DebateAction::Speech { action_id, content } = action.clone();
        let Some(actor) = state.find_actor(&action_id) else {
            return Err(LincolnErr::NotActor);
        };
        if actor.role != self.cur_role {
            return Err(LincolnErr::NotYourTurn);
        }
        state.history.push(action);
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
                if self.round >= self.max_round {
                    self.cur_role = DebateRole::Over;
                } else {
                    self.cur_role = DebateRole::Pro;
                }
            }
            _ => {}
        }
        let mut events = Vec::new();

        events.push(GameEvent::Broadcast(format!("{}: {}", action_id, content)));

        if let Some(next_actor) = state.actors.iter().find(|a| a.role == self.cur_role) {
            if matches!(next_actor.kind, ActionKind::Ai) {
                events.push(GameEvent::TriggerAi(next_actor.id.clone()));
            }
        }

        if self.cur_role == DebateRole::Over {
            events.push(GameEvent::GameOver);
        }

        Ok(events)
    }

    fn get_snapshot(
        &self,
        state: &RoomState<DebateRole, DebateAction>,
        _role: &DebateRole, // 林肯辩论全公开，所以当前视角 role 暂时用不上，用下划线忽略警告
    ) -> String {
        let history_logs: Vec<String> = state
            .history
            .iter()
            .map(|action| {
                let DebateAction::Speech { action_id, content } = action;
                format!("{}: {}", action_id, content)
            })
            .collect();

        let snapshot = LincolnSnapshot {
            cur_role: self.cur_role,
            round: self.round,
            max_round: self.max_round,
            history_logs,
        };

        serde_json::to_string(&snapshot).unwrap()
    }
}
