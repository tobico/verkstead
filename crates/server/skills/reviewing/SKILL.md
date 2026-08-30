---
name: reviewing
description: Review work that is already on a pull request — one, or one per repository it reached — propose the fixes it and its comments need as one Question Set, and land the ones the human accepts. Use when a session has been dispatched to review work at the end of a wrap-up.
---

Review the work this Conversation carried to a pull request, put what you find
to the human, and fix what they accept.
**You propose, and then you fix what was agreed to.** Nothing you find is
changed before they have said so, and everything they say yes to is changed here
rather than by somebody else afterwards.

You are the first thing to see this work whole. The sessions that wrote it each
saw one task and none of them saw the branch, and you have none of their
context — which is the point. Read it as somebody who has to live with it.

**The work may be on more than one pull request.** A Conversation that committed
in a companion repository ends on one there too, and the whole of the work is all
of them. Where your prompt has a **The pull requests this work is on** section,
it names each — the repository, the number, the URL, and the worktree that
repository's branch is checked out in. Read each of them where it lives: `cd`
into that worktree first, because `git` and `gh` both read their repository from
wherever they are run, so a `gh pr diff` from the wrong directory reads somebody
else's pull request. Where your prompt lists none, the branch in the worktree
you started in is the whole of the work.

They are one piece of work however many repositories they are spread across, and
that is why one session reads them: a change in a companion and the change here
that needed it are one thing to judge, and they go into one Set together.

What you propose about is the work **and** whatever has already been said on its
pull requests. Nobody else is sent to act on those comments: what they ask for
goes into your Set beside what you found yourself, so the human decides about
their own words rather than watching a session act on them unasked.

Where something you find is genuinely too big to fix in this sitting, you can
offer to split it out into a backlog for sessions of their own instead — see
step 5. That is the exception and not the shape of the job: the ordinary review
is a handful of fixes, made here.

Every branch of it is already pushed and already has an open pull request. There
is nothing to create, nothing to switch to, and nothing to open.

## 1. Read the work

Read the whole diff first, before you form an opinion about any part of it — and
where the work is on more than one pull request, all of them, each read in the
worktree its own repository is checked out in:

    cd <the worktree that pull request is in>
    gh pr diff

Then go and read what it landed in. A diff shows what changed and hides what it
changed *around* — the callers, the sibling module doing the same job a
different way, the test that should have caught this and does not. Where the work
is spread across repositories, that reading crosses them: the caller of what
changed in a companion may be what changed here.

Read what each repository says about itself, too: its `CLAUDE.md` or
`AGENTS.md`, the docs it keeps for agents, the conventions its neighbouring code
actually follows. The work is meant to look like it belongs where it landed, and
what belongs in one repository is not what belongs in the next — judge each half
by its own repository's conventions rather than by the one you started in.

The Brief and the handoff in your prompt say what the work was *for*. Review
against those rather than against what you would have built.

## 2. Read what has already been said

Where anything had been said on the pull requests before you started, it is under
**What has been said on the pull request** at the end of your prompt: the
comments whole, in the order they were said in, and where each of them was said.
Every comment names the pull request it was left on, and one left on a line of
the diff carries its file and line beside it — both halves are what it means.

Read it as what it is — somebody who has read this work, telling whoever wrote
it what they think — and go and look at what each one is about before you decide
what it is asking for. Which is in the worktree of the repository whose pull
request it was left on: the file it names is that repository's.

**You are the only session that will act on these.** Nothing else is dispatched
about them, so a comment you leave out is a comment nobody answers. Work out what
each is asking for and carry it into your Set with everything else you found: one
Question, in your own words, saying what you would do about it. Some ask for a
change, some are a question you can answer in the Question's own text, and some
are somebody saying they are happy — that last is nothing to propose, and
inventing work out of agreement is not answering it.

What you must not do is act on one because it is the human talking. A comment is
still a proposal until they have said yes to *this* reading of it: the words on a
pull request are not the same thing as an instruction to a session, and the
answer to "this is the wrong way round" is a decision they have not made yet.

## 3. What is worth raising

The seams are where this session earns its context. A session per task cannot
see across tasks, so look hardest at what only shows up from here:

- **Correctness** — the bug, the unhandled case, the thing that is wrong.
- **Seams between the pieces** — two tasks that solved the same problem twice,
  an abstraction introduced by one and ignored by the next, a half-done rename.
- **Seams between the repositories**, where the work is on more than one — the
  half that changed here and the half that changed in a companion disagreeing: a
  caller passing what the callee no longer takes, a rename one side made and the
  other did not. You are the only session that ever sees both, so this is the
  seam nobody else could have found.
- **Drift from what was settled** — where the branch quietly decided something
  the handoff had already decided differently.
- **Tests that do not test** — the assertion that passes whatever the code does,
  the case nobody covered.
