# TDN CREDIT

v0.1.1 / 完全無料・MIT・ローカル保存

架空のクレジットファンドを運用する、ローカル実行のブラウザゲーム。
初期65億円・Cash 100%、10 Issuer / 初期20 Bond、24か月で成績評価・以降も継続可能。

良い会社と、良い価格の投資は別物です。ニュース、利回り、Spread、
企業のCash Flowを観察し、ポジションを取り、損益の理由を振り返ります。
クイズや自動売買推奨はありません。

## 配布版の選び方

配布フォルダ名のOSとCPUを確認してください。Linuxのx86_64は一般的なIntel/AMD PC、aarch64はARM64です。macOSのaarch64はApple Silicon、x86_64はIntel Mac用です。
各版にREADME、はじめに.txt、LICENSE、依存ライセンス、BUILD_INFO.jsonを同梱します。BUILD_INFO.jsonにはその版の動作確認状況を明記しています。

GitHubのソース一覧からはビルドが必要です。遊ぶだけならReleasesの対応OS用zipまたはtar.gzを取得してください。

[GitHubでの公開手順](docs/github-release.md) / [更新履歴](CHANGELOG.md)

## 起動

配布バイナリがある場合は、展開したディレクトリで：

```sh
./tdn-credit --portable
```

ブラウザで **http://127.0.0.1:3000** を開きます。
ブラウザが自動で開かない環境では、表示されたURLを手動で開いてください。
Ctrl+Cで終了。操作ごとにローカルへAutosaveします。

ソースからビルドする場合は、Rust 1.89以上、Cargo、Cコンパイラが必要です。
現在の検証ツールチェーンはRust 1.99.1です。SQLiteは同梱ソースからビルドします。

```sh
cargo build --release --locked
./target/release/tdn-credit --portable
```

初回ビルドには依存crateの取得が必要です。キャッシュ後は`--offline`でビルド可能。
**ゲーム実行にはRust、Node.js、npm、Docker、Java、Python、Cloud Account、
API Key、.envは不要です。** Pythonは任意の開発用QAスクリプトだけに使用します。

```sh
./tdn-credit --no-open --port 3001
./tdn-credit --headless --seed 1919
./tdn-credit --headless --seed 42 --difficulty distressed
./tdn-credit --headless --dump-json scenario.json
```

`--seed`と`--difficulty`は新規セーブを作る場合の初期値です。
Autosaveがある場合はそちらを再開します。画面下の「セーブ・設定・新しいゲーム」でシナリオ番号・難易度を選び、新しいゲームを開始できます。

## 遊び方

1. 0か月目に初期配分。「市場」の注目3社、または「企業・債券」から選びます。
2. 額面を円で入力して「買う」または「売る」。1,000万円単位、初期入力は10億円。
3. BUYはAsk、SELLはBid。表示する注文価格は経過利息込みのDirty価格。
4. 「翌月へ」で1か月進み、ニュース、企業開示、金利、Spreadを確認。
5. 「保有資産」と「運用分析」で保有リスクと損益要因を確認。
6. 24か月目で成績とRankを確認。そのまま「翌月へ」で25か月目以降も継続できます。運用分析は現在までの成績を表示します。

Cashを維持することも選択です。Cash金利は0%。Short、Leverage借入、
Derivatives、実在証券、為替取引はありません。

画面の3層：MARKET概要 → Issuer／Bond詳細 → 折りたたみの財務履歴・
Exposure・Stress・Bond別Attribution。全操作をキーボードでも行えます。
NEXT MONTHのaccess keyは`x`（修飾キーはブラウザ／OSにより異なります）。

### Spreadの読み方

- Market Spread：実際に価格へ使う市場のSpread。
- Desk fair推計：前回開示値とモデルに基づく要求Spreadの範囲。正解表示ではありません。
- Internal Fair：Simulation内部の要求Spread。運用中は直接公開しません。
- 公表Rating：内部Scoreの継続的変化をAgencyが確認してから変更。

100bp = 1%。価格は額面100当たり。PortfolioのNAVはMid評価で、
その価格ですぐに売れるとは限りません。推定清算価値はBidベースです。

## Save / Load

画面下の「セーブ・設定・新しいゲーム」で「セーブする」「セーブを読み込む」を使用します。Autosaveは毎操作時に保存。
Exportは現在の状態をJSONとして取得します。Importは起動時に指定できます。

```sh
./tdn-credit --portable --import tdn-credit-save.json
```

Import前の有効なAutosaveは`before-import`としてDB内に保存します。
異なるSchema／Engine／Dataバージョンや不正な状態は拒否します。
非互換セーブを黙って初期化・上書きしません。

| OS | 標準Data保存先 |
|---|---|
| Linux / OpenBSD | `$XDG_DATA_HOME/tdn-credit`、未設定時`~/.local/share/tdn-credit` |
| macOS | `~/Library/Application Support/TDN CREDIT`（XDG指定時はそちら） |
| Windows | `%LOCALAPPDATA%/TDN CREDIT` |
| Portable | 現在の作業ディレクトリの`./data/` |

## 実装

