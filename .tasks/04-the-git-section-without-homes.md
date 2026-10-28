# 04. The Git section without homes

## What to build

The Git section, the pickers and every Author edit on the record model, with
the home device gone from code and screen.

**One list, no device marks**: `AuthorEntry` loses its `device`, the switcher
and every Author picker (Open repo, Create repo, the Repos pane's repoint) lose
*at home on* and the mark, and nothing in the viewer or the server's words
names an Author's home. Two Authors with one name and email stay two rows,
drawn as such.

**Add, edit and Remove are local writes** on whichever device is pressed, and
token verification against GitHub runs there, with that device's `gh`. The
relayed path goes: `mirroring::authors::edited`/`removed`, `Pressed::Away` for
Authors, the home lookup, and the words refusing an edit "at home". Remove
keeps today's single-device refusal for this device's Repos; task 05 widens it.

Read `docs/copy.md` before changing any string. Rewrite the **Author** term in
`CONTEXT.md` in full — the record held by every device, newest write wins, a
removal final, out of `config.yaml`, the token copied to every device — and
drop its *Becoming the cluster's* pointer; amend ADR-0025's home and
`config.yaml` passages with a pointer to ADR-0028.

## Acceptance criteria

- [ ] Editing an Author's email on B redraws it on A off the pulled record, with no device mark on either.
- [ ] Setting a token on B is verified on B and the result shown there.
- [ ] No route, type or string still speaks of an Author's home device; the relayed edit and remove are gone.
- [ ] The **Author** term in `CONTEXT.md` is rewritten and its pointer gone.
