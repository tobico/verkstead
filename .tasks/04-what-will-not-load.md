# 04. What will not load

## What to build

Two ways an entry in `config.yaml` fails, and one answer to both.

A descriptor that names a variable **the Sandbox sets itself** is refused. The
file is the installer's own and they can already open binds, but a descriptor
that quietly replaced the session's `PATH` would be a session that cannot find
`verkstead`. **The refused names are the union of all three platforms'**, on
every platform — Windows sets names the two Unixes do not, and a file that is
fine on one machine and breaks the same install on another is worse than a
refusal the installer sees wherever they wrote it.

A descriptor that **does not parse** is refused the same way.

Either way, the language **falls back to the built-in of that name**, and turns
off only where there is no built-in to fall back to. An installer whose override
is refused keeps the cache they already had, which is what merging key by key is
for in the first place: losing Rust's build cache to one mistyped variable is
the worse experience for not having checked. Every other language carries on
loading, and the server never refuses to start — an unreadable `config.yaml`
already means the settings stand at their defaults rather than a Verkstead that
will not come up.

**The page says why**, either way: the reason, naming the variable where a
variable is what was refused, and whether the language is running on its
built-in or off altogether.

**And the entry stays in the file.** A save from the page writes the whole of
`config.yaml`, and an entry it could not read is still text the installer wrote
and is about to fix — so it goes back as written, the way the descriptor keys
the page never drew do.

## Acceptance criteria

- [ ] An entry naming a refused variable falls back to the built-in of that name,
      with a reason naming the variable; an entry that does not parse does the
      same, and every other language still loads.
- [ ] A refused override of Rust leaves a session with exactly the variables it
      had before that file was written; an entry with no built-in behind it that
      will not load is off rather than on at nothing.
- [ ] The pane says the reason and which of the two happened, and a save from the
      page leaves the unreadable entry in the file as it was written.
