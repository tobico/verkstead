# 02. Package stores

## Goal

A session that installs packages with npm, pnpm, yarn, deno, bun, pip, uv,
poetry, pipenv, Go or NuGet downloads each one once for the machine. A second
Conversation against the same Repo installs from the store the first one
filled, and two sessions installing at once do not damage it. Go's build cache
is shared too, which is compiled output that is only a directory.

Each of these is a built-in descriptor and nothing else — no new behaviour in
the server — which is what makes this stage the proof that stage 01's system
is enough.

## Decisions in force

From [ADR-0021](../../adr/0021-language-descriptors.md).

- **These eleven tools, in one stage.** Splitting by ecosystem was offered and
  turned down: each tool is a descriptor and a test.
- **Everything each ecosystem can cache.** For these tools that is the package
  store, plus Go's build cache. NuGet has no compiled half to share.
- **On by default, for every session, on all three platforms.**
- **Proven two ways.** Assertions on the environment a session is given, on
  every platform; and a real install per tool inside a Sandbox on Linux,
  skipped where the tool is missing. The dev shell and CI on Linux gain the
  tools. The reason is who this is for: the maintainer builds only Rust, so a
  tool that ignores its variable would be found by a stranger.
- **The real install is what catches a tool ignoring its variable, and two
  sessions colliding.** So each proof installs twice, from two Sandboxes, and
  **the second install cannot reach its registry** — an offline flag where the
  tool has one (`npm --offline`, `pip --no-index`, `uv --offline`,
  `GOPROXY=off`, `deno --cached-only`), and a source pointed somewhere
  unreachable where it has none. An install that succeeds with nothing to fetch
  from succeeded out of the store, and one that reached past the store fails.
  *Found the store populated* is not the fallback: the store is populated
  because the first install populated it, so that assertion holds whether the
  second install read one byte of it or fetched the lot. This is the one proof
  the stage rests its case on, and a branch of it that passes whatever the tool
  does is where a tool quietly ignoring its variable would get through.
- **Accepted: one session can plant a package another installs.** The adoption
  doc says so in this stage, since this is where it becomes true of more than
  Rust.
- **How languages group into checkboxes was not asked.** Node, Deno and Bun
  were named together in the Brief and may be one language or three; Python's
  four tools are surely one. The stage's own grilling settles it.

Which variable each tool reads is not recorded here on purpose. It is a fact
about each tool's current release, checked against its documentation when the
stage starts and then proven by the real install.

## Settled when the stage started

The grilling of 2026-09-30, which is what the open questions above were left to.

- **The eleven tools are four descriptors** — `go`, `node`, `python`, `dotnet` —
  so one box on the **Language support** pane covers an ecosystem. Node, Deno
  and Bun are one language, not three: turning Node off turns all six
  JavaScript tools off — npm, pnpm, both yarns, deno and bun. Rejected: an entry
  per runtime, and an entry per tool.
- **One scenario per tool, not two.** Two installs at once in two Sandboxes,
  then a third denied its registry. That proves the store was read and that two
  writers did not damage it in three installs rather than five.
- **A skipped proof is green locally and red in CI.** A checkout run on a
  machine that builds only Rust stays green; a tool leaving CI's list cannot do
  so quietly.
- **CI gets a job of its own on Linux**, installing what the runner image lacks,
  rather than more steps in the Rust job.
- **.NET's `detect` list is empty**, and the YAML says why: `detect` matches
  literal filenames and a .NET project is `*.csproj` or `*.sln`. Rejected: globs
  in the grammar, which would be the new server behaviour this stage exists to
  show is unnecessary.
- **Both yarns, in one descriptor.** Classic and Berry read different variables,
  and the entry carries both.
- **poetry and pipenv are told to keep the virtual environment in the project.**
  Both otherwise keep venvs under the directory their cache variable names, and
  a venv holds absolute paths, so a shared one is broken in every Worktree but
  the one that built it. A descriptor's value does not have to be a path, so
  this needs no new placeholder.

  *Half of that premise did not survive the stage.* poetry's default place for a
  venv really is `{cache-dir}/virtualenvs`, so its variable is what keeps a
  Conversation's environment out of the shared store. pipenv's is not: it keeps
  environments under `WORKON_HOME` in the session's own home, so a shared cache
  was never a shared venv there. `PIPENV_VENV_IN_PROJECT` is set anyway, and
  deliberately — it buys an environment that outlives the session, and one
  sentence then covers both of these tools rather than two. See
  `crates/server/languages.yaml`.

## Proposed tasks (provisional)

1. **The tools in the dev shell and CI.** AC: each tool answers inside a
   Sandbox on Linux; a proof whose tool is missing is skipped and says so;
   CI's time is measured before and after.
2. **Go.** The first descriptor, because its store is the simplest and it has
   both halves. AC: a second Sandbox's build succeeds with `GOPROXY=off`, so it
   fetched nothing; two at once leave a store a third can use.
3. **Node: npm, pnpm, yarn, deno, bun.** AC as Go's, per tool; a Repo pinning
   its own store location in its own config still wins.
4. **Python: pip, uv, poetry, pipenv.** AC as Go's, per tool; virtual
   environments stay in the Worktree, only downloads are shared.
5. **NuGet.** AC as Go's.
6. **The docs.** The adoption doc lists what is shared per tool and says what
   a shared writable store means.

## Re-verify at start

- Assumes stage 01 landed, and that a built-in is added by adding YAML and a
  test with no change to the loader. If it is not, that is stage 01's gap to
  close first.
- Check each tool's current documentation for the variable that moves its
  store, for whether the store is safe for concurrent writers, and for how an
  install is denied its registry — the offline flag it documents, or a source
  pointed nowhere — since that last is what every proof here turns on.
- Check which tools keep more than a store under the directory the variable
  names — configuration, credentials — since a shared one must hold nothing
  that is a session's own.
- Check which tools hardlink out of their store into the project rather than
  copying — pnpm, bun and uv all do — and what each does where the store and the
  Worktree are on different filesystems, which the Build Cache and the Worktrees
  are free to be. A silent fall-back to copying leaves the store working and the
  point of it gone, and none of the proofs above can see it: a copy out of the
  store still fetches nothing. What can see it is the install's own words — uv
  and pnpm both say when they could not hardlink — or the link count on a file
  in the project. Stage 01 left the grammar able to take a second placeholder if
  this is what it needs.
- Check what the dev shell's nixpkgs carries for each tool and what CI's
  runner image has already.
