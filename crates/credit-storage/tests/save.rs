use credit_core::{model::*, units::*};
use credit_storage::*;

#[test]
fn sqlite_roundtrip_resumes_identical_market_and_trades() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.sqlite");
    let mut repo = SqliteRepository::open(&path).unwrap();
    let mut save = SaveEnvelope::new(credit_data::new_game(GameConfig::default()));
    save.game
        .trade("TDN-1", Side::Buy, FaceAmount(1_000_000_000))
        .unwrap();
    for _ in 0..40 {
        save.game.next_month().unwrap();
    }
    save.presentation.meme_enabled = false;
    repo.save("manual", &save).unwrap();
    drop(repo);
    let repo = SqliteRepository::open(&path).unwrap();
    let mut loaded = repo.load("manual").unwrap();
    assert!(!loaded.presentation.meme_enabled);
    while save.game.turn < 60 {
        save.game.next_month().unwrap();
        loaded.game.next_month().unwrap();
    }
    assert_eq!(
        serde_json::to_string(&save.game).unwrap(),
        serde_json::to_string(&loaded.game).unwrap()
    );
}
#[test]
fn rejects_incompatible_and_invalid_saves_without_overwriting() {
    let mut repo = SqliteRepository::open(std::path::Path::new(":memory:")).unwrap();
    let valid = SaveEnvelope::new(credit_data::new_game(GameConfig::default()));
    repo.save("a", &valid).unwrap();
    let mut invalid = valid.clone();
    invalid.game.portfolio.cash.0 = -1;
    assert!(repo.save("a", &invalid).is_err());
    invalid = valid.clone();
    invalid.schema = 99;
    assert!(repo.save("a", &invalid).is_err());
    assert_eq!(repo.load("a").unwrap().game.nav(), valid.game.nav());
    assert!(matches!(repo.load("empty"), Err(StorageError::Empty)));
}
