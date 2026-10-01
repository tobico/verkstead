#!/bin/sh
# Inside the daemon's Sandbox: have a client fork an ordinary daemon with an
# empty build in Verkstead's own HOME, then stay in the foreground so the
# Sandbox, and the daemon in it, last exactly as long as this does.
# $1 is the gradle to start; the rest goes on its command line.
set -e
gradle=$1; shift
mkdir -p "$HOME/empty"
cd "$HOME/empty"
: > settings.gradle
"$gradle" --daemon --console=plain -q "$@" help
echo "daemon started"
exec sleep 2147483647
