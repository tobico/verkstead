# 06. Try a server on save

## What to build

Saving an MCP server on the settings page **tries it**, and the section says
what came of that beside the server — the way saving the GitHub token says who
it authenticates as.

The try is one MCP `initialize` request to the server's URL, made by the
Verkstead server itself with the server's headers, over streamable HTTP. It is
made **after** the save has been written.

**It saves either way.** The outcome is told, never enforced: a server that
cannot be reached today is still declared, because it may be reachable tomorrow
or only from inside a session's network. Refusing the save was considered and
turned down.

What is said is one of two things: **reached**, with the name the server gives
for itself where it gives one, or **refused**, with why in words a person can
act on — it did not answer, it answered with a status that means the headers
were not accepted, or what answered was not an MCP server. A header's value
must not appear in any of it.

The try has a short deadline of its own, so a server that never answers cannot
hold the save's response for long.

This is a check at save and nowhere else. Nothing is tried when a server is
attached or when a session launches — an unreachable server never holds a
launch.

## Acceptance criteria

- [ ] Saving a server that answers `initialize` shows it as reached.
- [ ] Saving one that does not answer, that rejects the headers, or that is not
      an MCP server shows it as refused with which of those it was, and the
      server is saved all the same.
- [ ] The request carries the server's headers, and no header value appears in
      what the page is told.
- [ ] A server that never answers is reported within the deadline rather than
      holding the save.
