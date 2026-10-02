#!/usr/bin/env bash
# The per-session registry: GRADLE_OPTS points the daemon registry at the
# session's own /tmp instead of turning the daemon off. No daemon of
# Verkstead's is running. Usage: registry.sh <gradle>
set -u
here=$(cd "$(dirname "$0")" && pwd)
g=$1; meet=$SPIKE/data/cache/meet; rm -rf "$meet"; mkdir -p "$meet"
per='-Dorg.gradle.daemon.registry.base=/tmp/gradle-daemons'
show='grep -E "Daemon|marker=|pid=|BUILD|FAIL|setcwd"'
echo "=== daemon off (today), three builds in one session"
"$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; for i in 1 2 3; do '$g' --console=plain where 2>&1 | grep BUILD; done"
echo "=== per-session registry, three builds in one session"
SESSION_GRADLE_OPTS="$per" "$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; for i in 1 2 3; do '$g' --console=plain where 2>&1 | grep -E 'Daemon|BUILD'; done; ls /tmp/gradle-daemons/*"
echo "=== shared home's registry after that"; ls "$SPIKE/data/cache/gradle/daemon/" 2>&1
echo "=== session 0 holds an idle daemon; session 1 asks --daemon"
SESSION_GRADLE_OPTS="$per" "$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; '$g' --console=plain --daemon where 2>&1 | $show | sed 's/^/[s0] /'; touch $meet/s0-up; while [ ! -e $meet/close ]; do sleep 0.1; done" &
while [ ! -e "$meet/s0-up" ]; do sleep 0.1; done
SESSION_GRADLE_OPTS="$per" "$here/rig.sh" session 1 sh -c "echo s1 > /tmp/marker; '$g' --console=plain --daemon where 2>&1 | $show | sed 's/^/[s1] /'"
echo "=== and a Repo's gradle.properties pointing it back at the shared home"
for how in "org.gradle.daemon.registry.base" "systemProp.org.gradle.daemon.registry.base"; do
  SESSION_GRADLE_OPTS="$per" "$here/rig.sh" session 1 sh -c "printf '$how=%s\n' \"\$GRADLE_USER_HOME/daemon\" > gradle.properties; echo s1 > /tmp/marker; '$g' --console=plain where 2>&1 | grep -E 'Daemon|BUILD|FAIL'; ls /tmp/gradle-daemons 2>&1 | sed 's/^/per-session: /'" | sed "s/^/[$how] /"
done
rm -f "$SPIKE/data/worktrees/s1/gradle.properties"
touch "$meet/close"; wait
