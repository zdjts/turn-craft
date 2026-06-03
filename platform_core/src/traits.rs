#[derive(Debug, Clone)]
pub enum ActionKind {
    Ai,
    Human,
}

pub trait GameRole: Clone + Send + Sync {}
pub trait GameAction: Send + Sync {}

// pub #[derive(Debug)]
pub struct Actor<R: GameRole> {
    pub id: String,
    pub kind: ActionKind,
    pub role: R,
}

pub struct RoomState<R: GameRole, A: GameAction> {
    room_id: String,
    game_type: String,
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
pub enum NextStep {
    Continue,
    Over,
}

pub trait Playable<R: GameRole, A: GameAction> {
    fn validata_action(&self, state: &RoomState<R, A>, action: &A) -> bool;
    fn apply_action(&mut self, state: &mut RoomState<R, A>, action: A);
    fn check_next_step(&self) -> NextStep;
}
