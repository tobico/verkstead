# 18. Roadmap detail page

## What to build

The same detail page as task 17, for the roadmap: the stage list card opens
the details pane showing each stage's brief as a boxed markdown section in
stage order, with jump navigation. Reuse task 17's pane structure and
endpoint shape end to end — a second document source, not a second design.
Stage briefs stay in place forever (unlike task files), so every stage
renders its document, done or not, with its done state shown on the section
heading.

## Acceptance criteria

- [ ] Tapping a stage list card opens the pane showing every stage brief
      rendered as markdown in boxed sections with jump navigation
- [ ] Done stages are marked as such on their sections
- [ ] Route and web tests cover it
