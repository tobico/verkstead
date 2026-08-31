# Agent profile styling

Every place the workbench says who runs a session gets one shared reading: a
harness mark beside "harness + model name", with the profile's own name after an
em dash only where that harness has more than one profile — "Claude Code Fable 5
— Work", "Grok 4.6", "OpenCode Minimax M2.1". Today the three sites disagree
(the pickers say `opus — claude-opus-5`, the status button says `Work Fable 5`,
and the Agent run timeline card names nothing at all), no harness marks exist,
and the session record does not know which harness a run was launched under.

The grilling settled: the reading is composed (harness display name + prettified
model, collapsed where the model name already names the brand), the harness is
recorded at launch rather than derived, the marks come from the lobehub icon
set copied into the repo, and the pairing pickers become a custom listbox so
every row can carry its mark.

## Tasks

- [x] 01: Record the harness a session ran under — [details](01-record-harness.md)
- [x] 02: The shared reading — [details](02-shared-reading.md)
- [x] 03: The historical sites — [details](03-historical-sites.md)
- [x] 04: The harness marks — [details](04-harness-marks.md)
- [x] 05: The pairing listbox — [details](05-pairing-listbox.md)
