# 01. The menus the page needs nothing for

## What to build

The three menus a Mac can carry without the page telling the app anything:
Settings… in the application menu, a File menu and a Help menu. On `darwin`
only — the Windows and Linux templates stay exactly what they are, which the
existing roles test already pins item for item.

**The application menu, written out.** Today it is the `appMenu` role, which
Electron fills in and nothing can be added to. It becomes its own items so that
Settings… can stand where a Mac puts it: About · rule · **Settings…** `⌘,` ·
rule · Services · rule · Hide · Hide Others · Show All · rule · Quit — each of
the platform's items still its role (`about`, `services`, `hide`, `hideOthers`,
`unhide`, `quit`), so their labels and keys stay the platform's. Settings…
opens the settings page itself (`/settings`), not the Desktop section of it
(Set 1281 Q1, Q1a). It is called *Settings…* and not *Preferences…*: Apple
renamed it in macOS 13, the workbench calls the page Settings, and the
vocabulary lists *preferences* under Avoid.

**File.** **New Conversation** `⌘N` opens the compose page (`/compose`), the one
way work starts and the sidebar's only button. **Close Window** `⌘W` is the
platform's `close` role: on a Mac the close policy already hides the window to
the Dock, the way the red light does, and today the keystroke does nothing
because no menu carries it (Q2).

**Help.** The `help` role submenu, which is what gives a Mac Help menu its
search field. Three items: **Verkstead Help** opens the docs site
(`https://tobico.github.io/verkstead/`) in the browser; **View Logs** is the
tray item's own act reached a third way — the same call the tray and the
Desktop page already share, never a copy of it; **Report an Issue** opens
`https://github.com/tobico/verkstead/issues` in the browser (Q3). Links leave
the app the way links off the workbench do, handed to the browser and not
waited on.

**Title Case in the menu bar** (Q5): *New Conversation*, *View Logs*, *Report an
Issue* — the platform's own casing, not the viewer's sentence case.

**The template stays a value.** The menu module that vitest runs grows items
that *act* beside the items that are roles: an item carries an id, a label and
an accelerator rather than a handler, exactly as the tray's menu carries ids
that the edge maps to acts in a record — so an item added without an act is a
compile error at the edge, and a test can read every label, id and key off the
template. The one file that imports `electron` to build the menu is where ids
become clicks. Accelerators are written in Electron's spelling (`Cmd+,`,
`Cmd+N`); the menu is a Mac's, so `CommandOrControl` has nothing to say.

**The acts that go somewhere use the window's own `visit`** — the same channel
a clicked notification's path crosses on, so the page routes there in place
with nothing reloaded and nothing half typed into a composer lost, and a window
with no page listening yet is loaded on the path instead. The menu is set before
the window exists, on purpose (so the window is never briefly one whose keys do
nothing), so an act reads the window at the press rather than holding one at
the build — the tray's Open does the same through what `main.ts` keeps.

## Acceptance criteria

- [ ] On a Mac, ⌘, opens the settings page and ⌘N the compose page, each by
      routing in place: a brief half typed on the compose page survives ⌘,
      and Back returns to it.
- [ ] ⌘W hides the window to the Dock, and the Dock tile brings it back.
- [ ] Verkstead Help and Report an Issue open in the browser; View Logs opens
      the same log file the tray item does, and says the same when there is
      none.
- [ ] The roles test pins the Mac template: every label, id and accelerator
      above, the application menu's items in that order, and `appMenu` no
      longer among the roles — while the hidden-bar platforms' templates are
      unchanged and the test that reads them is untouched.
- [ ] `pnpm lint`, `pnpm typecheck` and `pnpm test` pass in `desktop/`, with
      the new acting-item mapping on the edge side of the lint wall and
      nothing new importing `electron`.
