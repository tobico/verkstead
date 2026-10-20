# Copy

How Verkstead speaks to the human: the rules for every string somebody reads
on a screen, and the reasons behind each, so that a person or a session writing
a new one can apply them cold.

The vocabulary itself — what a **Conversation**, a **Question Set** or a
**Worktree** is, and the words to avoid for each — is in
[CONTEXT.md](../CONTEXT.md), and is not repeated here. This document is about
how those words are put on a screen.

Every rule below comes with a pair: the string as the viewer had it when this
guide was written, and the string as the guide would have it. The *before* is
there to show the rule biting, not to point at a mistake.

## The four values

Everything Verkstead says answers to these, quoted as they were set down:

* Clear and Minimal — Everything should be clear and easy to understand, extra
  complexity only exposed when needed, and to the minimum amount required
* Beautiful and Intentional — Using Verkstead should be a calm and relaxing
  experience, the tool itself should never distract you from the important
  decisions you make with it.
* Expert and professional — Nothing is "dumbed down", the user is expected to
  be an expert in both software engineering and their problem domain, and
  they're given all the tools they need to make important decisions and
  accomplish the most difficult of goals.
* Open and honest — The user is in control at all times, nothing is hidden from
  them, no "magic", things are kept simple by their design, not by
  obfuscation.

Read as rules for copy: say what is true and no more than is needed (*clear and
minimal*); say it quietly, so the words never compete with the decision they
sit beside (*beautiful and intentional*); never explain what an engineer
already knows, and never hide what they would want to check (*expert and
professional*); and name what is happening plainly, including what Verkstead
itself is doing (*open and honest*).

## Voice: two registers

**On screen, copy is short and plain.** It says what is true and what to do, in
a sentence or two. The human is often reading it on a phone between other
things, and a long sentence there is one they scroll past.

**The documents keep the voice they have.** The docs, the skills, the Guide the
CLI prints and the code comments are written in a longer, essay voice — long
sentences, *rather than* contrasts, em-dashes — and that voice is right for
somebody reading at a desk to understand a decision. This guide does not reach
into them, and nothing here asks for them to be rewritten.

*On screen* means:

* the viewer, every page of it;
* the strings the server sends the viewer to be drawn — refusals, `why` and
  `trouble` lines, push notification titles and bodies, the render crate's own
  lines on the Timeline, and condition words;
* the desktop app's dialogs, menus and notices.

*Not on screen*, and written in the documents' voice:

* the CLI's `--help` and the lines it prints;
* prose written for an agent — skills, prompts, the Guide;
* the docs under `docs/`, `CONTEXT.md` and the README;
* code comments.

**Why two:** one voice everywhere was considered and rejected. The essay voice
is wrong on a phone, and pulling the documents toward the plain voice would be
a large rewrite that no reader of them gains from.

## Words

Use the words `CONTEXT.md` defines, and none of the ones it lists under
*Avoid*. On top of that:

### Lowercase on screen

A conversation, a repo, a worktree, a profile, a brief. The capitals in
`CONTEXT.md` and the docs are there to mark a term as defined and to point at
it; on a screen there is nothing to point at, and capitals scattered through a
sentence make it read like a form.

