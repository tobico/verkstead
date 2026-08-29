# 01. Foundations

The agent type becomes real end to end, and Claude Code runs on ground that is
no longer Claude's own. The bundled skills mount at `/verkstead/skills` — a
neutral path beside `/verkstead/bin` — with an empty directory bound over
`~/.claude/skills` in its place, so the hiding the old mount did is kept and a
session never rediscovers what the Profile's account keeps there. The skills
themselves stop naming Claude's mechanisms, since the Guide is where per-backend
advice will live. Verkstead passes `--dangerously-skip-permissions` on Claude's
launch line rather than leaning on the account's settings, through a per-type
mapping each later backend adds one arm to. And a Profile's account stops being
a bare pair and becomes a per-type shape, in the store, on the wire and on the
form.

Demonstrable end to end: a Claude session launches unattended with the flag
Verkstead passed, reads its skills from `/verkstead/skills`, cannot see the
account's own, and behaves exactly as before; every saved Profile reads back
unchanged and the form edits one exactly as today, with the shape a later stage
drops a backend into already in place. The form still offers Claude alone — a
type that cannot launch would be a lie in a picker.

Roadmap stage: [01: Foundations](docs/roadmaps/agent-backends/01-foundations.md)

## Tasks

- [ ] 01: The skills mount at `/verkstead/skills` — [details](01-skills-at-a-neutral-path.md)
- [ ] 02: The skills stop speaking Claude — [details](02-skills-stop-speaking-claude.md)
- [ ] 03: Verkstead passes Claude's bypass flag — [details](03-verkstead-passes-the-bypass-flag.md)
- [ ] 04: A Profile's account is per type — [details](04-a-profiles-account-is-per-type.md)
