#!/usr/bin/env bash

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The viewer, which the server serves.
(cd "$root/web" && pnpm install && pnpm build)

# The headless CLI, which the app starts beside itself as `serve --desktop`.
# The app looks for it at target/debug/verkstead.
(cd "$root" && cargo build)

# The app has no flags of its own: the sidecar inherits this shell, so the data
# directory and the log filter are said here rather than on a command line.
export VERKSTEAD_DATA_DIR=/var/lib/verkstead
export RUST_LOG=verkstead_server=debug

cd "$root/desktop"
pnpm install
pnpm start
