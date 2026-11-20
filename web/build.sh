#!/bin/sh
# Build the browser version into web/dist (needs: rustup target add
# wasm32-unknown-unknown, and wasm-bindgen-cli of the same version as the
# wasm-bindgen crate in Cargo.lock). Serve web/dist with any static web server.
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown
rm -rf web/dist
wasm-bindgen --target web --no-typescript --out-dir web/dist \
  target/wasm32-unknown-unknown/release/mikrokit-emu.wasm
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz -o web/dist/mikrokit-emu_bg.wasm web/dist/mikrokit-emu_bg.wasm
fi
cp web/index.html web/dist/
cp assets/icon-64.png web/dist/
ls -l web/dist
