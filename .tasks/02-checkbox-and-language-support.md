# 02. A plain checkbox with a nested group, proven on Language support

## What to build

Every on/off control on the settings page becomes a native, unpainted checkbox
in a label, and every piece of configuration that only means something while
its checkbox is on sits indented under it, greyed and refusing input while it is
off. Build that pattern once and prove it on the build cache section, which is
the first of four to take it.

The checkbox sits beside the painted switch component rather than replacing it,
and keeps the switch's one rule: it shows where things stand rather than where
somebody pressed. A tick is a request handed up as the state it would become,
the box goes straight back, and only the caller's answer arriving moves it.
Ticking saves at once, as the switches do today.

The nested group is the pattern's other half. Its fields and its Save button
carry the native disabled attribute while the group is off, the whole group is
indented from the left to read as belonging to the checkbox above it, and the
text greys with it. Native disabled rather than readonly: the browser greys it,
takes it out of the tab order and refuses input for nothing.

Then the section. Rust build cache becomes **Language support**, at
`/settings/languages`, with the old slug no such page rather than a redirect,
the way the earlier fold retired routes. The card's summary is the list of
languages with build support on, which today is the word Rust when the cache is
on and nothing at all under the heading when it is off. The pane is one checkbox
labelled **Rust**, with the size field and its Save nested under it. The group
is off while the checkbox is off and also while sccache is missing, since the
compiled half cannot be cached then; the field is shown disabled rather than
hidden as it is today. The sccache warning stays on both card and pane. The
paragraph about sharing one cache between sessions goes.

Tests: the build cache web tests follow the rename and cover the greyed group in
both of its off states, and the routes test learns the new slug and the retired
one. The design doc's settings block names the cache's card and pane.

## Acceptance criteria

- [ ] With the cache on the card reads Rust. With it off the card has no summary line under its heading.
- [ ] Ticking and unticking the Rust checkbox saves at once, and the size group greys and refuses input while the checkbox is off or while sccache is missing.
- [ ] `/settings/languages` opens the pane and `/settings/build-cache` is no such page.