- **What is now stale** — docs, comments and names that describe the code as it
  was before this branch.

Raise what is worth a human's decision, and nothing else. Style you would have
done differently, a name you would have picked, a refactor nobody asked for —
these cost a human a decision each and buy nothing. **If you would not defend it
in a review, do not raise it.**

## 4. Change nothing yet

No edits, no commits, no pushes, no `gh` command that writes anything — not
until the human has answered. Fixing your own findings before they have seen
them is deciding in their place, and the whole point of the Set is that the
decision is theirs.

## 5. Propose it as one Question Set

One Set, one Question per finding, and nothing beside them: what a review sends
is an ordinary Question Set, the same shape as every other ask.

```yaml
title: Review of the rate limiter branch
preface: |
  Three things worth a decision. Everything else looks right to me.

  Each one lists the ways I would fix it — pick the way you want and I do it
  that way here, before I push. **Leave it** and I will not raise it again.
  Anything you write beside an answer is part of what I do about it.
questions:
  - label: Q1
    text: |
      The window counter is never reset between windows, so a client that goes
      quiet for an hour is still refused. `window.rs` counts from the first
      request and nothing clears it.
    options:
      - n: 1
        text: Fix it — reset the counter as the window rolls
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      `limits.rs` and `window.rs` each grew their own clock. Two now, and the
      tests pin both.
    options:
      - n: 1
        text: Fix it — collapse them onto `window.rs`'s clock
        recommended: true
      - n: 2
        text: Fix it — inject one clock at construction instead
      - n: 3
        text: Leave it
```

- **One Question per finding**, and every one of them answerable.
- **Each credible way to fix it is an Option of its own**, worded in your own
  words: the Option says *which* fix it is, because that is what they are
  picking between. Offer alternatives wherever more than one credible way
  exists, and only there — a finding with one sensible fix carries one fix
  Option, and an alternative invented to fill the Question out is a way you
  would not defend, which costs them a read and buys nothing.
- **Leave it is always offered**, on every finding, so declining stays possible
  whatever else the Question puts.
- **Recommend the one you would take** — the way you would fix it, or leave it
  where that is your answer. One star per Question.
- **The Question is what the human reads**, on a phone, deciding. Write it as
  prose that says what is wrong and why it matters — not as a patch. Where the
  ways differ along more than one axis, the guide's `columns` and `cells` draw
  them as a comparison table rather than having them read the comparison twice.
- **Nothing else goes on the Set.** There is no findings block and no marker
  saying which Option means fix it: Verkstead reads how your session ended and
  what it left on the branch, rather than a record of what you were answered.
  Which makes the answers yours alone to act on — nothing else holds an account
  of what you found, so keep your own reading of each finding to hand, the file
  and the cause and what *done* would look like, for when they arrive.
- **A comment's fix is a finding like any other**, in the same Set. Say in the
  Question which comment it answers and whose it was, so the human can see their
  own words being taken up — their comment is where it came from and not what it
  says to do.
- **One Set for the whole of the work**, however many repositories it reached.
  Every finding goes in it, and a finding about a companion
  **names the repository it is about** in the Question's own words, so what they
  are picking between says plainly what would change and where. Findings about
  the repository you started in need no such label: unlabelled means the work's
  own repository, and saying so on every Question would cost a read and buy
  nothing.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found. A review that has found more than
  a sitting's worth of decisions is a review that should raise the ones that
  matter.

### When a finding is too big for this session

Everything above assumes what you found can be fixed between the human's answer
and your push, which is nearly always true. Where one genuinely cannot — a
rewrite that wants breaking into steps, a change whose blast radius is the
branch over again — offer splitting it out as an Option of its own, beside the
ways to fix it and the leave-it:

```yaml
  - label: Q3
    text: |
      The clock abstraction wants rebuilding rather than patching. Three
      modules hold their own notion of now, and untangling them touches most
      of the tests in the crate — more than I can do without leaving the
      branch half-migrated.
    options:
      - n: 1
        text: Fix it here anyway — one clock, injected at construction
      - n: 2
        text: Split it out as its own work
        recommended: true
      - n: 3
        text: Leave it
```

- **Offer it rarely, and never by default.** A Set that put the choice on every
  finding would be asking the human to plan the work as well as decide it, and
  what this whole phase is for is keeping the ordinary handful of fixes in one
  session. Most reviews offer no split at all and are not the poorer for it: if
  you could do it today, it is a fix.
- **Every Option means something** — fixed here this way, split out, left — and
  the human decides per finding. Recommend the one you would take.
- **Picked, it is step 8's backlog**: a task file written here and committed
  here, and none of the work done in this session. That commit is the whole of
  what says it was split out, because Verkstead reads the branch rather than the
  answers — a split nothing was written for is a finding nobody will ever work.
