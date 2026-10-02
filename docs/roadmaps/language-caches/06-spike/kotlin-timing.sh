#!/usr/bin/env bash
# Q4 and speed: a real Kotlin compile, changed and rebuilt three times in one
# session, daemon off against the per-session registry; then the RSS of what
# the per-session build left running, measured while the session is open.
# Usage: kotlin-timing.sh <gradle>
set -u
here=$(cd "$(dirname "$0")" && pwd); g=$1
loop="cd k; for i in 1 2 3; do echo \"fun f\$i() = \$i\" > src/main/kotlin/F.kt; '$g' --console=plain compileKotlin 2>&1 | grep BUILD; done"
echo "=== daemon off"; "$here/rig.sh" session 0 sh -c "$loop"
echo "=== per-session registry"
SESSION_GRADLE_OPTS='-Dorg.gradle.daemon.registry.base=/tmp/gradle-daemons' "$here/rig.sh" session 0 sh -c "$loop; touch $SPIKE/data/cache/measure; while [ -e $SPIKE/data/cache/measure ]; do sleep 0.2; done" &
while [ ! -e "$SPIKE/data/cache/measure" ]; do sleep 0.2; done
ps -eo rss,args | grep -E '[G]radleDaemon|[K]otlinCompileDaemon' | awk '{n=($0 ~ /KotlinCompileDaemon/)?"kotlin daemon":"gradle daemon"; printf "%s rss=%d MiB\n", n, $1/1024}'
rm "$SPIKE/data/cache/measure"; wait
