//! The Linux rendering: a [`Surface`] as bubblewrap's flags.
//!
//! What was written straight into a `Command` until the description was a thing
//! of its own — the same flags, in the same order, said about a description
//! rather than about a Conversation.
//!
//! **Everything here is a mount namespace.** A path a session may reach is bound
//! into one, and everything else is not there at all: the boundary hides rather
//! than refuses, which is the whole of the difference from [`super::seatbelt`]
//! and the reason a probe on one platform cannot be a probe on the other.

use std::process::Command;

use super::surface::{Access, Reach, Surface};

/// What the machine calls itself inside.
///
/// Said rather than inherited: the host's name is a fact about the host, and a
/// session that printed it into a commit message would be saying something it
/// has no business knowing.
const HOSTNAME: &str = "verkstead";

/// `surface` as the `bwrap` invocation that makes it.
pub(crate) fn command(surface: &Surface) -> Command {
    let mut bwrap = Command::new("bwrap");

    // Nothing of the server's environment comes through. What the sandbox holds
    // is the description's to say, and a variable the unit happened to be
    // started with — where the database is, what the server listens on — is not
    // part of it.
    bwrap.env_clear();

    bwrap.args([
        // A session outlives nothing: if the orchestrator goes, so does
        // whatever it left running. The one flag here with no equivalent on the
        // other platform, where the same promise is kept by a keeper of
        // Verkstead's own — see [`super::outliving`].
        "--die-with-parent",
        // Every namespace, and then the network back — see the sandbox module's
        // own documentation for why that one.
        "--unshare-all",
        "--share-net",
        "--hostname",
        HOSTNAME,
    ]);

    for access in surface.reaches() {
        match access {
            Access::Own { path, reach } => {
                bwrap.arg(flag(*reach)).arg(path).arg(path);
            }
            // Which is what a bind is: the host's path on the left and where a
            // session finds it on the right.
            Access::Elsewhere {
                host,
                inside,
                reach,
            } => {
                bwrap.arg(flag(*reach)).arg(host).arg(inside);
            }
            // Which is a bind too on this platform: an empty directory of
            // Verkstead's own, read-only over whatever a session would
            // otherwise have found there.
            Access::Nothing { inside, empty } => {
                bwrap.arg("--ro-bind").arg(empty).arg(inside);
            }
            // `/proc` and `/dev` are made rather than bound: they are the
            // sandbox's own, which is what makes the unshared pid namespace
            // mean anything.
            Access::ProcessTable => {
                bwrap.arg("--proc").arg("/proc");
            }
            Access::Devices => {
                bwrap.arg("--dev").arg("/dev");
            }
            Access::Temporary(path) => {
                bwrap.arg("--tmpfs").arg(path);
            }
            Access::Empty(path) => {
                bwrap.arg("--dir").arg(path);
            }
        }
    }

    for (key, value) in surface.env() {
        bwrap.arg("--setenv").arg(key).arg(value);
    }

    bwrap.arg("--chdir").arg(surface.chdir());
    bwrap.args(surface.argv());

    bwrap
}

/// The flag that makes a bind what the description said it is.
fn flag(reach: Reach) -> &'static str {
    match reach {
        Reach::ReadOnly => "--ro-bind",
        Reach::ReadWrite => "--bind",
    }
}
