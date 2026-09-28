# 04. The account mirror, and a session away from home

## What to build

B launching under a **mirror** Profile keeps a mirror of that account's login
and configuration **under its own Data Directory**, fetched from the home device
before each launch, and builds the session's Built Root out of it exactly as it
builds one out of a local account. The row travels and the account does not — so
what a session away from home gets is a copy of the little of an account a Built
Root is made from, made fresh each time.

**A peer route of its own, gated to members.** It carries the account's files
rather than a view of them, so it is a member's own call rather than anything in
the viewer namespace — the shape the news a member tells its members already
has. What it hands over is only what the Built Root's allowlist names: the login
file where the account has one, and what the written configuration is composed
from. Nothing else of the account travels — no plugins, hooks, rules, skills,
global instructions file, history, or any other repository's transcripts. The
memory store is task 06 and is not on this route's list.

**Per Profile, under the Data Directory**, which is what makes Windows' rule
hold by construction: the profile a session's root is built in is under that
same directory, so the hard link joining the login into it never crosses a
volume, and the check that refuses an account elsewhere cannot fire for a
mirror.

**Fetched before each launch**, rather than once and kept: a login refreshed at
home since the last session is the one this session has to be given, and a
mirror that went stale would sign the session out for no reason anybody could
see.

**Downstream of the launch nothing is special.** The mirror directory *is* the
account as far as the root is concerned — its four parts, the trust seeded into
the copied config, the launch line and the model it runs are what they already
are for an account on this machine.

## Acceptance criteria

- [ ] Start on B under a Profile whose account lives on A runs a session, and
      that session is logged in.
- [ ] A login refreshed at home between two sessions on B is the one the second
      session gets.
- [ ] Nothing lands in the mirror but the files a Built Root is made of, checked
      against an account with plugins, hooks and other repositories' transcripts
      in it.
- [ ] On Windows the mirror is on the Data Directory's volume, so the refusal
      for an account across volumes never fires for one.
