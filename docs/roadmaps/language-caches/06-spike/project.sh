#!/bin/sh
# Lay out the spike's Gradle build in a Worktree: one task, `where`, that says
# where the build really ran, and optionally waits on a file before it ends.
set -e
dir=$1
mkdir -p "$dir"
printf "rootProject.name = 'app'\n" > "$dir/settings.gradle"
cat > "$dir/build.gradle" <<'GRADLE'
tasks.register('where') {
  doLast {
    def p = ProcessHandle.current()
    println "pid=" + p.pid()
    println "cmd=" + p.info().command().orElse('?')
    println "env.HOME=" + System.getenv('HOME')
    println "env.SPIKE_SESSION=" + System.getenv('SPIKE_SESSION')
    println "user.home=" + System.getProperty('user.home')
    println "user.dir=" + System.getProperty('user.dir')
    println "java.io.tmpdir=" + System.getProperty('java.io.tmpdir')
    println "java.home=" + System.getProperty('java.home')
    println "gradle=" + gradle.gradleVersion
    def marker = new File('/tmp/marker')
    println "marker=" + (marker.exists() ? marker.text.trim() : 'none')
    println "projectDir=" + project.projectDir
    def w = new File(System.getenv('SPIKE_WORKTREES') ?: '/nonexistent')
    println "worktrees.visible=" + (w.list()?.sort()?.join(',') ?: 'none')
    def outside = System.getenv('SPIKE_OUTSIDE')
    if (outside) {
      def f = new File(outside, 'probe')
      try { f.text = 'written'; println "outside.write=ok " + f } catch (e) { println "outside.write=FAILED " + e }
    }
    def homeFile = new File(System.getenv('HOME') ?: '/nonexistent', 'probe-home')
    try { homeFile.text = 'x'; println "home.write=ok" } catch (e) { println "home.write=FAILED " + e }
    def wait = project.findProperty('waitFor')
    if (wait) {
      new File(wait + '.started').text = 'x'
      def deadline = System.currentTimeMillis() + 120000
      while (!new File(wait).exists() && System.currentTimeMillis() < deadline) sleep 100
      println "waited=" + new File(wait).exists()
    }
  }
}
GRADLE
