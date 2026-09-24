# 05. The pane's menu, and its three settings

## What to build

**The three settings ADR-0019 exposes** — word wrap, font size and minimap —
kept on the device beside the Diff's own wrap setting, and read by every editor
Code mounts. Per device and never sent to the server, for the reason the wrap
beside them is: a phone and a laptop are entitled to draw the same file
differently, and neither has any business deciding for the other. Storage is a
convenience the whole way down, so a browser that refuses it costs the setting
and nothing else — the editors draw VS Code's defaults, which is what an
untouched browser already gets.

**Drawn as rows on the pane's own menu**, which Code's header does not carry
yet: the ⋯ menu the app has, beside the maximise toggle, holding the three of
them. The place a pane keeps what is about the pane, rather than a settings
page somewhere else that would then be about one pane of one Conversation.

**And every editor follows them at once.** A change reaches every editor open
in every group, live, the way the light and dark themes already do — not the
active one, not the next one opened. Which means the settings are read where
the editors can all see them rather than held inside any one of them.

## Acceptance criteria

- [ ] A change to any of the three applies at once to every editor open in
      every group, with no tab reopened.
- [ ] A reload comes back to what was set, and a browser with no storage draws
      VS Code's defaults rather than failing.
- [ ] The three are rows on the pane's own menu in Code's header, and nothing
      about them is sent to the server.
