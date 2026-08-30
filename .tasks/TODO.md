# 02. Asking and idling

The two mechanisms every full-screen-TUI backend needs come into being, ahead
of the backend that will use them. **Store-and-nudge asking**: an ask on a
backend that cannot hold a shell command open for hours is stored and returns
at once, the session ends its turn, and when the Response lands Verkstead types
one line into its terminal telling it to fetch the Answers with `verkstead
answers`. It rides the Deferred Ask machinery and is a state of its own inside
it, because a Set nobody is idling on and a Set a session is idling on mean
opposite things to the enders, to Rescue and to the wrap-up. And
**screen-signature quiet**: for a backend that repaints for ever, idle is that
backend's at-the-prompt signature read off the Screen Verkstead already holds,
with a long byte-quiet behind it as the long-stop that catches a signature
which has drifted. The Guide learns to say which channel a session is on, and
`codex` becomes a word the store knows so the suite has a second backend to
prove all of it against.

Demonstrable end to end: a stub session on a store-and-nudge backend asks, ends
its turn, is neither reaped nor rescued while the Set stands, is nudged when the
Response lands and fetches it — or, having died first, its Answers fold into the
next session's prompt; `verkstead guide` inside that sandbox names that channel
where a Claude one still names the blocking ask; and a stub drawing a TUI-shaped
screen is judged idle at its signature, is not ended by a three-second byte
silence mid-turn, and is caught by the long-stop when its signature is taken
away. Claude Code behaves exactly as it does today throughout.

Roadmap stage: [02: Asking and idling](docs/roadmaps/agent-backends/02-asking-and-idling.md)

## Tasks

- [x] 01: `verkstead answers` — [details](01-verkstead-answers.md)
- [ ] 02: The agent type reaches the sandbox, and the Guide is tailored to it — [details](02-the-guide-per-backend.md)
- [ ] 03: The ask channel per agent type — [details](03-the-ask-channel.md)
- [ ] 04: The nudge — [details](04-the-nudge.md)
- [ ] 05: Screen-signature idle, and the byte-quiet long-stop — [details](05-screen-signature-idle.md)
