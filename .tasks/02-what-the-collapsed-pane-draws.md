# 02. What the collapsed pane draws

## What to build

The collapsed pane as the brief pictures it, in place of the clipped content
task 01 left there. Every part keeps the height it has expanded — only the
horizontal sizing shrinks — and nothing in it acts while collapsed, a mouse
expanding it before any click can land.

**The head**: the logo alone. The word *Verkstead* and the gear are hidden;
the head's row stays the height it is, so the list below starts where it
starts expanded. The gear's open state on the settings page is simply not
shown while collapsed.

**The new-conversation link**: a `pen-to-square` icon alone, centred in the
2rem, labelled *New conversation* for a screen reader and carrying the same
words as a tooltip, as the gear's icon button does. The icon is Font Awesome's
`faPenToSquare` from the solid set the viewer already carries. **The expanded
link gains the same icon before its text** — the brief asks for it on the
full-size button too — so collapsing hides the words and keeps the icon.

**Each card**: the first three whole characters of its title (Q4), cut at a
character boundary with no ellipsis, so a *Draft* reads *Dra* in its italic.
Under them, on the line where the device and repo stand expanded, the card's
mark — the waiting disc, the turning ring or the idle ring — centred in the
2rem; a card with no mark leaves that line blank at its height. The code icon
that stands before an active card's title is hidden (Q5); the mark line is the
whole of the collapsed card's status. The selected card keeps its fill, and a
finished or unreachable card keeps its fade. The card's padding shrinks with
it so the three characters have the room; the row's height does not change.

**The foot**: the *Show archived* switch and the *Forwarding* item are hidden
and the foot stands as blank paper at the height it has expanded — including
where the forwarding item would wrap under the switch, which is why the
content is laid out at the expanded width and hidden rather than laid out at
3rem. The rule over its head stays.

**Nothing acts while collapsed.** With a mouse that is already true, the
hover having expanded the pane before a click arrives; with a keyboard, focus
expands it. Touch is task 03, which is what the hiding here must not get in
the way of: hidden parts take no press, but the pane as a whole still will.

The vocabulary for the state is *collapsed* and *expanded*, nothing else — the
glossary already lists *left rail* under Avoid for another term, and task 06
writes the entry. Keep the module docs of the sidebar, the wordmark, the
archived switch and the forwarding item true to what they draw when
collapsed.

## Acceptance criteria

- [ ] Collapsed, the head shows the logo and nothing else; the link shows the
      pen-to-square icon and reads *New conversation* to a screen reader;
      expanded, the link shows that icon before its text.
- [ ] Collapsed, a card titled *collapsible-conversations* reads *col* with no
      ellipsis, a *Draft* reads *Dra*, a waiting card shows the disc centred
      under its characters, and a card with no mark leaves the line blank.
- [ ] The head, the link, every card and the foot each measure the same height
      collapsed as expanded, with the forwarding item present and absent.
- [ ] The foot is blank paper while collapsed, and the switch and the
      forwarding item come back on expansion exactly as they were.
- [ ] The code icon is not drawn on a collapsed card, and reappears when the
      pane expands.
