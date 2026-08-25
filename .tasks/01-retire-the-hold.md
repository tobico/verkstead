# 01. Retire the Hold

## What to build

The Hold goes, and nothing takes its place. Typing into a Screen commits
Verkstead to nothing — no register, no hand-back, no badge, and no clock a
keystroke puts back. Somebody who wants to intervene by hand presses **Stop**
first, and the stop is what holds the run off while they work; a session typed
into while the run is still driving it is ended and advanced by the ordinary
rules.

What goes with it:

- The Holds register itself, and the accessors the sessions register wraps it
  in. Ending a session no longer hands anything back.
- The gate every driver waits at before it ends a session or advances a run.
  It is awaited in **seven** places — six in the runner and one in the module
  that carries a Conversation on — and each of them acts at once instead.
- The hand-back endpoint, its outcome type, the field on the Conversation view
  that said which session the human had the keyboard of, and the hand-back
  control drawn on the Screen. The badge stops reading it.
- Telling a keystroke from a mouse report, which was the Hold's business
  alone: the Screen's socket carries one kind of input from a watcher, and
  what it is sent reaches the session's terminal unchanged.
- Its push. The news that a Conversation is waiting for a keyboard, the
  reminder that sends it once a Hold has stood a while, the delay that
  reminder waits out, and the pace knob for it have nothing left to announce,
  and the Screen is the one caller.

Leave `CONTEXT.md` alone: the vocabulary this stage retires is task 05's, and
that includes the Hold entry and the carve-outs the Timeline and Screen entries
carry for it.

## Acceptance criteria

- [ ] Typing into a driven session's Screen changes nothing about when the
      session ends or when the run advances. Pressing Stop first is what holds
      the run off.
- [ ] Nothing hands anything back anywhere — no endpoint, no button, no
      register — and no device is ever told a Conversation is waiting on a
      keyboard.
- [ ] The Screen's socket carries one kind of watcher input, and everything
      sent down it reaches the terminal straight through.
