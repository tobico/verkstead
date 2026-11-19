//! The host side of the `activity` world: what Verkstead lends whatever drives a
//! Conversation, in the shape `wit/plugin.wit` gives it (ADR-0030, *Activity*).
//!
//! **One way to launch a session for a Conversation, and one way to wait on
//! it.** [`Lent`] implements the trait `bindgen!` generates for the world's
//! imports, and every driver in [`crate::runner`], the grilling's first launch
//! and its relaunch included, starts and waits on its sessions through it. A
//! component hosted later reaches the server through the same calls, so the
//! interface is exercised by the built-ins long before a stranger's Activity
//! needs it.
//!
//! **The driver writes the prompt and the host adds the rest.** What a session
//! is told about its own work is the caller's, built with the same
//! [`crate::skills`] builder it always was; what every session is told —
//! Memos, previews, packs, companions, transfer, naming — is
//! [`crate::sessions::Sessions::start`]'s, and the Answers folded from Deferred
//! Asks are added here, under it all, as they always were.
//!
//! **What only an in-process driver can say is said to the [`Lent`] itself**,
//! rather than widened into the world: whether the caller already holds the
//! Turn, the Sets a relaunched grilling holds open, the building kind a
//! session's Memos are left under, and what the rescue says the session is
//! doing. And what a wait came to is read back in full — how a session ended,
//! and the Event it printed into — through [`Lent::see`] and
//! [`Lent::event_id`], where the world's `waited` says only *gone*.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use verkstead_schema::Nudge;
use verkstead_store as store;

use crate::AppState;
use crate::done::{Ends, Evidence};
use crate::memos::Building;
use crate::plugins::component::worlds::category::activity::verkstead::plugin::{
    activity_types as world, host,
};
use crate::runner::Landing;
use crate::sessions::{Ended, Session};

/// What the host lends one Conversation: its sessions, and what an in-process
/// driver has said about the next of them.
pub(crate) struct Lent {
    state: AppState,
    conversation_id: i64,

    /// Whether a launch waits for the Turn and asks after a stop, or the caller
    /// is holding the Turn already — see [`Lent::turn_held`].
    turn: Turn,

    /// The Sets a relaunched grilling leaves open rather than locking.
    held: Vec<i64>,

    /// What a session launched here is building, where it is — which is what
    /// says it may retire a Memo, and what one it leaves is left under.
    building: Option<Building>,

    /// What the rescue says the session is doing, where it goes idle without a
    /// word.
    doing: String,

    /// Every session launched or adopted here, by the handle the world names it
    /// by.
    watched: HashMap<world::Session, Watched>,
    issued: world::Session,

    /// The Timeline Event the newest session launched here prints into, which
    /// is what a stop asked for through the world carries the tail of.
    writing: Option<i64>,
}

/// Whether a launch waits for the Conversation's Turn.
///
/// Every launch takes it but the ones whose caller already holds it — a fix
/// session and the review, both dispatched by a wrap-up that took it before it
/// decided anything — and a grilling's, which nothing else is driving yet.
///
/// The wait is what keeps a launch from displacing a session somebody else put
/// there. [`crate::sessions::Sessions::start`] ends whatever is registered, which
/// is exactly what a run relaunching its own step wants and exactly what must
/// not happen to a session a steer set going — the human steers in a quiet
/// moment between steps, and a run that reached the next one a second later
/// would kill it mid-sentence. A steer holds the Turn for as long as its session
/// runs, so this waits for it rather than ending it.
///
/// Held across the launch alone rather than across the session, because that is
/// the whole of what it is protecting: once a session is registered, everything
/// else that might start one can see it there.
///
/// And a stop lands here, which is why it is asked after the wait rather than
/// before: *nothing new starts* has to be true of the moment a session would be
/// started, and the wait is however long the session in front of this one took.
/// Every launch a run makes goes through here, so this is the one place that can
/// say it of all of them — which is why what it asks is
/// [`crate::stopping::stopped`] and not [`crate::stops::asked`] alone. A press that
/// is still waiting is only one of the two ways a run stops: a Force stop writes
/// its stop outright, and so does a Stop pressed in one of the quiet moments
/// between sessions. A launch that looked for the waiting press alone would walk
/// straight past both of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Take,
    Held,
}

