# 12. The Forge interface and GitHub across

## Goal

A **Forge** is a plugin's to provide, and GitHub is the first one across the
boundary. The `forge` block of a manifest declares the hosts the Forge answers
for, the noun it calls a pull request, how git inside the sandbox is to
authenticate against it, the command-line tool to lend the agent, and the
**capabilities** it has — `checks`, `review`, `publish`, `stacks`, `create` —
and its component exports the operations the server has always asked of
GitHub: find a pull request by branch or by number, open one as a draft against
a base, read its details, comment on it, list the open ones, read how it landed,
publish a Share, create a repository, verify a token and its scopes, read a
target out of prose, and say whether a remote URL is one of its own.

When this stage is done the built-in GitHub plugin is a wasm component over
WASI HTTP, `gh` is gone from the server, and nothing the human can see has
changed on a Repo at github.com: the wrap-up watches checks and comments as it
did, Share publishes to a gist, **Create repo** makes a repository, the Target
field reads a URL out of a Brief, and `verkstead done` still refuses a branch
with no open pull request. What has changed is underneath and in two places the
agent touches: a session opens its pull request with **`verkstead pull-request
open`** rather than `gh pr create`, and every skill has lost its `gh` lines, so
a skill reads the same on a Forge that is not GitHub.

A Repo whose remote matches no enabled Forge has no Forge, and the workbench
says so where a pull request would have been drawn.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Forge*** — the category, its
  declared capabilities, the noun, the host matching, one token per Forge per
  Author, and the subcommand. The alternatives turned down are recorded there
  and not reopened here: skills carrying a per-forge section spliced into the
  prompt, skills staying in `gh` with other Forges reached only by the server,
  stacks or publishing required of every Forge, a Forge picked on the Repo's
  row, and Verkstead saying *pull request* whatever the Forge calls it.
- **GitHub crosses the boundary as a component** (Set 1768, Q36), revising
  Set 1760's Q2, which had kept it in-process Rust over `gh`. The human moved
  it across for the reason the Activities were moved: a boundary proved only by
  a third party is one nobody here has pushed on. So the component speaks
  REST and GraphQL over the `https://api.github.com` outbound host it asks for
  in its manifest, and `gh` is no longer run by the server anywhere. The
  consequences, each a decision in force:
  - **Walking a stack is read off the API.** `stacks.rs` asks `gh pr list` for
    the open pull requests and follows base to head; the component answers the
    same question from the pull requests endpoint, `isCrossRepository`
    included, so a fork is still refused at take-up.
  - **Syncing a stack is the session's job.** The server runs `gh stack sync`
    today for Fix Merge Issues, whatever the resolution strategy says, a stack
    being gh-stack's. A component cannot run a program, so the `addressing`
    session runs it in the sandbox, where the skills already run `gh stack`
    and where `gh` is still lent as the Forge's declared tool. The Fix Merge
    Issues prompt says so, and the server stops doing it.
  - **The Mac-only `gh` lookup goes.** `github.rs` looks `gh` up on
    `sandbox::machine_path` at every call on a Mac because the service's PATH
    is not the shell's; with no `gh` on the server there is nothing to look up.
  - **The token reaches the component as a secret the host sets**, under the
    `secrets` capability the manifest asks for by name: the server reads the
    Author's token for this Forge out of `secrets.yaml` at call time, exactly
    where `Gh::as_author` reads it today, and sets it in the component's
    environment for that call. Header values and tokens go the way ADR-0021
    sends MCP headers — written where a secret is written, never returned to
    the viewer — and the component never learns where the file is.
