# Verkstead runs the login

Builds on [ADR-0011](0011-agent-backends.md), whose Built Root is what a
session is given of its account and whose write-back is how a login a session
made reaches that account, and on [ADR-0020](0020-cluster-mode.md), whose
*Shared Profiles* are why a Profile can be at home on another device. The
feature that builds it is planned in `.tasks/` as *Claude login modal*.

## What was there

A Claude session whose login expired, or was refused, printed
`<reason> · Please run /login` and waited. What it needed was a human at its
terminal to type `/login`, open the address it printed, and paste the code
that page handed back into the same terminal. Through the browser terminal that
was unreliable: a long address wrapped and broken by the terminal's width, a
paste that landed in the wrong place, and a phone keyboard over the half of
the screen the code was going into. And nothing said the session was waiting
for it. The run sat there, quiet, until somebody went and looked.

## The decision

Verkstead runs the login itself, and the human finishes it in a modal any
device can open.

- **A login of its own on the account, not keystrokes into the session.**
  What runs is `claude auth login --claudeai`, a process of its own, rather
  than `/login` typed into the session that asked for it. A session is an
  agent's terminal, and the login is the account's business: one login fixes
  every run on that Profile, including runs that have not started yet. Typing
  into a session would also mean reading its screen to find out where the
  login had got to, and a screen is the least reliable record Verkstead has.
  Turned down: driving `/login` through the session's terminal.

- **In the Profile's Built Root, in the sandbox.** The login is run with the
  root a session of that Profile gets — see `Sandbox::for_login` in
  `crates/server/src/sandbox.rs` — so the file it writes reaches the account
  by the write-back a session's ending already does, on Linux, macOS and
  Windows alike. There is no second way of writing a login into an account to
  keep in step with the first. Turned down: running `claude` on the host
  against the account's own `~/.claude`, which is exactly the account access
  ADR-0011 took away from everything Verkstead launches. Its home is a
  directory of the login's own beside the Conversations' (`homes/login-<id>`),
  with no Worktree, handoff directory or server in it.

- **Over plain pipes.** Claude Code 2.1.283 prints
  `If the browser didn't open, visit: <url>` on a line of its own, then
  `Paste code here if prompted > ` with no line ending, and reads the code from
  standard input. That is the whole conversation, and it needs no terminal. So
  the address is read off standard output, the code is written to standard
  input, and `BROWSER` is a program that does nothing, so that nothing opens on
  the server. A clean exit is half of success. The other half is
  `claude auth status`, which exits `0` only for an account that is logged in.
  It is asked in a fresh root, which is the login as the next session will be
  given it.

- **One login per Profile.** A second device opening the modal joins the login
  already running and sees the same address. A second process would print a
  second address, and only one of the two codes could ever count. Each modal
  names itself as it opens. The login is killed when the last one closes, or
  after ten minutes: a modal on a phone that went into a pocket says nothing
  as it goes.

- **Claude only, and only Profiles at home on this device.** The other
  harnesses log in differently, and none of them has needed this yet. A
  mirrored Profile's account is a copy, refreshed from its home device before
  each launch, so a login written into it would be overwritten by the next
  refresh. It is logged in on the device it is at home on, and its card offers
  no Log in. Logging a mirrored Profile in across the cluster is left for
  later.

- **The phrase rule.** A session has signed out when one of its lines **ends**
  with `Please run /login`, once terminal decoration is stripped off it. Claude
  says it as `<reason> · Please run /login`. Anchoring the phrase at the end
  keeps a session that merely *mentions* the phrase — an agent reading this
  ADR aloud, a grep of the source — from being stopped. The match is checked
  against a named claude version, the way `limits.rs` records its own. It
  writes a **Signed out** stop through the shared stop machinery, like every
  other stop: one Notice, *blocked on you*, the devices told. The Notice has a
  Log in press that opens the same modal.

- **A login resumes what it unblocks.** A successful login resumes every run
  on that Profile that is stopped as Signed out. Each resume builds a fresh
  root, which is what picks the new login up: on Linux the old session's bind
  still holds the old file. A session that signs out *after* the human has
  already logged in again was built before the login. When the account's
  login is newer than that session's root, it resumes straight away instead
  of stopping.

- **And a launch is checked first.** Before a Claude session is launched under
  a local Profile, `claude auth status` is asked of its account. No login
  refuses the launch before a session starts, and the Profile reads as broken
  — signed out — until the card's Log in fixes it.

## Not yet

- **Windows.** A login there would be started as the session account with its
  standard input held open for the code. Nothing in `sandbox/starting.rs`
  does that yet: what is there writes all of a process's input up front. Until
  it does, the Log in press is refused on Windows with a message that says so.

## Consequences

- A login no longer needs the browser terminal, and it can be finished from
  any device the workbench is open on.
- A login is a process Verkstead holds, like a session. It has a limit, it is
  killed when nobody is watching it, and there is at most one per Profile.
- The root register in `sandbox/sharing.rs` is keyed by the directory a root
  is built in, not by a Conversation id. A login has such a directory and no
  Conversation.
