use crate::{model::*, portfolio, pricing, units::*, GameState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioMetrics {
    pub nav: Money,
    pub market_value: Money,
    pub cash: Money,
    pub yield_rate: f64,
    pub duration: f64,
    pub spread_duration: f64,
    pub spread_dv01: f64,
    pub average_rating: Option<Rating>,
    pub average_notch: f64,
    pub issuer_exposure: BTreeMap<String, f64>,
    pub sector_exposure: BTreeMap<String, f64>,
    pub rating_exposure: BTreeMap<String, f64>,
    pub maturity_exposure: BTreeMap<String, f64>,
    pub max_issuer_weight: f64,
    pub invested_weight: f64,
}
pub fn metrics(g: &GameState) -> PortfolioMetrics {
    let nav = portfolio::nav(&g.portfolio, &g.bonds);
    let mv = nav.0 - g.portfolio.cash.0;
    let mut m = PortfolioMetrics {
        nav,
        market_value: Money(mv),
        cash: g.portfolio.cash,
        yield_rate: 0.,
        duration: 0.,
        spread_duration: 0.,
        spread_dv01: 0.,
        average_rating: None,
        average_notch: 0.,
        issuer_exposure: BTreeMap::new(),
        sector_exposure: BTreeMap::new(),
        rating_exposure: BTreeMap::new(),
        maturity_exposure: BTreeMap::new(),
        max_issuer_weight: 0.,
        invested_weight: if nav.0 > 0 {
            mv as f64 / nav.0 as f64
        } else {
            0.
        },
    };
    for (id, p) in &g.portfolio.positions {
        let b = &g.bonds[id];
        let i = &g.issuers[&b.issuer];
        let v = p.face.value_at(b.dirty()).0 as f64;
        let w = if mv > 0 { v / mv as f64 } else { 0. };
        let nav_weight = if nav.0 > 0 { v / nav.0 as f64 } else { 0. };
        m.yield_rate += w * b.yield_rate.map_or(0., |y| y.value());
        m.duration += w * b.duration.value();
        m.spread_duration += w * b.spread_duration.value();
        m.average_notch += w * i.agency.published.0 as f64;
        *m.issuer_exposure.entry(i.ticker.clone()).or_default() += nav_weight;
        *m.sector_exposure
            .entry(format!("{:?}", i.sector))
            .or_default() += nav_weight;
        *m.rating_exposure
            .entry(i.agency.published.label().into())
            .or_default() += nav_weight;
        let years = b.maturity_month.saturating_sub(g.turn) as f64 / 12.;
        let bucket = if years <= 3. {
            "0–3Y"
        } else if years <= 7. {
            "3–7Y"
        } else {
            "7Y+"
        };
        *m.maturity_exposure.entry(bucket.into()).or_default() += nav_weight;
    }
    if mv > 0 {
        m.average_rating = Some(Rating(m.average_notch.round().clamp(0., 19.) as u8));
    }
    m.spread_dv01 = mv as f64 * m.spread_duration * 0.0001;
    m.max_issuer_weight = m.issuer_exposure.values().copied().fold(0., f64::max);
    m
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResult {
    pub name: String,
    pub mid_pnl: Money,
    pub liquidation_cost: Money,
    pub description: String,
}
pub fn scenarios(g: &GameState) -> Vec<StressResult> {
    [
        ("Rates +100bp", 100., 0., 1.),
        ("Rates −100bp", -100., 0., 1.),
        ("Spread +100bp", 0., 100., 1.),
        ("Spread −100bp", 0., -100., 1.),
        ("Recession", -75., 150., 2.),
        ("Credit Crisis", -100., 400., 4.),
    ]
    .into_iter()
    .map(|(name, rates, spread, liquidity_mult)| {
        let mut pnl = 0;
        let mut exit = 0;
        for (id, p) in &g.portfolio.positions {
            let b = &g.bonds[id];
            let next = if g.issuers[&b.issuer].defaulted {
                b.dirty()
            } else {
                pricing::dirty_price(
                    b,
                    g.turn,
                    g.treasury.shifted(BasisPoints::new(rates).unwrap()),
                    BasisPoints::new((b.market_spread.value() + spread).max(0.)).unwrap(),
                )
            };
            pnl += p.face.value_at(next).0 - p.face.value_at(b.dirty()).0;
            exit += (p.face.0 as f64 * (b.bid_ask.value() * liquidity_mult / 2.).min(next.value())
                / 100.)
                .round() as i64;
        }
        StressResult {
            name: name.into(),
            mid_pnl: Money(pnl),
            liquidation_cost: Money(exit),
            description: if liquidity_mult > 1. {
                "即時の価格・売却費用Stress。将来のDefault、Coupon、Rating変化は含まない。"
            } else {
                "他条件一定の平行Shock。Spreadは0bpを下限とする。"
            }
            .into(),
        }
    })
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalScore {
    pub total_return: f64,
    pub benchmark_return: f64,
    pub excess: f64,
    pub max_drawdown: f64,
    pub sharpe: Option<f64>,
    pub risk_adjusted: f64,
    pub default_loss: Money,
    pub trading_cost: Money,
    pub best_month: f64,
    pub worst_month: f64,
    pub average_duration: f64,
    pub average_rating: Option<Rating>,
    pub score: f64,
    pub rank: String,
}
pub fn final_score(g: &GameState) -> FinalScore {
    let n = g.history.len() as f64;
    let total = g.nav().0 as f64 / g.initial_capital.0 as f64 - 1.;
    let benchmark = g.benchmark_nav().0 as f64 / g.initial_capital.0 as f64 - 1.;
    let returns: Vec<f64> = g.history.iter().map(|r| r.monthly_return).collect();
    let mean = if n > 0. {
        returns.iter().sum::<f64>() / n
    } else {
        0.
    };
    let variance = if n > 1. {
        returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.)
    } else {
        0.
    };
    let sharpe = if variance > 1e-14 {
        Some(mean / variance.sqrt() * 12_f64.sqrt())
    } else {
        None
    };
    let mut peak = g.initial_capital.0 as f64;
    let mut drawdown: f64 = 0.;
    for r in &g.history {
        peak = peak.max(r.nav.0 as f64);
        drawdown = drawdown.max(1. - r.nav.0 as f64 / peak);
    }
    let default_loss = Money(
        g.history
            .iter()
            .map(|r| r.attribution.default_loss.min(0))
            .sum::<i64>(),
    );
    let concentration = if n > 0. {
        g.history
            .iter()
            .map(|r| (r.max_issuer_weight - 0.30).max(0.))
            .sum::<f64>()
            / n
    } else {
        0.
    };
    let investment = if n > 0. {
        g.history.iter().map(|r| r.invested_weight).sum::<f64>() / n
    } else {
        0.
    };
    let score = (60. + (total - benchmark) * 300. + sharpe.unwrap_or(0.).clamp(-2., 3.) * 5.
        - drawdown * 180.
        - concentration * 60.
        + default_loss.0 as f64 / g.initial_capital.0 as f64 * 100.)
        .clamp(0., 100.);
    let rank = if total <= -0.35 {
        "Defaulted"
    } else if score >= 85.
        && total > benchmark
        && drawdown < 0.08
        && concentration < 0.02
        && investment >= 0.3
    {
        "AAA PM"
    } else if score >= 75. && total > benchmark && drawdown < 0.12 {
        "AA PM"
    } else if score >= 65. {
        "A PM"
    } else if score >= 50. {
        "BBB PM"
    } else if score >= 35. {
        "HY Warrior"
    } else {
        "Distressed Investor"
    };
    let average_notch = if n > 0. {
        g.history
            .iter()
            .filter_map(|r| r.average_rating)
            .map(|r| r.0 as f64)
            .sum::<f64>()
            / g.history
                .iter()
                .filter(|r| r.average_rating.is_some())
                .count()
                .max(1) as f64
    } else {
        0.
    };
    FinalScore {
        total_return: total,
        benchmark_return: benchmark,
        excess: total - benchmark,
        max_drawdown: drawdown,
        sharpe,
        risk_adjusted: (total - benchmark) - drawdown,
        default_loss,
        trading_cost: g.portfolio.trading_cost,
        best_month: returns.iter().copied().reduce(f64::max).unwrap_or(0.),
        worst_month: returns.iter().copied().reduce(f64::min).unwrap_or(0.),
        average_duration: if n > 0. {
            g.history.iter().map(|r| r.average_duration).sum::<f64>() / n
        } else {
            0.
        },
        average_rating: if investment > 0. {
            Some(Rating(average_notch.round() as u8))
        } else {
            None
        },
        score,
        rank: rank.into(),
    }
}