- **One token per Forge per Author.** `secrets.yaml` holds
  `author_tokens: {<author uuid>: <token>}` today, one GitHub token per Author
  (ADR-0025). It becomes a map keyed by the Author and then the Forge plugin's
  uuid, an installation from before reading its one token as the built-in
  GitHub's; the Author record's `holds_token` becomes a set of Forge uuids, which is what a
  Repo on a Forge reads to know whether its Author can write there. The Git
  settings section asks for a token per **enabled** Forge under each Author and
  verifies it through the plugin's `verify` export, which answers with the
  scopes it found against the scopes the manifest declares it needs; today's
  `Scopes` type and `X-OAuth-Scopes` reading move into the GitHub component.
- **A Repo's Forge is its remote's host.** Read at the moment it is needed,
  against the hosts every enabled Forge declares plus the hosts the human has
  added to a Forge through a declared settings field — a self-hosted instance
  is a host added, not a plugin written. A Repo matching none has no Forge: no
  wrap-up is started for it, `verkstead done` accepts its signal the way it
  does when GitHub is out of reach, Share has no Publish, and the Repo's row
  and the Conversation's card say *No forge for <host>* rather than drawing an
  empty pull request pane. Picking the Forge on the Repo's row was turned down
  because the remote already says it, and a fact read is one that cannot go
  stale.
- **The noun is the plugin's.** GitHub declares *pull request*; the viewer
  draws the declared noun wherever a Repo on that Forge is drawn — the pinned
  card, the pane, the Target field's placeholder, the Share-to row, the push
  notification — and CONTEXT.md's *pull request* becomes the word for the
  thing in the documents, with the screen saying what the Forge says.
- **Capabilities are declared and their absence is settled, not failed.** A
  wrap-up on a Forge without `checks` or `review` pre-settles that watcher the
  way Fix Merge Issues pre-settles the review today; one without `stacks`
  leaves a roadmap's chain to the rebases the join already does, every stage's
  pull request opened against the branch below as the chain has always been
  git's; one without `publish` has no Publish on the Share pane and the pane
  says so; one without `create` makes **Create repo** stop at the local
  repository with a line saying no remote was made. GitHub declares all five.
- **`verkstead pull-request open`** is how a session opens one. The CLI asks
  the server, the server asks the Repo's Forge as the Author, records the
  pull request on the Conversation the way `wrapping.rs` records one it finds,
  and prints the URL as YAML — `url: https://…` — on stdout, nothing else;
  refused with a non-zero exit and the reason on stderr where the branch is
  not pushed, the Author holds no token for this Forge, or the Repo has no
  Forge. `--base <branch>` names the base, read off the Conversation's record
  where it is not given. The skills become forge-neutral: `gh pr create
  --draft` lines become the subcommand; `gh pr diff` becomes `git diff` against
  the base the prompt names; `gh pr checks` goes, the prompt already carrying
  what failed; `gh pr view --json number` goes, the Conversation's record
  being on the prompt; `gh stack` stays as the session's own job on a Forge
  with `stacks`, which the prompt says. The grilling and investigating skills
  mention `gh pr create` only to forbid it, and are reworded to forbid the
  subcommand. `docs/agents/git-workflow.md` is this repository's own process and
  stays written in `gh`: it is the Repo's and not Verkstead's, and the
  `implementing` skill still says to read it.
- **`Trouble` becomes the interface's error kinds.** `NoPullRequest`,
  `NotLoggedIn`, `NoRemote` and `Refused` are matched off `gh`'s stderr today
  (`github.rs:604-637`); the WIT world names them as the variants an export
  can fail with, so `runner.rs` and `done.rs` go on branching on the same four
  without reading anybody's text. `Ours` — a failure in Verkstead rather than
  at the Forge — stays the host's.
- **Stands on 09.** The wrap-up Activity is the Forge's main caller — checks,
  comments, review, landing — and crossed in 09 over the Forge as it is, so
  that this stage moves one thing. The host function an Activity calls the
  Forge through, settled in 08, is what this stage puts the trait behind.
- **Not forge work:** `updates.rs` reading `releases/latest`, the desktop
  updater's GitHub provider, the VAPID contact, and the annex flake's release
  URLs. They are Verkstead's own distribution and stay as they are.

