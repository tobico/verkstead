#!/usr/bin/env bash

pushd web
pnpm install && pnpm build
popd

cargo build

RUST_LOG=verkstead_server=debug cargo run --bin verkstead-desktop -- --data-dir /var/lib/verkstead
