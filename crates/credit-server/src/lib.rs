use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use credit_core::{model::*, units::*};
use credit_storage::{SaveEnvelope, SaveRepository, SqliteRepository, StorageError};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

struct Session {
    save: SaveEnvelope,
    repo: SqliteRepository,
    revision: u64,
    message: String,
}
#[derive(Clone)]
struct AppState {
    session: Arc<Mutex<Session>>,
    token: String,
    host: String,
}
#[derive(Default, Deserialize)]
struct QueryPage {
    tab: Option<String>,
    issuer: Option<String>,
    q: Option<String>,
    sort: Option<String>,
    all: Option<String>,
}
#[derive(Deserialize)]
struct Command {
    token: String,
    revision: u64,
    action: String,
    bond: Option<String>,
    amount: Option<i64>,
    seed: Option<u64>,
    difficulty: Option<String>,
}

async fn index(State(state): State<AppState>, Query(query): Query<QueryPage>) -> Response {
    let Ok(mut session) = state.session.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let page = credit_ui::Page {
        tab: query.tab.unwrap_or_default(),
        issuer: query.issuer.unwrap_or_default(),
        query: query.q.unwrap_or_default(),
        sort: query.sort.unwrap_or_default(),
        all: query.all.is_some(),
    };
    let message = std::mem::take(&mut session.message);
    let body = credit_ui::render(
        &session.save.game,
        &session.save.presentation,
        &page,
        &state.token,
        session.revision,
        &message,
    );
    ([(header::CACHE_CONTROL,"no-store"),(header::CONTENT_SECURITY_POLICY,"default-src 'self'; style-src 'self'; img-src 'self' data:; form-action 'self'; frame-ancestors 'none'; base-uri 'none'")],Html(body)).into_response()
}
// Preserve the current local page, including issuer/search/sort, without open redirects.
fn return_location(headers: &HeaderMap, host: &str) -> String {
    headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<axum::http::Uri>().ok())
        .filter(|u| {
            u.scheme_str() == Some("http")
                && u.authority().is_some_and(|a| a.as_str() == host)
                && u.path() == "/"
        })
        .and_then(|u| u.path_and_query().map(|p| p.as_str().to_owned()))
        .unwrap_or_else(|| "/".into())
}

