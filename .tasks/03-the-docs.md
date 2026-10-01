# 03. The docs

## What to build

Write down what this stage found and decided, whichever way it went, so that
nobody has to read the spike's findings file to know it. The findings and the
decision are in `docs/roadmaps/language-caches/06-spike-findings.md`; tasks
01 and 02 wrote them, and any build tasks between them and this one carried
them out.

- **ADR-0021** — amend *Gradle's daemon is off in a session*, and *What is
  accepted*, with what was measured, on which Gradles, and what was decided.
  Where a capability was added, amend *A descriptor is data* as well, since it
  currently says `sccache` is the one capability there is.
- **CONTEXT.md** — the **Descriptor** and **Build Cache** entries, which say
  Gradle's daemon is off in a session. If a daemon of Verkstead's own was
  built, it gets a term of its own beside the **Compile Server**.
- **The `jvm` entry's comments in `crates/server/languages.yaml`** — their
  account of the daemon, of the `--daemon` hole and of the per-session
  registry.
- **The installer's documentation** (`docs/adoption.md` and wherever else
  Gradle is mentioned for installers) — what a Gradle build in a session gets.

The stage-04 hole — an explicit `--daemon` on the command line putting one
session's build in another's Sandbox — reads afterwards either as closed (and
how) or as still accepted (and why).

## Acceptance criteria

- [ ] ADR-0021 says what the spike measured, on which Gradle versions, and what was decided and why
- [ ] CONTEXT.md, the `jvm` descriptor's comments and the installer's documentation agree with the ADR about whether a session's Gradle build has a daemon
- [ ] The `--daemon` hole is described as closed or as still accepted, consistently everywhere it is mentioned
