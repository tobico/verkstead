//! Telling the devices what happened while nobody was watching.
//!
//! Two kinds of thing are worth a phone lighting up. **Needs-you**: a Question
//! Set has arrived, driving has stopped on something Verkstead decided to stop
//! for, or the account the run was spending ran out of window. And
//! **milestones**: the work is on a pull request, a roadmap has moved on to its
//! next stage — or could not, for want of a git author — or run out of stages, or
//! a Conversation has reached Done. One push
//! per subscribed device in every case,
//! encrypted for that device's own keys and signed with the VAPID identity the
//! store generated on first run. The body is small on purpose: enough for the
//! service worker to draw the notification and to know which page to open, and
//! nothing that would put a Question — or the substance of the work — in a
//! notification the phone shows on a lock screen.
//!
//! And **the machine**, which is the one kind that is about no piece of work at
//! all: another device has asked to be let into this one's cluster, and
//! somebody has ten minutes to say yes. That one comes in through [`Word`]
//! rather than [`News`] — every arm of the latter loads a Conversation and
//! titles itself by its branch, and there is no Conversation here to load — and
//! goes out through the same [`notify`] underneath, which never knew about one.
//!
//! What tells one from another is its title, which is why all of them are
//! written in the same place: see [`News::title`], and [`Word::title`] beside
//! it. A phone that lights up with the same sentence whatever happened is a
//! phone the human learns to ignore.
//!
//! Sending always happens behind the thing it is announcing, never in front of
//! it. Delivery goes out through the browser vendors' push services, which is
//! the one place Verkstead reaches the public internet, and none of it is
//! reliable enough to make the record depend on: a service that cannot be
//! reached costs a notification, and never the Set, the pull request or the
//! stop it was about.
//!
//! No push writes anything down. Each is sent from the one place that already
//! knows the thing happened, and the record of it is whatever that place wrote.
//!
//! A stop's push is what a stop is worth: the run has stopped and will not start again
//! until the human presses Resume, so a silent stop is one they find days late.
//! Only the stops Verkstead decided on are worth a phone, though — a stop
//! nobody chose is one a restart picks up unasked, and telling somebody about a
//! run that is about to carry on by itself is a notification that asks for
//! nothing. The Notice on the Timeline is what says it in full either way; this
//! is only what reaches a pocket.
//!
//! A push service is also the only thing that can tell us a device has gone —
//! the app was uninstalled, the subscription expired — and it says so with a
//! `404` or a `410`. Those are pruned. Everything else, a timeout or a `503`,
//! is a notification lost and the device is left alone.
//!
//! **And in a cluster the news goes to the members too** (ADR-0020, *The opened
//! device relays*). A phone is installed from one device and subscribes to *that*
//! device's browsers, so without this a cluster would want a phone per machine.
//! So [`say`] has a second step: once the local push has gone out, the same
//! sentence goes to every member over the Peer Listener, and each of them shows
//! it with the sending device leading the title — see
//! [`crate::device::Devices::spread`], and [`crate::peer::news`], which is what
//! answers at the far end. Behind the local push and behind the record, exactly
//! as the local push is behind the record: a member that is switched off costs a
//! notification and nothing else, and nothing is queued and nothing is retried.
//!
//! Coming the other way, [`heard_from`] is a third kind beside [`News`] and
//! [`Word`]: a member's news, whose sentence this device did not write, titled
//! with the device in front of it and opened at that device's own URL. It relays
//! nothing onward — in a cluster everybody holds a link to everybody, so a device
//! passing on a third one's news would be saying that news was its own.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use verkstead_render::RelayedNews;
use verkstead_schema::QuestionSet;
use verkstead_store::{PushSubscription, VapidKeys};
use web_push_native::{Auth, WebPushBuilder};

/// How long a push service should hold a notification for a phone that is off
/// or out of signal. Half a day: a Question Set older than that has either been
/// answered at a desk or is not going to be answered from a lock screen.
const HOLD_FOR: Duration = Duration::from_secs(12 * 60 * 60);

/// How long to wait on one push service before giving up on it. A push service
/// that is this slow has cost this device its notification either way, and the
/// remaining devices are still waiting their turn.
const REACHABLE_WITHIN: Duration = Duration::from_secs(10);

/// How long a VAPID signature stays good for. RFC 8292 caps it at 24 hours;
/// each push is signed as it is sent, so this only has to outlive the request.
const SIGNATURE_GOOD_FOR: i64 = 12 * 60 * 60;

/// The contact the VAPID claim carries: whom a push service would complain to
/// about this server's traffic. It wants a `mailto:` or an `https:` URI, and
/// the project is the only thing that is true of every Verkstead — this is a
/// single-user tool with nobody's address configured in it.
const CONTACT: &str = "https://github.com/tobico/verkstead";

/// The one JWT header these signatures ever use, so it is spelled out rather
/// than serialized: ES256 is what VAPID is defined in terms of.
const JWT_HEADER: &str = r#"{"typ":"JWT","alg":"ES256"}"#;

