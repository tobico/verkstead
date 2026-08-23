# 01. Profile model lists

## What to build

A Profile stops carrying one model and starts carrying a list of the models it
can run. The list is the profile's own — different profiles reach different
accounts and configs, so each names what it can actually launch. This was
settled against two rejected alternatives: a global model list shared by all
profiles (claims every model for every account), and a hardcoded list of known
Claude models (goes stale as models ship).

The Settings form takes the models as free text, one per line, where the
single "Default model" field is today. A profile with no models is refused on
save, exactly as a modelless profile is refused today. Profile rows in
Settings show the whole list. There is no default or preferred model and no
meaningful order — the list only says what is available; every later pick is
explicit (settled in grilling: no first-entry default).

Existing profiles carry over: whatever single model a profile held becomes the
sole entry of its list, with nothing for the human to re-enter. The store has
no migration machinery by design — its convention is to hang a new table off
an existing one rather than alter a STRICT table, and the model list should
land that way.

This task ends at the profile: what a profile's list *feeds* (the pairing
pickers, session launch) is task 02, and until then the rest of the system may
keep reading a profile's first-or-only model as it read `model` before.

## Acceptance criteria

- [ ] The profile form edits models one per line; saving with none is refused
      with the existing modelless-style refusal
- [ ] A profile saved before this change shows its old model as the single
      entry of its list, in the form and in the Settings row
- [ ] Settings rows display every model a profile lists
