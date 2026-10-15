# 02. Clonable repos

## Goal

On the compose page, the Repo dropdown for whichever device the select names
lists that device's own Repos, then, behind a rule, a second group: the
repositories other members of the cluster have registered, by origin, that
this device lacks — each row the repository's name with where its origin is.
Picking one clones it on the named device into the parent **Create repo**
remembers there, over a path field where none is remembered yet, with that
device's own GitHub token; registers it; and it becomes the draft's Repo, the
page landing as a pick on an existing row does. The Transfer dialog's
Preflight, where it finds no match on the device picked, reads *clone it
there* and does the same, Go unlocking once it is done. Demonstrable with
two servers, one of them holding a Repo with an origin the other lacks.

## Decisions in force

- **A list rather than a URL** ([ADR-0024](../../adr/0024-the-annex.md),
  *Clonable repos*). The cluster already matches Repos by origin — the rule
  under **Repo** in CONTEXT.md — so what could be cloned where is known, and
  nobody types an origin. A *Clone repo* row taking a URL was the
  recommendation and was rejected for this.
- **Cluster-wide, not an Annex feature.** The human's words: the problem is
  the same when starting a Conversation on any remote device. An Annex only
  presets where the clone lands.
- **The second group comes after the device's own**, behind a rule, and
  before the two rows that act. A Repo the device already has by origin is
  not in it; one with no origin anywhere is not clonable and is not in it.
- **The clone is the named device's operation**, reached over the Relay as
  Open repo and Create repo are, with that device's GitHub token and git
  author; refused in the far end's own words. The parent is the one Create
  repo remembers per device, asked for where none is remembered; stage 03
  presets an Annex's to `/srv/repos`.
- **The Transfer dialog's no match offers the clone** instead of pointing at
  Open repo there, and the Preflight is read again once it lands. *Unreachable*
  is still never *no match*.
- **Nothing registers twice**: a clone that lands on a path already registered
  is the registered Repo, as Open repo's is.

## Proposed tasks (provisional)

1. **What the cluster could clone here** — a reading of every member's
   registry (origin, name, default branch) over the Relay, minus what this
   device matches by origin, served beside the device's own Repo list.
   - Two servers: the one lacking the Repo lists it as clonable; the one
     holding it does not.
   - A member that does not answer contributes nothing and refuses nothing.
2. **The clone on a device** — `git clone` of the origin into the parent,
   with the device's token, then the registration Open repo makes.
   - A failed clone leaves no registration and says git's own words.
   - The clone into an already-registered path answers that Repo.
3. **The group in the dropdown** — the rows, the rule, the pick that clones
   and then picks; the path field where no parent is remembered; the
   remembered parent written as Create repo writes it.
   - Picking a clonable row on another device lands the draft on the Repo
     it made there.
4. **The Transfer dialog's no match** — *clone it there* in place of *Open
   repo there*, the clone run on the far device, the Preflight re-read.
   - A Conversation whose Repo the far device lacks transfers after one
     press on the offer.

## Re-verify at start

- Assumes the Repo dropdown's acting rows are `Action`s in
  `web/src/picking.tsx` and the composer is `web/src/workbench/Setup.tsx`
  with `listRepos(device)` per device.
- Assumes Create repo's remembered parent is `web/src/remembered.ts` and is
  kept per device.
- Assumes the Preflight's wording lives in `crates/server/src/transfers.rs`
  and the dialog in `web/src/workbench/Moving.tsx`.
- Assumes the origin match is the rule under **Repo** in CONTEXT.md and is
  applied on the device that asks, with the far end sending what git said.
- Assumes a device's GitHub token is `github_token` in its own
  `secrets.yaml` and reaches git as it does for a session.
