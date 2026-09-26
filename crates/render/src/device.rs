//! What a device says it is when another one asks: the answer the identity
//! endpoint on the peer listener gives (ADR-0020).
//!
//! **The one thing on that listener nobody has to be a member to read.** A
//! device that has never been heard of dials the peer port, completes a
//! handshake without showing a certificate at all, and reads this — which is
//! what makes linking possible in the first place: the human types an address,
//! and what comes back is the device they are about to link, named and
//! fingerprinted, before anything has been agreed between the two machines.
//!
//! So it carries nothing that is anybody's work. The id is what a record and a
//! URL name this device by, and the fingerprint is the certificate that same
//! handshake just handed over, spelled to be read aloud — those two are the
//! whole of an identity, and neither of them is a secret.
//!
//! **And three things read off the machine rather than kept anywhere**: the
//! name it is shown under, the word for its operating system, and every address
//! a peer could reach it on. They are read at the moment they are answered —
//! a laptop moves between the LAN and the tailnet, and DHCP moves everybody —
//! and none of them is configured: the name is the hostname, the OS is the
//! platform's own word with *Linux (WSL)* for the one case a hostname cannot
//! tell apart, and the addresses are read off the machine's interfaces and its
//! Tailscale.
//!
//! A wire type in this crate like every other, and exported to TypeScript with
//! them: the Devices section of the Remote access pane draws this device's own
//! answer, so the browser reads the same shape a peer does.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::joining::PendingJoin;

/// One device, as it answers for itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DeviceIdentity {
    /// The Device Id: sixteen random bytes as lower-case hex, invented at this
    /// device's first start and never changing.
    ///
    /// Spelled `device` rather than `id` because that is what the startup line
    /// calls it and what a URL segment will be — an `id` on a payload that is
    /// about a device could be a Conversation's or a Repo's.
    pub device: String,

    /// And the fingerprint of the certificate the handshake that carried this
    /// answer presented: the SHA-256 of its own bytes, upper-case hex in
    /// colon-separated pairs.
    ///
    /// Answered rather than left to be worked out from the connection, because
    /// the two are checked against each other: a caller compares what it read
    /// here with what the handshake handed it, and a device that named a
    /// certificate other than the one it presented is not the device it says it
    /// is. And it is what the human compares by eye — one person reading off a
    /// phone while another reads off a screen — which is why it is spelled the
    /// way every other tool spells one.
    pub fingerprint: String,

    /// What this device is *shown* under: the hostname of the machine it is on,
    /// read at the moment this is answered.
    ///
    /// Nothing is configured and nothing is typed. A name somebody could set
    /// would be a name two devices could be given, and what a device is *named
    /// by* is the id above — which was invented precisely because neither the
    /// hostname nor the tailnet node name can be relied on to be anybody's
    /// alone.
    ///
    /// A machine that will not say what it is called reads *this machine*,
    /// which is what the one other sentence naming this box already falls back
    /// to.
    pub name: String,

    /// And the word for the operating system it is running, which is what
    /// draws the icon beside the name.
    ///
    /// **A WSL reads *Linux (WSL)*, and that is the whole reason this is
    /// here.** A Windows machine and the WSL on it share a hostname, so the
    /// name cannot tell the two apart and this is the only thing that can —
    /// which is the setup cluster mode was written for. Everywhere else it is
    /// the platform's own spelling of itself: *Linux*, *macOS*, *Windows*.
    pub os: String,

    /// And every address a peer could reach this device on, in the order one
    /// should try them: the tailnet name and its addresses first where
    /// Tailscale is up, then the LAN addresses.
    ///
    /// **All of them, read at each answer rather than configured.** A laptop
    /// moves between the LAN and the tailnet and DHCP moves everybody, so the
    /// address somebody typed to link two devices is only the first one ever
    /// known — this list is what keeps a device that moved reachable.
    ///
    /// Empty is a device on neither a tailnet nor a network, which is an answer
    /// rather than a failure: it still has an id and a fingerprint, and those
    /// are what somebody looking at this is comparing.
    pub addresses: Vec<String>,
}

/// What a device tells each of its members when it has made its certificate
/// again (ADR-0020, *The certificate is renewed before it runs out*).
///
/// **Not a viewer type.** This one crosses the peer listener rather than the
/// workbench — it is one Verkstead telling another what it is becoming, and no
/// browser ever reads it.
///
/// **The identity whole, and the incoming fingerprint beside it.** A renewal is
/// the one call in a cluster where a device has two certificates at once, and
/// the two have to be told apart by the machine reading this or the reading is
/// worthless. So [`DeviceIdentity`] keeps the meaning it has everywhere else —
/// the fingerprint in it is the certificate this call was *made* under, which
/// the receiver checks against what its own handshake handed over, exactly as it
/// would on any other exchange — and the one being changed to is a field of its
/// own. A single fingerprint field that meant something different here would be
/// a payload the receiver had to know which call it had arrived on to read.
///
/// **And the addresses ride along**, because every device advertises all of them
/// on every exchange and this is one: a laptop that moved and renewed is reached
/// at its new addresses on the next call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewedCertificate {
    /// What this device is, as it answers anybody — with the certificate it is
    /// still *presenting* named in it, which over a changeover is the outgoing
    /// one.
    pub identity: DeviceIdentity,

    /// And the fingerprint of the certificate it is changing over to: what the
    /// receiver records against the same Device Id, and what it answers to say
    /// that it holds.
    pub incoming: String,
}

