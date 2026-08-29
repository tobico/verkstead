# 01. Share file download

## What to build

A **Share** action on every Conversation, in any state, that downloads one
self-contained HTML file: a **share build** of the SPA that boots from a JSON
bundle inlined into the file instead of fetching from `/api/ui/`, drawn as the
two-pane workbench (timeline on one side, details pane on the other).

The server composes the bundle from the same rendered types the live viewer is
served — this task needs the Conversation view with its timeline; later tasks
add the Set and commit payloads to the same bundle. Which events board is a
fixed rule, decided during grilling:

- **In:** Brief, Question Set cards, commit cards, Steer, the Moved lifecycle
  lines, and legacy ManualTask events.
- **Out, silently:** agent output, Notices, handoffs, Unreadable Sets, and the
  pinned cards (pull request, task list, stage list). No placeholder marks the
  gap — the share is a curated record.

The share build is read-only everywhere: no action buttons, no answer forms,
no Start/Steer/Stop, nothing that talks to a server. It never loads xterm.
Every asset — CSS, the SVG icon definitions, the JS — is inlined; fonts stay
the system font stack, so nothing is fetched from anywhere. Opening a Set or
commit card may show a stub pane in this task; tasks 02 and 03 fill them.

Name the downloaded file for the Conversation's branch and the export date.

## Acceptance criteria

- [ ] The downloaded file opens from local disk with the server stopped and no
      network, and draws the two-pane timeline.
- [ ] Only the included event kinds appear; excluded kinds leave no trace, and
      no actionable control renders anywhere.
- [ ] The file is one `.html` with no external requests (verifiable from the
      browser's network panel), and the live workbench is unaffected.
