# 04. The vendor installers as the user

## What to build

The two Linux rows that are not a package: Claude Code through Anthropic's own
installer, and Grok Build through xAI's. Each is a unit of the run after the
elevated batch, run **as the user** with the server's own environment, never
elevated, and each reports its phase on the status line as *running <vendor>'s
installer*.

Anthropic's installer is the command the wizard already shows, `curl -fsSL
https://claude.ai/install.sh | bash`, and it lands in `~/.local/bin`. xAI's is
the script its install page at `https://x.ai/cli` names — verify the exact
line rather than guessing it — and lands in `~/.grok/bin`, with a link into
`/usr/local/bin` only where it can write there, which an unelevated run cannot.
Either way, **the directory the installer lands in is written to
`session_path`** through task 01, so the row goes present on the next probe and
a session finds the program from then on, with no shell profile to edit and no
restart. A run of the installer that exits non-zero fails the row with its
first line of stderr, and the hint screen carries the instruction as before.

Claude's packaged alternative — `npm install -g @anthropic-ai/claude-code` —
is not what a ticked row runs; the native installer is the one that stays
current, which is why the tab already leads with it.

The suite stands the run up over a stated machine whose installer commands are
stubs that drop a program into the directory the real one uses, so the
`session_path` write and the re-probe are proven without the network.

## Acceptance criteria

- [ ] With the sandbox, git and Claude ticked on a stated Ubuntu, the run is the
      elevated batch and then a unit running Anthropic's installer as the user,
      and after the stub lands `claude` in `~/.local/bin` the row reads Present
      there, `session_path` holds the directory, and it is on a session's `PATH`.
- [ ] A ticked Grok row runs xAI's installer, and `~/.grok/bin` reaches
      `session_path` the same way.
- [ ] An installer that exits non-zero leaves its row failed with the line it
      printed, and the other rows unaffected.
