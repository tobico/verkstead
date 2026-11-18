---
name: investigating
description: Find out what the human asked about this code — read it, write whatever probes answer the question, run them, and put what you found back in rounds of Question Sets. Nothing is committed and nothing is pushed. Use when a session has been dispatched to answer a question rather than to build anything.
---

Find out what the question at the end of this prompt asks, and keep talking to
the human about it. This is a conversation rather than a single step: they wrote
the question to you, and each round of it goes back to them as a Question Set.

You start in a worktree of the repository, on a branch of its own. There is
nothing to create, nothing to switch to and nothing to open. The branch is
yours to work in and it goes nowhere: what comes out of this session is what you
say, not what is left on it.

**Nothing you do here is committed.** No `git commit`, no `git push`, no pull
request, and nothing of what you write is meant to survive the session — which
is the one way this differs from every other session Verkstead runs. The
worktree is writable because finding things out means writing probes and running
them: a failing test that proves the bug, a script that counts the rows, a patch
applied to see what breaks. Write all of it. Leave all of it. The answer is what
is being asked for.

## 1. Read what was asked

**The question is written to this session**, in the human's own words and aimed
here. Act on what it plainly asks. It is not a remark somebody left in passing
and it is not a proposal to be put back to them — they have already decided that
this is what they want found out.

So there is nothing to propose first, and no Set to write before you start.
**Ask ahead of looking only what is genuinely ambiguous**: a question that could
be read two ways and would be answered differently each way. Everything else you
simply go and find out.

Then go and look. An answer written from memory of the code is a guess.

    git log
    git diff

Read the code each part of the question is about, and read what the repository
says about itself where the answer turns on a convention — its `CLAUDE.md` or
`AGENTS.md`, the docs it keeps for agents, what the neighbouring code actually
does.

## 2. Find out, by whatever means answers it

**Reading is rarely the whole of it.** A question about what the code does is
answered by running it; a question about whether something is a bug is answered
by a test that fails; a question about how big a change would be is answered by
starting it and seeing where it goes. Do all of that here — it is faster than
reasoning about it and it is the only thing that can be wrong out loud.

- **Write the probe.** A test, a script, a scratch binary, a patch. Put it
  wherever it belongs; nothing is going to review it.
- **Run it.** The repository's own test command, the binary, whatever the code
  is normally exercised by.
- **Say what you did.** The human reads your answer without the worktree in
  front of them, so an answer that turns on a measurement says what was
  measured and how.

**Keep to what was asked.** Anything else you notice on the way is worth a line
in the Set and nothing more: they asked a question, and an investigation that
also rewrote a module has answered a question nobody put.

**And fix nothing.** Where you find the bug, say where it is and what would fix
it — do not fix it. An investigation that lands a fix is a change nobody
reviewed, on a branch nobody is going to merge.

### Nothing is committed

    # not this, not ever, in this session
    git commit
    git push
    gh pr create

No commits, no pushes and no pull request — not at the end, not "to save the
probe", and not because the worktree is untidy. Verkstead asks for none of them
and accepts this session's ending over whatever is left lying about. Do not
touch any other branch, and do not merge anything.

The one git command this session is asked for is the rename, where your prompt
says the branch has no name yet: one `git branch -m`, and nothing else.

## 3. Put the round to the human as one Question Set

**The human sees nothing that does not arrive as a Set.** They are on a phone
rather than at this terminal, so what you print here reaches nobody: everything
you want them to read — what you found, what you did to find it, what you need
decided next — goes into the Set.

What you send is an ordinary Question Set, the same shape as every other ask.
Nothing about this session makes it a special one.

- **The findings lead.** What they asked, answered, goes in the `preface`, with
  what you did to find it out. That is the half of the round they are owed, and
  the Set is how it reaches them.
- **A decision you need is a Question**, with its Options in your own words and
  the one you would take recommended. Each credible answer is an Option of its
  own; offer alternatives only where more than one credible answer exists.
- **A question with no decision in it is still a Set.** An investigation often
  comes back with nothing to decide — the answer, and *what else would you like
  me to look at*. The answer goes in the `preface` and the invitation in the
  `postscript`, and the Set carries no Questions at all.
- **The `postscript` is an ordinary postscript** — the open-ended invitation,
  above the comment box that shares its section, and nothing that obliges a
  reply.
- **One Set for the round**, rather than one per thing you looked at.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found.

Then put it through `verkstead ask`, run the way the Guide says to run one on
this backend: they answer in their own time, and that may be hours.

**The answers are yours to wait for, whichever way the Guide says to wait.**
Waiting is the ask working rather than the ask failing, and nobody else is
dispatched to act on what they say, so there is nothing to do in the meantime.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never answer on their behalf.

## 4. Read the Response, and go round again

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it, and it is where the next thing to find out
usually arrives.

- **What they picked is what you do**, the way they picked it rather than the
  way you would have.
- **What they wrote beside an answer is part of the instruction**, and it is
  the reason their words come back to you at all.
- **A fresh question in the Response is the next round's**, read the way the
  first one was: theirs, aimed here, and gone and looked at rather than
  reasoned about.
- **Unanswered is not a yes.** Leave it.

Then round again from step 2: find it out, and put the next Set. An
investigation is as many rounds as they want it to be.

## 5. Say you are done

When a Response leaves you with nothing to find out and nothing to ask, say what
you found, and run `verkstead done`. That is all: no closing line to anybody, no
summing-up of the investigation, nothing to hand on, and nothing committed.

That command is what ends this session, and nothing else does: not going quiet,
and not the end of your turn. So a round that leaves a long test run going in
the background is not cut off while it waits.

**Whether there is anything else is the human's to say, not yours.** Verkstead
knows whether they have said it, and checks when you run the command. Where they
have not, it is refused: the command exits non-zero, says so on stderr, and the
session carries on. Then go round again from step 3 and put the next round to
them as a Set.

**It is not refused over what you left in the worktree.** Uncommitted changes
are what an investigation is made of, and Verkstead takes the signal over them.

**What becomes of this Conversation is not yours.** Do not open a pull request,
do not mark one ready, do not merge anything, and do not move the work anywhere
— Verkstead knows what comes next.

## Waiting on work in the background

Before you end a turn with work of your own still running in the background — a
build, a test run — run `verkstead waiting` with how long you expect it to take,
such as `verkstead waiting 20m`, and declare it again if the work runs over.
Otherwise Verkstead takes the session as stopped and prompts it. An ask needs no
declaration. The Guide's *Waiting in the background* section has the details.
