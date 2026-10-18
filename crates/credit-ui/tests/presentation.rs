use credit_core::{model::*, units::*};
use credit_presentation::PresentationState;

#[test]
fn critical_held_news_survives_three_item_limit_and_meme_is_separate() {
    let mut g = credit_data::new_game(GameConfig::default());
    g.trade("TDN-1", Side::Buy, FaceAmount(100_000_000))
        .unwrap();
    for n in 0..4 {
        g.news.push(NewsItem {
            turn: 0,
            event_id: "critical".into(),
            issuer: Some("TDN".into()),
            headline: format!("Important disclosure {n}"),
            detail: "Numeric credit detail".into(),
            severity: 3,
        });
    }
    let p = PresentationState {
        meme_enabled: false,
        current_meme: Some("ファッ！？".into()),
        ..Default::default()
    };
    let html = credit_ui::render(&g, &p, &credit_ui::Page::default(), "test-token", 0, "");
    for n in 0..4 {
        assert!(html.contains(&format!("Important disclosure {n}")));
    }
    assert!(!html.contains("ファッ！？"));
}

#[test]
fn continuation_keeps_controls_enabled_and_internal_truth_hidden() {
    let mut g = credit_data::new_game(GameConfig::default());
    g.event_definitions.clear();
    for _ in 0..40 {
        g.next_month().unwrap();
    }
    let p = PresentationState::default();
    let html = credit_ui::render(
        &g,
        &p,
        &credit_ui::Page {
            tab: "analysis".into(),
            ..Default::default()
        },
        "token",
        0,
        "",
    );
    assert!(html.contains("40か月時点の運用結果"));
    assert!(html.contains("継続運用できます"));
    assert!(html.contains("40か月目"));
    assert!(!html.contains("Internal credit"));
    assert!(!html.contains("内部信用スコア"));
    let next = html.find("value=\"next\"").unwrap();
    let end = html[next..].find("</button>").unwrap() + next;
    assert!(!html[next..end].contains("disabled"));
    assert!(html[next..end].contains("翌月へ"));
}
