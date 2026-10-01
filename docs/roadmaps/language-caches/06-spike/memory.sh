#!/usr/bin/env bash
# Q4: one daemon per distinct (Gradle, JDK, jvmargs), each after one trivial
# build, measured while the session holding them is still open.
set -u
here=$(cd "$(dirname "$0")" && pwd)
G8=$1 G9=$2 J17=$3 J21=$4
run=""
for combo in "$G8 $J21 -" "$G9 $J21 -" "$G9 $J17 -" "$G9 $J21 -Xmx1g"; do
  set -- $combo
  props=""; [ "$3" != - ] && props="printf 'org.gradle.jvmargs=$3\n' > gradle.properties;"
  run+="rm -f gradle.properties; $props JAVA_HOME=$2 '$1' --console=plain where >/dev/null 2>&1;"
done
SESSION_GRADLE_OPTS='-Dorg.gradle.daemon.registry.base=/tmp/gradle-daemons' "$here/rig.sh" session 0 sh -c "$run rm -f gradle.properties; touch $SPIKE/data/cache/measure; while [ -e $SPIKE/data/cache/measure ]; do sleep 0.2; done" &
while [ ! -e "$SPIKE/data/cache/measure" ]; do sleep 0.2; done
ps -eo rss,args | grep -E '[G]radleDaemon' | awk '{v=$NF; jdk=($0 ~ /openjdk-17/)?"JDK 17":"JDK 21"; x="default jvmargs"; for(i=2;i<=NF;i++) if ($i=="-Xmx1g") x="-Xmx1g"; printf "Gradle %s, %s, %s: rss=%d MiB\n", v, jdk, x, $1/1024}'
rm "$SPIKE/data/cache/measure"; wait
