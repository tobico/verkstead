# 02. Configure the companion

## What to build

The companion row becomes a configuration: where the companion comes off,
whether the session may write to it, and — where it may — what its branch is
called.

**A base picker per companion**, the same shape as the main one and offering the
same first entry: the rule, meaning that repo's default branch as it stands when
grilling starts, and then that repo's own branches. It stores a name rather than
a commit, resolved when grilling starts, because what the human is picking is a
line of work to come off rather than a moment to pin to. The branch list is the
companion repo's own and is read under that Repo's key, not the Conversation's —
two Conversations against one Repo are looking at the same list.

**A mode switch**, read-only or read-write. Flipping to read-write reveals the
field below.

**A branch-name field for read-write, mirroring until it is typed in.** Stored
empty means *mirroring*: the companion's branch is whatever the main branch is
called, so renaming the main branch renames the companion's with it. The field
is drawn prefilled with the main branch name so the human sees what they will
get, and the first thing typed into it makes it a name of its own that no longer
follows. Clearing it back to empty is going back to mirroring.

    branch_name = ""      → mirrors the Conversation's branch, now and later
    branch_name = "foo"   → is "foo", whatever the main branch is renamed to

**Autosave matching the branch field's** — on a pause in the typing, on the way
out of the field, and on Enter — because this is the same card and a Save button
here would be the one thing on it asking to be pressed. A refusal stands until
the next save answers it rather than clearing on the next keystroke, and the
same string is never asked about twice. The pickers and the switch save on the
change, as the base picker and the pairing pickers already do.

**Every edit is refused once the Conversation is no longer drafting**, with the
same named refusals the branch and base fields get. Steer-time changes are a
later stage's; the rows are gone from the card by then anyway, so a refusal here
is a race rather than a route.

## Acceptance criteria

- [ ] The base picker offers the rule — that repo's own default branch, named —
      followed by that repo's branches, and stores a branch name rather than a
      commit.
- [ ] Flipping a companion to read-write reveals a branch-name field prefilled
      with the main branch name, and renaming the main branch moves the
      companion's with it until something is typed into that field.
- [ ] A typed companion branch name stands on its own and stops following;
      clearing it goes back to mirroring.
- [ ] Every edit saves the way the branch field does, and every one of them is
      refused by name once the Conversation is no longer drafting.
