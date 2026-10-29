# Adding to a running Conversation from an Answer

Amends [ADR-0021](0021-declared-mcp-servers.md), which says a Conversation's
MCP servers are attached and removed while the round drafts and fixed once the
work starts, and extends [ADR-0020](0020-cluster-mode.md)'s *Transfer*, whose
**Carried Conversation** was the one way a session was ended mid-turn and the
harness's own resume launched in its place. Both rules stand where they stood;
this adds a second moment at which the set of things a Conversation's sessions
are given can widen, and a second place the carrying-on runs.

The occasion is a Question Set. A grilling asks what the work needs and the
human, answering on a phone, finds the answer is *another repository*, *that
server*, or *the Windows box* — and until now the only way to give the agent
any of those was to leave the sheet, open the Steer form, add the companion
there, and come back. The **Steer** already widens the sandbox mid-life and
never narrows it (the companion-repos roadmap, stage 04), so the question was
never whether a Conversation's set may grow once the work has started; it was
where the human is standing when it does.

## The decision

**The Attach control on every Answer field, and on the set-level comment box,
is the same menu the composer draws**: *Attach file* first, then the declared
MCP servers with *Add a server…*, the registered Repos not yet on the
Conversation, opening a read-only Companion up to read-write, and the cluster's
other devices under *May be transferred to*. One control with more rows rather
than a second control, because the plus-menu already exists and a paperclip
that grew into it is one the human learned once.

Settled in the grilling of 2026-10-29. Each of the following was decided there,
and the alternative named is what was turned down.

- **A pick is recorded on the Set the moment it is made, and applied at
  Submit.** It survives a reload and shows on another device, × takes it off,
  and a settled Set refuses both — exactly as a file on an Answer behaves. What
  differs is when it reaches the Conversation: in the transaction that stores
  the Response, once, rather than per pick. Applying as picked was turned down
  because a companion rebuilds the sandbox, and a rebuild before the answers
  exist restarts the agent to no purpose, once per pick. Holding the picks in
  the page was turned down because the draft is per device and a phone's pick
  would be invisible to the laptop.

- **The Answer keeps a reference to what came with it.** The Response carries an
  `added` list beside `attachments`, filled on the way out the same way, naming
  each addition in words and where it is: the companion's directory and mode,
  the server's name and the prefix in front of its tools, the device's name and
  id. So the digest, a handoff and a Share can say what came with Q7, and the
  one addition that restarts nothing — a device tick — still reaches the agent
  that asked.

- **Only what changes the sandbox restarts the agent.** A Companion changes what
  is bound and an MCP server changes what the Built Root is written with, and a
  running session keeps what it started with; so where either was added and a
  session is waiting on that very Set, the session is ended and carried on. A
  device tick is checked live by `verkstead transfer` and needs only telling. A
  file already reaches the running session. Restarting on every addition was
  turned down as a restart with nothing behind it; restarting on none was turned
  down because the agent that asked is the one the human is answering.

- **Only the session waiting on that Set.** A Deferred Ask answered with a
  companion, or a Set answered after the session that asked it has gone, lands
  the addition on the record and nothing is restarted: the next launch binds the
  wider set, which is the deferred ask's own rule. Restarting any running
  session of the Conversation was turned down.

- **The carrying-on is the Carried Conversation's, on this device.** The session
  is idling in `verkstead ask`, so ending it loses nothing. The Submit ends it
  where it stands **before the Response is stored**, so the wait never prints
  the Answers and the resumed agent reads them once; the Conversation is
  widened, the checkouts made, and the launch resumes the harness primed with a
  note saying the sandbox changed and naming the Set with its `verkstead
  answers` line. Where the harness cannot resume — the five reasons ADR-0020
  names, and the harness refusing — Verkstead's own Resume stands, re-primed
  from the record, and the Timeline says which it was and why, as an arrival
  says it. Verkstead's Resume always was turned down because it throws away the
  agent's own thinking mid-interview for no gain where the harness can carry
  on.

- **Every question before anything is ended, and the making after.** The order
  is the Steer's: the record and git are asked everything a checkout turns on,
  and a fetch that fails, a branch already taken, a server whose declaration or
  a Repo whose registration has gone since the pick refuses the Submit by name
  with nothing ended and the Answers kept on the sheet, × taking the addition
  off. Storing the Answers and dropping the addition with a Notice was turned
  down: the human answered on the strength of what they were adding.

- **The record is a line under the Set's Event**, in the Steer's own shape,
  naming what was added with its mode or branch, and the carrying-on said as a
  transfer says it; the Set page draws applied additions as chips under the
  Answer, the way a frozen Brief draws its servers, and additions a lock or a
  Close left unapplied struck through. A Steer Event with a frozen form was
  turned down: nobody steered.

- **Every waiting Set offers it, whatever role asked**, and the comment box's
  menu is the whole menu, file included. A file put there is a set-level
  attachment, keyed by the Set alone, listed in the prompt under the Set's
  title with no question, and not an Answer to anything.

- **A companion added from a phone is a card**, prefilled read-only off the
  default branch, with the three fields the Steer's rows ask — the mode, the
  base, and the read-write branch name. Picking the repo alone and opening it
  up on a later Set was turned down as a second round trip for a decision the
  human has already made. The menu lists registered Repos only: registering is
  the trust boundary, done at a desk, and the Repo dropdown's *Open repo* and
  *Create repo* rows are not drawn here.

- **Pending additions cross with the record on a Transfer**, each Repo matched
  on the far end by the rule the Companion rows use, and one that does not match
  dropped and struck. Dropping them all was turned down.

## What this does not change

The Steer's companion section stands, and still does not offer MCP servers:
asked, the human had nothing to add. Nothing removes a Companion or puts one
back to read-only, and nothing takes a server off once the work has started —
the set still only widens. ADR-0021's strip and write-back are untouched: the
account's own servers are never carried into a Built Root, and a server
attached from an Answer is a declared one, by name, read afresh at each launch.
And a Transfer's own carrying-on is as it was; what this borrows is the
machinery, not the occasion.
