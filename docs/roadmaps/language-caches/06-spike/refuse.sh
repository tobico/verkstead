#!/usr/bin/env bash
# Q2: with Verkstead's daemon up (started by start-daemon.sh), one session build
# per variation, each saying whether it reused a daemon and where it ran, and
# then the registry the daemon's Sandbox sees.
# Usage: refuse.sh <gradle-for-daemon> <label> <session-shell-prelude> <gradle-for-session> [args]
set -u
here=$(cd "$(dirname "$0")" && pwd)
dgradle=$1; label=$2; prelude=$3; gradle=$4; shift 4
echo "=== $label"
SESSION_GRADLE_OPTS= "$here/rig.sh" session 0 sh -c "echo s0 > /tmp/marker; $prelude
'$gradle' --console=plain $* where 2>&1 | grep -E 'Daemon|marker=|gradle=|java.home=|BUILD|FAIL|rror'" 
echo "--- registry, from the daemon's Sandbox"
"$here/rig.sh" daemon "$dgradle" --status 2>/dev/null | grep -E '^ +[0-9]+ '
"$here/rig.sh" daemon sh -c 'ls $GRADLE_USER_HOME/daemon'
