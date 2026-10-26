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

    /// And the Device Id of its **Keeper**, where it is an **Annex** — the
    /// mark saying whose it is (ADR-0024, *The Keeper's Join*).
    ///
    /// **Said by the Annex about itself** rather than asserted by anybody about
    /// it: an Annex is told in its own `config.yaml` that it is kept, and every
    /// answer it gives says so. A member row takes it when it is first written
    /// and keeps it through every refresh after, so a device linked as an
    /// ordinary member cannot later say itself out of being unlinked.
    ///
    /// `None` is every device but an Annex, and what a Verkstead from before
    /// the Annex existed answers by saying nothing.
    #[serde(default)]
    pub kept_by: Option<String>,

    /// And the release of Verkstead it is running, *0.2.2*: read off the
    /// binary answering rather than the machine, and drawn under the name on
    /// its Devices row (ADR-0027, *Seeing versions*).
    ///
    /// **A reading like the name, the OS and the addresses**, so a member's
    /// row takes whatever the last answer said, `None` included: a device
    /// moved back to an older Verkstead really does say none.
    ///
    /// `None` is what a Verkstead from before this field answers by saying
    /// nothing — which is itself the sign that it wants updating.
    #[serde(default)]
    pub version: Option<String>,
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

    /// And why this device's **Annex** would not come up at this start, where
    /// there is no row of the Annex's in `members` to carry it — an Annex that
    /// has never been linked. Drawn under this device's own row. Where the
    /// Annex has a row, the line is on it instead — see
    /// [`LinkedDevice::trouble`] — and this is `None`.
    #[serde(default)]
    pub annex_trouble: Option<String>,

    /// And the newer stable release than the one this device runs, where one
    /// exists, *0.3.0*: what tags its row *update available* — see
    /// [`LinkedDevice::update_available`], which is the same verdict for a
    /// member — and the version its Update's confirm card names.
    #[serde(default)]
    pub this_update_available: Option<String>,

    /// And the branch each session running on this device is running on,
    /// where its Conversation is called by its branch — which its Update's
    /// confirm card names as the sessions to be stopped and resumed
    /// (ADR-0027). Read at the moment the pane asks, as the sidebar's marks
    /// are; empty with none running.
    #[serde(default)]
    pub this_running: Vec<String>,

    /// And how many more are running on a Draft, whose branch is nobody's
    /// name for it yet — counted rather than named, so that the card reads
    /// *2 drafts* rather than *a draft and a draft*.
    #[serde(default)]
    pub this_running_drafts: u32,

    /// And what this device was installed as, which says whether its row is
    /// offered an Update or a line saying how to update it by hand — see
    /// [`InstallKind`].
    ///
    /// **This device's alone**, beside `this` rather than inside it: the
    /// identity is what a member reads off the peer listener too, and what a
    /// member tells of its own install is a later stage's to decide.
    #[serde(default)]
    pub this_install: InstallKind,

    /// And this device's own Update, while one is going: from the press until
    /// the server serving this answer stops, which is the app relaunching it on
    /// the new release — see [`ThisUpdating`]. `None` otherwise, a start
    /// included, so the first answer after the relaunch reads none.
    #[serde(default)]
    pub this_updating: Option<ThisUpdating>,

    /// And why this device's last Update didn't take it onto the release, as
    /// the desktop app reported it — a download or check that failed, or a
    /// relaunch that came up on another version than the one handed over to.
    /// Drawn under this device's own row, beside any line of the Annex's, until
    /// the next press or the next start. There is no rollback (ADR-0027): the
    /// line, and a higher release.
    #[serde(default)]
    pub this_update_trouble: Option<String>,
}

/// This device's Update, as far as the desktop app has said it has got
/// (ADR-0027, *A device updates itself*): the row reads *updating…* from the
/// press, with how much of the release is down once the app has said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct ThisUpdating {
    /// How much of the release electron-updater has downloaded, as a whole
    /// percentage — `None` until the app's first report.
    pub percent: Option<u8>,
}

/// What a Verkstead was installed as, read off its own executable at startup
/// (ADR-0027, *A device updates itself*).
///
/// **Three of them update themselves and three are updated by hand.** The
/// desktop app's three artifacts are what electron-updater can replace; what
/// nix, a checkout or a downloaded binary put there is the human's to replace
/// the way they put it there, and nothing is ever attempted on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum InstallKind {
    /// The desktop app out of `Verkstead-x86_64.AppImage`: its sidecar, run
    /// from inside the image's mount.
    AppImage,

    /// The desktop app out of the dmg: its sidecar, run from inside the
    /// bundle's `Contents/Resources/cli`.
    MacApp,

    /// The desktop app out of `Verkstead-x86_64.exe`: its sidecar, run from
    /// the per-user directory the installer puts it in.
    WindowsInstall,

    /// Anything run out of the nix store, which on a host is the NixOS module.
    Nix,

    /// A build under a checkout's `target/`, which is what `pnpm start` and
    /// `cargo run` both run.
    Checkout,

    /// Anything else, which is a downloaded `verkstead` started with `serve`.
    #[default]
    Binary,
}

