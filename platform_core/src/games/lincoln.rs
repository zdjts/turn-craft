use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::traits::{ActionKind, GameAction, GameEvent, GameRole, Payload, Playable, RoomState};
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize)]
pub enum DebatRole {
    Pro,
    Con,
    Judge,
    Over,
}
impl GameRole for DebatRole {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DebatAction {
    Speech { action_id: String, content: String },
}
impl GameAction for DebatAction {}
pub type DebatRoomState = RoomState<DebatRole, DebatAction>;

pub struct LincolnGame {
    pub max_round: usize,
    // pub first_role: DebatRole,
    pub round: usize,
    pub cur_role: DebatRole,
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
    pub cur_role: DebatRole,
    pub round: usize,
    pub max_round: usize,
    pub history_logs: Vec<String>,
}

impl Playable<DebatRole, DebatAction, LincolnPayload, LincolnErr> for LincolnGame {
    fn parse_action(&self, raw_content: &str) -> Result<DebatAction, LincolnErr> {
        serde_json::from_str(raw_content).map_err(|_| LincolnErr::InvalidProtocol)
    }

    fn step(
        &mut self,
        state: &mut RoomState<DebatRole, DebatAction>,
        action: DebatAction,
    ) -> Result<Vec<crate::traits::GameEvent<DebatRole, LincolnPayload>>, LincolnErr> {
        let DebatAction::Speech { action_id, content } = action.clone();
        let Some(actor) = state.find_actor(&action_id) else {
            return Err(LincolnErr::NotActor);
        };
        if actor.role != self.cur_role {
            return Err(LincolnErr::NotYourTurn);
        }
        state.history.push(action);
        match self.cur_role {
            DebatRole::Pro | DebatRole::Con => {
                self.round += 1;
                if self.round >= self.max_round {
                    self.cur_role = DebatRole::Judge;
                } else {
                    self.cur_role = match self.cur_role {
                        DebatRole::Pro => DebatRole::Con,
                        _ => DebatRole::Pro,
                    };
                }
            }
            DebatRole::Judge => {
                self.cur_role = DebatRole::Over;
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

        if self.cur_role == DebatRole::Over {
            events.push(GameEvent::GameOver);
        }

        Ok(events)
    }

    fn get_snapshot(
        &self,
        state: &RoomState<DebatRole, DebatAction>,
        _role: &DebatRole, // 林肯辩论全公开，所以当前视角 role 暂时用不上，用下划线忽略警告
    ) -> String {
        let history_logs: Vec<String> = state
            .history
            .iter()
            .map(|action| {
                let DebatAction::Speech { action_id, content } = action;
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
