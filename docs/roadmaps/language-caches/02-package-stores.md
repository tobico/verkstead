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
  the second is asserted to have fetched nothing — or, where a tool offers no
  way to tell, to have found the store populated.
- **Accepted: one session can plant a package another installs.** The adoption
  doc says so in this stage, since this is where it becomes true of more than
  Rust.
- **How languages group into checkboxes was not asked.** Node, Deno and Bun
  were named together in the Brief and may be one language or three; Python's
  four tools are surely one. The stage's own grilling settles it.

Which variable each tool reads is not recorded here on purpose. It is a fact
about each tool's current release, checked against its documentation when the
stage starts and then proven by the real install.

## Proposed tasks (provisional)

1. **The tools in the dev shell and CI.** AC: each tool answers inside a
   Sandbox on Linux; a proof whose tool is missing is skipped and says so;
   CI's time is measured before and after.
2. **Go.** The first descriptor, because its store is the simplest and it has
   both halves. AC: a second Sandbox's build fetches nothing; two at once leave
   a store a third can use.
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
  store, and for whether the store is safe for concurrent writers.
- Check which tools keep more than a store under the directory the variable
  names — configuration, credentials — since a shared one must hold nothing
  that is a session's own.
- Check what the dev shell's nixpkgs carries for each tool and what CI's
  runner image has already.
