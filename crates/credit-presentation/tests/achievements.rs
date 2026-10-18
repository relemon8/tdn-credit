use credit_core::{model::*, GameState};
use credit_presentation::{achievements::*, PresentationState};
fn game() -> GameState {
    credit_data::new_game(GameConfig::default())
}
#[test]
fn achievements_observe_without_financial_side_effects() {
    let mut g = game();
    assert_eq!(g.nav().0, 6_500_000_000);
    let mut p = PresentationState::default();
    let before = serde_json::to_string(&g).unwrap();
    update(&mut p, &g);
    assert!(p.achievements.contains_key("assets65"));
    assert!(!p.achievements.contains_key("assets114514"));
    assert_eq!(serde_json::to_string(&g).unwrap(), before);
    g.portfolio.cash = credit_core::units::Money(11_451_400_000_000 - 1);
    update(&mut p, &g);
    assert!(!p.achievements.contains_key("assets114514"));
    g.portfolio.cash.0 += 1;
    update(&mut p, &g);
    assert!(p.achievements.contains_key("assets114514"));
    g.portfolio.cash.0 = 0;
    update(&mut p, &g);
    assert!(p.achievements.contains_key("assets114514"));
    let restored: PresentationState =
        serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(p.achievements, restored.achievements);
}
#[test]
fn event_badges_and_trades_have_explicit_conditions_and_no_meme_dependency() {
    let mut g = game();
    let mut p = PresentationState {
        meme_enabled: false,
        ..Default::default()
    };
    update(&mut p, &g);
    assert!(!p.achievements.contains_key("fraud"));
    g.news.push(NewsItem {
        turn: 0,
        event_id: "fraud".into(),
        issuer: Some("TDN".into()),
        headline: "不正調査".into(),
        detail: "数値は別途".into(),
        severity: 3,
    });
    update(&mut p, &g);
    assert!(p.achievements.contains_key("fraud"));
    g.trade(
        "TDN-1",
        Side::Buy,
        credit_core::units::FaceAmount(10_000_000),
    )
    .unwrap();
    update(&mut p, &g);
    assert!(p.achievements.contains_key("buy"));
    assert!(!p.achievements.contains_key("sell"));
    g.trade(
        "TDN-1",
        Side::Sell,
        credit_core::units::FaceAmount(10_000_000),
    )
    .unwrap();
    update(&mut p, &g);
    assert!(p.achievements.contains_key("sell"));
    assert!(p.current_meme.is_none());
    let ids = CATALOG
        .iter()
        .map(|a| a.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), CATALOG.len());
    assert_eq!(CATALOG.len(), 45);
    for a in CATALOG {
        if let Condition::Event(id) = a.condition {
            assert!(g.event_definitions.iter().any(|e| e.id == id), "{id}");
        }
    }
}
#[test]
fn older_saves_keep_their_hundred_oku_return_baseline() {
    let mut value = serde_json::to_value(game()).unwrap();
    value.as_object_mut().unwrap().remove("initial_capital");
    value["portfolio"]["cash"] = serde_json::json!(10_000_000_000i64);
    let old: GameState = serde_json::from_value(value).unwrap();
    assert_eq!(old.initial_capital.0, 10_000_000_000);
    assert!(old.validate().is_ok());
    let mut new = game();
    for _ in 0..24 {
        new.next_month().unwrap();
    }
    let result = credit_core::analysis::final_score(&new);
    assert_eq!(result.total_return, 0.);
}
