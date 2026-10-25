# 03. Choosing

## What to build

The first step's checkbox screen as `01-choosing` and `dark-01-choosing`
draw it. The behaviour is today's: the sandbox and git start ticked, the
harnesses and gh unticked, WSL as its own rule says, and the press is
pressable once the present rows plus the ticked ones would be a sandbox, git
and one agent.

**Three groups** with an uppercase label and a quiet gloss each:
**Required** (*Every session needs both*) holding the sandbox and git as rows
in one card; **An agent** (*At least one*) holding the four harnesses as a 2x2
grid of tiles, each with the harness mark, the name, the command name in mono
and the box in the top right, the ticked tile taking an ember border and the
selected wash written as `color-mix(in srgb, var(--accent) 8%, var(--card))`
so dark follows; and **Optional** holding gh, and on Windows the WSL row
beside it (settled: Optional rather than Required, which is what its own
label already says). A refused WSL row keeps its reason in place of a box.

**Pills instead of details.** A present row carries no box, a green tick and a
green **Installed** pill; an absent one a grey **Missing** pill. The path a
present row resolved to, the link it lands on, the seen-elsewhere note and the
sandbox's own stderr are **not drawn on this screen** any more: they move to
the by-hand screen in task 04, where a block per row has the room. The
dependencies tests that assert on them against the checkbox screen move with
them. Checkboxes are ember: a custom 18px box, or at minimum
`accent-color: var(--accent)`.

**The button names what it does.** *Install N* with N the rows ticked, and
**Next** with none ticked, which goes straight on as today. Disabled is drawn
at 45% opacity with its reason as one short line beside it in place of the
paragraph note under the button today: *A sandbox, git, and one agent are
needed*. **Install by hand instead** is a quiet ember link in the foot, the
phone's alternative under the button, and it opens the by-hand screen with no
run, over **every row this machine is missing**, ticked or not. The other
reading, only the ticked rows, was rejected: somebody choosing to install by
hand wants the commands for everything that is missing.

Strings are sentence case except the group labels, which are drawn uppercase
by the stylesheet from sentence-case text.

## Acceptance criteria

- [ ] The three groups, the 2x2 tiles and the Installed and Missing pills match
      the board at 1088px and 390px; present rows have no box and no path or
      link detail under them.
- [ ] The button reads *Install N* for N ticked rows and Next with none, and
      its disabled reason stands beside it as one line.
- [ ] *Install by hand instead* opens the by-hand screen over every missing
      row without starting a run, and the existing tick and press tests pass
      against the new markup.
