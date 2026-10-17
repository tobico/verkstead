# 02. A Repo pointed at a mirror

## What to build

A Repo can be pointed at a mirrored Author, and everything that resolves a
Repo's Author resolves it out of the one list — local Authors from the files,
mirrors from task 01's rows — so a mirror is an Author like any other wherever
a Repo is concerned.

- **The pickers offer mirrors and the store takes them.** Open repo, Create repo
  (on this device) and the Repos pane's repoint list mirrors with the device
  mark, and registration, creation and repoint accept a mirror's uuid instead
  of refusing it as an Author that isn't configured. The live-conversation rule
  on a repoint holds unchanged.
- **One resolution, read everywhere.** Today `Config::author_of` /
  `author_missing`, `Gh`'s token, and the sandbox's git author and `GH_TOKEN`
  read `config.yaml` alone, so a mirror's uuid resolves to *no Author* — and
  `Gh` would then quietly use the host's own login. Every one of those call
  sites (roughly eighty across the server: the starts, the stage pipeline,
  the PR, checks and merge sweeps, sharing, Create repo) reads the one list
  instead. A mirror resolves to its git name and email, and to the token held
  under its uuid in this device's `secrets.yaml` — which task 03 writes.
- **A mirror whose home holds a token but whose copy has not arrived refuses
  by name**, naming the home device, the way a broken Profile mirror does:
  Start, Adopt and Take-up refuse with an outcome of their own, and a stage
  step that needed it halts with a Notice. Treating it as a tokenless Author was
  turned down, so **nothing Verkstead itself does on that Repo falls back to the
  host's login** while it waits. Until task 03 lands, every mirror whose home
  holds a token reads this way.
- **A mirror whose home holds no token** is a tokenless Author, under 02's rule
  for a local one: the session gets the name and email and no `GH_TOKEN`, and
  Verkstead's own git is left to this machine's credentials (decided in
  planning, over refusing for mirrors only).
- **A mirror that has gone** — removed at home, or its device unlinked — is a
  Repo whose Author is gone, refused by name under 02's rule.
- **The companion rule compares uuids across the one list**, so a companion
  pointed at a mirror and a Repo pointed at a local Author are two Authors.

Words for the new refusal follow `docs/copy.md`; keep `CONTEXT.md`'s **Author**
term current.

## Acceptance criteria

- [ ] With a mirror picked at Open repo or on the Repos pane, the Repo records
      its uuid, and a session on it under a tokenless mirror commits as that
      Author's name and email.
- [ ] A Repo pointed at a mirror whose home holds a token, with no copy here,
      refuses Start, Adopt and Take-up naming the home device, a stage step
      halts with a Notice saying the same, and no server-side `gh` or git on
      that Repo runs under the host's login.
- [ ] A Repo pointed at a mirror that has since gone refuses its next Start
      with 02's *author isn't configured any more* refusal.
- [ ] A companion pointed at a mirror beside a Repo pointed at another Author
      is refused by 02's companion rule, naming both.
