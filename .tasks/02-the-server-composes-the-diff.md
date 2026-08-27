# 02. The server composes the Diff

## What to build

The uncommitted changes attached to a Question Set stop being derived by the CLI
and start being read by the server. Nothing about how a Set is drawn changes in
this task — the same one Diff, from the same one repository — and everything
about where it comes from does.

**The server reads the Conversation's own worktree as the Set arrives.** It
knows which Conversation a Set is asked from without inferring it: the endpoint
is conversation-scoped, because that is the base URL the sandbox was given. So
the Diff is composed there, from the worktree that Conversation was checked out
into, and stored with the Set. The uncommitted changes are what they have always
been — everything not in the last commit, staged or not, plus the contents of
untracked files — and a clean worktree attaches nothing, as today.

This *strengthens* determinism over trust (ADR-0001) rather than bending it:
whatever the agent put in that field is overwritten, and now by the server's own
reads rather than by a CLI running wherever the agent happened to be standing. A
Conversation with no worktree — nothing has been checked out, or it has been
closed — attaches nothing rather than refusing the Set.

**Reading git blocks**, so it happens off the async worker the request is being
served on, the way every other patch read here does. The ask is on the critical
path of a session that is waiting, so the read is the repository's own cheap one
and nothing more.

**The CLI's Diff enrichment retires.** `verkstead ask` keeps deriving the project
and the branch — those are still the working directory's to answer — and stops
deriving the Diff, along with the machinery that only that derivation used. The
Guide ships inside the binary and currently promises agents that the CLI
attaches the uncommitted Diff, so it says what is true instead: the field is
still never supplied by the agent, and the server is what fills it.

**The glossary's Diff entry** says it is captured by the CLI at send time. It
becomes true here, so it is corrected here.

## Acceptance criteria

- [ ] A Set asked from a session whose worktree has uncommitted changes carries
      them, composed by the server, and one asked from a clean worktree carries
      no Diff — with the page drawing both exactly as it does today.
- [ ] A Set that arrives claiming a Diff of its own has it replaced by what the
      server read.
- [ ] `verkstead ask` no longer derives a Diff and still derives the project and
      the branch, and the Guide no longer says otherwise.
- [ ] CONTEXT.md's Diff entry says where the Diff now comes from.
