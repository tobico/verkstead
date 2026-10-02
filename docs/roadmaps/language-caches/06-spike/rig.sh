#!/usr/bin/env bash
# The spike's rig: two session Sandboxes and a daemon Sandbox, each a bwrap
# command line of the shape Verkstead renders (the flags were dumped from
# `Sandbox::command` and `build_cache::compile_server` on 2026-10-01 and are
# reproduced here, less the binds of a Claude account, a handoff directory,
# skills and attachments, which nothing Gradle does reads).
#
#   rig.sh session <n> <command...>   run in session n's Sandbox
#   rig.sh daemon <command...>        run in the daemon's Sandbox
#   rig.sh fresh                      start over: a new Data Directory
#
# Layout, under $SPIKE (default $TMPDIR/gradle-spike):
#   data/cache/gradle         the shared GRADLE_USER_HOME, as `{cache}/gradle`
#   data/worktrees/s<n>       session n's Worktree, and its only bound checkout
#   data/outside/s<n>         a directory only session n is bound, standing for
#                             a Conversation's own extra bind
#
# Extra bwrap flags for one run go in $BWRAP_EXTRA (word-split).
set -euo pipefail

SPIKE=${SPIKE:-${TMPDIR:-/tmp}/gradle-spike}
DATA=$SPIKE/data
CACHE=$DATA/cache
WORKTREES=$DATA/worktrees
BWRAP=${BWRAP:-$(command -v bwrap)}

# What a Sandbox is built on, as `on_the_machine` renders it here.
system() {
  echo --die-with-parent --unshare-all --share-net --hostname verkstead
  for d in /nix /usr /bin /lib64 /etc /run/current-system; do
    [ -e "$d" ] && echo --ro-bind "$d" "$d"
  done
  echo --proc /proc --dev /dev --tmpfs /tmp
}

case "${1:-}" in
fresh)
  rm -rf "$DATA"
  mkdir -p "$CACHE/gradle" "$WORKTREES/s0" "$WORKTREES/s1" "$DATA/outside/s0" "$DATA/outside/s1"
  ;;
session)
  n=$2; shift 2
  home=$SPIKE/home-s$n
  # shellcheck disable=SC2046
  exec env -i "$BWRAP" $(system) \
    --dir "$home" \
    --bind "$WORKTREES/s$n" "$WORKTREES/s$n" \
    --bind "$DATA/outside/s$n" "$DATA/outside/s$n" \
    --bind "$CACHE" "$CACHE" \
    ${BWRAP_EXTRA:-} \
    --setenv HOME "$home" \
    --setenv PATH "$PATH" \
    --setenv GRADLE_USER_HOME "$CACHE/gradle" \
    --setenv GRADLE_OPTS "${SESSION_GRADLE_OPTS--Dorg.gradle.daemon=false}" \
    --setenv SPIKE_SESSION "s$n" \
    --chdir "$WORKTREES/s$n" \
    "$@"
  ;;
daemon)
  shift
  # shellcheck disable=SC2046
  exec env -i "$BWRAP" $(system) \
    --dir /verkstead/home \
    --bind "$WORKTREES" "$WORKTREES" \
    --bind "$CACHE" "$CACHE" \
    ${BWRAP_EXTRA:-} \
    --setenv HOME /verkstead/home \
    --setenv PATH "$PATH" \
    --setenv GRADLE_USER_HOME "$CACHE/gradle" \
    --setenv SPIKE_SESSION daemon \
    --chdir /verkstead/home \
    "$@"
  ;;
*)
  sed -n '2,20p' "$0"; exit 2
  ;;
esac
