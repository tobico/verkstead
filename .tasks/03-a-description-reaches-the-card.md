# 03. A description reaches the card

## What to build

A Preview gains a **description**: markdown the agent writes under the
command when it suggests one, saying what the Preview is for in more words
than its title holds. This task carries it from `verkstead preview` to the
card task 02 built — through the CLI, the schema, the server, the store, the
view and the viewer — and leaves teaching the agent about it to task 04.

**On the wire from the agent.** Stdin to `verkstead preview <title>` is
today the one-line command, and a newline anywhere in it is refused. From
now on stdin is read the way a commit message is: the first line is the
command, and everything after a blank line is the description, as markdown.

    verkstead preview "Build and test" <<'SH'
    cargo build && cargo run -- serve

    Builds the server and starts it on port 8422. Open the viewer at
    `http://localhost:8422` and try the Code pane.
    SH

The rules, checked by the CLI as a courtesy and by the server as the truth,
each refusal saying what was wrong in the human's terms:

- The first line is the command, trimmed of its trailing newline as today,
  and refused empty.
- **A second line that is not blank is refused**, as a newline in the
  command is today: a command wrapped by accident never becomes a silent
  description. (A trailing newline alone, the heredoc's own, is still the
  one-line case.)
- The description is what follows the first blank line, with its leading and
  trailing blank lines trimmed; blank after trimming is no description at
  all.
- The description is **at most 2 KiB** — a paragraph or two, what a card can
  hold — and refused over that, naming the limit.
- The command's own limit of 4 KiB and the title's rules are unchanged.

The suggestion the server receives carries the description as an optional
field beside the title and the command, and the title the server prints back
is unchanged.

**Stored.** The previews table gains a nullable description column, added
through the store's migration module the way the memos table gained its
title: safe to run twice, the column's absence saying whether there is
anything to do. **The same title suggested again replaces the row whole**:
a suggestion without a description clears the one that stood, because what
the agent last said is what stands. The Transfer's carried-tables list
already copies the previews table by Conversation, so a transferred
Conversation keeps its descriptions; prove it rather than assume it.

**Served.** The Preview as the page receives it gains the description
**rendered to HTML by the server**, through the render crate's markdown
renderer, which is how a memo's and a Brief's markdown reach the page —
sanitized on the way out, nothing parsed on the client. Absent where there
is none. The viewer's generated types follow from the Rust type, as they do
for everything on that view.

**Drawn.** The card shows the description first, rendered, and the command as
a code block under it; with no description the card holds the command alone,
as task 02 left it. No line saying which session suggested it or when —
that was considered and left out.

The decision is recorded in the amendment at the end of ADR-0024 and in
CONTEXT.md under **Preview** and **Code**: build to those words.

## Acceptance criteria

- [ ] A heredoc with a command, a blank line and markdown stores the description; the card shows it rendered above the command, and a Preview with none shows the command alone.
- [ ] A non-blank second line and a description over 2 KiB are each refused by the CLI and by the server with a reason naming what was wrong; the one-line case is unchanged.
- [ ] The same title suggested again without a body clears the old description, and a transferred Conversation keeps its descriptions.
- [ ] The previews table's new column arrives by a migration that is safe to run on a database made before it and one made after.
- [ ] The sessions suite, the CLI's tests and the web suite cover each of the above.
