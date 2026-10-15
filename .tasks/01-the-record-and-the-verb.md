# 01. The record and the verb

## What to build

A building session can leave a preview on its conversation, and the workbench
can read what stands. Nothing is drawn yet: this task is the record, the verb
that writes it, and the view that serves it, proven end to end by the CLI on
one side and the conversation view on the other.

**The record.** A side table of the conversation, on the pattern of the memos
table and beside it: one row per standing preview, keyed by the conversation
and the title, holding the command, who left it (the same stamped words a memo
carries — *the session that worked task 03*) and when. It is **not an Event**:
no timeline row, no card, no place on a share. It is carried on a transfer the
way the memos table is, and deleted with the conversation the way the memos
table is. Retiring deletes the row; there is nothing to mark, because nothing
on the timeline points at it.

**The verb.** `verkstead preview <title>` takes the command on stdin, the way
`verkstead memo` takes its markdown, and `verkstead preview --retire <title>`
takes one back; nothing is read from stdin on a retire. Accepted, a suggestion
prints the title as YAML on stdout — `title: Build and test` — and says in
words on stderr what it did; a retire prints nothing on stdout. Both reach the
server over the per-session `VERKSTEAD_SERVER` URL, under the agent routes
beside the memo's, and both are refused unless a **building** session is
running on the conversation, by the same register and with the same wording
the memo refusal names the kinds by. A title already standing is **replaced in
place** — the command and the stamp change, nothing is duplicated — and a
retire of a title that does not stand is refused by name.

**What is refused on both ends**, in the schema crate so the CLI refuses as a
courtesy and the server as the rule, as the memo's bounds are: an empty title,
an empty command, a newline anywhere in the command, and a command over
4 KiB. Each refusal names what it refuses. The command is trimmed of its
trailing newline before any of that, since a heredoc always ends in one.

**The view.** The conversation view gains the standing previews, oldest first,
each with its title and its command, so the header icon can read them without
the Code pane open. A suggestion and a retire each announce the existing
`conversation` nudge, which is what re-reads that view. The generated viewer
types pick the new shape up through the conversation view as they do every
other field of it.

## Acceptance criteria

- [ ] A building session's `verkstead preview` lands on the conversation view with its title and command, and a second with the same title replaces the command rather than adding a row.
- [ ] A grilling session's `verkstead preview` is refused naming the kinds that may, and a retire of a title that does not stand is refused by name.
- [ ] An empty title, an empty command, a newline in the command, or a command over 4 KiB is refused by name by the CLI before anything is sent and by the server when sent by hand.
- [ ] A transferred conversation arrives with its previews, and deleting a conversation takes them with it.