/// What the service worker is handed: enough to draw the notification, and where
/// tapping it goes.
#[derive(Debug, Serialize)]
struct Notice<'a> {
    /// The page the notification opens — the Timeline Event a Set landed on,
    /// which is where a Set is read, and the Conversation for everything that
    /// happened to one.
    ///
    /// Said by the server rather than worked out by the worker, because what a
    /// push is about is the server's to know: a phone woken by a stop that
    /// landed on a Question Set it is not about would be worse than no
    /// notification at all.
    path: String,

    title: &'a str,

    /// Which repository this is about, when it is known — the one thing that
    /// tells two notifications apart at a glance on a lock screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<&'a str>,
}

/// What became of one push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    /// The push service took it. Whether the phone ever shows it is between the
    /// two of them.
    Taken,

    /// This subscription is finished with, for good. The device uninstalled the
    /// app, or its subscription expired.
    Gone,
}

/// Tell every subscribed device about a Set, without making the agent wait for
/// it.
///
/// Returns as soon as the work is handed to the runtime: the caller's job is to
/// answer the agent, and this must not be able to delay that or to fail it.
pub(crate) fn announce(pool: &SqlitePool, id: i64, set: &QuestionSet) {
    let title = set.title.clone();
    let project = set.project.clone();
    let pool = pool.clone();

    tokio::spawn(async move {
        // Where tapping it goes. A Set has no page of its own: it is read in the
        // details pane of the Timeline Event it landed on, so the path names the
        // Event and the Conversation it is on.
        //
        // Looked up rather than carried in, because it is a fact about the
        // record that was just written and the store is what holds it. A Set
        // whose Event cannot be found is a broken record rather than a Set with
        // nowhere to be — the two are written in one transaction — so there is
        // nowhere to send anybody and nothing worth waking a phone for.
        let found = match verkstead_store::opened_at(&pool, id).await {
            Ok(Some(found)) => found,
            Ok(None) => {
                tracing::error!(set = id, "the Set is on no Timeline, so nobody was told");
                return;
            }
            Err(error) => {
                tracing::error!(set = id, error = ?error, "looking for where the Set is read failed");
                return;
            }
        };

        let (conversation, event) = found;

        let notice = Notice {
            path: format!("/conversations/{conversation}/events/{event}"),
            title: &title,
            project: project.as_deref(),
        };

        let notice = match serde_json::to_vec(&notice) {
            Ok(notice) => notice,
            Err(error) => {
                tracing::error!(set = id, error = ?error, "the push notice could not be built");
                return;
            }
        };

        if let Err(error) = notify(&pool, &format!("Set {id}"), &notice).await {
            tracing::error!(set = id, error = ?error, "telling the devices about a Set failed");
        }
    });
}

/// What a push about a Conversation is saying.
///
/// One enum rather than a function each, because nearly all of it is common: a
/// Conversation to load, a branch to read it by, the Repo underneath and one
/// push per subscribed device. What differs is the sentence — and that is the
/// half worth keeping together, since the whole job of a title is to say which
/// of these it is to somebody glancing at a lock screen.
#[derive(Debug, Clone)]
pub(crate) enum News {
    /// Driving stopped on something Verkstead decided to stop for, and the
    /// Conversation stays stopped until Resume is pressed — see
    /// [`crate::stopping`]. `stopped` is what ought to have been happening, with
    /// its first letter up: the same words the Notice opens with, so that the
    /// phone and the Timeline say the same thing about the same stop.
    Stopped { stopped: String },

    /// A session went Idle and would not answer the Rescue, so the human is
    /// told instead — see [`crate::rescues`]. Not a stop: the session is still
    /// running. `idle` is what it ought to have been doing, first letter up, as
    /// the Notice opens with it.
    Escalated { idle: String },

    /// The account a run was spending ran out of window, which is a stop like
    /// the one above it, said in the words that decide whether the human gets
    /// up for it — see [`crate::limits`].
    OutOfWindow {
        profile: String,
        resets: Option<String>,
    },

    /// The work the Conversation was for is on a pull request, and the wrap-up
    /// has started.
    OnAPullRequest { number: i64 },

    /// The stage after the one that just settled has started, on a Conversation
    /// of its own — which is this one.
    StageStarted { label: String, roadmap: String },

    /// The stage after the one that settled was not started, because Verkstead
    /// has no git author to commit the clearing of an inherited task list as —
    /// see [`crate::continuing`]. Told like a stage that started, because it is
    /// the same fact with the answer reversed: the roadmap has stopped moving on
    /// its own, and nobody is at a Timeline to read why.
    StageNeedsAuthor { label: String, roadmap: String },

    /// A roadmap that has run out of stages: the one that settled was its last.
    RoadmapComplete { roadmap: String },

    /// The Conversation has reached Done. Verkstead has finished with the work;
    /// whether it is merged is the human's.
    Done,
}

