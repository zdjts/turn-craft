use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::error::EngineError;
use crate::traits::{ActionKind, EngineEvent, GameEngine};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy, Serialize, Deserialize)]
pub enum WerewolfRole {
    Werewolf,
    Seer,
    Witch,
    Hunter,
    Villager,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WerewolfPlayer {
    pub id: String,
    pub kind: String, // "Human" | "Ai"
    pub role: WerewolfRole,
    pub is_alive: bool,
    pub can_shoot: bool, // For hunter
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Init,
    NightWolf,
    NightSeer,
    NightWitch,
    DayAnnounce,
    DaySpeech,
    DayVote,
    DayHunterShoot(String, String), // shooter_id, next_phase ("DaySpeech" | "NightWolf")
    GameOver(String),               // winner faction: "Wolves" | "Good"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEvent {
    pub day: usize,
    pub phase: String,
    pub actor_id: Option<String>,
    pub action_type: String,
    pub target: Option<String>,
    pub content: Option<String>,
    pub visibility: String, // "public", "wolves", "seer", "witch", "private"
}

#[derive(Serialize, Deserialize)]
pub struct WerewolfEngine {
    pub room_id: String,
    pub players: Vec<WerewolfPlayer>,
    pub phase: Phase,
    pub day: usize,
    pub history: Vec<HistoryEvent>,

    // Night temp state
    pub wolf_votes: HashMap<String, String>, // voter -> target
    #[serde(default)]
    pub wolf_chat_count: usize,
    pub night_kill_target: Option<String>,
    pub night_poison_target: Option<String>,
    pub witch_has_save: bool,
    pub witch_has_poison: bool,

    // Day temp state
    pub speakers: Vec<String>,
    pub current_speaker_idx: usize,
    pub day_votes: HashMap<String, Option<String>>, // voter -> target
    pub pk_players: Option<Vec<String>>,            // if tie
    pub last_dead: Vec<String>,                     // to determine start of speech
}

impl WerewolfEngine {
    pub fn new(room_id: String) -> Self {
        Self {
            room_id,
            players: Vec::new(),
            phase: Phase::Init,
            day: 1,
            history: Vec::new(),

            wolf_votes: HashMap::new(),
            wolf_chat_count: 0,
            night_kill_target: None,
            night_poison_target: None,
            witch_has_save: true,
            witch_has_poison: true,

            speakers: Vec::new(),
            current_speaker_idx: 0,
            day_votes: HashMap::new(),
            pk_players: None,
            last_dead: Vec::new(),
        }
    }

    pub fn add_actor(&mut self, id: String, kind: ActionKind, role: WerewolfRole) {
        self.players.push(WerewolfPlayer {
            id,
            kind: match kind {
                ActionKind::Ai => "Ai".to_string(),
                ActionKind::Human => "Human".to_string(),
            },
            role,
            is_alive: true,
            can_shoot: role == WerewolfRole::Hunter,
        });
    }

    pub fn start(&mut self) -> Vec<EngineEvent> {
        self.phase = Phase::NightWolf;
        self.wolf_chat_count = 0;
        self.history.push(HistoryEvent {
            day: self.day,
            phase: "system".to_string(),
            actor_id: None,
            action_type: "start_game".to_string(),
            target: None,
            content: Some("游戏开始，天黑请闭眼。狼人请睁眼。".to_string()),
            visibility: "public".to_string(),
        });
        self.trigger_next()
    }

    fn check_win(&self) -> Option<String> {
        let alive_wolves = self
            .players
            .iter()
            .filter(|p| p.is_alive && p.role == WerewolfRole::Werewolf)
            .count();
        if alive_wolves == 0 {
            return Some("Good".to_string());
        }
        let alive_good = self
            .players
            .iter()
            .filter(|p| p.is_alive && p.role != WerewolfRole::Werewolf)
            .count();
        if alive_good == 0 {
            return Some("Wolves".to_string());
        }
        None
    }

    fn get_alive_players(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|p| p.is_alive)
            .map(|p| p.id.clone())
            .collect()
    }

    fn die(&mut self, target: &str, is_poison: bool) {
        if let Some(p) = self.players.iter_mut().find(|p| p.id == target) {
            p.is_alive = false;
            if is_poison {
                p.can_shoot = false; // poison denies shoot
            }
            self.last_dead.push(target.to_string());
        }
    }

