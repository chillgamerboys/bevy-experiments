#!/bin/bash

if [ -z "$1" ]; then
    echo "Usage: $0 <binary-name>"
    exit 1
fi

GAME=$1

cargo build --target wasm32-unknown-unknown --bin $GAME

wasm-bindgen --out-dir ./wasm --target web target/wasm32-unknown-unknown/debug/${GAME}.wasm --out-name game

if [ -d "assets" ]; then
    cp -r assets wasm/
fi

if [[ "$*" == *"--run"* ]]; then
    python3 -m http.server 2424 --directory wasm
fi
