#!/bin/sh
set -x

cargo build -p profiles_integrity --release --target wasm32-unknown-unknown
cargo build -p profiles --release --target wasm32-unknown-unknown
cargo build -p holomess_integrity --release --target wasm32-unknown-unknown
cargo build -p holomess --release --target wasm32-unknown-unknown

hc dna pack happ/workdir
hc app pack happ/workdir