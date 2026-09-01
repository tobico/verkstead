# 02. The tray icon, Open and Exit

## What to build

The icon in the system tray, and the two menu items that need nothing else
built first. **Open** opens the viewer in the default browser — the same thing
launch already does, now on demand — and is the menu's default item, so a
double-click on the icon does it too where the platform has one. **Exit**
quits. The artwork is Verkstead's own, embedded in the binary from
`assets/icons/` rather than read off disk beside it: this binary ships as one
file, and the packaging icons stage 03 generates are a different set for a
different purpose.

**The other two items are not here.** **View Logs** and **Launch on Startup**
arrive with the tasks that make them work, rather than sitting in the menu
disabled: a menu item that does nothing is a worse first impression than a
menu that is honestly short.

**Exit does what stopping the daemon already does.** The server has never had a
shutdown path — no signal handling anywhere in it, and `run` returns only on an
error — so under systemd it is stopped by SIGTERM and dies where it stands.
Exit is that, and no more: the tray does not get its own graceful-shutdown
machinery that no other caller of the server has. What matters is that quitting
actually quits, leaving nothing listening on the port and no session or
compile-server child of the running server behind, exactly as stopping the unit
leaves nothing behind.

**The event loop is the shape task 01 settled.** GTK's main loop holds the main
thread, the tokio runtime the server is on runs beside it, and a menu event
crosses between the two. A CI runner has no display and no tray host, so what
can be tested there is what the crate can assert without one — the menu's
construction, the item wiring, the opener's decision about what URL to open —
and the icon appearing, Open opening and double-click doing the same are
checked by running it on this machine.

## Acceptance criteria

- [ ] The Verkstead icon appears in the tray on this machine, with a menu of
      **Open** and **Exit**; Open opens the viewer, and so does a double-click
      on the icon.
- [ ] Exit quits: nothing is left listening on the port, and no session or
      compile-server process outlives the app.
- [ ] `cargo test --workspace` and the clippy line stay green on a runner with
      no display, with whatever of the menu can be asserted headlessly covered.
