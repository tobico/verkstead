//! What the Remote access section is drawn from: what this machine's Tailscale
//! is doing, read off the machine at the moment the pane is opened.
//!
//! Nothing here is a setting. Every field is what two commands said a moment
//! ago, which is why there is no stored half to reconcile against: a tailnet
//! joined from a terminal, or a `tailscale serve` somebody set up by hand, reads
//! here exactly as one set up from this page would.
//!
//! **Three answers rather than two**, because a machine with no `tailscale` on
//! it and a machine whose daemon is not answering want different things done
//! about them — one is an install and the other is a `tailscale up` — and
//! neither of them is *serve is off*. So the state the pane narrows on says
//! which of the three it is, and only the third one carries a node name and a
//! serve state at all.
//!
//! **And a fourth that is neither**: an answer this build cannot read. The JSON
//! `tailscale status` prints is documented as subject to change between
//! releases, and Tailscale is whatever the host has rather than anything this
//! repository pins — so a shape that is not recognised says so, rather than
//! being folded into the nearest state that happens to have room for it. The
//! serve state carries the same fourth answer for the same reason: a serve
//! configuration that could not be read is *cannot tell*, never *off*, because
//! *off* is what the switch beside it would offer to turn on.
//!
//! `trouble` is what the machine said, verbatim wherever there were words to
//! take — the line `tailscale` printed on standard error is the one that names
//! the systemd unit to start, and no sentence written here would be as useful.
//!
//! **And the one thing here that is pressed rather than read**: the serve
//! switch. It runs `tailscale serve` and answers with the machine read again,
//! so the position it settles at is a reading like every other field on this
//! page. Its own third answer is the operator grant — see [`ServePress`].
//!
//! **The login link rides along with the address**, because it is the address
//! with the Workbench Key on it and the key is the one thing the browser has
//! not got: the cookie carrying it is `HttpOnly`, so a page cannot build the
//! link it is about to draw as a QR code. Re-issuing the key answers with the
//! whole reading again — see [`RemoteView::Up`] — so that the QR and the link
//! beside it redraw on the new one rather than being asked for a second time.
//!
//! **And one field that is not about the machine at all**: whether the human is
//! done with the banner that points at this section — see [`RemoteBanner`]. It
//! is the one thing here that is stored rather than read off `tailscale`, and it
//! is here because what it is about is Remote access rather than any of the
//! Conversations the banner is drawn on.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What this machine's Tailscale is doing.
///
/// Flat on the wire — `{"tailscale": "Up", "node": "…", "serve": {…}}` — so the
/// viewer narrows on a field rather than unwrapping a variant name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tailscale")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum RemoteView {
    /// There is no `tailscale` anywhere the server looks: on its PATH, and on a
    /// Mac inside the Tailscale app too. The one state whose answer is somewhere
    /// else entirely: nothing on this page can install it.
    Absent,

    /// There is, and it is not up: the daemon is not answering, or it is and
    /// this machine has not joined a tailnet.
    Down {
        /// What `tailscale` said about it — its own line where it printed one,
        /// because that is what names the service to start.
        trouble: String,

        /// Whether the `tailscale` that said it is the Mac app's own CLI —
        /// the standalone download, the App Store build or the Homebrew cask —
        /// rather than one on the PATH. A fact about the program rather than a
        /// sentence: what the pane says about it is the viewer's, and a Mac
        /// app is logged in to by opening it rather than by `tailscale up`.
        mac_app: bool,
    },

    /// It answered, in a shape this build does not recognise.
    Unreadable { trouble: String },

    /// It is up, and this machine is on the tailnet as `node`.
    Up {
        /// The node's own name, as the tailnet knows it —
        /// `workbench.tailnet-name.ts.net`, with the trailing dot a DNS name
        /// carries taken off.
        node: String,

        /// And whether anything on that name is proxied to the workbench.
        serve: ServeView,

        /// The login link a phone is let in by: the served address with the
        /// Workbench Key on it, which is what the pane draws as a QR code and
        /// offers to copy.
        ///
        /// Composed here rather than in the browser because the key is the one
        /// thing the browser is not given — it is carried in a cookie no script
        /// reads — and it is a field of the reading rather than of
        /// [`ServeView::On`] because the serve is what `tailscale` said and this
        /// is what Verkstead makes of it.
        ///
        /// `None` where there is nothing to build one on: a machine on the
        /// tailnet serving nothing has no address a link could point at, and
        /// nor has one whose serve could not be read.
        link: Option<String>,
    },
}

/// Whether `tailscale serve` is putting this machine's tailnet name in front of
/// the port the workbench is served on.
///
/// The port matters: a serve of somebody else's port is not this one, and would
/// read as an address that answers with something that is not the workbench.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "serve")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ServeView {
    /// Nothing is proxied to the workbench.
    Off,

    /// Something is, and this is where it answers — the `https://….ts.net`
    /// address a phone on the tailnet is pointed at.
    On { address: String },

    /// And a serve configuration that could not be read, which is not the same
    /// as one that is empty.
    Unreadable { trouble: String },
}

/// Where the serve switch is being put.
///
/// A press rather than a setting: nothing of it is saved, and what the switch
/// reads as afterwards is the machine read again — see [`ServePress::Done`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ServeEdit {
    /// Whether the workbench is to be served to the tailnet.
    pub on: bool,
}

/// And what came of the press.
///
/// Three answers, because the middle one is the whole of why this is not simply
/// a command that worked or did not. `tailscale serve` is refused outright for a
/// process that is neither root nor the tailnet's **operator**, and the only
/// thing that lifts it is a line somebody runs in a terminal. So a refusal
/// carries that line rather than an apology, and the next press runs the same
/// command again — which is all a re-try is once the grant has been made.
///
/// Nothing here escalates anything. The server has no privilege to raise and no
/// business asking for one; what the desktop app does with the same line is its
/// own, and still the human's press.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "press")]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ServePress {
    /// It went through, and this is what the machine reads as now.
    ///
    /// The reading travels back with it because the switch's position is read
    /// rather than remembered: a press that answered only *yes* would leave the
    /// page holding an opinion of its own about a machine somebody else may have
    /// changed in the meantime.
    Done { reading: RemoteView },

    /// Tailscale would not take it from this user, for want of the operator
    /// grant.
    Ungranted {
        /// The line that grants it, for this machine's own user —
        /// `sudo tailscale set --operator=ada`. Copied into a terminal, run,
        /// and then the switch pressed again.
        grant: String,

        /// And what `tailscale` said when it refused, in its own words.
        trouble: String,
    },

    /// And every other way running it can fail.
    Trouble { trouble: String },
}

/// Whether the human is done with the banner that points at this section.
///
/// The banner stands on a Conversation page above the Timeline, at every
/// grilling start until it is dismissed, while the first Question Set is being
/// prepared — the one moment the human has nothing to do at the desk, and so
/// the moment worth telling them they need not stay at it.
///
/// Read off the server on every load rather than out of the browser it was
/// pressed in, which is the whole of why it is on this wire at all: the banner
/// is about picking up a phone, and a dismissal that did not travel would meet
/// the human again on the very device it had just sent them to.
///
/// One direction. There is nothing on any page that puts it back, so what is
/// sent is a press rather than a position — unlike the archived switch's
/// [`ShowingArchived`](crate::ShowingArchived), which this is otherwise written
/// beside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct RemoteBanner {
    /// True once somebody has pressed it away, on this device or any other.
    pub dismissed: bool,
}
