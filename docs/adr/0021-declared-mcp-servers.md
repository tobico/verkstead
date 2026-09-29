# Declared MCP servers

Amends [ADR-0011](0011-agent-backends.md), which says a session is given no MCP
servers at all: the `.claude.json` copy has `mcpServers` taken out at the top
level and under each `projects` entry, the written configuration file carries
none, and neither is ever written back. That rule stands — **the account's own
servers are still never carried into a Built Root, and still never touched on
the way out.** What this adds is a way in that is not the account's: a server
declared in Verkstead's own `config.yaml` and attached to the Conversation that
wants it.

The reason the account's are stripped is that a Built Root is an allowlist. A
human's `~/.claude` holds plugins, hooks, skills and servers they chose for
themselves, and a session that inherited the lot would be running under a
configuration nobody had decided on for it — see ADR-0011's *the account is
built rather than joined*. An MCP server is the sharpest case of that: it is a
credential and a network reach, handed to an unattended agent.

But the capability itself is not the problem, and taking it away left no way to
give an agent the one tool it actually needed for a piece of work. So the
capability comes back as something **said** rather than something **found** —
which is the rule `crates/server/src/settings.rs` is built on, and the same move
the git author and the global instructions were made by.

## The decision

An **MCP server** is declared once for the installation, in a new section of the
settings page, and **attached** to a Conversation from the Attach button where
its Brief is written. Every session of that Conversation, on whichever harness
runs it, is launched with the servers that Conversation attached — and with
nothing else.

Settled in the grilling of 2026-09-29. Each of the following was decided there,
and the alternative named is what was turned down.

- **HTTP only. No stdio.** A declaration is a name and a URL, and the form
  offers no command, no arguments and no transport to choose. A stdio server is
  a child process the agent starts inside its own sandbox: an executable that
  would have to be on the session's `PATH`, inside a namespace built to hold
  exactly what Verkstead put there, doing whatever that binary does. That is a
  hole in the sandbox rather than a setting, and it is the sandbox that makes an
  unattended session safe to run at all. Remote servers are reached over the
  network the session already has.

- **Static headers only. No OAuth.** A declaration carries header values typed
  once and sent as they were typed. An OAuth flow needs a browser, a redirect
  and a token that expires, none of which a machine working through a backlog at
  three in the morning has. A header is what a service's own documentation gives
  for exactly that case.

- **Header values are secrets, and they never come back.** They go the way the
  GitHub token goes — written where a secret is written, never returned to the
  page, never in a view, never in a log. What comes back about one is that it is
  set. **But they are readable by Verkstead**, because a session has to be
  handed them: this is a secret kept from the page and the wire rather than a
  secret kept from the machine, and it is not hashed.

- **A chip is a reference by name, read at each launch.** What a Conversation
  stores is the name it attached, and the URL and headers are read out of
  `config.yaml` at the moment a session starts. So correcting a URL fixes every
  Conversation that attached that server, and nothing goes stale. Which is why
  **the name is the identity** — lowercase letters, digits and hyphens, unique
  among the declarations, refused at the moment it is typed if it is neither —
  and why **a server is never renamed**: every chip pointing at it refers to it
  by that name, and changing one is deleting the declaration and making another.
  The alternative was copying the URL and headers onto the Conversation, which
  would have frozen a credential into a record and left a rotated one broken
  everywhere.

- **Servers belong to the Conversation, and freeze with its Brief.** Attached
  and removed while the round drafts, fixed once the work starts, and drawn as a
  read-only row afterwards — exactly what an Attachment does, and for the reason
  an Attachment does it: what a session was given is part of the record of what
  it did.

- **Every session of the Conversation, on all four harnesses.** Not the grilling
  alone and not the implementation alone. A Conversation that needed a server to
  understand the work needs it to do the work and to review it, and a rule that
  held for one role would be one somebody had to remember.

- **All of a server's tools are allowed. No per-tool filter.** What a filter
  would be protecting against is the server itself, and the decision to trust
  one was made when it was attached. A per-tool allowlist is also a list that
  goes stale the first time the server adds a tool, silently, with nothing to
  say so.

- **An unreachable server never holds a launch.** A session starts, is handed
  the configuration, and whatever the harness makes of a server that will not
  answer is the harness's own business to say inside the session. The
  alternative — a reachability check at launch — would make a network blip a
  Conversation that did not start, which is the failure mode this whole system
  exists to avoid. What does get tried is a declaration **on save**, once, while
  the human is looking at the page: that is the moment a wrong URL is worth
  saying something about, and it is a report rather than a refusal.

- **And one rule for later: a harness that cannot take an HTTP MCP server is
  launched without it, and the chip says that harness does not take it.**
  Nothing is built for this now, because all four harnesses take one. It is
  recorded so that the fifth is a line in a table rather than a decision taken
  again — and so that the answer, when it comes, is a session that runs and a
  chip that says what it did not get, rather than a launch refused.

## What this does not change

The strip and the write-back are ADR-0011's, unchanged. The account's
`mcpServers` are taken out of the `.claude.json` copy at both levels, are never
written or removed on the way back, and the configuration file Verkstead writes
still carries nothing of the account's beyond what it says about reaching a
model. A session with no attached server has no MCP server at all, which is
every session that ran before this and every Conversation that attaches none.

The declarations live in `config.yaml` and follow that file's contract: read at
the moment they are needed, so a change reaches the next session without a
restart, and an absent, empty or unparseable key is no servers rather than an
error.
