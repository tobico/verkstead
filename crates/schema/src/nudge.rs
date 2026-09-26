//! The Nudge: what the server tells an open viewer page has moved.
//!
//! Notify-only (ADR-0009). A Nudge names a kind and, where the change belongs to
//! one, the Conversation it happened in — never what changed. Data rides no
//! event: HTTP stays the single path for content, rendering and types, and a
//! Nudge is the word that a fetch is worth making.
//!
//! The kinds are the server's vocabulary for what moved, not the viewer's cache
//! layout. Which queries a kind stands for is decided on the other side of the
//! wire, in `nudge.ts`, so renaming a query key here is not a server change.
//!
//! And whose news it is rides beside the kind rather than in it — see [`Nudged`],
//! which is what goes down a stream: the device the browser opened says a
//! member's news is that member's, and its own is the frame it always was.

use serde::{Deserialize, Serialize};

// Read by the viewer off the stream, so its TypeScript comes from here — see
// `Response` for why the emitter is gated.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One thing that moved, said as briefly as it can be said.
///
/// A page too old to know a kind treats it as everything having moved, which is
/// the behaviour every kind used to get — so a new kind is safe to add and an
/// old page stays correct against a newer server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Nudge {
    /// A session put lines on a Conversation's Transcript.
    Transcript { conversation: i64 },

    /// A session printed, so what its terminal shows has moved — the Capture
    /// and the Screen painted from it alike.
    Screen { conversation: i64 },

    /// A commit landed on a Conversation's branch.
    Commit { conversation: i64 },

    /// The Worktrees of a Conversation moved: something wrote, made, or took a
    /// file away.
    ///
    /// The one kind that comes off the disk rather than off a record — a
    /// watcher the server runs while a Code pane is attached to the
    /// Conversation and not otherwise (ADR-0019, *Following the disk*). An
    /// agent writing, a build filling a folder, a terminal tab checking out a
    /// branch: each of them moves the Worktree under a pane that is drawing it.
    ///
    /// What it does not say is *what* moved, and deliberately: the page re-reads
    /// the folders it has expanded and the files it has open, and a re-read is
    /// cheap because a folder is one listing and a version is one hash.
    Files { conversation: i64 },

    /// A Question Set in a Conversation arrived, was answered, or was closed
    /// unanswered.
    Set { conversation: i64 },

    /// An agent took up or let go of the wait on a Question Set, so the badge
    /// saying whether anybody is listening has moved.
    ///
    /// Nothing durable changed and nothing is asked of the human: this is the
    /// one kind that is purely about a display. It exists because the verdict
    /// used to cycle with the viewer's poll, and the poll is gone (ADR-0009).
    Liveness { conversation: i64 },

    /// One Conversation moved in some other way: an Event on its Timeline, or
    /// its lifecycle.
    ///
    /// This is the Conversation everywhere it is drawn, its row in the sidebar
    /// included — a lifecycle that moved is a row that reads differently.
    Conversation { conversation: i64 },

    /// The list itself moved: a Conversation appeared or left, or the human
    /// dragged the sidebar into a different order. Which is a different thing
    /// from one of them moving, and the reason both kinds exist.
    Conversations,

    /// Something about the Repos moved, roadmaps nothing is driving included.
    Repos,

    /// The joins in flight moved: one was asked of this device, or one it was
    /// holding was settled or ran out.
    ///
    /// **It says which kind of thing moved and not which request**, as every
    /// other kind here does: the page reads the pending joins back, which is a
    /// handful of rows. A kind that carried the request id would be a second
    /// account of what that read already says, and a page drawing a modal out
    /// of an event rather than out of the server.
    Joins,

    /// The cluster moved without anybody here pressing anything: a member of it
    /// named a device this one had not heard of, and it is a member now.
    ///
    /// A kind of its own rather than [`Nudge::Joins`], because there is no join
    /// on this device to have moved — the press was on some other machine, and
    /// what arrives here is the announcement that followed it. What it names is
    /// the Devices section of the Remote access pane, which is where a row
    /// appears with nothing beside it to explain itself.
    Devices,

    /// The **Discovered** list moved: a device nobody has typed an address for
    /// was heard advertising itself, or one that had been heard stopped.
    ///
    /// **A kind of its own rather than [`Nudge::Devices`]**, because the two
    /// name two readings: nothing about this cluster has changed, and the rows
    /// the pane already drew of it are not to be re-read for it. What has
    /// changed is what is *out there* — which is the whole reason a browse can
    /// be held while somebody looks without the membership being read again
    /// every time the LAN says something.
    ///
    /// **And it is what makes the list arrive at all.** A browse is cold when it
    /// starts, so the first read of that list is empty or short and every row
    /// after it lands here: an open pane draws a device appearing without a
    /// reload and without a poll, which is what ADR-0009 put every other list in
    /// this viewer on.
    Discovered,

    /// The Agent Profiles moved.
    ///
    /// Nothing announces this yet: a Profile is only ever saved or deleted by a
    /// browser, which reads its own change back. It is here because the
    /// vocabulary is the taxonomy of ADR-0009 rather than a list of today's
    /// callers, and a second device watching is what it is waiting for.
    Profiles,

    /// Everything of one device's, which is the widest thing there is to say:
    /// read back whatever of it is on screen.
    ///
    /// **What a Nudge stream that has just been taken up says.** The hub holds
    /// one to each of its members, and a stream that has come back knows nothing
    /// about what it missed — so what it announces under that device is *look at
    /// all of it*, which is the reaction the browser's own stream makes of a
    /// reconnect, aimed at one device's queries (ADR-0020, *The opened device
    /// relays*). See [`Nudged`], which is what carries the device.
    ///
    /// **A kind of its own rather than one of every other kind said at once**,
    /// because the kinds say what moved rather than what to read: a member whose
    /// news was missed did not move its Discovered list, and a stream that said
    /// so would be inventing news to buy an invalidation with.
    ///
    /// **And rather than nothing at all**, which is what an unrecognised kind
    /// comes to: a page that read *everything* because one member came back
    /// would throw away what it holds of this device and of every other member
    /// with it, having missed nothing of either.
    Everything,
}

/// One Nudge as it goes down a stream: what moved, and **whose news it is**.
///
/// A page reaches a member's Conversation through the device it opened
/// (ADR-0020, *The opened device relays*), so the news of one has to arrive on
/// that device's stream too: the hub holds a Nudge stream to each of its members
/// and re-announces what comes down one under the Device Id it came from — see
/// `relaying::freshness`. Which device a Nudge is about is what the viewer's
/// table keys its invalidation by, ids being each device's own and colliding by
/// construction.
///
/// **A local Nudge is the JSON it always was.** The device is flattened over
/// [`Nudge`] and left out when there is none, so what an open page has been
/// reading since ADR-0009 goes down the wire byte for byte — a kind, and a
/// Conversation where the change belongs to one — and a member's carries one
/// field more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Nudged {
    /// Which device the news is about: a member of this one's cluster, by its
    /// Device Id — absent for this device's own, which is every Nudge a
    /// workbench has ever sent about its own work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,

    /// And what moved, which is the whole of what a Nudge says.
    #[serde(flatten)]
    pub moved: Nudge,
}

impl Nudged {
    /// This device's own news, which is what every caller inside a workbench
    /// announces.
    pub fn here(moved: Nudge) -> Nudged {
        Nudged {
            device: None,
            moved,
        }
    }

    /// And a member's, re-announced under the device it was heard from.
    pub fn of(device: &str, moved: Nudge) -> Nudged {
        Nudged {
            device: Some(device.to_owned()),
            moved,
        }
    }
}
