# 02. Browse dropdown on the Paths section

## What to build

The shared path-field component, and its first adoption. It **extends** the
plain text input — the input, its label, and the Add press stay exactly what
they are, and Add submits whatever the field holds whether it was typed or
tapped together.

The dropdown is a **drill-in browser, one directory per level** (the drill-in
precedent is `Menu`'s nested rows; the combobox roles, keyboard handling and
drop-direction measuring to follow are `Listbox`'s). The settled interaction,
decision by decision:

- **Typing steers it.** The dropdown shows the entries of the deepest existing
  directory the typed text names, fetched from task 01's endpoint; the trailing
  partial segment filters the rows shown.
- **A tap both writes and opens.** Tapping a directory row puts that path in
  the field *and* drills into it; a back row at the top goes up a level and
  rewrites the field the same way. There is no separate pick affordance.
- **The human closes it.** Backdrop or Escape dismisses the dropdown and the
  field keeps whatever it holds — closing is how a browse ends, not picking.

Adopt it in the shared adding field the settings page's Paths section and the
Repo pane's Sandbox Configuration both draw, so the watched-path field, the
global binds and the per-repo binds all browse in one move — all three in the
**anywhere** scope (their values are not required to sit inside the Watched
Paths), showing directories only, dotfiles hidden.

That field's module header states the old "typed rather than picked" stance;
rewrite it to say what is now true — browsing is offered, and what the server
makes of the submitted path still decides everything.

## Acceptance criteria

- [ ] Typing moves the dropdown to the directory the text names and filters its
      rows; tapping a row deepens both the field and the listing; the back row
      shallows both.
- [ ] Backdrop and Escape close the dropdown leaving the field as it stands,
      and Add submits it unchanged — saves go over the wire exactly as before.
- [ ] All three Paths-section fields browse, with vitest coverage driving the
      dropdown the way the existing picker tests drive `Listbox` (by role and
      visible words), and the rewritten module header lands.
