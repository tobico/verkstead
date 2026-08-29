# 09. Removing a repo

## What to build

A Repo can be removed, end to end — and removal is an **unregister rather
than a delete**, because every Conversation row references its Repo and
history must keep it (settled in the grilling; hard delete was rejected as it
would bar removal forever after first use).

In the store, the repos table gains a flag for an unregistered row. Every
read that lists or resolves Repos for new work — the settings list, the
sidebar's New conversation menu, the abandoned-roadmaps read — stops showing
flagged rows; a Conversation already on one keeps resolving its Repo by id,
so nothing on a Timeline changes. Registering a path that matches a flagged
row revives that row rather than being refused as already registered (the
path stays unique either way).

Removal is refused while any Conversation that is not Done or Closed is on
the Repo, with a named outcome the UI can say — mirroring how removing an
in-use Agent Profile is refused. The outcome enum travels the same way the
Profile one does: store → server module → render types → the UI's refusal
map.

In the UI, Remove sits in the repo details pane from task 08, beside the
facts. A refusal is said in the pane; a success takes the pane away, drops
the repo from every list, and leaves past Conversations untouched.

## Acceptance criteria

- [ ] Removing a repo flags it: gone from the settings list, the New
      conversation menu and the roadmap offers, while existing Conversations
      still show their repo
- [ ] Removal is refused with a said reason while a live (not Done/Closed)
      Conversation is on the repo
- [ ] Re-registering the same path revives the repo, and the Remove button
      lives in the repo details pane with its refusal said there
