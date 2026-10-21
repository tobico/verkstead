# 04. The agent is told about descriptions

## What to build

Task 03 made a description something the agent *can* send; this task makes
it something the agent is *told* to send, and told what stands.

**The prompt.** Every building session's prompt lists the Previews standing
under *The previews standing*, each as its title over its command. Under
each Preview that has one, list its description too, as the markdown the
agent wrote, after the command — so the next session can read what the last
one meant and keep it current when the command changes. A Preview with none
lists as today. The hint shown when none stands, and the shell line under
the list (which task 01 extended), are unchanged by this task.

**The Guide.** The *Suggesting a preview* section of `verkstead guide` shows
the heredoc with a body, says the first line is the command and what follows
a blank line is the description, says the 2 KiB limit, and says that
re-suggesting a title replaces the description along with the command. The
quoted CLI contract near the top of the Guide is the binary's own help text
and must match it verbatim, so the help changes first and the Guide's copy
follows — the Guide's tests check the two agree.

**The CLI's help.** `verkstead preview --help` says what stdin holds now: the
command on the first line, and the description after a blank line, at most
2 KiB.

**The building skills.** Each of the four skills that carries a *Suggesting a
preview* section — implementing, instruction, next-stage, next-task — shows
the heredoc with a body in its example and one sentence saying what the body
is for, in the Guide's voice and no longer than the section is now. Nothing
is said in a skill that is not a building one.

Write every string that reaches a screen or a prompt in the voice
`docs/copy.md` describes, and use the vocabulary CONTEXT.md holds: it is a
*description*, and it belongs to a *Preview*.

## Acceptance criteria

- [ ] A building session's prompt lists each standing Preview's description under its command, and a Preview with none lists exactly as before; a grilling's prompt still carries no such heading.
- [ ] The Guide's preview section documents the body, the blank line, the limit and the replacement rule, and its quoted CLI contract matches `verkstead preview --help` verbatim.
- [ ] All four building skills show the heredoc with a body.
- [ ] The sessions suite's prompt tests and the Guide's tests cover the above.
