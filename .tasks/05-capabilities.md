# 05. Capabilities

## What to build

Each of the five capabilities is read off the Forge's declaration rather than
off the `lacking` seam `forge.rs` carries today, which goes. An absent
capability is settled, not failed:

- **`checks` / `review`**: the wrap-up watcher over it is settled at once, as
  the seam does now, and the Timeline or card says that Forge has none.
- **`publish`**: no Publish on the Share pane, and the pane says so.
- **`create`**: **Create repo** stops at the local repository with a line
  saying no remote was made.
- **`stacks`**: a roadmap's chain is left to the rebases the join already
  does. Each stage's pull request is opened against the branch below. The
  stack walk at take-up is skipped. Fix Merge Issues sends the lone-pull-request
  resolution rather than the `gh stack` sync prompt. The `gh stack` installed
  check is not asked.

Prove it with a test Forge declaring no capabilities, an in-process trait
implementation the suite registers as a plugin. GitHub declares all five and
reads as before.

## Acceptance criteria

- [ ] A Conversation on the test Forge that declares none wraps up on
      mergeability alone and says so on the Timeline.
- [ ] A roadmap on it opens each stage's pull request against the branch below
      with no sync attempted and no `gh stack` asked about.
- [ ] Without `publish` the Share pane has no Publish and says why. Without
      `create`, Create repo makes the local repository and says no remote was
      made.
