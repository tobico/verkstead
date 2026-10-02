#!/usr/bin/env bash
# Q2, busy: session 0's build holds Verkstead's daemon busy; session 1 builds
# meanwhile and stays open after; then session 0 builds twice more. Prints
# where each build ran.
# Usage: busy.sh <gradle>
set -u
here=$(cd "$(dirname "$0")" && pwd)
g=$1; meet=$SPIKE/data/cache/meet; rm -rf "$meet"; mkdir -p "$meet"
show='grep -E "Daemon|marker=|pid=|BUILD|FAIL|setcwd|working directory"'
SESSION_GRADLE_OPTS= "$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; '$g' --console=plain where -PwaitFor=$meet/release 2>&1 | $show | sed 's/^/[s0 first] /'" &
while [ ! -e "$meet/release.started" ]; do sleep 0.1; done
SESSION_GRADLE_OPTS= "$here/rig.sh" session 1 sh -c "echo s1 > /tmp/marker; '$g' --console=plain where 2>&1 | $show | sed 's/^/[s1 while busy] /'; touch $meet/s1-built; while [ ! -e $meet/s1-close ]; do sleep 0.1; done" &
while [ ! -e "$meet/s1-built" ]; do sleep 0.1; done
touch "$meet/release"; wait %1
for i in 1 2; do
  SESSION_GRADLE_OPTS= "$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; '$g' --console=plain where 2>&1 | $show | sed 's/^/[s0 again $i] /'"
done
"$here/rig.sh" daemon "$g" --status 2>/dev/null | grep -E '^ +[0-9]+ '
touch "$meet/s1-close"; wait
