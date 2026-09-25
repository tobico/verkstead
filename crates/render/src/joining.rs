//! Asking to be let into a cluster: what Add sends, what the device being asked
//! sends back, and what the pending row it leaves behind says (ADR-0020, *The
//! join*).
//!
//! **Five types across two wires.** [`NewJoin`] goes from the browser to the
//! workbench — one typed address, which is the one thing on the Remote access
//! pane that is configured rather than read. What goes over the *peer* listener
//! from there is a [`DeviceIdentity`](crate::DeviceIdentity), the asking device
//! saying what it is to a device that has never met it, and [`JoinHeld`] is what
//! comes back: what the far end is calling this request, when it lets go of it,
//! and what the far end itself is. [`JoinSettled`] is what comes back *after*
//! that, on the dial the far end makes when its human has pressed something —
//! the roster on an Allow, and the word that there is nothing coming on a Deny
//! or an expiry. And the last two are the two sides of the waiting, each read by
//! a browser: [`PendingJoin`] is the row the asking device draws, and
//! [`AskingDevice`] is the question the device that was asked is holding, which
//! is what its modal is drawn from.
//!
//! **The asking device says what it is with the same type it would answer a
//! stranger with**, rather than with a shape of its own. A join is a device
//! introducing itself, and there is one description of a device in this tree:
//! the id every record names it by, the fingerprint of the certificate it is
//! presenting, the name and the OS word it is shown under, and every address a
//! peer could reach it on. A second shape would be the same five fields with
//! somewhere new for them to disagree.
//!
//! **The pending row does not carry the fingerprint it draws.** What it shows is
//! the *asking* device's own — the same string the modal on the far end shows,
//! so that two people, one reading a phone and one reading a screen, are
//! comparing one certificate. That string is already on the reading, on this
//! device's own row, so a copy of it beside every pending request would be the
//! same fact written twice.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::device::DeviceIdentity;

/// What **Add** takes: an address to go and ask at.
///
/// **A port is optional and usually absent.** Every device answers on the peer
/// port unless its host has been told another, so what somebody types is a
/// machine's name or its address — and the port is something they should not
/// have to know. One that carries a port is dialled at it, because an install
/// that was told to listen elsewhere is reachable no other way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct NewJoin {
    pub address: String,
}

/// What a device answers a join post with: the request it is now holding, and
/// itself.
///
/// **Not a viewer type.** This one crosses the peer listener rather than the
/// workbench — it is one Verkstead speaking to another, and no browser ever
/// reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinHeld {
    /// What this device is calling the request, which is what a cancel names
    /// and what the answer to it will arrive under.
    ///
    /// Invented by the device that is holding it, because that is where the
    /// request lives: the asker's row is a note about a question somebody else
    /// has been handed.
    pub request: String,

    /// And when it lets go of it, RFC 3339 — ten minutes on from the moment it
    /// arrived.
    ///
    /// Told to the asker rather than left for it to reckon, because this end is
    /// what will refuse a call naming a request that has run out: an asker
    /// counting its own ten minutes would go on drawing *waiting* over a
    /// question there was no longer anything to answer.
    pub expires: String,

    /// And what this device is, so the asker's pending row can name the device
    /// it is waiting on rather than repeating the address back at the human.
    ///
    /// The fingerprint on it is checked against the certificate the handshake
    /// presented, the way every other answer in a cluster is: a device that
    /// names one certificate and presents another is not the device it says it
    /// is.
    pub identity: DeviceIdentity,
}

