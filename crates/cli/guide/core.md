# Asking the human

Verkstead carries a Question Set from a coding agent to the human. The human
answers on a phone, away from the terminal, so hours between the ask and the
answer is the tool working rather than the tool failing — and where the work
does not turn on the answer, an ask nothing ever comes back to is the point of
the second kind below.

This Guide is everything the binary knows about asking well, and it ships
inside the binary: `verkstead guide` — or `verkstead` with no arguments — is
where an agent starts, and it is the whole of it. Nothing else has to be found.

## Two kinds of ask

Every Set lands on its conversation's timeline, notifies the human's devices and
waits there to be answered. What differs is this end.

<!-- the two kinds of ask, per channel -->

The rule decides a Question at a time, and a Set is one kind or the other — so
Questions of both kinds go in two Sets: what the work turns on in the first
kind, and everything else in a deferred one sent alongside it. Budget each of
them as a Set in its own right, by **Pacing** below.

A deferred ask is a foreground call: it returns as soon as the Set is stored,
and there is nothing to wait for. What it prints is the stored Set rather than a
Response — the `id` it was stored under and when the server took it — and that
is the whole of what comes back on this end, ever.

```
verkstead ask --deferred <<'YAML'
title: …
questions:
  - label: Q1
    text: …
YAML
```

```yaml
id: 42
created_at: 2026-08-24T09:12:03.114Z
```

Everything below is about writing a Set and holds for both kinds. **Running the
ask** is where this end's own mechanics are.

## Question labels

Label every Question, so each Answer keys unambiguously back to what was asked.
Not every Question needs Options — a bare clarifying Question just carries a
label. Add Options where there are discrete choices, and then recommend one.

- **Questions** are numbered `Q1`, `Q2`, … **monotonically across the whole
  session** — never reset the counter for a new Set, so `Q4` still means the
  same Question when either side points back to it later.
- **Sub-questions** append a letter: `Q7a`, `Q7b`. Use them when one Question
  has distinct parts that each need their own Answer. That label is what comes
  back rather than what is sent: a Sub-question sits under its parent's
  `subquestions` and carries a bare `letter: a`, and the `Q7` in front of it is
  the parent's own `label`.
- **Options** append `.N`: a Question's are `Q7.1`, `Q7.2`, a Sub-question's
  are `Q7a.1`, `Q7a.2`. The **Recommendation** is the `★` appended to an
  Option's number — `Q7.1★` — and there is at most one per Question or
  Sub-question.
- A Question **may carry both its own Options and Sub-questions.** Nothing is
  ambiguous, because an Answer at the Question level is a bare number and one at
  the Sub-question level always carries the letter.
- A Question with **Sub-questions and no Options of its own is a Heading.** Its
  text heads the Sub-questions under it rather than asking anything, so it is
  drawn without a field and **no Answer comes back for it** — a Response that
  carries one is refused, naming it. Use it for the framing a group of related
  Sub-questions is read against; anything that genuinely needs answering at that
  level takes Options, or becomes a Sub-question of its own.
- **Two levels maximum.** A Sub-question is a leaf: it carries Options, it never
  branches further. A decision that feels deeper than that splits into separate
  top-level Questions.

A Question carrying both:

```
Q11 — Which framing should the rewritten README take?
      Q11.1★  Personal curated suite
      Q11.2   Distribution-first catalog
      Q11a — Keep the documented install command?
             Q11a.1★  Keep it
             Q11a.2   Change it
      Q11b — Keep the workflow walkthrough section?
             Q11b.1★  Keep it, updated
             Q11b.2   Drop it
```

Those are the labels the Answers key back to, not the shape a Set is written
in. Nesting is the `subquestions` field and nothing else, a Sub-question's
`letter` is bare, and **Authoring the Set** below spells the whole of the tree
above as the YAML that actually goes over the wire.

## Pacing

