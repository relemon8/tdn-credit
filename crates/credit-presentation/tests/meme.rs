use credit_core::model::*;
use credit_presentation::PresentationState;

#[test]
fn meme_never_changes_financial_state_or_important_news() {
    for seed in 0..20 {
        let mut a = credit_data::new_game(GameConfig {
            seed,
            ..Default::default()
        });
        let mut b = a.clone();
        let mut enabled = PresentationState::default();
        let mut disabled = PresentationState {
            meme_enabled: false,
            ..Default::default()
        };
        let mut used = std::collections::BTreeMap::new();
        for _ in 0..24 {
            a.next_month().unwrap();
            b.next_month().unwrap();
            let news = serde_json::to_string(&a.news).unwrap();
            enabled.update(&a);
            disabled.update(&b);
            assert!(disabled.current_meme.is_none());
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap()
            );
            assert_eq!(news, serde_json::to_string(&a.news).unwrap());
            if let Some(text) = &enabled.current_meme {
                if let Some(last) = used.insert(text.clone(), a.turn) {
                    assert!(a.turn - last >= 6);
                }
            }
        }
    }
}