/// One session the host is watching, and what its Done signal is checked
/// against while it is waited on.
struct Watched {
    session: Session,
    evidence: Evidence,
    ends: Ends,

    /// How it ended, once a wait has seen it end.
    ended: Option<Ended>,
}

impl Lent {
    /// The host lent to `conversation_id`'s driver: a launch takes the Turn,
    /// nothing is held, nothing is building, and the rescue says the session is
    /// working.
    pub(crate) fn new(state: &AppState, conversation_id: i64) -> Lent {
        Lent {
            state: state.clone(),
            conversation_id,
            turn: Turn::Take,
            held: Vec::new(),
            building: None,
            doing: "working".to_owned(),
            watched: HashMap::new(),
            issued: 0,
            writing: None,
        }
    }

    /// The caller holds the Turn already — a wrap-up takes it before it decides
    /// what to dispatch — or, a grilling, is the only thing that could be
    /// starting a session: a launch neither waits for it nor asks after a stop.
    pub(crate) fn turn_held(mut self) -> Lent {
        self.turn = Turn::Held;
        self
    }

    /// Leave `sets` open across the launch rather than locking them — a
    /// grilling relaunched over an arrival. See [`crate::sessions::Held`].
    pub(crate) fn holding(mut self, sets: Vec<i64>) -> Lent {
        self.held = sets;
        self
    }

    /// What the sessions launched here are building, where they are.
    ///
    /// Two rules hang off it, and they are the one rule: a session that builds
    /// is the one told how to retire a Memo, and the kind is what a Memo it
    /// leaves is left under — see [`crate::memos`].
    pub(crate) fn building(mut self, building: Option<Building>) -> Lent {
        self.building = building;
        self
    }

    /// What the rescue says the session is doing — "following the work up".
    pub(crate) fn doing(mut self, doing: &str) -> Lent {
        self.doing = doing.to_owned();
        self
    }

    /// Launch a session through the world's *launch a session*, saying nothing
    /// of a refusal beyond what [`host::Host::launch`] has already logged.
    pub(crate) async fn start(
        &mut self,
        skill: &str,
        role: store::Role,
        prompt: String,
        evidence: Evidence,
        ends: Ends,
    ) -> Option<world::Session> {
        let launched = host::Host::launch(
            self,
            skill.to_owned(),
            role_named(role).to_owned(),
            prompt,
            lent_evidence(&evidence),
            lent_ends(ends),
        )
        .await;

        match launched {
            Ok(Ok(session)) => Some(session),
            Ok(Err(_)) | Err(_) => None,
        }
    }

    /// Watch a session something else launched — the grilling, whose tail is
    /// seen out once the human has picked — as though it had been launched
    /// here, expecting `evidence` of it.
    pub(crate) fn adopt(
        &mut self,
        session: Session,
        evidence: Evidence,
        ends: Ends,
    ) -> world::Session {
        self.watch(session, evidence, ends)
    }

    /// The Timeline Event `session` prints into.
    pub(crate) fn event_id(&self, session: world::Session) -> i64 {
        self.watched
            .get(&session)
            .map(|watched| watched.session.event_id)
            .expect("a session is only asked after by the handle it was launched under")
    }

    /// Wait on `session` through the world's *wait*, and say how it ended —
    /// `None` where it gave its Done signal, had it taken, and has been idle
    /// since.
    pub(crate) async fn see(&mut self, session: world::Session) -> Option<Ended> {
        match host::Host::wait(self, session).await {
            Ok(world::Waited::Done) => None,
            _ => Some(
                self.watched
                    .get(&session)
                    .and_then(|watched| watched.ended.clone())
                    .unwrap_or(Ended::Unknown),
            ),
        }
    }

    fn watch(&mut self, session: Session, evidence: Evidence, ends: Ends) -> world::Session {
        self.issued += 1;
        self.writing = Some(session.event_id);

        self.watched.insert(
            self.issued,
            Watched {
                session,
                evidence,
                ends,
                ended: None,
            },
        );

        self.issued
    }

