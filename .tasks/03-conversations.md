# 03. Conversations, briefs and the workbench shell

## What to build

The workbench itself, and the entity it exists to show.

**The shell** is the responsive 3-pane hierarchy: a conversations sidebar, the
timeline of the selected conversation, and a details pane for the selected
event. Responsive from the start rather than desktop-first — on a phone the
three become one pane at a time, and the same pages that answer question sets
today keep answering them. The shell takes `/`; the pending list moves to
`/pending`, and `/archive` and `/sets/:id` stay where they are. Those routes
are transitional: they retire next stage, once question sets are reached
through their conversation.

**A conversation** is attached to a registered repo and carries a brief. It is
created from the sidebar, against a repo picked from the ones task 02
registered. It holds:

- **the brief** — an editable markdown document, which is also the first event
  in the timeline, rendered inline through the existing markdown pipeline
- **a branch name**, prefilled with a random one and editable for as long as
  the conversation is still drafting
- **a base commit**, which defaults to the repo's default-branch tip and is
  overridable. It is captured at grill start, so while drafting this records
  the rule rather than a value — the conversation stores the override if one
  was made, and resolves the default when grilling begins next stage.

Nothing executes here: no branch is created, no worktree, no session. The
conversation is a record that a grilling session will later be run against,
and its lifecycle state stays Draft throughout this stage.

The timeline is a list of events with the brief as its only kind so far. Build
it as the list it will become — the next stage adds output, question set and
commit events to it — rather than as a special case for the brief.

## Acceptance criteria

- [ ] The workbench renders at `/` with all three panes, and reflows to one pane at a time on a narrow viewport
- [ ] A conversation is created against a registered repo and appears in the sidebar
- [ ] Selecting a conversation shows its timeline, with the brief as the first event, rendered inline as markdown
- [ ] Brief edits persist across a reload and a server restart
- [ ] The branch name is prefilled randomly and is editable while the conversation is drafting
- [ ] A base-commit override is recorded when set, and the default-branch rule is what applies when it is not
- [ ] `/pending`, `/archive` and `/sets/:id` still work, and the phone can still answer a set end to end
