# 03. The rotating log file, and View Logs

## What to build

The server's `tracing` goes to a file, and the menu gains the item that opens
it. A tray app launched from an icon has no stdout anybody will ever read,
which is the whole reason the **Log Directory** exists — `~/.local/state/verkstead`
on Linux, resolved by the helper stage 01 landed. **This binary is what makes
that directory**: stage 01 deliberately resolves without creating, the way the
Build Cache's own resolution does, and the binary that opens a file in it is
the binary that makes it.

**Rotating, and small.** The file is bounded by size and rolls over rather than
growing without end, keeping enough behind it to be worth reading — a machine
that has been running Verkstead for months should not be handed a log nobody
can open. The plain CLI is untouched: `verkstead serve` goes on writing to
stdout wherever it was started from, and the same `RUST_LOG` filtering applies
in both, because where the events go is the starting binary's call and not the
library's.

**Nowhere to resolve a Log Directory is not fatal.** A machine with no `HOME`
and no `XDG_STATE_HOME` gets no file — the app says so, logs to stderr instead
and goes on running, because putting a tray up is still the job it was launched
to do. That is the opposite of the Data Directory's refusal, and deliberately
so: a Verkstead with nowhere to keep its database has nothing to serve, while
one with nowhere to keep a log file has only lost the log.

**View Logs** joins the menu above Exit and opens the file with the platform's
own opener, the same way Open reaches the browser. Where there is no file — the
case above — the item says as much rather than opening nothing.

**And the words this makes untrue.** CONTEXT.md's **Log Directory** entry says
it "stands empty and uncreated until there is a desktop binary to write in it",
and this task is that binary. The entry is amended the way stage 01 amended
**Data Directory**: what it now holds, who makes it and what happens on a
machine that has nowhere for one.

## Acceptance criteria

- [ ] After launch the Log Directory exists and holds a log file carrying the
      server's own startup line, with `RUST_LOG` filtering it as it filters the
      CLI's stdout.
- [ ] The file rotates at its size bound rather than growing without end, and
      what it rolls over to is bounded too.
- [ ] **View Logs** opens the file with the platform opener; a machine with
      nowhere to resolve a Log Directory still runs the tray, saying where the
      logging went instead.
- [ ] No entry in CONTEXT.md still says the Log Directory stands uncreated.