    /// What a session that ended without its Done signal taken is to the one
    /// waiting on it.
    ///
    /// **Ended on purpose, with no stop standing, is the human taking the
    /// Conversation elsewhere** — a steer ends the session it moves past, and
    /// so does a transfer, after which the work carries on at the far end —
    /// so it is `steered`, and whatever was waiting has nothing left to do.
    /// Under a stop it is `gone` with nothing said, the stop's own Notice being
    /// the whole of it; otherwise `gone` carries how it ended, where that was
    /// the trouble — see [`Ended::badly`].
    ///
    /// **And a session whose rounds the human had already marked over is
    /// `marked`**, where its evidence is that mark: an agent that finishes its
    /// turn and exits before the grace runs out is the ordinary shape of a
    /// follow-up that is over, rather than one nobody is left to have. A Set
    /// still standing is asked about first — the human holding a question is
    /// worth a stop whatever the newest answer said — and both read the safe
    /// way round: a store that will not answer reads as open and not marked.
    async fn gone(&self, ended: &Ended, evidence: &Evidence) -> world::Waited {
        let state = &self.state;
        let conversation_id = self.conversation_id;

        if ended.on_purpose() {
            return match store::stopped(&state.pool, conversation_id).await {
                Ok(None) => world::Waited::Steered,
                Ok(Some(_)) => world::Waited::Gone(None),
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, "reading whether a Conversation had stopped under its session failed");
                    world::Waited::Gone(None)
                }
            };
        }

        let within = match evidence {
            Evidence::NothingElse => store::Lifecycle::FollowUp,
            Evidence::Investigated => store::Lifecycle::Investigating,
            _ => return world::Waited::Gone(ended.badly()),
        };

        if !crate::runner::open(state, conversation_id).await
            && crate::runner::marked(state, conversation_id, within).await
        {
            return world::Waited::Marked;
        }

        world::Waited::Gone(ended.badly())
    }

    /// Whatever a launch is refused over is logged here, where the reason is
    /// known, and handed back as the sentence the world refuses with.
    fn refused(&self, why: &str) -> Result<world::Session, String> {
        tracing::error!(
            conversation_id = self.conversation_id,
            why,
            "no session was started"
        );

        Err(why.to_owned())
    }
}

impl world::Host for Lent {}

impl host::Host for Lent {
    /// Start a session the way every driver always started one: under the
    /// Pairing the role names, once the Turn is the caller's and nothing has
    /// stopped the run, with the Answers nobody has been told about folded under
    /// the prompt and the session before it ended — see [`crate::runner`]'s
    /// drivers, which say why each of those holds.
    async fn launch(
        &mut self,
        skill: String,
        role: String,
        prompt: String,
        evidence: world::Evidence,
        ends: world::Ends,
    ) -> wasmtime::Result<Result<world::Session, String>> {
        let state = self.state.clone();
        let conversation_id = self.conversation_id;

        if !named_in_a_pack(&skill) {
            return Ok(self.refused(&format!(
                "`{skill}` isn't a skill named as `<pack>/<skill>`."
            )));
        }

        let Some(role) = role_called(&role) else {
            return Ok(self.refused(&format!("There's no role called `{role}`.")));
        };

        // See [`Turn`]: held across the launch alone, and a stop asked after
        // the wait for it rather than before.
        let _turn = match self.turn {
            Turn::Take => {
                let turn = state.sessions.turn(conversation_id).await;

                if crate::stopping::stopped(&state, conversation_id).await {
                    return Ok(Err("The run was stopped, so nothing new starts.".to_owned()));
                }

                Some(turn)
            }
            Turn::Held => None,
        };

        // Read back every time rather than held: where an agent is about to be
        // let loose is the one thing that must not be guessed at.
        let conversation = match store::load_conversation(&state.pool, conversation_id).await {
            Ok(Some(conversation)) => conversation,
            Ok(None) => return Ok(self.refused("There is no Conversation left to work in.")),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the Conversation to work in failed");
                return Ok(self.refused("The Conversation couldn't be read."));
            }
        };

        let paired = match role {
            store::Role::Grilling => conversation.grilling_pairing.clone(),
            role => crate::runner::under(
                role,
                &conversation.review_pairing,
                conversation.implementation_pairing.as_ref(),
            ),
        };

