//! Where the workbench is, which is one address and never a question.
//!
//! The port is fixed at 8422 and a foreign listener on it is a dialog and an
//! exit rather than a port picked (ADR-0012, carried into ADR-0020): the CLI's
//! own default names it and so does the `tailscale serve` the workbench sets
//! up, so a Verkstead that moved when the port was busy would be a Verkstead
//! reachable from the phone at an address nothing had told the phone about.
//! The dialog in front of it is `main.ts`'s, over the probe in
//! [`taken`](./taken.js); this is the address every part of that reads.

/// The loopback port the server binds when nothing says otherwise — the
/// `WORKBENCH_PORT` of `crates/server`.
export const PORT = 8422;

/// The interface it binds, which is this machine and no other: what reaches
/// Verkstead from the phone is the `tailscale serve` the workbench sets up
/// rather than anything listening wider than loopback.
export const HOST = "127.0.0.1";

/// The two together, which is what the probe asks about, what the dialog names
/// when somebody else is already there, and **what the sidecar is told to bind**
/// — see [`ARGUMENTS`](./sidecar.js).
///
/// Told rather than left to the server's own default, which is the same number:
/// the server reads [`LISTEN`] from the environment it inherits, so a shell that
/// exported one would otherwise put the server on an address the probe, the
/// health wait and the window all know nothing about. One address, named once,
/// and the app is what names it.
export const ADDRESS = `${HOST}:${PORT}`;

/// The variable the server takes an address from, which the app overrides on the
/// command line and reports having overridden.
///
/// It is read here for one line of logging and nothing else. The app has no
/// setting for where Verkstead listens and this is not the beginning of one: the
/// port is fixed by decision (ADR-0012, carried into ADR-0020), so a developer
/// who has this exported for a `verkstead serve` of their own is owed the news
/// that the app ignored it rather than half an hour wondering which of the two
/// halves is wrong.
export const LISTEN = "VERKSTEAD_LISTEN";

/// The origin the window loads and the sidecar answers on.
export const ORIGIN = `http://${ADDRESS}`;

/// The one route outside the gate: no **Workbench Key**, no cookie, and `ok`
/// once the server is up. Everything else needs a login the app has not made
/// yet at the moment it is waiting.
export const HEALTH = `${ORIGIN}/api/v1/health`;
