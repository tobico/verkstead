# 02. The project, the shell and the sidecar

## What to build

A new top-level `desktop/` pnpm project whose `pnpm start`, from the dev shell,
brings the headless `verkstead` up beside it as `verkstead serve --desktop` and
waits for the server to answer. No window yet: what this task delivers is a
running sidecar the app owns the lifetime of, and a project CI checks.

**A project of its own beside `web/`, not a package inside it** (ADR-0020): it
consumes a built binary rather than sharing the viewer's toolchain, so it has its
own `package.json` and its own lockfile. TypeScript for the main process,
compiled rather than bundled for now; `lint`, `typecheck` and `test` scripts that
say what the viewer's say, with vitest over the main process's pure parts — which
is the whole of this stage's proof (ADR-0020). There is no preload yet: nothing
here needs a bridge, and stage 03 brings one with the Desktop page.

**The dev shell's Electron is what `pnpm start` runs**, and the `electron`
`package.json` names is the same major — 43, which is what the previous task's
nixpkgs carries. electron-builder's own downloads stay CI's business, so the npm
package's binary download is not something a dev shell should be paying for: skip
it there and run the shell's Electron, and let stage 05 be the first thing that
downloads one.

**Finding the CLI is two answers, and only one of them exists yet.** In a
checkout it is the workspace's own `verkstead` binary under `target/`, built by
cargo; in a packed app it is the CLI beside the app inside the bundle, which
stages 05 to 07 put there. Write the dev-time answer, leave the bundled one a
named case, and let an environment variable override both — a developer running
against a release binary should not have to edit the app to do it.

**What the sidecar is started with.** `serve --desktop` and nothing else: the
flag is the whole of what the server is told about who started it (stage 01), and
every other setting is the server's own. The child inherits this process's
environment, which is how a checkout run reaches the checkout's data —
`VERKSTEAD_DATA_DIR` is already the server's, so the app grows no flag of its own
and a packed app that says nothing gets the platform **Data Directory**. The
child dies with the app, on every path out of it, including the one where the app
is killed rather than quit.

**Health is the wait**, and `/api/v1/health` is the one route outside the gate —
it needs no **Workbench Key** and answers `ok`. Poll it until it answers or until
a bound that is worth reporting, and say in the app's own logging which it was.

**A binary that is not there is a dialog**, naming the path the app looked at,
and a non-zero exit: a developer who has not run cargo yet is the common case,
and a message naming the path is the whole of what they need.

**And CI checks this project from the commit that adds it.** A `desktop` job
beside `viewer` in `ci.yml`, running lint, typecheck and test off a frozen
lockfile. The existing composite action that sets the viewer's toolchain up names
`web` in two places — it either grows the project as an input or the desktop job
gets a setup of its own; one of the two, not a third pnpm pin copied by hand.

## Acceptance criteria

- [ ] `pnpm start` from a fresh dev shell starts `verkstead serve --desktop`,
      waits for `/api/v1/health` and says so in the app's own logging; quitting
      the app — and killing it — leaves no `verkstead` process behind.
- [ ] A CLI that is not where the app looks is a dialog naming that path and a
      non-zero exit, with nothing started.
- [ ] A run with `VERKSTEAD_DATA_DIR` set serves out of that directory, and one
      with nothing set serves out of the platform Data Directory.
- [ ] `pnpm lint`, `pnpm typecheck` and `pnpm test` pass in `desktop/`, and a
      `desktop` job in `ci.yml` runs all three from a frozen lockfile.