impl News {
    /// The sentence the lock screen shows.
    ///
    /// Every title in one place, because the thing they all have to do is be
    /// told apart from each other at a glance. Two rules hold across the lot of
    /// them. What a piece of work is read by is its branch — see
    /// [`verkstead_store::ConversationRow`] — so a title names that, unless it
    /// has something the human knows the work by better: the account that ran
    /// out, or the stage of the roadmap. And nothing of the *substance* of the
    /// work goes in, which is the rule that keeps a Question out of a Set's push.
    fn title(&self, branch: &str) -> String {
        match self {
            // The step rather than how it went wrong: which part of the run
            // stopped is what decides whether the human gets up, and the
            // evidence underneath it is one tap away.
            News::Stopped { stopped } => format!("{stopped} stopped on {branch}"),
            // The same step, and the word that says nothing stopped: the session
            // is sitting there, and the human is the one to move it.
            News::Escalated { idle } => format!("{idle} has gone idle on {branch}"),
            // The account and when it comes back, because those are the two
            // things that decide whether the human does anything about it: an
            // account back in twenty minutes is one to leave alone.
            News::OutOfWindow {
                profile,
                resets: Some(resets),
            } => format!("{profile} is out of window until {resets}"),
            News::OutOfWindow {
                profile,
                resets: None,
            } => format!("{profile} is out of window"),
            News::OnAPullRequest { number } => format!("{branch} is on pull request #{number}"),
            // Read by the stage rather than by the branch: a stage is a
            // Conversation whose name in the human's head is its number in the
            // roadmap, and the branch is named after that anyway.
            News::StageStarted { label, roadmap } => {
                format!("Stage {label} of the `{roadmap}` roadmap has started")
            }
            // What the human has to go and do rather than what stopped: the
            // stage is waiting on one field in Settings, and a title saying so
            // is one they can act on without opening anything.
            News::StageNeedsAuthor { label, roadmap } => {
                format!("Stage {label} of the `{roadmap}` roadmap needs a git author")
            }
            News::RoadmapComplete { roadmap } => format!("The `{roadmap}` roadmap is complete"),
            News::Done => format!("{branch} is done"),
        }
    }

    /// What the log calls it, where a push could not be sent.
    fn about(&self) -> &'static str {
        match self {
            News::Stopped { .. } => "the stop",
            News::Escalated { .. } => "the session gone idle",
            News::OutOfWindow { .. } => "the stop for a window",
            News::OnAPullRequest { .. } => "the pull request",
            News::StageStarted { .. } => "the stage that started",
            News::StageNeedsAuthor { .. } => "the stage that needs a git author",
            News::RoadmapComplete { .. } => "the roadmap that is finished",
            News::Done => "the work being done",
        }
    }
}

/// Tell every subscribed device something that happened to a Conversation,
/// without making the thing that happened wait for it.
///
/// Returns as soon as the work is handed to the runtime, exactly as a Set's push
/// does: the caller's job is to put the stop or the pull request on the record,
/// and none of this may delay that or fail it. A push service that cannot be
/// reached costs a notification and nothing else.
pub(crate) fn told(pool: &SqlitePool, conversation_id: i64, news: News) {
    let pool = pool.clone();

    tokio::spawn(async move {
        if let Err(error) = say(&pool, conversation_id, &news).await {
            tracing::error!(
                conversation_id,
                about = news.about(),
                error = ?error,
                "telling the devices about a Conversation failed",
            );
        }
    });
}

/// Tell the devices that a Conversation has reached Done.
///
/// **One arm of [`News`] with a name of its own**, and the only one that has one,
/// because it is the only one a caller outside this crate can say: every other
/// arm is written by a stop, a session gone idle, an account out of window or a
/// roadmap moving on, each of which is a session running inside Verkstead. The
/// settle that says this is [`crate::settling`]'s, and a suite asking what a
/// *cluster* makes of a piece of news says it here rather than standing a
/// wrap-up up to produce one — see `tests/news.rs`. The path it takes is the
/// whole of the real one: the local push, and then every member.
pub fn the_work_is_done(pool: &SqlitePool, conversation_id: i64) {
    told(pool, conversation_id, News::Done);
}

/// The notice one piece of news is worth, sent — to this device's own phones,
/// and then to its members.
///
/// With the Repo underneath the title, so that a lock screen says which piece of
/// work this is about — that is the one thing that tells two notifications apart
/// where their titles read alike.
///
/// **The members are the second step rather than the first**, which is the same
/// ordering the whole of this module is written in: what a notification is about
/// is already on the record, this device's own phones are its own, and a member
/// whose machine is shut must not hold them up. See
/// [`crate::device::Devices::spread`].
async fn say(pool: &SqlitePool, conversation_id: i64, news: &News) -> Result<()> {
    let Some(conversation) = verkstead_store::load_conversation(pool, conversation_id).await?
    else {
        // Closed and gone between the thing happening and this being sent.
        // There is nobody left to tell anything about it.
        return Ok(());
    };

    let title = news.title(&conversation.branch);

    let notice = Notice {
        path: format!("/conversations/{conversation_id}"),
        title: &title,
        project: Some(&conversation.repo.name),
    };

    let notice = serde_json::to_vec(&notice).context("building the push notice")?;

    // This device's own phones first, and what they were told is kept rather than
    // returned: the members are owed the news whether or not a push service took
    // it, those being two different machines' problems.
    let locally = notify(
        pool,
        &format!("{} on {conversation_id}", news.about()),
        &notice,
    )
    .await;

    // And then every member, with the sentence this device's own lock screens
    // just got and no device on it: the receiver is what puts one in front, off
    // the certificate this call is made under — see [`crate::peer::news`].
    if let Some(cluster) = cluster_of(pool) {
        cluster
            .spread(&RelayedNews {
                conversation: conversation_id,
                said: title,
                project: Some(conversation.repo.name),
            })
            .await;
    }

    locally
}

