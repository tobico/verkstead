#!/usr/bin/env bash
#
# Run the `sessions` suite over and over under the load that made it flake, and
# say how many runs came back green.
#
# The failures this exists to catch were never reproducible by running the suite
# again: each loaded run failed a different test, and every one of them passed on
# its own. So the bar for the fix is a soak rather than a run — ten in a row,
# under load, all green.
#
# What "under load" means here is specific, and it is not simply a slow machine:
# three pinned runs of the suite passed even when one took twenty times its
# unloaded wall clock. What produced the failures was *bursty* contention — the
# suite competing with a compile that comes and goes. So this pins the suite to
# two cores and runs a workspace build round and round on the same two, which is
# the shape of a full `cargo test --workspace` on a two-core CI runner.
#
# About fifty minutes for the default ten runs, which is why nothing runs this
# for you: it is a thing to do by hand before putting a change up, not a check on
# every push.
#
#     scripts/soak-sessions.sh          # ten runs at the pace CI sets
#     scripts/soak-sessions.sh 3        # three of them
#     scripts/soak-sessions.sh 3 1      # and at the pace a developer's machine runs
#
# `SOAK_CORES` picks the cores, for a machine whose first two are busy with
# something else.
set -u

runs=${1:-10}

# The factor CI puts on the suite's own clock — see `VERKSTEAD_TEST_PACE` in
# `crates/server/tests/sessions.rs`. Soaking at anything else is soaking a
# configuration nobody runs.
export VERKSTEAD_TEST_PACE=${2:-2}

# A set VERKSTEAD_SERVER belongs to a session running inside Verkstead, and it
# changes what an unrelated test elsewhere in the workspace expects. Nothing here
# wants to inherit it.
unset VERKSTEAD_SERVER

cores=${SOAK_CORES:-0,1}

cd "$(dirname "$0")/.."

if ! command -v taskset >/dev/null; then
    echo "soak: taskset is not on this machine, so there is no way to pin anything" >&2
    exit 1
fi

# Built once and outside the runs, so that the first one is not the only one that
# pays for a compile.
echo "soak: building the suite"
if ! cargo build --workspace --tests >/dev/null; then
    echo "soak: the workspace does not build, so there is nothing to soak" >&2
    exit 1
fi

# The load: a workspace build going round and round on the same two cores.
#
# In a build directory of its own, and that is the point rather than tidiness.
# Cargo holds a lock on a build directory for the whole of a command, running a
# test binary included — so a build loop sharing this one would spend the soak
# waiting on the lock rather than competing for the cores, and every run would
# come back green having been under no load at all.
#
# `cargo clean -p` each time round so there is real compiling to do: the server
# crate and everything downstream of it, which on two cores is a burst of work
# rather than a cache read.
#
# Its own session, so that killing it at the end takes the `cargo` and `rustc`
# underneath it too. A build loop left running after the soak is a machine
# quietly at full tilt for as long as nobody notices.
setsid bash -c '
    while :; do
        CARGO_TARGET_DIR=target/soak-noise cargo clean -p verkstead-server >/dev/null 2>&1
        CARGO_TARGET_DIR=target/soak-noise taskset -c '"$cores"' \
            cargo build --workspace --tests >/dev/null 2>&1
    done
' &
noise=$!

# However this ends, including the interrupt somebody watching a fifty-minute
# soak is most likely to reach for.
trap 'kill -- -"$noise" 2>/dev/null; exit 130' INT TERM
trap 'kill -- -"$noise" 2>/dev/null' EXIT

green=0
logs=$(mktemp -d)

for run in $(seq 1 "$runs"); do
    started=$SECONDS

    if taskset -c "$cores" cargo test -p verkstead-server --test sessions \
        >"$logs/$run.log" 2>&1; then
        green=$((green + 1))
        echo "soak: run $run/$runs green in $((SECONDS - started))s"
        rm -f "$logs/$run.log"
    else
        echo "soak: run $run/$runs RED in $((SECONDS - started))s"
        # Which tests, not the whole log: a soak that printed every run's output
        # whole would bury the one line worth reading.
        sed -n '/^failures:$/,/^test result/p' "$logs/$run.log" | head -40
        echo "soak: the whole of it is in $logs/$run.log"
    fi
done

echo "soak: $green/$runs green at VERKSTEAD_TEST_PACE=$VERKSTEAD_TEST_PACE on cores $cores"

[ "$green" -eq "$runs" ]
