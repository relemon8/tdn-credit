use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

/// Domain-separated streams prevent presentation and order entry from consuming
/// financial randomness. Algorithm and dependencies are locked with the save version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomState {
    master: u64,
}
impl RandomState {
    pub fn new(master: u64) -> Self {
        Self { master }
    }
    pub fn stream(&self, domain: &str, turn: u32, entity: &str) -> ChaCha8Rng {
        let mut hash = blake3::Hasher::new_derive_key("TDN CREDIT RNG v1");
        hash.update(&self.master.to_le_bytes());
        hash.update(&(domain.len() as u64).to_le_bytes());
        hash.update(domain.as_bytes());
        hash.update(&turn.to_le_bytes());
        hash.update(&(entity.len() as u64).to_le_bytes());
        hash.update(entity.as_bytes());
        ChaCha8Rng::from_seed(*hash.finalize().as_bytes())
    }
}
pub fn uniform(rng: &mut impl RngCore) -> f64 {
    // Exactly 53 random bits, no floating endpoint at 1.
    ((rng.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
}
pub fn symmetric(rng: &mut impl RngCore) -> f64 {
    uniform(rng) * 2.0 - 1.0
}
