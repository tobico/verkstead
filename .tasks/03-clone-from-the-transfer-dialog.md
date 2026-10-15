# 03. Clone from the Transfer dialog

## What to build

The Transfer dialog's Preflight offers the clone where it finds no match on the
device picked, in place of sending the human to Open repo there.

**The finding carries the origin.** The Preflight's reading runs on the device
the Conversation is on. It reads the Conversation's Repo and each Companion's
origin afresh off git, as the match already does, and now also puts that origin
on each Repo finding with no match. A finding for a repository with no origin
carries none.

**The dialog offers the clone.** In the Transfer dialog, a no-match finding
with an origin reads *clone it there*, as a press:

- for the Conversation's own Repo, and for a Companion too, as the human chose;
- the press runs task 02's clone on the far device, through the same relayed
  route and with the far device's own GitHub token and git author;
- it uses that device's remembered Create repo parent, or the same path card
  where none is remembered there.

Once the clone lands, the Preflight is read again, and Go unlocks if nothing
else is left. A clone that is refused says so, in the far end's own words, under
that finding.

- A finding with no origin still reads as it does now and points at **Open
  repo** there. It is not clonable.
- **Unreachable is still never no match.** A device that does not answer stays
  the single *unreachable* finding and offers no clone.
- Go's own second Preflight on the press is unchanged.

Follow docs/copy.md for the words, and CONTEXT.md under **Transfer** and
**Clonable Repo**. Update the module documentation in the dialog and on the
Preflight that today says no match sends somebody to Open repo.

## Acceptance criteria

- [ ] In a two-server suite, the Preflight for a Conversation whose Repo the far
      device lacks carries that Repo's origin on the finding. A Repo with no
      origin carries none, and an unreachable device is still the single
      unreachable finding.
- [ ] In the viewer suite, a no-match finding with an origin reads *clone it
      there*, for a Companion as well as the Conversation's own Repo. One press
      clones on the far device and re-reads the Preflight, and Go unlocks, so a
      Conversation whose Repo the far device lacks transfers after one press on
      the offer.
- [ ] A finding with no origin still points at Open repo there, and a refused
      clone is said under its finding in the far end's words.
