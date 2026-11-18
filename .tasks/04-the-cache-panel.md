# 04. The cache panel in the plugin's pane

## What to build

With the language rows gone from the Sandbox page, what each one's panel held
moves into its **Language plugin's pane**, under the size field (Set 1784, Q2):
for each language the plugin provides, what its store holds against the size
in force, the store directories and the exceptions (sccache holding itself to
the size, a directory never trimmed), when it was last trimmed, and **Clear**,
which empties that language's stores (sccache's too, for Rust) and is off —
and refused by the server — while a session or terminal runs, saying how many
of each. An unticked plugin's pane still shows its usage and Clear, since an
off language still holds what it fetched. The Plugins page row may carry the
one figure — `Cache usage: 3.2/30GB` — the old row did.

This is the first Category-specific part of a plugin's pane: the pane draws the
declared fields, then what the Category adds.

**The Build caches part** of the Sandbox page keeps one thing, which is not a
language's (Set 1784, Q3): the Compile Server's own state — the line saying
there is no sccache where the server can see one — still drawn there and on
the Sandbox card's status. Everything else in that part, and the settings
payload's per-language fields that only it read, go.

## Acceptance criteria

- [ ] The Sandbox page's Build caches part has no language rows, and still says when there is no sccache where the server can see one.
- [ ] Rust & C/C++'s pane shows its cache usage against its size, its store directories and when it was last trimmed; Clear empties its stores and the figure redraws.
- [ ] Clear is drawn off with the count while a session runs, and the server refuses one that arrives anyway.
