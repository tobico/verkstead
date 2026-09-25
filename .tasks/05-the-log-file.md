# 05. The log file

## What to build

`verkstead.log` in the **Log Directory**, written by the app: the sidecar's
stdout and the app's own lines in the one file, with the byte-order mark and the
roll the Rust app gave it. The sidecar goes on logging to stdout exactly as
`verkstead serve` always has — nothing about its logging changes, and `RUST_LOG`
filters that stdout as it always did — and what is new is that somebody is
reading it (ADR-0020).

**The Log Directory is the platform's and the app makes it**:
`~/.local/state/verkstead` on Linux — `$XDG_STATE_HOME` where that is set to an
absolute path — `~/Library/Logs/Verkstead` on macOS, `%LOCALAPPDATA%\Verkstead`
on Windows, the local rather than the roaming application data because a log file
follows nobody between machines. It is not the Data Directory and nothing says
where it goes; the server resolves it already and deliberately does not create
it. Stage 03's **View Logs** is what opens the file, on the tray and on the
Desktop page both, so what this task settles is where it is and what is in it.

**The rules the Rust app set, kept as they are.** The file rolls at 4 MiB to
`verkstead.log.1` and keeps nothing behind that — at most two files, at most
twice the bound, whatever the machine has been up to. A file this app *starts*
opens with the UTF-8 byte-order mark, a fresh one and each roll both, and a run
appending to a file that already has content adds none: Verkstead's own messages
have em-dashes in them, and a Windows log viewer with no mark reads them in the
machine's code page, which is mojibake to the very person being asked to report
what it says.

**Nowhere to put one is not a failure.** A machine that names no Log Directory,
or one whose directory cannot be opened, gets no file: the app says so, logs to
its standard error instead, and goes on running — the opposite of what a missing
Data Directory means, and deliberately. A Verkstead with nowhere to keep a log
has only lost the log.

**And the startup line in it carries no key**, because the sidecar was started
with `--desktop`: the app has read the key out of the Data Directory and opened
its own window on the link, so the line names the address alone and the file on
somebody's desk is not a login for whoever reads over their shoulder (ADR-0015,
as amended). The one exception is a machine with no display, where the sidecar
says the link after all — and an app that could not have drawn a window is an app
that is not writing this file.

**What vitest covers**: the roll and the mark, over a directory a test hands it,
and the Log Directory resolved per platform from values it is given.

## Acceptance criteria

- [ ] The sidecar's startup line and the app's own lines are in `verkstead.log`
      under the Log Directory, interleaved as they happened.
- [ ] That startup line names the workbench address and carries no key.
- [ ] A file the app starts opens with the byte-order mark and one it appends to
      gains none; the file rolls to `verkstead.log.1` at 4 MiB and nothing is kept
      behind that.
- [ ] A Log Directory that cannot be made or opened leaves the app running and
      logging to standard error, saying where the log went instead.
