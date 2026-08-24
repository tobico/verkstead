# 06. Base commit as a branch dropdown

## What to build

The Base commit text field and Record button become a dropdown of the repo's
branches. End to end:

- **A new endpoint lists a repo's branches** — local and remote-tracking
  both (the existing `for-each-ref` helper is the natural base). No endpoint
  lists refs today.
- **The dropdown** replaces the field-and-button. Its first entry is the
  existing default rule — the repo's default branch as it stands at grill
  start, which is the stored-null case — followed by the branches. There is
  no free-text escape: arbitrary shas and tags were deliberately dropped.
- **Semantics change with it.** Today the server resolves what was typed to
  a full sha at record time. A picked branch is now stored *by name* and
  resolved when grilling starts, so the work branches from where that branch
  is then. Recording validates the ref exists; grill start keeps its
  existing refusal when the stored base no longer resolves.
- The explanatory note under the control keeps saying which rule applies —
  the default rule, or pinned to a named branch.
- Adopting conversations show the same control they do today, dropdown
  included.

## Acceptance criteria

- [ ] The dropdown offers the default rule first, then every local and
      remote-tracking branch of the repo
- [ ] Picking a branch stores its name; grilling started later branches from
      that branch's position at that moment
- [ ] Picking the first entry restores the default rule (stored null)
- [ ] The free-text field and Record button are gone
