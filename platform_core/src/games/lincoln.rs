use crate::traits::{GameAction, GameRole, Playable, RoomState};
#[derive(Clone, PartialEq, Eq, Debug, Copy)]
pub enum DebatRole {
    Pro,
    Con,
    Judge,
    Over,
}
impl GameRole for DebatRole {}

#[derive(Clone)]
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
impl Playable<DebatRole, DebatAction> for LincolnGame {
    fn set_next_role(&mut self) {
        if (self.cur_role == DebatRole::Pro || self.cur_role == DebatRole::Con)
            && self.round >= self.max_round
        {
            self.cur_role = DebatRole::Judge;
            return;
        }
        self.cur_role = match self.cur_role {
            DebatRole::Pro => DebatRole::Con,
            DebatRole::Con => DebatRole::Pro,
            _ => DebatRole::Over,
        }
    }
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
        self.round += 1;

        todo!()
    }

    fn get_next_role(&self) -> DebatRole {
        if (self.cur_role == DebatRole::Pro || self.cur_role == DebatRole::Con)
            && self.round >= self.max_round
        {
            return DebatRole::Judge;
        }

        match self.cur_role {
            DebatRole::Pro => DebatRole::Con,
            DebatRole::Con => DebatRole::Pro,
            _ => DebatRole::Over,
        }
    }
}
