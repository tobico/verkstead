# Onboarding

Builds on [ADR-0015](0015-open-boundary-and-workbench-key.md): the wizard is
designed for a Verkstead with no Watched Path and a workbench key.

A fresh Verkstead is unusable until four things are true, and today nothing
says which of them is false. The server never checks for `bwrap`, `git` or an
agent binary — a missing one is a session that silently fails to start,
logged and nothing more. Nothing reads `~/.claude` to offer a Profile, nothing
reads `~/.gitconfig` to offer an author, and the compose page draws an empty
Repo dropdown with no word about why. The adoption docs carry the whole of the
setup as prose. This decision replaces that prose with an **onboarding mode**
the server enters at startup, and gives the compose page a zero state and a
way to make a repository.

## Onboarding mode

**Evaluated once at startup.** The server computes whether the objective is
met — a sandbox, `git` and at least one harness present; at least one Agent
Profile; a git author — and enters onboarding mode where it is not. The mode
stays on until the wizard finishes and never re-enters until the next start:
deleting the last Profile mid-run is the settings page's empty state to say,
not a return to the wizard. There is no skip. The GitHub token is the one
thing the wizard asks for that does not gate it, because GitHub may not be in
use at all; git is not optional, so the author is.

**One page, one endpoint.** The wizard is `/setup`, the current step kept on
the device; every other URL redirects there while the mode is on. `GET
/api/ui/onboarding` is its model: the mode, the platform and distro, each
dependency's presence, and which steps stand met. It probes on every read —
the probes are cheap — and the wizard re-reads it every ten seconds while a
step is unmet and on window focus, which the app already does. Nothing runs
when nobody is looking. Nothing is folded into the settings view: the verdict
is a different question from what is configured.

**Present means a session would find it.** A session resolves `claude`, `git`
and the rest on the PATH inside the Sandbox, and every probe resolves on that
same list, read once at startup and held where the sandbox and the probes
both read it. **That list is the server's own `PATH`, in the order it was
written** (amended 2026-09-08 — it was a fixed list of system directories
until then): Verkstead's own `bin` first, then the entries of the `PATH` the
server was started with, then the machine's fixed list as a floor,
`/usr/local/bin` among it because that is where `npm install -g` on the Debian
family puts a binary. First occurrence wins, empty and relative entries are
dropped, and an entry neither under the server's home nor under the
platform's floor is dropped too, since a session could not reach it — which is
what removes the `/mnt/c/...` entries WSL appends. Every entry that remains
under the server's home is bound read-only into the sandbox, the rule the
Windows boundary already applied. For every name the wizard has a row for,
kept in one constant, a symlink is followed and the directory it lands in is
bound read-only where that is under the home, so Claude's native install —
`~/.local/bin/claude` linking into `~/.local/share/claude/versions/` — is
found and launched; a target elsewhere, or a dangling link, is not found. Every
Claude session runs with `DISABLE_AUTOUPDATER=1`, so no session writes into
the human's install. Nothing is added the `PATH` did not name: a Mac app
started from the Dock has launchd's `PATH` and never sees `~/.local/bin`, and
the NixOS module's service user has no `~/.local/bin` of the human's — both
are follow-ups rather than part of this. The row says where the name
resolved and what it links to, and a name seen on the server's `PATH`
somewhere a session cannot reach reads absent with a note saying so. The Linux
sandbox row is a trivial `bwrap` run rather than a PATH lookup, because
unprivileged user namespaces can be off and the AppImage cannot carry
`bwrap`; a failure's stderr is shown under the row. On macOS `sandbox-exec`
is on every Mac and the row is simply ticked; on Windows it reads *not
applicable* with the existing unsandboxed note and passes. `gh` is an
optional row: shown present or absent with its instruction, never gating.

**Instructions per OS.** The distro is read from `/etc/os-release`, `ID` then
`ID_LIKE`; the detected tab opens and the others stay reachable in case it is
wrong. macOS, Windows, NixOS, Ubuntu, Fedora, Debian and Arch — each with its
derivatives — get exact commands for `bwrap` and `git`; a harness gets the
exact command where the OS has a package for it (nixpkgs, Homebrew, npm) and
a link to the vendor's install page where not, each saying where the binary
must land. Claude's row leads with the native installer on every Linux tab and
on Windows, saying `~/.local/bin` must be on the `PATH` of the shell Verkstead
is started from and the server then restarted, the `PATH` being read once;
macOS keeps Homebrew first, the Dock never handing an app the shell's `PATH`.
Every tab draws the real list a session looks on, sent on the wire, rather
than prose about it. Any other Linux gets the generic list of what is needed.
The step waits for **Continue** — it never advances under somebody's hands.

