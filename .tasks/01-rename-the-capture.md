# 01. Rename the byte capture

## What to build

The raw terminal bytes a session prints are the **Capture**. Everything that
currently calls them a transcript is renamed to say so — tables, endpoint,
wire types, server and store modules, the web pane, CSS class names, tests,
and the doc-comment prose that explains any of it.

This is deliberately the first task and deliberately mechanical: it keeps the
rename's diff out of the substantive tasks that follow, and it frees the word
*transcript* for the readable record tasks 03–05 build. Nothing about what
the UI shows changes — the same bytes reach the same pane and look the same.

The prose is the real work here, not the identifiers. This codebase explains
itself in long doc comments, and a good number of them argue about "the
transcript" by name. They are rewritten to argue about the Capture, keeping
the argument intact; a blind search-and-replace would leave sentences that no
longer parse.

**No migration.** The store has no migration machinery and three separate
comments explaining why it has none — that stays true. The Capture tables are
created under their new names and the old ones are simply not carried over.
The only database in existence is the gitignored local dev one, and it is
empty, so it is deleted and recreated rather than upgraded.

## Acceptance criteria

- [ ] Grepping the tree for "transcript" turns up only the readable record's
      meaning, or nothing at all — no remaining use of the word for bytes
- [ ] The details pane shows a session's output byte for byte as it does
      today; the Timeline summary line is unchanged
- [ ] `web/src/api/types.ts` is regenerated from Rust by ts-rs, not hand-edited
- [ ] A fresh database comes up with the Capture tables and no transcript ones
- [ ] The full test suite passes
