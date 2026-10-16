# 02. The pick on screen and the rules

## Goal

With two Authors configured, **Open repo** and **Create repo** ask which one the
Repo is, the Repos pane shows each Repo's Author and lets it be changed, and
every mixing the grilling refused is refused by name: repointing under a live
Conversation, removing an Author a Repo points at, starting on a Repo whose
Author is gone, and a Companion Repo of another Author. Every path by which
Verkstead itself reaches GitHub on a Repo — plain git included — does so as that
Repo's Author, Create repo makes the GitHub repository under the picked
Author, and a share of a Conversation is published and linked under its Repo's
Author. The Brief names the Author beside the Repo. The Claude account email in
the config copy has been probed and either stripped or documented.

## Decisions in force

All from [ADR-0025](../../adr/0025-a-repo-has-an-author.md) and the **Author**
term in `CONTEXT.md`.

- **The pick is made at Open repo and Create repo**, as a picker on both cards
  drawn only where more than one Author exists, and recorded silently where
  there is one (01's rule). The Repos pane row shows the Author's display name
  under the same condition and is where it is changed.
- **Repointing is refused while a Conversation on the Repo is neither Done nor
  Closed**, the rule unregistering follows, because a branch half committed as
  one Author would carry on as another. Letting the next session pick up the
  change was turned down.
- **Removing an Author is refused while a registered Repo points at it.** An
  unregistered Repo keeps the association as history and blocks nothing.
  Removal is a press on the Git section's switcher, and asks once.
- **A Repo whose Author is gone refuses a start by name**, naming the Repo and
  sending the human to the Repos pane, and halts with a Notice where nobody
  pressed anything — the shape a missing git author's refusal has today. A hand
  edit of `config.yaml` is the way to get here. Falling back to the first Author
  was turned down.
- **A Companion Repo of another Author is refused on the composer**, read-only
  or read-write alike, naming the companion and both Authors. One sandbox is one
  environment, so one `GH_TOKEN` and one git author, and a private companion
  under the other account could not be fetched inside. Read-only companions of
  another Author were turned down.
- **Plain git of Verkstead's own on a Repo authenticates as that Repo's
  Author's token** — the fetch before a cut, the fetch that draws a branch list,
  the pushes — through the credential helper publishing already hands a token
  to, with `GIT_TERMINAL_PROMPT=0` as every server-side git has. An Author with
  no token leaves git to the host's own credentials, as today. Reading the
  Author's token for `gh` alone was turned down because a private repository
  under the second account would not fetch.
- **Create repo makes the GitHub repository under the picked Author's token**,
  and the GitHub tick is drawn only where that Author has a token. The card's
  *a remote is needed* sentence stands where the picked Author has none.
- **A share is published as the Conversation's Repo's Author.** The secret
  gist is made with that Author's token, the share is committed into it as that
  Author's name and email, and the comment linking it on the pull request is
  left with the same token — read once, as the publish reads it today, so the
  gist and the comment cannot disagree about who made them. An Author with no
  token refuses the publish, the refusal a publish with no token configured
  already gives: a write under the host's login would be a file in an account
  nobody chose, and under another Author's would be the leak itself.
- **The Author's display name sits beside the Repo on the Brief's record**,
  drawn only while more than one Author exists, the way the Repos pane row
  draws it.
- **The Claude account's email is probed.** The `.claude.json` copy keeps
  `oauthAccount.emailAddress`, asserted by a root test as what says the account
  is signed in. If Claude launches and runs a turn without it, it is stripped
  from the copy and the test says so; if not, it stays and ADR-0025's *What was
  reviewed and left* is amended to record what the probe found.

## Proposed tasks (provisional)

1. **The picker on Open repo and Create repo.** Both cards carry an Author
   picker where more than one exists, the registration and the create record
   it, and the Create card's GitHub tick follows the picked Author's token.
   - With two Authors, Open repo cannot land without a pick, and the Repo
     dropdown's row afterwards reads the registered Repo.
   - Picking an Author without a token on Create repo draws the sentence about
     a remote and no tick; picking one with a token draws the tick, on.
   - With one Author neither card draws a picker.

2. **The Repos pane shows and changes the Author.** Each row carries the
   display name under the two-Authors condition, with a change control; the
   server refuses a repoint while a Conversation on the Repo is neither Done
   nor Closed, naming it.
   - A repoint on a Repo with a live Conversation is refused and the row says
     which Conversation.
   - A repoint on an idle Repo lands and the next session on it commits as the
     new Author.

3. **Removal and the vanished Author.** Remove on the Git section's switcher,
   refused while a registered Repo points at the Author; a start on a Repo
   whose uuid is in no Author refuses by name and halts with a Notice unpressed.
   - Removing an Author two Repos point at is refused naming them;
     unregistering both and removing again succeeds.
   - Deleting an Author's entry from `config.yaml` by hand turns a Start on its
     Repo into a refusal naming the Repo and the Repos pane.

4. **Companions of another Author are refused.** Adding a companion whose
   Author differs from the Conversation's Repo's is refused on the composer in
   both modes, naming the companion and both Authors; a Stage inheriting its
   predecessor's set is unaffected, the set having been admitted once.
   - The composer's add is refused with the sentence; the same Repo under the
     same Author is admitted.

5. **Server-side git and shares as the Author.** Every fetch and push
   Verkstead runs on a Repo goes through the credential helper with that Repo's
   Author's token, an Author with no token leaving the host's credentials
   standing; and a share is published and linked as the Conversation's Repo's
   Author, refused where that Author has no token.
   - A fetch on a Repo whose Author has a token is proven, with a stand-in
     remote, to present that token and no other.
   - A Repo whose Author has no token fetches as the host does today.
   - Create repo's `gh repo create` and first push run as the picked Author.
   - Publishing a share of a Conversation on a Repo under the second Author
     makes the gist, commits into it and comments its link with that Author's
     token and name, proven with a stand-in `gh` and remote; publishing one on
     a Repo whose Author has no token is refused as a missing token is today.

6. **The Brief names the Author, and the email probe.** The Brief's record draws
   the display name beside the Repo under the two-Authors condition; the probe
   runs and its finding lands either as a strip in the root builder or as an
   amendment to ADR-0025.
   - The Brief of a Conversation on a Repo under the second Author names it;
     with one Author configured it does not.
   - The probe's answer is recorded in the ADR with the Claude version it was
     taken against.

## Re-verify at start

- Assumes 01 landed: Authors in the files keyed by uuid, the store's Repo
  association, the Git section's switcher, and the sandbox and `gh` reading by
  the Repo's Author.
- Assumes the server's own fetch still runs a bare `git fetch --all` with no
  credential helper, and that publishing still has a `git` helper that hands a
  token through `credential.helper` on `GH_TOKEN` — the function the fetches and
  pushes move onto.
- Assumes publishing a share still reads the one token and the one git author
  itself, on its blocking hop, and hands both to the gist's create, commit and
  push and the token to the comment after it — the reads that move onto the
  Conversation's Repo's Author.
- Assumes companions are added through one endpoint on the composer that
  already refuses the Conversation's own Repo and a Repo added twice; the
  Author check sits beside those two.
- Assumes the Repos pane is drawn by `web/src/repos/RepoList.tsx` with
  `OpenRepo`, `CreateRepo` and `RepoRegistration` beside it, and that the Repo
  view already carries how many live Conversations a Repo has.
- Assumes the Claude root still copies `.claude.json` with `mcpServers` and
  `projects` taken out and `oauthAccount` kept, and that there is a Claude on
  the machine the probe can be run against.
- Check whether a Stage's inherited companion set can reach a Repo repointed
  since the roadmap was grilled; if so, the inheriting cut refuses by name.