async fn command(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(command): Form<Command>,
) -> Response {
    if command.token != state.token
        || headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&state.host)
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        if origin.to_str().ok() != Some(format!("http://{}", state.host).as_str()) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let return_to = if command.action == "new" {
        "/".into()
    } else {
        return_location(&headers, &state.host)
    };
    let Ok(mut session) = state.session.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if command.revision != session.revision {
        session.message =
            "古い画面からの注文を拒否しました。最新の価格で再確認してください。".into();
        return Redirect::to(&return_to).into_response();
    }
    let before = session.save.clone();
    let result: Result<String, String> = (|| {
        let game = &mut session.save.game;
        match command.action.as_str() {
            "buy" | "sell" => {
                let id = command.bond.as_deref().ok_or("銘柄未指定")?;
                let face = FaceAmount(command.amount.ok_or("数量未指定")?);
                let side = if command.action == "buy" {
                    Side::Buy
                } else {
                    Side::Sell
                };
                let trade = game.trade(id, side, face).map_err(|e| e.to_string())?;
                Ok(format!(
                    "{} {} 約定 {:.3} / 額面 {} 億円 / 売買コスト {} 円",
                    id,
                    if side == Side::Buy {
                        "購入"
                    } else {
                        "売却"
                    },
                    trade.price.value(),
                    credit_presentation::oku(face.0),
                    trade.cost.0
                ))
            }
            "next" => {
                game.next_month().map_err(|e| e.to_string())?;
                let turn = game.turn;
                let Session { save, .. } = &mut *session;
                save.presentation.update(&save.game);
                Ok(format!("{}か月目の市場が更新されました。", turn))
            }
            "new" => {
                let meme_enabled = session.save.presentation.meme_enabled;
                let achievements = session.save.presentation.achievements.clone();
                let difficulty = match command.difficulty.as_deref() {
                    Some("Rookie") => Difficulty::Rookie,
                    Some("PM") => Difficulty::Pm,
                    Some("Distressed") => Difficulty::Distressed,
                    Some("Beast") => Difficulty::Beast,
                    _ => Difficulty::Analyst,
                };
                session.save = SaveEnvelope::new(credit_data::new_game(GameConfig {
                    seed: command.seed.unwrap_or(1919),
                    difficulty,
                    ..Default::default()
                }));
                session.save.presentation.meme_enabled = meme_enabled;
                session.save.presentation.achievements = achievements;
                Ok(
                    "新しい運用を開始しました。初期65億円です。配分後に「翌月へ」で1か月進みます。"
                        .into(),
                )
            }
            "meme" => {
                session.save.presentation.meme_enabled = !session.save.presentation.meme_enabled;
                if !session.save.presentation.meme_enabled {
                    session.save.presentation.current_meme = None;
                }
                Ok("Meme表示設定を変更しました。".into())
            }
            "save" => {
                let save = session.save.clone();
                session
                    .repo
                    .save("slot1", &save)
                    .map_err(|e| e.to_string())?;
                Ok("Slot 1に保存しました。".into())
            }
            "load" => {
                session.save = session.repo.load("slot1").map_err(|e| e.to_string())?;
                Ok("Slot 1を読み込みました。".into())
            }
            _ => Err("未対応の操作です。".into()),
        }
    })();
    match result {
        Ok(message) => {
            let Session { save, .. } = &mut *session;
            credit_presentation::achievements::update(&mut save.presentation, &save.game);
            let save = session.save.clone();
            if let Err(error) = session.repo.save("autosave", &save) {
                session.save = before;
                session.message = format!("保存に失敗したため操作を取り消しました: {error}");
            } else {
                session.revision += 1;
                session.message = message;
            }
        }
        Err(error) => {
            session.save = before;
            session.message = error;
        }
    }
    Redirect::to(&return_to).into_response()
}
async fn export(State(state): State<AppState>) -> Response {
    let Ok(session) = state.session.lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match serde_json::to_string_pretty(&session.save) {
        Ok(json) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=tdn-credit-save.json",
                ),
                (header::CACHE_CONTROL, "no-store"),
            ],
            json,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
pub async fn run(
    port: u16,
    repo: SqliteRepository,
    config: GameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let save = match repo.load("autosave") {
        Ok(save) => save,
        Err(StorageError::Empty) => SaveEnvelope::new(credit_data::new_game(config)),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("session entropy: {e}"))?;
    let token = blake3::hash(&bytes).to_hex().to_string();
    let host = format!("127.0.0.1:{port}");
    let state = AppState {
        session: Arc::new(Mutex::new(Session {
            save,
            repo,
            revision: 0,
            message: String::new(),
        })),
        token,
        host: host.clone(),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/command", post(command))
        .route("/export", get(export))
        .route(
            "/assets/style.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../assets/style.css"),
                )
            }),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&host).await?;
    println!("TDN CREDIT — http://{host} / Ctrl+C to stop");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_return_to_local_page_and_preserve_filters() {
        let mut headers = HeaderMap::new();
        for path in [
            "/?tab=portfolio",
            "/?tab=news",
            "/?tab=analysis",
            "/?tab=issuers&issuer=TNOK&q=TN&sort=yield&all=1",
        ] {
            headers.insert(
                header::REFERER,
                format!("http://127.0.0.1:3000{path}").parse().unwrap(),
            );
            assert_eq!(return_location(&headers, "127.0.0.1:3000"), path);
        }
        for referer in [
            "https://example.com/?tab=news",
            "http://127.0.0.1:9999/",
            "http://127.0.0.1:3000/command",
            "//example.com/",
            "bad",
        ] {
            headers.insert(header::REFERER, referer.parse().unwrap());
            assert_eq!(return_location(&headers, "127.0.0.1:3000"), "/");
        }
        assert_eq!(return_location(&HeaderMap::new(), "127.0.0.1:3000"), "/");
    }
}
