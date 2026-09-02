# 05. The compose page

## What to build

A new compose page: the same composer UI as the brief pane, working a
temporary state with no saved Conversation behind it. It is reached from a
**New conversation** button in the sidebar head; the existing New-conversation
menu stays beside it until task 07 retires it, so nothing is ever unreachable
mid-branch.

The state — repo, brief text, branch name, base, companions, pairings — is
client-held and kept per device the way answer-sheet drafts are, so closing
the tab or reloading loses nothing. Nothing reaches the server until a
button:

- **Start** creates the Conversation and replays the state through the
  existing per-field setup endpoints, then kicks the work off as the start
  button does; it then navigates into the Conversation. A refused field
  leaves a part-set draft, shown as the draft it is with the refusal in
  place — no new batched endpoint, no second validation path.
- **Save as draft**, a secondary button to the left of Start, does the same
  minus the kickoff: it creates without starting and navigates into the
  draft. Enabled once a repo is picked; everything else may stay empty, as it
  always may while drafting.

The Repo dropdown's trigger reads **Select** before a repo is picked. The
layout is the widened one from task 04: the compose page has no timeline.
A successful create clears the device draft.

## Acceptance criteria

- [ ] The sidebar carries a New-conversation button opening the compose page,
      with the old menu still working beside it.
- [ ] Compose state survives a reload per device; nothing exists server-side
      until Start or Save as draft, and a successful create clears the
      device draft.
- [ ] Start creates, replays every touched field through the existing
      endpoints, kicks off and navigates in; Save as draft does the same
      without kicking off and enables as soon as a repo is picked.
- [ ] A field the server refuses during replay surfaces its named refusal on
      the resulting draft rather than losing anything.
