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
$ cargo run -p verkstead-cli -- serve --data-dir . --watched-path ~/src
  INFO verkstead_server: verkstead is listening listen=127.0.0.1:8422 data_dir=. watched=["/home/you/src"]
```

`--watched-path` names a directory Verkstead may operate inside, and it is a
security boundary rather than a convenience: nothing outside the paths given is
touched, and a repo is registered only from within one. Repeat the flag for more
than one, or set `VERKSTEAD_WATCHED_PATHS` with them separated by `:`. A path
that is not there refuses startup, because a flag is the installation's own word
and nobody is watching when it is wrong.

It is not required, though: `cargo run -p verkstead-cli -- serve --data-dir .` on
its own comes up watching nothing, which admits nothing, and the settings page is
where it is pointed at its first directory.
Watched paths and sandbox binds are said in both places and what the server uses
is the union — see the `"paths"` payload further down this section. The flag is
the shape a service unit wants, where startup is the moment to hear about a
typo; the settings are the shape a bare binary wants, where a save has to land
whatever it was told and an entry that will not resolve is reported rather than
fatal.

Everything Verkstead makes goes in one place, the **Data Directory**: the
database at `verkstead.db`, the worktrees, the installed skills, the handoff
directories and the settings files. `--data-dir` says where, or
`VERKSTEAD_DATA_DIR`. Said nothing, it is the platform's own place for it —
`~/.local/share/verkstead` on Linux, `~/Library/Application Support/Verkstead`
on macOS — which is what an installed Verkstead wants and not what a dev run
out of a checkout does: `--data-dir .` is why every command here says it, and
it keeps the database, the worktrees and the settings beside the checkout where
they can be deleted with it.

The desktop app is the other binary, and the same server: `cargo run -p
verkstead-desktop -- --data-dir .` serves what the command above serves and
opens the viewer in your browser as it comes up. `--no-open` leaves the browser
alone, and every other flag is the server's own, because the app *is* the
server ([ADR 0012](adr/0012-desktop-tray-binary.md)) — started with nothing
said it is the platform's Data Directory again, which is what a machine that
installed it wants and not what a checkout does. It is the one crate here that
links system libraries, GTK among them, so it builds in the dev shell and
nowhere else. An address something is already listening on — the command above,
say — is a dialog and a nonzero exit rather than a second Verkstead beside the
first.

What it puts on the screen is an icon in the system tray, and the menu on it is
**Open** — the viewer again, in your browser — **View Logs**, which opens the
file the server's log goes to instead of a stdout nobody launched from an icon
will read, **Launch on Startup**, and **Exit**, which stops Verkstead where it
stands the way stopping the systemd unit does. Run it where there is no screen
to put an icon on, over SSH or under a test, and it is the server and the open
and no more: a warning in the log, and everything else exactly as it was.

**Launch on Startup** is a checkbox over the platform's own registration — your
desktop's autostart entry at `~/.config/autostart/net.tobico.Verkstead.desktop`
here, a launch agent at `~/Library/LaunchAgents/net.tobico.Verkstead.plist` on
macOS — and that file is the whole of the state: checking the box writes it,
unchecking removes it, turning it off in your desktop's own settings unchecks
it, and no setting of Verkstead's own keeps a second copy of the answer. Every
launch rewrites it while it is there, with the path of the executable that is
running, so a binary you moved heals its own entry the next time you start it
by hand. What it writes starts the app with `--no-open`: a login is not a
moment to be handed a browser window. The one thing the box cannot see is
macOS's Login Items list, which `launchd` keeps in a database of its own rather
than in the file: switch Verkstead off there and the box goes on showing what
the plist says.

One directory is made outside it: the **Build Cache**, at
`$XDG_CACHE_HOME/verkstead` — `~/.cache/verkstead` on most machines — unless
`--build-cache-dir` says otherwise. Every sandboxed session gets it writable,
with `CARGO_HOME` inside it, so a crate is downloaded once for the machine
rather than once per Conversation; with `sccache` on the `PATH` the server was
started from, it is bound into each sandbox as `RUSTC_WRAPPER` and the
compiling is cached too. The dev shell carries one, so a checkout run gets the
whole thing. It is on with nothing configured, and the settings page is where
it is switched off or given a size.

The sccache **server** is Verkstead's own, not the sessions'. It comes up as a
`bwrap` child of the running server the first time a session starts on a repo
with a root `Cargo.toml`, in a sandbox holding `<data-dir>/worktrees` and the
build cache and nothing else — so `ps` shows a `bwrap … verkstead-compiling`
beside each session's, and it goes when the server does. Every session's
`sccache` is only the client half reaching it. Sessions starting their own is
what this replaces: they all bind one port, and the loser's compiles then run
in the winner's sandbox where its worktree is not mounted.

A session's GitHub auth and the author of its commits are two of those settings
rather than anything found in a home directory. Put a token in `secrets.yaml`
beside the database, and who you are in `config.yaml`:

```yaml
# secrets.yaml
github_token: ghp_...
```

```yaml
# config.yaml
git_author:
  name: Tobias Cohen
  email: tobi@tobico.net
