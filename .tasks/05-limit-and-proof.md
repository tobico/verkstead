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
