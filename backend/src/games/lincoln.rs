use std::collections::HashMap;

use platform_core::{
    games::lincoln::{
        DebateAction, DebateRole, LincolnErr, LincolnGame, LincolnPayload,
    },
    traits::{ActionKind, Actor, Playable, RoomState},
};

use crate::ai::env::AiConfig;

pub fn create_lincoln(
    room_id: &str,
    player_id: &str,
    max_round: usize,
) -> (
    Box<dyn Playable<DebateRole, DebateAction, LincolnPayload, LincolnErr>>,
    RoomState<DebateRole, DebateAction>,
    HashMap<String, AiConfig>,
) {
    let engine = LincolnGame {
        max_round,
        round: 0,
        cur_role: DebateRole::Judge, // 🌟 默认一上来是裁判发言（开场白）
    };

    let actors = vec![
        Actor {
            id: player_id.to_string(),
            kind: ActionKind::Human,
            role: DebateRole::Judge, // 🌟 真人担任裁判
        },
        Actor {
            id: "ai_pro_debater".to_string(),
            kind: ActionKind::Ai,
            role: DebateRole::Pro, // 🌟 AI 1 担任正方
        },
        Actor {
            id: "ai_con_debater".to_string(),
            kind: ActionKind::Ai,
            role: DebateRole::Con, // 🌟 AI 2 担任反方
        },
    ];

    let room_state = RoomState {
        room_id: room_id.to_string(),
        game_type: "lincoln_debate".to_string(),
        actors,
        history: Vec::new(), // 剥离了错误的 peers 字段，回归纯净状态
    };

    let mut ai_configs = HashMap::new();

    // 配置正方 AI
    ai_configs.insert(
        "ai_pro_debater".to_string(),
        AiConfig {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            prompt: "你现在是激进的立论家。请作为正方，针对裁判给出的辩题，发表具有说服力的论点。字数控制在200字以内。".to_string(),
            max_tokens: 200,
        },
    );

    // 配置反方 AI
    ai_configs.insert(
        "ai_con_debater".to_string(),
        AiConfig {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            prompt: "你现在是沉稳的驳论家。请作为反方，严密审视正方的发言，并进行针锋相对的反驳。字数控制在200字以内。".to_string(),
            max_tokens: 200,
        },
    );
    (Box::new(engine), room_state, ai_configs)
}
// pub fn create_lincoln_debate
