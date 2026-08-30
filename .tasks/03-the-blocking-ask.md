# 03. The blocking ask under OpenCode

## What to build

A grilling under an OpenCode Profile that puts a Question Set to the human,
holds its shell command open while they answer, and carries on with the Response
on stdout.

Most of that already works: the CLI asks the same way everywhere, and OpenCode's
channel is the blocking one it was given in task 01. What is missing is the
Guide — and one sentence of it is actively wrong for this backend.

**The Guide is tailored per channel, and has to become per backend.** The
blocking half of *Running the ask* names Claude Code and tells the reader to make
the call a background one, because until now Claude was the only backend that
blocked. OpenCode is the second, and it does not work that way: its shell tool
runs the command synchronously in the model's own turn, holding nothing open on
the model's side, so what an OpenCode session needs told is to **pass a large
timeout**, not to background anything. A session handed Claude's instruction
would look for a harness feature it has not got.

So the splice moves from the channel to the agent type. Claude's blocking
section stays what it is; OpenCode gets one of its own; the store-and-nudge
backends keep sharing theirs, because what they share is real. Everything about
*writing* a Set is common and stays written once. The environment already
carries the agent type into every sandbox for exactly this, and the Guide read
outside a sandbox stays the blocking one a human at a terminal has.

**Raise the default timeout as well as saying it.** The shell tool opencode
registers takes any positive timeout — which is what makes the blocking ask free
here — but its default is two minutes, and its own description tells the model
that commands time out after the default. A model that passes nothing would have
its ask killed two minutes in, hours before the human answers. opencode reads a
variable that raises that default; set it in the sandbox environment so a session
that ignores the Guide still holds. Both, rather than either: the instruction is
the mechanism and the variable is what keeps a drifted instruction from being a
wedged ask.

**A held ask is a quiet session, and nothing must end it.** With an at-work
signature, a session sitting on `verkstead ask` draws no at-work line and reads
idle — correctly, since it is doing nothing. What keeps it alive is that an
unanswered Set of its own counts as open, which every ender and the Rescue both
read. That is already built and is not this task's to write; it *is* this task's
to prove under this backend, because a blocking backend whose held asks got
reaped would lose the answer the agent asked for.

## Acceptance criteria

- [x] `verkstead guide` printed inside an OpenCode sandbox gives OpenCode's own
      running-the-ask section; inside a Claude one it still gives Claude's; the
      two store-and-nudge backends still give theirs; and a Guide printed
      outside a sandbox is unchanged.
- [ ] A grilling under an OpenCode Profile blocks on `verkstead ask`, survives a
      gap far longer than the tool's own default timeout, and resumes with the
      Response as YAML on stdout — with the session's own turn never having
      ended and nothing typed into its terminal.
- [x] A session held on an open Set is not ended by any ender or prodded by the
      Rescue while it waits, and is ended by the ordinary rules once the Set is
      answered and its work is done.

## What was built

**The Guide's *Running the ask* is now per backend, and its *Two kinds of ask*
stays per channel.** What a channel decides is whether an ask waits or is
stored, which is what the kinds describe; how one is *run* is the backend's own,
and the two blocking backends do not run one the same way. So `claude` reads
what it always read, `opencode` reads a section of its own, `codex` and `grok`
go on sharing theirs, and a Guide printed outside a sandbox is Claude's — which
is what a human at a terminal has and what nothing set has always printed.
`running-blocking.md` is `running-claude.md` now, unchanged inside.

**OpenCode's section says to pass a large timeout**, in the units the shell tool
takes and with a value in it (`86400000` — a day), and it says the other thing
that follows from a synchronous tool: work the answers cannot invalidate is
done *before* the ask rather than while it runs, because the call holds the turn
where it stands. Claude's "while waiting, do any work that does not depend on
the answers" is true of a backgrounded call and false of this one.

**And the sandbox raises the tool's own default underneath it**, with
`OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` set to the same day, beside the
`OPENCODE_DB` the account is already told. Both, as the task asked: the Guide is
the mechanism and the variable is what stands under a drifted instruction.

## What was read off the real thing

opencode 1.18.25 again — the release tasks 01 and 02 pinned — driven outside
Verkstead, because there is still no `opencode` on the system profile and no
provider account: the binary unpacked into a scratch HOME whose XDG directories
resolve inside it, on the line `Agents::argv` writes (`-m provider/model`,
`--prompt`, `--auto`), against a stand-in OpenAI-compatible provider that makes
one shell-tool call and then says what came back. The command it was told to run
was a real `verkstead ask`, piped a Set on stdin, against a real `verkstead
serve` with a Repo and a Conversation registered through the quickstart's own
API calls — so what was measured is the whole round trip rather than a sleep.

- **The shell tool is `bash`, its timeout is milliseconds, and its default is
  120000.** The default is `bashDefaultTimeoutMs ?? 120000` and the tool's
  description tells the model that a command with no timeout is killed after it,
  so raising the variable raises what the model is told as well.
- **A held ask survives far past that default.** With `timeout: 3600000` the
  ask was held for 170 s and the Response arrived as the tool's own output
  within two seconds of the human answering — the Set's YAML on stdout, the
  model's turn never ended, and nothing typed into the terminal.
- **A call that passes nothing is killed at exactly the default.** The control
  run came back `(no output)` and `shell tool terminated command after exceeding
  timeout 120000 ms`, two minutes into a wait — which is what the variable is
  for.
- **And with the variable set and no timeout passed, it holds.** 147 s and
  counting when the human answered, and the Response came back the same way.

**A held ask is a session at work, which is not what this task file assumed.**
The plan above reads a session sitting on `verkstead ask` as one that draws no
at-work line and is idle. It is the opposite: opencode animates the dial beside
its `esc interrupt` label for as long as the tool holds the command — bytes
every 20–40 ms, and the label standing in the frame, read back through the same
`avt` the Screen is — so such a session reads *at work* by this backend's own
signature and nothing in the run is anywhere near ending it. The open Set is the
second line of defence here rather than the only one, which is the right way
round: it is what stands the day the label moves or the renderer settles. The
suite proves that case rather than the comfortable one — a stub that is
byte-quiet and draws no label for longer than the long-stop, with its Set open.
ADR-0011 carries the measurement.

## What is still waiting

The same half tasks 01 and 02 left waiting, and for the same reason: **a
grilling under an OpenCode Profile launched by Verkstead**. A session's `PATH`
inside the sandbox is fixed and `opencode` is not on it, so the second criterion
is proved everywhere except in a session Verkstead started — the ask, the hold,
the answer gap and the Response on stdout are the real binary's and the real
CLI's, and what is missing is the host installing `opencode` beside `claude`,
`codex` and `grok`, and an account behind it.
