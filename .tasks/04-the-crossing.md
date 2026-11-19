# 04. The crossing

## What to build

Codex runs as its plugin, and the in-process implementation is gone.

- **Built-in plugins stand up their harness at start.** Today only installed
  plugins reach `harness::plugged` (`plugins/installed.rs`'s `harness`); a
  built-in with an `agent` provision is loaded through the same loader, its mark
  read out of the embedded files, before any user plugin. Installing a plugin
  under a built-in harness's uuid stays refused; the embedded one is exempt.
- **The harness tables stop pairing by place.** `harness::BUILT_IN`,
  `BINARIES` and `every()` zip the in-process harnesses with `store::BUILT_IN`
  by position; with Codex gone they must find Codex through the plugin (`of`
  already falls through to the plugged map, keyed by `AgentType::read(uuid)`,
  which reads Codex's uuid as the word `codex`). The pickers, onboarding,
  `pairing_defaults` and the sandbox's list of followed binaries still see Codex
  in its usual place.
- **Listed under Built-in** on the Plugins page with a tick and nothing else (no
  Remove, no Update); the tick is the one switch for Codex and disabling follows
  stage 03's rule (Profiles stay, offered on no picker, a remembered Pairing not
  applied, running sessions finish).
- **Delete `harness/codex.rs`** and every Codex-only path in the server it
  leaves unused (the `Search::Rollout` search, `codex_search`/`codex_carried`,
  `names_itself` once task 01's generic path records the id, the Codex constants
  in `sandbox/root.rs` and `sandbox.rs` nothing else reads).
- **Stored Transcripts** (Set 1836, Q3a): new Codex Transcripts are stored as
  the entry lines the component answers; the render crate keeps drawing raw
  rollout lines already stored, or carried from a peer on an older release, as
  read-only history.
- **The suite is the proof and is not edited to pass** — a test failing against
  the component is a difference in the component — with one exception (Q3b):
  assertions that a Transcript's stored lines are "kept exactly as codex wrote
  them" are rewritten to assert the drawn kinds, each named in its commit.

## Acceptance criteria

- [ ] A fresh data directory lists Codex under Built-in with a tick and nothing else; unticking it takes Codex Profiles off every picker and a Conversation remembering one starts with that role empty
- [ ] Every Codex test in the sessions, transcript, nudging, sandbox, transfer, conversations, profiles and mirroring suites passes against the component — on the Linux, macOS and Windows CI legs — with only the Q3b assertions changed
- [ ] A Codex Profile's memory syncs to a device it is not bound on, and a Codex Conversation transferred mid-session resumes by the id the rollout chose
- [ ] A Conversation from before the crossing still draws its stored Codex Transcript
