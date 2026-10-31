# 05. The roadmap pane names the device

## What to build

The roadmap's details pane already names a stage's platform (`on windows`).
Beside it, it names the device the stage is on, wherever that is not home
(Set 1549, Q4): read off home's tombstone of the stage — the transferred mark
naming the device holding the live copy — by that member's name, whoever moved
it there. Nothing new is stored. A stage at home shows nothing extra, and a
stage moved by hand shows its device too, platform or none.

The server's roadmap pane source gains the device's name per stage; the viewer
draws it after the platform (e.g. *on windows · nuc-win*), in words by
`docs/copy.md`.

## Acceptance criteria

- [ ] A stage placed on nuc-win reads *on windows* and nuc-win in its roadmap's
      pane at home.
- [ ] A stage at home reads as it does today.
- [ ] A stage with no platform moved by hand names its device.
- [ ] Covered by a server test of the pane's source and a web test of the
      drawing.
