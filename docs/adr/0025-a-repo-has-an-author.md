# A repo has an author

Amends the *credentials and identity are said rather than found* decision of
2026-08-23 in [the design document](../design/verkstead.md), which gave every
session one git author out of `config.yaml` and one GitHub token out of
`secrets.yaml`. That rule stands — **nothing about who a session commits as is
read out of a home directory, and nothing of it is a file inside a sandbox.**
What this changes is *how many* there are and *which one a session gets*: an
installation may hold several, and the Repo says which is its.

It sits beside [ADR-0011](0011-agent-backends.md), whose Agent Profile is the
*agent's* account, and borrows from [ADR-0020](0020-cluster-mode.md), whose
*Shared Profiles* are how one row is drawn on every device of a cluster. The
roadmap that builds it is `docs/roadmaps/authors/`.

## What was there

One person, one name on every commit, one GitHub account behind every push.
That is most installations and it is what the settings page's **Git** section
says: a name, an email and a write-only token, handed to every session as
`GIT_CONFIG_COUNT` and the pairs it counts and as `GH_TOKEN`, and read by every
call Verkstead itself makes to GitHub.

The human asked for more than one. A developer works under one email and one
GitHub account for their employer and another for themselves; somebody
publishes personal projects under a pseudonym and does not want to be named as
their author. On a machine holding both, the failure that matters is not a
commit under the wrong name — that is undone with a rebase — but an agent, at
three in the morning, with the *other* identity's token in its environment and
no way to know it should not use it. The sandbox already decides what a session
can reach; this is the same decision about who it is.

## The decision

Settled in the grilling of 2026-10-16. Each of the following was decided there,
and the alternative named is what was turned down.

- **The concept is an Author**: the git name, the email and, optionally, a
  GitHub token. The word the code, the docs and the settings heading already
  used for the one there was. *Identity* and *persona* were turned down: both
  are on the Agent Profile's own *Avoid* list for the reason that a Profile is
  the agent's account and not the human's name, and *identity* already names
  the device endpoint and the cluster's certificates.

- **Keyed by a uuid, with an optional display name.** The uuid is what a Repo
  records and what every device agrees on; the display name is for pickers and
  rows, and falls back to the git name where none was typed. The MCP server's
  rule — the name is the identity and is never renamed — was turned down for
  this one: a label somebody will want to change should not be the key.

- **No signing key.** Verkstead hands no GPG or SSH signing key to a session
  today, and an Author is not where that starts.

- **`authors:` in `config.yaml`, tokens in `secrets.yaml` under the uuid.** The
  split the GitHub token and the MCP header values already follow: what is
  configured in the file somebody may hand-edit, what is secret in the file at
  mode `0600`. A database table holding the tokens was turned down, because it
  would move a credential out of the one file kept at that mode.

- **The first start moves the single keys into the list.** `git_author` and
  `github_token` become one Author with a fresh uuid and no display name, and
  every registered Repo is recorded as its. Leaving the single keys in place as
  an unnamed Author was turned down in favour of one shape in the files.

- **With one Author nothing on screen changes.** The pick exists and is
  recorded silently; the pickers are drawn only once there are two to choose
  between, and adding a second changes nothing about any Repo already
  registered.

- **A Repo picks its Author at Open repo and Create repo**, and the Repos pane
  shows and changes it. **Repointing is refused while a Conversation on the
  Repo is neither Done nor Closed** — the rule unregistering already follows —
  because a branch half committed as one Author would otherwise carry on as
  another. Letting the next session pick up the change was turned down for that
  reason.

- **Removing an Author is refused while a registered Repo of this device
  points at it.** An unregistered Repo keeps its association as history and
  blocks nothing. Leaving Repos unassigned on removal was turned down.

- **A Repo whose Author is gone refuses a start by name**, the way a missing
  git author already does, and halts with a Notice where nobody pressed
  anything. A hand edit of `config.yaml` is the one way to get here. Falling
  back to the first Author in the list was turned down: the feature exists to
  stop the wrong name reaching a commit.

- **A Companion Repo of another Author is refused on the composer, read-only
  or read-write.** A sandbox has one environment, so one `GH_TOKEN` and one git
  author; a private companion under the other account could not even be
  fetched inside. Allowing read-only companions was turned down.

- **Everything Verkstead itself does on a Repo authenticates as that Repo's
  Author.** Every server-side `gh` call already names the Repo it runs in. The
  gap was plain git — the fetch before a cut, the pushes — which ran as the host:
  those go through the credential helper publishing already hands a token to.
  An Author with no token leaves git to the host's own credentials, as today.
  Reading the Author's token for `gh` alone was turned down.

