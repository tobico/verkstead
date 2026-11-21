# 06. `verkstead pull-request open` and forge-neutral skills

## What to build

**The subcommand.** `verkstead pull-request open [--title …] [--body …]
[--base <branch>]` asks the server, which asks the Repo's Forge as the Author
to open a draft pull request. The server records it on the Conversation the
way `wrapping.rs` records one it finds. It prints `url: https://…` as YAML on
stdout and nothing else.

`--base` defaults to the base on the Conversation's record. It is refused, with
a non-zero exit and the reason on stderr, where:
- the branch is not pushed;
- the Author holds no token for this Forge;
- the Repo has no Forge;
- the Forge lacks `create`.

Document it in `verkstead guide` and the CLI help.

**Forge-neutral skills**, in the pack at `plugins/verkstead/skills` (nine
mention `gh` today):
- `gh pr create --draft` becomes the subcommand: implementing, submitting,
  staging, next-task;
- `gh pr diff` becomes `git diff` against the base the prompt names:
  reviewing, responding, following-up;
- `gh pr checks` goes, the prompt carrying what failed: reviewing, addressing;
- `gh pr view --json number` goes, the record being on the prompt:
  following-up, submitting;
- the investigating skill forbids the subcommand instead of `gh pr create`;
- `gh stack` lines stay, written as conditional on the prompt saying the Forge
  has `stacks`, and the prompts say so.

The Fix Merge Issues stack prompt already tells the session to sync. Reword it
forge-neutral: the Forge's name, not *GitHub*. Update the skills' own tests in
`skills.rs` to match. `docs/agents/git-workflow.md` is this repository's own
process and stays in `gh`.

## Acceptance criteria

- [ ] A Tinker whose session runs `verkstead pull-request open` lands in
      Wrapping with the pull request recorded. Each refusal exits non-zero
      with its reason.
- [ ] `grep -w gh plugins/verkstead/skills` finds only `gh stack` lines guarded
      by the `stacks` capability.
- [ ] The Fix Merge Issues prompt for a stack tells the session to sync,
      naming the Forge rather than GitHub.
