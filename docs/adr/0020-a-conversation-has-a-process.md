# A Conversation has a Process

Amends [ADR-0008](0008-pick-informs-artifacts-move.md) and
[ADR-0010](0010-one-stop-and-steer.md), and revises a decision recorded in the
design document on 2026-09-08: that a review-only mode "was not worth a state of
its own". It is worth a Process.

Every Conversation ran the one ladder — Draft, Grilling, Implementing, Wrapping,
Done — and everything that was not that ladder was a way of skipping rungs: *No
grilling* on the Grilling picker took a Brief straight to an inline build,
*Wrap up a pull request* under Other actions took one straight to Wrapping, and
a **Steer** into Follow-up was the only way to have a session simply answer
questions about work. Each was a different control in a different place, and
none of them was a thing the human could name. What was wanted was to say up
front what kind of work a Conversation is for, and have the ladder follow.

So a Conversation has a **Process**, picked on the composer beside the Repo and
frozen when the work starts, exactly as the Pairings are. Five of them in the
first version: **Develop**, **Investigate**, **Review**, **Tinker** and **Fix
Merge Issues**. Every one starts from a Brief, and every one cuts a branch and a
Worktree the way a grill start does, so the Sandbox and the companions are the
same whichever it is. What differs is where the Brief goes from there.

Decided in the grilling of 2026-09-25. The roadmap that builds it is
`docs/roadmaps/processes/`.

## Process beside Direction, not in place of it

**Direction** stays what it is: how Develop's work gets built, picked on the
grilling's closing Set. A Process is picked before anything runs and says which
states the work passes through at all; a Direction is picked inside one of them
and says how the building rung is run. Folding the two into one pick was
considered and rejected: a Direction is the outcome of an interview and cannot
be known before it, and a Process has to be known before the first session is
launched, because it decides which session that is.

Stored the way Direction is — a 1:1 side table keyed by the Conversation, the
`conversations` table left alone — and read for Conversations from before this
as **Develop**, except one that adopted a pull request, which reads as
**Review**, that being the Process its path already was.

## The five, and what each does

**Develop** is today's ladder, with one change: the interview always runs. The
*No grilling* row is retired from the Grilling picker, because skipping the
interview is now a different Process rather than a hole in this one. A skip a
Repo remembers for the role is not applied, exactly as a remembered Pairing
whose Profile has broken is not.

**Investigate** answers questions about the code without changing it. Draft to
a new state, **Investigating**, to Done. One session under the Implementation
Pairing, shaped like a follow-up — rounds of Question Sets, the human's
**Nothing else** mark ending it together with the Done signal — in a writable
Worktree so it can write and run code to find things out, and told to commit
none of it. The Done signal is accepted over uncommitted changes here and
nowhere else: the scratch is the point, the Diff on every Set already shows it,
and the Worktree goes with the close. Which holds for an investigation that ends
Done and not for one that hands its checkout back to a state something runs in:
the sessions a wrap-up or a follow-up dispatches stage everything they find, so
the ending takes the investigation's scratch back out and leaves the checkout as
the steer found it. What the steer found is written down beside it, because a
path the human had already left uncommitted is theirs and only what the
investigation added may go. No pull request is ever asked for. The
session is still asked to rename the branch, because the branch is the
Conversation's title and a title is worth a cheap rename.

Investigating is a Steer target from every state, taking a brief the way
Follow-up does, because a question about work can arise at any point in it — and
**a steered Investigating goes back to the state it was steered from**, the way
a follow-up lands back in the wrap-up it came off. Done is the ending of an
Investigate Conversation, which came from a Draft; making it the ending of every
Investigating was considered and rejected, because a Wrapping Conversation
steered into one to ask a question would come out of it Done, its pull request
unmerged and its watchers off it. Draft, Closed and Investigating are the three
states nothing returns to, so an Investigating steered out of any of them ends
Done as well: the first two have a way in of their own that nothing else may
use, and an investigation sent back to Investigating is one the Nothing-else
mark could never end, the move that lands it there opening a fresh window the
tick falls outside of. Which state to go back to is written down when the steer
is made rather than read off the Timeline: it is a fact of the steer, not
something to infer from history.

**Review** is *Wrap up a pull request* made a Process. Draft to Wrapping, the
ordinary wrap-up with its review, on the pull request or branch it is pointed
at. The pull request's own title and body no longer stand in for the Brief; the
human writes one, and names the target in it or in the Target field. Two roles,
Implementation and Review, and no *No review* row: a Review without a review is
Fix Merge Issues with the comments answered, which is a different thing to ask
for. The *Wrap up a pull request* level under Other actions goes, and with it
the list of open pull requests read off GitHub when the compose page opens; the
Brief and the Target field carry the target instead.

**Tinker** is the ungrilled path made interactive. Draft straight to
Follow-up on a fresh branch, under the following-up skill primed with the
Brief, for as many rounds as the human wants. When they mark Nothing else and
the session signals, commits on the branch mean Wrapping — `submitting` opens
the pull request and the ordinary wrap-up runs, reviewed where a Review Pairing
was picked — and none mean Done. Tinker is also what replaces *No grilling*:
the ungrilled inline build, its *Nothing was grilled* prompt and its start path
retire. A sixth Process holding that path under a name was considered and
rejected in favour of this — a build with no interview is one the human wants
to steer by hand, and the follow-up's rounds are how that is done.