    fn next_night_phase(&mut self) -> Vec<EngineEvent> {
        if let Some(winner) = self.check_win() {
            self.phase = Phase::GameOver(winner);
            return vec![EngineEvent::GameOver];
        }

        match self.phase {
            Phase::NightWolf => {
                let seer = self.players.iter().find(|p| p.role == WerewolfRole::Seer);
                if seer.map_or(false, |p| p.is_alive) {
                    self.phase = Phase::NightSeer;
                } else {
                    self.phase = Phase::NightSeer;
                    return self.next_night_phase();
                }
            }
            Phase::NightSeer => {
                let witch = self.players.iter().find(|p| p.role == WerewolfRole::Witch);
                if witch.map_or(false, |p| p.is_alive) {
                    self.phase = Phase::NightWitch;
                } else {
                    self.phase = Phase::NightWitch;
                    return self.next_night_phase();
                }
            }
            Phase::NightWitch => {
                self.phase = Phase::DayAnnounce;
                return self.resolve_night();
            }
            _ => {}
        }
        self.trigger_next()
    }

    fn resolve_night(&mut self) -> Vec<EngineEvent> {
        self.last_dead.clear();

        let killed = self.night_kill_target.clone();
        let poisoned = self.night_poison_target.clone();

        let mut dead_this_night = Vec::new();

        if let Some(target) = killed {
            self.die(&target, false);
            dead_this_night.push(target);
        }

        if let Some(target) = poisoned {
            if !dead_this_night.contains(&target) {
                self.die(&target, true);
                dead_this_night.push(target);
            }
        }

        let msg = if dead_this_night.is_empty() {
            "昨夜平安夜。".to_string()
        } else {
            format!("昨夜死亡的玩家是：{}", dead_this_night.join(", "))
        };

        self.history.push(HistoryEvent {
            day: self.day,
            phase: "DayAnnounce".to_string(),
            actor_id: None,
            action_type: "announce".to_string(),
            target: None,
            content: Some(msg),
            visibility: "public".to_string(),
        });

        if let Some(winner) = self.check_win() {
            self.phase = Phase::GameOver(winner);
            return vec![EngineEvent::GameOver];
        }

        self.speakers = self.get_alive_players();
        self.current_speaker_idx = 0;

        if self.check_hunter_shoot("DaySpeech") {
            return self.trigger_next();
        }

        self.phase = Phase::DaySpeech;

        self.trigger_next()
    }

    fn trigger_next(&self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        match &self.phase {
            Phase::NightWolf => {
                for p in &self.players {
                    if p.is_alive && p.role == WerewolfRole::Werewolf && p.kind == "Ai" {
                        events.push(EngineEvent::TriggerAi(p.id.clone()));
                    }
                }
            }
            Phase::NightSeer => {
                if let Some(p) = self
                    .players
                    .iter()
                    .find(|p| p.is_alive && p.role == WerewolfRole::Seer && p.kind == "Ai")
                {
                    events.push(EngineEvent::TriggerAi(p.id.clone()));
                }
            }
            Phase::NightWitch => {
                if let Some(p) = self
                    .players
                    .iter()
                    .find(|p| p.is_alive && p.role == WerewolfRole::Witch && p.kind == "Ai")
                {
                    events.push(EngineEvent::TriggerAi(p.id.clone()));
                }
            }
            Phase::DaySpeech => {
                if self.current_speaker_idx < self.speakers.len() {
                    let id = &self.speakers[self.current_speaker_idx];
                    if let Some(p) = self.players.iter().find(|p| p.id == *id && p.kind == "Ai") {
                        events.push(EngineEvent::TriggerAi(p.id.clone()));
                    }
                }
            }
            Phase::DayVote => {
                for p in &self.players {
                    if p.is_alive && p.kind == "Ai" {
                        if let Some(pk) = &self.pk_players {
                            if pk.contains(&p.id) {
                                events.push(EngineEvent::TriggerAi(p.id.clone()));
                            } else {
                                events.push(EngineEvent::TriggerAi(p.id.clone()));
                            }
                        } else {
                            events.push(EngineEvent::TriggerAi(p.id.clone()));
                        }
                    }
                }
            }
            Phase::DayHunterShoot(id, _) => {
                if let Some(p) = self.players.iter().find(|p| p.id == *id && p.kind == "Ai") {
                    events.push(EngineEvent::TriggerAi(p.id.clone()));
                }
            }
            Phase::GameOver(_) => {
                events.push(EngineEvent::GameOver);
            }
            _ => {}
        }
        events
    }

