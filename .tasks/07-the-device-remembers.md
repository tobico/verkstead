# 07. The device remembers

## What to build

**What is open survives a reload, not only a swap.** Stage 01 put Code's state
above the frame's switch, so opening an Event and coming back finds the tabs
where they were; this puts the same thing in the browser's storage, per
Conversation, so a reload does too. What is kept is the layout tree, each
group's tabs and which of them is showing, which group is active, and the
**dirty text** of every buffer that has some. Kept beside the pane widths and
the wrap setting, under the same `verkstead.*` namespace, and never sent to the
server: a device's layout is a device's, and a phone that opened Code has
nothing to say about a laptop's splits.

**Storage is a convenience the whole way down**, as `device.ts` says, and that
decides what happens when it fills. A dirty file may be two megabytes and the
browser's storage is a few, so a Conversation with two large unsaved files can
overrun it. **The layout is written apart from the text**, so that a storage
too full for the unsaved text still comes back to the splits and the tabs: the
text is what is lost, and never the layout. A stored body that will not parse,
or is not the shape of one of these, is dropped on the way past the way the
compose page's draft is.

**Two things are reconciled on the way back in.** Terminals are the server's:
the register is read on every opening, as it already is, and a tab whose shell
ended while the page was away is dropped from whichever group held it. And a
restored dirty buffer is dirty **against the version the file reads at now** —
the file is read on load like any other, the human's text goes over the top of
it, and if the disk moved while the page was away the comparison says so. That
is what puts the reload bar on a file the agent rewrote overnight, rather than
a save going out over a version that is no longer there.

## Acceptance criteria

- [ ] A reload comes back to the same splits, the same tabs, the same active
      tab and the same unsaved text.
- [ ] A terminal that ended while the page was away is not in the restored
      layout, and one opened on another device is.
- [ ] A dirty file whose disk contents moved while the page was away comes back
      dirty against what is there now, and its next save is refused with the
      bar.
- [ ] A storage that refuses the unsaved text still restores the layout, and a
      browser that refuses storage altogether costs the layout and nothing
      else.
