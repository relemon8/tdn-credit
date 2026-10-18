use serde::{Deserialize, Serialize};
pub mod achievements;
pub mod meme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationState {
    #[serde(default)]
    pub achievements: std::collections::BTreeMap<String, u32>,
    pub meme_enabled: bool,
    pub current_meme: Option<String>,
    pub history: meme::MemeHistory,
}
impl Default for PresentationState {
    fn default() -> Self {
        Self {
            achievements: Default::default(),
            meme_enabled: true,
            current_meme: None,
            history: Default::default(),
        }
    }
}
pub fn oku(yen: i64) -> String {
    format!("{:.2}", yen as f64 / 100_000_000.)
}
pub fn percent(value: f64) -> String {
    format!("{:+.2}%", value * 100.)
}

pub mod news;
