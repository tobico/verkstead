# 06. The runner emptied

## What to build

With both runners as components, take out what they replaced so no Activity
logic remains in the server outside the components:

- `runner.rs` loses the backlog drivers: `carry_on`, `work`, `backlog_again`,
  `plan_stage`, `next_step` and `Step`'s backlog arms, and the `skills.rs`
  prompt builders the components now write. It keeps what the host functions
  still call.
- `continuing.rs` and `stages.rs` lose the scheduling the stage runner now does
  (readiness, declarations, cut-from), keeping what *start a Conversation*,
  the read-back and the pinned roadmap card still need.
- `tasks.rs` keeps the reading behind the backlog card and pane, which is the
  viewer's and not the runner's.

The brief listed `units.rs` and `carrying.rs` here. They are store sweeps and
Transfer notices, unrelated to the runners, and stay as they are.

Update the documents that describe the runner: ADR-0030's *Activity* section
(the host functions this stage added and why reading the Repo takes no
capability), CONTEXT.md where **Activity** names the built-ins, and the module
docs that point at code that moved.

## Acceptance criteria

- [ ] No backlog or stage scheduling remains in the server outside the host
      functions and the components
- [ ] The Plugins page lists *Follow-up rounds*, *Backlog runner* and *Stage
      runner* under the Activities plugin's row
- [ ] The whole sessions suite passes unchanged, at CI's default threads
- [ ] Clippy and the Nix build (the built-in crates vendored) are clean
