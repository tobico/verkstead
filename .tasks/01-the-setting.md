# 01. The setting

## What to build

A text box on the settings page, saved as an `instructions` key in
`config.yaml` and read back through the settings API.

One text for the whole installation rather than one per Agent Profile: it sits
with the other things Verkstead is *told* rather than finds, beside the git
author and the sandbox binds, and it is read afresh at the moment a session
needs it like everything else in that file.

It is a value rather than an action, the way the build cache and the binds are:
what the page sends is what the file holds afterwards, so clearing the box
clears the key. Nothing about it can refuse a save — there is nothing in a
paragraph of prose to be wrong about — and an absent key, an absent file and
one nothing can parse all mean the same thing, which is an empty text.

It is its own card and its own details pane, drawn among the sections already
there, and the card's own words say what the text reaches: every session of
every Conversation, under whatever harness its Profile runs, above whatever
instructions the Repo itself carries. Nothing reaches a session yet — that is
task 02 — so what this task delivers is a text that saves, reads back, and says
what it is for.

## Acceptance criteria

- [ ] A text typed on the settings page is in `config.yaml` after the save, and
      comes back on the next read of the settings, verbatim and with its line
      breaks intact.
- [ ] A `config.yaml` with no `instructions` key, an empty one, and one that
      will not parse all read as an empty text; clearing the box writes the key
      away rather than writing an empty one; and no other key in the file —
      `session_path` included — is touched by a save.
- [ ] The card and its pane say what the text reaches and where it sits
      relative to a Repo's own instructions, and a fresh installation draws the
      section with an empty box rather than an error.
