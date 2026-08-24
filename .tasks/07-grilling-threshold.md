# 07. Redraw the grilling threshold

## What to build

Two changes to how a drafting conversation reads before grilling:

1. **The two agent-profile pickers sit side by side** where the width
   allows, stacked as today on narrow screens.
2. **Start grilling is always drawn.** Today an unready conversation gets a
   note instead of the button. Now the button always renders; when
   `ready_to_grill` is false it *looks* disabled but stays interactive
   (`aria-disabled`, not `disabled`), and activating it — tap, click or
   keyboard — shows the explanation of what is missing beneath the button
   instead of starting. Show the same text on hover for pointer users. This
   was settled deliberately against a native `title` on a truly disabled
   button, because the human mostly uses a phone, where neither reaches
   them.

Both existing not-ready messages go: the "Not ready to grill…" verdict line
on the brief card and the "Write the brief and choose both agent profiles…"
note that stood in for the button. The "Ready to grill." affirmation and the
adopting-conversation note keep their current behaviour.

## Acceptance criteria

- [ ] Both pickers share a row on wide screens and stack on narrow ones
- [ ] The unready button renders visibly inert, never starts grilling, and
      tapping it reveals what is missing
- [ ] A ready conversation's button starts grilling exactly as today
- [ ] Neither old not-ready message renders anywhere
