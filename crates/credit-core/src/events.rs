use crate::{model::*, random::*, units::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventScope {
    Macro,
    Sector(Sector),
    Issuer,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Condition {
    Any,
    Leveraged,
    CashStrained,
    SystemOnly,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    Revenue(f64),
    Ebitda(f64),
    Debt(f64),
    CashEbitda(f64),
    FcfEbitda(f64),
    Stress(f64),
    AssetQuality(f64),
    MarketBp(f64),
    BondLiquidity(f64),
    RatesBp(f64),
    PolicyBp(f64),
    Inflation(f64),
    Growth(f64),
    Appetite(f64),
    MacroLiquidity(f64),
    Regime(Regime),
    Acquisition,
    Refinance,
    EquityRaise,
    DebtReduction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    pub id: String,
    pub scope: EventScope,
    pub condition: Condition,
    pub weight: u32,
    pub cooldown: u32,
    pub effects: Vec<Effect>,
    pub headline: String,
    pub severity: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInstance {
    pub turn: u32,
    pub definition_id: String,
    pub issuer: Option<String>,
    pub before: Option<Financials>,
    pub after: Option<Financials>,
    pub market_shock_bp: f64,
}
pub fn eligible(def: &EventDefinition, issuer: Option<&IssuerState>) -> bool {
    if matches!(def.condition, Condition::SystemOnly) {
        return false;
    }
    match issuer {
        Some(i) => {
            !i.defaulted
                && match def.condition {
                    Condition::Leveraged => i.financials.leverage() > 3.,
                    Condition::CashStrained => i.financials.liquidity() < 1.2,
                    _ => true,
                }
        }
        None => matches!(def.scope, EventScope::Macro),
    }
}

/// Applies primitive financial effects, then derives credit metrics elsewhere.
/// News is a record of the applied deltas, never a source of financial mutation.
pub fn apply(
    def: &EventDefinition,
    target: Option<&str>,
    turn: u32,
    issuers: &mut BTreeMap<String, IssuerState>,
    bonds: &mut BTreeMap<String, BondState>,
    macro_state: &mut MacroState,
    curve: &mut TreasuryCurve,
) -> (EventInstance, NewsItem) {
    let before = target.map(|id| issuers[id].financials.clone());
    let mut market_shock = 0.;
    for effect in &def.effects {
        match effect {
            Effect::PolicyBp(bp) => {
                macro_state.policy_rate =
                    Rate::new((macro_state.policy_rate.value() + bp / 10_000.0).max(0.0)).unwrap()
            }
            Effect::Inflation(value) => macro_state.inflation = Rate::new(*value).unwrap(),
            Effect::Growth(value) => macro_state.gdp_growth = Rate::new(*value).unwrap(),
            Effect::RatesBp(bp) => *curve = curve.shifted(BasisPoints::new(*bp).unwrap()),
            Effect::Appetite(change) => {
                macro_state.risk_appetite = (macro_state.risk_appetite + change).clamp(0.02, 0.98)
            }
            Effect::MacroLiquidity(change) => {
                macro_state.liquidity = (macro_state.liquidity + change).clamp(0.05, 1.)
            }
            Effect::Regime(regime) => macro_state.regime = *regime,
            Effect::MarketBp(bp) => {
                market_shock += bp;
                for bond in bonds
                    .values_mut()
                    .filter(|b| target.is_none_or(|id| b.issuer == id))
                {
                    bond.market_spread =
                        BasisPoints::new((bond.market_spread.value() + bp).clamp(15., 3000.))
                            .unwrap();
                }
            }
            Effect::BondLiquidity(change) => {
                for bond in bonds
                    .values_mut()
                    .filter(|b| target.is_none_or(|id| b.issuer == id))
                {
                    bond.base_liquidity = (bond.base_liquidity + change).clamp(5., 95.);
                }
            }
            _ => {
                if let Some(id) = target {
                    let i = issuers.get_mut(id).unwrap();
                    let f = &mut i.financials;
                    match effect {
                        Effect::Revenue(mult) => f.revenue *= mult,
                        Effect::Ebitda(mult) => {
                            if f.ebitda > 0.0 {
                                f.ebitda *= mult;
                            } else {
                                f.ebitda += f.ebitda.abs().max(f.revenue * 0.02) * (mult - 1.0);
                            }
                        }
                        Effect::Debt(mult) => {
                            let delta = f.debt * (mult - 1.);
                            f.debt += delta;
                            f.debt_maturity[4] = (f.debt_maturity[4] + delta).max(0.);
                        }
                        Effect::CashEbitda(mult) => {
                            f.cash += f.ebitda.abs().max(f.revenue * 0.02) * mult
                        }
                        Effect::FcfEbitda(mult) => {
                            f.fcf += f.ebitda.abs().max(f.revenue * 0.02) * mult
                        }
                        Effect::Stress(delta) => {
                            i.event_stress = (i.event_stress + delta).clamp(0., 5.)
                        }
                        Effect::AssetQuality(delta) => {
                            i.asset_quality = (i.asset_quality + delta).clamp(0., 1.)
                        }
                        Effect::Acquisition => {
                            // Price = 12x acquired EBITDA. Entire consideration is new debt.
                            let debt_added = f.debt * 0.40;
                            let acquired_ebitda = debt_added / 12.;
                            f.debt += debt_added;
                            f.debt_maturity[4] += debt_added;
                            f.revenue += acquired_ebitda * 6.;
                            // Near-term integration expenses reduce consolidated EBITDA;
                            // this is why +40% gross debt can exceed +40% net leverage.
                            f.ebitda += acquired_ebitda - debt_added * 0.095;
                            f.interest_expense += debt_added * 0.07;
                            f.fcf -= debt_added * 0.10;
                            i.event_stress += 0.65;
                        }
                        Effect::Refinance => {
                            let due = f.debt_maturity[0] * 0.7;
                            f.debt_maturity[0] -= due;
                            f.debt_maturity[4] += due;
                            f.interest_expense += due * 0.02;
                            i.event_stress = (i.event_stress - 0.3).max(0.);
                        }
                        Effect::EquityRaise => {
                            f.cash += f.ebitda.abs().max(f.revenue * 0.02) * 0.7;
                            i.event_stress = (i.event_stress - 0.3).max(0.);
                        }
                        Effect::DebtReduction => {
                            let repaid = (f.debt * 0.1).min(f.cash * 0.5);
                            f.cash -= repaid;
                            f.debt -= repaid;
                            for amount in &mut f.debt_maturity {
                                *amount *= 1. - repaid / (f.debt + repaid).max(1.);
                            }
                        }
                        _ => {}
                    }
                    if f.cash < 0. {
                        f.debt -= f.cash;
                        f.debt_maturity[4] -= f.cash;
                        f.cash = 0.;
                    }
                }
            }
        }
    }
    let after = target.map(|id| issuers[id].financials.clone());
    let detail = if let (Some(a), Some(b)) = (&before, &after) {
        format!("Net leverage {:.2}x → {:.2}x / FCF {:.1} → {:.1} 億円 / Debt {:.1} → {:.1} 億円 / Market shock {:+.0}bp",a.leverage(),b.leverage(),a.fcf,b.fcf,a.debt,b.debt,market_shock)
    } else {
        format!(
            "Market spread shock {:+.0}bp / Liquidity {:.0}/100 / Risk appetite {:.0}/100",
            market_shock,
            macro_state.liquidity * 100.,
            macro_state.risk_appetite * 100.
        )
    };
    (
        EventInstance {
            turn,
            definition_id: def.id.clone(),
            issuer: target.map(str::to_string),
            before,
            after,
            market_shock_bp: market_shock,
        },
        NewsItem {
            turn,
            event_id: def.id.clone(),
            issuer: target.map(str::to_string),
            headline: def.headline.clone(),
            detail,
            severity: def.severity,
        },
    )
}

pub fn select<'a>(
    candidates: Vec<&'a EventDefinition>,
    rng: &mut impl rand_core::RngCore,
) -> Option<&'a EventDefinition> {
    let total: u32 = candidates.iter().map(|d| d.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = (uniform(rng) * total as f64) as u32;
    for d in candidates {
        if roll < d.weight {
            return Some(d);
        }
        roll -= d.weight;
    }
    None
}
