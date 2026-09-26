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
`git`, the `node` and `pnpm` the viewer is built with, and the Electron the
desktop app runs on.

### 2. Build the viewer and start the server (terminal 1)

```console
$ (cd web && pnpm install && pnpm build)
$ cargo run -p verkstead-cli -- serve --data-dir .
  INFO verkstead_server: verkstead is listening listen=127.0.0.1:8422 workbench=http://127.0.0.1:8422/?key=… data_dir=. home=/home/you sandbox_binds=0 build_cache=Some("/home/you/.cache/verkstead") skills=./skills
```

**`workbench=` is how you get in.** Every page of the workbench and the viewer's
own `/api/ui/` namespace answer 401 without the **Workbench Key**, and that link
is the address with the key on it: paste it once and the browser holds the
cookie from then on. The key is `workbench.key` in the Data Directory, made at
the first start and read back at every one after it.

**That is the whole of it — there is no boundary flag to say.** A repo is
registered from anywhere the server can read, an **Agent Profile** names an
account anywhere the server can read, and every path field browses the same
way, opening on the server's own `HOME` when nothing has been typed into it.
What keeps a session to its own Conversation is the **Sandbox**, composed from
the Repo and the Profile that Conversation names.

The one path list left is the **Sandbox Configuration** binds, and they are
said in two places — `--sandbox-bind DIR`, and the same grammar on the settings
page's **Sandbox binds** section. What a session gets is the union; see the
`"paths"` payload further down this section. The flag is the shape a service unit wants,
where startup is the moment to hear about a typo and a bind that is not there
refuses to start; the settings are the shape a bare binary wants, where a save
has to land whatever it was told and an entry that will not resolve is reported
rather than fatal.

Everything Verkstead makes goes in one place, the **Data Directory**: the
database at `verkstead.db`, the worktrees, the installed skills, the handoff
directories and the settings files. `--data-dir` says where, or
`VERKSTEAD_DATA_DIR`. Said nothing, it is the platform's own place for it —
`~/.local/share/verkstead` on Linux, `~/Library/Application Support/Verkstead`
on macOS — which is what an installed Verkstead wants and not what a dev run
out of a checkout does: `--data-dir .` is why every command here says it, and
it keeps the database, the worktrees and the settings beside the checkout where
they can be deleted with it.

The desktop app is the Electron project in [`desktop/`](../desktop), started
with `pnpm start` from this shell ([ADR 0020](adr/0020-electron-desktop.md),
which supersedes ADR 0012). It finds the headless `verkstead` the workspace
built, brings it up beside itself as `serve --desktop`, waits for the server to
answer on `/api/v1/health` and opens one window on the workbench already logged
in — the **Workbench Key** read out of the **Data Directory** rather than
pasted. So it wants the two commands above run first: the viewer built, because
the server serves it, and the CLI compiled, because that binary is what the app
starts. Nothing at `target/debug/verkstead` is a dialog naming the path it
looked at, and `VERKSTEAD_CLI` names a binary somewhere else — a release build,
or the one a Release shipped.

```console
$ export VERKSTEAD_DATA_DIR=$PWD   # the checkout, which is what --data-dir . says above
$ (cd desktop && pnpm install && pnpm start)
```

**The Data Directory is said through the environment here**, because the app
has no flag of its own to say it with: the sidecar inherits the shell the app
was started from, so `VERKSTEAD_DATA_DIR` is to the app what `--data-dir .` is
to every other command in this document, and a launch that says nothing gets
the platform's own place again. Every setting but one is the server's the same
way — **the exception is the address**, which the app hands the sidecar as
`--listen 127.0.0.1:8422` rather than inheriting: the app probes that address,
waits on it and loads it, so a `VERKSTEAD_LISTEN` you have exported for a
`verkstead serve` of your own is overridden and the log says it was. An address
something is already listening on — the `serve` above, say — is a dialog and a
nonzero exit rather than a second Verkstead beside the first, and a second
`pnpm start` is the first window brought forward rather than either of those.