- **A split is not a way out of deciding.** Something you have not thought
  through is not a task to hand on, and something you simply do not fancy is a
  fix. Offer it where the work is too big and nowhere else.

Then put it through `verkstead ask`, run the way the Guide says to run one on
this backend: they answer in their own time, and that may be hours — they are on
a phone rather than at this terminal.

**The answers are yours to wait for, whichever way the Guide says to wait.**
Waiting is the ask working rather than the ask failing. Nothing ends this
session when the Set lands and nobody else is dispatched to act on it: what
becomes of your findings happens here, whether that means holding the ask open
or ending the turn and being told when they land. So there is nothing to do in
the meantime. Do not start on what you have only proposed, and do not take your
own recommendations.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never decide on their behalf.

## 6. Fix what they accepted

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it.

- **A finding is accepted where they picked one of its fix Options**, and *which*
  one they picked is part of the answer: fix it the way they chose rather than
  the way you would have. Anything else is not a yes — leave it, an answer in
  their own words instead of a pick, a question left open.
- **What they wrote beside a yes is part of the instruction.** "Yes, but leave
  the public signature alone" changes what you do, and it is the reason their
  words come back to you at all.
- **A finding they declined is over.** Do not fix it, do not fix half of it, and
  do not raise it again.
- **A finding they split out is neither.** It is not fixed here and it is not
  forgotten: what you owe it is step 8's backlog, and starting on it here is
  doing the thing they said not to do in this session.
- **Unanswered is not a yes.** Leave it as declined. Where it is one you
  genuinely cannot leave — the correctness bug the rest of the branch turns on —
  go back with one short Set about that alone and wait as before.

Fix each accepted finding as what it is: the cause rather than the symptom, and
nothing beside what they agreed to. Anything else you notice on the way is a
finding you did not raise, and fixing it now is a decision they did not get to
make. Fix it where it lives — in the worktree of the repository the finding is
about — and then run that repository's tests and make sure what you did works
before it goes anywhere.

## 7. Fix whatever the checks have gone red on

You have held this Worktree since before you asked, and you hold it until you
end: the ask that blocked for hours is a session working rather than a Worktree
free. So a check that went red while you waited has had nobody sent to it —
nothing is dispatched into a Worktree something is already working in. It is
yours, the way the comments are. Every pull request's checks, not just the one
you started in: none of them had anybody sent to it either.

Once the answers are in, ask each pull request how its checks are getting on, in
the worktree that one lives in:

    cd <the worktree that pull request is in>
    gh pr checks

Asked where it lives, because `gh` reads its repository from wherever it is run:
a `gh pr checks` from the wrong directory answers about somebody else's suite,
which is the one report that looks exactly like the one you wanted.

Whatever is failing, fix it there, alongside the findings they accepted and
before you push. Go and read what the check actually complained about — its run
is linked from that output, and the repository says how to run the failing thing
yourself. A fix written from the name of a job is a guess.

- **There is nothing to propose about a red check.** It is the branch being
  broken rather than a decision the human has to make: it is not a finding, it
  does not go into a Set, and it is fixed whether or not they accepted anything
  else.
- **A check still running is nothing to do.** Your push starts the suite over
  anyway, and what is watching the checks reads it then.
- **The cause rather than the symptom**, and nothing beside it — the same rule
  the findings are fixed under. A check turned off is not a check fixed.
- **A red check you cannot fix is worth saying so about** rather than pushing
  over in silence. Fix what you can, and say which check beat you and what you
  found: the watcher has its own goes at it once you are done, and your account
  of it is what the human reads on the Timeline.
- **Its own commit**, like each finding's, so it reads against the check it was
  for.

## 8. Write down anything they split out

A finding they answered by splitting it out is not work for this session. What
you owe it is a `.tasks/` backlog — written here, committed here, and worked by
nobody in this Worktree today.

**In the worktree you started in**, whichever repository the finding is about.
That is the Conversation's own, and it is the one place Verkstead reads a backlog
off: a list written in a companion's worktree is one nothing will ever find, and
the work you split out would be waited for by nobody. Say in the task file which
repository the work is in, where it is not the one the list is sitting in — the
session that works it starts where you did and is checked out beside every
companion, exactly as you are.

The branch's own backlog is finished with by the time you are reading it, so
what you write is a fresh one:

- **`TODO.md` first**: a heading naming the work, a paragraph saying what it is
  for, and one `- [ ] NN: <title> — [details](NN-<slug>.md)` entry per split
  finding, in the order they should be worked.
- **One `NN-<slug>.md` task file per finding**, numbered from `01`. Each carries
  your own account of that finding — the file, the cause and what *done* would
  look like — whatever the human wrote beside their pick, and acceptance
  criteria that say when it is done. Write it for a session with none of your
  context that will never speak to you, because that is what works it: your
  reading of the finding lives nowhere else once this session ends.
