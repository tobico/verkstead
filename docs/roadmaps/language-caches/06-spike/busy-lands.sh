#!/usr/bin/env bash
# Q2, the hazard back: session 1 spawns a daemon of its own (Verkstead's is
# busy) and stays open; then, with Verkstead's busy again, session 0 builds.
# Usage: busy-lands.sh <gradle>
set -u
here=$(cd "$(dirname "$0")" && pwd)
g=$1; meet=$SPIKE/data/cache/meet; rm -rf "$meet"; mkdir -p "$meet"
show='grep -E "Daemon|marker=|pid=|BUILD|FAIL|setcwd|working directory"'
hold() { # session $1 holds Verkstead's daemon busy until $meet/$2 exists
  SESSION_GRADLE_OPTS= "$here/rig.sh" session "$1" sh -c "echo s$1 > /tmp/marker; '$g' --console=plain where -PwaitFor=$meet/$2 2>&1 | $show | sed 's/^/[s$1 holding] /'" &
  while [ ! -e "$meet/$2.started" ]; do sleep 0.1; done
}
hold 0 release1
SESSION_GRADLE_OPTS= "$here/rig.sh" session 1 sh -c "echo s1 > /tmp/marker; '$g' --console=plain where 2>&1 | $show | sed 's/^/[s1 while busy] /'; touch $meet/s1-built; while [ ! -e $meet/s1-close ]; do sleep 0.1; done" &
while [ ! -e "$meet/s1-built" ]; do sleep 0.1; done
touch "$meet/release1"; sleep 1
hold 0 release2
SESSION_GRADLE_OPTS= "$here/rig.sh" session 0 sh -c "echo s0-second > /tmp/marker; '$g' --console=plain where 2>&1 | $show | sed 's/^/[s0 second build] /'"
touch "$meet/release2" "$meet/s1-close"; wait
