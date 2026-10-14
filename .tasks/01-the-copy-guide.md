# 01. The copy guide and the CLAUDE.md

## What to build

Write `docs/copy.md`, titled *Copy*: the guide every later task applies, and
the one place the rules below are written down. Then write a short `CLAUDE.md`
at the repository root so that a session finds it, and link the guide from the
README's *Where things are written down*, from the preamble of `CONTEXT.md`
and from `docs/development.md` beside its line about the vocabulary.

The guide is for whoever writes a string the human will read — a person or a
session — and it is read cold, so it carries its reasons. Every rule below was
settled in the grilling that planned this backlog; the guide states each one,
says why, and shows it with a before-and-after pair lifted from the viewer as
it stands today (the screens are full of material: *"This Set is no longer
here."* beside *"This set is gone."* in the Set sheet, *"Agent Profiles"* in
the wizard beside *"Agent profiles"* on the settings page, the server's
*"there is no backlog on that Conversation"* drawn raw as the whole text of a
card).

### Shape of the guide

1. **The four values, quoted verbatim** from the brief:

   * Clear and Minimal — Everything should be clear and easy to understand,
     extra complexity only exposed when needed, and to the minimum amount
     required
   * Beautiful and Intentional — Using Verkstead should be a calm and relaxing
     experience, the tool itself should never distract you from the important
     decisions you make with it.
   * Expert and professional — Nothing is "dumbed down", the user is expected
     to be an expert in both software engineering and their problem domain,
     and they're given all the tools they need to make important decisions and
     accomplish the most difficult of goals.
   * Open and honest — The user is in control at all times, nothing is hidden
     from them, no "magic", things are kept simple by their design, not by
     obfuscation.

