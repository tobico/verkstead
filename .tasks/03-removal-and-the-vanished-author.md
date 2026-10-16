# 03. Removing an Author, and the Author that is gone

## What to build

**Removal.** There is no way to remove an Author today. Add **Remove** on the
Git section's Author switcher (the listbox drawn once there are two Authors),
asking once before it acts. The server removes the Author from `config.yaml`'s
`authors:` and its token from `secrets.yaml`'s `author_tokens:`, and **refuses
while a registered Repo points at it**, naming those Repos and sending the human
to the Repos pane. An unregistered Repo keeps its association as history and
blocks nothing. Removing down to one Author makes the pickers and names vanish
by the existing two-Authors condition.

**The vanished Author.** A Repo whose recorded uuid is in no Author — reached by
a hand edit of `config.yaml` — today falls through `Config::author_of` to
nobody, and every door then says *"no git author is set"*, which is wrong. Give
it its own refusal **at every door**, naming the Repo and sending the human to
the Repos pane to pick another:

- a plain Start of a session;
- the grilling cut and the adoption cut (`GrillingStarted` / `Adopted`, beside
  their `NoGitAuthor`);
- a roadmap stage's carry-on, which halts with a Notice where nobody pressed
  anything — the shape the missing-author halt already has (see `unsigned` in
  the continuing code).

Falling back to the first Author was turned down (ADR-0025). A Repo with no
Author recorded among several (`author_unpicked`) is told to pick one, by the
same route. Strings follow `docs/copy.md` (lowercase vocabulary on screen).

## Acceptance criteria

- [ ] Removing an Author two registered Repos point at is refused naming both; unregistering both and removing again succeeds, and the token is gone from `secrets.yaml`.
- [ ] Deleting an Author's entry from `config.yaml` by hand turns a Start, a grilling start and an adoption on its Repo into a refusal naming the Repo and the Repos pane.
- [ ] A stage that would carry on onto that Repo halts with a Notice saying the same, and starts nothing.