/// And what it says when its human has settled the question: the dial back
/// (ADR-0020, *A cluster is a membership*, *The join*).
///
/// **Not a viewer type either.** This crosses the peer listener in the opposite
/// direction to everything else about a join — the device that was *asked*
/// dials the device that asked it, at the addresses the request is holding and
/// pinned on the certificate it pinned. Which is the third and last route
/// outside the member gate: it arrives before the asker has recorded anybody,
/// so it is matched against that pending request rather than against a
/// membership.
///
/// **Three arms rather than two, because a refusal and an expiry are not one
/// thing.** A Deny is a press nothing on the asking end could ever have worked
/// out, so it is recorded there and the row reads it; an expiry is the far
/// end's own word for a moment the asking end already holds, so nothing is
/// recorded for it and the call is worth making only because it takes the row
/// from *waiting* to *expired* the moment it happens rather than whenever the
/// page next asks. A call that never arrives costs nothing: the asker's clock
/// reaches the same answer on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "settled", rename_all = "snake_case")]
pub enum JoinSettled {
    /// **Allow**: the roster, which is the whole of what a newcomer is given.
    Joined {
        /// The device that was asked, as it answers for itself — checked by the
        /// asker against the certificate it met when it posted the join, which
        /// is the other half of the pinning.
        introducer: DeviceIdentity,

        /// And every device that one is already linked to, so that a newcomer
        /// joining a cluster of three lands holding all three.
        ///
        /// **Carried even where it is empty**, which in a cluster of two it is:
        /// the handover is one shape whatever the cluster's size, and a
        /// newcomer that had to be told about the others in some second call
        /// would be a link that was half made until that call got through.
        ///
        /// A member arriving this way needs no press of its own. It came over a
        /// link the asker has just proved against a certificate it pinned
        /// itself, which is the same vouching that lets the introducer announce
        /// the newcomer to each of them.
        members: Vec<DeviceIdentity>,
    },

    /// **Deny**: the human at the far end said no, and nothing is recorded on
    /// either side.
    Denied,

    /// And the ten minutes running out with nobody having pressed anything.
    Expired,
}

/// One device asking to be let into this one's cluster, as the modal draws it.
///
/// **The other side of [`PendingJoin`].** That one is the row the *asking*
/// device draws while it waits; this is the question the device that was asked
/// is holding, and it carries the whole of what the asker said about itself
/// because that is what the human is being asked to judge.
///
/// **And it carries no moment, where the pending row carries a flag.** A row
/// that has run out is still drawn, reading so — somebody pressed Add over
/// there and is owed the answer that nobody pressed anything back. A question
/// that has run out is simply not asked any more, so it is left out of this
/// list where it is answered, and the modal over it goes when the page reads
/// the list again. Which leaves the page nothing to count down and nothing to
/// hold a clock for.
///
/// **The identity whole rather than four fields picked out of it**, the way
/// [`LinkedDevice`](crate::LinkedDevice) carries one: it is the same description
/// of a device the join post arrived as and the same one a member row keeps, so
/// the modal and the row a press on it creates cannot come to disagree about a
/// name or an OS word. The modal draws the name, the mark for the OS, the first
/// address advertised and the fingerprint — and that last is the string the
/// asking device's own pending row is drawing, which is the whole reason both
/// ends show one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct AskingDevice {
    /// What this device calls the request, which is what Allow and Deny name.
    pub request: String,

    /// And what the device asking said it is: the id, the fingerprint of the
    /// certificate the handshake took from it, the name and OS word it is shown
    /// under, and every address it advertised.
    pub identity: DeviceIdentity,
}

/// One join this device is waiting on, as the Devices section draws it.
///
/// **A row that is not a device yet.** It sits under the members with nothing of
/// a device's own on it — no OS mark and no addresses — because nothing has been
/// agreed: what is known is where this device knocked, what answered, and that
/// somebody at the far end has been asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PendingJoin {
    /// What the far end calls this request, which is what Cancel names.
    pub request: String,

    /// The address that was typed, which is the one place this device has been
    /// told to look for the other.
    pub address: String,

    /// And what the device at that address said it is shown under, which is what
    /// the row reads *waiting for confirmation on*.
    pub name: String,

    /// Whether the ten minutes have run out.
    ///
    /// **This device's own reading of the far end's word for the moment**, made
    /// as the pane is answered. Which is why it is a flag here rather than the
    /// moment itself: the page would otherwise be holding a clock, and the row
    /// says one of two things.
    ///
    /// An expired row is drawn as expired and dismissed rather than taken away
    /// on this device's say-so — somebody pressed Add and is owed the answer
    /// that nobody pressed anything back.
    pub expired: bool,

    /// And whether the far end came back and said no.
    ///
    /// **The one thing on this row that was told to this device rather than
    /// read off it**, which is why it sits beside the flag above rather than
    /// being folded into it: an expiry is a clock and a refusal is a press, and
    /// a row that said only *this did not happen* would leave the human unable
    /// to tell a machine nobody was at from a human who said no.
    ///
    /// It outranks the expiry when the page draws the row, because it is the
    /// thing that really happened: a request refused three minutes in is a
    /// refusal for as long as it is drawn, whatever the clock goes on to say.
    /// And it ends at the same press — Dismiss, which is Cancel under another
    /// word.
    pub refused: bool,
}
