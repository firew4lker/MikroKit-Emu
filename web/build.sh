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
# smaller and faster (optional: keeps the plain file if wasm-opt is missing or fails)
if command -v wasm-opt >/dev/null; then
  if wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
       --enable-reference-types --enable-multivalue --enable-mutable-globals \
       -o web/dist/opt.wasm web/dist/mikrokit-emu_bg.wasm; then
    mv web/dist/opt.wasm web/dist/mikrokit-emu_bg.wasm
  else
    echo "wasm-opt failed; keeping the unoptimised file" >&2
    rm -f web/dist/opt.wasm
  fi
fi
cp web/index.html web/dist/
cp assets/icon-64.png web/dist/
ls -l web/dist
