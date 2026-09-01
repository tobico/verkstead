# 08. A session outlives nothing

## What to build

The replacement for the one bubblewrap flag that has no macOS equivalent.

On Linux every session and the Compile Server are started with
`--die-with-parent`, and ADR-0012 leans on it: the tray's **Exit** is a stop
where it stands, with no shutdown path anywhere in the server, precisely because
what it leaves behind is nothing. Apple's sandbox offers no such thing — a
sandboxed process is an ordinary child, and killing the server leaves it running
with a Worktree open and an agent still talking to a model.

So the lifetime becomes Verkstead's own on macOS: whatever starts a session is
what makes sure it does not outlive the server, however the server ends —
**Exit** chosen off the tray menu, the process killed, or the app stopping on its
own account. The Compile Server goes the same way, being a sandboxed child like
any other.

Linux keeps the flag. What is written here is the arm for the platform that has
no flag to keep, not a second mechanism for the platform that does.

## Acceptance criteria

- [ ] Exit on the tray leaves no session process and no compile server behind
- [ ] A server killed outright leaves neither behind either
- [ ] Linux still relies on `--die-with-parent` and nothing about its lifetimes
      changes