2. **Voice: two registers.** What is on screen is short and plain — it says
   what is true and what to do, in a sentence or two. The documents, the
   skills, the Guide the CLI prints and the code comments keep the essay voice
   they have (long sentences, *rather than* contrasts, em-dashes), and the
   guide says so rather than reaching into them. The guide names what counts
   as *on screen*: the viewer, the strings the server sends the viewer
   (refusals, `why` and `trouble` lines, push notification bodies, the render
   crate's own lines on the timeline, condition words), and the desktop app's
   dialogs, menus and notices. It names what is out: the CLI's `--help` and
   printed lines, agent-facing prose, internal docs, comments.

3. **Words.** Point at `CONTEXT.md` for the vocabulary and its *Avoid* lists;
   the guide does not repeat definitions. What it adds:

   * **Lowercase on screen.** *A conversation, a repo, a worktree, a profile.*
     The capitals in `CONTEXT.md` and the docs are for defining and pointing
     at a term, not for the screen.
   * **"question set" in full** wherever it stands alone; *set* only where
     *question* is already in the sentence. Never a capitalised *Set* on
     screen.
   * **British spelling** — colour, behaviour, centre, recognise.
   * **The human is *you*.** Verkstead names itself when it acts (*"Verkstead
     will move this work"*), and never says *we* or *I*. Nothing says
     *please*, nothing apologises, nothing exclaims.
   * **The thing doing the work** is the harness by name where it is known
     (*"Starting Claude…"*) and *the agent* where it is not. *Session* and
     *run* are for the process — what starts, stops and is transferred — not
     for the worker.
   * **Contractions are preferred where they read naturally** — *can't*,
     *isn't*, *doesn't*. *Cannot* stays where a refusal is firm.

4. **A rule per kind of copy**, each with its before-and-after:

   * **Heading** — a pane or section heading (`h1` to `h3` in the viewer) is
     Title Case: *Pull Request*, *Agent Profiles*, *Git Author*, *On the Set
     as a Whole*. Articles, conjunctions and short prepositions stay lowercase
     — *a, an, the, of, on, as, to, in, and, or* — unless first or last.
     Markdown headings in the documents are not touched. No full stop.
   * **Everything else is sentence case** — card titles, menu groups, buttons,
     field labels, placeholders, table columns.
   * **Button** — the verb for what it does: *Save*, *Remove*, *Send anyway*,
     *Lock unanswered*. Never *OK*, *Yes* or *Submit*. No full stop.
   * **Confirmation** — a question naming the act, one sentence of
     consequence, and the verb against *Cancel*: *"Quit Verkstead?" / "Any
     sessions that are running will be stopped."* with *Quit* and *Cancel*.
   * **Destructive press** — says what goes. *Remove* over *Delete*; the card
     names what is about to be lost, never *Are you sure?*
   * **Menu item** — sentence case; one that opens a further step before
     anything happens carries the ellipsis: *Transfer to…*
   * **Field label and placeholder** — sentence case, no full stop; a
     placeholder is an example of a value (*ada@example.com*), not an
     instruction.
   * **Explanation beside a control** — one sentence, two at most. Anything
     longer goes behind a disclosure on the pane or into a document that is
     linked. The brief's *extra complexity only exposed when needed* is the
     reason.
   * **Empty state** — a sentence naming what is absent, and the one action
     that fills it where there is one: *"No repos are registered yet. Open one
     from the Repo dropdown."* Never a bare *Nothing here.* where there is
     something to say.
   * **Progress line** — a present participle naming what is being done, and
     the ellipsis: *Reading this machine…*, *Starting Claude…*. Not
     *Loading…* where the thing being loaded can be named.
   * **Refusal and error** — a full sentence ready to be shown as it is:
     capital, full stop, what is true, then the one action in a second
     sentence where there is one (*"This login is no longer running. Open it
     again."*). Where there is nothing to do, the fact alone. No *Error:*
     prefix, no apology, no exclamation mark, no blame. The server's strings
     follow the same rule, because the viewer draws them raw.
   * **Notification** — a sentence, as a refusal is, naming the branch or
     device it is about.
   * **Status and condition words** — the Process states and the conditions
     (*Blocked on you*, *Waiting on checks*) are the record's own and are
     spelled as `CONTEXT.md` has them.

5. **Typography on screen.**

   * The ellipsis is the one character *…*, never three dots.
   * No em-dashes on screen; start a new sentence instead.
   * Full stops on every complete sentence; never on a heading, a button, a
     label, a placeholder or a menu item.
   * A control is named in **bold** (*press **Open repo***); a path, a
     command, a branch, an environment variable or a file name is in `code`;
     nothing is put in quotation marks.
   * Serial comma.

6. **What was considered and rejected**, briefly, so it is not reopened: one
   voice everywhere (the essay voice is wrong on a phone); pulling the
   documents toward the plain register (too large a rewrite for no reader's
   gain); capitalised vocabulary on screen; a capitalised *Set* as the one
   exception; a lint over the viewer's strings (not now — the guide and the
   review first); editing Verkstead's own skills to name the guide (they are
   every project's, and already read a project's `CLAUDE.md`).

### The CLAUDE.md

A short pointer file, not an onboarding document: one line on what the
repository is, then `CONTEXT.md`, `docs/copy.md`, `docs/design/verkstead.md`
and `docs/development.md`, each with a line on when to read it. No build
commands — development.md has them.

### The links

The README's *Where things are written down* gains a **Copy** entry in the
shape of the others. `CONTEXT.md`'s opening says where the rules for writing
on screen are. `docs/development.md` says it beside its line that the
vocabulary in bold is the project's.

## Acceptance criteria

- [ ] `docs/copy.md` exists, opens with the four values verbatim, and states every rule above with its reason and a before-and-after pair taken from the viewer as it stands.
- [ ] `CLAUDE.md` exists at the root, fits on a page, and names `CONTEXT.md`, `docs/copy.md`, the design doc and `docs/development.md` with a line each on when to read it.
- [ ] The README, `CONTEXT.md` and `docs/development.md` each link to `docs/copy.md`.
- [ ] No string on screen was changed by this task; the review is the tasks that follow.
