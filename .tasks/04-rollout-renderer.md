# 04. The rollout renderer

## What to build

Codex's rollout drawn as a conversation in the details pane, and counted as one
by the Timeline row.

A rollout line is a wrapper — a timestamp, an ordinal and a `type` — around a
payload. The kinds seen on a real one are `session_meta`, `event_msg`,
`response_item`, `turn_context` and `world_state`, with `compacted` for a
session that ran long enough to be compacted. Codex adds kinds without
announcing them (`world_state` was not in this stage's brief), so the fall-back
matters as much as the list.

**A rollout writes the same turn down twice, and only one of them is the
conversation.** `response_item` lines are what the model was sent — the whole
developer preamble and the environment block among them — and `event_msg`'s
completed items are what the TUI actually drew. **The drawn conversation is
what the pane draws**: it is what the human would have seen, and rendering both
would double every turn while putting pages of injected prompt at the top of
every Transcript. The other stream folds into bookkeeping under its own name,
where nothing is hidden and it opens if somebody wants it.

Everything that is not the conversation is bookkeeping under the name the log
gave it, and a kind nobody has heard of folds the same way rather than being
dropped — ADR-0006's rule, and the reason a codex that adds a kind mid-week
costs a fold rather than a hole.

The count the Timeline row shows and the turns the pane draws come off the one
reading, as Claude's do: a second way of counting would be a second definition
of what a turn is. The same reading is what a stop Notice's evidence quotes and
what the row's last-said comes from, so a Codex session's row says what a
Claude session's row says.

A Claude Transcript draws exactly as it does today. Which reader a Transcript
gets is decided by what the lines are rather than by anything a caller has to
be told, since the same lines are rendered in three places and none of them
carries the agent type today.

## Acceptance criteria

- [ ] A real rollout draws as a conversation — the agent's prose, its
      reasoning, the tools it called and what was put to it — with the injected
      preamble and the raw model stream folded away rather than shown as turns.
- [ ] A line kind nobody here has heard of folds into bookkeeping under its own
      name, and neither the pane nor the count drops it.
- [ ] The Timeline row's turn count and last-said come off the same reading the
      pane draws, and a Claude Transcript draws exactly as it does today.
