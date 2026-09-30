# 02. Maven

## What to build

A built-in `jvm` descriptor, added as YAML plus proofs with no change to the
loader. Its first tool is Maven: a session's Maven resolves out of one local
repository under the Build Cache, with cross-process file locking switched on,
so two sessions writing at once cannot damage it.

What was observed at planning time with Maven 3.9.12, to be re-checked against
the current docs and then proven:

- Maven has no environment variable for its local repository. The system
  property `maven.repo.local`, passed in `MAVEN_OPTS`, is read.
- Resolver's named locks are switched on by
  `-Daether.syncContext.named.factory=file-lock` and
  `-Daether.syncContext.named.nameMapper=file-gav`, also through `MAVEN_OPTS`.
  The `-X` log says "Creating adapter using nameMapper 'file-gav' and factory
  'file-lock'". Check the docs for the version this arrived in and whether
  Maven 4 still needs it.
- A placeholder inside a flag is already supported by the grammar ("a flag with
  a path in it comes out right"). **But `mvn` splits `MAVEN_OPTS` on
  whitespace**, so a Build Cache path containing a space, such as a Windows
  user name under `%LOCALAPPDATA%`, would break it. Establish what happens and
  handle it: quoting, `MAVEN_ARGS`, or `.mvn/`, whichever the scripts actually
  honour on each platform.
- `MAVEN_USER_HOME` moves where `mvnw` keeps the Maven distributions it
  downloads, which otherwise land in each session's fresh `HOME`. Share them
  too, if the wrapper's docs confirm it.
- Precedence: a Repo's `.mvn/maven.config` gives user properties, which should
  beat `MAVEN_OPTS`'s system properties. A Repo pinning its own local
  repository therefore still wins, as a Repo's own config does for the
  package stores. Prove which way it goes.

Keep `detect` to `pom.xml` for now. Task 03 adds Gradle's manifests.

The proof follows stage 02's shape. Two Sandboxes build at once against one
Build Cache. A third builds with `-o` and must succeed. A control build on an
empty cache with `-o` must fail. The registry is this machine's own, served
over the loopback and shut before the offline builds, so nothing reaches the
internet.

## Acceptance criteria

- [ ] Two Maven builds at once in two Sandboxes both succeed. A third, with `-o` and the registry shut, succeeds out of the shared repository. The control on an empty Build Cache fails.
- [ ] A Repo's own settings file (`-s` in `.mvn/maven.config`) is still read, and a Repo that pins `maven.repo.local` itself keeps it, or the docs say otherwise.
- [ ] A Build Cache path with a space in it still gives Maven a working local repository, or the descriptor avoids the split.
- [ ] The variables a session is given are asserted on all three platforms, where the other descriptors' are (`tests/sandbox.rs`, `tests/sandbox_macos.rs`, `tests/sessions_windows.rs`).
