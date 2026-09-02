# 03. Switching a draft's repo

## What to build

Let a saved draft change which Repo it is on, from within the brief. A new
endpoint switches a Conversation's repo, shaped like the other setup
endpoints — a POST returning a named outcome rather than an HTTP error — and
refused once the Worktree exists, exactly as the other frozen-at-start
setup is. On the web side, a repo picker sits at the top of the Repo
dropdown's popover panel, listing the registered Repos; it reads disabled
once the branch has been cut.

What follows a switch, settled in the grilling:

- Companion repos are **kept**, except one that equals the newly chosen repo,
  which is dropped (a Conversation's own Repo can never be its companion).
- The base branch **resets to the new repo's default** (the rule value).
- The three Pairings and the branch name are **untouched** — a name the human
  typed survives the switch, and an invented one stays invented.

## Acceptance criteria

- [ ] A drafting Conversation switches repo from the Repo panel and the
      change is immediate and persisted; the sidebar, pane heads and Repo
      trigger all follow.
- [ ] The switch is refused with a named outcome once the worktree exists,
      and the picker reads disabled then.
- [ ] After a switch: base is the new repo's default, companions survive
      minus any equal to the new repo, pairings and branch name unchanged.
- [ ] Server tests cover the refusal and the three consequences.
