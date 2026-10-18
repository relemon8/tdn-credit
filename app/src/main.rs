use credit_core::{model::*, units::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("TDN CREDIT {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let seed = args
        .iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1919);
    let difficulty = match args
        .iter()
        .position(|a| a == "--difficulty")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
        .unwrap_or("analyst")
    {
        "rookie" => Difficulty::Rookie,
        "analyst" => Difficulty::Analyst,
        "pm" => Difficulty::Pm,
        "distressed" => Difficulty::Distressed,
        "beast" => Difficulty::Beast,
        _ => return Err("difficulty must be rookie/analyst/pm/distressed/beast".into()),
    };
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("TDN CREDIT\n  --portable  save in ./data\n  --port N    loopback port (default 3000)\n  --seed N    new-game seed\n  --no-open   do not launch browser\n  --headless  run 24-month sample portfolio\n  --difficulty rookie|analyst|pm|distressed|beast\n  --dump-json PATH  export headless financial state\n  --import PATH     import Save JSON on startup");
        return Ok(());
    }
    if !args.iter().any(|a| a == "--headless") {
        let port = args
            .iter()
            .position(|a| a == "--port")
            .and_then(|i| args.get(i + 1))
            .map(|s| s.parse())
            .transpose()?
            .unwrap_or(3000);
        let dir = credit_platform::data_dir(args.iter().any(|a| a == "--portable"))?;
        std::fs::create_dir_all(&dir)?;
        println!("Save directory: {}", dir.display());
        let mut repo = credit_storage::SqliteRepository::open(&dir.join("tdn-credit.sqlite"))?;
        if let Some(path) = args
            .iter()
            .position(|a| a == "--import")
            .and_then(|i| args.get(i + 1))
        {
            use credit_storage::SaveRepository;
            let bytes = std::fs::read(path)?;
            let save: credit_storage::SaveEnvelope = serde_json::from_slice(&bytes)?;
            save.validate()?;
            if let Ok(old) = repo.load("autosave") {
                repo.save("before-import", &old)?;
            }
            repo.save("autosave", &save)?;
        }
        if !args.iter().any(|a| a == "--no-open") {
            let url = format!("http://127.0.0.1:{port}");
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(800));
                let _ = credit_platform::open_browser(&url);
            });
        }
        return credit_server::run(
            port,
            repo,
            GameConfig {
                seed,
                difficulty,
                ..Default::default()
            },
        )
        .await;
    }
    let mut game = credit_data::new_game(GameConfig {
        seed,
        difficulty,
        ..Default::default()
    });
    println!("TDN CREDIT — headless simulation / seed {seed}");
    for id in ["TDN-1", "HTN-1", "TNOK-1", "DB-2"] {
        game.trade(id, Side::Buy, FaceAmount(1_000_000_000))?;
    }
    println!("Month    NAV (JPY)     Monthly     Benchmark     Residual");
    while !game.evaluation_complete() {
        let initial_capital = game.initial_capital.0;
        let r = game.next_month()?;
        println!(
            "{:>5} {:>14} {:>10.2}% {:>12.2}% {:>12}",
            r.turn,
            r.nav.0,
            r.monthly_return * 100.,
            (r.benchmark_nav.0 as f64 / initial_capital as f64 - 1.) * 100.,
            r.attribution.residual
        );
    }
    println!(
        "Complete. Return {:+.2}% / benchmark {:+.2}%",
        (game.nav().0 as f64 / game.initial_capital.0 as f64 - 1.) * 100.,
        (game.benchmark_nav().0 as f64 / game.initial_capital.0 as f64 - 1.) * 100.
    );
    println!("Rank: {}", credit_core::analysis::final_score(&game).rank);
    if let Some(path) = args
        .iter()
        .position(|a| a == "--dump-json")
        .and_then(|i| args.get(i + 1))
    {
        std::fs::write(path, serde_json::to_vec_pretty(&game)?)?;
    }
    Ok(())
}