/// The **Devices** list, as the Remote access pane reads it off this machine.
///
/// The other side of [`DeviceIdentity`]: that one is what this device tells a
/// *peer* over the peer listener, and this is what it tells the browser over
/// the workbench. The same device described twice rather than two descriptions
/// of it — the identity is carried whole inside this, so the row the pane draws
/// and the answer a stranger reads cannot come to disagree about a name, an OS
/// or an address.
///
/// **Read off the machine rather than out of the settings.** Nothing here is
/// configured, which is why the pane's Devices section reads this rather than
/// the settings query the rest of the page shares — the two sections beside it
/// are read the same way and for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DevicesView {
    /// This device, which is the row marked *this device* and the one the list
    /// always holds: a Verkstead linked to nothing still has an identity.
    pub this: DeviceIdentity,

    /// And every other device in its cluster, each as it last answered for
    /// itself — with whether it is still answering beside it.
    ///
    /// The identity is the same shape as the row above, because it is the same
    /// thing said: a member is drawn with its name, the mark for its OS and the
    /// addresses a peer could reach it on, exactly as this device is. What is
    /// different is where the answer came from — this device reads its own
    /// machine as the pane is drawn, and a member was read off the far end at
    /// the last exchange.
    ///
    /// **The count on the Remote access card comes off this**, rather than
    /// being answered beside it: there is one membership, and a number that
    /// could disagree with the rows would be two answers about it. An
    /// unreachable member counts like any other — it is linked, and a count
    /// that left it out would say the cluster had shrunk.
    pub members: Vec<LinkedDevice>,

    /// And every join this device has asked for and not yet been answered on —
    /// see [`PendingJoin`].
    ///
    /// **Beside the members rather than among them**, because a pending join is
    /// not a device: nothing has been agreed, and a row that sat in the list
    /// looking like a member would be a cluster this device had joined itself
    /// to. The count on the card is the members' alone for the same reason.
    ///
    /// Read at the moment the pane asks, the way the members are: a request
    /// whose ten minutes ran out a second ago reads expired on this answer and
    /// waiting on the one before it, and both are true when they are given.
    pub pending: Vec<PendingJoin>,
}

/// One device linked to this one: what it says it is, and whether the last dial
/// to it got through.
///
/// **Two things rather than one, because only one of them is the far end's.**
/// The identity is what that machine said about itself at the last exchange;
/// whether it is answering is this device's own finding, written by a dial that
/// worked down its addresses and reached none of them. So it sits beside the
/// identity rather than inside it — a device does not tell anybody it is
/// unreachable, and the row a peer reads off [`DeviceIdentity`] is the same
/// whichever machine is asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct LinkedDevice {
    /// The device, as it last answered for itself.
    pub identity: DeviceIdentity,

    /// And whether the last dial to it got through. False is the row drawn
    /// dimmed, reading *unreachable* — it stays on the list, with everything
    /// about it, and an Unlink on it still works.
    pub reachable: bool,
}

/// One device nobody has typed an address for, as the **Discovered** list under
/// that same section draws it (ADR-0020, *Discovery*).
///
/// **Not a [`DeviceIdentity`], and that is the difference between the two
/// lists.** An identity is what a device *answered*, over a handshake, with the
/// fingerprint of the certificate it presented in it. Nothing here has been
/// asked anything: this is drawn off an advertisement on the LAN, which says
/// where a device is and proves nothing about it — so there is no fingerprint on
/// this row, and the string two people compare by eye is on the pending row the
/// press leaves rather than on this one.
///
/// **Three kinds of device are not in this list**: a **Member**, which is in the
/// cluster already; this device, which hears its own advertisement; and a device
/// a **Join** is already pending for, whose pending row is the answer to the
/// press somebody made. Where they are left out is the server's `discovery`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DiscoveredDevice {
    /// The Device Id off the advertisement, which is what the row is keyed by
    /// and what an **Add** on it names.
    ///
    /// Keyed by it rather than by the address, because that is the one thing
    /// about a device that is nobody else's: two Verksteads on one machine
    /// answer to one hostname on one address, and a list keyed on where
    /// something was found would draw the two of them as one row.
    pub device: String,

    /// The name it is shown under: the hostname it advertised.
    pub name: String,

    /// And the word for its operating system, which draws the mark beside the
    /// name — *Linux (WSL)* being the one thing that tells a Windows machine
    /// from the WSL on it.
    pub os: String,

    /// Where it was found, the port it advertised and all, in the order to try
    /// them.
    ///
    /// **With the port on every one of them**, unlike the addresses a member
    /// advertises: what is known here is an advertisement rather than a device's
    /// own account of itself, and the port in it is the port that device's
    /// listener really bound. It is also the only thing that tells two
    /// Verksteads on one machine apart on the page, both of them answering to
    /// one hostname at one address.
    pub addresses: Vec<String>,

    /// And how this device came to hear of it.
    pub found: Vec<FoundOn>,
}

/// Where a discovered device was found.
///
/// **A list of these on the row rather than one**, because the two sources are
/// merged by Device Id and a device that is on one LAN *and* one tailnet is
/// found twice: the row says every way this device was heard of rather than
/// whichever way was heard of first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum FoundOn {
    /// Heard advertising `_verkstead._tcp.local` on a network this machine is
    /// on, which is what the row reads *LAN* for.
    Lan,
}