        let Some(pairing) = paired else {
            return Ok(self.refused(&format!(
                "The Pairing the {} role runs under is gone.",
                role_named(role)
            )));
        };

        let Some(evidence) = taken_evidence(evidence, conversation.worktree.as_deref()) else {
            return Ok(self.refused("The Conversation has no Worktree left to land anything in."));
        };

        // And whatever the human has since answered on this Conversation's
        // Deferred Asks that no session has been told about — under everything
        // above. Never into a grilling, which is primed with every Set already:
        // see [`crate::deferrals`].
        let folding = match role {
            store::Role::Grilling => None,
            _ => Some(crate::deferrals::unfolded(&state, conversation_id).await),
        };

        let prompt = match &folding {
            Some(folding) => folding.under(&prompt),
            None => prompt,
        };

        // One Worktree holds one agent: a register still holding a relay that has
        // not finished unwinding would be two agents editing each other's files.
        state.sessions.end(conversation_id).await;

        let started = state
            .sessions
            .start(
                &state.pool,
                &state.nudges,
                state.devices.as_ref(),
                &conversation,
                &pairing,
                &prompt,
                self.building.is_some(),
                crate::sessions::Held {
                    sets: &self.held,
                    settlements: &state.settlements,
                },
            )
            .await;

        match started {
            Ok(Some(session)) => {
                // Once there is a session reading them, and only then: a launch
                // that came to nothing would otherwise cost the human the one
                // session their Answers were folded into.
                if let Some(folding) = folding {
                    folding.recorded(&state.pool).await;
                }

                Ok(Ok(self.watch(session, evidence, taken_ends(ends))))
            }
            Ok(None) => Ok(self.refused("The session couldn't be started.")),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "a session could not be started");
                Ok(self.refused("The session couldn't be started."))
            }
        }
    }

    /// Wait until the session gives its Done signal, has it taken and is idle
    /// since, or ends — what its signal is checked against written down for
    /// exactly as long as it is waited on, and taken away with the wait if the
    /// driver is cancelled. A session that goes idle with nothing open and no
    /// signal given is spoken to, and the human told where it will not answer:
    /// see [`crate::rescues`]. Never an ending of its own. A session that ends
    /// is `gone`, `steered` or `marked` as [`Lent::gone`] reads it.
    async fn wait(&mut self, session: world::Session) -> wasmtime::Result<world::Waited> {
        let conversation_id = self.conversation_id;

        let Some(watched) = self.watched.get_mut(&session) else {
            return Err(wasmtime::Error::msg(format!(
                "there is no session {session} to wait on"
            )));
        };

        if let Some(ended) = &watched.ended {
            return Ok(world::Waited::Gone(ended.badly()));
        }

        let event_id = watched.session.event_id;
        let idle = watched.session.idle.clone();
        let pace = self.state.sessions.pace();

        let expecting = self.state.signals.expecting(
            conversation_id,
            event_id,
            watched.evidence.clone(),
            watched.ends,
            self.building,
        );
        let signal = expecting.signal();

        let ended = tokio::select! {
            ended = watched.session.ended() => Some(ended),
            () = crate::runner::signalled_and_idle(signal.clone(), &idle, pace) => None,
            never = crate::rescues::watched(
                &self.state,
                conversation_id,
                event_id,
                &idle,
                pace,
                signal,
                &self.doing,
            ) => match never {},
        };

        drop(expecting);

        match ended {
            Some(ended) => {
                let evidence = watched.evidence.clone();
                watched.ended = Some(ended.clone());

                Ok(self.gone(&ended, &evidence).await)
            }
            None => Ok(world::Waited::Done),
        }
    }

    /// What the Conversation's own branch, or a Companion's, holds past its
    /// base, and whether anything in its Worktree is uncommitted — read-only, a
    /// rename followed first, as [`crate::runner`] reads its own branch.
    async fn git_facts(
        &mut self,
        repo: String,
    ) -> wasmtime::Result<Result<world::BranchFacts, String>> {
        let state = &self.state;
        let conversation_id = self.conversation_id;

        let conversation = match store::load_conversation(&state.pool, conversation_id).await {
            Ok(Some(conversation)) => conversation,
            Ok(None) => return Ok(Err("There is no Conversation left to read.".to_owned())),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the Conversation for its git facts failed");
                return Ok(Err("The Conversation couldn't be read.".to_owned()));
            }
        };

        let (path, worktree, branch, base) = if conversation.repo.name == repo {
            let branch = match conversation.worktree.as_deref() {
                Some(worktree) => crate::renames::follow(
                    &state.pool,
                    conversation_id,
                    &conversation.repo.path,
                    worktree,
                    &conversation.branch,
                )
                .await
                .unwrap_or_else(|| conversation.branch.clone()),
                None => conversation.branch.clone(),
            };

            (
                conversation.repo.path.clone(),
                conversation.worktree.clone(),
                branch,
                conversation.base_commit.clone(),
            )
        } else if let Some(companion) = conversation
            .companions
            .iter()
            .find(|companion| companion.repo.name == repo)
        {
            (
                companion.repo.path.clone(),
                companion.worktree.clone(),
                companion
                    .branch_for(&conversation.branch)
                    .unwrap_or_else(|| companion.branch.clone()),
                companion.base_commit.clone(),
            )
        } else {
            return Ok(Err(format!(
                "`{repo}` is neither this Conversation's repo nor one of its companions."
            )));
        };

        let (Some(worktree), Some(base)) = (worktree, base) else {
            return Ok(Err(format!(
                "`{repo}` has no Worktree or no base commit to read against."
            )));
        };

        let read =
            tokio::task::spawn_blocking(move || read_branch(&path, &worktree, &base, branch)).await;

        Ok(match read {
            Ok(Some(facts)) => Ok(facts),
            Ok(None) | Err(_) => Err(format!("git wouldn't say what `{repo}`'s branch holds.")),
        })
    }

    /// Put `text` on the Timeline as a Notice, and tell open pages it is there.
    async fn notice(&mut self, text: String) -> wasmtime::Result<()> {
        let conversation_id = self.conversation_id;

        match store::note(&self.state.pool, conversation_id, &text).await {
            Ok(true) => self.state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            }),
            Ok(false) => {
                tracing::error!(
                    conversation_id,
                    "there is no Conversation left to put a notice on"
                )
            }
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "putting a notice on the Timeline failed")
            }
        }

        Ok(())
    }

    /// Stop where the Conversation stands, closing any Set left open first, with
    /// `reason` as what the stop's Notice says happened and the newest
    /// session's last words as its evidence.
    async fn stop(&mut self, reason: String) -> wasmtime::Result<()> {
        crate::runner::left_open(&self.state, self.conversation_id).await;

        crate::runner::stop(
            &self.state,
            self.conversation_id,
            crate::stopping::Decided::Verkstead,
            &self.doing,
            &reason,
            self.writing,
        )
        .await;

        Ok(())
    }
}

