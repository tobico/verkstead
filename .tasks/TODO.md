# Cleanup of archived conversations

Verkstead keeps everything forever; this adds the one part of the record that
lets go. A periodic Cleanup sweep trims an archived Conversation's bulk — the
full agent output, transcripts and session records a Share never included — a
set number of days after it was archived, and can permanently delete the whole
Conversation later. Each cleanup runs on its own clock from `archived_at` and
gets a switch and a days field in a new Cleanup settings section: trim defaults
to 3 days on, delete to 30 days off.

The grilling settled the shape: trim keeps every Timeline card and marks the
Conversation Trimmed; delete walks every table the Conversation owns in
dependency order and never touches the git branch or a published share;
existing archives are cleaned on the first sweep after this ships; archiving
stays confirmation-free; the sweep reports to the log and nowhere else, with
no advance notice drawn in the workbench.

## Tasks

- [x] 01: Trim, swept — [details](01-trim-swept.md)
- [x] 02: The Trimmed mark — [details](02-trimmed-mark.md)
- [x] 03: The Cleanup settings card — [details](03-cleanup-settings-card.md)
- [x] 04: Delete, swept — [details](04-delete-swept.md)
