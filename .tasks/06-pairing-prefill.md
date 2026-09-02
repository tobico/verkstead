# 06. Pairing prefill on compose

## What to build

A small read endpoint serving the set of Pairings a Repo was last grilled
with — the same per-Repo memory creation prefills from, validated the same
way: a remembered Pairing whose Profile has broken or which no longer lists
the model comes back as nothing, exactly as creation would silently skip it.

The compose page reads it the moment a repo is picked, so its three role
dropdowns show the same defaults a freshly created draft would show, before
anything is created. Switching the repo re-reads. A picker the human then
touches is theirs; one left showing the prefill is left untouched at create
time, so the server's own prefill applies and the replay from task 05 has
nothing to send for it.

## Acceptance criteria

- [ ] The endpoint returns the repo's remembered, still-valid Pairings per
      role, and nothing for a role whose memory no longer applies; server
      tests cover both.
- [ ] Picking a repo on compose fills the role dropdowns with the prefill,
      and switching repos re-reads it without disturbing a role the human
      has touched.
- [ ] A role left on its prefill is not replayed at create — the created
      Conversation carries the server's own prefill for it.