rust_build_cache:
  enabled: true
  size: 30G
share_viewer_url: https://ada.github.io/verkstead-shares/
conflict_resolution: merge
sandbox_binds:
  - /var/cache/verkstead-node
  - verkstead=/var/cache/verkstead-cargo
watched_paths:
  - /home/tobi/src
```

The two lists at the foot are the other place the Watched Paths and the Sandbox
Configuration binds are said. A bind is a plain path where every session gets
it, and `name=path` where only the sessions working in the Repo registered under
that name do; and what the server goes by, for both lists, is the union of this
file and the installation's own flags.

Every session started after that gets the token as `GH_TOKEN`, which `gh`
honours without being told to — as does the server's own `gh`, the one that
reads a pull request's checks, commits and comments onto a Timeline — and gets
git configured through the
environment — the author, `gh auth git-credential` as the credential helper for
GitHub, and SSH GitHub remotes rewritten to HTTPS so a `git push` inside
authenticates with the token. There is no file to write and nothing to log in
to inside a sandbox. With no token — no file, an empty one, one that will not
parse — sessions start anyway and `gh` inside says it is not logged in, and the
server's own `gh` falls back to whatever login the host has; with no author, git
inside asks to be told who you are.

Either file can be written through the viewer's API instead of by hand, which is
what the settings page saves through:

```console
$ curl http://127.0.0.1:8422/api/ui/settings
{"git_author":{"name":"","email":""},"github_token":null,
 "rust_build_cache":{"enabled":true,"size":"30G","size_configured":false,
   "compiles_cached":true},"share_viewer_url":"","conflict_resolution":"Merge",
 "paths":{"watched":[],"binds":[]}}
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
         "github_token":{"Set":{"token":"ghp_..."}},
         "rust_build_cache":{"enabled":true,"size":""},
         "share_viewer_url":"https://ada.github.io/verkstead-shares/",
         "conflict_resolution":"Merge",
         "watched_paths":["/home/tobi/src"],
         "sandbox_binds":["/var/cache/verkstead-node"]}' \
    http://127.0.0.1:8422/api/ui/settings
{"settings":{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
  "github_token":{"last_four":"cdef","at":"2026-08-23T08:23:15.041950412Z"},
  "rust_build_cache":{"enabled":true,"size":"30G","size_configured":false,
    "compiles_cached":true},
  "share_viewer_url":"https://ada.github.io/verkstead-shares/",
  "conflict_resolution":"Merge",
  "paths":{"watched":[{"path":"/home/tobi/src","source":"Settings",
    "resolution":"Resolves"}],
   "binds":[{"path":"/var/cache/verkstead-node","repo":null,
    "source":"Settings","resolution":{"Unresolved":{"why":
      "the server cannot see it: there is nothing at that path"}}}]}},
 "verified":{"Account":{"login":"tobico","missing":["gist"]}}}