/// The branch's facts, by git, off the runtime's threads. `None` where git
/// would not answer.
fn read_branch(
    repo: &Path,
    worktree: &Path,
    base: &str,
    branch: String,
) -> Option<world::BranchFacts> {
    let counted = crate::repos::git(
        repo,
        &[
            "rev-list",
            "--count",
            "--end-of-options",
            &format!("{base}..{branch}"),
        ],
    )?;

    let commits = counted.trim().parse().ok()?;
    let uncommitted = !crate::diffs::changed(worktree)?.is_empty();

    Some(world::BranchFacts {
        branch,
        commits,
        uncommitted,
    })
}

/// Whether `skill` is named `<pack>/<skill>`, as stage 07 names every skill.
fn named_in_a_pack(skill: &str) -> bool {
    skill
        .split_once('/')
        .is_some_and(|(pack, name)| !pack.is_empty() && !name.is_empty() && !name.contains('/'))
}

/// A role as the world spells it, which is as the store does.
pub(crate) fn role_named(role: store::Role) -> &'static str {
    match role {
        store::Role::Grilling => "grilling",
        store::Role::Implementation => "implementation",
        store::Role::Review => "review",
    }
}

fn role_called(role: &str) -> Option<store::Role> {
    [
        store::Role::Grilling,
        store::Role::Implementation,
        store::Role::Review,
    ]
    .into_iter()
    .find(|named| role_named(*named) == role)
}