/// Every Verkstead in this process and the cluster each of them owes its news to,
/// by the database it writes that news down in.
///
/// **Held rather than threaded**, the way the named pipe's grant is — see
/// [`crate::pipe::hold_the_grant`]. A piece of news is sent from nine places: a
/// stop, a rescue, an account out of window, a pull request, three roadmap moves,
/// a Conversation done. Between the deepest of them and here sit a session's
/// relay loop and a limit watcher that hold a pool and a Nudge channel and
/// nothing else on purpose, and threading a membership through the lot would put
/// a parameter about a cluster through code that has never heard of one.
///
/// **Keyed by the store, because that is what says which Verkstead the news
/// happened in**, and it is the one thing every one of those places has in hand.
/// A served process holds exactly one entry; a suite holds one per Verkstead it
/// stood up, which is what lets two of them tell each other news in one process
/// without either answering out of the other's membership.
static THE_CLUSTER: LazyLock<Mutex<HashMap<PathBuf, crate::device::Devices>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Hold `devices` as the cluster the news written into `pool` is owed to.
///
/// Called once as the server comes up, with the handle the Devices section is
/// answered out of — see `crate::run_on_keyed`. A Verkstead that is linked to
/// nothing still holds one: what makes the difference is that its membership has
/// no rows, which is a walk over nothing rather than a thing to check for here.
pub fn hold_the_cluster(pool: &SqlitePool, devices: &crate::device::Devices) {
    THE_CLUSTER
        .lock()
        .unwrap_or_else(|held| held.into_inner())
        .insert(keyed(pool), devices.clone());
}

/// And the cluster the news written into `pool` is owed to, where one is held.
///
/// `None` is every router stood up without a Data Directory — a suite's, which
/// invented no identity and holds no membership — and a server between opening
/// its database and reading its identity. Neither is a cluster with nobody in it:
/// it is a Verkstead with nobody to tell.
fn cluster_of(pool: &SqlitePool) -> Option<crate::device::Devices> {
    THE_CLUSTER
        .lock()
        .unwrap_or_else(|held| held.into_inner())
        .get(&keyed(pool))
        .cloned()
}

/// What a pool is held under: the database file it is open on.
///
/// One Verkstead is one Data Directory and one database in it, so this is that
/// Verkstead said in the one term a caller here has — see [`THE_CLUSTER`].
fn keyed(pool: &SqlitePool) -> PathBuf {
    pool.connect_options().get_filename().to_path_buf()
}

/// What a push about something that is not a piece of work is saying.
///
/// **The second entry point beside [`News`], rather than a tenth arm of it.**
/// Every one of those loads a Conversation and titles itself by its branch,
/// because every one of them is about work; this is about the machine. Bending
/// the enum to carry one would have meant a Conversation id nothing reads and a
/// branch nothing says, on the one code path every other notification takes. So
/// there are two ways in and one way out: the sending underneath is
/// [`notify`], which never knew about a Conversation to begin with.
///
/// What still holds is the rule all the titles are written in one place for —
/// see [`News::title`]. A notification has to be told from every other one at a
/// glance, and these are read on the same lock screen as those.
#[derive(Debug, Clone)]
pub(crate) enum Word {
    /// A device has asked to be let into this one's cluster, and it will go on
    /// asking for ten minutes — see [`crate::peer::joining`]. Named for the
    /// device, which is the whole of what the human is deciding about.
    ADeviceIsAsking { name: String },
}

impl Word {
    /// The sentence the lock screen shows.
    fn title(&self) -> String {
        match self {
            // The device rather than what it wants: a phone that lights up
            // while somebody is standing at another machine is one the name is
            // the whole answer on.
            //
            // And the name is the one thing in any title here that a *stranger*
            // wrote — a join arrives from a device this one holds no membership
            // for, which is what a join is. So it is cut to [`A_NAME`] before it
            // goes in, which is what keeps the rule below from being a rule
            // about the titles this tree writes rather than about the ones a
            // phone shows.
            Word::ADeviceIsAsking { name } => {
                format!("{} is asking to link with this device", fitting(name))
            }
        }
    }

    /// And where a tap goes: the pane the press is on.
    ///
    /// The Remote access pane rather than a page of its own, because the modal
    /// is not a page — it is raised in whatever workbench is open, and the one
    /// this tap opens raises it off the Nudge it reads on arrival. So this is a
    /// place to land rather than the thing being opened.
    fn path(&self) -> &'static str {
        match self {
            Word::ADeviceIsAsking { .. } => "/settings/remote",
        }
    }

    /// What the log calls it, where a push could not be sent.
    fn about(&self) -> &'static str {
        match self {
            Word::ADeviceIsAsking { .. } => "the device asking to link",
        }
    }
}

