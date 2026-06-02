use crate::traits::{GameAction, GameRole, NextStep, Playable, RoomState};
#[derive(Clone, PartialEq, Eq)]
pub enum DebatRole {
    Pro,
    Con,
    Judge,
}
impl GameRole for DebatRole {}

pub enum DebatAction {
    Speech { action_id: String, content: String },
}
impl GameAction for DebatAction {}
pub type DebatRoomState = RoomState<DebatRole, DebatAction>;

pub struct LincolnGame {
    pub max_round: usize,
    // pub first_role: DebatRole,
    pub cur_role: DebatRole,
}
impl Playable<DebatRole, DebatAction> for LincolnGame {
    fn validata_action(&self, state: &DebatRoomState, action: &DebatAction) -> bool {
        let DebatAction::Speech { action_id, .. } = action;
        if let Some(actor) = state.find_actor(action_id) {
            if actor.role == self.cur_role {
                return true;
            }
        }
        false
    }

    fn apply_action(&mut self, state: &mut DebatRoomState, action: DebatAction) {
        if state.history.len() >= self.max_round {
            self.cur_role = DebatRole::Judge;
            return;
        }
        match self.cur_role {
            DebatRole::Pro => self.cur_role = DebatRole::Con,
            DebatRole::Con => self.cur_role = DebatRole::Pro,
            _ => {}
        }
        state.history.push(action);

        todo!()
    }

    fn check_next_step(&self) -> crate::traits::NextStep {
        match self.cur_role {
            DebatRole::Judge => NextStep::Over,
            _ => NextStep::Continue,
        }
    }
}
