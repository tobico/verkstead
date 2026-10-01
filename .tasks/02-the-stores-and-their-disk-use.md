# 02. The stores, and their disk use

## What to build

A language's store directories exist today only inside its variables' values —
and not always as a whole value: Maven's repository is a path inside the line of
JVM options in `MAVEN_OPTS`. Eviction and disk use both need the list, so the
descriptor grammar gains it: each language names **the directories that are its
store**, with placeholders (`{cache}`, `{stores}`), written in the same grammar
an installer writes and merged key by key like the rest of an entry. Every
built-in names its own:

- Rust: `CARGO_HOME` (`{cache}/cargo`) and sccache's `SCCACHE_DIR` (`{cache}/sccache`)
- Go: `GOMODCACHE`, `GOCACHE`
- Node: npm's cache, pnpm's store-dir and cache-dir, Yarn Classic's cache folder, Berry's global folder, `DENO_DIR`, bun's install cache
- Python: pip, uv, poetry, pipenv caches
- .NET: `NUGET_PACKAGES`, `NUGET_HTTP_CACHE_PATH`, `NUGET_SCRATCH`
- JVM: Maven's local repository and `GRADLE_USER_HOME`
- C/C++: none (it is the sccache store, which is Rust's)

The pane shows each language's **disk use beside its size**. Measuring a large
store is a walk of many thousands of files, so it happens **in the background**
— cached, refreshed on a pace of its own and after anything that changes a
store (the sweep and Clear, later) — and **the settings read never waits on
it**: it returns the last figure, or says the store has not been measured yet.
Disk use is a server-side fact the page reads, like `taking`.

**Never follow a symlink** when walking a store: pnpm's `v11/projects/` holds
relative symlinks to project directories (Worktrees), bun keeps absolute
symlinks into its own cache, and uv's `wheels-v6` entries are symlinks to
`archive-v0`. Count a hardlinked file once per store (pnpm, deno, bun and uv
link out of their stores where they can). A store directory that does not exist
yet is zero, not an error. The walk will be shared with the sweep (task 04),
which wants the same files with their times, so shape it to be reused.

## Acceptance criteria

- [ ] The grammar names a language's store directories, every built-in lists its own (above), an installer can add or null one out key by key, and a descriptor naming none is still valid.
- [ ] The pane shows each language's disk use beside its size, measured in the background; a settings read with a measurement in flight returns at once with the last figure or "not measured yet" (proven with a slow or blocked measurement).
- [ ] The walk does not follow symlinks (a symlink to a large directory outside the store adds nothing) and counts a hardlinked file once.
