use crate::units::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ENGINE_VERSION: &str = "0.1.0";
pub const DATA_VERSION: u32 = 2;
pub const INITIAL_CASH: Money = Money(6_500_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Rookie,
    Analyst,
    Pm,
    Distressed,
    Beast,
}
impl Difficulty {
    pub fn rating_lag(self) -> u8 {
        match self {
            Self::Rookie => 2,
            Self::Analyst => 3,
            Self::Pm => 4,
            Self::Distressed => 4,
            Self::Beast => 5,
        }
    }
    pub fn disclosure_lag(self) -> u32 {
        match self {
            Self::Beast => 3,
            Self::Pm | Self::Distressed => 2,
            _ => 1,
        }
    }
    pub fn efficiency(self) -> f64 {
        match self {
            Self::Rookie => 0.20,
            Self::Analyst => 0.30,
            Self::Pm => 0.48,
            Self::Distressed => 0.25,
            Self::Beast => 0.60,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub seed: u64,
    pub difficulty: Difficulty,
    pub maximum_turns: u32,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 1919,
            difficulty: Difficulty::Analyst,
            maximum_turns: 24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sector {
    Industrials,
    Consumer,
    Financials,
    Transportation,
    Media,
    Healthcare,
    RealEstate,
    Technology,
    Utilities,
    DataInfrastructure,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Regime {
    Goldilocks,
    Inflation,
    Recession,
    CreditCrisis,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroState {
    pub regime: Regime,
    pub gdp_growth: Rate,
    pub inflation: Rate,
    pub policy_rate: Rate,
    pub unemployment: Rate,
    pub risk_appetite: f64,
    pub liquidity: f64,
    pub volatility: f64,
}
impl Default for MacroState {
    fn default() -> Self {
        Self {
            regime: Regime::Goldilocks,
            gdp_growth: Rate::new(0.025).unwrap(),
            inflation: Rate::new(0.025).unwrap(),
            policy_rate: Rate::new(0.03).unwrap(),
            unemployment: Rate::new(0.045).unwrap(),
            risk_appetite: 0.65,
            liquidity: 0.85,
            volatility: 16.0,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TreasuryCurve {
    pub two: Rate,
    pub five: Rate,
    pub ten: Rate,
}
impl Default for TreasuryCurve {
    fn default() -> Self {
        Self {
            two: Rate::new(0.028).unwrap(),
            five: Rate::new(0.031).unwrap(),
            ten: Rate::new(0.034).unwrap(),
        }
    }
}
impl TreasuryCurve {
    pub fn at(self, years: f64) -> Rate {
        let y = if years <= 2.0 {
            self.two.value()
        } else if years <= 5.0 {
            self.two.value() + (self.five.value() - self.two.value()) * (years - 2.0) / 3.0
        } else if years <= 10.0 {
            self.five.value() + (self.ten.value() - self.five.value()) * (years - 5.0) / 5.0
        } else {
            self.ten.value()
        };
        Rate::new(y).unwrap()
    }
    pub fn shifted(self, bp: BasisPoints) -> Self {
        let shift = |rate: Rate| Rate::new(rate.value() + bp.as_rate().value()).unwrap();
        Self {
            two: shift(self.two),
            five: shift(self.five),
            ten: shift(self.ten),
        }
    }
}

/// 0 = AAA, 1 = AA+, ... 16 = CCC+, 17 = CCC, 18 = CCC-, 19 = D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rating(pub u8);
impl Rating {
    pub fn label(self) -> &'static str {
        [
            "AAA", "AA+", "AA", "AA-", "A+", "A", "A-", "BBB+", "BBB", "BBB-", "BB+", "BB", "BB-",
            "B+", "B", "B-", "CCC+", "CCC", "CCC-", "D",
        ][usize::from(self.0.min(19))]
    }
    pub fn from_score(score: f64) -> Self {
        if score >= 90.0 {
            return Self(0);
        }
        let band = ((90.0 - score.clamp(30.0, 89.999)) / 10.0).floor() as u8;
        let within = (90.0 - score.clamp(30.0, 89.999)) % 10.0;
        Self((1 + band * 3 + (within / (10.0 / 3.0)).floor() as u8).min(18))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outlook {
    Positive,
    Stable,
    Negative,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingAgency {
    pub published: Rating,
    pub outlook: Outlook,
    pub watch: bool,
    pub candidate: Rating,
    pub breach_months: u8,
}

/// Financial statement figures in JPY 100 million (oku). Flows are annualized TTM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Financials {
    pub revenue: f64,
    pub ebitda: f64,
    pub cash: f64,
    pub debt: f64,
    pub interest_expense: f64,
    pub fcf: f64,
    pub debt_maturity: [f64; 5],
    pub credit_line: f64,
}
impl Financials {
    pub fn margin(&self) -> f64 {
        self.ebitda / self.revenue.max(1.0)
    }
    pub fn net_debt(&self) -> f64 {
        self.debt - self.cash
    }
    pub fn leverage(&self) -> f64 {
        if self.ebitda > 0.0 {
            self.net_debt() / self.ebitda
        } else {
            20.0
        }
    }
    pub fn coverage(&self) -> f64 {
        (self.ebitda / self.interest_expense.max(0.1)).max(0.0)
    }
    pub fn fcf_margin(&self) -> f64 {
        self.fcf / self.revenue.max(1.0)
    }
    pub fn liquidity(&self) -> f64 {
        ((self.cash + self.credit_line + self.fcf.max(0.0))
            / (self.debt_maturity[0] + self.interest_expense).max(1.0))
        .clamp(0.0, 3.0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerState {
    pub ticker: String,
    pub name: String,
    pub sector: Sector,
    pub profile: String,
    pub financials: Financials,
    pub disclosed: Financials,
    pub disclosure_turn: u32,
    pub stability: f64,
    pub event_risk: f64,
    pub asset_quality: f64,
    pub score: f64,
    pub annual_pd: Probability,
    pub agency: RatingAgency,
    pub event_stress: f64,
    pub defaulted: bool,
    pub default_turn: Option<u32>,
    pub recovery_factor: f64,
    pub payment_shortfall: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seniority {
    Secured,
    Senior,
    Subordinated,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondState {
    pub id: String,
    pub issuer: String,
    pub coupon: Rate,
    pub maturity_month: u32,
    pub face_value: Price,
    pub seniority: Seniority,
    pub clean_price: Price,
    pub accrued: Price,
    pub yield_rate: Option<Rate>,
    pub fair_spread: BasisPoints,
    pub market_spread: BasisPoints,
    pub duration: DurationYears,
    pub spread_duration: DurationYears,
    pub base_liquidity: f64,
    pub liquidity_score: f64,
    pub bid_ask: Price,
    pub recovery: Probability,
    pub settled: bool,
}
impl BondState {
    pub fn dirty(&self) -> Price {
        Price::new(self.clean_price.value() + self.accrued.value()).unwrap()
    }
    pub fn ask(&self) -> Price {
        Price::new(self.dirty().value() + self.bid_ask.value() / 2.0).unwrap()
    }
    pub fn bid(&self) -> Price {
        Price::new((self.dirty().value() - self.bid_ask.value() / 2.0).max(0.0)).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub face: FaceAmount,
    pub cost: Money,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioState {
    pub cash: Money,
    pub positions: BTreeMap<String, Position>,
    pub realized: Money,
    pub trading_cost: Money,
}
impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            cash: INITIAL_CASH,
            positions: BTreeMap::new(),
            realized: Money(0),
            trading_cost: Money(0),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub turn: u32,
    pub bond: String,
    pub side: Side,
    pub face: FaceAmount,
    pub price: Price,
    pub cash_amount: Money,
    pub cost: Money,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Attribution {
    pub carry: i64,
    pub rates: i64,
    pub spread: i64,
    pub default_loss: i64,
    pub trading_cost: i64,
    pub residual: i64,
}
impl Attribution {
    pub fn total(&self) -> i64 {
        self.carry
            + self.rates
            + self.spread
            + self.default_loss
            + self.trading_cost
            + self.residual
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyRecord {
    pub turn: u32,
    pub nav: Money,
    pub benchmark_nav: Money,
    pub monthly_return: f64,
    pub benchmark_return: f64,
    pub attribution: Attribution,
    pub bonds: BTreeMap<String, Attribution>,
    pub prices: BTreeMap<String, Price>,
    pub ratings: BTreeMap<String, Rating>,
    pub financials: BTreeMap<String, Financials>,
    pub average_duration: f64,
    pub average_rating: Option<Rating>,
    pub max_issuer_weight: f64,
    pub invested_weight: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub turn: u32,
    pub event_id: String,
    pub issuer: Option<String>,
    pub headline: String,
    pub detail: String,
    pub severity: u8,
}
