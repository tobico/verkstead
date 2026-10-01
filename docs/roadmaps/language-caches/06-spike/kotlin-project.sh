#!/bin/sh
# Q5: a real Kotlin/JVM build (Kotlin Gradle plugin 2.2.20) in a Worktree,
# with a task that reports where the Kotlin daemon's run files are and which
# Kotlin daemons a build can see.
set -e
dir=$1
mkdir -p "$dir/src/main/kotlin"
cat > "$dir/settings.gradle.kts" <<'K'
pluginManagement { repositories { gradlePluginPortal(); mavenCentral() } }
rootProject.name = "kapp"
K
cat > "$dir/build.gradle.kts" <<'K'
import java.io.File
plugins { kotlin("jvm") version "2.2.20" }
repositories { mavenCentral() }
kotlin { jvmToolchain(21) }
tasks.register("kotlinRuns") {
  doLast {
    val runs = File(System.getProperty("user.home"), ".kotlin/daemon")
    println("kotlin.runs=" + runs + " " + (runs.list()?.sorted()?.joinToString(",") ?: "missing"))
    val mine = ProcessHandle.current()
    ProcessHandle.allProcesses().filter { it.info().commandLine().orElse("").contains("KotlinCompileDaemon") }
      .forEach { println("kotlin.daemon pid=" + it.pid() + " parent=" + it.parent().map { p -> p.pid() }.orElse(-1) + " runFiles=" + Regex("--daemon-runFilesPath[= ](\\S+)").find(it.info().commandLine().orElse(""))?.groupValues?.get(1)) }
    println("gradle.pid=" + mine.pid())
  }
}
K
printf 'fun main() { println("hi") }\n' > "$dir/src/main/kotlin/Main.kt"
