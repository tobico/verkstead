# 03. The share-on-Done toggle

## What to build

A `share_on_done` boolean joins the config file, following the shape the build
cache switch set: absent, an absent file and an unparseable one all read as
**off**, and a saved value reads back as written. It travels the settings API
both ways and draws as a switch labelled **Share to pull request on Done** on
the GitHub and git author details page, beside the token it depends on.

The whole-file save contract holds: every settings pane's save must echo the
new field, so saving any other section leaves the toggle standing.

It controls nothing yet — task 04 wires the settle. This slice is the setting
existing, travelling and surviving.

## Acceptance criteria

- [ ] The switch on the GitHub and git author page reads back as saved, across
      a server restart.
- [ ] Saving any other settings section leaves the toggle where it was.
- [ ] An absent key reads as off.
