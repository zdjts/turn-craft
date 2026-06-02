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

impl<R: GameAction, A: GameAction> RoomState<R, A> {
    pub fn new(room_id: String, game_type: String) -> Self {
        Self {
            room_id,
            game_type,
            actors: Vec::new(),
            history: Vec::new(),
        }
    }
}
pub enum NextStep {
    Continue,
    Stop,
}

pub trait Playable<R: GameRole, A: ActionKind> {}
