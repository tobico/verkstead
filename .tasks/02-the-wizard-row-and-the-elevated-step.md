# 02. The wizard row and the elevated step

## What to build

On Windows, and only there, the wizard's first step gets a row reading
**Linux development (WSL)**. The human chose those words, and the row is
unticked by default because it is a multi-gigabyte download that may need a
restart. It is an optional row: the step's objective (sandbox, git, a
harness) does not wait on it.

The row's state comes from task 01's probe:

- **No WSL**: absent, with a checkbox.
- **Needs a restart**: absent. The hint screen carries the row's own
  instruction to restart Windows. Today the hint screen has one shared
  `RESTART` note under the `PATH` list, and this is a per-row instruction in
  `web/src/setup/instructions.ts`.
- **Virtualization off**: drawn **refused with the reason**, and it cannot be
  ticked. The wizard has no refusal-before-a-run today, so this adds one: the
  row says what is wrong and how to fix it (turn virtualization on in the
  firmware settings) and offers no checkbox. Check `docs/copy.md` for the
  wording.
- **WSL answers but there is no `Verkstead` distro**: the row reads
  **ticked by itself** after the restart. The wizard already re-probes on its
  poll, and that is what makes it tick. In this task nothing more runs for
  it. Task 04 is the run that makes the distro.
- **Distro present**: present.

Ticked, the press adds the row's units to the Windows arm of the install
plan, each raised through the same `Elevate::raise` as the arm's other rows:
`wsl --install --no-distribution`, then `wsl --update`. A server with no way
to raise sends the row to the hint screen, as it does every other row, and
the hint gives both commands. The firewall rule is *not* added here: its
adapter does not exist until the WSL VM first runs, so task 04 adds it.

## Acceptance criteria

- [ ] On Windows the row appears unticked. On other platforms it does not
      appear.
- [ ] The plan for a ticked row on a machine with no WSL is the two raised
      units, in that order. A server that cannot raise lands the row on the
      hint screen with both commands.
- [ ] A probe of *needs a restart* lands the row on the hint screen with its
      own restart instruction. A probe of *WSL answers, no distro* draws it
      ticked with nothing pressed.
- [ ] A probe of *virtualization off* draws the row refused with the reason
      and no checkbox. The press cannot include it.
