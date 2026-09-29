# 02. The languages map in `config.yaml`

## What to build

`config.yaml` gains one map keyed by language, and what an installer writes
there is merged over the built-ins. An entry under a built-in's name merges into
it **key by key**, so changing one variable is not a matter of copying the rest
and then missing every later fix to them. A variable set to `null` is taken out.
An entry under a name with no built-in behind it is a descriptor of its own: on
by default, its label on the page and its variables in the next session.

`enabled` and `size` are keys of the same entry, beside the descriptor's own.
They are the only two the settings page ever writes.

**`rust_build_cache` is still read**, as Rust's `enabled` and `size`, so an
install that wrote one keeps what it said. Where both it and the new map say
something, the new map wins.

**The map goes back as written.** One request writes the whole of `config.yaml`,
and what the page sends is what the file holds afterwards — so a save has to
carry every descriptor key the page never drew, exactly as the file had them.
The settings already have a key the page has no field for and carries along on
every save; this is the same shape, and it is also what lets stage 04's
unreadable entry survive a save.

Settings are read at every session spawn, as now, so a switch flipped on a phone
applies to the next session.

## Acceptance criteria

- [ ] A `config.yaml` written by the released version reads as it did — its
      `rust_build_cache` is Rust's `enabled` and `size` — and the new map wins
      where both are written.
- [ ] An override of one of Rust's variables leaves the rest as the built-in
      says, and a `null` takes one out of the next session's environment.
- [ ] An entry naming a language with no built-in behind it puts its variables in
      the next session, and a save through the settings endpoint leaves every
      descriptor key the page never drew exactly as the file had them.
