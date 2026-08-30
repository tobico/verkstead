# 02. OpenCode judged idle by its screen

## What to build

The one signature constant that says whether an OpenCode session has stopped,
read off the real TUI rather than guessed at, and the arm that hands it to the
idle judgement.

Without it an OpenCode session is judged the way Claude is — three seconds with
nothing printed — and that reading was calibrated on an interface that draws
inline. opencode draws a full screen at sixty frames a second, so a session that
paused mid-turn would read as idle and be rescued out from under its own work,
and one whose renderer keeps repainting at rest would never read idle at all.

**Which of the two readings it is, is a fact about opencode rather than a
choice.** A signature is either an at-the-prompt line, whose *standing* says the
session has stopped, or an at-work line, whose *going* says so once the ordinary
three seconds of quiet says so too. Codex and Grok Build both came out the
second way — each leaves the frame it works in and the frame it rests in
differing only by a status line — and the at-the-prompt reading has stood
unused since it was written. Measure opencode rather than assuming it joins
them: drive a real session, sample its frame once a second across a turn and
again at rest, and find what is in every working frame and no resting one, or
the reverse. Prefer a fragment that does not move — a keybinding label rather
than a spinner glyph or a seconds count.

**Measure the silence beside it.** The at-work reading asks for three seconds of
quiet as well, because the line is equally missing from a session that has drawn
nothing yet; the at-the-prompt reading does not. Whichever it turns out to be,
record what the terminal actually did — how long the widest gap between reads
was mid-turn, and whether it went byte-silent at rest and for how long. That is
what says the reading is safe, and it is what the two constants before this one
each carry in their own comment.

**Settle the alternate screen while the binary is in front of you.** codex and
grok each take a flag that keeps them drawing inline, and Verkstead passes it for
exactly one reason: the Capture is the record of what a session did, and an
alternate screen is a record thrown away as the program leaves it. opencode has
no such flag, and its renderer is built without the option being passed either
way. Find out what it does. If it takes the alternate screen, the Screen still
reads — the screen model already tracks one — but say plainly what the Capture
then holds, and whether the minimal interface opencode offers instead is worth
reaching for. This is a finding to write down, not a thing to work around
silently.

**The long-stop is already there and does not move.** A signature that has
drifted reads as a session that never stops, and what catches one is the
byte-quiet measured in minutes behind every screen-judged backend. Nothing here
adds to it; the proof is that it still ends a session whose signature never says
stopped.

## Acceptance criteria

- [ ] An OpenCode session running under a real Profile reads *at work* while its
      turn runs and *stopped* once it is over, judged off the frame, with the
      constant named for the release it was read on.
- [x] A session whose signature never says stopped is still ended by the
      byte-quiet long-stop, and lands in front of the human as the ordinary
      would-not-ask stop.
- [x] What opencode does with the alternate screen is settled and written down,
      along with what the Capture holds as a result; Claude's three-second
      reading and the two signatures already shipped are unchanged.

## What was read off the real thing

OpenCode comes out where codex and grok came out: **an at-work reading**.
Measured against opencode 1.18.25 — `latest` on npm the day this landed, the
release task 01 pinned — driven on a hundred-column terminal through the same
virtual terminal the Screen is, a throwaway probe on `avt`, so what was read is
what `Screen::showing` reads.

- **Its waiting frame is its working frame.** The composer, the `Build auto ·
  <model>` label on its border and the `tab agents` and `ctrl+p commands` hints
  stand in both. What differs is the status bar at the foot of the frame: while
  a turn runs it is a progress dial and an `esc interrupt` label —
  `⬝⬝⬝⬝⬝■■■  esc interrupt` — and at rest it is the project's path instead.
  Across two turns of one session sampled once a second — a tool call and then a
  streamed reply, twice, 69 working frames and 58 resting ones — the label was
  in every working frame and in none of the resting ones, and in none of the
  three blank frames before opencode had drawn anything.
- **The constant is the label**, `esc interrupt`: the dial in front of it goes
  and comes with it, but its cells fill and empty every frame where the label
  does not move, and a keybinding label is a harder thing to hit by accident in
  what a session printed than a run of block characters is. Two words where
  codex's is three — `esc interrupt` against `esc to interrupt` — so neither
  backend's constant reads the other's frame, which is what the second test
  turns on.
- **It is byte-silent at its prompt**: not one byte in the 106 seconds it was
  left sitting there. Mid-turn the widest gap between reads was 86 ms once it
  had drawn its first frame — 1.4 s once, between its first escape sequence and
  that first frame — so the three-second quiet asked for beside the label is
  never met while it works.
- **And the same label is in either interface.** `--mini` carries it in a
  status bar of its own and drops it at rest exactly as the full TUI does, so
  the reading does not turn on which of the two a session is started in.

## What opencode does with the alternate screen

**It takes it, and there is no flag to keep it inline** — `--no-alt-screen` is
codex's and grok's answer and opencode's help offers nothing like it. `\e[?1049h`
is among the first bytes it writes. What that costs, measured rather than
reasoned about:

- **The Screen still reads.** The screen model already tracks which buffer is in
  front, so the idle judgement and a human watching a live session both see the
  frames opencode is drawing. Nothing here needed changing.
- **The Capture replayed holds none of the session.** A clean exit writes
  `\e[?1049l`, and the grid a replay of the whole Capture then leaves is what
  was on the ordinary buffer: opencode's farewell banner, naming the session's
  id and the command to resume it, and nothing of the conversation. Every byte
  is still in the Capture — what is gone is the grid they drew.

**Is `--mini` worth reaching for?** Not for this. It is opencode's minimal
interface, it writes no `\e[?1049h` at all, and it carries the same at-work
label — so it would buy back a Capture that replays to the conversation. What it
costs is the interface a human attaching to the Screen gets, and what it buys is
a record this backend does not depend on: the Timeline draws from the session
store (task 04), and the Capture is this backend's fallback rather than its
record. It is what to reach for the day that stops being true, and the finding
is written into ADR-0011 so that day does not start from scratch.

## What is still waiting

There is no provider account on this machine and no `opencode` on the system
profile, so the frames above were read off an opencode run **outside
Verkstead**: `opencode-linux-x64` 1.18.25 unpacked into a scratch directory and
run with a `HOME` of its own — so its four XDG directories resolved inside it,
which is the shape a Profile's home has — against a stand-in OpenAI-compatible
provider configured in `opencode.json`, which calls the `bash` tool and then
streams a reply slowly so that a turn could be held open and looked at.
Everything on the screen is opencode's own; only the model behind it was not.
The line it was driven on is the one `Agents::argv` writes: `-m provider/model`,
`--prompt`, `--auto`.

What that leaves outstanding is the same half task 01 left outstanding, for the
same reason:

- an OpenCode session **launched by Verkstead** under a real account, judged
  idle by the running server rather than by the suite's stubs. The signature
  itself is settled and the reading is proved; what is left is the binary on the
  system profile and an account behind it.
