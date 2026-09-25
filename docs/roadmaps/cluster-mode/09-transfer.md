# 09. Transfer

## Goal

A Conversation moves from A to B at a press. *Transfer to…* on the actions
menu opens a dialog with the device select and what the preflight found
missing; on Go the session's turn ends as Stop ends it, memory and login
write back home, and the branch goes over as a bundle with a binary patch of
the working changes — companions the same — while the record's slice is
copied under new ids with its rank. B cuts the worktree, applies the patch,
and Verkstead's Resume starts what ought to be running. A keeps its copy
read-only under *transferred to B*; the merged list draws the live copy only;
A's old URL redirects; a transfer back replaces A's copy wholesale under its
local id. Demonstrable end to end across two OSes, and back.

## Decisions in force

- **The bundle and the patch over the link**
  ([ADR-0020](../../adr/0020-cluster-mode.md), *Transfer*): packed against
  what B says it has; tracked changes as a binary patch plus every untracked
  unignored file; ignored files stay behind. A WIP commit through origin was
  rejected.
- **Repos matched by origin then name** (stage 08); no match refuses by
  name, pointing at Open repo on B.
- **The slice copied, the source kept read-only**, every copy carrying the
  **birth key** — drafting device and id there — so the list shows one, URLs
  redirect, and a transfer back replaces the stale copy under its existing
  local id. A forwarding stub alone was the recommendation; the human chose
  to keep the copy.
- **The move runs when the turn ends**, as Stop does, the Timeline saying
  *Transferring to B* meanwhile. Force stop was rejected.
- **Verkstead's Resume on arrival** in this stage; the harness's own resume
  is stage 10.
- **Allowed from every state but Draft and Closed**; a Draft moves by the
  composer (stage 07).
- **Preflight before anything moves**: Repo matched, companions matched,
  harness present for each pairing, device reachable; refused by name.

## Proposed tasks (provisional)

1. **Birth key and copies** — the key on every Conversation, given at
   creation; a *transferred* state that draws read-only and redirects; the
   merged list drawing the live copy.
   - A copy's URL lands on the live one; the list holds one row.
2. **The preflight** — a reading of what B lacks, drawn in the dialog and
   refusing the press.
   - A missing Repo names it and points at Open repo on B.
3. **The git leg** — B's refs asked, the bundle packed and sent, the patch
   and untracked files sent, B fetching, cutting and applying; companions the
   same; the branch followed if renamed.
   - Uncommitted changes on A are uncommitted changes on B, binary included.
4. **The record leg** — the slice serialised, ids remapped on B, attachments
   and captures included, rank kept; A's row marked once B confirms.
   - The Timeline on B reads as it did on A, Sets answerable.
5. **The press and the move** — the menu row, the dialog, the turn-end wait,
   the Timeline Notice, Resume on B.
   - A running grilling on A continues as a re-primed grilling on B.
6. **Transfer back** — B's live record replacing A's copy under A's id.
   - A's old links work and show everything B added.

## Re-verify at start

- Stage 08 landed: Repo matching and mirror Profiles.
- Worktrees are still cut in `crates/server/src/worktrees.rs`, uncommitted
  changes read in `crates/server/src/diffs.rs` (`uncommitted`, `writable`),
  companions in `crates/store/src/companions.rs`, renames followed in
  `crates/server/src/renames.rs`.
- Resume is still `crates/server/src/resume.rs` recomputing from lifecycle
  and branch; the schema tour in `crates/store/src/lib.rs` lists the tables a
  slice has to carry.
- The actions menu is still `web/src/workbench/Actions.tsx`; the close's
  confirm is the dialog pattern.
- Line endings across Windows and Unix on a binary patch — verify `git
  apply` behaves with the repository's `core.autocrlf` on both sides.
