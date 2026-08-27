# 12. Transcript autoscroll

## What to build

Opening the transcript of a **running** session scrolls to the bottom, and the
view follows as new content arrives. Scrolling up pauses the following;
returning to the bottom resumes it. A finished transcript opens at the top, as
today.

The transcript itself does not scroll — the details pane around it does — so
the following watches the pane. The pause must trigger only on the human's own
scrolling: content growing under a pinned-to-bottom view is not a scroll away
from it, so the distinction between the hand and the growth needs to be real
(the contents navigation elsewhere in the app already tells by-hand scrolling
from its own, and is the pattern to match).

## Acceptance criteria

- [ ] Opening a running transcript lands at the bottom and stays there as
      content arrives
- [ ] Scrolling up holds the position while content keeps arriving; scrolling
      back to the bottom resumes following
- [ ] A finished transcript opens at the top and never auto-scrolls
