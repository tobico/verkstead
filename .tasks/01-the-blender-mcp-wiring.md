# 01. The Blender MCP wiring

## What to build

A session that starts on this branch has a `blender` MCP server, and a script
gives that server a Blender to talk to. Nothing of the hammer arrives here: what
this delivers is the tools task 02 will model with, proven to work from this
Worktree, so that task 02 opens with a working `get_scene_info` rather than an
hour of plumbing.

**Why a committed file, and why it is permanent.** A Claude session reads the
repository's `.mcp.json` when it starts and never again, and Verkstead's Sandbox
hands a session a copy of the account's `.claude.json` with `mcpServers` taken
out (ADR-0011) — so neither the account nor the session itself can add a server
mid-flight. The one route is the repository's own: `.mcp.json` at the root
naming the server, and `.claude/settings.json` beside it approving that one
server by name (`enabledMcpjsonServers`), which is the narrow key rather than the
blanket one so a future server somebody adds is not approved by accident. Both
were tried from this machine with a headless `claude -p`, which listed the
server's tools. They stay committed after the icon lands, because the next
re-render is a session that needs them back — the human settled this over
dropping them before the merge.

**The server.** `uvx blender-mcp`, which fetches the `blender-mcp` package
(2.0.0 today) and runs its MCP server; it connects to Blender lazily, on the
first tool call, so a session that never touches the tools spawns the server and
nothing else. Its environment sets `DISABLE_TELEMETRY=1` — the package phones
home unless told not to — and the host and port it dials, which are the loopback
and the port the script below listens on. A session's Sandbox shares the network
namespace, so loopback is the same loopback.

**The host script, and why it exists.** The package bundles the Blender-side
addon, and that addon's server refuses to start under `blender -b`: it drains
its command queue from a `bpy.app.timers` callback, and a background Blender
runs no timers. Its own advice is a virtual display, and that fails here —
Blender under Xvfb gets no GL context in the Sandbox. What works, and was proven
from this Worktree: a script run as `blender -b --python tools/hammer/serve.py`
that imports the bundled `addon.py` by path, registers it, starts the server
with the background check stepped around, and then pumps the queue itself on
Blender's main thread in a sleep loop, for as long as it is left running. The
addon file is found through `uvx --from blender-mcp python`, asking the package
for its bundled addon path, so the addon and the server are always one version.
The port is the addon's default, 9876, unless the script is told another.

The script says at the top what it is and why it is shaped this way, in the
voice the other scripts under `tools/` use: the addon refuses background mode,
Xvfb is not available to a Sandbox, and the loop is the timer the background
mode does not run.

**The development docs** get a short section beside the icon scripts: what
`.mcp.json` and the settings file are for, that a session on this repository
carries a Blender MCP server because of them, and how the serve script is run
and stopped. The three-artworks table stays wrong until task 03, which rewrites
it.

Rejected: installing the addon into a Blender profile (the Sandbox's home is the
account's, and nothing keeps it at the server's version); the blanket
`enableAllProjectMcpServers` (works, approves too much); a Python client that
talks to the addon's socket directly without the MCP (the human asked for the
MCP).

## Acceptance criteria

- [ ] From this Worktree, `claude mcp list` shows the `blender` server without a
      pending approval, and a headless `claude -p` asked which MCP servers it
      sees names it.
- [ ] With `blender -b --python tools/hammer/serve.py` running in the
      background, a socket client sending `{"type": "ping"}` to the port gets
      `pong`, and `{"type": "get_scene_info"}` gets the default scene; the
      script keeps running until it is killed.
- [ ] `.mcp.json` sets `DISABLE_TELEMETRY=1` and dials the loopback and the
      script's port; `.claude/settings.json` approves `blender` alone.
- [ ] `docs/development.md` says what the two files and the script are for.
