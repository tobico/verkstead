# 04. Update

## What to build

**Update** on the Components row moves the box's inputs and rebuilds it.
The re-pin at start (task 03) moves only Verkstead. Update moves **both nixpkgs
inputs**: `nixpkgs` along the stable channel the flake names, and
`nixpkgs-unstable`, which is where the harnesses come from. It also re-pins
Verkstead to the Keeper's release in the same pass. **NixOS-WSL does not move
on Update.** It is pinned at a release tag, and a new one arrives with a
Verkstead release (Set 30 Q2; ADR-0024 and CONTEXT.md amended on this branch).

The press runs, on the server and in the background:

1. **The refusal first.** If a session is running in the Annex, the press is
   refused, **naming the Conversation** (or the first of them and how many
   more), because a rebuild restarts the Annex's server and every session with
   it. Writing it down to run once the Annex is idle was the alternative, and it
   was not taken. The Keeper already holds the Annex's Conversation list through
   the merge (`crate::merging`, read over the Relay). Use that list, or ask the
   Annex directly at the moment of the press, whichever says *running* reliably.
   A list that is stale or unreadable refuses rather than guesses.
2. The flake and configuration written again, as task 03 does.
3. The lock updated for every input but `nixos-wsl`.
4. `nixos-rebuild switch`, the row reading *updating*, and the rebuild's
   last lines as the trouble line on failure.
5. The Annex linked again once it answers, by the same idempotent link the start
   runs, because the rebuild restarted its server.

One Update at a time, and none while task 03's re-pin is running. A second
press reads the one already going.

The refusal and every line on the row are copy. Read `docs/copy.md` first.

## Acceptance criteria

- [ ] Update with a session running in the Annex is refused with a line naming
      that Conversation, and nothing is rebuilt.
- [ ] Update on an idle Annex leaves a `flake.lock` whose nixpkgs inputs are
      newer and whose `nixos-wsl` is unchanged. The Annex comes back running
      and linked, and the row reads *updating* until it does.
- [ ] A failed rebuild leaves its trouble line on the row and the Annex
      answering on what it ran before.
- [ ] The refusal and the input selection are covered over the stub `wsl.exe`,
      and the whole press is proven once on Windows.
