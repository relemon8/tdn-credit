use credit_core::{model::*, random::uniform, GameState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Positive,
    Negative,
    Shock,
    RatingLag,
    Liquidity,
}
pub struct MemeDefinition {
    pub id: &'static str,
    pub text: &'static str,
    pub category: Context,
    pub allowed: &'static [Context],
    pub blocked: &'static [Context],
    pub rarity: f64,
    pub cooldown: u32,
    pub min_difficulty: u8,
    pub weight: u32,
}
pub const CATALOG: &[MemeDefinition] = &[
    MemeDefinition {
        id: "nice",
        text: "やりますねぇ！",
        category: Context::Positive,
        allowed: &[Context::Positive],
        blocked: &[Context::Shock],
        rarity: 0.22,
        cooldown: 6,
        min_difficulty: 0,
        weight: 10,
    },
    MemeDefinition {
        id: "bad",
        text: "まずいですよ！",
        category: Context::Negative,
        allowed: &[Context::Negative, Context::Liquidity],
        blocked: &[],
        rarity: 0.18,
        cooldown: 6,
        min_difficulty: 0,
        weight: 10,
    },
    MemeDefinition {
        id: "shock",
        text: "ファッ！？",
        category: Context::Shock,
        allowed: &[Context::Shock],
        blocked: &[],
        rarity: 0.18,
        cooldown: 8,
        min_difficulty: 0,
        weight: 10,
    },
    MemeDefinition {
        id: "lag",
        text: "おっ、そうだな",
        category: Context::RatingLag,
        allowed: &[Context::RatingLag],
        blocked: &[Context::Shock],
        rarity: 0.20,
        cooldown: 8,
        min_difficulty: 0,
        weight: 8,
    },
    MemeDefinition {
        id: "fine",
        text: "いいゾ～これ",
        category: Context::Positive,
        allowed: &[Context::Positive],
        blocked: &[Context::Shock],
        rarity: 0.16,
        cooldown: 8,
        min_difficulty: 0,
        weight: 7,
    },
    MemeDefinition {
        id: "cold",
        text: "おバッチェ冷えてますよ",
        category: Context::Liquidity,
        allowed: &[Context::Liquidity],
        blocked: &[Context::Shock],
        rarity: 0.12,
        cooldown: 8,
        min_difficulty: 0,
        weight: 5,
    },
    MemeDefinition {
        id: "unclear",
        text: "これもうわかんねぇな",
        category: Context::Negative,
        allowed: &[Context::Negative],
        blocked: &[Context::Shock],
        rarity: 0.10,
        cooldown: 8,
        min_difficulty: 0,
        weight: 5,
    },
    MemeDefinition {
        id: "clear",
        text: "はっきりわかんだね",
        category: Context::Positive,
        allowed: &[Context::Positive],
        blocked: &[Context::Shock],
        rarity: 0.10,
        cooldown: 8,
        min_difficulty: 0,
        weight: 5,
    },
];
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemeHistory {
    pub last_turn: Option<u32>,
    pub used: BTreeMap<String, u32>,
}

impl crate::PresentationState {
    /// Immutable game input: this API cannot mutate any financial state.
    pub fn update(&mut self, game: &GameState) {
        self.current_meme = None;
        if !self.meme_enabled || self.history.last_turn.is_some_and(|t| game.turn < t + 2) {
            return;
        }
        let context = if game
            .news
            .iter()
            .any(|n| n.turn == game.turn && n.event_id == "default")
        {
            Context::Shock
        } else if game.macro_state.liquidity < 0.35 {
            Context::Liquidity
        } else if game.issuers.values().any(|i| i.agency.watch) {
            Context::RatingLag
        } else if game
            .history
            .last()
            .is_some_and(|r| r.monthly_return > 0.005)
        {
            Context::Positive
        } else if game
            .history
            .last()
            .is_some_and(|r| r.monthly_return < -0.005)
        {
            Context::Negative
        } else {
            return;
        };
        let difficulty = match game.config.difficulty {
            Difficulty::Rookie => 0,
            Difficulty::Analyst => 1,
            Difficulty::Pm => 2,
            Difficulty::Distressed => 3,
            Difficulty::Beast => 4,
        };
        let candidates: Vec<_> = CATALOG
            .iter()
            .filter(|d| {
                d.allowed.contains(&context)
                    && !d.blocked.contains(&context)
                    && difficulty >= d.min_difficulty
                    && !self
                        .history
                        .used
                        .get(d.id)
                        .is_some_and(|t| game.turn < t + d.cooldown)
            })
            .collect();
        let total: u32 = candidates.iter().map(|d| d.weight).sum();
        if total == 0 {
            return;
        }
        let mut rng = game.random.stream("meme-v1", game.turn, "desk");
        let mut roll = (uniform(&mut rng) * total as f64) as u32;
        for d in candidates {
            if roll < d.weight {
                if uniform(&mut rng) < d.rarity {
                    self.current_meme = Some(d.text.into());
                    self.history.used.insert(d.id.into(), game.turn);
                    self.history.last_turn = Some(game.turn);
                }
                return;
            }
            roll -= d.weight;
        }
    }
}
