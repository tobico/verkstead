# 06. The sign-out and usage-window reader preservation

## What to build

A carried-on session resumes with the harness's own resume, and a harness that
redraws the conversation it resumes may redraw an old sign-out or usage-limit line
onto the terminal. That line must not stop the run a second time.

The existing readers already protect against this and the job here is to keep it
true across a carry-on, and prove it: Claude's sign-out phrase is read in its log
alone (`ReadIn::Log`), followed from where the new session begins, and agy's needs
its live login-picker under it, which a valid-login resume does not redraw. The
carried session's sign-out watch (`signouts::Watch`) and usage-window watch
(`limits`) must begin their reading at the resumed log's / terminal's new start, so
a redrawn past falls outside what they read.

Check the frame and Capture readers for the same hazard, as the Brief asks — a
redrawn frame or a Capture holding replayed bytes must not re-trigger a stop.

## Acceptance criteria

- [ ] A session carried on after a sign-out, whose history holds the old sign-out line, does not stop the run again (tested).
- [ ] The usage-window reader is held to the same rule and is tested the same way.
- [ ] The frame and Capture readers are checked for a redrawn-history re-trigger, as the Brief asks.
