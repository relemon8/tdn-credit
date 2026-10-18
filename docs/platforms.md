# Platform verification

> 下表は初期v0.1.0の検証実績です。v0.1.1の配布可否・動作確認は配布物のBUILD_INFO.jsonを優先してください。未検証の新バイナリを過去の実績だけで検証済みとは表示しません。

Development order: Linux, OpenBSD, macOS, Android, Windows. The shipping model is
a Rust local server plus a browser, with CSS embedded in the executable.

| Platform | Checked | Scope / limits |
|---|---|---|
| Linux x86_64 | musl static release build; QEMU Linux execution; finance tests; 24-month headless; HTTP trading/24 turns; restart/save reload | No runtime language tools or external network required |
| Linux aarch64 | musl static release build; native-virtualized Linux; workspace test binaries; headless/HTTP/restart | Offline VM with no network interface; loopback only |
| OpenBSD 7.9 arm64 | native Rust 1.94.1 build/tests; headless; full HTTP replay | QEMU/HVF VM; no Linux-only API substitutions needed; amd64 still needs native validation |
| macOS arm64 | Rust 1.99.1 workspace tests, Clippy, release build, headless and full HTTP replay | Firefox desktop/mobile viewport QA; browser engine is native Firefox |
| Android | `aarch64-linux-android` Core cross-check; responsive browser layout | No APK, native server packaging or physical Android runtime validation yet |
| Windows | `x86_64-pc-windows-gnu` Core cross-check; platform save path implementation | Full Windows executable/runtime validation remains; CI job supplied |

Source-only cross-checks are not runtime certification. CI files are supplied,
but have not been pushed to or executed by a remote CI service in this session.

## Linux build

Native Linux requires Rust/Cargo and a C compiler to build. Playing a release
binary requires neither. For cross-building static Linux releases from macOS,
the QA environment used cargo-zigbuild and Zig 0.15.2:

```sh
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
cargo zigbuild --release --locked --target x86_64-unknown-linux-musl -p tdn-credit
cargo zigbuild --release --locked --target aarch64-unknown-linux-musl -p tdn-credit
```

Zig/cargo-zigbuild are development tools, not gameplay dependencies. A linker
warning about deprecated optimization setting `1` was emitted by Zig 0.15.2;
the artifacts linked and executed successfully. Native Clippy is warning-free.

## OpenBSD build

Use the OS package toolchain (Rust >=1.89; verified 1.94.1), not a Linux rustup
toolchain. Rust's official support notes explain the platform's distribution
and build constraints: <https://doc.rust-lang.org/rustc/platform-support/openbsd.html>.

```sh
doas pkg_add rust
cargo test --workspace --locked
cargo build --release --locked -p tdn-credit
./target/release/tdn-credit --portable --no-open
```

The bundled SQLite C source built on OpenBSD. Core uses the portable Rust BLAKE3
implementation and seeded ChaCha; networking/filesystem/database remain outside
Core. `xdg-open` is optional: if unavailable, open the printed URL manually.

## Android and Windows remaining work

Android's first deliverable is the shared responsive UI. A standalone device
also needs a way to execute the local Rust server. This repository does not
claim that viewport emulation is an Android port. Native Android/Termux or a
wrapper, lifecycle handling and device save-path tests are still required.

Windows uses Local App Data, and the browser-launch helper is isolated. Native
MSVC/SQLite build, save paths, browser launch and real browser play still need
to be exercised on Windows. These lower-priority platforms are not included in
the verified binary releases.

## Determinism

Within an engine/data version and platform, seeded replay and save/resume are
exact. Fixed RNG domains and sorted map iteration prevent incidental UI or
order-entry calls from shifting financial random streams.

For seed 1919, Linux, OpenBSD and macOS produced identical monthly integer-JPY
NAV and benchmark NAV. Floating intermediate results can differ by platform
libm/compiler; observed maximum differences were below 1e-12. Bit-identical
cross-platform floating state is not promised. Financial comparisons and
attribution tests use explicit tolerances where appropriate.