impl InstallKind {
    /// Whether electron-updater can take this install onto a newer release.
    pub fn updates_itself(self) -> bool {
        matches!(
            self,
            InstallKind::AppImage | InstallKind::MacApp | InstallKind::WindowsInstall
        )
    }
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

    /// And, on a Keeper's row for its own **Annex**, why the box it runs in
    /// would not come up at this start: it would not boot, answer or link.
    /// Drawn under the row, which reads *unreachable* beside it until the
    /// Annex answers (ADR-0024).
    ///
    /// **And the background reads of this member that did not land** —
    /// its Agent Profiles, its Authors, its Conversations — after the Annex's
    /// line where there is both, and then the gap between its release and this
    /// device's where they differ: *winvm's Agent Profiles couldn't be read.
    /// winvm is on 0.2.0 and this device is on 0.2.2. Update winvm.* Held until
    /// the next read of that kind lands.
    ///
    /// This device's own findings, like the flag above: `None` on a row with
    /// none of either.
    #[serde(default)]
    pub trouble: Option<String>,

    /// And whether the version it last said is older than the latest stable
    /// release, which tags the row *update available* (ADR-0027, *Seeing
    /// versions*).
    ///
    /// **This device's finding, out of the Update Notice's own daily check**,
    /// so it is false while that check is off or unanswered, and on a row that
    /// says no version. Worked out by the server, so the viewer does no version
    /// arithmetic.
    #[serde(default)]
    pub update_available: bool,
}

/// One device nobody has typed an address for, as the **Discovered** list under
/// that same section draws it (ADR-0020, *Discovery*).
///
/// **Not a [`DeviceIdentity`], and that is the difference between the two
/// lists.** An identity is what a member *answered* over a handshake this device
/// had pinned, with the fingerprint of the certificate it presented in it.
/// Nothing on this row has been proved like that: it is drawn off an
/// advertisement on the LAN, which says where a device is and nothing about what
/// it is, or off a stranger's answer taken under whatever certificate it
/// happened to show — so there is no fingerprint here, and the string two people
/// compare by eye is on the pending row the press leaves rather than on this
/// one.
///
/// **Three kinds of device are not in this list**: a **Member**, which is in the
/// cluster already; this device, which hears its own advertisement; and a device
/// a **Join** is already pending for, whose pending row is the answer to the
/// press somebody made. Where they are left out is the server's `discovery`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct DiscoveredDevice {
    /// The Device Id this one was heard under, which is what the row is keyed by
    /// and what an **Add** on it names.
    ///
    /// Keyed by it rather than by the address, because that is the one thing
    /// about a device that is nobody else's: two Verksteads on one machine
    /// answer to one hostname on one address, and a list keyed on where
    /// something was found would draw the two of them as one row.
    pub device: String,

    /// The name it is shown under: the hostname it advertised, or the one it
    /// answered with.
    pub name: String,

    /// And the word for its operating system, which draws the mark beside the
    /// name — *Linux (WSL)* being the one thing that tells a Windows machine
    /// from the WSL on it.
    pub os: String,

    /// Where it was found, the port and all, in the order to try them.
    ///
    /// **With the port on every one of them**, unlike the addresses a member
    /// advertises: these are places this device has *heard* something rather
    /// than a device's own account of where it is, so a row drawn from them has
    /// to be one an **Add** can dial as it is written. The port on a LAN address
    /// is the one that device's listener really bound; the port on a tailnet
    /// address is the peer port assumed, a peer list naming none. It is also the
    /// only thing that tells two Verksteads on one machine apart on the page,
    /// both of them answering to one hostname at one address.
    ///
    /// **The LAN ones first where both halves found it**, that being the shorter
    /// road: two machines on one network reach each other without a tailnet in
    /// the middle.
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

    /// And answered for itself when this device asked a node of its tailnet what
    /// it was, which is what the row reads *Tailscale* for.
    ///
    /// **Asked rather than heard**, which is the difference between the two
    /// halves. A tailnet carries no multicast for an advertisement to go out
    /// over, so what finds a device there is this one working down the peers
    /// `tailscale status` names and asking each of them on the peer port. A node
    /// that is not a Verkstead answers nothing this can read, and is no row at
    /// all.
    Tailscale,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Verkstead from before the version answers without the field at all,
    /// and that reads as having said none rather than as an answer refused.
    #[test]
    fn an_identity_without_a_version_says_none() {
        let older: DeviceIdentity = serde_json::from_str(
            r#"{"device":"aa","fingerprint":"3F:0A","name":"winvm","os":"Windows","addresses":[]}"#,
        )
        .expect("an older Verkstead's answer still reads");

        assert_eq!(older.version, None);
        assert_eq!(older.kept_by, None);
    }

    /// And one that says it is read back as it was said.
    #[test]
    fn an_identity_with_a_version_says_it() {
        let newer: DeviceIdentity = serde_json::from_str(
            r#"{"device":"aa","fingerprint":"3F:0A","name":"wolf","os":"Linux","addresses":[],"version":"0.2.2"}"#,
        )
        .unwrap();

        assert_eq!(newer.version.as_deref(), Some("0.2.2"));
    }
}
