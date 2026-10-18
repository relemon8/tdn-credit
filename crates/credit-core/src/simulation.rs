use crate::{
    credit,
    events::{self, EventDefinition, EventInstance, EventScope},
    market,
    model::*,
    portfolio, pricing,
    random::*,
    units::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid configuration or initial dataset: {0}")]
    InvalidData(String),
    #[error(transparent)]
    Trade(#[from] portfolio::TradeError),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    #[serde(default = "legacy_initial_capital")]
    pub initial_capital: Money,
    pub engine_version: String,
    pub data_version: u32,
    pub config: GameConfig,
    pub turn: u32,
    pub macro_state: MacroState,
    pub treasury: TreasuryCurve,
    pub issuers: BTreeMap<String, IssuerState>,
    pub bonds: BTreeMap<String, BondState>,
    pub portfolio: PortfolioState,
    pub benchmark: PortfolioState,
    pub history: Vec<MonthlyRecord>,
    pub trades: Vec<Trade>,
    pub news: Vec<NewsItem>,
    pub random: RandomState,
    pub pending_costs: BTreeMap<String, i64>,
    pub event_definitions: Vec<EventDefinition>,
    pub event_history: Vec<EventInstance>,
}
fn legacy_initial_capital() -> Money {
    Money(10_000_000_000)
}

impl GameState {
    pub fn validate(&self) -> Result<(), GameError> {
        let bad = |reason: &str| GameError::InvalidData(reason.into());
        if self.random != RandomState::new(self.config.seed)
            || self.event_history.iter().any(|e| e.turn > self.turn)
            || self.trades.iter().any(|t| t.turn > self.turn)
            || self
                .history
                .iter()
                .enumerate()
                .any(|(n, r)| r.turn as usize != n + 1)
        {
            return Err(bad("inconsistent replay history"));
        }
        if self.engine_version != ENGINE_VERSION
            || self.data_version != DATA_VERSION
            || !matches!(self.config.maximum_turns, 24 | 36)
            || self.history.len() != self.turn as usize
        {
            return Err(bad("incompatible configuration or history"));
        }
        if ![6_500_000_000, 10_000_000_000].contains(&self.initial_capital.0)
            || self.issuers.len() != 10
            || self.bonds.len() < 20
            || self.bonds.values().filter(|b| !b.settled).count() > 20
            || self.portfolio.cash.0 < 0
            || self.benchmark.cash.0 < 0
        {
            return Err(bad("invalid universe or cash"));
        }
        if [self.portfolio.cash.0, self.benchmark.cash.0]
            .iter()
            .any(|v| *v > 1_000_000_000_000_000)
        {
            return Err(bad("cash exceeds supported range"));
        }
        for (id, i) in &self.issuers {
            let f = &i.financials;
            if id != &i.ticker
                || !(0.0..=100.0).contains(&i.stability)
                || !(0.0..=100.0).contains(&i.event_risk)
                || !(0.0..=1.0).contains(&i.asset_quality)
                || [
                    f.revenue,
                    f.ebitda,
                    f.cash,
                    f.debt,
                    f.interest_expense,
                    f.fcf,
                    f.credit_line,
                ]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1e12)
                || i.agency.published.0 > 19
                || i.defaulted != i.default_turn.is_some()
                || i.default_turn.is_some_and(|t| t > self.turn)
                || f.cash < 0.
                || f.debt < 0.
                || f.revenue <= 0.
                || f.interest_expense < 0.
                || f.debt_maturity.iter().any(|v| *v < 0. || !v.is_finite())
            {
                return Err(bad("invalid issuer state"));
            }
        }
        for (id, b) in &self.bonds {
            if id != &b.id
                || !(0.0..=1.0).contains(&b.coupon.value())
                || b.maturity_month > self.turn.saturating_add(600)
                || b.face_value.value() != 100.0
                || !self.issuers.contains_key(&b.issuer)
                || !(0.0..=1000.0).contains(&b.clean_price.value())
                || b.bid_ask.value() < 0.
            {
                return Err(bad("invalid bond state"));
            }
        }
        for p in [&self.portfolio, &self.benchmark] {
            for (id, position) in &p.positions {
                if !self.bonds.contains_key(id)
                    || position.face.0 <= 0
                    || position.face.0 > 1_000_000_000_000_000
                    || position.cost.0 < 0
                {
                    return Err(bad("invalid position"));
                }
            }
        }
        Ok(())
    }
    pub fn new(
        config: GameConfig,
        issuers: Vec<IssuerState>,
        bonds: Vec<BondState>,
    ) -> Result<Self, GameError> {
        if !matches!(config.maximum_turns, 24 | 36) || issuers.len() != 10 || bonds.len() != 20 {
            return Err(GameError::InvalidData(
                "expected 10 issuers, 20 bonds, 24 or 36 turns".into(),
            ));
        }
        let mut game = Self {
            initial_capital: INITIAL_CASH,
            engine_version: ENGINE_VERSION.into(),
            data_version: DATA_VERSION,
            random: RandomState::new(config.seed),
            config,
            turn: 0,
            macro_state: MacroState::default(),
            treasury: TreasuryCurve::default(),
            issuers: issuers.into_iter().map(|i| (i.ticker.clone(), i)).collect(),
            bonds: bonds.into_iter().map(|b| (b.id.clone(), b)).collect(),
            portfolio: PortfolioState::default(),
            benchmark: PortfolioState::default(),
            history: Vec::new(),
            trades: Vec::new(),
            news: Vec::new(),
            pending_costs: BTreeMap::new(),
            event_definitions: Vec::new(),
            event_history: Vec::new(),
        };
        if game.issuers.len() != 10
            || game.bonds.len() != 20
            || game
                .bonds
                .values()
                .any(|b| !game.issuers.contains_key(&b.issuer) || b.maturity_month < 36)
        {
            return Err(GameError::InvalidData(
                "duplicate IDs, unknown issuer, or maturity too short".into(),
            ));
        }
        for issuer in game.issuers.values_mut() {
            issuer.score = credit::score(issuer, &game.macro_state);
            issuer.annual_pd = credit::annual_pd(issuer, &game.macro_state, game.config.difficulty);
        }
        game.reprice();
        // Fixed initial units, same mid pricing, no periodic index rebalancing.
        let allocation = INITIAL_CASH.0 / game.bonds.len() as i64;
        for bond in game.bonds.values() {
            let face =
                FaceAmount((allocation as f64 / (bond.dirty().value() / 100.0)).floor() as i64);
            let cost = face.value_at(bond.dirty());
            game.benchmark.cash.0 -= cost.0;
            game.benchmark
                .positions
                .insert(bond.id.clone(), Position { face, cost });
        }
        Ok(game)
    }
    /// The evaluation milestone is reached; trading and simulation remain available.
    pub fn evaluation_complete(&self) -> bool {
        self.turn >= self.config.maximum_turns
    }
    // Public bonds represent refinancing already included in issuer funding flows.
    // Never extend an existing contract or add issuer debt for the same refinancing twice.
    fn issue_replacement_bonds(&mut self, renewals: Vec<(BondState, i64)>) {
        let mut issued = Vec::new();
        for (old, budget) in renewals {
            let leg = if old.id.split('-').nth(1) == Some("2") {
                2
            } else {
                1
            };
            let tenor = if leg == 2 { 84 } else { 36 };
            let Some(maturity) = self.turn.checked_add(tenor) else {
                continue;
            };
            let mut bond = old;
            bond.id = format!("{}-{leg}-R{}", bond.issuer, self.turn);
            bond.maturity_month = maturity;
            bond.coupon = Rate::new(
                (self.treasury.at(tenor as f64 / 12.).value()
                    + bond.market_spread.as_rate().value())
                .clamp(0., 1.),
            )
            .unwrap();
            bond.settled = false;
            issued.push((bond.id.clone(), budget));
            self.bonds.insert(bond.id.clone(), bond);
        }
        if issued.is_empty() {
            return;
        }
        self.reprice();
        let mut names = Vec::new();
        for (id, budget) in issued {
            let bond = &self.bonds[&id];
            names.push(format!(
                "{} / 利息 年{:.2}% / 満期 {}か月目 / 価格 {:.2}",
                id,
                bond.coupon.value() * 100.,
                bond.maturity_month,
                bond.clean_price.value()
            ));
            if budget <= 0 {
                continue;
            }
            let face = FaceAmount((budget as f64 / (bond.dirty().value() / 100.)).floor() as i64);
            let cost = face.value_at(bond.dirty());
            if face.0 > 0 && cost.0 <= self.benchmark.cash.0 {
                self.benchmark.cash.0 -= cost.0;
                self.benchmark.positions.insert(id, Position { face, cost });
            }
        }
        self.news.push(NewsItem {
            turn: self.turn, event_id: "new_issue".into(), issuer: None,
            headline: "満期を迎えた企業から、新たな借り換え債が登場".into(),
            detail: format!("{}。旧債は契約どおり償還。保有者の元本・利息は現金へ、新債の購入は任意です。比較指数は償還元本だけを同一企業の新債へ再投資します。", names.join(" / ")),
            severity: 1,
        });
    }

    pub fn nav(&self) -> Money {
        portfolio::nav(&self.portfolio, &self.bonds)
    }
    pub fn benchmark_nav(&self) -> Money {
        portfolio::nav(&self.benchmark, &self.bonds)
    }
    pub fn trade(&mut self, id: &str, side: Side, face: FaceAmount) -> Result<Trade, GameError> {
        let result = portfolio::trade(&mut self.portfolio, &self.bonds, self.turn, id, side, face)?;
        *self.pending_costs.entry(id.into()).or_default() -= result.cost.0;
        self.trades.push(result.clone());
        Ok(result)
    }
    pub fn reprice(&mut self) {
        for bond in self.bonds.values_mut() {
            let issuer = &self.issuers[&bond.issuer];
            if !issuer.defaulted && self.turn >= bond.maturity_month {
                bond.settled = true;
            }
            bond.fair_spread = credit::fair_spread(issuer, bond, &self.macro_state, self.turn);
            bond.liquidity_score = (bond.base_liquidity * self.macro_state.liquidity / 0.85
                - issuer.event_stress * 10.0)
                .clamp(1.0, 100.0);
            bond.bid_ask = credit::bid_ask(
                bond.liquidity_score,
                self.macro_state.regime,
                issuer.event_stress,
            );
            if bond.settled {
                bond.clean_price = Price::new(0.0).unwrap();
                bond.accrued = Price::new(0.0).unwrap();
                bond.duration = DurationYears::new(0.0).unwrap();
                bond.spread_duration = bond.duration;
                bond.yield_rate = None;
                continue;
            }
            if issuer.defaulted {
                bond.clean_price = Price::new(100.0 * bond.recovery.value()).unwrap();
                bond.accrued = Price::new(0.0).unwrap();
                bond.duration = DurationYears::new(0.0).unwrap();
                bond.spread_duration = bond.duration;
                bond.yield_rate = None;
            } else {
                let dirty =
                    pricing::dirty_price(bond, self.turn, self.treasury, bond.market_spread);
                bond.accrued = pricing::accrued(bond, self.turn);
                bond.clean_price = Price::new(dirty.value() - bond.accrued.value()).unwrap();
                (bond.duration, bond.spread_duration) =
                    pricing::durations(bond, self.turn, self.treasury);
                bond.yield_rate = pricing::yield_to_maturity(bond, self.turn, dirty);
            }
        }
    }
    pub fn next_month(&mut self) -> Result<&MonthlyRecord, GameError> {
        let old_bonds = self.bonds.clone();
        let old_curve = self.treasury;
        let old_policy = self.macro_state.policy_rate;
        let prior_nav = self
            .history
            .last()
            .map_or(self.initial_capital.0, |r| r.nav.0);
        let prior_benchmark = self.benchmark_nav().0;
        let holdings = self.portfolio.positions.clone();
        self.turn = self
            .turn
            .checked_add(1)
            .ok_or_else(|| GameError::InvalidData("month counter exhausted".into()))?;
        let mut rng = self.random.stream("macro", self.turn, "");
        market::advance_macro(
            &mut self.macro_state,
            &mut self.treasury,
            self.config.difficulty,
            &mut rng,
        );
        self.process_events(true);
        for issuer in self.issuers.values_mut() {
            let mut rng = self.random.stream("financials", self.turn, &issuer.ticker);
            let spread = self
                .bonds
                .values()
                .filter(|b| b.issuer == issuer.ticker && !b.settled)
                .map(|b| b.market_spread.value())
                .reduce(f64::min)
                .unwrap_or(200.0);
            let funding = Rate::new(
                (self.treasury.five.value() + BasisPoints::new(spread).unwrap().as_rate().value())
                    .max(0.0025),
            )
            .unwrap();
            let policy_change =
                Rate::new(self.macro_state.policy_rate.value() - old_policy.value()).unwrap();
            market::advance_financials(issuer, &self.macro_state, funding, policy_change, &mut rng);
        }
        self.process_events(false);
        self.news.push(NewsItem { turn:self.turn,event_id:"market_comment".into(),issuer:None,headline:format!("Market desk / {:?}",self.macro_state.regime),detail:format!("5Y Treasury {:.2}% / Liquidity {:.0}/100。価格は金利、信用力、需給を同時に反映しています。",self.treasury.five.value()*100.,self.macro_state.liquidity*100.),severity:0 });
        for issuer in self.issuers.values_mut() {
            if issuer.default_turn.is_some_and(|t| self.turn == t + 3) {
                self.news.push(NewsItem {turn:self.turn,event_id:"restructuring".into(),issuer:Some(issuer.ticker.clone()),headline:format!("{} 回収金を支払い",issuer.ticker),detail:"保有していた回収請求権を確定RecoveryでCash化。Default時に評価済みの損失は重ねて計上しません。".into(),severity:2});
                self.event_history.push(EventInstance {
                    turn: self.turn,
                    definition_id: "restructuring".into(),
                    issuer: Some(issuer.ticker.clone()),
                    before: None,
                    after: None,
                    market_shock_bp: 0.,
                });
            }
            issuer.score = credit::score(issuer, &self.macro_state);
            issuer.annual_pd = credit::annual_pd(issuer, &self.macro_state, self.config.difficulty);
            let old_rating = issuer.agency.published;
            credit::update_rating(issuer, self.config.difficulty);
            if old_rating != issuer.agency.published {
                self.news.push(NewsItem {
                    turn: self.turn,
                    event_id: "rating_migration".into(),
                    issuer: Some(issuer.ticker.clone()),
                    headline: format!(
                        "{} Rating {} → {}",
                        issuer.ticker,
                        old_rating.label(),
                        issuer.agency.published.label()
                    ),
                    detail: "Agencyは継続した信用指標の変化を確認し、公表Ratingを変更しました。"
                        .into(),
                    severity: 2,
                });
            }
            let mut rng = self.random.stream("default", self.turn, &issuer.ticker);
            let funding_failure = issuer.payment_shortfall > 0.000001;
            if !issuer.defaulted
                && (funding_failure || uniform(&mut rng) < issuer.annual_pd.monthly().value())
            {
                issuer.defaulted = true;
                issuer.default_turn = Some(self.turn);
                issuer.agency.published = Rating(19);
                issuer.annual_pd = Probability::new(1.0).unwrap();
                issuer.score = 0.0;
                self.news.push(NewsItem {
                    turn: self.turn,
                    event_id: "default".into(),
                    issuer: Some(issuer.ticker.clone()),
                    headline: format!("{} 支払い不履行", issuer.ticker),
                    detail:
                        "債券は回収請求権へ移行。Coupon停止、優先順位と資産価値から回収額を算定。"
                            .into(),
                    severity: 3,
                });
                self.event_history.push(EventInstance {
                    turn: self.turn,
                    definition_id: "default".into(),
                    issuer: Some(issuer.ticker.clone()),
                    before: Some(issuer.financials.clone()),
                    after: Some(issuer.financials.clone()),
                    market_shock_bp: 0.,
                });
            }
            if self
                .turn
                .is_multiple_of(self.config.difficulty.disclosure_lag())
                && !issuer.defaulted
            {
                issuer.disclosed = issuer.financials.clone();
                issuer.disclosure_turn = self.turn;
            }
        }
        for bond in self.bonds.values_mut() {
            let issuer = &self.issuers[&bond.issuer];
            if issuer.defaulted {
                if issuer.default_turn == Some(self.turn) {
                    bond.recovery = credit::recovery(issuer, bond.seniority, &self.macro_state);
                }
                if self.turn >= issuer.default_turn.unwrap() + 3 {
                    bond.settled = true;
                }
            } else {
                let fair = credit::fair_spread(issuer, bond, &self.macro_state, self.turn).value();
                let mut rng = self.random.stream("market", self.turn, &bond.id);
                let efficiency =
                    self.config.difficulty.efficiency() * (0.3 + bond.base_liquidity / 100.0 * 0.7);
                let target = fair * (1.20 - self.macro_state.risk_appetite * 0.40);
                let noise = symmetric(&mut rng)
                    * (5.0 + self.macro_state.volatility * 0.3)
                    * (1.2 - bond.base_liquidity / 150.0);
                bond.market_spread = BasisPoints::new(
                    (bond.market_spread.value()
                        + efficiency * (target - bond.market_spread.value())
                        + noise)
                        .clamp(15.0, 3000.0),
                )
                .unwrap();
            }
        }
        self.reprice();
        let live: Vec<_> = self
            .bonds
            .values()
            .filter(|b| !b.settled && !self.issuers[&b.issuer].defaulted)
            .collect();
        let spread_change = live
            .iter()
            .map(|b| b.market_spread.value() - old_bonds[&b.id].market_spread.value())
            .sum::<f64>()
            / live.len().max(1) as f64;
        for news in self.news.iter_mut().filter(|n| n.turn == self.turn) {
            if news.event_id == "market_comment" {
                news.detail=format!("5Y Treasury {:+.1}bp / 非Default銘柄の平均Spread {:+.1}bp / Market liquidity {:.0}/100。",(self.treasury.five.value()-old_curve.five.value())*10_000.,spread_change,self.macro_state.liquidity*100.);
            }
            if let Some(issuer) = &news.issuer {
                if let Some(old) = old_bonds
                    .values()
                    .find(|b| &b.issuer == issuer && !b.settled)
                {
                    let new = &self.bonds[&old.id];
                    news.detail.push_str(&format!(
                        " / {} close: Spread {:.0} → {:.0}bp, Price {:.2} → {:.2}",
                        new.id,
                        old.market_spread.value(),
                        new.market_spread.value(),
                        old.clean_price.value(),
                        new.clean_price.value()
                    ));
                }
            }
        }
        let mut attribution = Attribution::default();
        let mut bond_attribution = BTreeMap::new();
        for (id, position) in &holdings {
            let old = &old_bonds[id];
            let new = &self.bonds[id];
            let issuer = &self.issuers[&new.issuer];
            let previously_default = issuer.default_turn.is_some_and(|t| t < self.turn);
            let (p1, p2, p3, payment) = if previously_default {
                (
                    old.dirty(),
                    old.dirty(),
                    old.dirty(),
                    Price::new(0.0).unwrap(),
                )
            } else {
                (
                    pricing::dirty_price(old, self.turn, old_curve, old.market_spread),
                    pricing::dirty_price(old, self.turn, self.treasury, old.market_spread),
                    pricing::dirty_price(old, self.turn, self.treasury, new.market_spread),
                    pricing::payment(old, self.turn),
                )
            };
            let final_cash = if issuer.default_turn == Some(self.turn) {
                Price::new(0.0).unwrap()
            } else if new.settled && !old.settled && issuer.defaulted {
                old.dirty()
            } else {
                payment
            };
            let value = |p: Price| position.face.value_at(p).0;
            let mut a = Attribution {
                carry: value(p1) + value(payment) - value(old.dirty()),
                rates: value(p2) - value(p1),
                spread: value(p3) - value(p2),
                default_loss: if issuer.defaulted {
                    value(new.dirty()) + value(final_cash) - value(p3) - value(payment)
                } else {
                    0
                },
                trading_cost: *self.pending_costs.get(id).unwrap_or(&0),
                residual: 0,
            };
            let delta = value(new.dirty()) + value(final_cash) - value(old.dirty());
            a.residual = delta + a.trading_cost - a.total();
            attribution.carry += a.carry;
            attribution.rates += a.rates;
            attribution.spread += a.spread;
            attribution.default_loss += a.default_loss;
            attribution.trading_cost += a.trading_cost;
            attribution.residual += a.residual;
            bond_attribution.insert(id.clone(), a);
        }
        for (id, cost) in &self.pending_costs {
            if !holdings.contains_key(id) {
                attribution.trading_cost += cost;
                bond_attribution.insert(
                    id.clone(),
                    Attribution {
                        trading_cost: *cost,
                        ..Default::default()
                    },
                );
            }
        }
        let renewals: Vec<_> = self
            .bonds
            .values()
            .filter(|b| {
                b.settled
                    && !old_bonds[&b.id].settled
                    && !self.issuers[&b.issuer].defaulted
                    && self.turn >= b.maturity_month
            })
            .map(|b| {
                (
                    b.clone(),
                    self.benchmark.positions.get(&b.id).map_or(0, |p| p.face.0),
                )
            })
            .collect();
        // Cashflows paid to opening holdings, never to positions bought after a shock.
        for portfolio in [&mut self.portfolio, &mut self.benchmark] {
            let mut settled = Vec::new();
            for (id, position) in &portfolio.positions {
                let old = &old_bonds[id];
                let new = &self.bonds[id];
                let issuer = &self.issuers[&new.issuer];
                let paid = if issuer.default_turn == Some(self.turn) {
                    Price::new(0.0).unwrap()
                } else if issuer.default_turn.is_some_and(|t| t < self.turn) {
                    if new.settled && !old.settled {
                        old.dirty()
                    } else {
                        Price::new(0.0).unwrap()
                    }
                } else {
                    pricing::payment(old, self.turn)
                };
                portfolio.cash.0 += position.face.value_at(paid).0;
                if new.settled {
                    settled.push(id.clone());
                }
            }
            for id in settled {
                if let Some(p) = portfolio.positions.remove(&id) {
                    /* Settlement is tracked in total-return P/L; include basis in realized ledger. */
                    let old = &old_bonds[&id];
                    let proceeds = if self.issuers[&old.issuer].defaulted {
                        p.face.value_at(old.dirty()).0
                    } else {
                        p.face.0
                    };
                    portfolio.realized.0 += proceeds - p.cost.0;
                }
            }
        }
        self.issue_replacement_bonds(renewals);
        let nav = self.nav();
        let benchmark_nav = self.benchmark_nav();
        let metrics = crate::analysis::metrics(self);
        attribution.residual += nav.0 - prior_nav - attribution.total();
        self.history.push(MonthlyRecord {
            average_duration: metrics.duration,
            average_rating: metrics.average_rating,
            max_issuer_weight: metrics.max_issuer_weight,
            invested_weight: metrics.invested_weight,
            turn: self.turn,
            nav,
            benchmark_nav,
            monthly_return: nav.0 as f64 / prior_nav as f64 - 1.0,
            benchmark_return: benchmark_nav.0 as f64 / prior_benchmark as f64 - 1.0,
            attribution,
            bonds: bond_attribution,
            prices: self
                .bonds
                .iter()
                .map(|(id, b)| (id.clone(), b.clean_price))
                .collect(),
            ratings: self
                .issuers
                .iter()
                .map(|(id, i)| (id.clone(), i.agency.published))
                .collect(),
            financials: self
                .issuers
                .iter()
                .map(|(id, i)| (id.clone(), i.financials.clone()))
                .collect(),
        });
        self.pending_costs.clear();
        Ok(self.history.last().unwrap())
    }

    fn process_events(&mut self, macro_only: bool) {
        let multiplier = if matches!(self.config.difficulty, Difficulty::Beast) {
            1.8
        } else {
            1.0
        };
        let scopes = if macro_only {
            [EventScope::Macro]
        } else {
            [EventScope::Issuer]
        };
        for scope in scopes {
            let targets: Vec<Option<String>> = if scope == EventScope::Macro {
                vec![None]
            } else {
                self.issuers.keys().map(|id| Some(id.clone())).collect()
            };
            for target in targets {
                let entity = target.as_deref().unwrap_or("market");
                let mut rng = self.random.stream("events", self.turn, entity);
                let probability = target
                    .as_ref()
                    .map_or(0.22, |id| 0.02 + self.issuers[id].event_risk / 100. * 0.13)
                    * multiplier;
                if uniform(&mut rng) > probability {
                    continue;
                }
                let issuer = target.as_ref().map(|id| &self.issuers[id]);
                let candidates = self
                    .event_definitions
                    .iter()
                    .filter(|d| {
                        let matches_scope = match d.scope {
                            EventScope::Macro => target.is_none(),
                            EventScope::Issuer => target.is_some(),
                            EventScope::Sector(_) => false,
                        };
                        matches_scope
                            && events::eligible(d, issuer)
                            && !self.event_history.iter().any(|e| {
                                e.definition_id == d.id
                                    && e.issuer == target
                                    && self.turn - e.turn < d.cooldown
                            })
                    })
                    .collect();
                if let Some(definition) = events::select(candidates, &mut rng).cloned() {
                    let (instance, news) = events::apply(
                        &definition,
                        target.as_deref(),
                        self.turn,
                        &mut self.issuers,
                        &mut self.bonds,
                        &mut self.macro_state,
                        &mut self.treasury,
                    );
                    self.event_history.push(instance);
                    self.news.push(news);
                }
            }
        }
        if macro_only {
            return;
        }
        let mut rng = self.random.stream("sector-events", self.turn, "");
        if uniform(&mut rng) < 0.32 * multiplier {
            let candidates = self
                .event_definitions
                .iter()
                .filter(|d| {
                    matches!(d.scope, EventScope::Sector(_))
                        && !self
                            .event_history
                            .iter()
                            .any(|e| e.definition_id == d.id && self.turn - e.turn < d.cooldown)
                })
                .collect();
            if let Some(def) = events::select(candidates, &mut rng).cloned() {
                let EventScope::Sector(sector) = def.scope else {
                    unreachable!()
                };
                let targets: Vec<_> = self
                    .issuers
                    .values()
                    .filter(|i| i.sector == sector && events::eligible(&def, Some(i)))
                    .map(|i| i.ticker.clone())
                    .collect();
                for target in targets {
                    let (instance, news) = events::apply(
                        &def,
                        Some(&target),
                        self.turn,
                        &mut self.issuers,
                        &mut self.bonds,
                        &mut self.macro_state,
                        &mut self.treasury,
                    );
                    self.event_history.push(instance);
                    self.news.push(news);
                }
            }
        }
    }
}
