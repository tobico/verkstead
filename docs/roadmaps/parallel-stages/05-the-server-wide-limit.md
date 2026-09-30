# 05. The server-wide limit

## Goal

Verkstead starts nothing by itself while **four Conversations** are already
taking a place on the server, whatever roadmap or Process they belong to, and
starts what it held back as places come free. The number is a setting. What the
human presses is never held back.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *What starts, and how
  many***. The limit is the human's addition and is there for the machine: a
  stage may be a heavy build and a test run, and the hardware is shared by
  everything the server runs.
- **It counts every Conversation with a session running, of any kind** — a
  grilling, a Review, a Tinker, another roadmap's stage — not only stages.
- **A stage waiting on the human, or waiting to join, takes a place here
  too.** The recommendation was that it should not, nothing being running; the
  human chose that it does. So a server whose places are all held by stages
  waiting on answers starts nothing more until one is answered, and that has
  to be visible rather than looking like a stall.
- **It holds back only what Verkstead starts by itself**: a stage started by a
  settle. A press — Start, Continue a roadmap, Resume — goes ahead over the
  limit and is counted afterwards.
- **Four unless changed**, beside the roadmap's own limit of three, so one
  roadmap cannot take the whole server by default.
- **What was held back starts when a place comes free**, oldest roadmap
  waiting first, and within a roadmap lowest number first.

## Proposed tasks (provisional)

1. **Counting places** — AC: a running grilling takes one; a stage blocked on
   a Question Set takes one; a Conversation that is Done or stopped takes none.
2. **Holding a start back** — a stage ready under its roadmap's limit and over
   the server's is not started, and the Timeline says it is waiting for a
   place. AC: nothing is started at four; a press at four still starts.
3. **Starting what waited** — AC: a place coming free starts the stage that
   waited longest; a restart of the server loses nothing that was waiting.
4. **The setting** — on the Settings page beside the roadmap's limit. AC: a
   change takes effect at the next start and stops nothing already running.

## Re-verify at start

- Stage 04 landed, and there is one place where a start is permitted.
- Nothing else caps sessions: `Sessions.running` is still one per
  Conversation and no more than that.
- What *a session running* means once cluster mode has landed: a Conversation
  transferred to another device is running on that device's hardware. Whether
  the limit is per device is cluster mode's to say, and until it does this
  limit is this server's own.
- Waiting for a place is a third way a stage can be not started and not
  halted, beside waiting on a dependency and waiting to join. Stage 06 shows
  the first two; agree with it what this one reads as.
