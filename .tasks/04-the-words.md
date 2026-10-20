# 04. The words

## What to build

The documents brought up to what the three tasks before this built. Today
ADR-0020 says a Mac keeps the application menu it cannot do without, and
nothing anywhere names what is in it.

**ADR-0020 amended** (Set 1281 Q6), as its earlier amendments are — a section
headed *Amended:* at the foot, in the documents' own voice — naming the Mac
menu bar as it now stands: the six menus and their items with their keys; that
items which go somewhere send a path to the page over the channel a clicked
notification uses, so nothing reloads; that the Go menu is enabled by what the
page pushes about the open conversation, the way the head's colours are
pushed, and greyed where the page names nothing; that Back and Forward are the
window's history on ⌘[ and ⌘] with the editor caveat as task 03 left it; Title
Case because the bar is the platform's; and that Windows and Linux keep their
hidden bars exactly as they were. Cite the Sets — 1281, 1283, 1284 — the way
the ADR cites the ones before them.

**The vocabulary.** `CONTEXT.md`'s Desktop and Window Decorations entries
describe the tray menu and the Mac's traffic lights and say nothing of the menu
bar; add the sentence or two that names it, using the words above and none
under *Avoid* — the item is *Settings…*, never *preferences*.

**Anything else that still says otherwise.** `docs/development.md` and the
Electron roadmap's stage documents mention the menu bar and its shortcuts;
read each mention and change only a line that is now untrue. The roadmap's
stage documents are a record of what was decided then and are not rewritten.

## Acceptance criteria

- [ ] ADR-0020 carries an amendment naming every menu and item above, with
      the mechanism and the Sets, and no sentence in it still says the Mac
      menu is only what the platform fills in.
- [ ] `CONTEXT.md` names the Mac menu bar under the Desktop entry, in its own
      words and none it lists under *Avoid*.
- [ ] No document under `docs/` other than the roadmap's stage records
      describes the Mac menu as it was before task 01.
