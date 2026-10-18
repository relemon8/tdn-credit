use credit_core::{credit, model::*, portfolio, pricing, units::*};
use credit_data::new_game;
use proptest::prelude::*;

#[test]
fn basis_points_and_probability_units() {
    assert_eq!(BasisPoints::new(100.).unwrap().as_rate().value(), 0.01);
    assert_eq!(Rate::from_percent(1.).unwrap().value(), 0.01);
    let p = Probability::new(0.12).unwrap().monthly().value();
    assert!(((1. - p).powi(12) - 0.88).abs() < 1e-12);
    assert!(Probability::new(1.1).is_err());
    assert!(Price::new(f64::NAN).is_err());
    assert!(serde_json::from_str::<Probability>("-0.1").is_err());
}
#[test]
fn duration_and_spread_sensitivity() {
    let g = new_game(GameConfig::default());
    let mut short = g.bonds["TDN-1"].clone();
    let mut long = short.clone();
    short.maturity_month = 36;
    long.maturity_month = 120;
    let shift = BasisPoints::new(100.).unwrap();
    let loss = |b: &BondState, curve: TreasuryCurve, s: BasisPoints| {
        1. - pricing::dirty_price(b, 0, curve, s).value()
            / pricing::dirty_price(b, 0, g.treasury, b.market_spread).value()
    };
    assert!(
        loss(&long, g.treasury.shifted(shift), long.market_spread)
            > loss(&short, g.treasury.shifted(shift), short.market_spread)
    );
    let spread = BasisPoints::new(short.market_spread.value() + 100.).unwrap();
    assert!(loss(&long, g.treasury, spread) > loss(&short, g.treasury, spread));
    let (d, sd) = pricing::durations(&long, 0, g.treasury);
    assert!(d.value() > 5.);
    assert!((d.value() - sd.value()).abs() < 1e-7);
}
#[test]
fn credit_deterioration_widens_fair_and_pd() {
    let g = new_game(GameConfig::default());
    let original = g.issuers["TDN"].clone();
    let bond = &g.bonds["TDN-1"];
    let base = credit::fair_spread(&original, bond, &g.macro_state, 0).value();
    for kind in 0..3 {
        let mut i = original.clone();
        match kind {
            0 => i.financials.debt *= 1.5,
            1 => i.financials.interest_expense *= 2.,
            _ => i.financials.fcf -= 100.,
        }
        i.score = credit::score(&i, &g.macro_state);
        assert!(credit::fair_spread(&i, bond, &g.macro_state, 0).value() > base);
        assert!(
            credit::annual_pd(&i, &g.macro_state, Difficulty::Analyst).value()
                >= original.annual_pd.value()
        );
    }
}
#[test]
fn liquidity_costs_and_recovery_priority() {
    assert!(
        credit::bid_ask(20., Regime::Goldilocks, 0.).value()
            > credit::bid_ask(80., Regime::Goldilocks, 0.).value()
    );
    assert!(
        credit::bid_ask(80., Regime::CreditCrisis, 0.).value()
            > credit::bid_ask(80., Regime::Goldilocks, 0.).value()
    );
    let g = new_game(GameConfig::default());
    let i = &g.issuers["AKYS"];
    assert!(
        credit::recovery(i, Seniority::Secured, &g.macro_state).value()
            > credit::recovery(i, Seniority::Subordinated, &g.macro_state).value()
    );
}

#[test]
fn initial_credit_premia_cover_reference_expected_default_loss() {
    // Sanity check for the initial near-par universe, not an arbitrage theorem
    // for distressed prices. Fair spread must not omit its basic loss budget.
    let g = new_game(GameConfig::default());
    for b in g.bonds.values() {
        let i = &g.issuers[&b.issuer];
        let recovery = credit::recovery(i, b.seniority, &g.macro_state).value();
        let loss_bp = i.annual_pd.value() * (1. - recovery) * 10_000.;
        assert!(
            b.fair_spread.value() >= loss_bp,
            "{} fair {:.1}bp, expected loss {:.1}bp",
            b.id,
            b.fair_spread.value(),
            loss_bp
        );
    }
}

