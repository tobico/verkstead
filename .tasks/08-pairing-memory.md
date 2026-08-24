# 08. Remember pairings per repo

## What to build

The last-used agent pairings are remembered per repo, server-side, so a new
conversation on that repo arrives with both pickers already filled.

- **Written at grill start:** when grilling starts, record the
  conversation's grilling and implementation pairings (profile and model)
  against its repo, replacing what was there.
- **Read at conversation creation:** a new conversation on that repo is
  created with both pairings prefilled. The pickers show them and they stay
  changeable — this is a default, not a lock.
- **Stale memory is dropped silently:** a remembered pairing whose profile
  no longer exists, is broken, or no longer lists that model is not applied;
  the picker arrives unchosen as today.
- Server-side storage was chosen over browser storage so phone and desktop
  share the memory; a new small store table keyed by repo, following the
  store crate's existing schema conventions.

## Acceptance criteria

- [ ] Start grilling on a repo, add a new conversation on the same repo:
      both pickers arrive filled with the remembered pairings
- [ ] The prefill can be changed before grilling, and the change is what
      grill start records
- [ ] A remembered pairing whose profile or model has since gone arrives
      unchosen, with no error
- [ ] A repo with no memory behaves exactly as today
