# Client-side decorations

The window loses its title bar. On Windows and Linux the platform's controls
overlay stands at the top-right corner in the heads' own colours, light and
dark, recoloured the moment the theme flips; on a Mac the traffic lights are
inset to the left of the Wordmark. Every pane head is the drag region with its
own controls excepted, so the window moves by whatever bar is at the top of it,
and the frame — not each pane — keeps its outermost heads clear of the
controls, reading the overlay's rectangle, in three panes and in one. A page
with no pane head — onboarding, the no-such-page, and the moment before the
onboarding verdict lands — gets a bare drag bar of the same height, drawn only
inside the app.

Nothing about any of it reaches a browser: a drag region is inert outside the
app, every inset is zero where there is no overlay, and the colours are pushed
over the preload bridge that a browser does not have. The traffic-light inset
is written here and proven in stage 06, and the Windows overlay is the same
code as Linux's and proven in 07. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md).

Roadmap stage: [04: Client-side decorations](docs/roadmaps/electron-desktop/04-decorations.md)

## Tasks

- [x] 01: Frameless, with the overlay on — [details](01-frameless-with-the-overlay.md)
- [x] 02: Every pane head drags the window — [details](02-the-heads-drag-the-window.md)
- [x] 03: The frame keeps the outermost heads clear — [details](03-insets-from-the-frame.md)
- [ ] 04: The head's colours and height over the bridge — [details](04-colours-over-the-bridge.md)
- [ ] 05: The bare drag bar — [details](05-the-bare-drag-bar.md)
- [ ] 06: COSMIC, and a window a compositor would rather decorate — [details](06-cosmic-and-the-overlay.md)
- [ ] 07: The words — [details](07-the-words.md)