#[test]
fn net_cash_is_reported_as_negative_net_debt() {
    let mut issuer = issuers_for_test();
    issuer.financials.cash = issuer.financials.debt + 100.;
    assert_eq!(issuer.financials.net_debt(), -100.);
    assert!(issuer.financials.leverage() < 0.);
}
#[test]
fn ask_buy_bid_sell_and_cash_invariants() {
    let mut g = new_game(GameConfig::default());
    let bond = g.bonds["TDN-1"].clone();
    let buy = g
        .trade("TDN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    assert_eq!(buy.price, bond.ask());
    assert_eq!(g.portfolio.cash.0, INITIAL_CASH.0 - buy.cash_amount.0);
    let sell = g
        .trade("TDN-1", Side::Sell, FaceAmount(100_000_000))
        .unwrap();
    assert_eq!(sell.price, bond.bid());
    assert_eq!(
        g.portfolio.cash.0,
        INITIAL_CASH.0 - buy.cash_amount.0 + sell.cash_amount.0
    );
    assert!(g.portfolio.cash.0 < INITIAL_CASH.0);
    assert!(g.portfolio.positions.is_empty());
}
#[test]
fn invalid_orders_are_atomic() {
    let mut g = new_game(GameConfig::default());
    let before = serde_json::to_string(&g).unwrap();
    for (id, side, face) in [
        ("missing", Side::Buy, 10_000_000),
        ("missing", Side::Sell, 10_000_000),
        ("TDN-1", Side::Sell, 10_000_000),
        ("TDN-1", Side::Buy, 100_000_000_000),
        ("TDN-1", Side::Buy, -10_000_000),
        ("TDN-1", Side::Buy, 123),
    ] {
        assert!(g.trade(id, side, FaceAmount(face)).is_err());
    }
    assert_eq!(before, serde_json::to_string(&g).unwrap());
    g.trade("TDN-1", Side::Buy, FaceAmount(10_000_000)).unwrap();
    assert!(g
        .trade("TDN-1", Side::Sell, FaceAmount(20_000_000))
        .is_err());
}
#[test]
fn seed_benchmark_and_save_replay() {
    let mut a = new_game(GameConfig::default());
    let mut b = a.clone();
    a.trade("TDN-1", Side::Buy, FaceAmount(1_000_000_000))
        .unwrap();
    b.trade("TDN-1", Side::Buy, FaceAmount(1_000_000_000))
        .unwrap();
    for turn in 1..=24 {
        a.next_month().unwrap();
        b.next_month().unwrap();
        if turn == 10 {
            b = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        }
        assert_eq!(a.nav(), b.nav());
        assert_eq!(a.benchmark_nav(), b.benchmark_nav());
        let r = a.history.last().unwrap();
        assert!(
            r.attribution.residual.abs() <= 10,
            "residual at {turn}: {}",
            r.attribution.residual
        );
    }
    assert!(a.next_month().is_ok());
    assert_eq!(a.turn, 25);
    assert!(a.trade("TDN-1", Side::Buy, FaceAmount(10_000_000)).is_ok());
}
#[test]
fn identical_holdings_price_identically_to_benchmark() {
    let mut g = new_game(GameConfig::default());
    g.portfolio = g.benchmark.clone();
    for _ in 0..24 {
        g.next_month().unwrap();
        assert_eq!(g.nav(), g.benchmark_nav());
    }
}
#[test]
fn financial_rng_is_independent_of_presentation_and_orders() {
    let mut a = new_game(GameConfig::default());
    let mut b = a.clone();
    for turn in 0..24 {
        let _meme = b.random.stream("meme", turn, "desk");
        a.next_month().unwrap();
        b.next_month().unwrap();
        assert_eq!(
            serde_json::to_string(&a.bonds).unwrap(),
            serde_json::to_string(&b.bonds).unwrap()
        );
    }
}
#[test]
fn rating_requires_persistent_evidence() {
    let mut i = issuers_for_test();
    let old = i.agency.published;
    i.score = 30.;
    credit::update_rating(&mut i, Difficulty::Analyst);
    assert_eq!(i.agency.published, old);
    credit::update_rating(&mut i, Difficulty::Analyst);
    assert!(i.agency.watch);
    credit::update_rating(&mut i, Difficulty::Analyst);
    assert!(i.agency.published > old);
}
fn issuers_for_test() -> IssuerState {
    credit_data::issuers().remove(0)
}

#[test]
fn event_catalog_has_fifty_unique_financial_definitions() {
    let catalog = credit_data::events::catalog();
    assert_eq!(catalog.len(), 50);
    let ids: std::collections::BTreeSet<_> = catalog.iter().map(|d| &d.id).collect();
    assert_eq!(ids.len(), 50);
    assert!(catalog.iter().all(|d| !d.effects.is_empty()));
}

#[test]
fn macro_news_changes_the_reported_macro_variables() {
    let mut g = new_game(GameConfig::default());
    let catalog = credit_data::events::catalog();
    let hike = catalog.iter().find(|d| d.id == "hike").unwrap();
    let old = g.macro_state.policy_rate.value();
    credit_core::events::apply(
        hike,
        None,
        1,
        &mut g.issuers,
        &mut g.bonds,
        &mut g.macro_state,
        &mut g.treasury,
    );
    assert!((g.macro_state.policy_rate.value() - old - 0.0025).abs() < 1e-10);
    let shock = catalog.iter().find(|d| d.id == "inflation").unwrap();
    credit_core::events::apply(
        shock,
        None,
        1,
        &mut g.issuers,
        &mut g.bonds,
        &mut g.macro_state,
        &mut g.treasury,
    );
    assert_eq!(g.macro_state.regime, Regime::Inflation);
    assert!(g.macro_state.inflation.value() > 0.05);
}

#[test]
fn negative_ebitda_does_not_reverse_cash_event_directions() {
    let catalog = credit_data::events::catalog();
    for (id, increase) in [
        ("dividend", false),
        ("buyback", false),
        ("equity_raise", true),
    ] {
        let mut g = new_game(GameConfig::default());
        let i = g.issuers.get_mut("YJSN").unwrap();
        i.financials.ebitda = -50.;
        i.financials.cash = 100.;
        let d = catalog.iter().find(|d| d.id == id).unwrap();
        credit_core::events::apply(
            d,
            Some("YJSN"),
            1,
            &mut g.issuers,
            &mut g.bonds,
            &mut g.macro_state,
            &mut g.treasury,
        );
        let cash = g.issuers["YJSN"].financials.cash;
        assert_eq!(cash > 100., increase, "incorrect cash direction: {id}");
    }
    for (id, improves) in [("beat", true), ("miss", false)] {
        let mut g = new_game(GameConfig::default());
        g.issuers.get_mut("YJSN").unwrap().financials.ebitda = -50.;
        let d = catalog.iter().find(|d| d.id == id).unwrap();
        credit_core::events::apply(
            d,
            Some("YJSN"),
            1,
            &mut g.issuers,
            &mut g.bonds,
            &mut g.macro_state,
            &mut g.treasury,
        );
        assert_eq!(g.issuers["YJSN"].financials.ebitda > -50., improves);
    }
}

#[test]
fn higher_market_funding_cost_only_reprices_rolled_debt() {
    use credit_core::{market, random::RandomState};
    let g = new_game(GameConfig::default());
    let mut a = g.issuers["TDN"].clone();
    let mut b = a.clone();
    let due = a.financials.debt_maturity[0] / 12.;
    let expected = due * 0.02;
    let random = RandomState::new(10);
    let mut ra = random.stream("test", 1, "");
    let mut rb = ra.clone();
    market::advance_financials(
        &mut a,
        &g.macro_state,
        Rate::new(0.05).unwrap(),
        Rate::new(0.).unwrap(),
        &mut ra,
    );
    market::advance_financials(
        &mut b,
        &g.macro_state,
        Rate::new(0.07).unwrap(),
        Rate::new(0.).unwrap(),
        &mut rb,
    );
    assert!(
        (b.financials.interest_expense - a.financials.interest_expense - expected).abs() < 1e-10
    );
    assert!(b.financials.coverage() < a.financials.coverage());
}

#[test]
fn floating_debt_reprices_without_resetting_fixed_legacy_cost() {
    use credit_core::{market, random::RandomState};
    let g = new_game(GameConfig::default());
    let mut a = g.issuers["TDN"].clone();
    a.financials.debt_maturity = [0., 0., 0., 0., a.financials.debt];
    let mut b = a.clone();
    let expected = a.financials.debt * 0.2 * 0.01;
    let random = RandomState::new(10);
    let mut ra = random.stream("test", 1, "");
    let mut rb = ra.clone();
    market::advance_financials(
        &mut a,
        &g.macro_state,
        Rate::new(0.05).unwrap(),
        Rate::new(0.).unwrap(),
        &mut ra,
    );
    market::advance_financials(
        &mut b,
        &g.macro_state,
        Rate::new(0.05).unwrap(),
        Rate::new(0.01).unwrap(),
        &mut rb,
    );
    assert!(
        (b.financials.interest_expense - a.financials.interest_expense - expected).abs() < 1e-10
    );
}

#[test]
fn debt_funded_acquisition_has_a_reconciled_financial_bridge() {
    let mut g = new_game(GameConfig::default());
    let d = credit_data::events::catalog()
        .into_iter()
        .find(|d| d.id == "debt_acquisition")
        .unwrap();
    let initial = g.issuers["TNOK"].financials.clone();
    let (instance, news) = credit_core::events::apply(
        &d,
        Some("TNOK"),
        1,
        &mut g.issuers,
        &mut g.bonds,
        &mut g.macro_state,
        &mut g.treasury,
    );
    let after = &g.issuers["TNOK"].financials;
    assert!((after.debt - initial.debt * 1.4).abs() < 1e-9);
    assert!((after.leverage() - 5.7).abs() < 0.15);
    assert!(after.fcf < initial.fcf);
    assert!(instance.after.is_some());
    assert!(news.detail.contains("Debt"));
}

#[test]
fn default_claim_reprices_and_settles_without_extra_carry() {
    let mut g = new_game(GameConfig::default());
    g.trade("YJSN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    let i = g.issuers.get_mut("YJSN").unwrap();
    i.defaulted = true;
    i.default_turn = Some(0);
    i.agency.published = Rating(19);
    g.reprice();
    assert!((g.bonds["YJSN-1"].clean_price.value() - 40.).abs() < 1e-9);
    // Reset monthly starting NAV because this is a controlled fixture shock.
    g.history.clear();
    let before = g.portfolio.cash.0;
    for _ in 0..3 {
        g.next_month().unwrap();
    }
    assert!(!g.portfolio.positions.contains_key("YJSN-1"));
    assert_eq!(g.portfolio.cash.0, before + 40_000_000);
    assert_eq!(g.bonds["YJSN-1"].clean_price.value(), 0.);
}

#[test]
fn one_notch_rating_migration_is_possible_after_evidence_lag() {
    let mut i = issuers_for_test();
    i.agency.published = Rating(8);
    i.score = 62.;
    for _ in 0..Difficulty::Analyst.rating_lag() {
        credit::update_rating(&mut i, Difficulty::Analyst);
    }
    assert_eq!(i.agency.published, Rating(9));
}

#[test]
fn partial_trades_and_coupon_cashflows_reconcile_each_month() {
    let mut g = new_game(GameConfig::default());
    let mut prior = INITIAL_CASH.0;
    for turn in 0..24 {
        if turn % 4 == 0 {
            g.trade("HTN-1", Side::Buy, FaceAmount(100_000_000))
                .unwrap();
        }
        if turn % 4 == 2 {
            g.trade("HTN-1", Side::Sell, FaceAmount(50_000_000))
                .unwrap();
        }
        let r = g.next_month().unwrap();
        assert_eq!(r.attribution.total(), r.nav.0 - prior);
        assert!(r.attribution.residual.abs() <= 10);
        prior = r.nav.0;
    }
}

#[test]
fn coupon_is_not_paid_in_default_month() {
    let mut g = new_game(GameConfig::default());
    g.event_definitions.clear();
    g.trade("YJSN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    for _ in 0..5 {
        g.next_month().unwrap();
    }
    let i = g.issuers.get_mut("YJSN").unwrap();
    i.financials.cash = 0.;
    i.financials.credit_line = 0.;
    i.financials.fcf = -1000.;
    let cash = g.portfolio.cash;
    g.next_month().unwrap();
    assert!(g.issuers["YJSN"].defaulted);
    assert_eq!(g.portfolio.cash, cash);
    assert!(g.history.last().unwrap().attribution.residual.abs() <= 10);
}

#[test]
fn longer_game_matures_bonds_and_final_score_remains_finite() {
    let mut g = new_game(GameConfig {
        maximum_turns: 36,
        ..Default::default()
    });
    g.trade("TDN-1", Side::Buy, FaceAmount(1_000_000_000))
        .unwrap();
    for _ in 0..36 {
        g.next_month().unwrap();
        g.validate().unwrap();
    }
    assert!(g.bonds["TDN-1"].settled);
    assert!(!g.portfolio.positions.contains_key("TDN-1"));
    let score = credit_core::analysis::final_score(&g);
    assert!(score.score.is_finite());
    assert!(score.sharpe.is_none_or(f64::is_finite));
}

#[test]
fn cash_only_never_receives_highest_rank_and_stress_is_zero() {
    let mut g = new_game(GameConfig::default());
    for _ in 0..24 {
        g.next_month().unwrap();
    }
    assert_ne!(credit_core::analysis::final_score(&g).rank, "AAA PM");
    assert!(credit_core::analysis::scenarios(&g)
        .iter()
        .all(|s| s.mid_pnl.0 == 0 && s.liquidation_cost.0 == 0));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn twenty_four_months_remain_finite_and_reconcile(seed in any::<u64>()) {
      for difficulty in [Difficulty::Rookie,Difficulty::Analyst,Difficulty::Pm,Difficulty::Distressed,Difficulty::Beast] {
        let mut g=new_game(GameConfig { seed,difficulty,..Default::default() });
        g.trade("YJSN-1",Side::Buy,FaceAmount(1_000_000_000)).unwrap();
        for _ in 0..24 {
            g.next_month().unwrap();
            g.validate().unwrap();
            for issuer in g.issuers.values() {prop_assert!((issuer.financials.debt_maturity.iter().sum::<f64>()-issuer.financials.debt).abs()<1e-6);}
            prop_assert!(g.portfolio.cash.0>=0);
            prop_assert!(g.bonds.values().all(|b| b.clean_price.value().is_finite() && b.clean_price.value()>=0.));
            let r=g.history.last().unwrap(); prop_assert!(r.attribution.residual.abs()<=10);
        }
        prop_assert!(portfolio::nav(&g.portfolio,&g.bonds).0>=0);
      }
    }
}

#[test]
fn continuation_remains_tradeable_and_replayable_through_120_months() {
    let mut a = new_game(GameConfig::default());
    a.event_definitions.clear();
    let mut b = a.clone();
    for month in 1..=120 {
        a.next_month().unwrap();
        b.next_month().unwrap();
        a.validate().unwrap();
        b.validate().unwrap();
        assert_eq!(a.nav(), b.nav());
        assert_eq!(a.benchmark_nav(), b.benchmark_nav());
        assert!(a.history.last().unwrap().attribution.residual.abs() <= 10);
        if month == 24 || month == 60 {
            b = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        }
        if month == 25 {
            for g in [&mut a, &mut b] {
                g.trade("TDN-1", Side::Buy, FaceAmount(10_000_000)).unwrap();
                g.trade("TDN-1", Side::Sell, FaceAmount(10_000_000))
                    .unwrap();
            }
        }
    }
    assert_eq!(a.turn, 120);
    assert!(a.bonds.len() > 20, "maturities should create new contracts");
    assert!(a.bonds.values().any(|bond| !bond.settled));
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    let score = credit_core::analysis::final_score(&a);
    assert!(score.score.is_finite());
}

#[test]
fn maturity_returns_cash_and_new_bond_does_not_steal_player_redemption() {
    let mut g = new_game(GameConfig::default());
    g.event_definitions.clear();
    g.trade("TDN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    for _ in 0..35 {
        g.next_month().unwrap();
    }
    assert!(!g.issuers["TDN"].defaulted);
    let before = g.portfolio.cash.0;
    let old = g.bonds["TDN-1"].clone();
    let redemption = FaceAmount(100_000_000)
        .value_at(credit_core::pricing::payment(&old, 36))
        .0;
    g.next_month().unwrap();
    assert_eq!(g.portfolio.cash.0 - before, redemption);
    assert!(!g.portfolio.positions.contains_key("TDN-1"));
    assert!(!g.portfolio.positions.contains_key("TDN-1-R36"));
    let new = &g.bonds["TDN-1-R36"];
    assert!(!new.settled);
    assert_eq!(new.maturity_month, 72);
    assert!(g.benchmark.positions.contains_key("TDN-1-R36"));
    assert!(!g.benchmark.positions.contains_key("TDN-1"));
    assert!(g.history.last().unwrap().attribution.residual.abs() <= 10);
    g.trade("TDN-1-R36", Side::Buy, FaceAmount(10_000_000))
        .unwrap();
    assert!(g.trade("TDN-1", Side::Buy, FaceAmount(10_000_000)).is_err());
    g.validate().unwrap();
}

#[test]
fn default_claim_survives_contractual_maturity_until_recovery_payment() {
    let mut g = new_game(GameConfig::default());
    g.event_definitions.clear();
    g.trade("YJSN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    g.bonds.get_mut("YJSN-1").unwrap().maturity_month = 1;
    let issuer = g.issuers.get_mut("YJSN").unwrap();
    issuer.defaulted = true;
    issuer.default_turn = Some(0);
    issuer.agency.published = Rating(19);
    g.reprice();
    let cash = g.portfolio.cash.0;
    g.next_month().unwrap();
    assert!(g.portfolio.positions.contains_key("YJSN-1"));
    assert_eq!(g.portfolio.cash.0, cash);
    g.next_month().unwrap();
    assert!(g.portfolio.positions.contains_key("YJSN-1"));
    assert_eq!(g.portfolio.cash.0, cash);
    assert!(g.history.last().unwrap().attribution.residual.abs() <= 10);
    g.next_month().unwrap();
    assert!(!g.portfolio.positions.contains_key("YJSN-1"));
    assert_eq!(g.portfolio.cash.0, cash + 40_000_000);
    assert!(g.history.last().unwrap().attribution.residual.abs() <= 10);
    assert!(!g.bonds.keys().any(|id| id.starts_with("YJSN-1-R")));
}