/// Tell every subscribed device a piece of news a member has just told this one,
/// without making the member wait for it (ADR-0020, *The opened device relays*).
///
/// **The third entry point beside [`News`] and [`Word`]**, and the one thing that
/// makes it its own: the sentence was written on another machine. Nothing here
/// chooses what the news *is* — there is no variant to match on, which is exactly
/// what lets a member running a newer Verkstead tell this one about a kind of news
/// it has never heard of. What this device decides is the two things that are its
/// own: the title, which leads with the device the news came from, and the path,
/// which is that device's own URL for the Conversation.
///
/// Returns as soon as the work is handed to the runtime, exactly as the three
/// above it do: the route's job is to answer the member, and a push service that
/// cannot be reached costs a notification and nothing else.
///
/// **And nothing goes on from here.** The news stops at this device: in a cluster
/// everybody holds a link to everybody, so a device passing on what a third one
/// told it would be saying that news was its own — which is why this does not go
/// through [`say`], the one place that spreads.
pub(crate) fn heard_from(pool: &SqlitePool, device: &str, name: &str, news: RelayedNews) {
    let pool = pool.clone();
    let path = format!("/devices/{device}/conversations/{}", news.conversation);
    let title = relayed(name, &news.said);
    let device = device.to_owned();

    tokio::spawn(async move {
        let notice = Notice {
            path,
            title: &title,

            // The repository the member named, which stands under a relayed title
            // exactly as it stands under a local one: what tells two
            // notifications apart at a glance is the same thing whichever machine
            // the work is on.
            project: news.project.as_deref(),
        };

        let notice = match serde_json::to_vec(&notice) {
            Ok(notice) => notice,
            Err(error) => {
                tracing::error!(device, error = ?error, "the push notice could not be built");
                return;
            }
        };

        if let Err(error) = notify(&pool, &format!("news from device {device}"), &notice).await {
            tracing::error!(
                device,
                error = ?error,
                "telling the devices what a member said failed",
            );
        }
    });
}

/// A member's news as this device's lock screens read it: the device it came
/// from, and then what it said.
///
/// **The device leads**, because that is the whole of what a relayed notification
/// adds: the human has one phone for a cluster, and which machine the work was on
/// is what they are reading the title for. A push about this device's own work
/// carries no device at all — see [`News::title`], which is unchanged — so the
/// mark of a member's news is that something is in front of the sentence.
///
/// **And both halves are bounded**, which is the rule [`fitting`] was written
/// for, here applied twice: a relayed title is the one this tree writes where
/// *neither* the name nor the sentence is its own. A member that called itself
/// something enormous, or sent a sentence that was not one, is cut rather than
/// allowed to be the whole of a lock screen.
fn relayed(device: &str, said: &str) -> String {
    format!("{} — {}", fitting(device), fitting_sentence(said))
}

/// How much of a name a title will carry.
///
/// Forty, which is the longest a name can be before the sentence around it goes
/// past the eighty characters a lock screen shows whole — the rule every title
/// here is written to and the suite below asserts. It is not a bound on what a
/// device may be called: the join that carries one has its own, in
/// [`crate::peer::joining`], and it is generous enough to hold any real
/// hostname. This is a bound on the one line where somebody else's hostname is
/// a sentence of this machine's.
const A_NAME: usize = 40;

/// A name as a title will carry it: whole where it fits, and cut with an ellipsis
/// where it does not.
///
/// **Because this is a thing in a notification that another machine wrote.** A
/// device asking to link is a non-member by definition, and the name it gives is
/// its own word for itself; a member's news leads with the name that member said
/// at its last exchange. Either way what stops it from being the whole of a lock
/// screen is this rather than anything the far end did.
fn fitting(name: &str) -> String {
    cut_to(name, A_NAME)
}

/// How much of a sentence another machine wrote a title will carry.
///
/// Eighty, which is what a lock screen shows whole — the width every title here
/// is written to. A sentence longer than that is one the phone would cut anyway,
/// so cutting it here loses nothing the human would have read and stops a member
/// from being a paragraph on somebody's lock screen. It is not a bound on what a
/// member may *send*: the route that takes one has its own, in
/// [`crate::peer::news`], and it is generous enough to hold any title this tree
/// writes.
///
/// A relayed title therefore runs past eighty where the device's name and the
/// sentence together do — as a local title already runs past it where the branch
/// in it is long. What the two bounds are for is the half of a title this machine
/// did not write, rather than the width, which the branch decided long ago.
const A_SENTENCE: usize = 80;

/// The same for a sentence — see [`A_SENTENCE`].
fn fitting_sentence(said: &str) -> String {
    cut_to(said.trim(), A_SENTENCE)
}

/// A string as a title will carry it: whole where it fits `most` characters, and
/// cut with an ellipsis where it does not.
///
/// Counted in characters rather than bytes, and cut on one: a name in kanji is
/// forty characters like any other, and cutting a string mid-character would
/// panic rather than shorten anything.
fn cut_to(said: &str, most: usize) -> String {
    if said.chars().count() <= most {
        return said.to_owned();
    }

    said.chars().take(most).collect::<String>() + "…"
}

