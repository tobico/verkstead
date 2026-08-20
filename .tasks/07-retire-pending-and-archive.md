# 07. Retire the pending and archive namespaces

## What to build

The standalone pending and archive lists go. Stage 01 kept them so the phone
never stopped answering while the workbench was being built; once Question Sets
arrive on a Timeline they are reached through their Conversation, and a second
way in is a second thing to keep working.

This is wider than it sounds, and the width is the reason it is its own task
rather than a tidy-up at the end of the previous one:

- `/api/ui/pending` and `/api/ui/archive`, and the handlers behind them
- the viewer's pending and archive pages, and the App routing that reaches them
- the tests that cover both, across the server crate, the store and the viewer
- `nix/vm-test.nix`, which drives its **entire** agent round trip through
  `/api/ui/pending` — the helper that waits for a Set to arrive, and both
  subtests that use it

The VM test is the load-bearing part. Its round trip is still worth having, so
re-express it through a Conversation rather than deleting it: a Set submitted
from a sandboxed session, found on its Conversation's Timeline, answered
through the API the viewer posts through, and printed by the CLI. The restart
subtest keeps its point too — a pending Set and its waiting agent surviving the
service stopping and starting.

What is being retired is the *route into* Sets, not the Archive itself: settled
Sets remain, reachable through the Conversation they belong to. A Set with no
Conversation cannot exist once the previous task landed, so nothing is
orphaned by this.

## Acceptance criteria

- [ ] No `/api/ui/pending` or `/api/ui/archive` route remains, and the viewer
      has no standalone pending or archive page
- [ ] Settled Sets are still reachable through their Conversation's Timeline
- [ ] The VM test still proves the agent round trip end to end, driven through a
      Conversation
- [ ] The VM test still proves a pending Set and its waiting agent survive a
      restart
- [ ] No test is deleted merely because its route went — each is either moved to
      the Conversation route or explicitly no longer meaningful
- [ ] `nix flake check` passes
