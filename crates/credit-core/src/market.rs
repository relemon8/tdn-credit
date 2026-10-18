use crate::{model::*, random::*, units::*};
use rand_core::RngCore;

const TRANSITIONS: [[f64; 4]; 4] = [
    [0.82, 0.10, 0.07, 0.01],
    [0.12, 0.76, 0.10, 0.02],
    [0.10, 0.03, 0.77, 0.10],
    [0.04, 0.01, 0.25, 0.70],
];
pub fn advance_macro(
    m: &mut MacroState,
    curve: &mut TreasuryCurve,
    difficulty: Difficulty,
    rng: &mut impl RngCore,
) {
    let row = match m.regime {
        Regime::Goldilocks => 0,
        Regime::Inflation => 1,
        Regime::Recession => 2,
        Regime::CreditCrisis => 3,
    };
    let roll = uniform(rng);
    let mut cumulative = 0.0;
    for (index, weight) in TRANSITIONS[row].iter().enumerate() {
        cumulative += weight;
        if roll < cumulative {
            m.regime = [
                Regime::Goldilocks,
                Regime::Inflation,
                Regime::Recession,
                Regime::CreditCrisis,
            ][index];
            break;
        }
    }
    // GDP, inflation, policy, unemployment, appetite, liquidity, volatility targets.
    let target = match m.regime {
        Regime::Goldilocks => [0.025, 0.022, 0.030, 0.043, 0.75, 0.88, 15.0],
        Regime::Inflation => [0.015, 0.055, 0.055, 0.050, 0.45, 0.70, 24.0],
        Regime::Recession => [-0.020, 0.015, 0.015, 0.075, 0.25, 0.50, 32.0],
        Regime::CreditCrisis => [-0.045, 0.005, 0.005, 0.095, 0.08, 0.20, 55.0],
    };
    let common = symmetric(rng);
    let blend = |old: f64, goal: f64, speed: f64| old + speed * (goal - old);
    m.gdp_growth =
        Rate::new(blend(m.gdp_growth.value(), target[0], 0.25) + common * 0.002).unwrap();
    m.inflation =
        Rate::new((blend(m.inflation.value(), target[1], 0.20) - common * 0.001).max(-0.01))
            .unwrap();
    m.policy_rate = Rate::new(blend(m.policy_rate.value(), target[2], 0.25).max(0.0)).unwrap();
    m.unemployment =
        Rate::new(blend(m.unemployment.value(), target[3], 0.15).clamp(0.01, 0.20)).unwrap();
    m.risk_appetite = (blend(m.risk_appetite, target[4], 0.35) + common * 0.04).clamp(0.02, 0.98);
    m.liquidity = (blend(m.liquidity, target[5], 0.40) + symmetric(rng) * 0.03).clamp(0.05, 1.0);
    if matches!(difficulty, Difficulty::Distressed | Difficulty::Beast) {
        m.liquidity = (m.liquidity - 0.05).max(0.05);
    }
    m.volatility = blend(m.volatility, target[6], 0.40);
    let targets = [
        m.policy_rate.value() + 0.001,
        m.policy_rate.value() * 0.65 + m.inflation.value() * 0.35 + 0.003,
        m.policy_rate.value() * 0.4 + m.inflation.value() * 0.6 + 0.006,
    ];
    let values = [curve.two, curve.five, curve.ten];
    let next: Vec<Rate> = values
        .iter()
        .zip(targets)
        .map(|(old, t)| {
            Rate::new(
                (blend(old.value(), t, 0.20) + common * 0.001 + symmetric(rng) * 0.0004)
                    .clamp(-0.005, 0.15),
            )
            .unwrap()
        })
        .collect();
    *curve = TreasuryCurve {
        two: next[0],
        five: next[1],
        ten: next[2],
    };
}
pub fn advance_financials(
    issuer: &mut IssuerState,
    m: &MacroState,
    funding_rate: Rate,
    policy_change: Rate,
    rng: &mut impl RngCore,
) {
    if issuer.defaulted {
        return;
    }
    let sensitivity = match issuer.sector {
        Sector::Healthcare | Sector::Utilities => 0.3,
        Sector::Consumer | Sector::Media | Sector::Technology => 1.5,
        _ => 1.0,
    };
    let growth = m.gdp_growth.value() * sensitivity / 12.0
        + symmetric(rng) * (1.0 - issuer.stability / 100.0) * 0.025;
    let f = &mut issuer.financials;
    f.revenue *= 1.0 + growth;
    if f.ebitda > 0.0 {
        f.ebitda *= (1.0 + growth * 2.0).clamp(0.85, 1.15);
    } else {
        f.ebitda += f.ebitda.abs().max(f.revenue * 0.02) * (growth * 2.0).clamp(-0.15, 0.15);
    }
    // Only the floating-rate share reprices immediately. Fixed debt keeps its
    // legacy cost until it is actually rolled; a HY issuer cannot refinance
    // magically at the same cost as a defensive A-rated issuer.
    const FLOATING_DEBT_SHARE: f64 = 0.20;
    f.interest_expense =
        (f.interest_expense + f.debt * FLOATING_DEBT_SHARE * policy_change.value()).max(0.0);
    let legacy_rate = f.interest_expense / f.debt.max(1.0);
    f.fcf += ((f.ebitda - f.interest_expense) * 0.45 - f.fcf) * 0.03;
    let monthly_cash = f.fcf / 12.0;
    issuer.payment_shortfall = 0.0;
    f.cash += monthly_cash;
    if f.cash < 0.0 {
        let deficit = -f.cash;
        let drawn = deficit.min(f.credit_line);
        f.credit_line -= drawn;
        f.debt += drawn;
        f.debt_maturity[4] += drawn;
        f.interest_expense += drawn * funding_rate.value();
        issuer.payment_shortfall += deficit - drawn;
        f.cash = 0.0;
        issuer.event_stress += 0.05;
    }
    // One twelfth of the next-year maturity bucket comes due, and is refinanced
    // only if the current funding environment and issuer liquidity permit it.
    let due = f.debt_maturity[0] / 12.0;
    let old = f.debt_maturity;
    for index in 0..4 {
        f.debt_maturity[index] = old[index] * 11.0 / 12.0 + old[index + 1] / 12.0;
    }
    f.debt_maturity[4] = old[4] * 11.0 / 12.0;
    let refinance = m.liquidity > 0.35 && f.coverage() > 1.3;
    if refinance {
        f.debt_maturity[4] += due;
        f.interest_expense =
            (f.interest_expense + due * (funding_rate.value() - legacy_rate)).max(0.0);
    } else {
        let cash_paid = due.min(f.cash);
        f.cash -= cash_paid;
        let drawn = (due - cash_paid).min(f.credit_line);
        f.credit_line -= drawn;
        f.debt += drawn;
        f.debt_maturity[4] += drawn;
        let paid = cash_paid + drawn;
        f.debt -= paid;
        f.interest_expense =
            (f.interest_expense + drawn * funding_rate.value() - paid * legacy_rate).max(0.0);
        if paid < due {
            issuer.event_stress += (due - paid) / f.debt.max(1.0) * 10.0;
            issuer.payment_shortfall += due - paid;
            f.debt_maturity[0] += due - paid;
        }
    }
    issuer.event_stress *= 0.90;
}