/// `evidence` in the world's words. A landing's Worktree goes unsaid: the host
/// reads it off the Conversation as the session is launched.
fn lent_evidence(evidence: &Evidence) -> world::Evidence {
    match evidence {
        Evidence::Landed { landing, .. } => world::Evidence::Landed(match landing {
            Landing::Gone(path) => world::Landing::Gone(path.to_string_lossy().into_owned()),
            Landing::Arrived(path) => world::Landing::Arrived(path.to_string_lossy().into_owned()),
            Landing::Ticked(number) => world::Landing::Ticked(*number),
            Landing::Roadmap(base) => world::Landing::Roadmap(base.clone()),
            Landing::Handoff(path) => world::Landing::Handoff(path.to_string_lossy().into_owned()),
        }),
        Evidence::Committed { already } => world::Evidence::CommittedSince(*already),
        Evidence::Nothing => world::Evidence::Nothing,
        Evidence::NothingElse => world::Evidence::NothingElse,
        Evidence::Investigated => world::Evidence::Investigated,
    }
}

/// The world's evidence as [`crate::done`] checks it, a landing in `worktree` —
/// `None` where it is a landing and there is no Worktree to land it in.
fn taken_evidence(evidence: world::Evidence, worktree: Option<&Path>) -> Option<Evidence> {
    Some(match evidence {
        world::Evidence::Landed(landing) => Evidence::Landed {
            worktree: worktree?.to_owned(),
            landing: match landing {
                world::Landing::Gone(path) => Landing::Gone(PathBuf::from(path)),
                world::Landing::Arrived(path) => Landing::Arrived(PathBuf::from(path)),
                world::Landing::Ticked(number) => Landing::Ticked(number),
                world::Landing::Roadmap(base) => Landing::Roadmap(base),
                world::Landing::Handoff(path) => Landing::Handoff(PathBuf::from(path)),
            },
        },
        world::Evidence::CommittedSince(already) => Evidence::Committed { already },
        world::Evidence::Nothing => Evidence::Nothing,
        world::Evidence::NothingElse => Evidence::NothingElse,
        world::Evidence::Investigated => Evidence::Investigated,
    })
}

fn lent_ends(ends: Ends) -> world::Ends {
    match ends {
        Ends::WithItsWork => world::Ends::WithItsWork,
        Ends::OnAPullRequest => world::Ends::OnAPullRequest,
    }
}

fn taken_ends(ends: world::Ends) -> Ends {
    match ends {
        world::Ends::WithItsWork => Ends::WithItsWork,
        world::Ends::OnAPullRequest => Ends::OnAPullRequest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_is_named_in_a_pack() {
        assert!(named_in_a_pack("verkstead/following-up"));
        assert!(!named_in_a_pack("following-up"));
        assert!(!named_in_a_pack("verkstead/"));
        assert!(!named_in_a_pack("/following-up"));
        assert!(!named_in_a_pack("verkstead/following-up/SKILL.md"));
    }

    #[test]
    fn a_role_is_called_what_the_store_calls_it() {
        for role in [
            store::Role::Grilling,
            store::Role::Implementation,
            store::Role::Review,
        ] {
            assert_eq!(role_called(role_named(role)), Some(role));
        }

        assert_eq!(role_called("reviewer"), None);
    }

    /// Every piece of evidence a driver registers crosses the world and comes
    /// back as it went, its landing in the Worktree the host reads.
    #[test]
    fn evidence_crosses_the_world_and_back() {
        let worktree = Path::new("/work");

        for evidence in [
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Ticked(3),
            },
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Arrived(PathBuf::from(".tasks/TODO.md")),
            },
            Evidence::Landed {
                worktree: worktree.to_owned(),
                landing: Landing::Handoff(PathBuf::from("/data/handoffs/4.md")),
            },
            Evidence::Committed { already: 7 },
            Evidence::Nothing,
            Evidence::NothingElse,
            Evidence::Investigated,
        ] {
            let crossed = taken_evidence(lent_evidence(&evidence), Some(worktree)).unwrap();

            assert_eq!(format!("{crossed:?}"), format!("{evidence:?}"));
        }

        assert!(taken_evidence(world::Evidence::Landed(world::Landing::Ticked(1)), None).is_none());
    }
}