/// Tell every subscribed device something that happened to this machine, without
/// making the thing that happened wait for it.
///
/// Returns as soon as the work is handed to the runtime, exactly as the two
/// above do and for their reason: the caller's job is to write the request down
/// and answer the device that made it, and none of this may delay that or fail
/// it. A push service that cannot be reached costs a notification and nothing
/// else — the modal is up and the press works regardless.
pub(crate) fn mentioned(pool: &SqlitePool, word: Word) {
    let pool = pool.clone();

    tokio::spawn(async move {
        let title = word.title();

        let notice = Notice {
            path: word.path().to_owned(),
            title: &title,

            // Nothing. The project is what tells two notifications about two
            // pieces of work apart, and this is about no work at all — a
            // repository named under it would be a repository this has nothing
            // to do with.
            project: None,
        };

        let notice = match serde_json::to_vec(&notice) {
            Ok(notice) => notice,
            Err(error) => {
                tracing::error!(about = word.about(), error = ?error, "the push notice could not be built");
                return;
            }
        };

        if let Err(error) = notify(&pool, word.about(), &notice).await {
            tracing::error!(
                about = word.about(),
                error = ?error,
                "telling the devices about this machine failed",
            );
        }
    });
}

/// Send the notice to every device, and prune the ones the push services have
/// finished with.
///
/// One device at a time rather than all at once: this is a single human's
/// handful of devices, and the timeout bounds what a slow push service can cost
/// the ones behind it.
async fn notify(pool: &SqlitePool, about: &str, notice: &[u8]) -> Result<()> {
    let devices = verkstead_store::push_subscriptions(pool).await?;
    if devices.is_empty() {
        return Ok(());
    }

    let keys = verkstead_store::vapid_keys(pool).await?;
    let client = reqwest::Client::builder()
        .timeout(REACHABLE_WITHIN)
        .build()
        .context("building the push client")?;

    for device in devices {
        match send(&client, &keys, &device, notice).await {
            Ok(Delivery::Taken) => {
                tracing::debug!(about, endpoint = %device.endpoint, "a device was told");
            }
            Ok(Delivery::Gone) => {
                tracing::info!(
                    endpoint = %device.endpoint,
                    "the push service has finished with this device, so it is forgotten",
                );
                verkstead_store::forget_subscription(pool, &device.endpoint).await?;
            }
            // Logged rather than returned: the devices behind this one have
            // done nothing wrong, and there is nobody to hand the failure to —
            // the agent was answered before any of this started.
            Err(error) => {
                tracing::warn!(
                    about,
                    endpoint = %device.endpoint,
                    error = ?error,
                    "a device was not told",
                );
            }
        }
    }

    Ok(())
}

/// Put one push on the wire, and read what the push service made of it.
async fn send(
    client: &reqwest::Client,
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<Delivery> {
    let request = addressed(keys, device, notice)?;

    let response = client
        .execute(reqwest::Request::try_from(request).context("preparing the push request")?)
        .await
        .context("reaching the push service")?;

    let status = response.status();

    if status.is_success() {
        return Ok(Delivery::Taken);
    }

    // The only two answers that mean the subscription itself is over. Anything
    // else — a timeout, a 429, a 503 — is this notification lost and no word at
    // all about the device.
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
        return Ok(Delivery::Gone);
    }

    let said = response.text().await.unwrap_or_default();
    bail!("the push service answered {status}: {said}");
}

