# 06. COSMIC and the late panel

## What to build

Nothing new — this is the run that discharges the risk ADR-0020 accepted. The
Linux tray is Electron's own, Chromium speaks StatusNotifierItem itself and
watches for the panel's name, and none of that is verifiable until it is built
(Set 847 Q13). So: the app on a COSMIC session with every item activated, and the
app started before the panel to see whether the icon arrives when the panel does.

**What is already known, so the run is not spent rediscovering it.** The pinned
Electron constructs the tray with no StatusNotifierWatcher on the session bus,
takes a context menu and is not destroyed — probed while this stage was planned.
So what is being proven is the icon *appearing* and the items *working*, not the
app surviving a desktop without a panel.

**And a click is the panel's choice of gesture.** The specification does not say
what causes an activation: some desktops send it on a left click, some on a double
click, and some only open the menu. So what COSMIC actually does with a left click
is written down rather than asserted — **Open** being the first item on the menu is
what honours the decision either way.

**What fails here is written into the brief for stage 05 rather than fixed here.**
That is the stage that takes the Linux release leg and the words that go with it,
and **Show tray icon** off is the way out on a desktop that misbehaves. A finding
that lives only in a session's transcript is a finding the next stage does not
have.

## Acceptance criteria

- [ ] **Open**, **View Logs** and **Quit** each do what they say from a COSMIC
      panel, and what COSMIC sends for a left click on the icon is written down.
- [ ] An app started before the panel either gets its icon when the panel arrives,
      or the failure is written into the brief for stage 05.
- [ ] The findings are recorded where the next stage reads them rather than only in
      this session.
