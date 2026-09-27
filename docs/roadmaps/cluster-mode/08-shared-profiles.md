# 08. Shared Profiles

## Goal

An Agent Profile from one device runs sessions on another. Every device's
pickers list every member's Profiles with the device on the row; every
Profiles section lists everyone's, an edit or a removal relayed to the home
device. Start on B under a Profile whose account lives on A, and B mirrors
the login and configuration from A before launch, syncs this Repo's memory
entries over with paths rewritten, and writes both back to A as the session
ends. A Profile whose home has no login file, or whose harness is absent on
B, reads as broken there and Start is refused by name. Repos are matched
across devices by origin URL, then by name. Demonstrable end to end: a Claude
login on A, a session on B, its transcript back on A.

## Decisions in force

- **Mirror rows** ([ADR-0020](../../adr/0020-cluster-mode.md), *Shared
  Profiles*): a local row per member Profile with its home device and id
  there, refreshed over the link, so every Profile id stays local. A device
  column on every pairing was rejected.
- **Every Profiles section lists everyone's**, edits relayed home — the
  human's choice over home-only editing.
- **Away from home**: login and configuration mirrored under the Data
  Directory before launch, written back after; the memory switch honoured by
  syncing this Repo's store entries over and back, their paths rewritten to
  the landing machine. Fetching the whole account was rejected.
- **Last write wins on a login** refreshed on two machines; a sign-out reads
  as broken there, the fix a login at home. Exclusive lending was rejected.
- **No login file means not usable away**, and the row says so; **no harness
  on the device** reads as broken in the onboarding probe's words.
- **Repos match by origin URL, then by name** where neither has one, needed
  here for the path rewrite and again by transfer.

## Proposed tasks (provisional)

1. **Mirror rows and the cluster-wide picker** — member Profiles fetched and
   upserted as mirrors with home and remote id; the pickers and the Profiles
   section draw the device; edits and removals relayed.
   - Removing a Profile at home nulls pairings on every device, as locally.
2. **Repo matching** — a reading that says which of B's Repos is A's, by
   origin then name.
   - Two Repos with the same name and different origins do not match.
3. **The account mirror** — login and configuration fetched from the home
   device into a per-Profile directory under the Data Directory, the Built
   Root made from it, the login written back at session end.
   - A token refreshed inside the session lands in the home account.
4. **Memory sync** — this Repo's entries pulled before launch with paths
   rewritten for the landing machine, pushed back after.
   - The session's transcript is readable on the home device afterwards.
5. **Broken states** — no login file at home, harness absent here, home
   unreachable — each named on the row and refusing Start.

## Re-verify at start

- Stage 04 landed: `/api/ui/profiles` relays; a peer route for account files
  is new and must be gated to members.
- The Profile is still name, account, models, memory in
  `crates/store/src/profiles.rs`, brokenness answered per read in
  `crates/server/src/profiles.rs`.
- The Built Root is still `crates/server/src/sandbox/root.rs` in four parts
  with write-back at end; the memory store's shape per harness is there.
- Claude's memory is still `projects/<encoded path>` under the account; the
  encoding rule is what the path rewrite has to match.
- `across_volumes` on Windows still requires the account and Data Directory on
  one volume — a mirror under the Data Directory satisfies it by construction.