/// The notice encrypted for one device, in the request its push service takes.
fn addressed(
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<http::Request<Vec<u8>>> {
    let endpoint: http::Uri = device
        .endpoint
        .parse()
        .with_context(|| format!("the stored endpoint {:?} is not a URL", device.endpoint))?;

    let public = URL_SAFE_NO_PAD
        .decode(&device.p256dh)
        .context("the stored p256dh is not base64url")?;
    let public = p256::PublicKey::from_sec1_bytes(&public)
        .context("the stored p256dh is not a P-256 key")?;

    let auth = URL_SAFE_NO_PAD
        .decode(&device.auth)
        .context("the stored auth secret is not base64url")?;
    if auth.len() != 16 {
        bail!("the stored auth secret is {} bytes, not 16", auth.len());
    }

    let mut request = WebPushBuilder::new(endpoint.clone(), public, Auth::clone_from_slice(&auth))
        .with_valid_duration(HOLD_FOR)
        .build(notice.to_vec())
        .map_err(|err| anyhow::anyhow!("encrypting the push: {err}"))?;

    let authorization = authorization(keys, &endpoint)?;
    request.headers_mut().insert(
        http::header::AUTHORIZATION,
        authorization
            .parse()
            .context("the VAPID header is not a header value")?,
    );

    Ok(request)
}

/// The `Authorization` header a push service checks this server by: a signed
/// claim about who is sending, alongside the public key it was signed with —
/// which is the key the device subscribed against.
fn authorization(keys: &VapidKeys, endpoint: &http::Uri) -> Result<String> {
    let claims = serde_json::json!({
        "aud": audience(endpoint)?,
        "exp": OffsetDateTime::now_utc().unix_timestamp() + SIGNATURE_GOOD_FOR,
        "sub": CONTACT,
    });

    let token = signed(
        &keys.private_key,
        &serde_json::to_vec(&claims).context("serialising the VAPID claims")?,
    )?;

    Ok(format!("vapid t={token}, k={}", keys.public_key))
}

/// Who a signature is for: the push service's origin, and nothing of the path
/// that identifies the device. A signature made out to one service is no use at
/// another, which is the whole point of the claim.
fn audience(endpoint: &http::Uri) -> Result<String> {
    let scheme = endpoint
        .scheme_str()
        .context("the endpoint has no scheme to make the signature out to")?;
    let authority = endpoint
        .authority()
        .context("the endpoint has no host to make the signature out to")?;

    Ok(format!("{scheme}://{authority}"))
}

/// The claims as a signed ES256 JWT.
///
/// Signed with `p256` directly rather than through a JWT crate: this is one
/// signature over two claims under a fixed header, and the keypair is already
/// a `p256` one.
fn signed(private_key: &str, claims: &[u8]) -> Result<String> {
    let scalar = URL_SAFE_NO_PAD
        .decode(private_key)
        .context("the stored private key is not base64url")?;
    let key = p256::ecdsa::SigningKey::from_slice(&scalar)
        .context("the stored private key is not a P-256 scalar")?;

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(JWT_HEADER),
        URL_SAFE_NO_PAD.encode(claims),
    );

    // Fixed-width r‖s rather than DER: JWS defines ES256 as the 64-byte form,
    // and a push service handed DER rejects the signature.
    let signature: p256::ecdsa::Signature = key.sign(signing_input.as_bytes());

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes()),
    ))
}

#[cfg(test)]
mod tests {
    use super::{JWT_HEADER, News, Word, audience, relayed, signed};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::signature::Verifier;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::elliptic_curve::sec1::ToEncodedPoint;

    /// Every piece of news, as one Conversation would produce them.
    fn all() -> Vec<News> {
        vec![
            News::Stopped {
                stopped: "Implementing the work".to_owned(),
            },
            News::Escalated {
                idle: "Implementing the work".to_owned(),
            },
            News::OutOfWindow {
                profile: "implementation".to_owned(),
                resets: Some("3pm".to_owned()),
            },
            News::OutOfWindow {
                profile: "implementation".to_owned(),
                resets: None,
            },
            News::OnAPullRequest { number: 41 },
            News::StageStarted {
                label: "01".to_owned(),
                roadmap: "rate-limiting".to_owned(),
            },
            News::StageNeedsAuthor {
                label: "01".to_owned(),
                roadmap: "rate-limiting".to_owned(),
            },
            News::RoadmapComplete {
                roadmap: "rate-limiting".to_owned(),
            },
            News::Done,
        ]
    }

    /// And everything that is not about a piece of work, which is the other way
    /// in — see [`Word`].
    fn about_the_machine() -> Vec<Word> {
        vec![Word::ADeviceIsAsking {
            name: "laptop".to_owned(),
        }]
    }

    /// Every title a lock screen can show, from both ways in.
    ///
    /// One list rather than two, because there is one lock screen: a phone does
    /// not know which enum a notification came out of, and what has to hold is
    /// that the human can tell any of them from any other.
    fn every_title() -> Vec<String> {
        all()
            .iter()
            .map(|news| news.title("rate-limiting"))
            .chain(about_the_machine().iter().map(Word::title))
            .collect()
    }

    /// The whole job of a title is to say which of these it is, to somebody
    /// glancing at a lock screen with the app shut. Two of them reading alike is
    /// a phone that says only that *something* happened.
    #[test]
    fn no_two_notifications_read_alike() {
        let said = every_title();

        let mut titles = said.clone();
        titles.sort();
        titles.dedup();

        assert_eq!(
            titles.len(),
            said.len(),
            "two of these read the same: {said:?}"
        );
    }

    /// And each of them fits on a lock screen, which is where every one of them
    /// is read: a title the phone cuts off mid-sentence has thrown away whichever
    /// half was at the end of it.
    #[test]
    fn a_title_is_short_enough_to_be_shown_whole() {
        for title in every_title() {
            assert!(
                title.len() <= 80,
                "a title a lock screen would cut off mid-sentence: {title:?}",
            );
        }
    }

    /// A notification about the machine names the device asking, which is the
    /// whole of what the human is deciding about — and lands them on the pane
    /// the press is on rather than on a Conversation, there being none.
    #[test]
    fn a_device_asking_to_link_is_named_and_lands_on_the_remote_access_pane() {
        let asking = Word::ADeviceIsAsking {
            name: "laptop".to_owned(),
        };

        assert!(
            asking.title().contains("laptop"),
            "the device asking is what the title is for: {:?}",
            asking.title(),
        );
        assert_eq!(asking.path(), "/settings/remote");
    }

