# 02. Install from the pane

## What to build

**Install** on the Components row of a machine with no Annex starts **the
wizard's own install run for the WSL row**: the same plan, the same units
(turning WSL on behind UAC, the firewall rule, the download, the import, the
flake written in, the rebuild, and the boot and link through the Keeper's
`Annexed::up`), and the same screens. Those are the install screen's progress
bar and status line, and the hint screen with the row's own instruction where a
restart is owed or something failed (ADR-0024: *Install from the pane is the
wizard's run*). This is for a machine that skipped the row at setup.

**The drift this task exists for:** the onboarding handle's install refuses any
run once the wizard is not the page there is (`Refusal::Over`). That refusal is
right for the wizard's own press and stays. The Components press needs its own
way in that is allowed after the wizard has finished. It runs the WSL row alone,
through the same `Installer`, so there is still one run at a time per server: a
press while any run is going is refused as `Going`, and the pane reads the run
already going rather than starting a second one.

**The screens are reused, not copied.** The pane hands over to the setup
screens' install view and hint view for this one row, or draws those same
components inside the pane. Pick whichever keeps a single implementation. On a
finished run the human lands back on the Components pane, with the row reading
what the probe now says (running, once linked).

**Elevation is the wizard's.** A server with no way to raise a dialog (anything
not started by the desktop app with a display) fails the row with the wizard's
existing sentence, and nothing new is invented.

The restart path must work too: WSL turned on, restart owed, the hint screen
saying so. After the restart, Install on the pane again picks the run up (the
probe now says WSL answers with no distro) and makes the box.

## Acceptance criteria

- [ ] On a Windows Keeper whose wizard skipped the WSL row, Install on the
      Components pane makes, boots and links the Annex, showing the install
      screen's progress, and the row then reads *running* with the Devices list
      showing the Annex.
- [ ] Where WSL was not on, the run turns it on and lands on the hint screen
      with the restart instruction. After the restart, Install finishes the
      Annex.
- [ ] The wizard's own install press is still refused once the wizard is over,
      and a Components press while a run is going is refused as already going.
      Both are covered by the install suite.
