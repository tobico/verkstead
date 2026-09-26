//! Whether somebody else is already on the address.
//!
//! The port is fixed and a foreign listener on it is a dialog and an exit
//! (ADR-0012, carried into ADR-0020), so the app asks before it starts
//! anything. **Asked by connecting rather than by binding**: taking the address
//! to see whether it can be taken means letting go of it again, and the moment
//! between is the one the sidecar needs it in. A connection that is accepted is
//! something there; a connection refused is nobody.
//!
//! **And the single-instance lock is what makes the answer unambiguous.** By
//! the time this is asked, a second copy of the app has already handed over to
//! the first and gone, so whatever answers here is a daemon the machine starts
//! for itself, a `verkstead serve` somebody left in a terminal, or something
//! else entirely — one dialog for all three, because there is one thing to do
//! about them.
//!
//! **Anything that is neither is taken as nobody**, and that is not a guess
//! being waved through: the sidecar's own bind is the authority, and a bind
//! that fails is a child that exits, which the app reports and quits on. What
//! this probe buys is the *good* message in the case that actually happens.

import { connect } from "node:net";

/// How long loopback is given to say one way or the other. Generous for a
/// connection to this machine, which either lands or is refused at once.
const PATIENCE = 1_000;

/// Whether anything is listening at this address.
///
/// Never throws and never rejects: every way of not connecting is the same
/// answer, and the caller has one question.
export function taken(host: string, port: number, patience: number = PATIENCE): Promise<boolean> {
  return new Promise((answer) => {
    const socket = connect({ host, port });

    /// Say so and let the address go again, immediately — the sidecar is about
    /// to want it, and a probe that lingered would be the thing in its way.
    const said = (there: boolean): void => {
      socket.destroy();
      answer(there);
    };

    socket.setTimeout(patience);
    socket.once("connect", () => said(true));
    socket.once("timeout", () => said(false));
    // Refused, reset, unreachable: nobody is there, or nobody this app could
    // have been talking to anyway.
    socket.once("error", () => said(false));
  });
}
