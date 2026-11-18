# 05. The installer's languages

## What to build

**An override shows on its plugin's row.** A `languages:` entry in
`config.yaml` naming a language a plugin provides still merges over that
plugin's block key by key (task 01). Where it will not load — a key nothing
knows, a variable the Sandbox sets itself — the language runs on the block
Verkstead ships, as today, and the reason is that plugin's **trouble**, on its
row and in its pane, rather than a sentence on the Sandbox page.

**An eighth language is a plugin of its own.** An entry naming a language no
plugin provides is drawn as a plugin from the installer in the **Installed**
group, saying it comes from this device's `config.yaml`, so an installer who
adds a language still gets its row with no code change. Its tick and its size
live on a cluster record keyed by a uuid derived from the language's name (Set
1784, Q5): drawn only on devices whose `config.yaml` has the entry, and shared
by every device that does. It has no Remove (the entry is the file's to take
out), and its pane has the size field and the cache part of task 04 where its
block names a store. An entry naming no built-in that will not load is in
trouble and off, as today.

Its `enabled` and `size` keys are taken up once, the way task 02 and 03 took up
the built-ins', on winning and the larger size winning. The Sandbox page's last
language rows — the ones task 02 left for these — go.

## Acceptance criteria

- [ ] A `languages:` entry naming `zig` with a variable draws an eighth row under Installed; a session gets the variable; unticking the row drops it from the next session.
- [ ] Two devices with the same `zig` entry share one tick; a device without the entry does not draw the row.
- [ ] A mistyped `detct` in an override of Go shows as trouble on Go's row, and Go's sessions get the variables Verkstead ships; an installer's variable override that loads still wins.
