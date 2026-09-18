# 03. The docs

## What to build

The written record of what the two tasks before this one landed: the term in
`CONTEXT.md`, and the adoption doc saying where a human's global `CLAUDE.md`
went and what takes its place.

`CONTEXT.md` gains the term for the text itself, and the Sandbox and Built Root
entries name the file each harness's root is given. Written in the project's
own voice, with the `_Avoid_` line the other terms carry.

The adoption doc says in three places that none of the human's global
instructions are in a Built Root — the Linux section, the Mac's and the
Windows one. Each of those is now half the story: the human's file does not
travel, *and* the settings text is written in its place. The doc should also
say where the text is set, that it is one text for the whole installation, and
that it applies to the next session rather than to a running one.

One correction beyond the two documents: the skills module's own prose says the
sandbox has no global `CLAUDE.md` to say what a session is for, and reasons
from it about why the prompt names the skill by path. A root may now have one —
holding the human's text, never Verkstead's instruction — so that sentence is
no longer true as written, and the reasoning it carries still holds for a
different reason. Correct it rather than delete it.

Nothing behavioural in this task: the two tasks before it are what a reader of
these documents is being told about.

## Acceptance criteria

- [ ] `CONTEXT.md` carries the term, and the Sandbox and Built Root entries
      name the file each of the four harnesses' roots is given.
- [ ] The adoption doc's three *none of your global instructions* passages say
      what takes their place, where it is set, and when a change to it applies.
- [ ] The skills module's claim that a sandbox has no global `CLAUDE.md` is
      corrected without losing the reason a prompt names its skill by path.
