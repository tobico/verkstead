# Development

Working *on* Verkstead, rather than with it: running the loop out of a checkout,
and the commands the two halves are built and tested with. There is no other
way in yet — nothing has been released under this name, so a checkout is the
only Verkstead there is.

The vocabulary in bold is the project's, and is defined in
[CONTEXT.md](../CONTEXT.md).

## Quickstart

The whole loop, from a fresh checkout. It takes two terminals: `verkstead ask`
blocks until it is answered, which is the entire point of it.

### 1. Enter the dev shell

```console
$ nix develop
```

Everything below assumes this shell — it carries the Rust toolchain, `sqlite`,
`git`, and the `node` and `pnpm` the viewer is built with.

### 2. Build the viewer and start the server (terminal 1)

```console
$ (cd web && pnpm install && pnpm build)
$ cargo run -p verkstead-cli -- serve --watched-path ~/src
  INFO verkstead_server: verkstead is listening listen=127.0.0.1:8422 database=verkstead.db watched=["/home/you/src"]
```

`--watched-path` is the one flag with no default. It names a directory
Verkstead may operate inside, and it is a security boundary rather than a
convenience: nothing outside the paths given is touched, and a repo is
registered only from within one. Repeat the flag for more than one, or set
`VERKSTEAD_WATCHED_PATHS` with them separated by `:`. The server refuses to
start with none.

One binary serves both halves: the agent API under `/api/v1/`, and the web UI
on <http://127.0.0.1:8422/>. It creates `verkstead.db` in the working directory
on first run. Leave it running; check it in a third terminal if you like:

```console
$ curl http://127.0.0.1:8422/api/v1/health
ok
```