## Proposed tasks (provisional)

1. **The trait and the WIT world** — a `Forge` trait in the server with the
   operations above, `github.rs` as its one implementation, every caller going
   through it; the `forge` world in WIT with the same operations and the four
   error kinds; the `forge` block of the manifest grammar — hosts, noun,
   credential username and hosts, tool and token variable, capabilities,
   needed scopes — refused by `verkstead plugin check` where incomplete.
   AC: no module but the trait's implementation names `gh`; the check refuses
   a manifest naming a capability that is not one of the five.
2. **Tokens per Forge per Author** — `secrets.yaml` rekeyed, the single token
   read as `github`'s, `holds_token` a set, the Git section asking per enabled
   Forge and verifying through the plugin. AC: an installation from before
   starts with its GitHub token where it was; a second Forge enabled draws a
   second field under each Author; a token with scopes short of the declared
   ones is refused with the missing scopes named.
3. **A Repo's Forge** — read off the remote host against enabled Forges and
   the extra-hosts setting; a Repo with none draws *No forge* where a pull
   request would be, starts no wrap-up, and has no Publish. AC: a Repo on a
   host nobody declared lands Done at the signal with nothing watching, the
   Timeline saying why; adding the host to a Forge's setting makes the next
   Conversation on it wrap up.
4. **The noun** — the viewer draws the Forge's noun on every site that says
   *pull request* for a Repo, the generated types carrying it per Repo. AC: a
   Forge declaring *merge request* reads so on the card, the pane, the Target
   placeholder, the Share row and the notification; GitHub reads as before.
5. **Capabilities** — each of the five gated: a watcher over an absent one
   settled at once, no Publish without `publish`, Create repo stopping local
   without `create`, a chain left to rebases without `stacks`. AC: a fake Forge
   declaring none wraps up on mergeability alone and says so; a roadmap on it
   opens each stage against the branch below with no sync attempted.
6. **`verkstead pull-request open`** — the subcommand, its server route, the
   record it writes, its refusals; every skill forge-neutral. AC: a Tinker
   whose session runs the subcommand lands in Wrapping with the pull request
   recorded; `grep gh crates/server/skills` finds only `gh stack` lines guarded
   by the capability; the Fix Merge Issues prompt tells the session to sync.
7. **GitHub across** — the component over WASI HTTP, REST and GraphQL, the
   token set as a secret per call, stack walking off the API, the Mac lookup
   gone, `gh` removed from the server's programs. AC: the server suite's
   GitHub stand-in is an HTTP stand-in rather than a fake `gh` script; every
   test that drove the fake passes against it; the binary no longer execs
   `gh`; Share publishes a gist and the wrap-up watches checks on a real Repo.

## Re-verify at start

- Stage 09 landed: the wrap-up is an Activity reaching the Forge through a
  host function, and 08's host interface still has one call per operation
  above rather than a coarser one.
- Stage 01's manifest grammar still has a `secrets` capability and a declared
  settings field of kind `list`, which the token and the extra hosts rely on.
- `github.rs` is still the only module that runs `gh`; `stacks.rs` still asks
  `gh pr list`, and the server still runs `gh stack sync` for Fix Merge
  Issues — if the sync has moved into the session already, task 7 shrinks.
- `secrets.yaml` still keys `author_tokens` by Author uuid alone, and
  `holds_token` is still one flag on the Author record.
- The store's pull request tables are still keyed by `(repo_id, number)` with
  no host column, which this stage leaves alone: the Repo knows its Forge.
- The `Scopes` reading is still `gh api -i user` and `X-OAuth-Scopes`; the
  component reads the same header off the REST call.
- Ten skills still mention `gh`; check which, since stages 07 to 11 move the
  skills into a pack and may have reworded some.
- wasmtime's WASI HTTP is still what 01 wired in, and it still reaches an
  outbound host by the manifest's grant alone.
