//! The **slice** of the record one Conversation is: everything the store holds
//! about it, read out here and written down on the device the work is moving to
//! (ADR-0020, *Transfer*).
//!
//! **What travels is this Conversation and nothing about the machine it is on.**
//! The Timeline Events themselves, the Question Sets asked from them with their
//! Responses and deferrals and deliveries and endings, the Captures, the
//! Transcripts, the session names and Pairings and endings, the Steers, the
//! stops and escalations, the commits recorded, the pull request and wrap-up
//! bookkeeping, the unseen mark, the shares, the Companion rows and the
//! Attachments' rows. What does not is the device's own: its Repos, its Agent
//! Profiles, what it remembers of a Repo's Pairings, its Members, its joins, its
//! push subscriptions and its Remote access banner — none of which is a fact
//! about a piece of work, and every one of which the far end has its own. And
//! nor do the Worktrees, which are directories on one machine.
//!
//! **A table at a time, by a manifest rather than by a query apiece.** Verkstead
//! keeps a Conversation in two dozen tables and declares them a module at a
//! time, so the thing most likely to go wrong here is not a read being wrong
//! today but a table added next year that nobody carries. So the walk is a list
//! — see [`CARRIED`] — held against the schema by a test: every table a delete
//! empties either travels or is named in [`STAYS_BEHIND`], with the reason it
//! does beside it. That is a test which fails when the walk falls behind, rather
//! than one that agrees with a list because both were copied from the same
//! place.
//!
//! **Every id is renumbered as it lands.** Both ends issue their own and every
//! Verkstead has a Conversation 1, so an Event id referenced by a Capture, a
//! Transcript, a session Pairing or a Set has to come out pointing at the Event
//! it actually landed as — and a Set's own id moves with it, which is what
//! leaves one left open answerable on the far end afterwards. The manifest says
//! what each column holds and the landing does the arithmetic; nothing here
//! guesses from a column's name.
//!
//! **The two ids that are not the store's to renumber** are the Repo and the
//! Agent Profile, because neither is this Conversation's: a Repo is one
//! directory on one machine and an account is bound per device, so both are
//! settled outside — the Repos by the match the whole cluster runs on and the
//! Profiles by their uuids — and arrive here as a
//! [`Renaming`].
//!
//! **And a slice is bounded.** A Transcript is megabytes of a session talking,
//! so one is refused whole rather than carried part way — half a record is worse
//! than a move that did not happen — and the refusal says which Conversation was
//! too big to move. See [`MOST_A_SLICE_IS`].

use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// The most one Conversation's record may weigh on the way across: **sixty-four
/// megabytes**.
///
/// The memory store's bound, for the memory store's reason — see
/// `server::mirroring::memory::MOST_A_STORE_IS`. What makes a slice large is the
/// same thing that makes a store large: a session talking for hours, kept
/// verbatim because keeping it is most of Verkstead's case for being trusted
/// with a record. Sixty-four megabytes is a Conversation nobody has finished
/// reading; past it the move is refused whole, naming the Conversation, rather
/// than carried in part.
pub const MOST_A_SLICE_IS: usize = 64 * 1024 * 1024;

/// What stands in a Repo column that names no repository.
///
/// Zero, and it means what `wrap_up`'s own `NO_PULL_REQUEST` means: a wrap-up's
/// review is settled about the Conversation rather than about any one pull
/// request, so the column holds an id nothing was ever registered under. There
/// is nothing there to match across, so it crosses as it stands.
const NO_REPOSITORY: i64 = 0;

/// One value of one row, in the four shapes SQLite stores.
///
/// **The record as it stands rather than as this build reads it**, which is the
/// rule every stored body in this crate is kept by — see [`super::Asked`]. A
/// slice is copied from one database into another, so what it carries is
/// whatever the column held: a Set body this build cannot deserialize crosses
/// exactly as it was written, and lands readable by whatever build can.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cell {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Cell {
    /// Whatever this weighs towards [`MOST_A_SLICE_IS`].
    ///
    /// The bytes it is made of, which for a Transcript line or a Capture chunk
    /// is the whole of what a slice weighs. A number weighs its own eight.
    fn weight(&self) -> usize {
        match self {
            Cell::Null => 0,
            Cell::Int(_) | Cell::Real(_) => 8,
            Cell::Text(text) => text.len(),
            Cell::Blob(bytes) => bytes.len(),
        }
    }

    /// The id in it, where it holds one.
    fn id(&self) -> Option<i64> {
        match self {
            Cell::Int(id) => Some(*id),
            _ => None,
        }
    }
}

/// One table's rows, as they cross.
///
/// The column names travel with them rather than being assumed, because the two
/// ends may be one version apart: a column this device has and the far end does
/// not is a skew worth refusing by name, and a column the far end has and this
/// device does not takes whatever default it was declared with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rows {
    pub table: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

/// And a whole Conversation's record: every carried table, in the order they
/// land in.
///
/// **In order**, because foreign keys are on at both ends: an Event before
/// whatever hangs off it, a Question Set before its Response, and the Captures
/// before the turn count that points at one. The order is [`CARRIED`]'s own, and
/// a landing walks it as it stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slice {
    pub tables: Vec<Rows>,

    /// The stop and the escalation, which are columns on the Conversation's own
    /// row rather than a table beside it — see [`Marks`].
    pub marks: Marks,
}