**Accounts are detected in the server's HOME**: `~/.claude` with
`~/.claude.json`, `~/.codex`, `~/.grok`, and opencode's XDG pair; on Windows
under `%USERPROFILE%`. Each is offered ticked where its harness is present,
listed unticked and greyed where not. A ticked account becomes a Profile with
a **null name** and every model this build knows for that harness, both
editable later on the settings page. A name is for telling two accounts of
one harness apart, which is rare, so at most one Profile per harness may be
unnamed, and an unnamed one shows no name wherever the harness mark and the
model already say enough — reading *Default* only where a name must be shown.
Nothing found says what to run to make one (`claude` once, then log in),
keeps probing on the same cadence, and offers the manual form under it.

**The step's Continue needs a Profile**, ticked or made in the manual form —
one of the three the objective is, and the only one a step could otherwise
walk past. Unticking everything and pressing Continue would clear the mode
with nothing saved and land on exactly the empty state a skip was rejected
for, until the next start put the human through the wizard again. The
dependency step has its rows and the git step has its required fields; this
is the same rule said for the step that had none.

**The git step asks for what git needs**: author name and email, required,
and the token, optional. The settings module's rule is *told, not found*, and
a prefill the human confirms is still telling: name and email come from the
server's own `git config --global`, the token from `GH_TOKEN` or
`GITHUB_TOKEN` in the server's environment and then from the host `gh`'s own
login, each labelled with where it was found and none saved until Continue.
The GitHub login is shown from verifying the token, never typed. Saving goes
through the settings save the page already has, verification included.

## Installing what is missing

*Amended 2026-09-11.* The dependency step drew every missing dependency's
install command and waited for it to be pasted into a terminal on the server
machine. Now **an absent row carries a checkbox** where its tick would be, the
gating rows ticked by default and the rest not, and **Next installs what is
ticked** — pressable once the present rows plus the ticked ones would meet the
objective. Nothing says in advance which rows cannot be installed here.

**The privilege comes through the `Elevate` seam the Remote access pane
already uses**: the desktop app with a screen hands the server the platform's
own password dialog, and a server started any other way is handed none. So the
dialog appears on the machine running Verkstead, and a server with no way to
ask — the NixOS service, a headless `verkstead serve`, the app over SSH —
sends every ticked row to the hint screen without asking. **One dialog per
Next**: the distribution's package manager run directly, naming every ticked
package, with `nodejs` and `npm` added where an npm harness is ticked and the
`npm install -g` joined onto the same line. `apt` has its list brought up to
date in front of that install and joined to it with `;` — a stale list is an
install that fails fetching a version the pool has dropped, and there is no
second dialog to retry on, while a refresh that could not reach one source is
no reason to refuse the packages the archive still carries. `dnf` refreshes
itself and `pacman`'s refresh is Arch's own `-Syu`, which is more than a press
of Next asked for. The vendor installers run after it
**as the user**, never elevated: Anthropic's for Claude Code, xAI's for Grok
Build, and Homebrew's where a Mac has no `brew`, after one elevated step makes
its prefix. Every Windows install goes through the runas arm. A run is a
sequence of units, one command each; cancel finishes the unit under way and
skips the rest.

**A directory Verkstead installs into is `session_path`**, a list in
`config.yaml` composed ahead of the server's own `PATH` entries under the same
rules, read at startup and grown mid-run when an install lands. This is the
one amendment to *nothing is added the `PATH` did not name*: an install the
human ticked is a directory they were told about, which is the settings
module's *told, not found* rather than the guess that rule refused.

**The Windows sandbox row is the session account.** It read *not applicable*
and passed; it is now probed, present when the local account this Data
Directory's sessions run as exists, and it gates the step, since no session
starts without it. A ticked row runs `verkstead session-account create`
elevated, and the pipe is re-opened granting the new account, so nothing
restarts. The by-hand line on the hint screen names this server's Data
Directory, which is why the reading carries it: an account is named after the
directory whose sessions run as it, and a bare line resolves the platform's
default instead — a different account, on a row that gates the step, and on the
one screen a server with a Data Directory of its own is most likely to reach.

**Three screens.** The checkboxes; then an install screen with a progress bar
of one unit per ticked row and a status line naming the phase and the machine
the dialog is on, which moves on by itself when the run ends — to the next step
when every ticked row is present, to the hint screen otherwise; and the hint
screen, drawing only the ticked rows still absent with the failure under each,
the OS tabs, the PATH list and the restart note, its Next held with a counter
beside it until every ticked row is detected, and Back to the checkboxes. The
page polls every two seconds while a run is going. The install endpoint answers
only while onboarding mode is on, and every button in the wizard reads Next.

