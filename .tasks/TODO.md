# Conversation terminals

A Conversation gets a terminal of its own: a **Terminal** icon button beside
Share on the Timeline's head opens a details pane holding a tab bar of shells,
each one a pseudo-terminal the server holds, run inside the Conversation's
Sandbox with its Worktree as the working directory. It is for the moment the
agent's work is done and a human wants to try it, make a small change or work
with git — without leaving the workbench, and without the run noticing.

The grilling settled it as the Screen's own machinery pointed at a shell rather
than an agent: the same xterm over the same kind of socket to the same
server-held virtual terminal, in the implementation Profile's Sandbox with a
session's whole environment, running the server user's login shell inside the
worktree's dev shell. Memory only — no Capture, no Event, nothing in a Share —
and alive until the shell exits, its tab is closed, the Conversation closes or
the server stops. ADR-0013 and the `CONTEXT.md` **Terminal** entry are written
beside this plan.

## Tasks

- [x] 01: One terminal, end to end — [details](01-one-terminal.md)
- [x] 02: The login shell — [details](02-login-shell.md)
- [ ] 03: Tabs — [details](03-tabs.md)
- [ ] 04: Tabs ending — [details](04-tabs-ending.md)
- [ ] 05: Closing — [details](05-closing.md)
- [ ] 06: Titles — [details](06-titles.md)
