# 01. Rename to Verkstead

## What to build

The project answers to Verkstead everywhere: the workspace crates, the binary
and its verbs, the environment variables, the SPA's branding, the NixOS
module and its service and state directory, the CI and release workflows, and
the documentation. Behaviour is unchanged — this is the whole name moving at
once, before anything is written against a name that is about to shift under
it.

The agent-facing surface renames with everything else: `verkstead ask`,
`verkstead guide`, `verkstead serve`, and `VERKSTEAD_SERVER`,
`VERKSTEAD_DATABASE`, `VERKSTEAD_LISTEN`, `VERKSTEAD_NO_UPDATE_CHECK`. The
real askance stays installed on this host for daily work, so one name
answering to two different binaries depending on which sandbox you are in is
the thing being avoided.

Two pieces of prose need judgement rather than substitution:

- **The Guide** (`crates/cli/guide/`) is written for agents and names the
  command throughout. It keeps saying what it says about asking well; only
  the command it names changes.
- **CONTEXT.md** is askance's glossary. Verkstead's starts here, keeping the
  question-set vocabulary that still holds and adding the terms the design
  introduces: conversation, brief, timeline, event, watched paths, agent
  profile, grilling and implementation class, blocking and deferred ask.

The update check points at this repository's releases, which has none yet —
so the Update Notice must degrade quietly rather than break the viewer.
`docs/agents/git-workflow.md`'s notes still describe askance's origin and its
public/private history; correct them for this repo.

## Acceptance criteria

- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` all pass, and the generated `web/src/api/types.ts` is unchanged by the run
- [ ] `nix flake check` passes, including the NixOS VM test under the new service, unit and state-directory names
- [ ] `verkstead serve` serves the existing viewer and `verkstead ask` puts a set to it end to end, with `VERKSTEAD_SERVER` selecting the server
- [ ] `verkstead` with no arguments prints the Guide, naming `verkstead` throughout
- [ ] No occurrence of "askance" survives outside deliberate historical references — the design doc's account of the clone, the roadmap briefs, ADR-0004, and the git-workflow note about the process this repo inherited
- [ ] CONTEXT.md is headed Verkstead and carries the design's new terms alongside the question-set vocabulary
- [ ] The viewer loads with no release to compare against, and shows no Update Notice
