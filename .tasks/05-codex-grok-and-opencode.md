# 05. Codex, Grok and OpenCode

## What to build

What tasks 03 and 04 gave a Claude session, the other three harnesses get: the
Conversation's attached servers, with their headers, written into the Built
Root in the form that harness reads. One declaration, translated per harness.
Kept as one task by choice — each is a small addition to a configuration file
Verkstead already writes for that root, and the three share a shape.

The forms, as checked on 2026-09-29 against codex 0.155.1, Grok Build 1.0.34
and opencode 1.18.31:

**Codex**, in the root's `config.toml`:

```toml
[mcp_servers.<name>]
url = "<url>"
http_headers = { "Authorization" = "Bearer …" }
```

**Grok Build**, in the root's `config.toml`:

```toml
[mcp_servers.<name>]
url = "<url>"
headers = { "Authorization" = "Bearer …" }
```

**OpenCode**, in the root's `opencode.json`:

```json
{"mcp": {"<name>": {"type": "remote", "url": "<url>", "oauth": false,
                    "headers": {"Authorization": "Bearer …"}}}}
```

`oauth: false` stops OpenCode looking for an OAuth login on a server that
authenticates by header.

**What is proven and what is not.** Grok and OpenCode were seen to connect to a
probe server and send the headers. **Codex was seen to parse the form, and its
headers were not seen on the wire** — prove that here, against a local server
that records what it receives, before calling Codex done.

**Tools must be usable without a prompt.** Each harness already launches with
its permission bypass, and whether that covers MCP tools was not established
for Codex or OpenCode. Codex has a per-server `default_tools_approval_mode`;
find out whether it is needed and write it if it is.

**The account's own servers are still stripped** for each of the three, as they
are today — what goes into a root is only what Verkstead wrote.

**The prompt names tools the way the harness does**, which differs for each:
`mcp__<name>__<tool>` on Codex, `<name>__<tool>` on Grok Build, and
`<name>_<tool>` on OpenCode.

Everything else is as task 03 has it: read at launch by name, a deleted server
skipped, every session of the Conversation, nothing checked first.

If one of the three turns out not to be able to do this after all, the ADR
from task 01 has the rule: the session launches without the server and the chip
says that harness does not take it. That would be a question for the human
rather than something to build unasked.

## Acceptance criteria

- [ ] A root built for each of Codex, Grok Build and OpenCode holds the
      attached servers and their headers in that harness's form, and none of
      the account's own.
- [ ] Each harness, run against a local server that records its requests, is
      seen to connect and send the headers — Codex included.
- [ ] A session on each harness can call an attached server's tool with no
      approval prompt.
- [ ] The prompt on each harness names the tools in that harness's own form.
- [ ] A Conversation with nothing attached builds, for each harness, the root
      it built before.
