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

/// The two together, which is what the probe asks about and what the dialog
/// names when somebody else is already there.
export const ADDRESS = `${HOST}:${PORT}`;

/// The origin the window loads and the sidecar answers on.
export const ORIGIN = `http://${ADDRESS}`;

/// The one route outside the gate: no **Workbench Key**, no cookie, and `ok`
/// once the server is up. Everything else needs a login the app has not made
/// yet at the moment it is waiting.
export const HEALTH = `${ORIGIN}/api/v1/health`;
