# 01. A JDK, Maven and Gradle in the dev shell and CI

## What to build

The tools the JVM proofs need, in the places the package stores' proofs already
run, so that every later task can prove its claim with a real build inside a
Sandbox.

- The dev shell gains a JDK, Maven and Gradle beside the tools stage 02 added,
  with a comment in the same style saying which descriptor they prove.
  nixpkgs carried Gradle 8.14.4, Maven 3.9.12 and OpenJDK 21 when this was
  planned.
- The **Package stores** CI job on Linux gains them too. Check what the runner
  image already carries: it ships JDKs, Maven and Gradle, but they have to
  resolve somewhere a Sandbox binds, which is the trap `found` in
  `tests/package_stores.rs` exists to catch. The job's tool check loop lists
  them.
- The proof suite's skip machinery covers the new tools. Where one is missing,
  a proof is skipped locally in a line naming it, and it fails where
  `VERKSTEAD_TEST_STORES` is set. Decide whether the JVM proofs live in
  `package_stores.rs` or in a sibling file, and keep to how that suite is laid
  out.
- A smallest proof that `java`, `mvn` and `gradle` each run inside a Sandbox,
  since `JAVA_HOME` and a JDK under `/nix/store` or `/usr/lib/jvm` are the
  likeliest things not to be reachable.

## Acceptance criteria

- [ ] `java -version`, `mvn -v` and `gradle --version` each succeed inside a Sandbox built the way the package-store proofs build one, both in the dev shell and in CI.
- [ ] Without the tools, the JVM proofs skip in a line naming the missing tool. With `VERKSTEAD_TEST_STORES` set, they fail instead.
- [ ] The CI job's time before and after is noted in the commit or PR.