/// What a Conversation's own row says about how things are with it, as it
/// crosses.
///
/// The columns [`super::stops`] and [`super::escalations`] added to
/// `conversations` — a stop is how things are rather than something that
/// happened, and what happened is the Notice it points at. So the Notice is an
/// Event id, renumbered as it lands like every other.
///
/// **What does not cross is `stop_asked_at`**: a stop somebody pressed for and
/// nothing has acted on is a press against the run on *that* machine, and there
/// is no run on this one. It is spent by the time the work leaves, the move
/// having seen the session out before it went.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Marks {
    pub stopped_at: Option<String>,
    pub stopped_by: Option<String>,
    pub stopped_resets: Option<String>,

    /// The Timeline Event the stop wrote, by the id it has on the device the
    /// slice came off.
    pub stopped_notice: Option<i64>,

    /// And the one an escalation wrote, the same way.
    pub escalated_notice: Option<i64>,
}

/// What the two ids a slice cannot renumber for itself come out as on the far
/// end.
///
/// **Neither is this Conversation's.** A Repo is one directory on one machine
/// and the same repository is another on the next, so which of the far end's
/// Repos each of these rows means is the cluster's own match — see
/// `server::matching`. An Agent Profile is one record under one uuid on every
/// device, bound on some and run from a copy on the rest, so which row means it
/// there is that uuid read back — see `server::peer::transfers`.
///
/// Both are settled by the devices rather than by the store, and arrive here
/// already answered.
#[derive(Debug, Clone, Default)]
pub struct Renaming {
    /// Every Repo the slice names, as the far end numbers it.
    pub repos: HashMap<i64, i64>,

    /// And every Agent Profile it names, where the far end has one at all.
    ///
    /// A Profile missing here is one that device has never heard of, which is
    /// the ordinary end of an account deleted a year after the Steer that ran
    /// under it: the column lands empty rather than pointing at somebody else's
    /// account, and the rest of the row stands.
    pub profiles: HashMap<String, String>,
}

/// What one column of a carried row holds.
///
/// Anything not named here is a value rather than a reference, and crosses as it
/// stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holds {
    /// The row's own id, which the far end issues afresh — and remembers as
    /// whatever points at it.
    Mine(Remembered),

    /// The Conversation, which is the row this slice lands beside.
    Conversation,

    /// A Timeline Event of this Conversation's.
    Event,

    /// A Question Set asked from it.
    Set,

    /// One of the device's Repos.
    Repo,

    /// And one of its Agent Profiles.
    Profile,
}

/// What the far end has to remember an issued id as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Remembered {
    /// The Event everything of a session's hangs off.
    Event,

    /// The Question Set its Response and its deferral hang off.
    Set,

    /// Nothing points at it, so nothing has to be kept.
    Nothing,
}

/// How one table's rows are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    /// By the Conversation they name.
    ByConversation,

    /// By the Timeline Event they hang off, which is one of this
    /// Conversation's.
    ByEvent,

    /// By the Question Set they hang off, which was asked from one of those
    /// Events.
    OfASet,

    /// And the Sets themselves, by their own id.
    Sets,
}

impl Found {
    /// The `WHERE` clause that finds one Conversation's rows, with one `?` in
    /// it.
    fn clause(self) -> &'static str {
        match self {
            Found::ByConversation => "conversation_id = ?",

            Found::ByEvent => {
                "event_id IN (SELECT id FROM timeline_events WHERE conversation_id = ?)"
            }

            Found::OfASet => {
                "set_id IN (SELECT set_id FROM set_events
                            WHERE event_id IN
                              (SELECT id FROM timeline_events WHERE conversation_id = ?))"
            }

            Found::Sets => {
                "id IN (SELECT set_id FROM set_events
                        WHERE event_id IN
                          (SELECT id FROM timeline_events WHERE conversation_id = ?))"
            }
        }
    }
}

/// One table the record carries.
#[derive(Debug, Clone, Copy)]
struct Carried {
    table: &'static str,
    found: Found,

    /// The columns holding an id, and what each of them is an id of. Every other
    /// column is a value and crosses untouched.
    ids: &'static [(&'static str, Holds)],
}

