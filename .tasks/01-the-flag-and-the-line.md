# 01. The flag, and the line without the key

## What to build

`verkstead serve --desktop` serves exactly what `verkstead serve` serves, and
differs in one visible thing: the one line an operator reads as Verkstead comes
up names the workbench address alone rather than the login link. A plain
`verkstead serve` is unchanged and keeps the whole link in its line — a machine
started from a unit file has no tray and no window, and what somebody reading
the journal pastes is the link (ADR-0015, as amended).

**The flag is the serve verb's rather than the server's configuration.** It does
not go on the server's `Config`: that struct is flattened into the tray app's own
verb as well, and a `verkstead desktop --desktop` would be a flag meaning nothing
on the verb that already hands the link over in-process. So the serve verb
becomes a thing of its own — the flag, with `Config` flattened beneath it — and
what crosses into the server is a value naming *who started it* rather than a
field the configuration carries. Mind the existing help: `Config` is a
`clap::Parser` carrying the verb's own name and about, it is flattened in two
places, and there is already a test asserting what `verkstead serve --help` says
about the flags and their defaults.

**What that value selects here is `HandsOverTheLink`.** The server has both arms
already and the tray app already passes the caller-hands-over one in-process; what
is new is the same arm being reached from the command line, through the entry
point a plain `serve` comes in at. Task 03 hangs the second behaviour off the very
same value, so what this task settles is that seam as much as the line — which
argues for a named value rather than a bare `bool`:

```rust
/// Who started this server, which is the whole of what the flag says.
pub enum StartedBy {
    /// A shell, or a unit file: the line hands the link over, and there is
    /// nobody at the machine to put a password dialog in front of.
    AnOperator,
    /// The desktop app, which started this as its sidecar: it opens its own
    /// window on the link, so the line names the address alone.
    TheDesktopApp,
}
```

**Nothing about the flag reaches the wire.** No `AppState`, nothing serialisable,
nothing a viewer can read (ADR-0020): the viewer learns it is inside the app from
the preload bridge in stage 03, and this is behaviour on this side of the socket
only.

**The key is not hidden, only kept off that line.** It stays in its own file in
the Data Directory at the mode it has always had, which is where the Electron app
reads it from in stage 02 — a sidecar printing the link on a machine-readable
line was considered and rejected, the file being the documented thing. A human
who runs `serve --desktop` by hand reads it from there, and the startup line
already names which Data Directory that turned out to be.

The flag's own help text says it is the desktop app's and nobody else's, and
CONTEXT.md's **Workbench Key** entry — which already says the daemon's line
carries the key and the desktop app's does not — gains the sidecar as the third
install and the reason it redacts.

## Acceptance criteria

- [ ] `serve --desktop` logs the workbench as the address alone, and plain
      `serve` still logs the whole login link — a browser opened on what each
      logged lands where that line promised.
- [ ] A `--desktop` run serves the workbench and answers `verkstead ask` from the
      same binary, with the key read from its file in the Data Directory.
- [ ] `verkstead serve --help` names the flag as the desktop app's, and
      `verkstead desktop --help` does not name it at all.
- [ ] CONTEXT.md's **Workbench Key** entry says which install redacts the startup
      line and why.
