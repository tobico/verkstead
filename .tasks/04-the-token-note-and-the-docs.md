# 04. The token note and the docs

## What to build

**The token field says what a token has to be able to do**, in both places one is
asked for: the wizard's git step, and the settings page's GitHub token pane.

- A **classic** token wants `repo` and `workflow`, and `gist` for shares.
- A **fine-grained** one wants Contents, Pull requests, Issues and Workflows to
  write, and Actions to read.

**Why it is worth saying at all.** The push a session makes runs inside the
sandbox with this token, and a push GitHub refuses for a scope comes back as a
puzzle the session has to solve with a refusal that names no scope — the
reporter spent the time. The push stays the session's to report, because it is
the session's to make; the note is the only half that is Verkstead's, and saying
it once at the moment the token is pasted is cheaper than every session
diagnosing it afterwards.

**Beside the field rather than in place of what is already there.** Each of the
two already draws a line about what GitHub said of the token that was *saved* —
the account it authenticates as, and the scope it turns out not to have. That is
a different thing to do something about, and it only appears after a save. This
one is what to tick before pasting one, and it stands whether or not anything
has been saved. The words are said in one place and drawn by both, the way two
places drawing one sentence have to be.

**And the adoption doc's Windows section carries the Issue 5 note.** A per-user
install registered its uninstall entry under `HKLM` rather than under the
human's own `HKCU`, which is not where the section says it lands. It is parked
rather than fixed and the note says so with the reason: finding the cause wants
a Windows machine, and the package is already per-user — so what it costs is an
entry in the wrong list rather than an install that touched the machine.

## Acceptance criteria

- [ ] The wizard's git step and the settings page's GitHub token pane each name
      the classic scopes and the fine-grained permissions, worded in one place
      and drawn by both.
- [ ] The note stands whether or not a token has been saved, and the existing
      line about what GitHub said of a saved one is unchanged.
- [ ] The adoption doc's Windows section carries the uninstall-entry note beside
      what it says about where the install lands, saying it is parked and why.