/// Every table one Conversation's record is kept in, **in the order they land**.
///
/// Parent before child all the way down, because foreign keys are on: the Events
/// first, then the Sets and what hangs off them, then what hangs off the Events,
/// then the sidecars beside the Conversation itself. It is the walk
/// [`super::delete_conversation`] makes, read backwards and with the tables that
/// stay behind taken out — see [`STAYS_BEHIND`].
const CARRIED: &[Carried] = &[
    // The Timeline itself. Everything below either hangs off one of these or off
    // the Conversation, so it goes first and its ids are what the rest are
    // renumbered against.
    Carried {
        table: "timeline_events",
        found: Found::ByConversation,
        ids: &[
            ("id", Holds::Mine(Remembered::Event)),
            ("conversation_id", Holds::Conversation),
        ],
    },
    // The Question Sets asked from those Events, and the pairing that says which
    // Event each landed on.
    Carried {
        table: "question_sets",
        found: Found::Sets,
        ids: &[("id", Holds::Mine(Remembered::Set))],
    },
    Carried {
        table: "set_events",
        found: Found::ByEvent,
        ids: &[("set_id", Holds::Set), ("event_id", Holds::Event)],
    },
    // And how each of them was settled, or was not: the Response, the lock on
    // one closed unanswered, how it was asked, whether its Answer ever reached a
    // session, and whether that Answer said there was nothing else.
    Carried {
        table: "responses",
        found: Found::OfASet,
        ids: &[("set_id", Holds::Set)],
    },
    Carried {
        table: "archivings",
        found: Found::OfASet,
        ids: &[("set_id", Holds::Set)],
    },
    Carried {
        table: "deferrals",
        found: Found::OfASet,
        ids: &[("set_id", Holds::Set)],
    },
    Carried {
        table: "deliveries",
        found: Found::OfASet,
        ids: &[("set_id", Holds::Set)],
    },
    Carried {
        table: "endings",
        found: Found::OfASet,
        ids: &[("set_id", Holds::Set)],
    },
    // What each session printed, and the turn count that points at the Capture
    // rather than at the Event — so the Capture lands first.
    Carried {
        table: "captures",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "capture_chunks",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "capture_turns",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    // The record the session's own backend kept of itself, what Verkstead called
    // it, what it ran under and how it ended.
    Carried {
        table: "transcript_lines",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "session_names",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "session_pairings",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "session_agents",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "session_endings",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    // The summary a commit card is drawn from, and the companion rows a Steer
    // asked for — both keyed by the Event rather than by the Conversation.
    Carried {
        table: "commit_summaries",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    Carried {
        table: "steer_additions",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event), ("repo_id", Holds::Repo)],
    },
    Carried {
        table: "steer_upgrades",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event), ("repo_id", Holds::Repo)],
    },
    // And where a steer came from, which is the state it is to go back to: a
    // word about the work rather than about either machine.
    Carried {
        table: "steer_sources",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event)],
    },
    // And what its checkout was holding uncommitted, which travels because the
    // changes themselves do: the far end applies the patch onto the worktree it
    // cuts, so the paths this names are paths it really has.
    Carried {
        table: "steer_scratch",
        found: Found::ByEvent,
        ids: &[("event_id", Holds::Event), ("repo_id", Holds::Repo)],
    },
    // What the work committed and what it ended up on, both of which name an
    // Event and the Conversation.
    Carried {
        table: "commits",
        found: Found::ByConversation,
        ids: &[
            ("event_id", Holds::Event),
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "pull_requests",
        found: Found::ByConversation,
        ids: &[
            ("event_id", Holds::Event),
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    // The Pauses a Verkstead of before put on a Timeline. Nothing writes one any
    // more and those Events are still the record of what happened, so they cross
    // with the rest of the Timeline they are on.
    Carried {
        table: "pauses",
        found: Found::ByConversation,
        ids: &[
            ("event_id", Holds::Event),
            ("conversation_id", Holds::Conversation),
        ],
    },
    // And what each Steer settled beside its Event.
    Carried {
        table: "steers",
        found: Found::ByConversation,
        ids: &[
            ("event_id", Holds::Event),
            ("conversation_id", Holds::Conversation),
            ("profile_id", Holds::Profile),
        ],
    },
    // And who left each Memo, and out of which session — both of them Events of
    // this Timeline's, renumbered with the rest.
    Carried {
        table: "memos",
        found: Found::ByConversation,
        ids: &[
            ("event_id", Holds::Event),
            ("conversation_id", Holds::Conversation),
            ("left_in", Holds::Event),
        ],
    },
    // And the Previews suggested for trying the work, which name nothing but
    // the Conversation: how to try the branch is true wherever it is worked.
    Carried {
        table: "previews",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // What GitHub last said about the work, and what the wrap-up got through.
    Carried {
        table: "pull_request_checks",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "pull_request_merges",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "pull_request_standings",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "wrap_up_settled",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "check_fix_attempts",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "conflict_fix_attempts",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "addressed_comments",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "wrap_up_narrowings",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // The other repositories the work is done in. The rows are the record — which
    // repository, in which mode, off which ref, on which branch — and the
    // checkouts they are cut into are each device's own, which is why
    // `companion_worktrees` stays behind.
    Carried {
        table: "companions",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    // What was shared of it, and whether anybody commented on the pull requests.
    Carried {
        table: "shares",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "share_comments",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // Whether there is news on it nobody has looked at, what it was doing, and
    // how it was set up to do it.
    Carried {
        table: "unseen_conversations",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "directions",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // What kind of work it is, and what it is pointed at where its Process takes
    // a target. Both are the Conversation's own answer rather than anything
    // about the machine it was answered on, so both cross.
    Carried {
        table: "processes",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "targets",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "stage_branches",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "stage_roadmaps",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // And its place in the queue to join its roadmap's chain, where its tasks
    // have finished: a stage that crosses queued arrives queued, rather than
    // waiting for its finish to be launched again over there to take a place.
    // The place is an order rather than a fact, so the far end issues its own —
    // behind whatever is already waiting there, the way it would have been had
    // its tasks finished at the moment it landed. See the server's `joins`.
    Carried {
        table: "stage_joinings",
        found: Found::ByConversation,
        ids: &[
            ("place", Holds::Mine(Remembered::Nothing)),
            ("conversation_id", Holds::Conversation),
        ],
    },
    Carried {
        table: "adoptions",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "pull_request_adoptions",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // And the Steer somebody had started and not decided, which is beside the
    // Conversation rather than on its Timeline — so it crosses unfinished, and
    // the human finishes it wherever the work now is.
    Carried {
        table: "pending_steers",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("profile_id", Holds::Profile),
        ],
    },
    Carried {
        table: "pending_steer_additions",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    Carried {
        table: "pending_steer_upgrades",
        found: Found::ByConversation,
        ids: &[
            ("conversation_id", Holds::Conversation),
            ("repo_id", Holds::Repo),
        ],
    },
    // Whether the human had put it away, and whether a Cleanup has since taken
    // the bulk out of it — the second because a copy that arrived without it
    // would be a record explaining its own missing output as breakage.
    Carried {
        table: "archived_conversations",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    Carried {
        table: "trimmed_conversations",
        found: Found::ByConversation,
        ids: &[("conversation_id", Holds::Conversation)],
    },
    // And the MCP servers it was given, which travel as the names they are:
    // what a Conversation holds is a reference to a declaration, and the far
    // end reads its own settings for what the name refers to — a name nothing
    // is declared by over there is skipped at the launch, exactly as one taken
    // out of the settings here is.
    Carried {
        table: "conversation_mcp_servers",
        found: Found::ByConversation,
        ids: &[
            ("id", Holds::Mine(Remembered::Nothing)),
            ("conversation_id", Holds::Conversation),
        ],
    },
    // And the devices its agent may move it to, which travel as the Device Ids
    // they are: an id names the same machine in every database of a cluster,
    // so the far end reads the same list — and the drafting device, which is
    // no row, is still implicit there because the birth key crosses too.
    Carried {
        table: "permitted_devices",
        found: Found::ByConversation,
        ids: &[
            ("id", Holds::Mine(Remembered::Nothing)),
            ("conversation_id", Holds::Conversation),
        ],
    },
    // And the files the human put on it. The rows here and the bytes beside them:
    // the files land in the far end's own attachments directory, under its own
    // Conversation id, so a session there is given the paths its own sandbox
    // expects — see `server::attachments`.
    //
    // Last, because a file put on an Answer names the Set it was put on.
    Carried {
        table: "attachments",
        found: Found::ByConversation,
        ids: &[
            ("id", Holds::Mine(Remembered::Nothing)),
            ("conversation_id", Holds::Conversation),
            ("set_id", Holds::Set),
        ],
    },
    // And what was picked on a Set to add to the work, which names the Set as a
    // file on an Answer does and a Repo where it is a companion.
    Carried {
        table: "set_additions",
        found: Found::ByConversation,
        ids: &[
            ("id", Holds::Mine(Remembered::Nothing)),
            ("conversation_id", Holds::Conversation),
            ("set_id", Holds::Set),
            ("repo_id", Holds::Repo),
        ],
    },
];

/// The tables a delete empties that a move leaves where they are, each with what
/// makes it the device's rather than the work's.
///
/// Held against the schema beside [`CARRIED`] — see [`carried_tables`]. Naming
/// them is the point: a table nobody has decided about is the failure this pair
/// of lists exists to catch, and *stays behind* is a decision.
pub const STAYS_BEHIND: &[&str] = &[
    // The checkouts, which are directories on one machine: the far end cuts its
    // own, its own Repo being somewhere else entirely.
    "worktrees",
    "companion_worktrees",
    // The Pairings and the roles picked away, which cross on the Conversation's
    // own row as ids of the far end's — see [`super::arrivals`].
    "pairing_models",
    "skipped_roles",
    // The birth key, which is stamped on the arriving row out of the move
    // itself: it is the one thing about a Conversation that is the cluster's,
    // and the arrival is where it is written.
    "births",
    // And the two marks about where a copy of the work went, each of which
    // names a machine from the point of view of the device holding it: which
    // device has the live record, and a move somebody pressed for and nothing
    // has made yet.
    "transferred",
    "transfers",
    // And what this device last told a stage's home, which is this copy's
    // bookkeeping: the far end owes home its own report, starting from none.
    "stage_reports",
    // And which session a launch here carries on from, which is this device's own
    // reading of an arrival: it is written in the leg after the record lands, out
    // of what landed — see [`super::continuations`].
    "continued_sessions",
    // And what this landing renumbered each Question Set to, which is this
    // device's arithmetic rather than anything about the work: the far end issues
    // its own ids and writes its own map as the record lands over there.
    "landed_sets",
    // And when a stage that has landed here again joined its chain, which is an
    // Event id of this device's: the far end's sequence is no order here, and
    // keeps its own — see [`Held`].
    "stage_joined_at",
    // The two tables a Verkstead of before kept a stopped Conversation in.
    // Nothing writes either any more and the stop itself crosses on the row —
    // see [`Marks`].
    "halts",
    "stops_asked",
    // And the row itself, which the far end writes from the move rather than
    // from the record — see [`super::arrivals`]. What of it is the record and
    // not the row crosses beside the slice, which is the stop and the
    // escalation.
    "conversations",
];

/// Every table a slice carries, said as a value.
///
/// Public for [`super::deleted_tables`]'s reason and against that very list: the
/// test holds the two together, so a table added to the schema next year is a
/// failure here until somebody has decided whether a move takes it.
pub fn carried_tables() -> Vec<&'static str> {
    CARRIED.iter().map(|carried| carried.table).collect()
}

/// Read one Conversation's whole record out.
///
/// Every carried table in the order it lands, with the ids exactly as they are
/// in *this* database: renumbering happens where the slice is written, because
/// that is the end that issues the numbers.
///
/// **Refused whole past [`MOST_A_SLICE_IS`]**, naming the Conversation. Half a
/// record is a Timeline that stops in the middle and a Transcript that ends
/// mid-sentence, and neither is better than a move that did not happen.
///
/// **One transaction, and so one moment of the record.** The work can still be
/// writing while it is read — a session recording a commit, which is an Event
/// and the row hanging off it — and a table at a time read as separate moments
/// is a slice holding that commit without its Event, which the far end refuses
/// the whole move over. See [`super::reading`].
pub async fn slice(pool: &sqlx::SqlitePool, conversation_id: i64) -> Result<Slice> {
    let mut tx = super::reading(pool, "reading a Conversation's record to move it").await?;

    let mut tables = Vec::with_capacity(CARRIED.len());
    let mut held = 0usize;

    for carried in CARRIED {
        // In the order they were written, which is what keeps a Timeline in the
        // order it was read in: the far end issues its ids as it inserts, so
        // rows landing in this order land in that one.
        let sql = format!(
            "SELECT * FROM {table} WHERE {clause} ORDER BY rowid",
            table = carried.table,
            clause = carried.found.clause(),
        );

        let read = sqlx::query(&sql)
            .bind(conversation_id)
            .fetch_all(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "reading the {table} of Conversation {conversation_id}",
                    table = carried.table,
                )
            })?;

        let mut columns = Vec::new();
        let mut rows = Vec::with_capacity(read.len());

        for row in &read {
            if columns.is_empty() {
                columns = row
                    .columns()
                    .iter()
                    .map(|column| column.name().to_owned())
                    .collect();
            }

            let mut cells = Vec::with_capacity(columns.len());

            for at in 0..columns.len() {
                let cell = read_cell(row, at).with_context(|| {
                    format!(
                        "reading the {table} of Conversation {conversation_id}",
                        table = carried.table,
                    )
                })?;

                held += cell.weight();

                if held > MOST_A_SLICE_IS {
                    return Err(anyhow!(
                        "the record of Conversation {conversation_id} is larger than the \
                         {MOST_A_SLICE_IS} bytes one may be to cross a link",
                    ));
                }

                cells.push(cell);
            }

            rows.push(cells);
        }

        tables.push(Rows {
            table: carried.table.to_owned(),
            columns,
            rows,
        });
    }

    let marks = marked(&mut tx, conversation_id).await?;

    Ok(Slice { tables, marks })
}

/// One value, in whichever of SQLite's shapes the column is holding.
fn read_cell(row: &sqlx::sqlite::SqliteRow, at: usize) -> Result<Cell> {
    let raw = row.try_get_raw(at)?;

    if raw.is_null() {
        return Ok(Cell::Null);
    }

    Ok(match raw.type_info().name() {
        "INTEGER" | "BOOLEAN" => Cell::Int(row.try_get(at)?),
        "REAL" => Cell::Real(row.try_get(at)?),
        "BLOB" => Cell::Blob(row.try_get(at)?),
        _ => Cell::Text(row.try_get(at)?),
    })
}

/// The stop and the escalation off the Conversation's own row.
async fn marked(connection: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<Marks> {
    /// The five columns, in the order they are read.
    type Read = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );

    let row: Option<Read> = sqlx::query_as(
        "SELECT stopped_at, stopped_by, stopped_resets, stopped_notice, escalated_notice
         FROM conversations
         WHERE id = ?",
    )
    .bind(conversation_id)
    .fetch_optional(&mut *connection)
    .await
    .with_context(|| format!("reading how things stand with Conversation {conversation_id}"))?;

    let Some((stopped_at, stopped_by, stopped_resets, stopped_notice, escalated_notice)) = row
    else {
        return Ok(Marks::default());
    };

    Ok(Marks {
        stopped_at,
        stopped_by,
        stopped_resets,
        stopped_notice,
        escalated_notice,
    })
}

impl Slice {
    /// What this weighs towards [`MOST_A_SLICE_IS`], for whatever travels beside
    /// it — the attached files, whose bytes are the same bound's.
    pub fn weight(&self) -> usize {
        self.tables
            .iter()
            .flat_map(|rows| rows.rows.iter())
            .flat_map(|row| row.iter())
            .map(Cell::weight)
            .sum()
    }

    /// Every Repo this record names, on the device it came off.
    ///
    /// What the sending device matches across before anything moves: a row
    /// landing under the wrong repository is the one failure the whole matching
    /// rule exists to prevent, so a Repo with nowhere to land refuses the move
    /// rather than being guessed at.
    ///
    /// **Less the ones only a pending pick on a Question Set names**, which are
    /// [`Self::picked_repos`]: those are a Repo the human asked for while the Set
    /// waited, nothing yet has been done in it, and one with nowhere to land is
    /// struck rather than refusing the move (ADR-0028).
    pub fn repos(&self) -> BTreeSet<i64> {
        self.named(Holds::Repo)
    }

    /// The Repos a pending pick on one of this record's Question Sets names and
    /// nothing else in it does: what may be struck where the far end has no
    /// match — see [`Self::strike`].
    pub fn picked_repos(&self) -> BTreeSet<i64> {
        let mut picked = BTreeSet::new();

        for rows in &self.tables {
            let Some((repo, _, _)) = picks(rows) else {
                continue;
            };

            picked.extend(
                rows.rows
                    .iter()
                    .filter(|row| pending(rows, row))
                    .filter_map(|row| row.get(repo)?.id()),
            );
        }

        &picked - &self.repos()
    }

    /// Strike every pending pick of Repo `repo_id` as it crosses, under the name
    /// it is registered by here: the far end has no Repo it is, so the row lands
    /// naming none, struck, with that name to say what it was — see
    /// [`super::Addition::Unmatched`]. Its `carried_on` is left as it stands.
    pub fn strike(&mut self, repo_id: i64, name: &str) {
        for rows in &mut self.tables {
            let Some((repo, state, named)) = picks(rows) else {
                continue;
            };

            for row in &mut rows.rows {
                if row.get(state) != Some(&Cell::Text("pending".to_owned()))
                    || row.get(repo).and_then(Cell::id) != Some(repo_id)
                {
                    continue;
                }

                row[repo] = Cell::Null;
                row[state] = Cell::Text("struck".to_owned());
                row[named] = Cell::Text(name.to_owned());
            }
        }
    }

    /// And every Agent Profile it names, by uuid.
    pub fn profiles(&self) -> BTreeSet<String> {
        let mut named = BTreeSet::new();

        for (rows, at) in self.columns(Holds::Profile) {
            named.extend(rows.rows.iter().filter_map(|row| match row.get(at)? {
                Cell::Text(id) => Some(id.clone()),
                _ => None,
            }));
        }

        named
    }

    /// Every table of the slice that has a column of that kind, and where in
    /// its rows the column is.
    fn columns(&self, kind: Holds) -> Vec<(&Rows, usize)> {
        let mut found = Vec::new();

        for rows in &self.tables {
            let Some(carried) = CARRIED.iter().find(|carried| carried.table == rows.table) else {
                continue;
            };

            for (column, holds) in carried.ids {
                if *holds != kind {
                    continue;
                }

                if let Some(at) = rows.columns.iter().position(|name| name == column) {
                    found.push((rows, at));
                }
            }
        }

        found
    }

    /// The ids held by every column of that kind.
    fn named(&self, kind: Holds) -> BTreeSet<i64> {
        let mut named = BTreeSet::new();

        for (rows, at) in self.columns(kind) {
            named.extend(
                rows.rows
                    .iter()
                    // A pending pick's Repo, which crosses or is struck —
                    // see [`Self::picked_repos`].
                    .filter(|row| !(kind == Holds::Repo && pending(rows, row)))
                    .filter_map(|row| row.get(at)?.id())
                    // The Repo column that holds no repository, which is
                    // nothing to go and match — see [`NO_REPOSITORY`].
                    .filter(|id| !(kind == Holds::Repo && *id == NO_REPOSITORY)),
            );
        }

        named
    }
}

/// Where a table's rows are the picks on a Question Set, the columns of the
/// Repo, the state, and the name a struck Repo keeps.
fn picks(rows: &Rows) -> Option<(usize, usize, usize)> {
    if rows.table != "set_additions" {
        return None;
    }

    let at = |name: &str| rows.columns.iter().position(|column| column == name);

    Some((at("repo_id")?, at("state")?, at("repo_name")?))
}

/// Whether a row is a pick still waiting on its Set.
fn pending(rows: &Rows, row: &[Cell]) -> bool {
    picks(rows)
        .is_some_and(|(_, state, _)| row.get(state) == Some(&Cell::Text("pending".to_owned())))
}

/// Write a slice down against Conversation `conversation_id` of *this*
/// database, renumbering every id as it lands.
///
/// **One transaction.** A record that landed with its Events and without the
/// Sets asked from them would be a Timeline with holes in it, and one that
/// landed with its Captures pointing at Events that were never written would not
/// land at all — foreign keys are on, so the walk is one SQLite refuses if it is
/// wrong.
///
/// **And it lands over whatever was here**, which is what makes a transfer back
/// one Conversation rather than two records in one (ADR-0020, *Transfer*). A copy
/// coming home to a device that already holds one is the live record arriving
/// over a stale copy of itself: what is here goes first, in this same
/// transaction, and what lands is the whole of what this Conversation has. See
/// [`super::cleanup::cleared`], which empties exactly the tables this writes.
/// Nothing to take out is the ordinary arrival, and it costs a walk over empty
/// tables.
///
/// `Err` says what could not be written, which is what the sending device turns
/// into the Notice on the Conversation it still has: a move that falls over here
/// is swept and the work stays where it was.
pub async fn land(
    pool: &sqlx::SqlitePool,
    conversation_id: i64,
    slice: &Slice,
    renaming: &Renaming,
) -> Result<()> {
    let mut tx = super::writing(pool, "taking a Conversation's record in").await?;

    let held = held(&mut tx, conversation_id).await?;

    super::cleanup::cleared(&mut tx, conversation_id).await?;

    let mut landing = Landing {
        conversation_id,
        renaming,
        events: HashMap::new(),
        sets: HashMap::new(),
    };

    for rows in &slice.tables {
        // A table this build has none of, sent by one that has: left out rather
        // than written blind. The device it came off is a version along, and
        // what it keeps about a Conversation that this build has no table for is
        // nothing this build could draw.
        let Some(carried) = CARRIED.iter().find(|carried| carried.table == rows.table) else {
            continue;
        };

        for row in &rows.rows {
            landing.row(&mut tx, carried, &rows.columns, row).await?;
        }
    }

    stand(&mut tx, conversation_id, &slice.marks, &landing.events).await?;
    held.kept(&mut tx, conversation_id).await?;

    // And the Set half of the arithmetic written down, which is the one part of it
    // anything after this transaction has to know. A resumed agent asks after its
    // Questions by the ids it asked them under, and the record lands in a leg of
    // its own, one before the session that resumes is started — so the map goes on
    // the record rather than being handed on. The Event half is nobody's outside
    // this: what points at an Event landed pointing at it. See
    // [`super::continuations::sets_landed`].
    super::continuations::sets_landed(&mut tx, conversation_id, &landing.sets).await?;

    tx.commit()
        .await
        .context("taking a Conversation's record in")?;

    Ok(())
}

/// What this device held of a stage's places before its record landed again:
/// where it is in its roadmap's queue to join, and when it joined the chain.
///
/// **A return keeps both** (ADR-0029, *Home drives a roadmap across devices*).
/// The landing issues the queue's places and the Timeline's ids afresh, and the
/// queue and the chain are ordered by exactly those — so a stage that came home
/// would otherwise have moved to the back of the one and the top of the other,
/// overtaking stages that finished and joined after it. A stage that never
/// queued here or never joined holds nothing to keep, and lands the way any
/// record does.
struct Held {
    /// Its place in the queue to join.
    place: Option<i64>,
}

/// Read [`Held`] before the record here is taken out, writing down when the
/// stage joined where only the Event about to be renumbered says so.
async fn held(tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<Held> {
    let place: Option<(i64,)> =
        sqlx::query_as("SELECT place FROM stage_joinings WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_optional(&mut *tx)
            .await
            .with_context(|| {
                format!("reading Conversation {conversation_id}'s place in the queue to join")
            })?;

    // The Event `stage_chain` reads a join's order off, which is the first pull
    // request in the Conversation's own Repo. Once only: a second landing finds
    // the first's id kept and leaves it.
    sqlx::query(
        "INSERT OR IGNORE INTO stage_joined_at (conversation_id, event_id)
         SELECT p.conversation_id, MIN(p.event_id)
         FROM pull_requests p
         JOIN conversations c ON c.id = p.conversation_id AND c.repo_id = p.repo_id
         WHERE p.conversation_id = ?
         GROUP BY p.conversation_id",
    )
    .bind(conversation_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("keeping when Conversation {conversation_id} joined its chain"))?;

    Ok(Held {
        place: place.map(|(place,)| place),
    })
}

impl Held {
    /// Put the stage back in the queue at the place it held, whatever the
    /// landing gave it — or did not, the copy that sent it never having
    /// queued over there.
    async fn kept(&self, tx: &mut sqlx::SqliteConnection, conversation_id: i64) -> Result<()> {
        let Some(place) = self.place else {
            return Ok(());
        };

        sqlx::query("DELETE FROM stage_joinings WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("taking the place Conversation {conversation_id} landed with")
            })?;

        sqlx::query("INSERT INTO stage_joinings (place, conversation_id) VALUES (?, ?)")
            .bind(place)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("putting Conversation {conversation_id} back in its place to join")
            })?;

        Ok(())
    }
}

/// What a landing renumbers against as it walks.
///
/// The two maps are filled in as the rows that own those ids land, and read by
/// every row that points at one — which is what the manifest's order is for: an
/// Event before whatever hangs off it, every time.
struct Landing<'a> {
    /// The Conversation the record lands beside, which is what every
    /// `conversation_id` column comes out as.
    conversation_id: i64,

    /// And the two the store cannot work out for itself.
    renaming: &'a Renaming,

    /// What each Timeline Event landed as.
    events: HashMap<i64, i64>,

    /// And each Question Set.
    sets: HashMap<i64, i64>,
}

impl Landing<'_> {
    /// One row written down, with each of its id columns renumbered.
    async fn row(
        &mut self,
        tx: &mut sqlx::SqliteConnection,
        carried: &Carried,
        named: &[String],
        row: &[Cell],
    ) -> Result<()> {
        let mut columns: Vec<&str> = Vec::with_capacity(named.len());
        let mut values: Vec<Cell> = Vec::with_capacity(named.len());
        let mut issued: Option<(Remembered, i64)> = None;

        for (at, column) in named.iter().enumerate() {
            let cell = row.get(at).cloned().unwrap_or(Cell::Null);

            let holds = carried
                .ids
                .iter()
                .find(|(named, _)| named == column)
                .map(|(_, holds)| *holds);

            let landed = match holds {
                None => cell,

                // The far end's own number, taken from the insert rather than
                // written into it.
                Some(Holds::Mine(remembered)) => {
                    // Unless nothing points at it, in which case there is no number
                    // to wait for on the way back either.
                    if remembered != Remembered::Nothing
                        && let Some(was) = cell.id()
                    {
                        issued = Some((remembered, was));
                    }

                    continue;
                }

                Some(Holds::Conversation) => Cell::Int(self.conversation_id),

                Some(Holds::Event) => match cell.id() {
                    None => cell,
                    Some(was) => Cell::Int(*self.events.get(&was).ok_or_else(|| {
                        anyhow!(
                            "the {table} of a transferred Conversation names Timeline Event {was}, \
                             which did not cross with it",
                            table = carried.table,
                        )
                    })?),
                },

                Some(Holds::Set) => match cell.id() {
                    None => cell,
                    Some(was) => Cell::Int(*self.sets.get(&was).ok_or_else(|| {
                        anyhow!(
                            "the {table} of a transferred Conversation names Question Set {was}, \
                             which did not cross with it",
                            table = carried.table,
                        )
                    })?),
                },

                // The zero that means *no repository* crosses as it stands: a
                // wrap-up's review is settled about the Conversation rather than
                // about any one pull request, and there is nothing to match — see
                // `wrap_up`'s own `NO_PULL_REQUEST`.
                Some(Holds::Repo) => match cell.id() {
                    None | Some(NO_REPOSITORY) => cell,
                    Some(was) => Cell::Int(*self.renaming.repos.get(&was).ok_or_else(|| {
                        anyhow!(
                            "the {table} of a transferred Conversation names a repository no Repo \
                             on this device is",
                            table = carried.table,
                        )
                    })?),
                },

                // An account this device has never heard of leaves the column
                // empty. It is history — what a Steer was run under a year ago —
                // and the alternative to an empty column is a row pointing at
                // somebody else's account.
                Some(Holds::Profile) => {
                    let here = match &cell {
                        Cell::Text(was) => self.renaming.profiles.get(was),
                        _ => None,
                    };

                    match here {
                        Some(here) => Cell::Text(here.clone()),
                        None => Cell::Null,
                    }
                }
            };

            columns.push(column);
            values.push(landed);
        }

        let sql = format!(
            "INSERT INTO {table} ({columns}) VALUES ({places}){returning}",
            table = carried.table,
            columns = columns.join(", "),
            places = std::iter::repeat_n("?", columns.len())
                .collect::<Vec<_>>()
                .join(", "),
            returning = if issued.is_some() {
                " RETURNING id"
            } else {
                ""
            },
        );

        let mut query = sqlx::query(&sql);

        for value in values {
            query = bound(query, value);
        }

        match issued {
            None => {
                query.execute(&mut *tx).await.with_context(|| {
                    format!(
                        "writing a row of the {table} a transfer brought",
                        table = carried.table,
                    )
                })?;
            }

            Some((remembered, was)) => {
                let landed = query.fetch_one(&mut *tx).await.with_context(|| {
                    format!(
                        "writing a row of the {table} a transfer brought",
                        table = carried.table,
                    )
                })?;

                let id: i64 = landed
                    .try_get(0)
                    .context("the id the far end gave an arriving row")?;

                match remembered {
                    Remembered::Event => self.events.insert(was, id),
                    Remembered::Set => self.sets.insert(was, id),
                    Remembered::Nothing => None,
                };
            }
        }

        Ok(())
    }
}

/// One value bound onto a query, in whichever shape it crossed in.
fn bound<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    value: Cell,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match value {
        Cell::Null => query.bind(Option::<i64>::None),
        Cell::Int(number) => query.bind(number),
        Cell::Real(number) => query.bind(number),
        Cell::Text(text) => query.bind(text),
        Cell::Blob(bytes) => query.bind(bytes),
    }
}

/// And the stop and the escalation onto the Conversation's own row.
async fn stand(
    tx: &mut sqlx::SqliteConnection,
    conversation_id: i64,
    marks: &Marks,
    events: &HashMap<i64, i64>,
) -> Result<()> {
    // A Notice that did not cross is no Notice: the column points at a Timeline
    // Event, and one naming an Event of somebody else's would be a stop drawn
    // over the wrong card.
    let stopped_notice = marks
        .stopped_notice
        .and_then(|was| events.get(&was).copied());
    let escalated_notice = marks
        .escalated_notice
        .and_then(|was| events.get(&was).copied());

    sqlx::query(
        "UPDATE conversations
         SET stopped_at = ?,
             stopped_by = ?,
             stopped_resets = ?,
             stopped_notice = ?,
             escalated_notice = ?
         WHERE id = ?",
    )
    .bind(&marks.stopped_at)
    .bind(&marks.stopped_by)
    .bind(&marks.stopped_resets)
    .bind(stopped_notice)
    .bind(escalated_notice)
    .bind(conversation_id)
    .execute(&mut *tx)
    .await
    .with_context(|| format!("writing how things stand with Conversation {conversation_id}"))?;

    Ok(())
}