| Before | After |
| --- | --- |
| *Agent Profiles* (the wizard's step name) | *Agent profiles* |

### "question set" in full

Write *question set* wherever it stands alone; *set* on its own only where
*question* is already in the sentence. Never a capitalised *Set* on screen.
*Set* alone is a common word that reads as a verb as often as a noun, and the
capital, as the one exception to lowercase, would be the rule nobody remembers.

| Before | After |
| --- | --- |
| *This Set is no longer here.* | *This question set is no longer here.* |
| *On the Set as a Whole* | *On the Question Set as a Whole* (a heading, so Title Case) |

### British spelling

Colour, behaviour, centre, recognise, licence (the noun). One spelling, so that
two screens never disagree. Code identifiers and CSS properties (`color`,
`prefers-color-scheme`) are code, not copy, and stay as the platform spells
them.

| Before | After |
| --- | --- |
| *Recognized* (the shape to avoid; the viewer has none) | *Recognised* |

The viewer is consistent here already; the rule is written down so it stays so.

### The human is *you*

Verkstead names itself when it acts — *Verkstead will move this work* — and
never says *we* or *I*. There is no team behind the screen and no character
inside it, and *we* would claim both. Nothing says *please*, nothing
apologises, and nothing exclaims: politeness formulas and exclamation marks are
noise to an expert, and an apology on a refusal says something went wrong with
the tool when what happened is a fact about the world.

| Before | After |
| --- | --- |
| *Sorry, something went wrong!* (the shape to avoid; the viewer has none) | *The server couldn't be reached.* |

### The thing doing the work

Name the harness where it is known — *Starting Claude…* — and say *the agent*
where it is not. *Session* and *run* are for the process: what starts, stops
and is transferred. A session doesn't answer questions; an agent does, in a
session.

| Before | After |
| --- | --- |
| *…a session that answers what you ask, does what you want done about it…* (Steer's *Following up* note) | *The agent answers what you ask and does what you want done about it…* |

### Contractions

Prefer them where they read naturally — *can't*, *isn't*, *doesn't*. They are
how an expert colleague talks, and they make a short line shorter. *Cannot*
stays where a refusal is firm and the weight is the point: *This cannot be
undone.*

| Before | After |
| --- | --- |
| *That is not a directory — an account's home is mounted from one.* | *That isn't a directory. An account's home is mounted from one.* |

## A rule per kind of copy

### Heading

A pane or section heading — `h1` to `h3` in the viewer — is Title Case. Short
words stay lowercase unless first or last: *a, an, the, of, on, as, to, in,
and, or*. No full stop. A heading is a name, and Title Case is what tells it
apart from the sentence-case copy under it at a glance.

Markdown headings in the documents are not touched.

| Before | After |
| --- | --- |
| *Pull request* | *Pull Request* |
| *Git author* | *Git Author* |
| *On the Set as a whole* | *On the Question Set as a Whole* |

### Everything else is sentence case

Card titles, menu groups, buttons, field labels, placeholders, table columns.
Only the first word and proper nouns are capitalised. Two cases on one screen
are enough to say *this is a heading*; a third would only be decoration.

| Before | After |
| --- | --- |
| *Launch on Startup* (the desktop app's menu item) | *Launch on startup* |

**Except a Mac's menu bar, which is Title Case**: *New Conversation*, *View
Logs*, *Report an Issue*. The bar is the platform's furniture, and an item that
read differently from *Hide Others* beside it would be the one that looked
wrong — see ADR-0020, *Amended: the Mac menu bar*.

### Button

The verb for what it does: *Save*, *Remove*, *Send anyway*, *Lock unanswered*.
Never *OK*, *Yes* or *Submit*, which make the human reread the question to know
what they are agreeing to. No full stop.

| Before | After |
| --- | --- |
| *OK* (the shape to avoid; the viewer has none) | *Save* |

### Confirmation

A question naming the act, one sentence of consequence, and the act's verb
against *Cancel*. The question is what a native dialog draws large; the
consequence is the reason there is a dialog at all.

The desktop app's quit dialog is the model as it stands:

| Before | After |
| --- | --- |
| *Quit Verkstead?* / *Any sessions that are running will be stopped.* — **Quit**, **Cancel** | unchanged |

### Destructive press

Say what goes. *Remove* over *Delete*: removing a repo or a profile takes it
out of Verkstead and leaves the thing itself alone, and *Delete* is kept for
the one act that really forgets. The card names what is about to be lost,
never *Are you sure?*, which asks the human to remember what they pressed
instead of telling them.

| Before | After |
| --- | --- |
| *Delete archived conversations for good* | *Delete archived conversations*, with a line naming what goes: *Their timelines are removed from Verkstead for good. Their branches stay.* |

### Menu item

Sentence case. An item that opens a further step before anything happens
carries the ellipsis, so the human knows pressing it is safe.

| Before | After |
| --- | --- |
| *Transfer to…* | unchanged — it opens the device picker |
| *New terminal* | unchanged — it acts at once |

### Field label and placeholder

Sentence case, no full stop. A placeholder is an example of a value, not an
instruction: an instruction in a placeholder vanishes the moment the human
starts typing, which is when they need it.

| Before | After |
| --- | --- |
| *Token, pasted — it is stored and never shown again* (a label) | *Token*, with *Stored, and never shown again.* beside it |
| *ada@example.com* | unchanged — an example of the value |

### Explanation beside a control

One sentence, two at most. Anything longer goes behind a disclosure on the pane
or into a linked document — *extra complexity only exposed when needed*.

| Before | After |
| --- | --- |
| *A question about this work answered without changing it: a session that reads, writes and runs whatever it needs to find out, commits none of it, and keeps asking what else there is until you are finished. It then goes back to the state you steered it from, or to Done where there is nowhere to go back to.* | *The agent answers a question about this work without committing anything. It then returns to the state you steered it from.* |

### Empty state

A sentence naming what is absent, and the one action that fills it where there
is one. Never a bare *Nothing here.* where there is something to say: an empty
pane is often the first thing a new user sees, and it should say how it stops
being empty.

| Before | After |
| --- | --- |
| *No repos are registered yet.* | *No repos are registered yet. Open one from the **Repo** dropdown.* |
| *Nothing here.* (the path field's empty directory) | *This directory is empty.* |

### Progress line

A present participle naming what is being done, then the ellipsis: *Reading
this machine…*, *Starting Claude…*. Not *Loading…* where what is being loaded
can be named — the name is what tells the human whether to wait.

| Before | After |
| --- | --- |
| *Loading…* (the instructions card) | *Reading the instructions…* |

### Refusal and error

A full sentence ready to be shown as it is: a capital, a full stop, what is
true, and then the one action in a second sentence where there is one. Where
there is nothing to do, the fact alone. No *Error:* prefix, no apology, no
exclamation mark, no blame. The server's strings follow the same rule, because
the viewer draws them raw — a server string is screen copy the moment it can
reach a card.

| Before | After |
| --- | --- |
| *there is no backlog on that Conversation* (the server's, drawn as the whole card) | *This conversation has no backlog.* |
| *this login is no longer running; open it again* | *This login is no longer running. Open it again.* |
| *This set is gone.* beside *This Set is no longer here.* | *This question set is no longer here.*, in both places |
| *Write the brief first — it is what the work starts from.* | *Write the brief first. The work starts from it.* |

### Notification

A sentence, as a refusal is, naming the branch or device it is about. On a lock
screen the notification is all the human has, so the branch or device is what
tells them whether to get up.

| Before | After |
| --- | --- |
| *pwa-and-push is done* | *pwa-and-push is done.* |

### Status and condition words

The Process states and the conditions — *Blocked on you*, *Waiting on checks* —
are the record's own words and are spelled exactly as `CONTEXT.md` has them, so
that the screen, the docs and the data never name one state three ways.

| Before | After |
| --- | --- |
| *Blocked on you* | unchanged |

## Typography on screen

* **The ellipsis is one character**, *…*, never three dots. Three dots break
  across a line and read differently to a screen reader.
* **No em-dashes on screen.** Start a new sentence instead. An em-dash is the
  essay voice's join, and on a phone it makes one long line where two short
  ones read faster.

  | Before | After |
  | --- | --- |
  | *That is not a git repository — name the repository's own directory.* | *That isn't a git repository. Name the repository's own directory.* |

* **Full stops on every complete sentence**, and never on a heading, a button,
  a label, a placeholder or a menu item. The full stop is what says *this is a
  statement*; a name doesn't need one.
* **A control is named in bold**: *press **Open repo***. A path, a command, a
  branch, an environment variable or a file name is in `code`. Nothing is put
  in quotation marks: quotes on screen read as irony or as a quotation, and
  bold and code already say *this is a thing you can find*.

  | Before | After |
  | --- | --- |
  | *Pick a review profile and model — or No review — first, on the brief.* | *Pick a review profile and model, or **No review**, on the brief first.* |

* **Serial comma**: *repos, worktrees, and profiles*. It is the comma that
  keeps the last two items of a list from reading as one.

## Considered and rejected

So they are not reopened without a new reason:

* **One voice everywhere.** The essay voice is wrong on a phone.
* **Pulling the documents toward the plain register.** Too large a rewrite for
  no reader's gain; the documents' readers are at a desk.
* **Capitalised vocabulary on screen.** It makes sentences read like forms, and
  `CONTEXT.md` already marks the terms where marking them helps.
* **A capitalised *Set* as the one exception.** One exception is the rule
  nobody remembers; *question set* in full is clearer anyway.
* **A lint over the viewer's strings.** Not now: the guide and the review that
  applies it come first, and a lint can follow once the rules have settled.
* **Editing Verkstead's own skills to name this guide.** The skills are every
  project's, not this one's, and they already read a project's `CLAUDE.md`,
  which names it.
