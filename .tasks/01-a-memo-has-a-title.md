# 01. A memo has a title

## What to build

A building session names the memo it leaves: `verkstead memo <title>` with the
markdown on stdin, the title being the command's one argument the way
`verkstead preview <title>` already takes its own. The title reaches the
record, the viewer and every later session's prompt. Nothing about the card or
the pane changes in this task; task 02 draws the title.

**The command.** The title is required: a call with none is refused by clap
before anything is read, so an agent following the old shape is told the new
one. `--retire <id> [reason]` is unchanged and takes no title. The CLI trims
the title and refuses it empty or with a newline in it, before sending, as a
courtesy; there is no length cap. The markdown's own refusals — empty, over
8 KiB — stand as they are. What is printed on stdout and stderr is unchanged:
the id as YAML, and the confirmation in words.

**The wire.** The memo is posted today as raw markdown in the body. It becomes
a YAML document of the title and the markdown, the shape a preview suggestion
is sent in, and the server refuses by name a title that is empty or holds a
newline, the way it refuses the markdown — the CLI's check is the courtesy and
the server's is the rule. Put the refusal rule beside the memo's existing one in
the schema crate, so both ends hold one rule.

**The record.** The title is a column on the memo's own row, beside who left
it, nullable so that every memo left before this keeps none; it is declared
through `ALTER TABLE` as other columns have been added, and it crosses on a
Transfer with the rest of the row, which the slice already carries. The memo's
Event body stays the markdown alone.

**The viewer's wire.** The Memo Event the page receives gains the title, null
for a memo left before this, and the Event id of the session's own output,
which the row already holds as `left_in` and the card has never been sent. The
press that retires or restores a memo answers with the same shape. Nothing in
the viewer draws either yet.

**The prompt.** Under *What earlier sessions left for you*, a memo is named by
its title in bold and no longer by its number: `**<title>**, left by the
session that worked task 03.` The retire line is unchanged and is the one place
the id appears — `Retire it with …` — so a session that may retire one still
knows how. A memo with no title is named `**Memo 42**` as before, there being
nothing else to call it.

**What teaches it.** The Guide's section on leaving a memo, the CLI contract it
quotes verbatim, and the four building skills — next-task, implementing,
instruction, next-stage — show the command with a title, and say what a title
is for: the words the human reads down the Timeline, a handful of them naming
what the memo is about. The server's test that each building skill teaches
leaving a memo should look for the titled shape.

Follow docs/copy.md for every word that reaches a screen or stderr, and
CONTEXT.md's **Memo** entry for every name.

## Acceptance criteria

- [ ] The CLI suite shows `verkstead memo` refusing a missing, an empty and a
      multi-line title without a request, and leaving a titled memo whose id
      comes back on stdout as before; the server refuses a titleless or
      multi-line body sent by hand, by name.
- [ ] A later session's prompt names a titled memo by its title in bold with
      the id only in the retire line, names an untitled one as it did before,
      and the four building skills and the Guide teach the titled command.
- [ ] The Timeline read carries each memo's title — null for one left before
      this — and the Event of the session that left it, and a memo crosses a
      Transfer with its title.
