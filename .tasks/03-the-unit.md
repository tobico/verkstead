# 03. The unit, in the grammar and the built-ins

## What to build

Each store directory named in task 02 says **how it is bounded**, and the pane
says it per language:

- **by its tool** — handed the size and left alone. sccache is the only one:
  `SCCACHE_CACHE_SIZE`, LRU by size. (Checked per tool at planning: Cargo 1.88+,
  Go's build cache and Gradle clean up by *age* only and no other tool evicts at
  all, so none of them counts.)
- **by unit** — the sweep (task 04) removes whole units, oldest first.
- **not at all** — a store naming no unit is never swept, and the pane says so.

**A unit is named by a depth or by a marker** (decided in Set 996, amending
ADR-0021's "a directory depth"): either *every entry at depth N* under the store,
or *the first directory down whose name, or whose contents, match a pattern* —
the walk stops descending at a unit. Spell the grammar; it is data, so it says
patterns and never a command. **Never by file**, except where the store is
**content-addressed and its tool verifies every blob it reads**, where one blob
file is the unit (also Set 996). Rust's **cargo half is swept** under Rust's
size, separately from sccache (Set 996), so a Rust machine holds up to twice its
size.

What planning measured, per store, by installing with the real tools (re-check
anything that looks off against the real install proof this task adds):

| Store | Unit | Notes |
|---|---|---|
| `CARGO_HOME` | `registry/src/<reg>/<crate-ver>/`, `registry/cache/<reg>/<crate-ver>.crate`, `git/db/<name>-<hash>/`, `git/checkouts/<name>-<hash>/<rev>/` | fixed depths 2–3; git packs are mode 0444 |
| sccache | by its tool | |
| `GOMODCACHE` | a directory named `*@*` (extracted `path@version`, and `cache/download/<path>/@v/`) | variable depth; **0555 dirs / 0444 files** |
| `GOCACHE` | blob files in the `00`–`ff` shards | content-addressed |
| npm `_cacache` | blob files under `content-v2/sha512/<2>/<2>/` | a stale index entry is a clean miss |
| pnpm store-dir | blob files under `v11/files/<2hex>/` | a shard holds a slice of every package, so never a shard; `projects/` symlinks into Worktrees |
| pnpm cache-dir | the `<name>.jsonl` metadata file | scoped names one level deeper |
| Yarn Classic | `v6/npm-*-integrity/` (depth 1) | a partial entry **silently** installs a broken package |
| Yarn Berry | `cache/*.zip` (depth 1) | Berry was not on PATH at planning; check it |
| `DENO_DIR` | `npm/<reg>/<name>/<ver>/` (a version is beside a `registry.json`; scoped is one deeper), `remote/<scheme>/<host>/<hash>` files | partial version breaks at run time |
| bun | `<name>@<ver>@@@1/` (scoped is one deeper) | absolute symlinks `<name>/<ver>@@@1` beside them; partial unit installs broken silently |
| pip, pipenv | leaf hash directories under `http-v2/` and `wheels/` (pipenv also has a pip-style cache at its root) | |
| uv | `archive-v0/<id>/` | `wheels-v6` symlinks into it; partial archive fails install |
| poetry | leaf dirs under `artifacts/` and `cache/repositories/<repo>/` | |
| `NUGET_PACKAGES` | `<id>/<ver>/` (holds `.nupkg.metadata`) | partial version fails the build |
| NuGet http | `.dat` files at depth 2 | |
| `NUGET_SCRATCH` | not swept, or depth 1 | temp files and locks |
| Maven repository | a directory holding a `*.pom` or `_remote.repositories` | groupId puts it at depth 3–6+; **a jar's mtime is the server's Last-Modified**, not the download time |
| `GRADLE_USER_HOME` | `caches/modules-2/files-2.1/<group>/<module>/<ver>/` (depth 3), and the rest of the home's caches as fits | Gradle records use in its own journal, not in file times |

Whatever cannot be given a unit safely is left unswept and says so rather than
guessed at.

The demonstrable end of this slice, before any sweep exists: the pane says, per
language, how each store is bounded, and **a test populates every store with
its real tool inside a Sandbox (as the stage 02–04 proofs do, skipped where the
tool is missing) and asserts each unit found is one whole package** — every
file a package installed is inside exactly one unit, and no unit spans two
packages, except where the store is content-addressed.

## Acceptance criteria

- [ ] The grammar names a unit by depth or by marker (or none), and a store as bounded by its tool; every built-in store has one of the three, and a descriptor naming no unit is never swept and the pane says so.
- [ ] Listing the units of a real store that each tool filled in a Sandbox finds whole packages for every built-in, with no symlink followed and the walk stopping at a unit.
- [ ] The cargo half of Rust names its units and is shown on the pane as swept under Rust's size, separately from sccache's own eviction.
