# 05. Launch on Startup, and the hidden login start

## What to build

**Launch on Startup** on the Desktop page, and the registration that is its whole
state. Read from the platform through the bridge and written to it, never copied
into the app's JSON file (Q12a): a human who turns it off with their desktop's own
settings has unchecked the box, and Verkstead agrees with them rather than argues.

**On Linux it is an XDG autostart entry, and it is the tray app's own file.** The
same name under the same directory, which is how the old registration is taken
over rather than doubled (ADR-0020). So the readings carry over exactly as the
Rust arm makes them, because a human may have edited that file:

- The entry being there is a checked box **unless** `Hidden=true` or
  `X-GNOME-Autostart-enabled=false` says otherwise — both read case-insensitively,
  both being how a desktop's own settings turn an entry off rather than delete it.
- The directory is `$XDG_CONFIG_HOME/autostart` where that names an absolute path
  and `~/.config/autostart` otherwise, which is the specification's own reading and
  the one the server gives its directories.
- The `Exec` path is quoted whole, so a binary under a directory with a space in it
  is still one argument.
- `$APPIMAGE` names the file in preference to the running executable where it is
  absolute, because an AppImage's own path is a mount that will not exist at the
  next login. There is no AppImage to meet until stage 05, and the reading belongs
  beside the writing rather than in the stage that packs one.

**What does not carry over is the command.** The Rust entry says a `desktop` verb
and `--no-open`, and this app has neither: the flag it writes is one of the app's
own, saying come up hidden.

**The box is greyed on an unpackaged run**, with a note saying it needs a packaged
app (Q2, settled with the human). Every run in this stage is `pnpm start` from a
checkout, and what an entry could name there is the dev shell's Electron in the
nix store plus the checkout directory — a registration that breaks the next time
either moves. So the whole path is written and tested here, and the writing is
refused where there is nothing worth writing.

**The entry is rewritten while it is there, at every launch**, as the tray app
does: an app that was moved leaves a registration naming a path that is no longer
anything, and the next launch by hand is the moment that heals. **Unregistered
stays unregistered** — nothing here ever writes a registration that was not
already asked for.

**The login start comes up hidden while the tray is shown, and shown otherwise**
(Q9): a launch carrying the flag opens no window where there is an icon to reach
the app by, and opens one where there is not — which is the same reading
`--no-open` made of a login for the tray app, and not a **Launch on Startup** that
needs the tray.

**The other two platforms are Electron's login-item API**, written here as arms and
proven in stages 06 and 07. The registration is still the state there: the box is
drawn from reading it back through the same call, and the hidden start is that
API's own.

**What vitest covers**: the entry's text, the two off-readings, the directory
resolution including the relative variable that is no answer, the quoting, and the
rewrite-while-present — which is the Rust arm's own suite, arm for arm, since all
of it is path and text. The hidden start is proven by launching with the flag.

## Acceptance criteria

- [ ] The box writes and removes the autostart entry under the name the Rust tray
      app already uses, and an entry a desktop turned off reads as unchecked.
- [ ] A launch with the hidden flag opens no window while the tray is shown, and
      opens one when it is not.
- [ ] An entry that is there is rewritten at every launch and one that is not is
      never written, and an unpackaged run greys the box with its note.