- **Nothing else goes in.** What they accepted is fixed above and what they
  declined is over: a backlog of anything but what they split out is work nobody
  agreed to.
- **Do not build any of it.** Writing the list is the whole of the job here, and
  a task you fixed on the way is a task the session sent to do it will find
  already done.
- **Its own commit**, and a bookkeeping one — a backlog commit carries no
  summary, the way a plan commit does not.

What follows is Verkstead's. It reads the backlog off the branch, sends this
Conversation back to be built, and works the list a session at a time; the finish
that follows the last task wraps the work up again on the pull requests it
already had, and a fresh review reads the whole of it then. So there is nothing
here to hand over and nobody to hand it to — the task files are the handover.

If they split nothing out, there is nothing to write. Leave no `.tasks/` behind:
an empty backlog is a run Verkstead would start and find nothing in.

## 9. Commit it and push it

**Nothing waits on approval.** The approval was their Response, and there is
nobody at this terminal to ask for a second one.

One commit per finding, so each reads against the decision that asked for it, in
the worktree that finding's fix was made in:

    git add -A
    git commit -m "fix: <the finding, and what you did about it>"

Then push each worktree you committed in, once, when the last of its commits is
in — the backlog's commit included, if you wrote one:

    git push

Every one of them: a repository you fixed something in and did not push is a
decision the human made and nobody can see. `git` reads its repository from
wherever it is run, so a push from the wrong directory pushes the wrong branch —
and a repository you committed nothing in has nothing to push, which is most of
them on most reviews.

Push, unlike most sessions here: these branches are already on pull requests, and
a fix that stays local is one nobody can see and nothing re-runs. The push is
what puts the commits in front of the checks again.

### What the message body says

A commit that delivers work — code, tests or documentation the work asked for —
carries a summary as its message body. That body is what the workbench shows
beside the diff when the human reviews this branch later, so it is written for
the reviewer who reads it before reading the patch. Pure bookkeeping carries
none: a plan or backlog commit, a roadmap commit, the finish commit, an ADR
recorded along the way. A commit still counts as delivering work when the list's
tick rides along with the code.

- **The prose first** — what you built and how it hangs together.
- **The diagram after it**, whenever the diff is more than three changed lines.
  The words are what the reviewer reads and the picture is what they check them
  against, so it sits under the prose and above the trailers. Diagram the
  delta rather than the system: the parts this change touches and the
  relationships between them, and nothing else. Tag each node `new`, `modified`
  or `removed` — the workbench colours those from the diff's own added and
  removed shades, so the picture and the patch read as one account of the
  change. Around ten nodes, so that it reads on a phone.

Trailers go at the end as usual; the workbench takes them off what it shows.

    feat: share the rate limiter's count between instances

    The counter moves out of the process, so every instance counts against the
    same window, and the in-process throttle it replaces goes away.

    ```mermaid
    flowchart LR
      api[POST /v1/messages] --> limiter[Rate limiter]
      limiter --> counter[(Redis counter)]
      api --> throttle[In-process throttle]

      class limiter,counter new
      class api modified
      class throttle removed
    ```

Do not open a pull request, do not merge anything, and do not touch any branch
beyond the ones you were sent to: the branches checked out in the worktrees your
prompt named, or the one you started in where it named none. Every other branch —
in these repositories and in every companion beside them — belongs to somebody
else's piece of work. The pull requests exist, and merging is the human's act.

Then say what you fixed, what you split out and what you left, and where each of
it was, and stop.

## 10. A review with nothing to raise

Nothing to raise means both halves: you found nothing yourself, *and* nothing
said on the pull requests asks for anything. Comments you were given are the
other source of a decision here — work you would not have touched, where somebody
has asked for a change, is a review that proposes about that change alone.

Ask nothing where there is genuinely neither.
**Say plainly, as the last thing you print, that you reviewed the work and
found nothing worth raising** — that line is what the human sees on the
Timeline — and stop. Say which pull requests you read, where there was more than
one, and that you read what was said on them too where there was anything to
read: it is the only report that any of it was looked at.

A Set with no findings in it is a row for them to dismiss, and the point of this
phase is to spend their attention only where there is a decision. Finding
nothing is a fine outcome; inventing a finding so that something happened is
not.

The same holds at the other end: a review whose every finding was declined has
nothing of its own to commit, and committing nothing is the right end to it. Say
what you raised and what they left, and stop.

The checks are the one exception, at both ends. A review that asked nothing was
never away long enough for one to go red behind it, and hands the Worktree
straight back to whatever watches them. A review that waited did: a check that
went red while it waited is yours whatever they decided about your findings, so
step 7 still runs and what it fixes is still pushed.
