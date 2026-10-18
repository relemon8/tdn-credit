use credit_core::GameState;
use credit_presentation::PresentationState;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEnvelope {
    pub schema: u32,
    pub game: GameState,
    pub presentation: PresentationState,
}
impl SaveEnvelope {
    pub fn new(game: GameState) -> Self {
        let mut presentation = PresentationState::default();
        credit_presentation::achievements::update(&mut presentation, &game);
        Self {
            schema: 1,
            game,
            presentation,
        }
    }
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.schema != 1
            || self.game.engine_version != credit_core::model::ENGINE_VERSION
            || self.game.data_version != credit_core::model::DATA_VERSION
        {
            return Err(StorageError::Version);
        }
        self.game
            .validate()
            .map_err(|e| StorageError::Invalid(e.to_string()))
    }
}
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("save version is incompatible")]
    Version,
    #[error("save slot is empty")]
    Empty,
    #[error("invalid save: {0}")]
    Invalid(String),
}
pub trait SaveRepository {
    fn save(&mut self, slot: &str, save: &SaveEnvelope) -> Result<(), StorageError>;
    fn load(&self, slot: &str) -> Result<SaveEnvelope, StorageError>;
}
pub struct SqliteRepository {
    connection: Connection,
}
impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err(StorageError::Version);
        }
        connection.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS saves(slot TEXT PRIMARY KEY, payload TEXT NOT NULL); PRAGMA user_version=1; COMMIT;")?;
        Ok(Self { connection })
    }
}
impl SaveRepository for SqliteRepository {
    fn save(&mut self, slot: &str, save: &SaveEnvelope) -> Result<(), StorageError> {
        save.validate()?;
        let payload = serde_json::to_string(save)?;
        let transaction = self.connection.transaction()?;
        transaction.execute("INSERT INTO saves(slot,payload) VALUES (?1,?2) ON CONFLICT(slot) DO UPDATE SET payload=excluded.payload",params![slot,payload])?;
        transaction.commit()?;
        Ok(())
    }
    fn load(&self, slot: &str) -> Result<SaveEnvelope, StorageError> {
        let payload: String = self
            .connection
            .query_row("SELECT payload FROM saves WHERE slot=?1", [slot], |r| {
                r.get(0)
            })
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    StorageError::Empty
                } else {
                    StorageError::Sql(e)
                }
            })?;
        let save: SaveEnvelope = serde_json::from_str(&payload)?;
        save.validate()?;
        Ok(save)
    }
}
