use crate::{model::*, units::*};

const SCORE_WEIGHTS: [f64; 7] = [0.24, 0.18, 0.17, 0.14, 0.12, 0.08, 0.07];
// Score already contains these risks. Temper correlated multiplicative factors
// to avoid counting the same leverage/liquidity weakness several times.
const PD_FACTOR_EXPONENT: f64 = 0.35;
const SPREAD_ANCHORS: [(f64, f64); 7] = [
    (30.0, 850.0),
    (40.0, 450.0),
    (50.0, 250.0),
    (60.0, 160.0),
    (70.0, 100.0),
    (80.0, 60.0),
    (90.0, 30.0),
];
const PD_ANCHORS: [(f64, f64); 7] = [
    (30.0, 0.20),
    (40.0, 0.06),
    (50.0, 0.015),
    (60.0, 0.004),
    (70.0, 0.001),
    (80.0, 0.0003),
    (90.0, 0.00005),
];
fn interpolate(score: f64, anchors: &[(f64, f64)]) -> f64 {
    for pair in anchors.windows(2) {
        if score <= pair[1].0 {
            let t = ((score - pair[0].0) / (pair[1].0 - pair[0].0)).clamp(0.0, 1.0);
            return pair[0].1 + t * (pair[1].1 - pair[0].1);
        }
    }
    anchors.last().unwrap().1
}

/// Sector-specific normalization against fictional peer balance sheets.
/// Calibrated once in data v1; not a per-turn or per-issuer rating override.
/// Changes to these constants require a data-version bump and replay review.
pub fn sector_score_adjustment(sector: Sector) -> f64 {
    match sector {
        Sector::Industrials => -7.1,
        Sector::Consumer => 6.7,
        Sector::Financials => -3.5,
        Sector::Transportation => 7.0,
        Sector::Media => 15.7,
        Sector::Healthcare => -9.0,
        Sector::RealEstate => 10.8,
        Sector::Technology => -6.7,
        Sector::Utilities => -1.1,
        Sector::DataInfrastructure => 9.8,
    }
}