Cargo Workspace。`credit-core`はOS API、Filesystem、Network、UI、SQLiteに依存しません。
Coreの乱数はChaCha8で用途／Turn／Issuerごとに分離。金融計算には実時間を使用しません。
Leptos SSR + Axum + SQLite。CSSをバイナリに埋め込み、外部Font／CDNなし。
MVPのUIはHTMLフォーム方式で、JS／WASM／Nodeのビルドを必要としません。

```text
app/                         CLIと起動
crates/credit-core/           Simulation / Pricing / Credit / Trading / Risk
crates/credit-data/           初期Universe / 50 Event definitions
crates/credit-presentation/   表示 / 独立Meme System
crates/credit-ui/             Leptos / 5画面 / Charts
crates/credit-server/         Axum / localhost / 操作Token / 二重送信防止
crates/credit-storage/        Repository / SQLite / Versioned save
crates/credit-platform/       保存先 / Portable / Browser起動
assets/                      同梱CSS
docs/                        設計・金融モデル・検証・開発記録
```

## 検証

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check licenses
```

Financial Unit／Property Test、売買Invariant、DefaultとRecovery、
決定性、Save再開、Benchmark価格一致、Memeによる金融不変を検証します。
ブラウザ通しプレイ用の`python3 scripts/http-smoke.py URL`は標準ライブラリのみ使用。
テスト用New Gameを開始するため、普段のセーブとは別のPortableディレクトリで実行してください。

## プラットフォーム方針

開発優先順位は **Linux → OpenBSD → macOS → Android → Windows**。
Linuxはx86_64／aarch64を対象とします。
OSごとの検証済み範囲は[platforms.md](docs/platforms.md)を参照してください。
CI設定の存在だけを対応完了とは扱いません。

OpenBSDではOS提供のRust／Cargoを使用します。CoreにLinux固有APIはなく、
ブラウザ起動失敗はゲーム起動を妨げません。SQLite以外のネイティブC依存を抑えます。
AndroidはまずBrowser UIを共有し、単体実行はRustサーバーの実行環境が必要です。
APK／PWAの配布を、レスポンシブ表示だけで実装済みとは扱いません。

## 無料・OSS・開発方針

- TDN CREDITは架空の企業・架空の市場を使う教育・娯楽目的のゲームです。
- 実在企業・証券とは関係ありません。投資助言ではありません。
- ゲーム内の資金・損益は架空です。実際のお金は使用せず、換金もできません。
- 完全無料・オープンソースです。ゲーム本体は[MIT](LICENSE)。
- 開発にはLLMによるコーディング支援を使用しています。
- 金融モデルはゲーム用に簡略化されています。[金融モデル](docs/financial-model.md)に制約を記載。
- 外部APIやアカウントなしで主要機能を利用できます。LLM APIも不要です。
- 課金、広告、ガチャ、NFT、暗号資産、ユーザーデータ販売は実装しません。
- Telemetry／Analyticsを送信しません。セーブはローカル保存です。
- 一部テロップやDesk Commentには、広く使われる短いネットミーム表現を使用します。
- 語録テロップは画面下の設定から無効化可能。ON/OFFで市場シナリオは変わりません。
- 元映像、実在人物の画像、音声は同梱しません。
- 依存ライセンスは`cargo deny check licenses`で確認し、[Notices](THIRD_PARTY_NOTICES.md)へ収録。

画面例：[MARKET](docs/screenshots/market-desktop.png) / [390px表示](docs/screenshots/market-mobile.png) / [Bond一覧](docs/screenshots/bond-directory.png)。

現在は最初のローカルMVPです。バランス調整、OS別の実機検証、アクセシビリティの
追加検証を継続します。[開発記録](docs/development-log.md)に実装と残課題を記録しています。

### 初期資産と実績

新しいゲームの初期資産は65億円です。従来の100億円セーブは初期元本を保持して読み込めます。
各画面下部の「実績図鑑」に45個の実績があります。資産額・売買・イベント・完走で解除し、報酬や金融計算への影響はありません。
実績は現在のセーブに保存し、「新しいゲーム」へ引き継ぎます。古いセーブをロードすると実績もその時点に戻ります。
語録テロップOFF時は実績名も通常の条件文で表示します。
114,514億円の実績はロマン枠で、通常の24か月プレイでの到達を想定していません。

### はじめて遊ぶとき

画面下部の「遊び方：30秒版 / 詳しい版」を開いてください。
30秒版は毎月の3手順、詳しい版は注文・用語・損益の見方を3〜5分で確認できます。
遊び方は初期状態では閉じています。必要なときに短い版・詳しい版を開けます。

### 24か月以降の継続運用

24か月は終了ではなく評価の目安です。既存の24か月到達済みセーブも、そのまま翌月へ進めます。
保有資産・現金・実績・履歴を引き継ぎ、売買と保存を続けられます。
満期の旧債は元本と利息を現金で受け取り、破綻していない企業の新しい借り換え債（短期側3年・長期側7年）が登場します。
プレイヤーの再投資は任意です。比較指数は償還元本のみ同一企業の新債へ再投資し、受取利息は現金で保有します。
新債は既存の企業資金調達モデル内で行う借り換えの市場表現であり、企業の負債を二重に加算しません。
償還済み債券は「企業・債券」の「償還済みも表示」で参照できます。破綻企業は自動復活しません。
ヘッドレスCLIのサンプル運用は従来どおり24か月（設定により36か月）で停止します。