The viewer is built into the binary, so `pnpm build` is what puts a UI at that
address. Skip it if only the API matters — the server starts either way, and
says on every page that the viewer was not built. While working on the viewer
itself, `pnpm dev` is the better half of this: see [The dev loop](#the-dev-loop).

### 3. Start a conversation (in the browser)

Every Question Set is asked *from* a Conversation and lands on its Timeline, so
there has to be one before an agent can ask anything. Open
<http://127.0.0.1:8422/>, add a repo from inside the watched path, and press
**New conversation**. The URL then names it — `/conversations/1` — and that
number is the one below.

The same two steps over the API, which is what a script does:

```console
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"path":"'"$HOME"'/src/verkstead"}' \
    http://127.0.0.1:8422/api/ui/repos
"Added"
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"repo_id":1}' http://127.0.0.1:8422/api/ui/conversations
{"Started":{"id":1}}
```

### 4. Ask (terminal 2)

```console
$ export VERKSTEAD_SERVER=http://127.0.0.1:8422/conversations/1
$ cargo run -p verkstead-cli -- ask examples/questions.yaml
```

`VERKSTEAD_SERVER` is the whole of what says which Conversation is asking. A
real session never sets it by hand: the orchestrator injects it into the
sandbox, scoped to the Conversation the session is running for, so the bundled
CLI attributes every Set explicitly without knowing it is doing so. Nothing is
inferred from the project or the branch — two Conversations against one repo
would be indistinguishable by either.

A wait that goes to plan is silent, and the little the CLI does have to say —
reconnecting, or refusing a Set — is on **stderr**, written as a YAML comment.
Stdout carries the Response and nothing else, so an agent can parse it as it
stands, even out of the one file its harness merged both streams into.

The command does not return. It has submitted
[`examples/questions.yaml`](../examples/questions.yaml) — along with the project
and branch it derived from this working directory, and the **Diff** of its
uncommitted changes if there are any — and is now holding a long-poll on
Question Set 1. There is no timeout: only an answer or a kill ends the wait
([ADR-0001](adr/0001-blocking-cli-for-agent-integration.md)).

A Set can also arrive on stdin, which is how an agent usually sends one:

```console
$ cat examples/questions.yaml | cargo run -p verkstead-cli -- ask
```

### 5. Answer (in the browser)

This is the human's part. Open <http://127.0.0.1:8422/> and pick the
Conversation the Set was asked from: it is on that Conversation's Timeline,
summarised as the table of number, question and answer, and badged `waiting on
you` — the CLI is still holding its long-poll. Press the row and the details
pane is the whole ask.

(A Set is also a page of its own at `/sets/<id>`, which is what a push
notification opens on a phone. It draws the same sheet and answers through the
same endpoint; what it has that the pane does not is a way back to the
Conversation it belongs to.)

The sheet is the whole ask: the Preface, the Diff of the working tree the agent
asked from, and each Question with its Options. Pick one, or write your own
words, or both — an Option with a ★ is the agent's Recommendation, and
**Accept all ★ Recommendations** fills in every question you have not answered
yet. Leave a question alone to send it back open; **Submit** asks you to
confirm that before it goes.

Under the last Question is the comment box, for anything about the Set as a
whole rather than about one Question — and directly above it, where the agent
wrote one, the **Postscript**: what it wanted to raise without making a
Question of it, which the example Set closes with. Nothing there has to be
answered, and an empty box says there was nothing to add.

The same Response can go in over the API instead, which is what an integration
test or a script does — see
[`examples/response.yaml`](../examples/response.yaml), which answers every
Question in the example Set, leaves one explicitly open, and adds a set-level
comment.

### 6. Delivery

Back in terminal 2, the still-waiting CLI has printed the Response and exited —
this is what it prints for the answers in `examples/response.yaml`:

```console
answers:
- label: Q1
  selected: 1
  free_text: Start in-process. Revisit it the day we run more than two instances.
- label: Q2
  selected: 2
- label: Q2a
  unanswered: true
- label: Q3
  free_text: acct_8f21c3, the nightly export job in `ops/export`.
comment: |
  Ship it behind a flag and turn it on for the one noisy client first.

  That export job hammers the endpoint on purpose, so give it an allowlist
  entry rather than a bigger bucket for everyone.

  On Q2a I genuinely don't know — pick whatever our SDK's retry logic
  already understands, and say in the PR which one that was.

$ echo $?
0
```

That is the loop. Run step 4 again and Question Set 2 appears on the same
Conversation's Timeline, to be answered the same way. To answer it from your
phone instead, put `tailscale serve --bg 8422` in front of the server and open
the `ts.net` url — push notifications need the HTTPS that gives you.

## The dev loop

```console
$ cargo test              # unit, schema and end-to-end tests
$ cargo clippy --all-targets
$ cargo fmt
$ nix fmt                 # the Nix files
$ nix flake check         # the viewer's suite, and the NixOS module in a VM

$ tools/generate-icons.sh # the PWA icons, after editing their SVG
```

And in `web/`, which is the Solid viewer
([ADR 0003](adr/0003-solid-spa-viewer.md)):

```console
$ pnpm install
$ pnpm dev                # the viewer on :5173, /api proxied to the server
$ pnpm test               # the vitest suite
$ pnpm typecheck          # tsc, which the tests do not run
$ pnpm lint               # the wall around the query hook, and nothing else
$ pnpm build              # static assets, into web/dist
```

`pnpm dev` serves the viewer alone and proxies everything under `/api` to a
server on its usual `127.0.0.1:8422`, so the two run side by side in two
terminals and the browser sees one origin. The proxy is a development thing
only: the built assets are served by the server itself, out of the same binary.

Which is `pnpm build`'s output, embedded by rust-embed. A release build compiles
it in; a debug build reads it off disk per request, so a `cargo run -p
verkstead-cli -- serve` serves whatever `pnpm build` last wrote without a
recompile — and a checkout that has never built the viewer still builds the
server, which then says so on every page instead of serving one.

`cargo test` covers the round trip in-process. `nix flake check` runs the
viewer's vitest suite from the pinned pnpm and node, and boots a VM with the
NixOS module enabled to put a Question Set through it again, for the sake of
everything the module wraps around that round trip: a unit that starts itself
at boot, the state directory systemd hands over, a database that survives the
service being stopped and started under a waiting agent, a store-path binary
serving the viewer that was built into it, and the CLI on `PATH` with nothing
set in the environment. The VM needs a Linux host to boot the guest on, so on
macOS that half of the check is absent rather than failing.

The viewer's dependencies are fetched by a fixed-output derivation named by a
hash in [`nix/web.nix`](../nix/web.nix) — move `web/package.json` or
`web/pnpm-lock.yaml` and that hash has to move too, and nix will print the one
it wanted. That file both builds the viewer and, with `runTests` on, is the
`nix flake check` above, so the two cannot disagree about a lockfile.

`assets/` is vite's `publicDir`, copied verbatim into the site root: the web
manifest, the icons and the service worker. They cannot live under `/assets/`
with the hashed bundles — a service worker only controls the paths beneath the
one it was served from, so one under the bundles' directory could never show a
notification for `/sets/12`, and the manifest and the icons keep the names the
phone knows them by. Which is also why the server keeps everything under
`/assets/` for a year and revalidates everything outside it.

The worker itself does no caching; every list and every Set is read from live
SQLite, and a cached copy of one that has since been answered is worse to the
human than a failure to load.

The icons are all one SVG, `assets/icons/verkstead.svg`, rasterized by the
script above (using `resvg` from the dev shell) to the PNG sizes the manifest
and iOS ask for. The PNGs are committed so a build needs nothing but cargo — edit the
SVG and re-run the script rather than touching them.

The tests run the real server in-process, so the round trip they check is the
one an agent gets — including the quickstart above, whose example files
[`crates/cli/tests/ask.rs`](../crates/cli/tests/ask.rs) drives end to end, taking
the human's part over the API the viewer's **Submit** posts through.

`verkstead-render` is everything the server does to what an agent wrote before
it leaves: markdown to sanitized HTML, the Diff parsed and highlighted, and the
view types the viewer draws a Set from. It knows nothing of the store, the router or
the viewer, so it is the seam the browser never reaches across — everything past
it is HTML the viewer only has to put in the page.

Two things under `web/` are written by `cargo test` rather than by hand, and
both are committed so that the diff is the review. `web/src/api/types.ts` is
those view types as TypeScript, generated by ts-rs — the viewer imports them and
declares no shape of its own, so the two languages cannot come to disagree about
a field. `web/tests/fixtures/` holds a payload of each kind, rendered by the real
`/api/ui/` endpoints, which is what the vitest suite is fed: a component test
against a hand-written mock proves only that the mock and the component agree.

Every query in the viewer is made with `useReading` from `web/src/freshness.ts`
rather than with the hook underneath it. A Nudge invalidates what the page is
showing ([ADR 0009](adr/0009-scoped-nudges.md)), so each query has to say what a
re-read does to what is drawn: `freshness` names the key the re-read is merged
by, or says `"static"` where the payload cannot change. There is no default —
a query that says neither does not typecheck — and `web/eslint.config.js` is
the other half of the same rule, refusing the raw hook everywhere but the
wrapper's own module. That is the whole of what `pnpm lint` checks.

Which queries a Nudge invalidates is one table in `web/src/nudge.ts`, keyed by
the kind the server sent. The kinds themselves are a Rust enum in
`crates/schema/src/nudge.rs`, so adding one is two edits — the variant and the
announce site on the server, and a row in that table on the viewer. A kind with
no row falls back to reading everything, which is what an older page does
against a newer server; `web/tests/nudging.test.tsx` sweeps the generated
fixture and fails on a kind the table has forgotten. Nothing polls: what stands
behind a Nudge that never landed is the catch-up on reconnect and on the page
becoming visible again.
