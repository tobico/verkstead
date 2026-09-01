# 02. Sessions are honestly absent on Windows

## What to build

One fact, said once: **this build runs no sessions**. The API answers it, the
viewer draws it where a session would start, and pressing the button that would
start one is met with words rather than a spawn that fails halfway down.

**The words say Windows, and they say not yet.** ADR-0012 planned this as
"sessions need Linux"; stage 04 ported the Sandbox to macOS, so sessions run
there and Windows is the only platform without them. A later stage brings them
to Windows too, so what the human reads is a platform that has not got them yet
rather than a product that will never have them.

**The fact is a value, not a `cfg`.** The codebase already does this for the
platform directories and for the sandbox surface: the arm a machine will never
run is still an arm its tests call. So the state is testable on the Linux runner
and the viewer's half is testable without a Windows machine anywhere.

**Every way in refuses the same way.** Starting a grilling is the obvious one,
but a Conversation is also resumed, followed up, steered, rescued and adopted,
and each of those wants a session. They give the human the one answer rather
than each inventing wording of its own — and where the viewer can tell before
the press, it says so on the page instead of offering a button that will be
refused.

Linux and macOS answer exactly as they do today. Nothing about this task is
visible on either.

## Acceptance criteria

- [ ] On Windows the viewer draws the state where a session would start, and
      offers no press that would start one; vitest covers what it draws
- [ ] The API refuses to start a session on Windows with an outcome that names
      Windows and reads as *not yet*, and every other way into a session gives
      the same answer; unit tests cover the refusal and the generated wire
      types are committed
- [ ] Linux and macOS are untouched: the sessions suite and both sandbox
      suites pass as they do today