**Fix Merge Issues** is a wrap-up narrowed to the two things GitHub can refuse a
merge for. Draft to Wrapping with Review and Comments pre-settled, exactly as
the **Resolve conflicts** press pre-settles the review, so only **Mergeable**
and the checks are waited on and no review or `responding` session ever runs.
One role, Implementation. It takes a stack: the server walks the chain on
GitHub from what the Brief names, both ways — a base that is another open pull
request's head, a head that is another's base — records every pull request on
the Conversation, and dispatches one `addressing` session told the whole
ordered list, bottom up, because a fix low in a stack changes everything above
it. A stack is synced with `gh stack sync` whatever the configured resolution
strategy says, a stack being gh-stack's and a merge into each branch of one
being what the strategy's own documentation warns against; a lone pull request
follows the strategy as it always has.

## Naming a pull request or a branch

Review and Fix Merge Issues need a target. A pull request URL or `#number` is
unambiguous anywhere in prose, so the Brief is scanned for the first one. A
bare branch name is not, so it goes in a **Target** field of its own in the Repo
panel, drawn for these two Processes and reading **Pull request or branch**: it
takes a URL, a `#number` or a branch, and a URL found in the Brief fills it
while it is empty and never overwrites what the human typed. Start is refused
while neither names anything. A bare branch keeps the base picker, and that base
is what the wrap-up's `submitting` step opens the pull request against; a pull
request hides it, GitHub's base being the fact. Reading the Brief's first line
as the target, and a field with nothing read from the Brief, were both
considered: the first makes the Brief's shape load-bearing, and the second makes
the human type a URL twice.

**A field of its own rather than the Branch field re-read**, which was the first
shape and does not work. The Branch field is a rename: it saves through
`POST /api/ui/conversations/<id>/branch`, which runs
`git check-ref-format refs/heads/<value>` and answers `NotABranchName`, and git
refuses a pull request URL over its colon — so the shape this section leads
with, a URL out of the Brief filling the field, is the one value that field
cannot hold, on the composer and in the compose page's create replay alike.
Underneath that the two are not the same fact: the Branch field says what the
Conversation's branch is called, and take-up decides that from the pull
request's head. The Target field says which work to take up, and is read once,
at Start.

## The composer: Repo, Process, Agent

The setup row reads **Repo**, **Process**, **Agent**. The three role pickers
become one control whose shape is the Process's: where the Process has one
role, today's dropdown labelled *Agent*; where it has several, a panel dropping
from the trigger the way the Repo panel does, holding the role pickers stacked
under their labels. The trigger reads the Implementation Pairing's reading,
then ` +1` for each other role picked onto a different Pairing — a role skipped
or matching adds nothing — and *Not chosen* while a required role is empty. A
modal was considered and rejected: the Repo panel is the pattern beside it.

| Process | Roles | Control |
| --- | --- | --- |
| Develop | Grilling, Implementation, Review (or none) | panel |
| Investigate | Implementation | dropdown |
| Review | Implementation, Review | panel |
| Tinker | Implementation, Review (or none) | panel |
| Fix Merge Issues | Implementation | dropdown |

Develop is the default on every new draft and is not remembered per Repo: the
Pairings are remembered because they are the same answer most of the time, and
a Process is the one thing about a Conversation most likely to differ from the
last. The per-Repo Pairing memory stays keyed by role and shared across
Processes.

The Process is drawn on the Brief's setup facts and nowhere else: the state word
on the sidebar and the card already says where the work is. Beside the Repo, the
branch and the base rather than beside the Pairings (*amended 2026-09-25,
planning stage 01*): those facts come in two halves, and the half holding the
worktree path and the three Pairings is the half a published share does not
draw. A Process is a fact about the work rather than about the machine it was
worked on, so it belongs in the half a share says.

## Consequences

- The ladder is no longer the one shape. `resume`, the drivers and the Steer
  form each key off the state, and every new shape has to be recomputable from
  the record alone.
- **Investigating** is a Lifecycle state, with its own row in Steer, its own
  ending rules and its own resume — and the first state whose ending depends on
  where it was entered from, which is why a steer now records the state it left.
- The Done signal's evidence table gains a kind: the Nothing-else mark, with
  uncommitted changes allowed, for an Investigate session.
- Follow-up is no longer the one state with no way in but a Steer: Tinker
  starts in it. Its ending rule gains a branch: no pull request and no commits
  is Done.
- The wrap-up learns about stacks, which it never has: a Conversation can hold
  several pull requests in one repository, and one session can be sent at all
  of them. Which is a schema change — everything about a pull request is keyed
  one per repository, inline, so five tables rebuild through `migrations.rs` —
  and so it is a stage after the narrowed wrap-up rather than part of it.
- Three things retire: the *No grilling* row and the ungrilled start path, the
  *Wrap up a pull request* level and the open-pull-request list behind it, and
  the pull request's title and body standing as a Brief.
- The Process picker offers a Process only once its stage has landed, as an
  agent type is offered only once it can launch the real thing.
