//! A Profile's login, run by Verkstead and drawn in a modal any device can
//! open — see the server's `logins` module for what runs it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Where a Profile's login has got to.
///
/// Flat on the wire — `{"state": "Waiting", "url": "…"}` — so the viewer
/// narrows on a field rather than unwrapping a variant name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum LoginState {
    /// The harness is being started and has not printed its address yet.
    Starting,

    /// The harness has printed the address to log in at and is waiting for
    /// the code that page hands back.
    Waiting {
        /// The address, as the harness printed it.
        url: String,

        /// Why the last code handed over was not taken, in the words the
        /// modal shows — where one was handed over and was not.
        refused: Option<String>,
    },

    /// A code has been handed over, and the harness is checking it.
    Checking,

    /// The harness took the code, and the account reads as logged in.
    LoggedIn,

    /// It is over without a login: the harness failed, the login ran out of
    /// time, or the account still reads as logged out afterwards.
    Failed {
        /// What happened, in the words the modal shows.
        reason: String,
    },
}

/// A device opening the modal — the one that starts the login, or one joining
/// a login already running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LoginOpened {
    /// A name the modal made up for itself as it opened, so that its closing
    /// is told apart from another device's — see [`LoginClosed`].
    pub viewer: String,
}

/// The code the login page handed back, typed into the modal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LoginCode {
    /// The code, as pasted.
    pub code: String,
}

/// A device closing the modal. The login is killed once the last one has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LoginClosed {
    /// The name the modal opened under — see [`LoginOpened::viewer`].
    pub viewer: String,
}
