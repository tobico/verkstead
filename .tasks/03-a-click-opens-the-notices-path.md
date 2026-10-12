# 03. A click opens the notice's path

## What to build

A click on a notification lands where a tap on the phone lands: the Timeline
Event a Set is read on, the Conversation for everything that happened to one,
the Remote access pane for a device asking to join, and a member's Conversation
under that device's own segment. The app tells the page over a new channel on
the bridge — the first in the app-to-page direction that the page acts on,
`head` being the page telling the app — and the page routes there in place, so
nothing reloads and nothing drafted is lost. Where the window's page is not
there to be told, the window is loaded on the path instead.

The preload exposes a way for the page to listen, in the shape both sides of
the bridge already read; the page reads it off the window the way it reads the
rest, and a page with no bridge has nothing to listen to.

## Acceptance criteria

- [ ] A click on a Set's notification opens that Set's Timeline Event with no
      reload, and a click on a stop's opens its Conversation
- [ ] A click while the window is not yet on the workbench opens the window on
      the path
- [ ] The channel's name is pinned by the bridge tests on both sides, and the
      page's test drives the listener through the stubbed bridge
