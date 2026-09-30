//! Turning notifications on for one device: the key a browser subscribes
//! against, the subscription it hands back, and what the server made of it.
//!
//! Per device, and never anything the viewer remembers: what the switch says is
//! read out of the browser on every load, because an installed app reopened a
//! week later must not offer to enable what is already enabled.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// The public half of the server's VAPID keypair, base64url-encoded from the
/// uncompressed point — what `PushManager.subscribe` takes as its
/// `applicationServerKey`.
///
/// The private half stays on the server: this is only how a browser names the
/// server it is subscribing to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct PushKey {
    pub key: String,
}

/// A subscription as `PushManager.subscribe` describes one, flattened to the
/// three things a push needs: where to send it, and the two keys it is
/// encrypted for.
///
/// Flattened rather than passed through as the browser's own JSON, because the
/// nesting it uses — `keys.p256dh`, `keys.auth` — is the browser's shape and not
/// something the server has any reason to learn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// What became of a device asking to be notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Subscribed {
    /// This device will be told about a Set from now on. It is stored once
    /// however many times it subscribes.
    Stored,

    /// Refused: the browser handed over a subscription with no endpoint, or
    /// missing a key. Nothing could ever be sent to it, so nothing was stored.
    Incomplete,
}

/// A device asking not to be told any more, named by its endpoint — which is the
/// only name a subscription has.
///
/// An endpoint the server never stored is not an error: a browser can drop its
/// own subscription without the server having heard of it, and afterwards what
/// was asked for holds either way — nothing is sent there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Unsubscribe {
    pub endpoint: String,
}

/// What a device tells each of its members when it has something worth a phone
/// (ADR-0020, *The opened device relays*).
///
/// **Not a viewer type.** This one crosses the peer listener rather than the
/// workbench, like [`crate::RenewedCertificate`] beside it: it is one Verkstead
/// telling another what has just happened to a piece of its work, so that a
/// phone installed from one device hears from the whole cluster and a lock
/// screen says which machine the work was on. No browser ever reads it.
///
/// **What travels is what the far end needs to title it and route a tap**, and
/// nothing more. The sentence the sender would have shown, the repository that
/// stands under it, and which Conversation it is about — the *device* is not on
/// it, because the certificate the caller presented at the handshake is what
/// says which device this is, and a member passing on what a third one told it
/// would be saying that news was its own.
///
/// **Prose rather than a variant to match on.** The kind of news is decided
/// where it happens and written into `said` there — see `News::title` in
/// `crates/server/src/push.rs`, where every title in the cluster is written. So
/// a news kind a newer member has and this one has not still reads: what arrives
/// is a sentence, and the receiver has only to put a device in front of it.
///
/// **And the path is not on it either.** The receiver composes one onto its own
/// device segment from `conversation`, because a path taken verbatim would open
/// the receiver's Conversation of the same number — ids being each device's own,
/// which is the collision a cluster's URLs are addressed by device for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayedNews {
    /// Which Conversation of the sender's it is about, by that device's own id
    /// for it — the number the receiver writes into
    /// `/devices/{device}/conversations/{id}` and nothing it looks up.
    pub conversation: i64,

    /// The sentence the sender would have shown on its own lock screen, with no
    /// device in front of it: the receiver is what puts one there.
    pub said: String,

    /// And the repository the work is in, where the sender knows it — which is
    /// the one thing that tells two notifications apart at a glance, and stands
    /// under the title here exactly as it does under a local one.
    pub project: Option<String>,
}
