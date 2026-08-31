# 03. The historical sites

## What to build

The three places that say what a session *ran* under adopt the shared reading
from task 02, fed by the record task 01 writes: the Agent run timeline card's
head (which today says only "Agent run"), the matching details pane header, and
the status button's running-agent line (which today reads "Work Fable 5",
profile-first — that convention and the module commentary defending it go).

These sites read the recorded facts — profile name, model id, and now agent
type — never the Conversation's current pairing. The rules where history and
the present disagree, settled in the grilling:

- **A recorded harness** composes the full reading: mark position reserved for
  task 04, text "Claude Code Fable 5 — Work".
- **No recorded harness** (events from before task 01): the reading minus the
  harness word — "Fable 5 — Work" — with the profile suffix always shown, since
  without a harness there is nothing to count against.
- **The suffix on recorded-harness events** hides only when the recorded
  profile name is *itself* the sole saved profile of that harness today. A
  recorded name that no longer matches any profile, or matches one of several,
  keeps its suffix — hiding it would misattribute the run to whatever profile
  remains.
- Nothing recorded at all keeps today's fallbacks ("Agent running" on the
  status line; the card and pane still render).

The profile count comes from the same client-side profiles query the pickers
already use — no new server field.

## Acceptance criteria

- [ ] A run with a recorded harness reads "Claude Code Fable 5 — Work" style on
      the timeline card head, the details pane header, and the status button's
      running line.
- [ ] A pre-change event reads "Fable 5 — Work" style with no harness word; a
      session with nothing recorded still shows the existing fallbacks.
- [ ] The suffix hides exactly when the recorded name is today's sole profile
      of that harness, and shows in every other case — renamed and deleted
      profiles included.
