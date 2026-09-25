# 06. The Devices section

## What to build

The **Remote access** pane gains a **Devices** section, a third one beside the
two it already has, whose list holds this device alone: its name with an OS icon,
*this device*, its addresses, and no Unlink — there is nothing to unlink from yet.

**A section inside the pane rather than a settings section of its own**
(ADR-0020, *Discovery*): linking is how this machine is reached as much as the
serve and the key are, which is where the Brief put it. So there is no new word in
the settings' own list of openings, no card and no route of its own — what the pane
grows is a section and a reading. A section of its own was considered, the pane
being long already and Devices bringing a list, an Add, a Discovered list and a
pending row in the stages after this one, and was not taken.

**It reads off the machine, not out of the settings.** The two sections already
there read nothing of the settings query the rest of the page shares, because
nothing about them is configured — and neither is this: what it draws is the
identity endpoint's own answer about this device. Follow how the pane's existing
reading is set up, freshness and all.

**Where it stands matters.** The pane's `reach` section is drawn only inside the
branch where Tailscale is up and serving, while the `key` section stands under all
four Tailscale states. Devices belongs with `key`, outside that choice: a machine
with no Tailscale at all still has a device identity, and a Devices list that
vanished on such a machine would be a cluster feature that appeared to need
Tailscale.

**The Remote access card's line says how many devices are linked**, beside what it
already says. That line is a choice over the four Tailscale states with three
sub-states for *Up*, so the clause has to read right after every one of them
rather than being appended to one.

**The OS icon comes from Font Awesome's brand set**, which is not installed yet —
add `@fortawesome/free-brands-svg-icons` and draw the Linux, Apple and Windows
icons through the same component every other icon in the app is drawn through, so
the bundle carries only the icons that are imported. A WSL reads *Linux (WSL)*
with the Linux icon, as the machine reading already says it.

## Acceptance criteria

- [ ] The Devices section stands in the Remote access pane beside the other two,
      with no new settings word, card or route, and it draws on a machine with no
      Tailscale at all.
- [ ] The list holds one row — this device — carrying the OS icon, the name,
      *this device* and the addresses, and offering no Unlink.
- [ ] The Remote access card's line says how many devices are linked, and reads
      correctly in each of the Tailscale states it already covers.
- [ ] The section reads its own answer rather than anything of the settings query,
      and a WSL draws the Linux icon with *Linux (WSL)* beside it.
