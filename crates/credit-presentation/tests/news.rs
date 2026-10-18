use credit_core::model::*;
use credit_presentation::news;
#[test]
fn newsroom_covers_catalog_without_mutating_events_or_numbers() {
    for event in credit_data::events::catalog() {
        assert!(news::lead(&event.id).is_some(), "{}", event.id);
    }
    let n = NewsItem {
        turn: 1,
        event_id: "debt_acquisition".into(),
        issuer: Some("TNOK".into()),
        headline: "買収".into(),
        detail: "Net leverage 3.80x → 5.70x / FCF -12.3 / Debt 42.0 / Market shock +84bp".into(),
        severity: 3,
    };
    let before = serde_json::to_string(&n).unwrap();
    assert!(news::headline(&n).starts_with("TNOK、"));
    let text = news::detail(&n);
    for number in ["3.80", "5.70", "-12.3", "42.0", "+84bp"] {
        assert!(text.contains(number));
    }
    assert_eq!(before, serde_json::to_string(&n).unwrap());
    assert!(news::lead("default").unwrap().contains("支払い"));
    let unknown = NewsItem {
        event_id: "custom".into(),
        issuer: None,
        headline: "重要なお知らせ".into(),
        ..n
    };
    assert_eq!(news::headline(&unknown), "重要なお知らせ");
    assert!(news::lead("custom").is_none());
}
