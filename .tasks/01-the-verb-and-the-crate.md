# 01. The verb and the crate

## What to build

Delete the Rust tray app. `crates/desktop` goes off disk and out of the
workspace members; the CLI loses its `desktop` verb, its default-on `desktop`
cargo feature and the `tracing` dependency that feature carried; the CLI's
end-to-end desktop suite goes with them, as does the one test in the `serve`
suite that asserts the verb does not take `--desktop`. Nothing of what that
suite covers is lost: the `serve` suite already asserts the sidecar half — the
startup line naming the address alone, the key left in its file, a sidecar with
nowhere to draw — and the tray half is the Electron app's own vitest suites and
the three Release legs.

That leaves the server crate with three public ways in where one is used: the
one that binds an address itself, the one handed a bound socket, and the keyed
one the tray app reached for. Only the tray app ever called the keyed one, and
only the first calls the second, so **collapse all three into the one entry**
and follow the doc links that name the two that go — the pipe, sandbox and
platform modules each point at one.

CI names the crate in more places than the stage brief counted, and every one
of them belongs here rather than in task 02: a workspace build passing
`--features verkstead-desktop/shim` fails outright the moment the package is
gone, so the Windows job's build, test and doc-test steps go with the Linux
job's toolkit install step, the dbus tray leg at the end of it, and the nextest
override that held the registry tests serial. A grep for `verkstead-desktop` and
`verkstead_desktop` outside the Electron project is the list — and the Electron
project's own log lines carry that same spelling deliberately, so they are not
part of it.

And one script runs the verb: the packaging generator writes a desktop entry
whose `Exec` is `verkstead desktop`. **Stop writing the entry at all.** Nothing
installs it — electron-builder writes the packed app's own entry with an `Exec`
no configuration replaces, and the two fields this one lent it are already
written in the Electron project's build configuration — so the entry, the
`desktop-file-validate` call over it and the committed file all go, and the
script keeps the icons, the icns and the ico. The script's own pointers into the
deleted crate go with it, and so does the sentence in the Electron build
configuration that says where the entry's fields come from; every other pointer
at the crate is task 03's.

Leave the Electron app otherwise alone. The code that takes over the tray app's
launch agent on a Mac and its Run value on Windows is what upgrades the machines
that app ran on, and it is unaffected by the app itself being gone.

## Acceptance criteria

- [ ] `cargo build --workspace` and `cargo clippy --workspace --all-targets`
      are clean with nothing excluded, in a shell carrying no GTK and no
      pkg-config
- [ ] `verkstead --help` names no `desktop`, and `Cargo.lock` holds no `gtk`,
      `tray-icon`, `ksni`, `zbus`, `objc2` or `windows-registry`
- [ ] No workflow step, `--features` flag or nextest override names
      `verkstead-desktop`, and neither the toolkit install nor the tray leg is
      in CI any more
- [ ] `packaging/` holds no desktop entry, the script that wrote it validates
      nothing, and the whole test suite passes on this machine
