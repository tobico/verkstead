# 01. The `grok` type launches

## What to build

A Grok Build Profile that runs the real `grok`. Today the agent type has two
values in it, the launch line has an arm apiece for them, and the form's picker
offers those two — so a Grok Profile cannot be written at all.

Four things, one slice:

- **The type, and the account shape it names.** A third value beside Claude and
  Codex, whose account is one relocatable home — the same shape Codex's is, and
  the table hung off `profiles` that holds it needs nothing new. Grok Build
  keeps its whole account under `~/.grok`: a subscription login writes
  `auth.json` there, an API key would come by environment, and the Profile's
  directory is the account either way. The home is bound over `~/.grok` inside
  the sandbox.

- **The line grok actually takes.** The prompt as the one positional, the
  Pairing's model as `-m`, `--always-approve` and `--sandbox off` (grok's own
  sandbox will not start where bwrap is unavailable inside, and bwrap is
  already the boundary), and `--no-alt-screen` so the Capture stays a record
  somebody can read — stage 03's finding, and grok documents the flag.

- **The session named at launch.** Grok Build is the only one of the new
  backends that takes a session id, and it takes it under the spelling the line
  builder already writes — so this type says it names its session and the id
  goes on the line. It has to be a valid UUID and it has to be new: grok
  refuses a malformed one and refuses one it already has a session for.
  Verkstead's own names are version-4 UUIDs drawn fresh per session, so both
  hold — but check the real binary takes the flag *after* the positional
  prompt, which is where this line builder puts it, and that it takes it in
  interactive mode rather than only under `-p`.

- **The form offers the type.** A third row in the picker, drawing this type's
  own path row under it — one home, as Codex's is. That is the whole of what
  the rule about a type being offered only once it can launch was waiting for.

**Store-and-nudge comes free.** Grok Build's shell tool yields after seconds
and has the model poll, so its channel is store-and-nudge — which is a fact
about the type and nothing to build here beyond naming it: the Guide, the
stored Set, the nudge and `verkstead answers` are all stage 02's, and all of
them read the channel off the agent type.

**The home is left as the account keeps it.** Grok Build discovers skills in
`~/.grok/skills`, which is inside the home the Profile names, so ADR-0011's
exception applies exactly as it does to Codex: covering it would hide the
skills grok itself ships. Nothing is bound over anything inside a Grok home,
and the ADR says so as of this stage's plan. Nothing special is needed for the
skills themselves either — grok reads a Claude-shaped skill directory and reads
whatever path it is given, so the prompt naming `/verkstead/skills/...` is
enough.

## Acceptance criteria

- [ ] A Grok Build Profile is saved from the form — the type picked there
      rather than over the API — paired with a Conversation, and launched; its
      Capture shows grok running under that Profile's home.
- [ ] The line carries the prompt as the positional, the model as `-m`, the
      session id Verkstead named, `--always-approve`, `--sandbox off` and
      `--no-alt-screen`; Claude's and Codex's lines are unchanged and the
      suite's stub agents still read the line they read today.
- [ ] A launched session's own directory appears under the home's `sessions/`
      store, named by the id Verkstead gave it — which is what task 03 looks
      it up by.