Pace by complexity, not by Question count. The unit is the Set: one delivery of
Questions and the Answers that come back. Each Set should carry about one
sitting's worth of decision effort, where the effort is what an Answer costs the
human — not what it costs the agent to ask.

| Question complexity | Per Set |
|---|---|
| **complex** — weighs trade-offs, is open-ended, or has downstream consequences | ~1 |
| **medium** | ~4 |
| **simple** | ~8 |
| **trivial** — a fact, a name, a yes/no, or accepting an obvious default | ~15 |

A Set is a sitting rather than a passing remark: the human sees the whole of
it at once, in a UI built for reading it, where one more cheap Question costs
a tap. The round trip may be hours, so a Set is worth filling. The ceiling on
hard Questions doesn't move, though — thinking effort doesn't parallelize.

Mix freely, one medium alongside a couple of trivial, as long as the total stays
inside the budget. Batch independent Questions right up to it; the labels above
are what keep a full Set unambiguous.

Keep Questions sequential only where a later one genuinely depends on an earlier
Answer — never ask a Question whose very wording would change with an Answer
requested in the same Set. Nothing can be asked mid-Set, so a dependent Question
waits for the next one and costs a whole round trip. Where the dependency is
shallow, enumerate instead of deferring: fold the branches into Options, or hang
them off the parent as Sub-questions.

## The CLI contract

Verbatim, as shipped:

```
Submit a Question Set and wait for the human to answer it.

Whether waiting means blocking here until the Response comes back, or storing the Set and ending the turn to be told later, is this backend's own — run `verkstead guide` for which it is and how to run one. Either way the human answers in their own time, and that may be hours.

Prints the Response as YAML on stdout and exits 0 where it blocked, and the stored Set where it did not. Nothing else is ever written to stdout, so the agent can parse what comes back as it stands.

Usage: verkstead ask [OPTIONS] [FILE]

Arguments:
  [FILE]
          The Question Set, as YAML. Read from stdin when absent

Options:
      --deferred
          Nobody is to wait on it: store the Set and carry straight on.
          
          Prints the stored Set as YAML instead of a Response — its `id` and when the server took it — and exits 0, on every backend. The human answers it in their own time, and their Answers reach a later session of this Conversation and never this one, so `verkstead answers` refuses one. Wait only on Questions whose Answers affect the work about to be done.

      --server <SERVER>
          Base URL of the Verkstead server
          
          [env: VERKSTEAD_SERVER=]
          [default: http://127.0.0.1:8422]

  -h, --help
          Print help (see a summary with '-h')
```

Set shape — `title` (required), `preface`, `postscript`, and
`questions[].{label, text, columns, options[].{n, text, recommended, cells},
subquestions[].{letter, text, columns, options}}`.

Response shape — `answers[].{label, selected, free_text, unanswered}` plus a
set-level `comment`.

## Authoring the Set

One Set is one round, budgeted as above. Because the round trip is expensive,
sweep ahead: carry the Questions that would otherwise wait for the next round or
two, provided none of them depends on an Answer in this one.

What can't be swept ahead closes the Set as the **`postscript`**, and the line
between it and the Questions is what the reply would be, not how big the ask
is: **a decision, however small, is a Question, and the `postscript` carries
only the open-ended invitation.** "Write an ADR for this?" is a decision — two
Options, priced as trivial by the budget above — so it is asked as a Question,
never parked in the `postscript`, where nothing obliges a reply. "Anything else
worth knowing?" invites rather than decides, so it is **never a Question**: the
`postscript` is markdown drawn in the section closing the page, above the
set-level comment box that shares it, and that box is on every Set whether or
not one was written — a catch-all Question asks for a second time what the box
already asks, and costs the human a row they then have to leave explicitly
open. Suggest in the `postscript` what would be worth a word in the comment,
and let the box take it.

