# 05. Grok Build's usage limit, and the whole stage proved

## What to build

The last of Grok Build's own constants, and the stage's demonstration.

**The matcher learns to say *no phrase*.** Recognition today is one sentence
per backend and every backend has one, so the mapping hands back a phrase and
the matcher compares the opening of a line against it. A backend whose wording
nobody has seen would have to be given the empty string — which opens every
line there is, and would stop every session on that backend at its first flush.
So a backend without a phrase is **skipped** rather than matched against
nothing, and the mapping says which is which. OpenCode is the backend this is
really for — it retries provider limits internally and may have no phrase for a
long time — and it is built here because Grok Build is where the question first
comes up.

**Grok Build ships with the wording it has.** A free account's is confirmed and
opens `You've reached your free Grok Build usage limit`, the rest of the line
being the upgrade pitch — so the stable prefix is what gets matched, read off
the Capture and the Transcript exactly as today. A paid account's is not known,
and xAI sends a different server message per plan, so a paid stop lands as the
ordinary stall until somebody sees one. That is the accepted state rather than
a gap.

Nothing else about the stop changes. It is the ordinary stop — one Notice, one
*blocked on you*, one Resume — naming the Profile whose account ran out, and it
ends the session for the reason it already does. The rule that a line must
*say* it rather than mention it does not change either: it is what keeps a
session grepping this repository from stopping its own run.

**And then the stage is proved end to end**: a Conversation grilled, built and
wrapped under a Grok Build Profile. That is where the pieces meet — the launch
line and the named session, the signature that says it has stopped, the updates
log on the Timeline, and store-and-nudge asking on the channel this type names.
Two things have never run against the real thing and are worth watching:

- **The nudge landing in grok's composer.** Stage 03 settled this for codex —
  the line goes in as one burst with the carriage return a quarter-second
  behind it, and it lands as a send — but grok's composer is its own program,
  and a paste whose return is a line break rather than a send is the failure
  the gap was written for.
- **The Guide a Grok session reads.** It should be the store-and-nudge one, and
  the session should end its turn on an ask and come back for its Answers when
  the line arrives.

Bring CONTEXT.md up with what this stage lands: a Transcript's log is named on
this backend and named by a session id it takes at launch, and the entries that
name Codex as the example of a one-home account and of a drawn idle judgement
have a second one to name.

**The proof needs an xAI account**, and everything above needs a `grok` on the
machine. If either is missing when this is worked, land what can be landed
against the stubs and record the run as outstanding rather than declaring it
done.

## Acceptance criteria

- [ ] A backend with no usage-limit phrase is skipped rather than matched, and
      a session on such a backend runs to its ordinary stall instead of
      stopping at its first flush.
- [ ] A Grok session printing its limit line stops the run with a Notice naming
      the Profile, read off the Capture and off the Transcript both; Claude's
      and codex's phrases and stops are unchanged.
- [ ] A Conversation is grilled, asked by store-and-nudge, nudged when the
      Response lands, answered, built and wrapped end to end under a Grok Build
      Profile — and CONTEXT.md says what this stage changed.

## What was read off the real thing

The wording this task was to ship turned out to be two wordings and neither of
them the one the plan had. Read off grok 1.0.13, pulled down and driven outside
Verkstead: installed under a `GROK_HOME` of its own from `@xai-official/grok`,
pointed by `GROK_CLI_CHAT_PROXY_BASE_URL` and `XAI_API_KEY` at a stand-in xAI
chat proxy — grok speaks `POST /v1/chat/completions` to it — which answered a
turn with `429` and `{"error":"subscription:free-usage-exhausted"}`, the code
grok's own binary carries. Everything below is grok's; only the server behind it
was not.

- **Grok does not print its limit. It draws a card.** The interactive grok
  retries the refused turn, gives up, and puts up a bordered panel headed `You
  hit your free usage limit.` with three tiers under it. The sentence this stage
  planned to ship — `You’ve reached your free Grok Build usage limit for now.
  Get SuperGrok…`, and a typographic apostrophe at that, not the ASCII one the
  constant had — is what `grok -p` prints and exits with. Verkstead launches the
  interactive one.
- **And a card is drawn where no line-reader can see it.** Grok writes a frame
  as a cursor move per row with no newline anywhere, so everything it has drawn
  since the last read is *one* line of bytes: in the capture this was read off,
  the sentence sat at column 4102 of a line 5015 characters long. The matcher
  asks what a line opens with, so it would have found nothing whatever phrase it
  held. What it now reads beside the printed bytes and the log is the **frame**,
  off the same Screen the idle judgement reads — one line of a grid, the card's
  border in front of it as decoration.
- **The log says nothing a reader here can use.** What reaches `updates.jsonl`
  is two `retry_state` lines whose reason is the server's own error string —
  bookkeeping by kind, folded under its own name, and never a statement the
  summary reads. So a Grok limit is caught on the frame and only there; the
  Transcript arm is a guard against a release that starts saying it in prose.
- **A paid plan's card is headed differently**: `You hit your weekly limit.`,
  with `Upgrade to a higher tier for more usage` and `Purchase credits to keep
  using Grok Build` under it, in the same binary and the same card. Nobody has
  watched one drawn and one phrase per backend cannot open both, so a paid stop
  still stalls — the same shape as a backend with no phrase, and the reason the
  next thing this wants is a backend holding more than one wording.
- **The nudge lands in grok's composer as a send.** Typed the way Verkstead
  types it — the line as one write, the carriage return a quarter-second behind
  it — into a real grok on a pseudo-terminal: the composer submitted it, the
  model was sent it as a user turn, and the composer was empty afterwards. So
  burst detection stays off this backend's launch line, as it did for codex.

## What is still waiting

There is no `grok` on the system profile and no xAI account on this machine, so
the **end-to-end run is outstanding rather than met**: a Conversation grilled,
asked by store-and-nudge, nudged, answered, built and wrapped under a Grok Build
Profile. Two halves of it are settled above — the frame a limit lands on and the
nudge landing as a send — and what is left needs a real model turn: that a Grok
session reads the store-and-nudge Guide, ends its turn on an ask, and comes back
for its Answers when the nudge arrives.

The first criterion's own end-to-end half waits on something else again: there
is no phrase-less backend to run a session on yet. What is proved is the skip
itself — the mapping, the reading, and a Watch on such a backend finding nothing
in any of the three records it is fed. OpenCode is what will make it a session.
