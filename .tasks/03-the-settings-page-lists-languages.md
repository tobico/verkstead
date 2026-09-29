# 03. The settings page lists languages

## What to build

The settings view stops holding one build cache and carries a language per
loaded descriptor instead: its label, whether it is on, and — for the one whose
store an sccache bounds — the size and whether a session's compiling is cached
at all.

The **Language support** section already has its shape: a card in the middle
pane naming what is switched on, and a details pane of controls behind it. What
changes is what they are drawn from. The card names the languages that are on
rather than a hardcoded `Rust`, and the pane draws a checkbox each. The size
still hangs off Rust's box — indented under it, disabled while that box is off
and while there is no sccache to read it — and the warning about uncached
compiles stays where it is.

The page's two existing rules hold. A checkbox saves itself the moment it is
ticked, carrying the size the *server* last gave it rather than what the field
holds, so a tick never commits a number nobody pressed Save on. And a save says
what the whole file holds afterwards, so every form on the page carries the
languages along untouched — the build cache rides on six saves today, in four
shapes: spelled out in the Sandbox binds, Cleanup and Instructions panes, and
through the two shared helpers used by the Language support pane, the settings'
git pane and the onboarding wizard's git step. Change the shape once in the
helpers and let the panes that spell it out follow.

**The file only.** There is no descriptor editor here: the page draws a checkbox
per language the server lists, and an installer who wants more edits
`config.yaml`.

The view types are the render crate's and generate the viewer's TypeScript, and
the viewer's fixtures are rendered by the real endpoints. Both are written by
`cargo test` and committed, so the diff is the review.

## Acceptance criteria

- [ ] The pane draws a checkbox per language the server lists, and a tick saves
      that language's `enabled` without committing a size nobody pressed Save on.
- [ ] A language an installer wrote into `config.yaml` appears on the card and in
      the pane under its own label.
- [ ] Every other settings pane and the wizard's git step still carry the
      languages along on their own saves, and the generated TypeScript and the
      viewer's fixtures are up to date.
