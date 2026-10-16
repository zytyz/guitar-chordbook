#!/bin/sh
# Builds the website into web/www: compiles the Rust core to WebAssembly and
# generates its JavaScript bindings in web/www/pkg.
#
# Needs the wasm32 target and a wasm-bindgen CLI of the same version as the
# wasm-bindgen crate:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <version printed below> --locked
set -eu
cd "$(dirname "$0")/.."

cargo build -p chordbook-web --target wasm32-unknown-unknown --profile web

wanted=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
have=$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2 || true)
if [ "$have" != "$wanted" ]; then
    echo "wasm-bindgen CLI ${have:-not found}, but the crate is $wanted." >&2
    echo "Install it with: cargo install wasm-bindgen-cli --version $wanted --locked" >&2
    exit 1
fi

rm -rf web/www/pkg
wasm-bindgen --target web --no-typescript --out-dir web/www/pkg \
    target/wasm32-unknown-unknown/web/chordbook_web.wasm
echo "Built web/www. Serve it with: python3 -m http.server -d web/www 8000"
