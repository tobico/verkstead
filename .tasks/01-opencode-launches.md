# 01. The `opencode` type launches

## What to build

An OpenCode Profile that runs the real `opencode`. Today the agent type has
three values in it, the launch line has an arm apiece for them, and the form's
picker offers those three — so an OpenCode Profile cannot be written at all.

**First, put `opencode` on the machine.** The host provides each backend's
binary the way it provides `claude`, `codex` and `grok` — on the system profile
the sandbox already reads — and this one is not there. Nothing below can be
proved against a stub alone, so installing it is this task's first step rather
than a prerequisite somebody else is assumed to have met. Pin what release
lands, and name it in whatever constant this task writes, the way codex 0.149.0
and grok 1.0.13 are named in theirs.

Then five things, one slice:

- **The type, and the account shape it names.** A fourth value beside Claude,
  Codex and Grok Build, whose account is one relocatable home — the same shape
  the last two have, and the table hung off `profiles` that holds it needs
  nothing new. What is different is where that home *lands*: opencode keeps no
  single dot-directory. It reads four XDG directories — config, data, cache and
  state — and appends `opencode` to each, and it creates all four at startup.
  The account is the data one: `auth.json` is there, and OAuth and API-key
  logins both write to it.

  The sandbox's HOME is made fresh and empty, so the XDG defaults all resolve
  inside it, and binding the Profile's directory at those default paths needs no
  environment variable at all. That is the second of the two shapes ADR-0011
  allowed, and it is the cheaper one. Decide what of the Profile's one directory
  goes where — at minimum its config and its data — and write down why the rest
  is or is not bound.

- **The line opencode actually takes.** The Pairing's model as `-m`, in the
  `provider/model` form the human types on the Profile; the Brief as
  `--prompt`; and `--auto` for the approvals, which is where every other
  backend's bypass already sits. `--prompt` auto-submits on v1.18.25 — the home
  screen fills the prompt and submits it once the model store is ready — so
  nothing has to type a submit through the terminal. **Confirm that on the real
  binary before relying on it**; if it only prefills, Verkstead types the submit
  through the terminal it already has.

  The prompt is a *flagged* argument here, where all three backends before it
  take it as the one positional. The line builder writes the positional
  unconditionally, so it needs a way to say which a backend takes.

- **Pin the database the account writes.** opencode names its store after the
  channel the install came from — a beta build writes a differently-named file
  beside a stable one's — and it reads a variable that pins the name instead.
  Set it, so that task 04's reader opens a file Verkstead chose rather than
  guessing which of several is this session's.

- **The blocking channel, and no usage-limit phrase.** OpenCode's shell tool is
  synchronous and holds no model turn open, so its channel is
  [`Blocking`] — the same one Claude has, and nothing to build here beyond
  naming it. Its usage limits are provider-shaped and retried internally before
  anything surfaces, so this backend ships with **no phrase at all**: the
  matcher skips a backend that has none rather than matching it against the
  empty string, which is already how that mapping reads. Such a stop lands as an
  ordinary stall until a wording is observed.

- **The form offers the type.** A fourth row in the picker, drawing this type's
  own path row under it — one home, as Codex's and Grok Build's are. Adding it
  is the last thing this task does: the rule is that a type is offered only once
  it can launch the real thing.

**No skills bind is needed, and that follows the rule rather than skipping it.**
opencode discovers global skills under `~/.claude/skills` and `~/.agents/skills`
— Claude-shaped directories, under HOME rather than under its own account — and
for an OpenCode Profile neither path is bound into the sandbox, so there is
nothing there to hide and an empty directory over either would cover nothing. Its
own skill directory sits inside the config directory the Profile names, which is
the exception ADR-0011 already carries for Codex and Grok Build. Say so in the
comment that keeps that rule, so a later reader can tell a considered absence
from a forgotten bind. The skills themselves need nothing: the prompt names
`/verkstead/skills/...` and opencode reads whatever path it is given.

**A session id is not asked for.** opencode has a `--session` flag, but it means
*continue this one* and the TUI validates it before starting, so a fresh name is
unlikely to be taken the way grok takes one. Confirm cheaply while the binary is
in front of you; if it does take one, say so in the task 04 notes rather than
changing this line, because the discovery task is where that would pay.

## Acceptance criteria

- [ ] `opencode` runs on the machine, and the release it is pinned at is named
      wherever this task writes a constant read off it.
- [ ] An OpenCode Profile is saved from the form — the type picked there rather
      than over the API — paired with a Conversation, and launched; its Capture
      shows opencode running, on the Pairing's model, under that Profile's own
      account rather than any of the host's.
- [ ] The Brief reaches the session and it starts working on it with nothing
      typed into the terminal; the line carries `-m`, `--prompt` and `--auto`,
      and Claude's, Codex's and Grok Build's lines are unchanged with the
      suite's stub agents still reading what they read today.
- [ ] A launched session leaves a store under the Profile's data directory, in
      the database whose name Verkstead pinned — which is what task 04 reads.
