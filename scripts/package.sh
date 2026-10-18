#!/bin/sh
set -eu
# Build-time only. Python packages archives; players do not need Python.
release_target=${1:-}
if [ -n "$release_target" ]; then
    cargo build --release --locked --target "$release_target" -p tdn-credit
    release_binary="target/$release_target/release/tdn-credit"
else
    cargo build --release --locked -p tdn-credit
    release_binary="target/release/tdn-credit"
    release_target=$(rustc -vV | sed -n 's/^host: //p')
fi
case "$release_target" in *windows*) release_binary="$release_binary.exe" ;; esac
python3 scripts/package-release.py --binary "$release_binary" --target "$release_target"