`pnpm lint`, `pnpm typecheck` and `pnpm test` in that same directory are the
three things CI runs over it. The lint is one rule and it is the wall around
Electron: everything the app decides is a function of values the entry file
hands it, so that vitest can call it without an application under it. Which is
what the tests are over — the main process's pure parts, there being no driven
end-to-end suite (ADR 0020).

What it puts on the screen is one window and one icon in the tray. The window
loads the workbench off `127.0.0.1:8422` with nothing about the viewer changed
to draw inside it; a link that leads off the workbench opens in the browser you
already have rather than navigating the window away from it; and where the
window was last time is where it comes back, remembered in the app's own user
data because where a window sits is a fact about the desk rather than about
your Verkstead. The menu bar is hidden on Linux and Windows with copy, paste,
zoom, reload and the developer tools still on their keystrokes — Alt brings the
bar down — and a Mac keeps the strip at the top of the screen that says which
application is in front. The icon's menu is **Open**, **View Logs** and
**Quit**, a left click on it opens the window, and its **Quit** never asks.
Closing the window is a choice rather than a quit — keep running in the tray,
which is the default, ask before quitting, or quit — while the server ending is
still the app quitting. The sidecar's stdout and the app's own lines both go to
`verkstead.log` under the **Log Directory**, which the app names on the terminal
as it opens it and which both **View Logs** open.

**And the window has no title bar of its own.** What stands at its top-right
corner on Linux and Windows is the platform's own controls overlay, drawn on the
paper the page's heads are drawn on with its symbols in their ink and as tall as
the head beneath it: the page pushes those three values over the same preload
bridge at load and at every flip of the machine's colour scheme, so going dark
recolours the controls with no restart and the log says each push. A Mac has the
traffic lights inset at its top-left instead and ignores the three values. What moves
the window is any pane head — the sidebar's wordmark, a pane's own title row,
the settings, the composer — with every button standing in one still pressing,
and a double-click on one maximising. The frame pads whichever head is at an
edge of the window clear of the controls, so nothing in a head ends up
underneath them; and a page with no head at all — the setup wizard, the
no-such-page, the moment before the onboarding verdict lands — draws a bare bar
of the same height to be moved by. In a browser on this machine there is none of
it: the drag regions are inert, the insets are nought, and there is no bridge
for the colours to cross. CONTEXT.md's **Window Decorations** is the account of
the whole of it.

