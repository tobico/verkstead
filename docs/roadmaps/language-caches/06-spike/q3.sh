#!/usr/bin/env bash
# Q3: with Verkstead's daemon up (start-daemon.sh), what a session's build
# reaches. Session 0's Worktree is a real git worktree of a Repo outside the
# Data Directory, whose .git the session is bound and the daemon is not, as
# Verkstead binds them. Usage: q3.sh <gradle>
set -u
here=$(cd "$(dirname "$0")" && pwd); g=$1
repo=$SPIKE/repo
if [ ! -d "$repo/.git" ]; then
  git init -q "$repo"
  git -C "$repo" commit -q --allow-empty -m init
  rm -rf "$SPIKE/data/worktrees/s0"
  git -C "$repo" worktree add -q "$SPIKE/data/worktrees/s0"
fi
"$here/project.sh" "$SPIKE/data/worktrees/s0"
cat >> "$SPIKE/data/worktrees/s0/build.gradle" <<'GRADLE'
tasks.register('reach') {
  doLast {
    def git = ['git', 'rev-parse', '--short', 'HEAD'].execute(null, projectDir)
    git.waitFor()
    println "git=" + (git.exitValue() == 0 ? git.text.trim() : 'FAILED ' + git.err.text.trim())
    def sh = ['sh', '-c', 'echo "child.HOME=$HOME child.marker=$(cat /tmp/marker 2>/dev/null || echo none)"'].execute()
    sh.waitFor(); println sh.text.trim()
    new File('/tmp/left-by-' + System.getenv('SPIKE_SESSION')).text = 'x'
    println "tmp=" + new File('/tmp').list().findAll { it.startsWith('left-by-') }.sort().join(',')
  }
}
GRADLE
SESSION_GRADLE_OPTS= BWRAP_EXTRA="--bind $repo/.git $repo/.git" "$here/rig.sh" session 0 sh -c "cd $SPIKE/data/worktrees/s0 2>&1; echo s0 > /tmp/marker; echo shell.git=\$(git rev-parse --short HEAD); '$g' --console=plain reach 2>&1 | grep -E 'Daemon|git=|child|tmp=|BUILD|FAIL'" | sed 's/^/[s0] /'
cp "$SPIKE/data/worktrees/s0/build.gradle" "$SPIKE/data/worktrees/s1/build.gradle"
SESSION_GRADLE_OPTS= "$here/rig.sh" session 1 sh -c "echo s1 > /tmp/marker; '$g' --console=plain reach 2>&1 | grep -E 'Daemon|child|tmp=|BUILD|FAIL'" | sed 's/^/[s1] /'
"$here/project.sh" "$SPIKE/data/worktrees/s1"
