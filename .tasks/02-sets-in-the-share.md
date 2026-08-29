# 02. Question Sets readable in the share

## What to build

Each Question Set in the shared file opens as the full sheet in the details
pane, exactly as the workbench draws a settled or locked Set: preface, the
questions with their options and recommendations, the recorded answers and the
set-level comment, the attached worktree diff blocks (they were decided in —
they are what the human approved against), and the postscript. Read-only
always — no answer form ever renders, whatever the Set's standing was at
export time.

The bundle grows a rendered Set payload per Question Set on the timeline,
reusing the server's existing Set rendering. The share reuses the live sheet
components rather than a second implementation, with the answering surface
suppressed.

Mermaid boards **only when needed**: the rendered payloads already say per
document whether a diagram exists, so a share with no diagrams carries no
mermaid bytes, and one with diagrams renders them offline from the inlined
library.

## Acceptance criteria

- [ ] A Set card opens to the complete sheet — preface, questions, answers,
      comment, diffs, postscript — with nothing pressable.
- [ ] A share containing a mermaid diagram renders it with no network; a share
      containing none is measurably free of the mermaid library.
- [ ] An answered, an unanswered-but-open, and a locked Set all draw as
      read-only records without flashing a form.
