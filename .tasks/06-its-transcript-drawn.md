# 06. Its Transcript drawn

## What to build

Draw a plugin harness's session on the Timeline and in the Transcript pane out
of its component's `parse-transcript` export.

The export owns **discovery as well as parsing** (stage brief): it is handed
the account's root and the session id, and answers with the kinds the
Transcript pane draws — said, tool call, result, system notice and unknown. An
unknown kind folds away under its own name, as ADR-0006 has always had it. That
rule is the host's.

Wire the export into the places the five built-ins' transcripts reach today:

- the server's transcript following and search
- carrying a prior session's log into a resume
- the Timeline's session output

Then hand what the export answered to the render crate's drawing alongside the
built-ins' parsed lines. The render crate's per-format kind lists stay for the
five built-ins, until each crosses.

## Acceptance criteria

- [ ] A stub plugin session's said, tool-call and result lines are drawn on its
      Transcript, live while it runs and after it ends.
- [ ] A kind the export marks unknown folds away under its own name.
- [ ] An export that traps, or answers malformed, leaves the Transcript empty
      with that said. The session itself is not affected.
