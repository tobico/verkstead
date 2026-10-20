# Mac menu bar

The Mac app's menu bar is the platform's furniture and nothing of Verkstead's:
the application menu, Edit, View and Window, every item an Electron role. There
is no Settings… on ⌘, and no File menu, so ⌘W does nothing, and nothing in the
bar goes anywhere in the workbench. Now it is filled out, on a Mac only:

- **Verkstead** — About · Settings… `⌘,` · Services · Hide · Hide Others · Show All · Quit
- **File** — New Conversation `⌘N` · Close Window `⌘W`
- **Edit**, **View** — as they are
- **Go** — Back `⌘[` · Forward `⌘]` · Conversations · Timeline `⌘1` · Code `⌘2` · Backlog `⌘3` · Roadmap `⌘4` · Share `⌘5`
- **Window** — the platform's own
- **Help** — Verkstead Help · View Logs · Report an Issue

Items that go somewhere send a path to the page over the channel a clicked
notification already uses, so the page routes there in place. For the Go menu
the page tells the app what the open conversation can go to, each pane with its
path, the way it already tells it the head's colours; what it does not name is
greyed. Title Case throughout, the Windows and Linux menus byte for byte what
they were, and ADR-0020 amended. Settled in the grilling of 2026-10-20, Sets
1281, 1283, 1284 and 1286.

## Tasks

- [x] 01: The menus the page needs nothing for — [details](01-the-menus-the-page-needs-nothing-for.md)
- [x] 02: The page says what is open — [details](02-the-page-says-what-is-open.md)
- [x] 03: The Go menu — [details](03-the-go-menu.md)
- [x] 04: The words — [details](04-the-words.md)