    fn check_hunter_shoot(&mut self, next_phase: &str) -> bool {
        for dead_id in &self.last_dead {
            if let Some(p) = self.players.iter().find(|p| p.id == *dead_id) {
                if p.role == WerewolfRole::Hunter && p.can_shoot {
                    self.phase = Phase::DayHunterShoot(p.id.clone(), next_phase.to_string());
                    return true;
                }
            }
        }
        false
    }
}

impl GameEngine for WerewolfEngine {
    fn game_type(&self) -> &str {
        "werewolf"
    }

    fn step(&mut self, actor_id: &str, action: Value) -> Result<Vec<EngineEvent>, EngineError> {
        let parsed_action =
            if let Some(tool_calls) = action.get("tool_calls").and_then(|v| v.as_array()) {
                if let Some(first_call) = tool_calls.first() {
                    let args_str = first_call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                        .unwrap_or("{}");
                    serde_json::from_str::<Value>(args_str).unwrap_or(action.clone())
                } else {
                    action.clone()
                }
            } else {
                action.clone()
            };

        let action_type = parsed_action
            .get("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if action_type == "start" && self.phase == Phase::Init {
            self.phase = Phase::NightWolf;
            self.wolf_chat_count = 0;
            self.history.push(HistoryEvent {
                day: self.day,
                phase: "Init".to_string(),
                actor_id: Some(actor_id.to_string()),
                action_type: "start".to_string(),
                target: None,
                content: Some("游戏开始，天黑请闭眼。狼人请行动。".to_string()),
                visibility: "public".to_string(),
            });
            return Ok(self.trigger_next());
        }

        let target = parsed_action
            .get("target")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let content = parsed_action
            .get("content")
            .or_else(|| action.get("content"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let actor = self
            .players
            .iter()
            .find(|p| p.id == actor_id)
            .cloned()
            .ok_or(EngineError("Unknown actor".into()))?;

        if !actor.is_alive
            && !matches!(self.phase, Phase::DayHunterShoot(ref id, _) if id == actor_id)
        {
            return Err(EngineError("Dead players cannot act".into()));
        }

        if action_type == "explode" && actor.role == WerewolfRole::Werewolf {
            if matches!(self.phase, Phase::DaySpeech | Phase::DayVote) {
                self.history.push(HistoryEvent {
                    day: self.day,
                    phase: "Day".to_string(),
                    actor_id: Some(actor_id.to_string()),
                    action_type: "wolf_explode".to_string(),
                    target: None,
                    content: Some("自爆".to_string()),
                    visibility: "public".to_string(),
                });

                self.die(actor_id, false);

                if let Some(winner) = self.check_win() {
                    self.phase = Phase::GameOver(winner);
                    return Ok(vec![EngineEvent::GameOver]);
                }

                if self.check_hunter_shoot("DaySpeech") {
                    return Ok(self.trigger_next());
                } else {
                    self.day += 1;
                    self.phase = Phase::NightWolf;
                    self.wolf_chat_count = 0;
                    return Ok(self.trigger_next());
                }
            }
        }

        let current_phase = self.phase.clone();
        match current_phase {
            Phase::Init => {
                // Should not happen since start is handled above, but if it does, return an error
                return Err(EngineError("游戏还未开始，或者已经被开始".into()));
            }
            Phase::NightWolf => {
                if actor.role != WerewolfRole::Werewolf {
                    return Err(EngineError("Not your turn".into()));
                }

                let alive_wolves = self
                    .players
                    .iter()
                    .filter(|p| p.is_alive && p.role == WerewolfRole::Werewolf)
                    .count();

                if action_type == "speak" {
                    if alive_wolves <= 1 {
                        return Err(EngineError("只剩一匹狼时不需要沟通，请直接选择击杀目标".into()));
                    }
                    if let Some(msg) = content {
                        self.history.push(HistoryEvent {
                            day: self.day,
                            phase: "NightWolf".to_string(),
                            actor_id: Some(actor_id.to_string()),
                            action_type: "speak".to_string(),
                            target: None,
                            content: Some(msg),
                            visibility: "wolves".to_string(),
                        });
                        self.wolf_chat_count += 1;
                    }
                } else if action_type == "kill" {
                    let t = target.ok_or(EngineError("Missing target".into()))?;
                    self.wolf_votes.insert(actor_id.to_string(), t);

                    if let Some(msg) = content {
                        self.history.push(HistoryEvent {
                            day: self.day,
                            phase: "NightWolf".to_string(),
                            actor_id: Some(actor_id.to_string()),
                            action_type: "speak".to_string(),
                            target: None,
                            content: Some(msg),
                            visibility: "wolves".to_string(),
                        });
                    }
                    self.wolf_chat_count += 1;
                } else {
                    return Err(EngineError("Invalid action".into()));
                }

                let max_wolf_chat = 15;
                let mut should_advance = false;
                let mut final_target = None;

                if self.wolf_votes.len() == alive_wolves {
                    // Check consensus
                    let mut targets: Vec<&String> = self.wolf_votes.values().collect();
                    targets.sort();
                    targets.dedup();
                    if targets.len() == 1 {
                        should_advance = true;
                        final_target = Some(targets[0].clone());
                    }
                }

                if !should_advance && self.wolf_chat_count >= max_wolf_chat {
                    should_advance = true;
                    use rand::seq::IteratorRandom;
                    let mut rng = rand::thread_rng();
                    if !self.wolf_votes.is_empty() {
                        final_target = self.wolf_votes.values().choose(&mut rng).cloned();
                    } else {
                        final_target = self.players.iter()
                            .filter(|p| p.is_alive)
                            .map(|p| p.id.clone())
                            .choose(&mut rng);
                    }
                }

                if should_advance {
                    self.night_kill_target = final_target.clone();

                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "NightWolf".to_string(),
                        actor_id: None,
                        action_type: "wolf_kill".to_string(),
                        target: final_target,
                        content: None,
                        visibility: "wolves".to_string(),
                    });

                    self.wolf_votes.clear();
                    self.wolf_chat_count = 0;
                    return Ok(self.next_night_phase());
                } else {
                    let mut events = Vec::new();
                    for p in &self.players {
                        if p.is_alive && p.role == WerewolfRole::Werewolf && p.kind == "Ai" && p.id != actor_id {
                            events.push(EngineEvent::TriggerAi(p.id.clone()));
                        }
                    }
                    return Ok(events);
                }
            }
            Phase::NightSeer => {
                if actor.role != WerewolfRole::Seer {
                    return Err(EngineError("Not your turn".into()));
                }
                if action_type != "check" && action_type != "skip" {
                    return Err(EngineError("Invalid action".into()));
                }
                if action_type == "check" {
                    let t = target.ok_or(EngineError("Missing target".into()))?;
                    let target_role = self.players.iter().find(|p| p.id == t).map(|p| p.role);
                    let is_wolf = target_role == Some(WerewolfRole::Werewolf);

                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "NightSeer".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "seer_check".to_string(),
                        target: Some(t),
                        content: Some(if is_wolf {
                            "狼人".to_string()
                        } else {
                            "好人".to_string()
                        }),
                        visibility: "seer".to_string(),
                    });
                }
                return Ok(self.next_night_phase());
            }
            Phase::NightWitch => {
                if actor.role != WerewolfRole::Witch {
                    return Err(EngineError("Not your turn".into()));
                }
                if action_type == "save" {
                    if !self.witch_has_save {
                        return Err(EngineError("No save potion".into()));
                    }
                    let saved_target = self.night_kill_target.clone();
                    self.night_kill_target = None;
                    self.witch_has_save = false;
                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "NightWitch".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "witch_save".to_string(),
                        target: saved_target,
                        content: None,
                        visibility: "witch".to_string(),
                    });
                } else if action_type == "poison" {
                    if !self.witch_has_poison {
                        return Err(EngineError("No poison potion".into()));
                    }
                    let t = target.ok_or(EngineError("Missing target".into()))?;
                    self.night_poison_target = Some(t.clone());
                    self.witch_has_poison = false;
                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "NightWitch".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "witch_poison".to_string(),
                        target: Some(t),
                        content: None,
                        visibility: "witch".to_string(),
                    });
                } else if action_type == "skip" {
                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "NightWitch".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "witch_skip".to_string(),
                        target: None,
                        content: None,
                        visibility: "witch".to_string(),
                    });
                } else {
                    return Err(EngineError("Invalid action".into()));
                }
                return Ok(self.next_night_phase());
            }
            Phase::DaySpeech => {
                if self.speakers.get(self.current_speaker_idx) != Some(&actor_id.to_string()) {
                    return Err(EngineError("Not your turn to speak".into()));
                }
                if action_type != "speak" && action_type != "speech" {
                    return Err(EngineError("You must speak".into()));
                }
                self.history.push(HistoryEvent {
                    day: self.day,
                    phase: "DaySpeech".to_string(),
                    actor_id: Some(actor_id.to_string()),
                    action_type: "speak".to_string(),
                    target: None,
                    content: content.clone(),
                    visibility: "public".to_string(),
                });

                self.current_speaker_idx += 1;
                if self.current_speaker_idx >= self.speakers.len() {
                    self.phase = Phase::DayVote;
                    self.day_votes.clear();
                }
                return Ok(self.trigger_next());
            }
            Phase::DayVote => {
                if action_type != "vote" && action_type != "skip" {
                    return Err(EngineError("Must vote or skip".into()));
                }
                self.day_votes.insert(
                    actor_id.to_string(),
                    if action_type == "vote" {
                        target.clone()
                    } else {
                        None
                    },
                );

                let alive_count = self.players.iter().filter(|p| p.is_alive).count();
                if self.day_votes.len() == alive_count {
                    let mut vote_counts = HashMap::new();
                    for t in self.day_votes.values().flatten() {
                        *vote_counts.entry(t.clone()).or_insert(0) += 1;
                    }

                    let mut vote_details = Vec::new();
                    for (voter, target) in &self.day_votes {
                        if let Some(t) = target {
                            vote_details.push(format!("{} 投票给 {}", voter, t));
                        } else {
                            vote_details.push(format!("{} 弃权", voter));
                        }
                    }
                    let vote_str = vote_details.join("，");

                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "DayVote".to_string(),
                        actor_id: None,
                        action_type: "vote_result".to_string(),
                        target: None,
                        content: Some(format!("投票明细：{}", vote_str)),
                        visibility: "public".to_string(),
                    });

                    let mut max_votes = 0;
                    let mut max_targets = Vec::new();
                    for (t, count) in vote_counts {
                        if count > max_votes {
                            max_votes = count;
                            max_targets = vec![t];
                        } else if count == max_votes {
                            max_targets.push(t);
                        }
                    }

                    if max_targets.len() == 1 {
                        let out_id = max_targets[0].clone();
                        self.history.push(HistoryEvent {
                            day: self.day,
                            phase: "DayVote".to_string(),
                            actor_id: None,
                            action_type: "voted_out".to_string(),
                            target: Some(out_id.clone()),
                            content: None,
                            visibility: "public".to_string(),
                        });
                        self.last_dead.clear();
                        self.die(&out_id, false);
                        self.pk_players = None;

                        if let Some(winner) = self.check_win() {
                            self.phase = Phase::GameOver(winner);
                            return Ok(vec![EngineEvent::GameOver]);
                        }

                        if self.check_hunter_shoot("NightWolf") {
                            return Ok(self.trigger_next());
                        }
                    } else if max_targets.len() > 1 {
                        if self.pk_players.is_some() {
                            self.history.push(HistoryEvent {
                                day: self.day,
                                phase: "DayVote".to_string(),
                                actor_id: None,
                                action_type: "tie_no_out".to_string(),
                                target: None,
                                content: None,
                                visibility: "public".to_string(),
                            });
                            self.pk_players = None;
                        } else {
                            self.pk_players = Some(max_targets.clone());
                            self.speakers = max_targets.clone();
                            self.current_speaker_idx = 0;
                            self.phase = Phase::DaySpeech;
                            self.history.push(HistoryEvent {
                                day: self.day,
                                phase: "DayVote".to_string(),
                                actor_id: None,
                                action_type: "pk_start".to_string(),
                                target: None,
                                content: Some(max_targets.join(", ")),
                                visibility: "public".to_string(),
                            });
                            return Ok(self.trigger_next());
                        }
                    } else {
                        self.pk_players = None;
                    }

                    self.day += 1;
                    self.phase = Phase::NightWolf;
                    self.wolf_chat_count = 0;
                    return Ok(self.trigger_next());
                }
            }
            Phase::DayHunterShoot(shooter, next_phase) => {
                if shooter != actor_id {
                    return Err(EngineError("Not your turn to shoot".into()));
                }
                if action_type == "shoot" {
                    let t = target.ok_or(EngineError("Missing target".into()))?;
                    self.last_dead.clear();
                    self.die(&t, false);
                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "DayHunterShoot".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "hunter_shoot".to_string(),
                        target: Some(t),
                        content: None,
                        visibility: "public".to_string(),
                    });

                    if let Some(p) = self.players.iter_mut().find(|p| p.id == *actor_id) {
                        p.can_shoot = false;
                    }

                    if let Some(winner) = self.check_win() {
                        self.phase = Phase::GameOver(winner);
                        return Ok(vec![EngineEvent::GameOver]);
                    }
                } else if action_type == "skip" {
                    self.history.push(HistoryEvent {
                        day: self.day,
                        phase: "DayHunterShoot".to_string(),
                        actor_id: Some(actor_id.to_string()),
                        action_type: "hunter_skip".to_string(),
                        target: None,
                        content: None,
                        visibility: "public".to_string(),
                    });
                } else {
                    return Err(EngineError("Invalid action".into()));
                }

                if next_phase == "DaySpeech" {
                    self.phase = Phase::DaySpeech;
                    self.speakers = self.get_alive_players();
                    self.current_speaker_idx = 0;
                } else {
                    self.day += 1;
                    self.phase = Phase::NightWolf;
                    self.wolf_chat_count = 0;
                }
                return Ok(self.trigger_next());
            }
            Phase::GameOver(_) => {
                return Err(EngineError("Game is over".into()));
            }
            _ => {}
        }
        Ok(vec![])
    }

    fn to_json(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(obj) = v.as_object_mut() {
            obj.insert("finished".to_string(), Value::Bool(self.is_finished()));
            obj.insert(
                "game_type".to_string(),
                Value::String(self.game_type().to_string()),
            );
            obj.insert(
                "active_actor".to_string(),
                serde_json::to_value(self.current_actor()).unwrap_or(Value::Null),
            );
            let phase_hint = match &self.phase {
                Phase::NightWolf => "现在是狼人行动阶段，请选择要击杀的目标".to_string(),
                Phase::NightSeer => "现在是预言家行动阶段，请选择要查验的玩家或跳过".to_string(),
                Phase::NightWitch => "现在是女巫行动阶段，请选择救人或毒人或跳过".to_string(),
                Phase::DaySpeech => "现在是发言阶段，请发言分析局势".to_string(),
                Phase::DayVote => {
                    "发言阶段已结束，现在是投票阶段，请选择要投票的玩家或跳过".to_string()
                }
                Phase::DayHunterShoot(..) => "你是猎人，请选择要开枪的玩家或跳过".to_string(),
                _ => String::new(),
            };
            if !phase_hint.is_empty() {
                obj.insert("phase_hint".to_string(), Value::String(phase_hint));
            }
        }
        v
    }

    fn to_json_for_player(&self, actor_id: &str) -> Value {
        let mut v = self.to_json();
        let role = self
            .players
            .iter()
            .find(|p| p.id == actor_id)
            .map(|p| p.role);

        let is_finished = self.is_finished();

        if let Some(history) = v.get_mut("history").and_then(|h| h.as_array_mut()) {
            history.retain(|evt| {
                let vis = evt
                    .get("visibility")
                    .and_then(|v| v.as_str())
                    .unwrap_or("public");
                match vis {
                    "public" => true,
                    "wolves" => role == Some(WerewolfRole::Werewolf),
                    "seer" => role == Some(WerewolfRole::Seer),
                    "witch" => role == Some(WerewolfRole::Witch),
                    "private" => evt.get("actor_id").and_then(|a| a.as_str()) == Some(actor_id),
                    _ => false,
                }
            });
        }

        if let Some(obj) = v.as_object_mut() {
            obj.insert("your_id".to_string(), Value::String(actor_id.to_string()));

            if !is_finished {
                if role != Some(WerewolfRole::Witch) {
                    obj.remove("witch_has_save");
                    obj.remove("witch_has_poison");
                    obj.remove("night_poison_target");
                }
                if role != Some(WerewolfRole::Werewolf) {
                    obj.remove("wolf_votes");
                }
                if role != Some(WerewolfRole::Witch) && role != Some(WerewolfRole::Werewolf) {
                    obj.remove("night_kill_target");
                }
            }
        }

        if let Some(players) = v.get_mut("players").and_then(|p| p.as_array_mut()) {
            for p in players.iter_mut() {
                if let Some(obj) = p.as_object_mut() {
                    let id = obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if id != actor_id && !is_finished {
                        obj.remove("can_shoot");

                        let is_wolf = role == Some(WerewolfRole::Werewolf);
                        let target_role = obj.get("role").and_then(|v| v.as_str()).unwrap_or("");

                        if !(is_wolf && target_role == "Werewolf") {
                            obj.remove("role");
                        }
                    }
                }
            }
        }

        v
    }

    fn to_ai_prompt(&self, actor_id: &str) -> String {
        let is_finished = self.is_finished();
        let role_opt = self
            .players
            .iter()
            .find(|p| p.id == actor_id)
            .map(|p| p.role.clone());

        // 1. 分离 public 和 private history
        let mut public_history = Vec::new();
        let mut private_history = Vec::new();

        for evt in &self.history {
            if evt.visibility == "public" {
                public_history.push(evt);
            } else {
                let is_visible = is_finished
                    || match evt.visibility.as_str() {
                        "wolves" => role_opt == Some(WerewolfRole::Werewolf),
                        "seer" => role_opt == Some(WerewolfRole::Seer),
                        "witch" => role_opt == Some(WerewolfRole::Witch),
                        "private" => evt.actor_id.as_deref() == Some(actor_id),
                        _ => false,
                    };
                if is_visible {
                    private_history.push(evt);
                }
            }
        }

        // 2. 构造专属人设 (Role instruction)
        let role_instruction = match role_opt {
            Some(WerewolfRole::Werewolf) => {
                "你是【狼人】。每晚你可以使用`speak`动作和另一只狼人队友在隐秘的狼队频道进行战术交流。商量好后，所有狼人必须使用`kill`动作投票给*同一个人*才能达成一致进行击杀（或者你们也可以在一次`kill`中顺便使用`content`字段附带交流信息）。如果长时间无法达成一致，系统将强制随机选定一人。白天如果局势不利，你可以选择'自爆'（直接结束白天进入黑夜）。请隐藏好自己的身份，发言时伪装成好人。"
            }
            Some(WerewolfRole::Seer) => {
                "你是【预言家】。每晚你可以查验一名玩家的身份（好人或狼人）。白天你需要通过发言带领好人阵营投票出狼人。"
            }
            Some(WerewolfRole::Witch) => {
                "你是【女巫】。你有一瓶解药和一瓶毒药，解药可救活今晚被狼杀的人，毒药可毒杀任意一人。每晚你只能使用其中一瓶。"
            }
            Some(WerewolfRole::Hunter) => {
                "你是【猎人】。如果你被狼人杀害或白天被投票出局，你可以开枪带走任意一名存活玩家。但如果是被女巫毒死，你将无法开枪。"
            }
            Some(WerewolfRole::Villager) => {
                "你是【平民】。你没有任何夜间技能，只能在白天认真听取大家发言，分辨谁是狼人并投票将其出局。"
            }
            None => "你是旁观者。",
        };

        // 3. 构建 private state
        let mut safe_state = self.to_json_for_player(actor_id);
        if let Some(obj) = safe_state.as_object_mut() {
            obj.remove("history"); // 已经被提取出来，不需要重复
            obj.insert(
                "your_role_instruction".to_string(),
                serde_json::Value::String(role_instruction.to_string()),
            );
            if let Some(ref r) = role_opt {
                obj.insert(
                    "your_role".to_string(),
                    serde_json::Value::String(format!("{:?}", r)),
                );
            }
        }

        // 4. 剧本化格式化输出历史
        let format_evt = |evt: &HistoryEvent| -> String {
            let actor = evt.actor_id.as_deref().unwrap_or("(系统)");
            let target = evt.target.as_deref().unwrap_or("");
            let content = evt.content.as_deref().unwrap_or("");
            let desc = match evt.action_type.as_str() {
                "start" => format!("游戏开始：{}", content),
                "speak" => format!("发言：\"{}\"", content),
                "vote" => format!("投票给 {}", if target.is_empty() { "弃权" } else { target }),
                "wolf_kill" => format!("投票击杀 {}", target),
                "seer_check" => format!("查验了 {} 的身份，结果是：{}", target, content),
                "witch_save" => format!("使用解药救活了 {}", target),
                "witch_poison" => format!("使用毒药毒杀了 {}", target),
                "witch_skip" => "放弃使用药水".to_string(),
                "hunter_shoot" => format!("开枪带走了 {}", target),
                "hunter_skip" => "放弃开枪".to_string(),
                "announce" => format!("公告：{}", content),
                "wolf_explode" => "自爆了！".to_string(),
                _ => format!("执行了 {}", evt.action_type),
            };
            format!("[第{}天 | {}] {} {}", evt.day, evt.phase, actor, desc)
        };

        let public_str = public_history
            .into_iter()
            .map(format_evt)
            .collect::<Vec<_>>()
            .join("\n");
        let private_str = private_history
            .into_iter()
            .map(format_evt)
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "=== PUBLIC HISTORY ===\n{}\n\n=== PRIVATE HISTORY ===\n{}\n\n=== PRIVATE STATE ===\n{}",
            public_str,
            private_str,
            serde_json::to_string(&safe_state).unwrap_or_default()
        )
    }

    fn current_actor(&self) -> Option<String> {
        match &self.phase {
            Phase::NightWolf => self
                .players
                .iter()
                .find(|p| p.is_alive && p.role == WerewolfRole::Werewolf)
                .map(|p| p.id.clone()),
            Phase::NightSeer => self
                .players
                .iter()
                .find(|p| p.is_alive && p.role == WerewolfRole::Seer)
                .map(|p| p.id.clone()),
            Phase::NightWitch => self
                .players
                .iter()
                .find(|p| p.is_alive && p.role == WerewolfRole::Witch)
                .map(|p| p.id.clone()),
            Phase::DaySpeech => {
                if self.current_speaker_idx < self.speakers.len() {
                    Some(self.speakers[self.current_speaker_idx].clone())
                } else {
                    None
                }
            }
            Phase::DayHunterShoot(id, _) => Some(id.clone()),
            _ => None,
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.phase, Phase::GameOver(_))
    }

    fn tools(&self) -> Option<serde_json::Value> {
        let (action_enum, required) = match &self.phase {
            Phase::NightWolf => (vec!["kill"], vec!["action_type", "target"]),
            Phase::NightSeer => (vec!["check", "skip"], vec!["action_type"]),
            Phase::NightWitch => (vec!["save", "poison", "skip"], vec!["action_type"]),
            Phase::DaySpeech => (vec!["speak"], vec!["action_type", "content"]),
            Phase::DayVote => (vec!["vote", "skip"], vec!["action_type"]),
            Phase::DayHunterShoot(..) => (vec!["shoot", "skip"], vec!["action_type"]),
            // Init / DayAnnounce / GameOver — no AI action needed
            _ => return None,
        };

        Some(serde_json::json!([
            {
                "type": "function",
                "function": {
                    "name": "werewolf_action",
                    "description": "执行狼人杀动作",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "action_type": {
                                "type": "string",
                                "enum": action_enum,
                                "description": "要执行的动作"
                            },
                            "target": {
                                "type": "string",
                                "description": "目标玩家ID（如果有目标）"
                            },
                            "content": {
                                "type": "string",
                                "description": "发言内容或行动说明"
                            }
                        },
                        "required": required
                    }
                }
            }
        ]))
    }
}
