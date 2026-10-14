# 06. The Guide and the skills

## What to build

Teaching agents the command and when to use it.

**The Guide.** A section of its own in the core Guide — *Leaving a memo for the
sessions after you* — placed before *Ending the session*, since a memo is
written during the work and the signal ends it. It says what a memo is for in
the words of the **Memo** entry in `CONTEXT.md` (working knowledge no artifact
holds; not a running account of the work, which the commits are; not the
design, which the Brief and the Handoff are), which sessions may leave one and
that the rest are refused by name, the two invocations with a quoted heredoc
the way the ask is shown, the id printed on stdout, the 8 KiB bound, and the
prompt shape task 03 settled so an agent recognises what it is reading. The
Guide's CLI contract section quotes the subcommand's help verbatim, as it does
the others'. The Guide prints the section on every backend, and the Guide test
holds it to that and to the contract being this binary's.

**The skills.** A short section apiece in the three skills whose sessions have
a next one: the next-task skill (the task and the finish), the implementing
skill (the inline run, whose wrap-up and follow-up read memos too) and the
next-stage skill (the stage's planning). Each says, in a paragraph: leave a
memo when you learn something the next session would otherwise find again or
miss — a flaky suite, a trap in the build, a shape you settled that the task
did not ask about — and leave none for what the commit message already says
or what the backlog is about to say; retire one you find no longer holds. The
grilling, reviewing, addressing, responding, submitting, following-up and
investigating skills say nothing of it: their sessions are refused, and a
skill that mentioned a command its session cannot run would be sending it to a
refusal.

## Acceptance criteria

- [ ] `verkstead guide` prints the memo section on every backend, and the
      contract it quotes for `verkstead memo` is the binary's own help.
- [ ] The next-task, implementing and next-stage skills each carry the section,
      naming both invocations; no other skill mentions the command.
- [ ] The Guide's wording of what a memo is for matches the glossary's.
