# 04. Directly editable brief

## What to build

While a Conversation is drafting, the brief is edited where it stands — no
Edit button, no swap between a rendered view and a form. The brief body *is*
a permanently visible auto-growing textarea showing raw markdown, and it
saves itself: on blur, and after a pause in typing, with a quiet saved
indicator so the human knows the record has it. (Settled against two
alternatives: rendered markdown that swaps to a textarea on tap, and an
explicit Save button appearing when dirty.)

Once grilling starts the brief freezes — the server already refuses edits
outside drafting — and from then on it renders as markdown, read-only. The
editor never appears again, matching how adopted conversations already show
a brief with no editor at all.

Autosave and the freeze meet at the edge: a save refused because grilling
started mid-edit should surface the existing refusal text rather than fail
silently.

## Acceptance criteria

- [ ] While drafting there is no Edit or Save control; the textarea is
      always present, grows with its content, and edits persist on blur and
      after a typing pause, with a visible saved indication
- [ ] After grilling starts the brief renders as markdown with no editor,
      and a save attempt that races the freeze shows the refusal
