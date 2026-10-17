# 06. The preflight compares Authors

## What to build

A Transfer onto a device whose matching Repo points at another Author is
refused by the Preflight, naming both. Repointing the far Repo as part of the
move was turned down: the fix is a repoint over there, under the
live-conversation rule.

- **The far end's Repo answer carries its Author** — the uuid its Repo resolves
  to and that Author's display name — beside the origin URL it already
  answers.
- **A new `Lacking` finding** in the render crate's transfer module, read on the
  asking device as the others are: the Conversation's Repo and each companion
  are compared with their match over there **by uuid**, and a difference names
  the Repo (or companion) and both Authors.
- **A far Repo with no Author picked** — several Authors there and none
  recorded — is a finding too, saying it has no author picked on that device,
  fixed by picking one there (decided in planning).
- Both renderers say it: the viewer's Transfer dialog and the server's own
  words for `verkstead transfer`'s refusal, in `docs/copy.md`'s voice.

## Acceptance criteria

- [ ] A Transfer onto a device whose matching Repo points at another Author is
      refused with both Authors' names, in the dialog and from
      `verkstead transfer`, and repointing that Repo over there clears it.
- [ ] A companion whose match over there points at another Author is a finding
      naming the companion and both Authors.
- [ ] A far Repo with no Author picked is a finding saying so; the same Author
      on both sides, mirror or local, is no finding.
