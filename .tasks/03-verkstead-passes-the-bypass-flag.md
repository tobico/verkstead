# 03. Verkstead passes Claude's bypass flag

## What to build

Unattended is the product's promise rather than the account's configuration, so
Verkstead says so on the launch line instead of hoping the Profile's own
settings do. What stops a session doing harm is the Sandbox, which is unchanged
and stays the boundary; what a bypass flag stops is a session halting to ask
approval in front of nobody, which stalls a run with the backlog behind it.

**The launch line becomes the backend's to shape.** The argv builder gains a
mapping keyed on the agent type of the Pairing's Profile, saying what flags
that backend needs beyond the model, the prompt and the session name. Claude's
is `--dangerously-skip-permissions`. Each later backend adds one arm and
nothing else — that is the whole reason the mapping exists rather than a flag
being pushed straight into the line.

**The flags go on the end**, after the session name, so nothing already on the
line moves. Claude reads its options on either side of the positional prompt,
and everything that already reads this line — every stub the test suite stands
where the agent goes — finds the model and the Brief at the positions they are
at today. The builder's own comment already says options added here go on the
end for exactly this reason.

**The program stays separate from the flags.** What a Profile's agent is run as
is a field, so that a test can stand its own script where the real agent goes
and prove that a session's output reaches the Timeline without needing an
account, a network and a model's patience. That stays true: the mapping
contributes the flags, the field contributes the program, and a stub therefore
sees the same flags a real session does.

Nothing else in the run depends on a Profile's permission settings — nothing
reads them at all — so this line is the whole of the change.

## Acceptance criteria

- [ ] A session launched under a Claude Profile runs with
      `--dangerously-skip-permissions` on its command line, passed by Verkstead
      with nothing in the account's own configuration saying so.
- [ ] The flag lands after the session name, and every stub agent in the test
      suite still reads the model and the Brief off the positions it reads them
      off today — the existing session tests pass without being re-indexed.
- [ ] A stub agent can see the flag on its own line, so what the mapping does is
      provable without a real account.
- [ ] Adding a later backend's flags is one arm of the mapping, and the type it
      is keyed on comes off the Pairing's Profile rather than being plumbed
      through separately.
