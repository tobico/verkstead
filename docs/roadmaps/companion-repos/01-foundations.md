# 01. Foundations

## Goal

A Conversation configured with companion repos while its brief drafts has each
of them checked out and bound into every session's sandbox by mode, named in
the session prompt, refused cleanly at grill start when git cannot deliver one,
and torn down at close. Demonstrable end to end: add a read-only and a
read-write companion on the setup card, start grilling, and the session can
read one, commit to the other on a fresh branch, and was told about both —
with the Brief's details pane saying, ever after, what the Conversation was
configured with.

## Decisions in force

- **Companions are registered Repos, added per Conversation.** The registry is
  the trust boundary and the sandbox stays a composed list (design doc,
  *Sandbox configuration* and *Companion repos*). The conversation's own repo
  and duplicates are refused.
- **Always a Verkstead worktree; the human's checkout never enters a sandbox.**
  The sandbox principle already excludes "even the checkout the Worktree was
  made from"; a shared checkout would also carry uncommitted state, and git
  refuses a second checkout of a checked-out branch anyway. Read-only checks
  out **detached** at the selected branch's resolved commit — a new worktree
  shape (`git worktree add --detach`), today's primitives only ever cut a new
  branch (`add -b`) or rebuild an existing one. Read-write **always cuts a new
  branch** from the selected base, as the main repo does; "work directly on an
  existing branch" was considered and rejected (commits would land on e.g. the
  companion's `main`, and the checkout collision returns).
- **The read-write branch name mirrors the main branch name until typed.**
  Stored empty means *mirroring* — rename the main branch and it follows; a
  typed name stands on its own. Settled in the grilling's own words (Q2a).
- **Config freezes at grill start** with the branch and base; while drafting it
  is freely added, edited and removed. Steer-time changes are stage 04's.
- **Fetch-then-resolve per companion at grill start**, exactly the main repo's
  order; every failure — fetch failed, base unresolvable, branch taken —
  refuses grill start naming the companion. Nothing new gates the grill button:
  companion config is always complete, so refusal at start is the whole story.
- **Sandbox binds by mode.** Worktree and the repo's git common dir both bound,
  read-only or read-write with the companion's mode; the companion repo's own
  per-repo sandbox binds (build caches) are composed in too, because its builds
  need them. Read-only extra binds are new — the extras loop today only knows
  `--bind`.
- **The prompt carries one neutral listing and no instructions** — a
  `# Companion repositories` section naming each companion, its worktree path,
  its branch and its write status, on every session prompt of the Conversation,
  the grilling one included. The agent decides from the Brief what to use.
- **UI: extend the one `Menu` component to support nesting** (the human's
  explicit call over flat rows) — the ⋯ beside the branch row opens "Add
  companion repo" with the repo picked in a nested level. Each companion draws
  as a row under the branch row: base picker (default-branch rule first), mode
  switch, mirroring branch-name field for read-write, × while drafting.
- **Close removes companion worktrees and keeps their branches**, like the main
  one.
- **The Brief's details pane summarises the whole of a Conversation's
  configuration.** The setup rows go when the card freezes, so without this a
  read-only companion would leave no trace anywhere for the rest of the
  Conversation's life — read-write ones surface later through their commits and
  pull requests, and a read-only one never does. The summary is not only the
  companions: the worktree directories and the picked Pairings are shown
  nowhere today either, and they belong in the same place.

## Proposed tasks (provisional)

1. **Store and API for companion config** — a companions relation per
   Conversation (repo, mode, base ref, branch name where empty means
   mirroring) and per-companion worktree records; add/edit/remove endpoints
   refused once no longer drafting.
   - Own repo and duplicate refused with named refusals.
   - The `worktrees` table's one-row-per-conversation shape is left to the
     main repo; companions get room of their own.
2. **Setup UI** — `Menu` nesting support, the ⋯ trigger beside the branch row,
   and the companion rows with autosave matching the branch field's.
   - Mode flip reveals the branch field prefilled by mirroring.
   - Rows vanish with the rest of the setup when the card freezes.
3. **Worktree primitives and grill start** — a detached `add`, companion
   fetch/resolve/create in the grill-start path, refusals naming the
   companion, teardown at close.
   - A refused companion leaves nothing behind — no branch, no directory.
4. **Sandbox binds** — worktree and git common dir per companion by mode,
   read-only extras support, per-repo `SandboxConfig` binds composed for
   companions.
   - A read-only companion's git dir refuses a push from inside.
5. **Prompt section** — the listing threaded through every prompt builder,
   grilling included.
6. **Configuration summary on the Brief's details pane** — everything the
   Conversation was set up with, read where the frozen Brief is read: the repo,
   the branch and the base it came off, every worktree directory, the grilling
   and implementation Pairings, and each companion with its mode, its branch
   and its directory.
   - A Conversation with no companions still gets the summary — the worktree
     directory and the Pairings are as unfindable today as a companion is.
   - Read-only throughout: the pane reports the configuration, and the setup
     card is still the only place it is changed.

## Re-verify at start

- The sandbox bind list and its extras loop still live in
  `crates/server/src/sandbox.rs` (`Sandbox`, `SandboxConfig::binds_for`), and
  the one composition call in `crates/server/src/sessions.rs`.
- Worktree primitives still in `crates/server/src/worktrees.rs` with `add`
  always `-b` and `worktree_path` naming `<repo>-<branch>[-<id>]`.
- The grill-start order (fetch → resolve → branch check → add) in
  `crates/server/src/conversations.rs::start_grilling`.
- The setup card layout in `web/src/workbench/Setup.tsx` and the single
  dropdown in `web/src/Menu.tsx` (still no nesting).
- The Brief's details pane — still the plain `Document` the three documents
  share, in `web/src/workbench/Workbench.tsx` — and what the Conversation view
  already carries to it (`ConversationView`, `web/src/api`).
- Prompt builders all in `crates/server/src/skills.rs`; grilling's built
  separately from `on_the_documents`.
