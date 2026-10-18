# MVP verification record

> 以下のOS横断記録と31テストは初期v0.1.0の記録です。v0.1.1は40テスト・65億円開始・継続運用に更新されています。最新版のOS別確認結果は各配布物のBUILD_INFO.jsonとRELEASE_MANIFEST.jsonを参照してください。

## Automated checks

- Rust unit/integration tests: 31 named tests, including a property test that
  generates 32 seeds and runs all five difficulties for 24 months each.
- Cash, quantities, finite prices, save validity, debt/maturity conservation,
  and attribution residual bounds are checked throughout those paths.
- Financial tests cover unit conversions, monthly PD, DCF sensitivities, fair
  spread direction, one-notch/evidence-lag rating changes, default/recovery,
  coupon suppression, ask/bid execution, and rejected-order atomicity.
- Replay tests cover save/resume, index/portfolio identical-holdings parity,
  meme ON/OFF financial equality and phrase cooldowns.
- UI presentation test ensures all critical held-issuer disclosures survive the
  normal three-item news limit and disabled memes never appear.
- Native `cargo fmt --all --check` and Clippy with `-D warnings` pass.
- `cargo deny check licenses` passes. Dependency notices accompany releases.

## Server replay

`scripts/http-smoke.py` verifies all five pages, initial 10/20 universe and cash,
invalid-token rejection, stale-order rejection, buy/sell holdings, manual save/
load, two complete 24-turn runs, final scoring, and identical market paths with
memes enabled/disabled. It uses a separate disposable save directory.

Linux VM QA additionally verified restart/load from SQLite. Linux guests were
given no external API or account; aarch64 QA had no external network interface.
OpenBSD used its native packaged compiler and full HTTP replay inside the VM.

## Browser QA

Firefox headless is a development-only tool. Desktop and narrow mobile viewport
checks cover navigation, issuer/bond directory, search/sort, order submission,
analysis, settings and document overflow. Screenshots are optional QA artifacts,
not bundled game imagery. A true 390px CSS layout was checked in an iframe snapshot (Firefox macOS
headless windows themselves have a 500px minimum). Physical Android/Windows
and screen-reader testing
are not substituted by viewport checks.

## Known remaining work

- Longer human playtesting and parameter/rank balance across many strategies.
- Physical Android/Windows validation and distribution; OpenBSD amd64 testing.
- Broader assistive-technology/browser testing.
- Optional UI hydration if a future feature needs richer client interaction.
- Full hazard/recovery term structures, callable debt and endogenous price impact
  are outside the current MVP; see financial-model.md for model simplifications.

No hosted site, GitHub repository or public release was created automatically.
The local source, MIT license, CI definitions and distributable binaries are
prepared for review and publication.