    /// And a device that calls itself something enormous is cut to fit rather
    /// than allowed to be the whole of a lock screen.
    ///
    /// The one title here whose substance a stranger writes: a join comes from a
    /// device this one holds no membership for, so the name in it is whatever
    /// that device said. Every other title on this enum is about work this
    /// machine is doing.
    #[test]
    fn a_name_a_stranger_chose_is_cut_to_fit_a_lock_screen() {
        let title = Word::ADeviceIsAsking {
            name: "l".repeat(400),
        }
        .title();

        assert!(
            title.chars().count() <= 80,
            "a title a lock screen would cut off mid-sentence: {title:?}",
        );
        assert!(
            title.ends_with("is asking to link with this device"),
            "the sentence has to survive the cutting: {title:?}",
        );
    }

    /// And a name that fits goes in whole, ellipsis and all left off.
    #[test]
    fn a_name_that_fits_is_left_alone() {
        assert_eq!(
            Word::ADeviceIsAsking {
                name: "laptop".to_owned(),
            }
            .title(),
            "laptop is asking to link with this device",
        );
    }

    /// A member's news reads as a member's: the device in front of the sentence
    /// it sent, and the sentence itself left as the machine that wrote it wrote
    /// it.
    #[test]
    fn a_members_news_leads_with_the_device_it_came_from() {
        assert_eq!(
            relayed("the-laptop", "pwa-and-push is done"),
            "the-laptop — pwa-and-push is done",
        );
    }

    /// And a title about this device's own work carries no device, which is the
    /// other half of that: what says a notification is a member's is that
    /// something is in front of the sentence, so nothing may be in front of a
    /// local one.
    #[test]
    fn this_devices_own_news_has_no_device_in_front_of_it() {
        for title in all().iter().map(|news| news.title("pwa-and-push")) {
            assert!(
                !title.contains(" — "),
                "a local title reading like a relayed one: {title:?}",
            );
        }
    }

    /// And a member that calls itself something enormous is cut to fit rather
    /// than allowed to be the whole of a lock screen — the same bound a device
    /// asking to link is held to, and for the same reason: the name is a string
    /// another machine wrote.
    #[test]
    fn a_members_name_is_cut_to_fit_a_lock_screen() {
        let title = relayed(&"l".repeat(400), "pwa-and-push is done");

        assert!(
            title.starts_with(&format!("{}…", "l".repeat(40))),
            "the name has to be cut rather than carried whole: {title:?}",
        );
        assert!(
            title.ends_with("— pwa-and-push is done"),
            "the sentence has to survive the cutting: {title:?}",
        );
    }

    /// And so is a sentence that is not one. It is the other half of a relayed
    /// title another machine wrote, so it is bounded for that reason rather than
    /// trusted to be a title.
    #[test]
    fn a_sentence_a_member_sent_is_cut_to_fit_a_lock_screen() {
        let title = relayed("the-laptop", &"s".repeat(4000));

        assert!(
            title.chars().count() <= 40 + " — ".chars().count() + 80 + 1,
            "a relayed title with no bound on the sentence in it: {}",
            title.chars().count(),
        );
        assert!(
            title.starts_with("the-laptop — "),
            "the device still leads it: {title:?}",
        );
        assert!(
            title.ends_with('…'),
            "a sentence that was cut says so: {title:?}",
        );
    }

    /// A relayed title of the two things they really are — a hostname and one of
    /// the titles above — fits the lock screen every other title here is written
    /// to.
    #[test]
    fn a_relayed_title_of_real_words_is_shown_whole() {
        for said in all().iter().map(|news| news.title("pwa-and-push")) {
            let title = relayed("the-laptop", &said);

            assert!(
                title.chars().count() <= 80,
                "a title a lock screen would cut off mid-sentence: {title:?}",
            );
        }
    }

    #[test]
    fn a_signature_is_one_a_push_service_can_check_against_the_public_key() {
        let secret = p256::SecretKey::random(&mut OsRng);
        let private_key = URL_SAFE_NO_PAD.encode(secret.to_bytes());

        let token = signed(&private_key, br#"{"aud":"https://push.example"}"#).unwrap();

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT is three parts: {token}");
        assert_eq!(
            URL_SAFE_NO_PAD.decode(parts[0]).unwrap(),
            JWT_HEADER.as_bytes(),
        );

        let signature =
            p256::ecdsa::Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap())
                .expect("the signature has to be the 64-byte r‖s form JWS defines");

        // Through the encoding the browser was handed, because that is the key
        // the push service checks against: the `k=` half of the header.
        let handed_out = secret.public_key().to_encoded_point(false);
        let verifying = p256::ecdsa::VerifyingKey::from_sec1_bytes(handed_out.as_bytes()).unwrap();

        verifying
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("the push service has to be able to check the signature");
    }

    #[test]
    fn a_signature_is_made_out_to_the_push_service_and_not_to_the_device() {
        let endpoint = "https://push.example/devices/abc123?token=xyz"
            .parse()
            .unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "https://push.example");
    }

    #[test]
    fn a_push_service_on_an_unusual_port_is_a_different_audience() {
        let endpoint = "http://127.0.0.1:8422/devices/abc123".parse().unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "http://127.0.0.1:8422");
    }
}