```

The token goes one way. What comes back about it is its last four characters and
when `secrets.yaml` was written, never the token itself. Saving one asks GitHub
who it authenticates as and answers with the account or with what went wrong —
and writes it down either way, because a token is pasted once out of a page that
will not show it again, and a network that was briefly down is no reason to send
somebody back for another. `"missing"` beside the account is the scopes
Verkstead needs that GitHub says the token has not been given — `gist`, which
publishing a share writes with, and empty on a token that carries it or on a
fine-grained one GitHub named no scopes for at all. `"github_token"` is `"Keep"`
to leave the configured one alone, which is what a save of the author fields
sends, and `"Clear"` to take it away. `"rust_build_cache"` is a pair of values
rather than an action: an empty `"size"` is no size configured, which puts the
default back, and `"compiles_cached"` is read-only — it says whether the server
found an `sccache`, which is its own environment rather than anybody's setting.

`"watched_paths"` and `"sandbox_binds"` are the two lists `config.yaml` holds,
sent as values in the grammar the flags use — so a Verkstead started with no
flags at all is pointed at its first directory through this endpoint. What is
sent is what the file holds afterwards, and the `"paths"` that comes back is
both sources at once: every entry says whether the installation's flags or the
settings said it, and whether the server can see what it names right now. Only
the settings' own can be sent, and nothing about them is checked as it is
written — the save lands whatever it was told, and an entry the server cannot
see is a `"resolution"` saying so rather than a refusal.

`"share_viewer_url"` is where a **share viewer** of your own is hosted, and it is
the plainest value here: written as it was typed, read back as itself, and empty
where you host none. The viewer is a small static page that draws a published
share in a browser — a gist link on its own shows source — and a published share
is read at `<share-viewer-url>#<gist-id>`.

**Empty is not "no viewer".** Verkstead keeps a copy of the page on its own
GitHub Pages, at
<https://tobico.github.io/verkstead/share-viewer.html>, and every link it hands
out — the toast, the Share row and the comment on a pull request — is composed
through that unless this setting says otherwise. So a Verkstead nobody has told
anything still hands out links that draw. The page is published by
`.github/workflows/pages.yml` whenever `crates/server/share-viewer.html` lands
on `main`; its address is `HOSTED` in `crates/server/src/sharing.rs`, and
`web/tests/viewing.test.ts` is what holds the two spellings together.

Fill the field in to serve the page yourself instead — so that nothing about
your shares goes past a site of Verkstead's. Verkstead ships the file rather
than serving it:

```console
$ curl -O -J http://127.0.0.1:8422/api/ui/share-viewer.html
```

Put that on a public site of your own, a GitHub Pages repository being what it
was written for, and save its address here. Nothing about it is secret either
way: the page is public, and the id after the `#` is never sent to the host that
serves it.

`"conflict_resolution"` is what a session sent at a pull request that will not
merge is told to do about it: `"Merge"`, which merges the base branch into the
work branch and pushes, or `"Rebase"`, which rebases the branch onto its base and
force-pushes it with `--force-with-lease`. An absent key, an absent file and one
nothing can parse all mean a merge — a rebase rewrites what reviewers have
already read and breaks anything stacked on the branch, and nobody should meet
that for never having found the settings page. In `config.yaml` the word is
lowercase, as `merge` or `rebase`.

One repo can say otherwise, which is a fact about the repo rather than about the
machine and so lives in the database beside it — set from that repo's own pane on
the settings page, or over the API:

```console
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"resolution":"Rebase"}' \
    http://127.0.0.1:8422/api/ui/repos/1/resolution
```

`null` there takes the override back, so that repo does whatever every other one
does. It is nothing at all rather than a copy of today's global, so a repo left
alone follows the setting above when it is changed.

The link is composed as a page is drawn rather than written down at the publish.
What the record holds is the gist's own URL, so a share published before a viewer
was configured links through one now, and a viewer moved later retargets every
link there is without republishing anything.

That link is what **Share to pull request** leaves behind. One press on a
conversation whose work is on a pull request publishes a share and comments on
every pull request the conversation holds — its own repository's and each
companion's — carrying the link and an itemized summary of what is in the file.
Comments only: nothing edits a description, and sharing again leaves another
comment rather than rewriting the one before it. A pull request the comment
could not land on is named beside the ones that worked.