The axis is specific against open-ended, not Options against free text. A
trailing open Question is still right when it asks for something *specific* the
agent cannot find out for itself — a name, an id, a fact the repo does not
hold. And an open-ended ask the work cannot proceed without is a Question too:
a reply to the `postscript` is always optional — a blank comment means *nothing
to add* — so anything the agent actually needs takes a labelled row, where
withholding it comes back explicit as `unanswered`.

Decide the Questions first, then serialize them:

```yaml
title: Rate limiting for the public API
preface: |
  `POST /v1/messages` has no rate limit, and last night one client sent 40k
  requests in a minute. A limiter can land today, but where the counter lives
  is a product call rather than a technical one.
questions:
  - label: Q11
    text: Which framing should the rewritten README take?
    options:
      - n: 1
        text: Personal curated suite
        recommended: true
      - n: 2
        text: Distribution-first catalog
    subquestions:
      - letter: a
        text: Keep the documented install command?
        options:
          - n: 1
            text: Keep it
            recommended: true
          - n: 2
            text: Change it
  - label: Q12
    text: |
      **The rest of the rewrite.** Neither of these turns on Q11.
    subquestions:
      - letter: a
        text: Keep the screenshot?
        options:
          - n: 1
            text: Keep it
            recommended: true
          - n: 2
            text: Drop it
      - letter: b
        text: Keep the badge row?
        options:
          - n: 1
            text: Keep it
          - n: 2
            text: Drop it
            recommended: true
postscript: |
  Anything else you'd like to add, such as:

  * How the README is found and read today
  * Anything the framing above misses
```

Mapping from the labels:

- `label` is the `Qn` label, straight from the session counter — the server
  never assigns one, so the `Q11` here is the `Q11` either side can point back
  to.
- `letter` is the Sub-question suffix; `Q11` plus `a` is answered as `Q11a`.
  Sub-questions are leaves, as above — a third level is refused.
- `recommended: true` is the `★`. At most one per Question or Sub-question.
- Options are optional. A Question with none is a bare clarifying Question, and
  the Answer is whatever the human writes — unless it carries Sub-questions, in
  which case it is a Heading over them and takes no Answer at all. `Q12` above
  is one: Options on its Sub-questions, none of its own, so the Response comes
  back with `Q12a` and `Q12b` and no `Q12`.
- `postscript` closes the Set after the last Question, and carries no label
  because nothing answers it directly: what it raises comes back in the
  set-level `comment`, in the box drawn beneath it.

Three things to get right:

- **`preface` is not optional in practice.** The human answers without seeing
  the session, so the context that would otherwise sit in the session has to
  live here instead. Markdown. Enough that the Questions make sense cold.
- **Never supply `project`, `branch` or `diff`.** All three are overwritten
  whatever the Set claims: the CLI derives the project and the branch from the
  working directory, and Verkstead attaches the uncommitted Diff from its own
  read of the worktrees — the work's own and each companion repo it may write
  in, a labeled block each — so the human can see what has already been written.
- **Prose does not survive plain YAML scalars.** A colon-space anywhere in a
  Question, an Option or the Preface ends the scalar and the server refuses the
  whole Set — and quoting a command or a log line is exactly when it bites. Use
  a block scalar (`|`), or a folded one (`>-`), for anything longer than a few
  plain words. Markdown inside a block scalar needs no escaping at all, which
  is the other reason to reach for one.

And write it to be **grasped at a glance.** A Set is read on a phone, often
between other things, and one that has to be studied is one that gets put off.
That is a property of the writing rather than of the Questions:

- **Lead the `preface` with the bottom line** — the decision, the state of play,
  the one thing worth having from a single sentence. The context that justifies
  it comes after, for when one sentence isn't enough.
- **Keep the `preface` short.** Context that only one Question needs belongs in
  that Question's `text`, not up front. Aim for Questions that can be answered
  without reading the Preface at all.
