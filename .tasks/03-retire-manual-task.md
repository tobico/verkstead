# 03. Retire Manual Task

## What to build

Manual Task goes entirely. A steer into Implementing with a hand-written
instruction covers what it was for and covers it better — the instruction
session drives the pipeline instead of leaving the Conversation stopped beside
its own work — so a second way to set one session going by hand is a second
thing to keep true.

Removed: the composer under the Timeline, the endpoint behind it, the submission
and outcome types the browser reads, the module that saw a manual session out,
and the store call that wrote the instruction to the Timeline. The pace entry
for how long a manual session must have been quiet goes with the module that
read it.

**And the skill, which is the one retired term that is also a shipped file.**
The `manual-task` directory under the server's skills, the constant naming its
installed path, the builder that wrapped an instruction in it, and that skill's
own tests. The skills are installed by clearing the directory and writing out
what the binary carries, so a withdrawn skill stops being installed the moment
it stops being carried — but check that before trusting it.

**Check the instruction skill is standing first.** Stage 02's instruction skill
is what is left after this, so confirm it is carried, installed and actually
launched by a steer into Implementing before the old one is deleted. Deleting
the wrong one leaves every instruction session reading a file that is not there.

**The folding rule simplifies.** A Manual Task's session was carved out of
Deferred Ask folding, because its prompt was the instruction and nothing else.
That carve-out goes with the feature, leaving the relaunched grilling as the one
session that is never folded into — and it is written down rather than inferred,
so the writing has to change too.

Two other things are explained by the Manual Task and outlive it. A driver waits
for the Turn rather than ending whatever holds it, which was justified by a
manual session running in a quiet moment; a steer holds the Turn the same way,
so the mechanism stays and the reason it gives changes. And the stall sweep was
run once on its own the moment a manual session ended; its standing loop stays,
and what is removed is the one-off call and the paragraph explaining it.

**Old records stay readable.** A Timeline holding a Manual Task Event still
reads: the stored Event kind stays, and the instruction is drawn as plain text
rather than as the openable document card it used to get. Nothing is rewritten
and nothing is dropped — it is the record of something that happened.

## Acceptance criteria

- [ ] Nothing installs or names a manual-task skill, and a steer into
      Implementing with a hand-written instruction still launches its session and
      still hands the pipeline on when that session finishes cleanly.
- [ ] There is no Manual Task composer anywhere in the workbench, and no endpoint
      or exported type for starting one.
- [ ] A Conversation whose Timeline holds a Manual Task Event still renders, with
      the instruction shown as plain text.
- [ ] The only session never folded into is the relaunched grilling, and that is
      what the folding is documented as doing.
