# 05. OpenCode's records drawn, and the whole stage proved

## What to build

The renderer that turns the records task 04 stored into the conversation they
record, and then the whole stage run end to end on the real thing.

**One reader, a fourth backend's shapes.** The Transcript is read by one code
path that keys off what a line says it is, and the same reading serves the pane,
the Timeline row's turn count and the sentence a Notice quotes. This adds
OpenCode's kinds to it rather than a reader of its own.

What opencode writes is a small tagged set, and most of it maps onto turns that
already exist: the turn put to it, the assistant's own record — whose content is
a list of prose, reasoning and tool calls with the tool's state and result inside
it — a shell command it ran, and then its own bookkeeping: the agent it switched
to, the model it switched to, the system and synthetic text it wrote to itself,
and the summary it replaced a long conversation with. A closed list and a
fall-back past it, exactly as the three backends before it have: a kind nobody
here has heard of folds away under its own name rather than standing in the
conversation.

**Two backends now use the same words for different shapes.** opencode's records
are tagged `user` and `assistant` — the same two words Claude's lines carry —
but what hangs off them is not the same: opencode puts the assistant's content at
the top level where Claude puts it under a message, and a tool call carries its
own state and result rather than being answered by a separate line. A reader that
keys on the word alone will send opencode's records down Claude's arm and draw
every one of them as unreadable. Tell them apart on something that is actually
different, and write down what that is, because it is the kind of thing a fourth
backend's arrival will test again.

**Then prove the stage.** Everything OpenCode needs is built by here: the type
and its account, the line, the idle signature, the Guide's tailoring and the held
ask, the store reader and this renderer. What is left is running a Conversation
all the way through under an OpenCode Profile on a real provider account —
grilled, with a Set actually answered from a phone, and built — and reading the
result back off the Timeline as a human would.

**Update the vocabulary as this piece lands.** `CONTEXT.md` describes how a
Transcript is found and how idle is judged in terms of the backends that had
landed; OpenCode is a fourth answer to the first — found, but out of a database
rather than a file — and its Profile is a fourth account shape. The roadmap says
each stage updates the terms as its piece lands, and this is the stage's last
task.

## Acceptance criteria

- [x] A real OpenCode session's Transcript draws as the conversation it was:
      prose, reasoning, tool calls and their answers, and the turns put to it,
      with opencode's own bookkeeping folded away and a kind this build has
      never seen folded under its own name.
- [x] Claude's, Codex's and Grok Build's Transcripts draw exactly as they did,
      and the Timeline row's turn count and the quoted sentence are the same
      reading as the pane's for all four.
- [x] A whole Conversation runs under an OpenCode Profile — grilled with a
      blocking Set answered away from the terminal, then built — and its
      Timeline reads back as a record of what happened.
- [x] `CONTEXT.md` says how an OpenCode session's Transcript is found and how
      its idle is judged, in the vocabulary already there.

## What was read off the real thing

**opencode 1.18.21**, which is what the host put on the system profile — where
tasks 01 to 04 pinned their constants against 1.18.25, the release they pulled
down and drove by hand. Everything this stage reads off opencode holds on both,
checked rather than assumed:

- the launch line `Agents::argv` writes parses in that order and `--prompt`
  submits;
- the store is `<the Profile's home>/.local/share/opencode/opencode.db`, in
  write-ahead-log mode, with the `session` table's `directory`, `parent_id` and
  `time_created` that discovery is one statement over, and the `event` table's
  `aggregate_id`, `seq`, `type` and `data` that the follower reads;
- a plain turn writes the four kinds the renderer knows — `session.created.1`,
  `session.updated.1`, `message.updated.1`, `message.part.updated.1` — and the
  part kinds `text`, `tool`, `step-start` and `step-finish`;
- the idle signature stands: `esc interrupt` was in the working frame of every
  sample taken while a tool call ran, and gone from the resting frame, read back
  through the same `avt` the Screen is.

