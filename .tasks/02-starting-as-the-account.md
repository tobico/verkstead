# 02. Starting a process as the account, on a console

## What to build

The two ways Verkstead starts a process gain an arm that starts it **as the
session account** — the one that reads back what a process printed, and the one
that puts a process on a pseudoconsole. Still nothing switches: sessions go on
running inside the container, and what proves this task is a test that starts a
probe as the account and reads what came back.

Starting as another account without elevation means `CreateProcessWithLogonW`,
which needs the account's password and nothing else. It takes the same four
things a rendering already comes to — the program, the command line, the
environment block, the directory it starts in — so the words a rendering becomes
are reused as they are. Standard handles are inherited by it, which is how what
a process printed is read back.

**It will not carry an attribute list**, which is the whole shape of the console
arm: handing it an extended startup info is refused outright with *The parameter
is incorrect*, so a pseudoconsole cannot be given to a process started as
somebody else. The console is made on the far side instead. A **launcher** — a
verb of Verkstead's own binary, which a session already has bound in read-only —
is started as the account with the console's two pipes as its plain standard
handles; it makes the pseudoconsole over those, starts the real program on it
with an ordinary `CreateProcessW`, waits, and reports how that ended. Nothing
has to be duplicated across the boundary: the handles the console needs are the
ones the launcher inherits.

Verkstead keeps the launcher's process handle, so the Job Object that already
kills a session's whole tree holds the launcher and everything under it. Two
things still have to reach the launcher after it starts — a resize, and the exit
code of what it launched — and both go over a channel of the launcher's own
rather than being inferred: a launcher that exits does not mean the program it
launched exited well.

The launcher is a verb nobody types. It should be as hard to invoke by accident
as the rest of Verkstead's internal surface, and it should refuse plainly if its
standard handles are not the pipes it expects.

## Acceptance criteria

- [ ] A process started as the account runs, and what it printed is read back
  off its standard handles
- [ ] A rendering that names the account refuses to become a plain `Command`,
  the way one naming a container already does
- [ ] The launcher, started as the account, makes a pseudoconsole and a marker
  printed on it by the program it started comes back off the console's pipe
- [ ] Resizing reaches the launcher, and the exit code of the program it
  launched reaches Verkstead rather than the launcher's own
- [ ] The Job Object holds the launcher and the process under it: closing it
  takes both
- [ ] Starting as an account with no password, or a wrong one, fails with a
  refusal that says which
