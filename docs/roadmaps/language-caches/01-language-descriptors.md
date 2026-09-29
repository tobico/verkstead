# 01. Language descriptors

## Goal

A language is a descriptor, and Rust is the first one. The server loads its
built-in descriptors from YAML embedded in the binary, merges what
`config.yaml` says over them, and builds a session's environment from the
result. A Rust build inside a session behaves exactly as it did before — the
same variables, the same Compile Server, the same directory — and an installer
who writes a descriptor for a language Verkstead has never heard of sees its
variables in the next session and its checkbox on the settings page.

No new language ships in this stage. What ships is the system, and the proof
that it carries the one language already there.

## Decisions in force

All from [ADR-0021](../../adr/0021-language-descriptors.md); restated here where
the stage turns on them.

- **Data only.** A descriptor says a label, detection manifests, variables and
  (from stage 05) how its store is evicted. Behaviour is a capability built
  into the server and named by the descriptor. Rust's descriptor names the
  sccache capability, and the **Compile Server** stays Rust code — most of
  `build_cache.rs` is that and survives as it is.
- **Embedded YAML, one grammar.** The built-ins are a file in the binary in the
  grammar an installer writes, so they are the documentation's examples.
- **One map keyed by language**, with `enabled` and `size` as keys of the same
  entry. The settings page writes those two and nothing else, so a save from
  the page must leave an installer's descriptor keys untouched.
- **Merge key by key; `null` removes a variable.** Chosen over replacing a
  built-in whole, so an installer changing one variable still gets every later
  fix to the rest.
- **The Sandbox's own variables are refused by name at load.**
- **A descriptor that does not load falls back to the built-in of that name**,
  and off only where there is no built-in; the page says why either way, and
  everything else carries on. Not a refusal to start, and not the loss of a
  cache that was already working.
- **`rust_build_cache` is still read** as Rust's `enabled` and `size`. Which of
  the two wins where both are written was not asked; the new map winning is
  the obvious reading, and the stage's own grilling confirms it.
- **The file only** — no descriptor editor. The page draws a checkbox per
  language the server lists.
- **Variables for every session**, as Rust's are today. **Detection is for the
  setup card's warning alone**: the Compile Server starts wherever a language
  naming the sccache capability is on and there is an sccache, whatever the Repo
  holds. A Repo whose manifest is not at the root is handed `RUSTC_WRAPPER`
  regardless, and with no server up its client starts one inside its own
  Sandbox — which is what the server exists to stop.
- **Settings are read at every session spawn**, as now, so a switch flipped on
  a phone applies to the next session.

The grammar in the ADR is the shape agreed, not its spelling. Key names, the
placeholder for the cache directory and how a capability is named are this
stage's to settle — and, with them, **whether one placeholder is enough**. pnpm,
bun and uv hardlink packages out of their store into the project rather than
copying them, and fall back to a full copy where the store and the project are
on different filesystems. The Build Cache and the Worktrees are free to be: the
packaged unit gives systemd `CacheDirectory=verkstead` and
`StateDirectory=verkstead`, two directories a sysadmin mounts separately as a
matter of course, and `--build-cache-dir` can name a second disk outright. If
that wants a store beside the Worktrees rather than under the cache, it wants a
second placeholder, and this is the stage that can still add one. Stage 02 is
where it is measured.

## Proposed tasks (provisional)

1. **The grammar and the loader.** Parse a descriptor, embed the built-ins,
   merge `config.yaml` over them. AC: a `null` takes a variable out; an entry
   naming a refused variable falls back to the built-in of that name, with a
   reason naming the variable, and turns the language off only where there is no
   built-in; an entry that does not parse does the same and leaves the others
   loaded; a refused override of Rust leaves a session with exactly the
   variables it had before that file was written.
2. **Rust as a descriptor.** The session environment is built from the loaded
   descriptors, and Rust's says what `sandbox.rs` sets today. AC: the
   environment of a session is byte for byte what it was; the Compile Server
   starts for any session where Rust is on and there is an sccache, a Repo with
   no `Cargo.toml` at its root included, and on no machine without one; the
   existing sandbox suite passes unchanged, and what changes in the sessions
   suite is its assertions about when the server starts and nothing else.
3. **The config key.** The new map, with `rust_build_cache` read as Rust's.
   AC: a `config.yaml` written by the released version reads as it did; a save
   from the page preserves descriptor keys the page never drew.
4. **The settings API and page.** The view lists languages rather than holding
   one cache; the pane draws a checkbox each, the size still hanging off
   Rust's, and, for a language whose entry did not load, its reason and whether
   it is running on its built-in or off altogether. AC: every pane that carries
   the cache along on its own save still does; generated TypeScript is up to
   date; a custom language appears with its label.
5. **The docs.** CONTEXT.md's **Build Cache** entry stops saying Rust alone and
   gains the descriptor as a term; the adoption doc gains the grammar, with the
   built-ins as its examples.

## Re-verify at start

- Assumes a session's Rust variables are still set in one place in
  `sandbox.rs`, and that the Compile Server's own surface sets its own.
- Count what asserts today that the Compile Server does not start for a Repo
  that is not Rust — `sessions.rs` starts it on that question — since those are
  the assertions this stage inverts.
- Assumes `rust_build_cache` is still carried along by several settings panes'
  saves (`held.ts` and the panes that spell it out) — count them before
  changing the shape.
- Assumes the settings view and edit types still live in the render crate and
  generate `web/src/api/types.ts`.
- Check what the full list of variables the Sandbox sets is, per platform —
  Windows sets names the two Unixes do not — since that list is what is
  refused.
- Check how Windows grants the cache directory: whole, in which case a
  descriptor's subdirectory needs no entry of its own.
- Check whether the Build Cache and the Worktrees are on one filesystem on this
  machine, and whether anything already assumes they are — that is what decides
  whether the grammar needs somewhere other than the cache to point at.
