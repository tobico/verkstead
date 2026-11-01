# 03. A session on agy: the Brief, the ask channel, idle

## What to build

With a signed-in Profile from task 02, the first real session: the Brief
reaches the model and the session runs unattended through every state.

- **The Brief.** `--prompt-interactive` is meant to run the initial prompt and
  stay interactive; confirm it submits, and if it only prefills, type the
  submit through the terminal as Verkstead already can.
- **The ask channel, measured.** Inside a session, have agy's shell tool hold
  `verkstead ask` while a Set waits minutes: does it hold for as long as the
  timeout the model passes with no model turn open, or does it yield and poll?
  Set `AgentType::channel` by ADR-0011's rule — blocking where it holds free,
  store-and-nudge where it polls — and give the Guide this backend's own
  *Running the ask* section, saying what timeout to pass if one is needed and
  what the backend's shell tool is called. Where a default timeout would cut a
  hold, raise it underneath the session the way `OPENCODE_EXPERIMENTAL_BASH_
  DEFAULT_TIMEOUT_MS` does, if agy offers such a knob.
- **Idle, measured.** Sample the screen once a second through a turn and at
  rest, with the inline drawing and the running light off. Decide which reading
  the signature takes — an at-work line that goes, or a prompt that stands — or
  whether a resting agy is byte-silent enough for Claude's own three-second
  rule; write the constant and say which, with the agy version beside it, in
  the style of the three before it.
- **The rest of a running session.** Any first-run screen or prompt that still
  parks a session (trust, consent, usage mode) is seeded away. The MCP tool
  naming agy uses for an attached server is read off a session and written into
  the Guide's table. `agy models` lists the account's models: put the real Pro
  and Flash ids into the pairing defaults, Pro for grilling and Flash for
  implementation and review. Leave the usage-limit phrase empty until one is
  seen.

## Acceptance criteria

- [ ] A grilling under an agy Pairing puts a Question Set and reads its Response on the channel the measurement chose, and the Guide printed inside the session says how.
- [ ] A resting session is judged idle by the written signature and a working one is not; the signature is named with the agy version it was read from.
- [ ] A fresh account's session reaches the Brief with nothing parked in front of it, and the pairing defaults name models `agy models` really lists.
