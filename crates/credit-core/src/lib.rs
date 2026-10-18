//! OS-independent, deterministic credit-market simulation.
//! Rates are decimal fractions. Prices are per 100 face. Cash is JPY.
pub mod analysis;
pub mod credit;
pub mod events;
pub mod market;
pub mod model;
pub mod portfolio;
pub mod pricing;
pub mod random;
pub mod simulation;
pub mod units;

pub use model::*;
pub use simulation::{GameError, GameState};
