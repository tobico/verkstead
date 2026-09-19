//! The Linux rendering: a [`Surface`] as bubblewrap's flags.
//!
//! What was written straight into a `Command` until the description was a thing
//! of its own — the same flags, in the same order, said about a description
//! rather than about a Conversation, and handed back as a [`Rendering`] rather
//! than as a way of spawning one.
//!
//! **Everything here is a mount namespace.** A path a session may reach is bound
//! into one, and everything else is not there at all: the boundary hides rather
//! than refuses, which is the whole of the difference from [`super::seatbelt`]
//! and the reason a probe on one platform cannot be a probe on the other.

use super::rendering::Rendering;
use super::surface::{Access, Reach, Surface};

/// What the machine calls itself inside.
///
/// Said rather than inherited: the host's name is a fact about the host, and a
/// session that printed it into a commit message would be saying something it
/// has no business knowing.
const HOSTNAME: &str = "verkstead";

/// `surface` as the `bwrap` invocation that makes it.
///
/// Nothing of the server's environment comes through, which is said by there
/// being none here at all: a [`Rendering`] is the whole of what the process is
/// handed — see that module — and what the sandbox holds is said in
/// `--setenv` flags below rather than to `bwrap` itself.
pub(crate) fn command(surface: &Surface) -> Rendering {
    let mut bwrap = Rendering::running("bwrap");

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
            // Made on the host rather than in the namespace, and so an arm
            // with no flag: a root has to be a real
            // directory before anything can be bound out of it, and what
            // reaches it is the bind said after this.
            //
            // **Failures are logged rather than raised**, for the reason the
            // Mac's own making is: a rendering cannot refuse, and a root that
            // could not be made is a bind that fails saying which path.
            Access::Built(path) => {
                if let Err(error) = super::emptied(path) {
                    tracing::error!(
                        error = ?error,
                        built = %path.display(),
                        "a directory a session's root is built in could not be made, so the \
                         session will not find what was to be built there"
                    );
                }
            }
            // And a file written into that directory, on the host for the same
            // reason and logged for the same reason.
            Access::Written { path, contents } => {
                if let Err(error) = std::fs::write(path, contents) {
                    tracing::error!(
                        error = ?error,
                        written = %path.display(),
                        "a file a session's root is given could not be written, so the session \
                         will not find it"
                    );
                }
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
