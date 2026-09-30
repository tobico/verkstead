# 04. The prompt and the Guide

## What to build

Telling the agent the call exists, **only where the list is not empty**
(ADR-0020, *The agent's call*).

- **In the prompt**, applied at the one launch point where the other
  prompt-wide additions (`alongside`, `attached`, `naming`, …) are, so every
  kind of session gets it: a short section naming each permitted device other
  than this one — its **name, its id and its OS** (a WSL as the OS it reports,
  as the device select draws it) — and when to reach for
  `verkstead transfer <device>`: **work the current platform cannot do**, such as
  building or testing for another operating system. That the call ends this
  session at the turn's end and the work carries on over there, and that a
  refused call says why so the agent can pick another device or ask the human.
- **In the note a carried session starts on.** A Conversation carried on at the
  far end (stage 10) is started on its arrival note rather than on the launch
  prompt, so that note carries the list too — **named from this device's side**:
  the device it came from listed if it is still permitted (it always is when it
  is the drafting device), this device not listed.
- **A Conversation with no ticks, and no other permitted device, says nothing
  about transfer** anywhere in its prompt.
- **The Guide** (`verkstead guide`, the markdown the CLI embeds) gains a section
  on the call: when to use it, by name or id, what a refusal means, that it ends
  the session at the turn's end, and that it is only offered where the human
  ticked devices. The CLI's own help for the verb says the same in brief.

## Acceptance criteria

- [ ] A session on a Conversation with ticked devices is prompted with each
      one's name, id and OS and the reason to reach for the call; one with none
      ticked gets no mention of transfer.
- [ ] A carried session on B is told the devices it may move to from B, the
      drafting device A among them and B not.
- [ ] `verkstead guide` prints the new section, and its tests cover it.