- **A share is published as its Repo's Author**: the secret gist made with
  that Author's token, the share committed into it as that Author, and the
  comment linking it left with the same token. An Author with no token refuses
  the publish, as no token configured does today, rather than falling back to
  the host's login or another Author's: a pseudonymous pull request carrying a
  link into the human's other account is the leak this decision exists to
  stop. *Added in review, 2026-10-17: the grilling's list missed sharing.*

- **Create repo makes the GitHub repository under the picked Author's token**,
  and the GitHub tick is drawn only where that Author has one.

- **One list across the cluster, mirrored the way Profiles are.** An Author is
  at home on the device whose files hold it. Every other device keeps a
  **mirror row** in its database — the uuid, the display name, the git name, the
  email, whether a token is held, and the home device — refreshed over the
  Relay at start, on news and on membership moving. Keeping Authors per device,
  with a transfer preflight comparing name and email, was turned down: the
  human wants one list.

- **And a copy of the token in each device's own `secrets.yaml`**, under the
  uuid, written and taken away as the mirrors refresh. Fetching the token from
  home at the moment it is needed and keeping it nowhere — the account mirror's
  rule — was turned down: a home device that is quiet should not stop a repo
  here from working, and a rotation at home being stale until the next refresh
  is accepted.

- **A mirror whose copy has not arrived yet refuses a Start by name**, naming
  the home device, the way a broken Profile mirror does; a pipeline step that
  needed it halts with a Notice. Treating it as a tokenless Author was turned
  down.

- **A new Author is at home on the device that made it; editing a mirror is
  relayed home**, token verification included, as the ordinary edit of that
  device's own. **Removal checks the home device's Repos only**: a member's Repo
  left pointing at a mirror that has gone is a start refused by name there,
  under the rule above. Asking every member first was turned down, because a
  member not answering would block the removal.

- **A Transfer whose far Repo points at another Author is refused by the
  Preflight**, naming both. Repointing the far Repo as part of the move was
  turned down: repointing is a press of its own, under the live-conversation
  rule.

- **Create repo on a member device offers the one list.**

- **The Git section keeps its shape** and gains a dropdown at its head that
  picks which Author the fields are about, with **Add author** at its foot the
  way the Repo dropdown carries **Open repo** and **Create repo**. A card per
  Author, and a list opening a pane apiece, were both turned down.

- **The Author's display name sits beside the Repo** on the Brief's record and
  on the Repos pane row, drawn only while more than one Author exists.

- **The onboarding wizard still asks for one Author.** More are added on the
  settings page afterwards.

## What was reviewed and left

The Brief asked for a review of everything else identifying that a session is
given. Four things were found, and each was decided, and the review of the
roadmap on 2026-10-17 found two more that the first of them only began on:

- **The Claude account's email.** The `.claude.json` copy a session gets keeps
  `oauthAccount.emailAddress`, which a test asserts comes over as what says the
  account is signed in. Stage 02 probes whether Claude runs without it and
  strips it if so; if not, it stays and is documented here as an exposure. The
  account is the Profile's, and a pseudonymous Repo can already be paired with
  a Profile of its own.
- **Every harness login names its account**, not Claude's alone. Codex's
  `auth.json` carries an `id_token` whose claims hold the account's email, and
  a login cannot be stripped of what makes it a login. So whatever the probe
  finds, a session knows whose agent account it runs under.
- **A Profile whose memory is shared carries every Repo it worked on.** With
  the memory switch on, the whole store is joined — Claude's whole `projects/`
  — so a session on one Repo can read the transcripts of the same Profile's
  sessions on another Author's Repos, with that Author's name, email and
  `git log` in them.
- **What follows from all three**: the Author keeps the wrong name off a commit
  and the wrong token out of a sandbox, and the Profile is the one channel it
  does not close. **Keeping two Authors apart takes a Profile per Author**, and
  a Profile's memory not shared across Repos of different Authors. Nothing
  enforces that pairing; it is said, in the **Author** term, rather than
  refused.
- **The operating-system username.** The home inside a sandbox is the server's
  own at the same path, so every Worktree and Build Cache path a session sees
  spells it out. Rehoming the sandbox under a neutral path would break how the
  harnesses find their roots, and is out of scope. A known exposure.
- **The Instructions text** is one for the installation and may name the human.
  Left as one text; the settings page says it reaches every Repo.
- **Declared MCP servers and their header secrets** are installation-wide and
  attached per Conversation by hand. Left as they are: attaching one is already
  a deliberate press on the Brief.

## What this does not change

A session still finds no `.gitconfig` and no `gh` files inside, and the host's
own are still never bound in. The author and the token still arrive as
environment variables, read at the moment a session is launched, so a change
reaches the next session without a restart. A missing, empty or unparseable
settings file still configures nothing rather than refusing to start. The Agent
Profile, the Built Root and the account mirror are untouched: an Author is who
the *human* is on the work, and a Profile is the account the agent runs under,
and the two are picked independently.