One binary serves both halves: the agent API under `/api/v1/`, and the web UI
on <http://127.0.0.1:8422/>. It creates `verkstead.db` in the Data Directory on
first run, which `--data-dir .` above makes the checkout. Leave it running;
check it in a third terminal if you like:

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
[`examples/questions.yaml`](../examples/questions.yaml), along with the project
and branch it derived from this working directory, and is now holding a
long-poll on Question Set 1. There is no timeout: only an answer or a kill ends
the wait ([ADR-0001](adr/0001-blocking-cli-for-agent-integration.md)).

The **Diff** on the Set is not the CLI's doing: the server reads it off the
Worktrees as the Set arrives — the Conversation's own, and each read-write
companion repo beside it, one labeled block each. A Conversation started this
way has never been grilled and so has no Worktree at all, which is why the Set
below carries no Diff — one asked from inside a real session carries whatever
that session has left uncommitted, in whichever repository it left it.

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

The sheet is the whole ask: the Preface, the Diff of the Worktrees the agent
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

$ tools/generate-icons.sh     # the favicon and PWA icons, after replacing the artwork
$ tools/generate-packaging.sh # the desktop entry, the launcher icons and the icns
$ tools/build-appimage.sh     # Verkstead-x86_64.AppImage, once the viewer is built
$ tools/build-macos-dmg.sh    # Verkstead-universal.dmg, on a Mac
```

The last two are the desktop artifacts a release ships, one per platform. Both
take everything from the working tree and leave one file under `target/`, and
both want `web/dist` already built, because the viewer is compiled into the
binary they wrap.

The AppImage is the desktop binary, the packaging assets and every library the
tray is drawn over, in one file, and it is the same command CI runs. It builds
what `cargo build --release -p verkstead-desktop` builds, so it wants the dev
shell for the same reason that does.

The dmg is `Verkstead.app` — the same binary built for both Apple targets and
`lipo`-ed into one, the icns from `packaging/`, and an `Info.plist` that says
`net.tobico.Verkstead` and `LSUIElement`, which is what makes it a menu-bar app
with no Dock tile. It runs on a Mac only: `lipo`, `codesign` and `hdiutil` are
the operating system's own tools, and there is no cross build of it from here.
The bundle is ad-hoc signed rather than signed with a Developer ID, because
Apple silicon will not execute a binary with no signature at all — that is not
the signing that gets an app past Gatekeeper, and there is none of that.

### The sessions suite, and the machine under it

`crates/server/tests/sessions.rs` is the one suite that is really a hundred and
forty small servers, each running a real session in a real sandbox and judged on
wall clock. That makes it the one suite whose result depends on what else the
machine is doing, so it has two knobs of its own.

`VERKSTEAD_TEST_PACE` is a multiplier over everything time-shaped in it — the
budgets a session is ended by, how long a wait gives up after, and every window a
test holds open to prove nothing happened. Unset is `1.0`. CI sets `2`, because a
two-core runner building the workspace alongside the run cannot meet a
developer's machine's budgets, and a session descheduled past one is ended by the
wrong rule and fails a test that is not about the code. Raise it locally if the
suite fails on a busy machine and passes on a quiet one; the file's own `PACE`
explains the rest.

Nothing needs setting for the concurrency: the suite caps how many fixtures stand
at once by itself, at twice the cores it can see.

```console
$ scripts/soak-sessions.sh        # ten runs, pinned to two cores, under a build loop
```

The soak is how a change to that suite is shown to hold. Running it again was
never how its flakes reproduced — each loaded run failed a different test and
every one of them passed alone — so the bar is ten in a row under load rather
than one green run. It takes about fifty minutes, which is why nothing runs it
for you.

And in `web/`, which is the Solid viewer
([ADR 0003](adr/0003-solid-spa-viewer.md)):

```console
$ pnpm install
$ pnpm dev                # the viewer on :5173, /api proxied to the server
$ pnpm test               # the vitest suite
$ pnpm typecheck          # tsc, which the tests do not run
$ pnpm lint               # the wall around the query hook, and nothing else
$ pnpm build              # static assets into web/dist, and the share into web/dist-share
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

