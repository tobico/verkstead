# 01. The crate, the server in-process, and the browser

## What to build

A second binary beside the CLI. `crates/desktop` is a workspace member holding
`verkstead-desktop`, which links `verkstead-server` exactly as `crates/cli`
does and runs it in-process on `127.0.0.1:8422` out of the platform **Data
Directory** — the default stage 01 landed, so nothing here parses a directory
of its own. Started with nothing said, it serves the viewer and opens it in the
default browser; `--no-open` suppresses that and nothing else. The headless
`verkstead` is untouched and carries no GUI dependency (ADR-0012).

**A taken port is an error, and the check comes first.** If something is
already bound to the address — a second copy, or the daemon a NixOS module
started — the app shows a dialog naming the port and exits nonzero. It does not
front the running server and it does not pick another port; both were rejected
in the ADR. The server's own `run` binds only after it has made the Data
Directory, installed the Skills and opened the database, so a taken port
discovered there is discovered after side effects and comes back as a startup
error with nowhere to be drawn. The desktop settles the address itself before
any of that happens, and a failure at that point is the one thing this binary
has a dialog for.

**The tray's stack is what makes this the first crate here to need system
libraries.** `tray-icon` on Linux is GTK3 plus libappindicator, and GTK wants a
main loop on the main thread while the server wants a tokio runtime — the
inverse of `crates/cli/src/serve.rs`, which builds a runtime and blocks the
main thread on it. Settle that shape now, with the runtime on threads of its
own, so that task 02 has a main thread free to hand the tray.

**The packages land with the crate rather than after it**, in both places that
build the workspace. `ci.yml` builds it whole — `cargo clippy --workspace
--all-targets -- -D warnings` and `cargo test --workspace` — on a runner that
installs bubblewrap and nothing else, so the GTK and appindicator development
packages have to be there the first time `crates/desktop` is pushed, installed
by a step of their own rather than by the runner image happening to carry them.
`flake.nix`'s dev shell has neither them nor `pkg-config`, so without the same
addition nothing on a developer's machine could build the crate at all. The
same packages are what stage 03 has to put inside the AppImage.

`nix/verkstead-source.nix` builds `--package verkstead-cli` and wants no
change: the packaged binary, the flake and the NixOS module go on being the
headless one. `docs/development.md` gains the line that says how to run the
desktop app out of a checkout, beside the `--data-dir .` commands that are
already there.

## Acceptance criteria

- [ ] `cargo run -p verkstead-desktop` serves the viewer at `127.0.0.1:8422`
      out of the platform Data Directory and opens it in the default browser;
      `--no-open` opens nothing and serves just the same.
- [ ] With the address already bound, the app shows a dialog naming the port
      and exits nonzero, having made no Data Directory and written nothing.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
      warnings` and `cargo test --workspace` pass both in the dev shell and on
      the CI runner, with the GTK and appindicator development packages
      installed by a step in `ci.yml` and carried by `flake.nix`'s dev shell.
- [ ] The headless `verkstead` binary builds and behaves exactly as before, and
      the nix package still builds the CLI alone.