## The zero state

The wizard ends on the compose page, and the compose page has a **zero
state** that is a fact about the list rather than about onboarding: whenever
the sidebar's list *as filtered* is empty — no unarchived Conversation, and
archived ones either absent or hidden — the sidebar is not drawn, `/`
redirects to `/compose`, the wordmark and the gear take the pane title's
place, and the archived switch is pinned to the page's bottom-left, hidden
when nothing is archived. Switching it on with archived Conversations present
brings the sidebar back with them in it, because a switch pinned to a page
has to show something when pressed. The settings page keeps its sidebar.

## Repositories that do not exist yet

A fresh install has no Repo to compose against, so the Repo dropdown grows two
rows at its foot, behind a rule: **Create repo** and **Open repo**, each a
modal, serving the compose page and a draft's composer alike — one control,
so a draft moved onto a repo made there is the same move as picking a
registered one.

Create takes a parent directory and a name. The parent is browsed from the
server's `HOME`, which is where ADR-0015 starts an unbounded browse and the
only answer a first run has, there being no last parent to remember yet. It
makes the directory, runs `git init` on `main`, and commits a `README.md`
holding the name as the configured author — an empty repository has a default
branch but no commit, and a Conversation cannot take a base from one. When a
token is saved it offers **Create on GitHub too**, ticked and private,
through `gh repo create` after the initial commit; without a token it says a
remote is needed before the work is finished, since the pipeline ends in a
push and a pull request. Open is the registration form's path field in a
modal, browsing anywhere with repositories marked, refusing as the settings
page does. Either way the Repo is registered and becomes the draft's.

## Considered Options

- **A live onboarding predicate**, re-entering the wizard whenever the
  objective stops being met. Rejected: deleting a Profile mid-run would drop
  the human into a first-run page over work they have.
- **A skip.** Rejected: nothing runs short of the objective, and the empty
  states it would fall through to are the ones the wizard exists to replace.
- **Gating on the token and the author too**, or on neither. Rejected for
  the shape above: git is a dependency and its author is what git asks for;
  GitHub is a choice.
- **Probing on the server's PATH and binding what was found into every
  sandbox.** Rejected at first: a `node`-based install needs `node` bound in
  too, and a directory of the human's read-only in every session is a hole.
  Reversed on 2026-09-08, when a distribution's own `claude` on Ubuntu under
  WSL proved too old to connect and the native installer's `~/.local/bin` was
  the only current one: the hole is bounded to the per-user directories the
  `PATH` names and the link targets under the home, `node` under nvm sits in
  one of those directories already, and a harness the human cannot run is the
  larger failure. What stays rejected is binding anything outside the home on
  the `PATH`'s account — such an entry is dropped instead.
- **Adding `~/.local/bin` whether or not the `PATH` names it**, and **reading
  the login shell's `PATH` by running the shell.** Rejected: the first hands a
  session a directory the human never put on their `PATH`, and the second
  spawns a shell whose rc files can hang or print. The server's own `PATH` is
  the one the human started it with.
- **Probing on the sandbox PATH as it was.** Rejected: only nix and the
  Fedora and Arch distro packages land on it.
- **A server timer that nudges the page.** Rejected: the probes are cheap
  and the page already re-reads on focus; probing on read means nothing runs
  unwatched.
- **Advancing a step by itself when its rows tick.** Rejected as jarring.
- **A separate GitHub username field.** Rejected: the login is a fact the
  token verification already returns.
- **An empty initial commit**, or none. Rejected: a README is what a fresh
  repository conventionally holds, and no commit leaves the draft without a
  base.
- **A projects-directory setting** in place of the paths step. Rejected: the
  modal asks for a parent directory, and nothing else needed one.
- **A password field in the wizard**, so a phone could drive a headless
  server through `sudo`. Rejected: a password typed into the workbench is a
  secret on the wire the seam already avoids, and the machines with nobody at
  them are the machines whose instructions the hint screen keeps.
- **One dialog per ticked row.** Rejected: the packages are one package-manager
  command, and a dialog per row is the same password typed five times.
- **Claude's packaged install under elevation**, landing in `/usr/local/bin`
  where no PATH question arises. Rejected: the packaged one is the one that
  goes stale, which is why the tab leads with the native installer.
- **Holding the installed-into directory for this run only**, or in the
  database. Rejected: the next start would lose it, and a directory the human
  cannot see is one they cannot correct.
- **A restart after the session account is made.** Rejected: the pipe can be
  re-opened with the grant, and a wizard that asks for a restart mid-step is
  the terminal it was replacing.
- **Saying in advance which rows need a step by hand.** Rejected as clutter
  on the checkbox screen; the hint screen says it where it matters.