`pnpm build` writes three times, out of the same sources. The site goes to
`web/dist` as it always did; the **share** goes to `web/dist-share` as one HTML
file with its script and its stylesheets inlined
([`vite.share.config.ts`](../web/vite.share.config.ts)), which is the template
the server writes a Conversation into and hands over as a download — see
[`crates/server/src/sharing.rs`](../crates/server/src/sharing.rs). Both are
embedded the same way and both are `allow_missing`, so a checkout that has built
neither still builds the server; ask it for a share and it says the build is not
in the binary. That config refuses to write a document that still points at a
file beside it, which is what makes *no external requests* a property of the
build rather than something to remember.

The third is mermaid, on its own, into `web/dist-share/mermaid.js`
([`vite.mermaid.config.ts`](../web/vite.mermaid.config.ts)). It is the one thing
a Set's page draws for itself and it is three megabytes, so it is the one thing
a share does not carry as a matter of course: the share build aliases the
package to a stub that reaches for whatever the *document* is holding, and the
server writes the library into a second slot only where something in the record
has a Diagram on it — a Set's Preface or a Commit Summary alike. A Conversation
nobody drew a picture in stays the size of its own record.

The same file is what a **publish** puts in a secret gist — see
[`crates/server/src/publishing.rs`](../crates/server/src/publishing.rs), where
the API makes the gist and git fills it, because the Gists API will not take a
file this size. So a publish wants the share build too, and a token with the
`gist` scope on it.

`cargo test` covers the round trip in-process. `nix flake check` runs the
viewer's vitest suite from the pinned pnpm and node, and boots a VM with the
NixOS module enabled to put a Question Set through it again, for the sake of
everything the module wraps around that round trip: a unit that starts itself
at boot, the Data Directory systemd hands over, a database that survives the
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

The icons are all downscaled by the script above (using ImageMagick from the dev
shell) to the sizes the favicon, the manifest and iOS ask for. The smaller PNGs
are committed so a build needs nothing but cargo — replace the artwork and
re-run the script rather than touching them.

There are three pieces of artwork, because one square does not serve every size
and every platform:

| Artwork | Cut into | Why it is its own file |
| --- | --- | --- |
| `icons/verkstead.png` | `icon-192`, `icon-512` | The full mark on a transparent field: the manifest's icons, and the sidebar's at `3rem` |
| `icons/verkstead-hammer.png` | `icon-32`, and every icon under `packaging/` | The hammer alone — at 32px the full mark is a grey smudge with confetti on it, and no filter rescues artwork with too much in it for the size |
| `icons/verkstead-bg.jpg` | `apple-touch-icon.png` | The only one drawn with a field of its own, because iOS composites a transparent icon onto black |

The iOS icon used to be the full mark flattened onto the manifest's
`theme_color`; now it carries its own field, so the browser chrome's colour and
the icon's are no longer the same value and nothing keeps them in step.

The manifest asks for `any` rather than `any maskable`: the artwork runs to the
edges of its square, and a launcher masking it to a circle would cut the hammer
and the anvil's horn off. Art with a margin inside it could claim `maskable`
back.

`packaging/` is the second tree of generated assets, and it sits outside
`assets/` deliberately. Everything under that directory is `publicDir` — served
at the web root and, because the viewer is embedded, carried inside every binary
including the headless CLI — and a desktop entry and a launcher's icons are
neither the viewer's to serve nor the CLI's to hold. So the desktop packaging
gets a directory of its own: `net.tobico.Verkstead.desktop` and the hicolor icon
tree that `tools/build-appimage.sh` installs into the AppImage,
`net.tobico.Verkstead.icns` that `tools/build-macos-dmg.sh` puts in the app
bundle, and the Windows launcher artwork beside them when that stage lands. It
is written by [`tools/generate-packaging.sh`](../tools/generate-packaging.sh)
from the same hammer, and committed for the same reason the viewer's icons are.
That script rewrites the whole directory from nothing on every run — so a size
that stops being generated stops being committed, and nothing under it is ever
edited by hand.

The icns is written by the script itself rather than by `iconutil`, which is a
Mac's: the format is a header and a PNG per icon slot, so it is generated in the
dev shell alongside everything else here rather than on the one platform that
reads it.

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
