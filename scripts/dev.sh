#!/usr/bin/env bash

pushd web
pnpm install && pnpm build
popd

cargo build

RUST_LOG=verkstead_server=debug cargo run serve --watched-path $HOME/src --data-dir /var/lib/verkstead