pub fn score(issuer: &IssuerState, macro_state: &MacroState) -> f64 {
    if issuer.defaulted {
        return 0.0;
    }
    let f = &issuer.financials;
    let components = [
        (100.0 - f.leverage() * 12.0).clamp(0.0, 100.0),
        (15.0 + 24.0 * f.coverage().max(0.01).ln()).clamp(0.0, 100.0),
        (f.liquidity() * 45.0).clamp(0.0, 100.0),
        (50.0 + f.fcf_margin() * 250.0).clamp(0.0, 100.0),
        issuer.stability,
        match macro_state.regime {
            Regime::Goldilocks => 75.0,
            Regime::Inflation => 55.0,
            Regime::Recession => 35.0,
            Regime::CreditCrisis => 10.0,
        },
        (100.0 - issuer.event_risk - issuer.event_stress * 30.0).clamp(0.0, 100.0),
    ];
    let raw = components
        .iter()
        .zip(SCORE_WEIGHTS)
        .map(|(a, b)| a * b)
        .sum::<f64>();
    (raw + sector_score_adjustment(issuer.sector)).clamp(0.0, 100.0)
}
pub fn fair_spread(
    issuer: &IssuerState,
    bond: &BondState,
    macro_state: &MacroState,
    month: u32,
) -> BasisPoints {
    let f = &issuer.financials;
    let base = interpolate(issuer.score, &SPREAD_ANCHORS);
    let leverage = 16.0 * (f.leverage() - 3.0).max(0.0);
    let coverage = 20.0 * (4.0_f64.ln() - f.coverage().max(0.1).ln()).max(0.0);
    let fcf = 180.0 * (0.04 - f.fcf_margin()).max(0.0);
    let liquidity = 35.0 * (1.2 - f.liquidity()).max(0.0);
    let macro_premium = match macro_state.regime {
        Regime::Goldilocks => 0.0,
        Regime::Inflation => 20.0,
        Regime::Recession => 60.0,
        Regime::CreditCrisis => 150.0,
    };
    let sector = match issuer.sector {
        Sector::Healthcare | Sector::Utilities => -5.0,
        Sector::Media | Sector::RealEstate => 20.0,
        Sector::DataInfrastructure => 15.0,
        _ => 0.0,
    };
    let refinancing =
        ((f.debt_maturity[0] - f.cash - f.credit_line) / f.debt.max(1.0)).max(0.0) * 150.0;
    let seniority = match bond.seniority {
        Seniority::Secured => -50.0,
        Seniority::Senior => 0.0,
        Seniority::Subordinated => 85.0,
    };
    let bond_liquidity = (1.0 - bond.base_liquidity / 100.0) * 35.0;
    let term = (bond.maturity_month.saturating_sub(month) as f64 / 12.0 - 3.0).max(0.0) * 3.0;
    BasisPoints::new(
        (base
            + leverage
            + coverage
            + fcf
            + liquidity
            + macro_premium
            + sector
            + refinancing
            + seniority
            + bond_liquidity
            + term
            + issuer.event_risk * 0.2
            + issuer.event_stress * 50.0)
            .clamp(25.0, 2500.0),
    )
    .unwrap()
}
pub fn annual_pd(
    issuer: &IssuerState,
    macro_state: &MacroState,
    difficulty: Difficulty,
) -> Probability {
    if issuer.defaulted {
        return Probability::new(1.0).unwrap();
    }
    let f = &issuer.financials;
    let base = interpolate(issuer.score, &PD_ANCHORS);
    let leverage = (1.0 + (f.leverage().max(0.0) - 3.0) * 0.15).clamp(0.5, 4.0);
    let coverage = (4.0 / f.coverage().max(0.25)).sqrt().clamp(0.5, 4.0);
    let liquidity = (1.2 / f.liquidity().max(0.1)).clamp(0.5, 5.0);
    let macro_factor = match macro_state.regime {
        Regime::Goldilocks => 1.0,
        Regime::Inflation => 1.2,
        Regime::Recession => 1.8,
        Regime::CreditCrisis => 3.0,
    };
    let refinance =
        1.0 + ((f.debt_maturity[0] - f.cash - f.credit_line) / f.debt.max(1.0)).max(0.0) * 3.0;
    let support = if difficulty == Difficulty::Rookie {
        0.4
    } else {
        1.0
    };
    let factors =
        leverage * coverage * liquidity * macro_factor * refinance * (1.0 + issuer.event_stress);
    Probability::new((base * factors.powf(PD_FACTOR_EXPONENT) * support).clamp(0.0, 0.95)).unwrap()
}
pub fn update_rating(issuer: &mut IssuerState, difficulty: Difficulty) {
    if issuer.defaulted {
        issuer.agency.published = Rating(19);
        return;
    }
    let target = Rating::from_score(issuer.score);
    let a = &mut issuer.agency;
    // Score hysteresis, not a two-notch gate: a persistent BBB -> BBB- must
    // remain possible. Require 0.8 points beyond the published band boundary.
    let lower = if a.published.0 == 0 {
        90.0
    } else {
        90.0 - a.published.0 as f64 * (10.0 / 3.0)
    };
    let upper = if a.published.0 == 0 {
        100.0
    } else {
        90.0 - (a.published.0 - 1) as f64 * (10.0 / 3.0)
    };
    if target == a.published
        || (target > a.published && issuer.score >= lower - 0.8)
        || (target < a.published && issuer.score <= upper + 0.8)
    {
        a.breach_months = 0;
        a.watch = false;
        a.outlook = Outlook::Stable;
        return;
    }
    if target == a.candidate {
        a.breach_months = a.breach_months.saturating_add(1);
    } else {
        a.candidate = target;
        a.breach_months = 1;
    }
    a.outlook = if target > a.published {
        Outlook::Negative
    } else {
        Outlook::Positive
    };
    a.watch = a.breach_months >= 2;
    if a.breach_months >= difficulty.rating_lag() {
        a.published.0 = if target > a.published {
            a.published.0 + 1
        } else {
            a.published.0 - 1
        };
        a.breach_months = 0;
        a.watch = false;
    }
}
pub fn bid_ask(liquidity: f64, regime: Regime, stress: f64) -> Price {
    let l = (liquidity / 100.0).clamp(0.0, 1.0);
    let multiplier = match regime {
        Regime::Goldilocks => 1.0,
        Regime::Inflation => 1.3,
        Regime::Recession => 1.8,
        Regime::CreditCrisis => 4.0,
    };
    Price::new((0.15 + 2.4 * (1.0 - l).powi(2)) * multiplier * (1.0 + stress)).unwrap()
}
pub fn recovery(
    issuer: &IssuerState,
    seniority: Seniority,
    macro_state: &MacroState,
) -> Probability {
    let base = match seniority {
        Seniority::Secured => 0.60,
        Seniority::Senior => 0.40,
        Seniority::Subordinated => 0.20,
    };
    let fire_sale = if macro_state.regime == Regime::CreditCrisis {
        0.12
    } else {
        0.03
    };
    Probability::new(
        (base + (issuer.asset_quality - 0.5) * 0.25 - fire_sale - issuer.event_stress * 0.02)
            .clamp(0.03, 0.85)
            .min(0.25 + issuer.asset_quality * 0.5),
    )
    .unwrap()
}