- **Prefer a Diagram to prose for structure.** Relationships, flows and state
  are quicker to see than to read, and a ```` ```mermaid ```` fence in the
  `preface` or a Question's `text` is drawn as a Diagram in the viewer. Keep it
  small — roughly ten nodes, so it stays legible on a phone. A fence degrades
  to the source text it was written as wherever it can't be drawn, so it is
  safe to send even to an older viewer.
- **Declare a comparison table rather than writing one.** Where Options trade
  off along several axes, the Question's — or the Sub-question's — `columns`
  names the axes, and each Option's `cells` fills one per axis in that order.
  The viewer draws those Options as the rows of the table itself, the Option's
  `text` as the row's leading cell and the whole row as the tap target, so a
  table written into the Question's `text` only has the human read the same
  comparison twice, once as prose and again as the Options beneath it.

  ```yaml
  questions:
    - label: Q12
      text: Where should the rate-limit counter live?
      columns: [Accuracy, Ops cost, Blast radius]
      options:
        - n: 1
          text: In-process counter
          cells: [Per-node, Nothing to run, One node]
          recommended: true
        - n: 2
          text: Shared Redis counter
          cells: [Exact, A service to run, Every node]
  ```

- **Bold the load-bearing phrases** so skimming lands on them.

## Running the ask

<!-- running the ask, per channel -->

There is no health probe — the attempt is the probe. If the ask fails for a
reason that isn't the Set — the server down, the connection refused, any other
non-zero exit — **report the failure to the human and wait for instructions.**
There is nowhere else to put the Questions. Say what failed and what is now
waiting on them, then stop: answering on their behalf, or taking the
Recommendations and carrying on, decides in their place the very thing that was
worth asking about.

A Set refused as malformed is not the transport breaking: the server is up and
answering, and the fault is in what was sent. Fix the Set and send it again —
the refusal names the Question at fault, and the server is local, so the round
trip costs almost nothing.

## Reading the Response

Stdout is the Response YAML and nothing else — all chatter goes to stderr — so
it parses as it stands:

```yaml
answers:
  - label: Q11
    selected: 1
    free_text: Start there, revisit if the catalog case gets stronger.
  - label: Q11a
    unanswered: true
  - label: Q12a
    selected: 1
  - label: Q12b
    selected: 2
comment: |
  On Q11a I genuinely don't know — pick whatever's least work to change later.
```

That holds wherever the two streams land in one file — a harness collecting a
command it ran, a shell redirect: a run that goes to plan says nothing at all on
stderr, and the little the CLI ever has to say there is written as a YAML
comment. Hand the whole thing to a parser.

Every Question and Sub-question the Set actually asked comes back exactly once,
so there is never anything to infer about what the human passed over. A Heading
asked nothing and so has no entry — its absence is the grammar rather than an
omission, and there is nothing there to follow up:

- `selected` → the number of the Option they picked; `selected: 1` on `Q11` is
  `Q11.1`.
- `free_text` → their own words. Alongside `selected` it is the rationale or a
  qualification; on its own it is an answer of their own instead of one of the
  Options, and it wins over the Options offered.
- `unanswered: true` → **still open.** Ask a brief follow-up. Never read it as
  accepting the Recommendation.
- `comment` → about the Set as a whole rather than any one Question, and the
  reply to the `postscript` where the Set closed with one. Read it before acting
  on the answers; it may reframe them. **An absent `comment` means they had
  nothing to add** — the box is on every Set and always optional, so an empty
  one is an answer of its own, and never a Question left open.
- `direction` → the direction they picked, on a Set that carried a `proposal`
  and on no other. It answers no Question, so it is a field of the Response
  rather than an entry in `answers`. **A `direction` is the proposal accepted**,
  and its absence is the proposal sent back. See the grilling skill for what to
  do about either.

A Response of nothing but `unanswered` entries plus a `comment` is a valid
counter-question. It means the human is not answering as asked — take the
discussion back a step rather than putting the same Set again.
