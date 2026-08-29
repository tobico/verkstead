# 04. A Profile's account is per type

## What to build

A Profile stops holding a bare pair of paths and starts holding an account
whose shape is its agent type's. Claude keeps the directory-and-config-file
pair it already has and works exactly as it does today; what changes is that
the pair is one type's shape rather than the only shape there is, so a later
stage drops a backend in beside it without reworking the store, the wire and
the form all over again.

**The store grows the shape, not a second type.** `AgentType` stays a closed
word list with Claude alone in it — variants arrive with the stages that can
launch them, and an unknown word in the column is still a database written by a
newer Verkstead, refused by name rather than guessed past. What lands here is
the per-type account:

```
Claude { claude_dir, config_file }   // the pair, as today
```

with the discriminator saying which shape a row carries. Nothing yet writes the
other shape, because there is no type that has one.

**And the table the other shape will live in.** The pattern for a new fact is a
new table beside `profiles` rather than a column migrated into it — `profiles`
is STRICT and there is no migration machinery — which is how the model list
already hangs off it. So the table that holds a Profile's single home directory
is created here, keyed by profile id, ready for the first backend that keeps
its whole account under one relocatable home. It stays empty until then.

**Every saved Profile reads back unchanged.** A Claude row written before this
task, including one written before the model list existed and carrying its one
model in the old column, comes back with the same name, the same pair and the
same list, with nothing for the human to retype. The pair still reaches the two
places that use it: the sandbox binds it over `~/.claude` and `~/.claude.json`,
and the transcript reader looks for the session log under the directory.

**The wire carries the shape too.** A Profile as the viewer receives it, and a
Profile as the human has just written it, both say which type they are and
carry the fields that type has — rather than the pair being a field every
Profile is assumed to have. The refusals stay named per path, because "that
path is wrong" would not say which one.

**And the form draws its fields from the discriminator.** What a Profile is
asked for comes off its type rather than being hard-coded — so the stage that
adds a backend adds fields rather than restructuring the form. The type is
still not offered: there is one, a select with a single option is theatre, and
a type that cannot launch would be a lie in a picker. The pane goes on saying
what a saved Profile's type is, beside the fields.

CONTEXT.md's **Agent Profile** entry describes the pair as the whole of what a
Profile is, so it is corrected here.

## Acceptance criteria

- [ ] Every Profile already saved reads back unchanged — name, pair and model
      list — including one written before the model list existed, and the form
      edits and saves a Claude Profile exactly as today with every refusal
      still named by the path it is about.
- [ ] The pair still reaches the sandbox and the transcript reader, and a
      grilling session launched under a Claude Profile runs as it did.
- [ ] The table holding a Profile's single home directory exists beside
      `profiles`, hangs off it by id, and needs no migration for the stage that
      first writes to it.
- [ ] `AgentType` holds Claude alone, the form offers no type picker, and a row
      naming a word the binary does not know is refused by name rather than
      read past.
