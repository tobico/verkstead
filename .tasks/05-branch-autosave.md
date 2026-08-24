# 05. Autosave the branch, drop the saved indicator

## What to build

Two quietings of the drafting Brief card, settled together:

1. **The brief's autosave indicator goes entirely.** The Saving… / Not saved
   yet / Saved line beside the Brief heading is distracting; autosave itself
   is unchanged, and the frozen-brief refusal message is separate and stays.
   Update the tests that read the indicator.
2. **The branch field autosaves and its Rename button goes.** Save the way
   the brief saves: a settle delay after typing pauses (the brief's 800ms
   constant) and a save on leaving the field, single-flight, re-saving when
   a save lands stale. The name is validated server-side only, so a save of
   a half-typed name can come back refused — keep showing the existing
   refusal copy; it was accepted that a refusal may flash mid-typing and
   clear on the next save. No new indicator for the branch either.

## Acceptance criteria

- [ ] No autosave indicator renders on the Brief card in any save state
- [ ] The branch field has no button; a typed name is saved after the settle
      delay and on blur, and the sidebar picks up the new name
- [ ] An invalid name shows the existing refusal text and self-heals once
      the name is valid