**On a Wayland session the overlay can be a close button alone, and that is
Chromium rather than your desktop.** The pinned Electron picks Wayland by itself
where the session says it is Wayland, and on a nested COSMIC 1.2.0 the overlay
came out 32 px wide with nothing in it but close; the same app on the same
session over Xwayland is 96 px and the usual three. COSMIC is refusing nothing —
its toplevel carries all four window capabilities, and neither its own toolkit
settings nor a GTK decoration layout moves it. So maximise is a double-click on
any head, and minimise is the app's own to ask for: nothing COSMIC ships binds a
key to one. The whole measurement is in
[stage 05's brief](roadmaps/electron-desktop/05-linux-appimage.md), and what a
packaged Linux app does about it is that stage's to settle.

**The Desktop section at the top of the settings page is where those choices
are made**, and it is drawn only inside the app: the page reaches the app over
a preload bridge on the app's own window, so the same settings page in a
browser on this machine — or on a phone over the tailnet — has no such section,
and nothing about any of it is on the wire or in `config.yaml`. It holds **When
the window is closed**, **Show tray icon**, **Launch on Startup** and a **View
Logs** button opening the file the tray item opens. What a dev run writes it to
is `desktop.json`, beside the `window.json` the window's place is kept in, under
Electron's own user data rather than anywhere `--data-dir` says: deleting it is
the app back at its defaults. Turning the tray off takes the icon away that
moment and greys the keep-running position, the choice falling to Quit while
there is no icon to come back from. **And Launch on Startup is greyed on a run
from a checkout**, with a note saying it needs an installed Verkstead: what an
autostart entry could name here is the dev shell's Electron in the nix store
plus this build directory, which breaks the next time either moves — so the
whole path is written and under vitest, and the writing is what is refused. Its
account is CONTEXT.md's **Desktop Settings** and **Startup Registration**.

**And the Rust tray app is still what a release carries**, on each platform
until the stage that takes that platform's release leg: the packaging section
below builds it, `crates/desktop` is its tray half behind the CLI's default-on
`desktop` feature — a build that says nothing gets both halves, which is what
makes every image that can serve one that can also `ask`, and
`--no-default-features` is the headless build the musl CLI and the nix package
take — and it is the one crate here that links a system toolkit, GTK on Linux,
which is why the workspace builds in the dev shell and nowhere else here.
Its own account is [ADR 0012](adr/0012-desktop-tray-binary.md) and CONTEXT.md's
**Startup Registration**, which is where **Launch on Startup** is written down;
nothing in this section starts it.

One directory is made outside it: the **Build Cache**, at
`$XDG_CACHE_HOME/verkstead` — `~/.cache/verkstead` on most machines — unless
`--build-cache-dir` says otherwise. Every session gets it writable, with
`CARGO_HOME` inside it, so a crate is downloaded once for the machine rather
than once per Conversation; with `sccache` on the `PATH` the server was started
from, every session is told to compile through it as `RUSTC_WRAPPER` and the
compiling is cached too, on all three platforms. The dev shell carries one, so a
checkout run gets the whole thing. It is on with nothing configured, and the
settings page is where it is switched off or given a size.

The sccache **server** is Verkstead's own, not the sessions'. It comes up as a
child of the running server the first time a session starts on a repo with a
root `Cargo.toml`, in a sandbox holding `<data-dir>/worktrees` and the build
cache and nothing else — so `ps` shows one more sandboxed child beside each
session's, and it goes when the server does. That sandbox is described and
rendered by the code a session's is, so it is `bwrap` on Linux, `sandbox-exec`
on a Mac and the session account on Windows without any half saying which.
Every session's `sccache` is only the client half reaching it. Sessions starting
their own is what this replaces: they all bind one port, and the loser's
compiles then run in the winner's sandbox where its worktree is not reachable.

**Windows had neither half for a while**, and the reason has gone. A session
there ran inside an AppContainer, which is refused every connection to the local
machine — the probe behind [ADR 0014](adr/0014-windows-sessions.md) timed out on
`127.0.0.1` and on the machine's own LAN address alike, and the sccache client it
ran panicked reading its own configuration before it got that far. A session
runs as a local account of Verkstead's own now: an ordinary local account
reaches the loopback like anything else, and the profile it is handed is a real
one with both halves made, which is what that client was looking for. So the
switch is on there like everywhere else, and the compile server on that platform
runs as the same account a session does, behind entries of its own — a compile
server started as the human would be every dependency's proc macro running with
the database and the settings files in reach.

**And a Windows session finds the machine's Rust through rustup.** There is no
`/nix` on that platform for a toolchain to be on, so what a session runs is the
human's own: `~/.cargo/bin` is on its `PATH` already, being under the home, and
the rustup home those shims resolve their toolchain out of is granted read-only
beside it with `RUSTUP_HOME` naming it — a sandbox has a profile of its own, so
rustup left to resolve against that one answers *could not choose a version of
rustc to run*. `RUSTUP_HOME` where the machine sets one, `~/.rustup` where it
does not, and nothing at all where there is no rustup on the `PATH`, which is
every nix machine.

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
cleanup:
  trim:
    enabled: true
    days: 3
  delete:
    enabled: false
    days: 30
conflict_resolution: merge
share_on_done: false
sandbox_binds:
  - /var/cache/verkstead-node
  - /var/cache/verkstead-cargo
```

`sandbox_binds` at the foot is the other place the Sandbox Configuration binds
are said. A bind is one absolute path, every session gets every one of them, and
what the server goes by is the union of this file and the installation's own
flags.

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
   "compiles":"Cached"},
 "cleanup":{"trim":{"enabled":true,"days":3,"days_configured":false},
   "delete":{"enabled":false,"days":30,"days_configured":false}},
 "conflict_resolution":"Merge",
 "share_on_done":false,"paths":{"binds":[]}}
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
         "github_token":{"Set":{"token":"ghp_..."}},
         "rust_build_cache":{"enabled":true,"size":""},
         "cleanup":{"trim":{"enabled":true,"days":""},
           "delete":{"enabled":false,"days":""}},
         "conflict_resolution":"Merge",
         "share_on_done":false,
         "sandbox_binds":["/var/cache/verkstead-node"]}' \
    http://127.0.0.1:8422/api/ui/settings
{"settings":{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
  "github_token":{"last_four":"cdef","at":"2026-08-23T08:23:15.041950412Z"},
  "rust_build_cache":{"enabled":true,"size":"30G","size_configured":false,
    "compiles":"Cached"},
  "cleanup":{"trim":{"enabled":true,"days":3,"days_configured":false},
    "delete":{"enabled":false,"days":30,"days_configured":false}},
  "conflict_resolution":"Merge",
  "share_on_done":false,
  "paths":{"binds":[{"path":"/var/cache/verkstead-node","repo":null,
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
fine-grained one GitHub named no scopes for at all. `"github_token"` is
`"Keep"` to leave the configured one alone, which is what a save of the author
fields sends, and `"Clear"` to take it away. `"rust_build_cache"` is a pair of
values rather than an action: an empty `"size"` is no size configured, which
puts the default back, and `"compiles"` is read-only — `"Cached"` where the
server found an `sccache` and `"NoSccache"` where it did not. Its own
environment rather than anybody's setting.

`"sandbox_binds"` is the list `config.yaml` holds, sent as values in the grammar
the flags use — so a Verkstead started with no flags at all gets its first bind
through this endpoint. What is sent is what the file holds afterwards, and the
`"paths"` that comes back is both sources at once: every entry says whether the
installation's flags or the settings said it, and whether the server can see
what it names right now. Only the settings' own can be sent, and nothing about
them is checked as it is written — the save lands whatever it was told, and an
entry the server cannot see is a `"resolution"` saying so rather than a refusal.

A published share is read through the **share viewer**, a small static page that
draws it in a browser — a gist link on its own shows source. It is not
configurable and there is nothing to configure: Verkstead keeps the one copy on
its own GitHub Pages, at
<https://tobico.github.io/verkstead/share-viewer.html>, and every link it hands
out — the toast, the Share pane and the comment on a pull request — is composed
through that, as `<share-viewer-url>#<gist-id>`. The page is published by
`.github/workflows/pages.yml` whenever `crates/server/share-viewer.html` lands
on `main`; its address is `HOSTED` in `crates/server/src/sharing.rs`, and
`web/tests/viewing.test.ts` is what holds the two spellings together.

Nothing about that is secret: the page is public, the id after the `#` rides in
the fragment and is never sent to the host that serves it, and the share itself
is fetched from GitHub by the reader's own browser.

`"share_on_done"` is whether a conversation's record is published and linked on
its pull request when the work settles to Done, which is otherwise something the
human presses for. It is the one setting here that is **off** with nothing
configured — an absent key, an absent file and one nothing can parse all mean
off — because what it turns on publishes a gist under the human's own account
and comments on a pull request other people are reading. It is set from the Git
pane, beside the token it is published with.

What it turns on is the wrap-up's own settle rather than the state: a steer into
Done shares nothing. It fires once per conversation, gated on the row in
`share_comments` that says a share-to-PR comment has been left on it — written
wherever a comment lands, the Share pane's press and the settle alike, and never
deleted. So a conversation shared by hand stays quiet, and a share that failed
wrote no row and is tried again at the next settle. A failure is a Notice on the
timeline naming it; a share that worked writes nothing there.

`"cleanup"` is what becomes of a conversation the human has archived: a **trim**
that takes its bulk — the full agent output, the transcripts and the session
records, which is everything a share never carried — and a **delete** that takes
the whole conversation. Each is a switch and a number of days, each clock counts
from the archiving, and unarchiving stops both. The two default the opposite
ways about, and an absent key, an absent file and one nothing can parse all say
it: the trim is **on** at three days, because what it takes is what nobody opens
twice, and the delete is **off** at thirty, because it is the one thing here
that forgets. A duration that is not a whole number of days — an empty field
included — is no duration configured, which is how the default is asked for
back; `"days_configured"` beside the number is what says which of the two is
being shown, and the settings page draws the default as a placeholder. Nothing
here is refused, a delete sooner than the trim included: the clocks are
independent, so the conversation is simply deleted before it was ever trimmed.
The sweep reads the file on every pass, an hour apart, so a switch flipped from
a phone is in force on the next one without a restart. It reports what it did in
the log and asks nothing of anybody — no timeline card, no notice — while
announcing on the nudge stream that the record moved, so an open page stops
drawing what has been taken. A pass that took something ends with a `VACUUM`,
because SQLite frees the pages a delete emptied inside the file and leaves the
file the size it was; a pass that found nothing does not. And it touches nothing
outside Verkstead's own record: the git branch stays, and a published share
stays published.

`"conflict_resolution"` is what a session sent at a pull request that will not
merge is told to do about it: `"Merge"`, which merges the base branch into the
work branch and pushes, or `"Rebase"`, which rebases the branch onto its base and
force-pushes it with `--force-with-lease`. An absent key, an absent file and one
nothing can parse all mean a merge — a rebase rewrites what reviewers have
already read and breaks anything stacked on the branch, and nobody should meet
that for never having found the settings page. In `config.yaml` the word is
lowercase, as `merge` or `rebase`.

That word is the whole of the answer, in every repo. One repo could once say
otherwise, from its own pane on the settings page; an override no settings page
draws would be a repo rebasing with nowhere to read why, so it is gone and the
file is the only place this is asked. It is picked on the Git pane, under the
author — a select of two words, saving on the pick.

The link is composed as a page is drawn rather than written down at the publish.
What the record holds is the gist's own URL, so a share published before there
was a viewer to compose through links through one now, and the viewer moving
retargets every link there is without republishing anything. The gist's own URL
travels beside it to the workbench, because the Share pane draws both: the
viewer's link is what a reader is sent, and the gist is where the file is and
the only place a share can be deleted — Verkstead deletes none of them.

Everything about sharing is on one pane of the workbench, opened by the share
icon on the timeline pane's header: where the last share went, and the download,
the publish and the share to the pull requests under it. It was four rows of the
conversation's actions menu, and none of them did anything to the conversation,
which is what that menu is for.

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
<http://127.0.0.1:8422/> and press **New conversation**. That opens the composer:
drop the **Repo** dropdown along the bottom of the box and press **Open repo**,
which takes any git repository root the server can read — the path field's
dropdown opens on your home to browse for one — and puts the draft on it. Then
press **Save as draft**.
The URL then names what it made — `/conversations/1` — and that number is the
one below.

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
phone instead, open the settings page's **Remote access** section: it says what
this machine's Tailscale is doing, its checkbox puts the tailnet name in front
of the port this server is listening on — the HTTPS push notifications need to
work at all — and the login link is drawn there as a QR code to point the
phone's camera at, the address and the **Workbench Key** together. Nothing to
run in a terminal, and nothing to paste: Tailscale itself is the machine's, and
a serve set up by hand reads on that page exactly as one set up from it would.

Where the press is refused for want of the operator grant — Tailscale allows a
serve from nobody but root and the tailnet's operator — the pane shows the
`sudo tailscale set --operator=…` that lifts it, and the next press is the
re-try. The desktop app puts that through `pkexec` instead — its sidecar
is a `serve --desktop`, and the flag is how the server knows there is somebody
at the machine to ask, where a `serve` started in a terminal has not.

## The dev loop

```console
$ cargo test              # unit, schema and end-to-end tests
$ cargo clippy --all-targets
$ cargo fmt
$ nix fmt                 # the Nix files
$ nix flake check         # the viewer's suite, and the NixOS module in a VM

$ tools/generate-icons.sh     # the favicon and PWA icons, after replacing the artwork
$ tools/generate-packaging.sh # the desktop entry, the launcher icons, the icns and the ico
$ tools/build-macos-dmg.sh    # Verkstead-universal.dmg, on a Mac
$ tools/build-windows-msi.sh  # Verkstead-x86_64.msi, on Windows
```

The last two are the Mac and Windows desktop artifacts a release ships — the
Linux one is the packed Electron app, which is `pnpm run pack` in `desktop/`
rather than a script here. Each takes everything from the working tree and
leaves one file under `target/`, and each wants `web/dist` already built,
because the viewer is compiled into the binary they wrap. Each also runs only
where its artifact does: the dmg wants a Mac for `lipo` and `hdiutil`, and the
msi wants Windows for the WiX toolset and the MSVC build under it, so the dev
shell has neither of the two.

The AppImage is the unified binary, the packaging assets and every library the
tray is drawn over, in one file, and it is the same command CI runs. It builds
what `cargo build --release -p verkstead-cli` builds, feature and all, and its
`AppRun` supplies the `desktop` verb — a desktop launcher names a file and has
nowhere to say one — so it wants the dev shell for the same reason that build
does.

The dmg is `Verkstead.app` — the same binary built for both Apple targets and
`lipo`-ed into one, the icns from `packaging/`, and an `Info.plist` whose
`CFBundleIdentifier` is `net.tobico.Verkstead` and whose `LSUIElement` is what
makes it a menu-bar app with no Dock tile. Its `CFBundleIconFile` names that
icns with the extension on it — macOS appends `.icns` only to a value that has
none, and a dotted identifier reads as having one already, so a value without
it is a name no file answers to and Finder draws the generic app icon. The
release's dmg leg reads the key back off the mounted bundle and has `iconutil`
open what it names, which is the only way that failure is visible from outside
a Finder window. It runs on a Mac only: `lipo`, `codesign` and `hdiutil` are
the operating system's own tools, and there is no cross build of it from here.
The bundle is ad-hoc signed rather than signed with a Developer ID, because
Apple silicon will not execute a binary with no signature at all — that is not
the signing that gets an app past Gatekeeper, and there is none of that.

The msi is the two files a Windows install is — the unified `verkstead` and the
windows-subsystem shim that opens it from an icon — wrapped in an installer,
because two files beside each other are not a portable download. What goes where
is [`tools/verkstead.wxs`](../tools/verkstead.wxs), and all of it goes into the
profile: `%LOCALAPPDATA%\Programs\Verkstead`, the user's own Start menu, and
the user's own `PATH`, so that `verkstead ask` works in a terminal opened
afterwards. Per-user because the package is unsigned, and asking for
administrator would be an unsigned program asking for the machine. It runs on
Windows only: the WiX toolset's `candle` and `light` compile the package, and
MSVC and the Windows SDK build what goes in it.

### The sessions suite, and the machine under it

`crates/server/tests/sessions.rs` is the one suite that is really a hundred and
forty small servers, each running a real session in a real sandbox and judged on
wall clock. That makes it the one suite whose result depends on what else the
machine is doing, so it has two knobs of its own.

`VERKSTEAD_TEST_PACE` is a multiplier over everything time-shaped in it — the
budgets a session is judged idle, rescued and ended by, how long a wait gives
up after, and every window a test holds open to prove nothing happened. Unset
is `1.0`. CI sets `2`, because a two-core runner building the workspace
alongside the run cannot meet a developer's machine's budgets, and a session
descheduled past one is judged by the wrong rule and fails a test that is not
about the code. Raise it locally if the suite fails on a busy machine and passes
on a quiet one; the file's own `PACE` explains the rest.

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

**Windows runs an arm of its own**, because that suite is `cfg(unix)`: it
stands on a mount namespace, a `/bin/sh` probe and a `stty`, and a Windows
session has none of the three. `crates/server/tests/sessions_windows.rs` asks a
dozen of the same questions of the other machine — a grilling started by
pressing the button, run on a pseudoconsole in a profile of the Conversation's
own, with a PowerShell script where claude goes — and
`crates/server/tests/terminal_windows.rs` is the console's own pair beside
`terminal.rs`, asking `mode con` what `stty` is asked there. Both are
`cfg(windows)`, so a Linux or a Mac checkout compiles neither: the
`windows-2025` job is where they run, and it installs an `sccache` on the
runner for the three of them that are about the build cache — a Compile Server
coming up as the session account, a session handed a `RUSTC_WRAPPER` it can
run, and a Rust repository's session really compiling through that server. The
last of those wants a rustup-installed Rust on the runner as well, which is what
a Windows runner has: it runs the machine's own `rustc`, through the rustup home
a session is granted.

**And a Windows checkout needs the session account, once.** Every suite that
starts a session starts it *as* that account — `sessions_windows.rs`, the two
boundary suites, `account_windows.rs`, `launcher_windows.rs`, and the
`sandbox::granting::writing` tests inside the server crate — and the wizard's
sandbox row at the foot of `account_windows.rs` asks whether there is one at
all. None of them can make an account, that being an administrator's call. So a
machine that has never run the verb fails there with the line naming it rather
than passing quietly, and the verb is run once from an elevated terminal — which
is also what the wizard's first step now does for whoever is at the app:

```console
$ verkstead session-account create   # elevated, once per Data Directory
$ verkstead session-account remove   # elevated, and the way to undo it
```

**It is the account of the machine's *own* Data Directory that they use.** An
account's name is a fingerprint of the Data Directory it belongs to, and every
one of those suites runs against a temporary one — so the account they would be
named after is one no verb was ever run for. What they run as instead is the
account the human really has, said outright on the `Homes` a sandbox is built
against and with its password copied into the fixture's own `secrets.yaml`. Run
the verb with no `--data-dir`, which is the default the suites resolve.

The `windows-2025` job runs the same verb in a step of its own, a runner being
elevated already. Nothing else about a test run is privileged, here or there.

**And `sessions_windows.rs` wants the workspace built first**, which
`cargo test` does not do for it: a session comes up on a console made on the far
side of the boundary by `verkstead session-launcher`, so the image a description
names has to be a real `verkstead.exe` rather than the test harness's own —
and a server-crate test has no `CARGO_BIN_EXE_verkstead` to read. It looks for
one beside its own binary, in `target/debug`, and says so where there is none.
`cargo build --workspace --all-targets` puts it there, which is what the
`windows-2025` job does before it runs anything.


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
bundle, and `net.tobico.Verkstead.ico`, which Windows wants twice:
`crates/desktop/build.rs` compiles it into the shim as a resource, nothing
installed beside an exe being what Alt-Tab and the taskbar draw it with, and
`tools/verkstead.wxs` names it again for the entry the msi leaves in Apps &
Features. It is written by
[`tools/generate-packaging.sh`](../tools/generate-packaging.sh) from the same
hammer, and committed for the same reason the viewer's icons are. That script
rewrites the whole directory from nothing on every run — so a size
that stops being generated stops being committed, and nothing under it is ever
edited by hand.

The icns is written by the script itself rather than by `iconutil`, which is a
Mac's: the format is a header and a PNG per icon slot, so it is generated in the
dev shell alongside everything else here rather than on the one platform that
reads it. The ico is `magick`'s own output, that tool being in the dev shell
and having no such restriction — and the sizes it is handed are those same
committed downscales, so every platform draws the same pixels.

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
