# 04. Agent Profile account paths browse

## What to build

The Agent Profile form's per-type account path fields adopt the component, in
the **watched** scope — the server refuses each of them outside the Watched
Paths — and this task adds the two per-field modes the component has not
needed until now:

- **Files.** A field naming a file (Claude Code's `config_file`, a path like
  `.claude.json`) shows files as well as directories: a file row writes the
  field and is a leaf, a directory row drills as everywhere else. Fields naming
  directories keep showing directories only.
- **Dotfiles.** These fields exist to point at dotfiles (`.claude`,
  `.claude.json`), so all three show them — as a per-field switch, with every
  field from tasks 02 and 03 keeping them hidden. The endpoint always lists
  them; showing is the field's decision.

The Profile form's module header carries the third copy of the "typed rather
than picked" sentence; rewrite it as the other two were.

## Acceptance criteria

- [ ] The directory fields browse to directories and the file field to a file,
      with dotfiles visible in all three.
- [ ] The fields from tasks 02 and 03 still hide dotfiles and still show
      directories only.
- [ ] Saving a Profile with browsed paths behaves exactly as with typed ones,
      and the rewritten module header lands.
