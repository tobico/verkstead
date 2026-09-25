# 06. The merged list

## Goal

One sidebar for the cluster. On any member, the list holds every member's
Conversations merged by rank, each row's second line reading OS icon, device,
then repo — this device's own rows too — with the pane header carrying the
device beside the branch and the row read aloud saying it. A drag across
devices writes one rank to the owning device and the order reads the same
everywhere. A member's push news reaches the hub's phones with the device
leading the title; the hub's archived switch governs the merged view; a member
that stops answering keeps its rows, dimmed *unreachable*, presses on them
refused by name. Demonstrable end to end with two devices and a phone.

## Decisions in force

- **Served from memory** ([ADR-0020](../../adr/0020-cluster-mode.md), *The
  opened device relays*): the hub keeps each member's list, refreshed on that
  member's nudges, and merges with its own. Fanning out per load was rejected.
- **Merged by rank** (stage 05): a drag computes one key between its
  neighbours whichever devices they belong to, and writes it to the owning
  device through the relay.
- **The row** reads OS icon, device, repo on the second line once anything is
  linked, on every row; the Brief's *before the branch name* was read as the
  second line since the branch is the first. The header and the spoken row
  carry the device too.
- **Push news is relayed**; a phone installed from one device hears from all.
- **The hub's archived switch** governs; each member is asked accordingly.
- **Unreachable rows stay**, dimmed, from the last list held.

## Proposed tasks (provisional)

1. **Member lists in memory** — per member, the list fetched on connect and
   on the member's `conversations` nudges, held on the hub.
   - Creating a Conversation on B shows on A within a nudge.
2. **The merged endpoint** — `/api/ui/conversations` answering the merge with
   a device on every row and a reachability flag.
   - Ranks from two devices interleave; an unreachable member's rows carry
     the flag.
3. **The row, the header and the spoken row** — the second line, the icon,
   the header mark, the aria label.
   - A lone device draws no device on any row.
4. **Cross-device drag** — the rank computed on the merged neighbours and
   written to `/api/ui/devices/{device}/…` for a remote row.
   - Reloading either device shows the same order.
5. **Push relay and the archived switch** — member news heard over the link
   and pushed locally with the device name leading; the archived flag passed
   to each member's list read.
   - A stop on B lights A's phone with *B — …*.

## Re-verify at start

- Stages 04 and 05 landed: the relay, the member nudge streams, and ranks.
- `ConversationEntry` in `crates/render/src/conversations.rs` still carries
  `repo` as a name only and no device.
- The sidebar is still one `For` over `shown()` in
  `web/src/workbench/Conversations.tsx`, the spoken row in `spoken()`.
- `News` in `crates/server/src/push.rs` and the archived switch in
  `crates/store/src/archives.rs`.