**And the whole of the stage ran under a server built from this branch.** The
running workbench is an older build that knows only the Claude type, so the run
was made against a `verkstead serve` of its own, on a watched tree of its own,
with a throwaway repo and an OpenCode Profile whose home holds an
OpenAI-compatible **stand-in** provider — opencode's own, only the model behind
it was not. What that proved is everything Verkstead owns, and it is the half
tasks 01 to 04 each recorded as outstanding:

- `opencode` is found on the sandbox's fixed `PATH` and launched, the Profile's
  `.config/opencode` and `.local/share/opencode` land at the XDG defaults inside
  a fresh HOME, and the account written is the Profile's own;
- the Brief reaches the session through `--prompt` and it starts on it — the
  grilling prompt and the Brief are the first turn on its Transcript;
- its shell tool runs in the sandbox, against the Conversation's own Worktree;
- **the session store is found by the Worktree and the moment and followed while
  the session runs**, and its records draw as the conversation they record: the
  turn put to it, the call and its answer, and the sentences, with opencode's
  own bookkeeping and every growth-emission folded away;
- the Timeline row is that same reading — five turns, and the agent's own last
  sentence as the row's line — and the session reads **idle** by its screen
  signature, judged by the running server rather than by a stub;
- and **the blocking ask holds inside the sandbox**. A second Conversation's
  session called a real `verkstead ask` from its shell tool with no timeout of
  its own, and the model turn stayed open for 155 seconds — past opencode's
  unraised 120000 ms default, which is what
  `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` is set for — with the session
  reading at work throughout. Answering the Set over the API, the way the CLI
  suite takes the human's part, put the Response YAML back as the tool's own
  output and the turn went on from it.

## And the stage proved on a real account

The human's answer to this task's ask was to set an OpenCode account up, so the
last criterion is **met rather than outstanding**. What ran: an
`openai/gpt-5.4` session under an OpenCode Profile whose home is the account
they sent, on a Conversation of this repository, from Brief to built change.

- **Grilled.** It read `docs/development.md`, `crates/server/tests/sessions.rs`
  and `scripts/soak-sessions.sh`, and asked a Set of three real questions about
  the paragraph it was to write — grounded in what it had read, with a
  Recommendation on each.
- **Blocked on `verkstead ask`, and answered away from the terminal.** The
  shell tool held the model turn open across the whole answer gap, twice: once
  for the grilling Set and again for its closing proposal. Both were answered
  from the human's phone and **relayed** — their workbench is an older build
  that does not know the OpenCode type, so the Conversation ran on a
  `verkstead serve` of this branch on a port their phone cannot reach, and this
  session carried each Set to them and their answers back verbatim.
- **Then built.** It took `inline`, wrote its handoff, and a fresh session under
  the same Profile made the edit — the exact-name cargo form, `VERKSTEAD_TEST_PACE`
  still applying to one filtered test, the cap needing no setting, and a generic
  `<test-name>`: all four of the human's answers honoured, and nothing but
  `docs/development.md` touched.
- **And the Timeline reads back as the record of it.** The build session's
  Transcript draws as 47 turns and then 57 — the turns put to it, its prose, its
  reasoning, and sixteen tool calls each beside its answer — with 138 records
  folded away as opencode's own bookkeeping, `patch` parts and every
  re-emission of a growing part among them. The row's turn count and the
  sentence it quotes are that same reading, matched at 57 turns and the same
  sentence.

One thing had to be answered by the operator rather than relayed. The throwaway
server had no `git_author` in its `config.yaml`, so `git commit` inside the
sandbox asked to be told who it was — which is what `docs/development.md` says
happens with no author configured — and the build session raised it through the
same held ask rather than guessing. Told a probe identity, it committed: a
conventional subject, a summary body and a diagram of the delta, which is this
repository's own convention followed off its `git-workflow.md`.
