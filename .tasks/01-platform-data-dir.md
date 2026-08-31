# 01. The platform data directory, by default

## What to build

`verkstead serve` started with nothing said keeps its **Data Directory** in the
platform's own place rather than in whatever directory it was started from:
`~/.local/share/verkstead` on Linux, `~/Library/Application Support/Verkstead`
on macOS, `%APPDATA%\Verkstead` on Windows. Everything the Data Directory holds
moves with it — the database, the Worktrees, the installed Skills, the handoff
directories and both settings files — because it is one directory and the only
thing changing is where it is when nobody has said.

**One rule, every entry point.** Nothing branches on which binary parsed the
flags: the desktop binary stage 02 adds gets this default by parsing the same
configuration, and a developer running out of a checkout asks for the old
behaviour with `--data-dir .`. An explicit flag or `VERKSTEAD_DATA_DIR` wins
exactly as it does today.

**Resolved by hand, as a function of values rather than of the process.** No
crate: `dirs` would answer Linux and macOS and then resolve Windows through a
Win32 known-folder call, which this CI cannot compile — let alone run — until
stage 05 puts Windows in it. Write the resolution to take the environment
values it needs as arguments and hand back a path, so every platform arm is
exercised by ordinary unit tests on the Linux runner. Reading the process
environment then happens once, at the edge, and no test has to mutate it —
`std::env::set_var` is `unsafe` under this edition and races the other tests in
its binary, which is why the Build Cache's own resolution has no unit test at
all today.

Linux is XDG, so `$XDG_DATA_HOME` where it is set and absolute and
`~/.local/share` otherwise — the same reading, and the same refusal to resolve
a relative value, that the Build Cache already gives `$XDG_CACHE_HOME`. The
other two platforms have no such variable in the picture.

**Nowhere to resolve to refuses startup**, naming `--data-dir`, exactly as the
Build Cache refuses naming `--build-cache-dir` and for the same reason: a
service unit that says nothing about a home would otherwise be handed a Data
Directory nobody chose and nobody will find. That makes the refusal a startup
error rather than anything the flag parser can express, so the flag holds what
was *said* and the resolution happens where a failure has somewhere to be
worded. The startup log line goes on naming the directory actually chosen — it
is where a human finds out what happened, and now it has something to say on
every run rather than only on a configured one.

**And the words, which this task makes untrue if it leaves them.** CONTEXT.md's
**Data Directory** entry gives the default in its own words. `docs/development.md`
says it twice — in the prose about `--data-dir`, and in the sample startup line
showing `data_dir=.` — and its dev commands want `--data-dir .` spelled out,
because a checkout run keeping its database in the checkout is now something to
ask for rather than something that happens. `docs/adoption.md` names the new
default where it describes getting it running, and its closing pointer to
development.md describes a checkout run in the same stale terms.

## Acceptance criteria

- [ ] With nothing said, the resolved Data Directory is the platform one on each
      of the three platforms, every arm covered by a unit test that runs on the
      Linux CI; the Linux arm honours an absolute `$XDG_DATA_HOME` and ignores a
      relative one.
- [ ] `--data-dir` and `VERKSTEAD_DATA_DIR` win exactly as they do today,
      `--data-dir .` included, and the database is still `verkstead.db` inside
      whichever directory won.
- [ ] A machine with nowhere to resolve to refuses startup with a message naming
      `--data-dir`; every run that does start logs the directory it chose.
- [ ] No entry in CONTEXT.md and no line in `docs/development.md` or
      `docs/adoption.md` describes the default as the working directory, and the
      dev commands pass `--data-dir .`.
