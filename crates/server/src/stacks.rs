//! The **stack** a Fix Merge Issues was pointed into the middle of: GitHub's
//! chain walked both ways from the pull request that was taken up, every link
//! recorded beside it, and what was found said on the Timeline.
//!
//! A stack is a chain of open pull requests in one repository, each based on the
//! one below's head branch. Verkstead did not make it — the human's own work
//! stacks, `docs/agents/git-workflow.md` says how — and a conflict low in one
//! moves every branch above it, so a wrap-up pointed at any of them is a wrap-up
//! over all of them.
//!
//! **Read through `gh` rather than through `gh stack`.** That extension keeps
//! its registry per worktree and the worktree a take-up makes is minutes old, so
//! there is nothing in it to read; the chain itself is plain GitHub — an open
//! pull request's base is another open pull request's head — and one
//! `gh pr list` is the whole of what it takes to assemble. See
//! [`crate::github::open_pull_requests`].
//!
//! **The neighbours are recorded without being claimed.** The pull request the
//! Conversation was pointed at is the one it is on: its head is the branch, its
//! Worktree is where the work happens, and it is the one
//! [`store::conversation_on_pull_request`] answers with. The rest are recorded so
//! that they are watched — the wrap-up waits on Mergeable and the checks for
//! every recorded pull request — and in this workbench they usually belong to a
//! Conversation each, that being what a stacked stage is. So a row recorded
//! beside the work is no claim on it, and the note below says which of the stack
//! somebody else holds, that being a thing to be told rather than to find out
//! later.
//!
//! **But whoever is *standing* on a link has to make way, by the rule the pull
//! request being taken up follows.** One open Conversation per pull request — see
//! ADR-0020 — and a stack is synced by rebasing and force-pushing every branch of
//! it, which git refuses over a branch that is checked out anywhere else. Every
//! link of a stack Verkstead built is a stage Conversation's, usually Done, and
//! every one of those keeps its Worktree. So each Done Conversation on the chain
//! is closed, a Closed or Archived one has nothing to give up, and one still at
//! work refuses the start naming it — all or nothing, and before anything at all
//! is closed, a sync that cannot move one branch being a sync that moves none.
//! Which is why the chain is walked before the take-up records or closes
//! anything: see [`walk`], which is the reading, and
//! [`crate::conversations::take_up`], which decides on it.
//!
//! **And a checkout that is no Conversation's is refused over rather than worked
//! round.** The human's own clone of the stage below is not something Verkstead
//! may close, so the take-up says where it is, the way it already does about the
//! branch it was pointed at.
//!
//! **A chain that leaves the Repo is not followed.** A head in a fork is not a
//! branch this Conversation could ever push a fix to, which is the refusal the
//! pull request it was pointed at already gets — see
//! [`crate::conversations::resolve`] — so a fork is no link at all and the chain
//! stops where one would be.
//!
//! **And it is recorded where the pull request is**, which is two doors rather
//! than one. A take-up over a pull request records it at the press and walks
//! from there — the walk ahead of the press's own record, the recording after it.
//! A take-up over a bare branch records nothing, sends one `submitting` session
//! for the pull request nobody opened, and both halves run where *that* is
//! recorded — the same walk, a few minutes later, with a note of its own because
//! the take-up's was written before there was anything to walk from.
//!
//! **Nothing can be asked of the human at the second door**, which is the one way
//! the two differ. At the press a neighbour holding uncommitted changes stops the
//! press to be confirmed; minutes later there is nobody standing there, so it
//! stops the run with a Notice naming it, exactly as one still at work does. See
//! [`crate::conversations::neighbours_give_way`].
//!
//! **And what syncs a stack is asked for before anything is sent at one.** A
//! conflict anywhere in a recorded stack dispatches one session told to run
//! `gh stack sync` — see [`crate::checks::resolve`] — and that extension is a
//! separate install of a `gh` running under a home of Verkstead's own. So it is
//! asked for in the environment a session gets, and a Sandbox without it stops
//! the run with a Notice naming it rather than spending the stack's goes on
//! sessions that cannot do what they were told. See [`missing`].

use crate::AppState;
use crate::github::{self, Numbered};
use crate::store;

/// The `gh` extension a stack is synced with, as `gh` is asked for it and as
/// the Timeline names it.
///
/// One word here because it is two things in one: the sub-command `gh` is asked
/// to print the usage of, and the name a human reads off a Notice that says it
/// is missing.
pub(crate) const EXTENSION: &str = "stack";

/// And how a human installs it, which is the other half of that Notice: a stop
/// that named what was missing and not how to get it would be a stop somebody
/// had to go and look something up for.
pub(crate) const INSTALL: &str = "gh extension install github/gh-stack";

/// Whether a session sent at this Conversation's stack would find `gh stack` —
/// `None` where it would, and the reason in words where it would not.
///
/// **Asked before a session is sent rather than found in one's failure.** The
/// extension is a separate install and the wrap-up has never driven it, so the
/// first thing a stack session would do is the first thing that could go
/// missing — and a go spent on a session that cannot do what it was told is a
/// go the human paid for and got nothing from. Two of those and the run would
/// stop over a conflict nothing had actually tried to resolve.
///
/// **In the environment a session gets rather than the server's own**, which is
/// the only reason the answer is worth having: an extension lives inside the
/// home `gh` is run under, and a session's home is Verkstead's own. See
/// [`crate::sessions::Sessions::session_environment`], which composes it, and
/// [`github::Gh::extension`], which runs it.
///
/// A server that runs no session answers as though the extension were there:
/// there is nothing here to equip, so there is nothing for this to be in front
/// of, and a stop written by a server that dispatches nothing would be a Notice
/// about a session that was never going to start.
pub(crate) async fn missing(state: &AppState, conversation_id: i64) -> Option<String> {
    let environment = state.sessions.session_environment(conversation_id)?;

    // Off the runtime's threads, as every other reach for `gh` is: it is a
    // process, and running one blocks.
    let asked = tokio::task::spawn_blocking({
        let gh = state.github.clone();

        move || gh.extension(EXTENSION, &environment)
    })
    .await;

    match asked {
        Ok(Ok(())) => None,
        Ok(Err(trouble)) => Some(trouble.why()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh whether it has the stack extension failed");

            Some("Verkstead could not run `gh` to find out".to_owned())
        }
    }
}

/// What a walk of GitHub's chain found, which is what a take-up needs before it
/// records or closes anything.
///
/// **Read apart from recorded**, because the two happen at different moments
/// now. Knowing the links is what says who is standing on the chain, and that
/// has to be answered before the first Conversation is closed; recording them is
/// what makes the wrap-up wait on them, and that belongs after the take-up's own
/// pull request is on the record. See [`walk`] and [`record`].
pub(crate) enum Walked {
    /// Nothing to walk: a Process that does not walk a chain, or a Conversation
    /// with no pull request recorded to walk from.
    Nothing,

    /// The chain from the bottom, the pull request walked from included. Fewer
    /// than two links is a lone pull request, which is the ordinary case and
    /// reads exactly as it read before there were stacks.
    Chain(Vec<Numbered>),

    /// GitHub would not list the repository's pull requests, so whether there is
    /// a chain at all is not known — in `gh`'s own words.
    Unread(String),
}

impl Walked {
    /// Every link of the chain but `own`, from the bottom: the neighbours.
    ///
    /// Which are the pull requests nobody pressed anything about and the ones a
    /// sync would move all the same — so they are what is asked to make way, and
    /// what is recorded to be watched.
    pub(crate) fn neighbours(&self, own: i64) -> Vec<&Numbered> {
        let Self::Chain(chain) = self else {
            return Vec::new();
        };

        if chain.len() < 2 {
            return Vec::new();
        }

        chain.iter().filter(|link| link.number != own).collect()
    }
}

/// Walk GitHub's chain both ways from `own`, and hand back what it is.
///
/// **This is the reading and nothing else.** Nothing is recorded, nothing is
/// closed and nothing is refused for: what a caller does about the neighbours is
/// its own, and a take-up has to know them before it may do any of it — git will
/// not move a branch that is checked out elsewhere, and a stack sync moves every
/// branch of the chain.
///
/// One `gh pr list` is the whole of it: a stack is plain GitHub, an open pull
/// request's base being another open pull request's head. See
/// [`github::open_pull_requests`] and [`chain`].
pub(crate) async fn walk(state: &AppState, repo: &store::Repo, own: i64) -> Walked {
    // Off the runtime's threads, as every other reach into GitHub is: `gh` is a
    // process, and running one blocks.
    let asked = tokio::task::spawn_blocking({
        let gh = state.github.clone();
        let path = repo.path.clone();

        move || github::open_pull_requests(&gh, &path)
    })
    .await;

    let open = match asked {
        // Said rather than swallowed, for [`unread`]'s reason: nothing walks
        // again, so a silence here is a wrap-up quietly over one link of a chain
        // nobody can see it missed.
        Ok(Err(trouble)) => {
            tracing::warn!(
                repo = repo.name,
                why = trouble.why(),
                "the Repo's open pull requests could not be listed, so nothing was walked",
            );

            return Walked::Unread(trouble.why());
        }
        Err(error) => {
            tracing::error!(error = ?error, repo = repo.name, "asking gh for the Repo's open pull requests failed");

            return Walked::Unread("Verkstead could not run `gh` to find out".to_owned());
        }
        Ok(Ok(open)) => open,
    };

    Walked::Chain(chain(&open, own).into_iter().cloned().collect())
}

/// The chain to walk for a Conversation that has a pull request recorded
/// already — which Repo it is in, which pull request the walk starts from, and
/// what the walk found.
///
/// The second door's way in: a take-up over a bare branch records nothing at the
/// press, sends one `submitting` session for the pull request nobody opened, and
/// the walk runs where *that* is recorded — see [`crate::wrapping::record`].
///
/// `None` where there is nothing to walk at all: a Process that does not walk
/// one, or a Conversation with no pull request in its own repository yet.
pub(crate) async fn walking(
    state: &AppState,
    conversation_id: i64,
) -> Option<(store::Repo, i64, Walked)> {
    let pool = &state.pool;

    let conversation = match store::load_conversation(pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => {
            tracing::error!(
                conversation_id,
                "there is no Conversation left to walk a stack for"
            );
            return None;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation whose stack to walk failed");
            return None;
        }
    };

    if !crate::conversations::walks_the_stack(conversation.process) {
        return None;
    }

    let repo = conversation.repo.clone();

    // What the walk starts from, which is the pull request this Conversation was
    // pointed at: the first recorded in its own repository — see
    // [`store::pull_request`]. A wrap-up with none there yet is a bare branch
    // whose `submitting` session has not landed, and the walk runs when it does.
    let own = match store::pull_request(pool, conversation_id, repo.id).await {
        Ok(Some(own)) => own,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the pull request to walk a stack from failed");
            return None;
        }
    };

    let walked = walk(state, &repo, own.number).await;

    Some((repo, own.number, walked))
}

/// Record every link of the chain beside the one taken up, and say in words what
/// the stack is.
///
/// `None` where there is nothing to say: nothing was walked, or the walk found a
/// lone pull request — which is the ordinary case and reads exactly as it read
/// before there were stacks.
///
/// Called before the watchers start, which is what watches the neighbours: a
/// wrap-up starts one checks watcher and one comments watcher per *recorded*
/// pull request, so a chain recorded first is a chain watched without anything
/// here spawning a thing. See [`crate::wrapping::watching`].
///
/// Nothing is refused for. A stack that could not be read is a wrap-up over the
/// one pull request it was pointed at, which is what every Fix Merge Issues was
/// until this stage: the press it is answering has already succeeded, and there
/// is nothing here worth undoing it for.
///
/// **But it is said, rather than left to look like a lone pull request.** A
/// `gh` that would not answer and a pull request with nothing above or below it
/// leave the same record otherwise — the same silence on the same Timeline,
/// over a wrap-up that may be watching one link of three — and this runs where
/// a pull request is recorded and at no poll after it, so nothing later puts it
/// right. See [`unread`].
///
/// `closed` is every Conversation this take-up closed to make way, by id. The
/// note names them where it names the link they were standing on: a stack of
/// stages is a Conversation per link, and a human who pressed Start on one
/// Conversation and watched four of them move is owed the list.
pub(crate) async fn record(
    state: &AppState,
    conversation_id: i64,
    repo: &store::Repo,
    own: i64,
    walked: &Walked,
    closed: &[i64],
) -> Option<String> {
    let pool = &state.pool;

    let chain = match walked {
        Walked::Nothing => return None,
        Walked::Unread(why) => return Some(unread(own, why)),
        Walked::Chain(chain) => chain,
    };

    // A chain of one is a lone pull request, and a chain of none is a repository
    // whose list did not hold the pull request this Conversation is on. Both are
    // the wrap-up that was here before this stage, and neither is worth a word.
    if chain.len() < 2 {
        return None;
    }

    // Who holds each of them, asked before anything of this chain is recorded:
    // what the note is for is saying which of the stack is somebody else's work,
    // and a row written a line from now would make every one of them this
    // Conversation's own.
    let mut said = Vec::new();

    for link in walked.neighbours(own) {
        said.push((
            link.number,
            held(state, conversation_id, repo.id, link.number, closed).await,
        ));
    }

    for link in walked.neighbours(own) {
        let recorded = store::record_another_pull_request(
            pool,
            conversation_id,
            repo.id,
            &store::PullRequest {
                number: link.number,
                title: link.title.clone(),
                url: link.url.clone(),
                head: Some(link.head.clone()),
                base: Some(link.base.clone()),
                // Which repository it is in is the caller's to know, and this
                // one is the Conversation's own — see [`store::PullRequest`].
                repo: None,
            },
        )
        .await;

        match recorded {
            Ok(true) => tracing::info!(
                conversation_id,
                number = link.number,
                head = link.head,
                "a pull request of the stack is recorded, so the wrap-up waits on it too",
            ),
            Ok(false) => tracing::error!(
                conversation_id,
                "there is no Conversation left to record the stack against"
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, number = link.number, "recording a pull request of the stack failed");
                return None;
            }
        }
    }

    Some(found(chain, own, &said))
}

/// Which Conversation holds `number`, where one that is not this Conversation
/// does — by the name it goes under, and whether this take-up closed it.
///
/// The branch rather than the id, because that is what a Conversation is called
/// once anybody has named one, and it is the name the human will find it under
/// in the sidebar. It is not always the pull request's own head branch: a push
/// that followed a repository's branch-naming rule went under a name of its own,
/// and the Conversation kept the one it was cut with.
///
/// A Conversation closed to make way is still the one this lookup answers with:
/// the pull request stays on the record it was written to, and the Conversation
/// taking over has this link recorded beside its own rather than as the one its
/// work is on — see [`store::conversation_on_pull_request`].
async fn held(
    state: &AppState,
    conversation_id: i64,
    repo_id: i64,
    number: i64,
    closed: &[i64],
) -> Option<Whose> {
    let other = match store::conversation_on_pull_request(&state.pool, repo_id, number).await {
        Ok(Some(other)) if other != conversation_id => other,
        Ok(_) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, number, "reading which Conversation holds a pull request of the stack failed");
            return None;
        }
    };

    match store::load_conversation(&state.pool, other).await {
        Ok(Some(conversation)) => Some(Whose {
            branch: conversation.branch,
            closed: closed.contains(&other),
        }),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, other, "reading the Conversation that holds a pull request of the stack failed");
            None
        }
    }
}

/// Whose a link of the stack is: the Conversation's branch, and whether this
/// take-up closed it to make way.
struct Whose {
    /// The name that Conversation goes under.
    branch: String,

    /// Whether it was closed here — see [`crate::conversations::take_up`]. One
    /// open Conversation per pull request, and a stack sync moves every branch
    /// of the chain, so a Done Conversation standing on a link gives its
    /// checkout back before this one starts.
    closed: bool,
}
/// The chain through `open` that `from` is a link of, from the bottom.
///
/// Both ways from it, because a wrap-up may be pointed at any link and the ones
/// it was not pointed at are exactly the ones nobody would otherwise watch: down
/// while the base is another open pull request's head, and up while some open
/// pull request's base is this one's head.
///
/// **Forks are no link at all.** A pull request whose head is in another
/// repository is one nothing here could push a fix to, so the chain stops where
/// one would be rather than following it into somewhere this Conversation has no
/// branch. Which is the refusal the pull request it was pointed at already got.
///
/// **And a branch two pull requests sit on stops the climb.** A chain is a
/// chain: where two open pull requests are based on the same head, what is above
/// is a tree rather than a stack, and the part of it to sync is not something to
/// guess at.
///
/// Empty where `from` is not among the open pull requests at all — a number that
/// has since been merged or closed, or one past the end of the page `gh`
/// answered with.
fn chain(open: &[Numbered], from: i64) -> Vec<&Numbered> {
    // A fork's head is in another repository, so it is not a branch anything
    // here sits on and not one anything here sits under.
    let links: Vec<&Numbered> = open.iter().filter(|listed| !listed.fork).collect();

    let Some(&start) = links.iter().find(|listed| listed.number == from) else {
        return Vec::new();
    };

    let mut chain = vec![start];

    // Down: the pull request whose head is this one's base, as far as there is
    // one. A base nothing is open on is the branch the stack merges into, which
    // is the bottom.
    let mut below = start;

    while let Some(&under) = links
        .iter()
        .find(|listed| listed.head == below.base && listed.number != below.number)
    {
        // A chain that came back round on itself is not one. GitHub does not
        // allow it, and a walk that trusted it would not finish.
        if chain.iter().any(|link| link.number == under.number) {
            break;
        }

        chain.insert(0, under);
        below = under;
    }

    // And up: the pull request based on this one's head, while there is exactly
    // one of them.
    let mut above = start;

    loop {
        let mut over = links
            .iter()
            .filter(|listed| listed.base == above.head && listed.number != above.number);

        let Some(&next) = over.next() else {
            break;
        };

        if over.next().is_some() {
            break;
        }

        if chain.iter().any(|link| link.number == next.number) {
            break;
        }

        chain.push(next);
        above = next;
    }

    chain
}

/// What the Timeline is told the stack is: the chain from the bottom, and which
/// of it belongs to somebody else.
///
/// Written to stand on its own, because it is read in two places: appended to
/// what a take-up says about the pull request it took up, and as a Notice of its
/// own where the walk ran at the second door — see [`crate::wrapping::record`].
///
/// `said` is the Conversation holding each neighbour, where one does, by the
/// name it goes under. Every link is listed either way — what the stack *is* is
/// the thing the human cannot see from here, and a chain with nobody else on it
/// is still worth writing out, the pull requests above and below having arrived
/// on this record without anybody pressing anything.
///
/// **And which of them were closed to make way**, which is the other half of the
/// same sentence: a stack of stages is a Conversation per link, and every one of
/// them that had finished gave its checkout back so that a sync could move the
/// branch. The human pressed Start on one Conversation and several moved, so the
/// list of them belongs where the list of the links already is.
fn found(chain: &[Numbered], own: i64, said: &[(i64, Option<Whose>)]) -> String {
    let listed: Vec<String> = chain
        .iter()
        .map(|link| format!("#{} (`{}`)", link.number, link.head))
        .collect();

    let whose: Vec<String> = said
        .iter()
        .map(|(number, held)| match held {
            Some(Whose {
                branch,
                closed: true,
            }) => format!(
                "#{number} belonged to the Conversation on `{branch}`, which had finished with it \
                 and was closed to make way"
            ),
            Some(Whose { branch, .. }) => {
                format!("#{number} belongs to the Conversation on `{branch}`")
            }
            None => format!("#{number} belongs to no Conversation"),
        })
        .collect();

    format!(
        "Pull request #{own} is one of a stack of {deep}, which this wrap-up waits on whole — \
         from the bottom: {listed}. The rest of the chain is recorded here to be watched rather \
         than taken up: {whose}.",
        deep = chain.len(),
        listed = listed.join(", "),
        whose = whose.join(", "),
    )
}

/// And what it is told where the chain could not be read at all: that the
/// question was asked, that GitHub did not answer it, and what the wrap-up is
/// therefore over.
///
/// [`found`]'s opposite, and the reason it exists is that without it the two
/// are the same record. A walk that could not ask and a pull request with
/// nothing above or below it both leave the take-up's note saying only what was
/// taken up — so a human reading a wrap-up over one pull request cannot tell
/// whether that is all there was, or whether there are two more above it that
/// nothing is waiting on.
///
/// Which nothing later puts right: the walk runs where a pull request is
/// recorded and at no poll after it, so this is the record's one chance to say
/// so. It says what is *not* known rather than promising a retry, there being
/// none.
///
/// `why` is `gh`'s own account of it, which is the half that says whether this
/// is a token to renew or a network that was down for a second.
fn unread(own: i64, why: &str) -> String {
    format!(
        "Verkstead could not list this repository's open pull requests, so whether #{own} is \
         one of a stack is not known: {why}. This wrap-up is over #{own} alone — if there is \
         a chain above or below it, nothing here is waiting on it."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One open pull request of a repository, as `gh pr list` answered about it.
    fn listed(number: i64, head: &str, base: &str) -> Numbered {
        Numbered {
            number,
            title: format!("Stage {number}"),
            url: format!("https://github.com/tobico/verkstead/pull/{number}"),
            head: head.to_owned(),
            base: base.to_owned(),
            fork: false,
            open: true,
        }
    }

    /// The numbers of a chain, which is what every assertion below is about.
    fn numbers(chain: &[&Numbered]) -> Vec<i64> {
        chain.iter().map(|link| link.number).collect()
    }

    /// Pointed at the middle, the chain is walked both ways and comes back from
    /// the bottom.
    #[test]
    fn the_chain_is_walked_both_ways_from_the_pull_request_it_was_pointed_at() {
        let open = vec![
            listed(42, "stage-03", "stage-02"),
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
        ];

        assert_eq!(numbers(&chain(&open, 41)), [40, 41, 42]);
        assert_eq!(
            numbers(&chain(&open, 40)),
            [40, 41, 42],
            "and from the bottom it is the same chain, climbed the whole way",
        );
        assert_eq!(
            numbers(&chain(&open, 42)),
            [40, 41, 42],
            "and from the top, walked the whole way down",
        );
    }

    /// A pull request with nothing above or below it is a chain of one.
    #[test]
    fn a_lone_pull_request_is_a_chain_of_itself() {
        let open = vec![
            listed(41, "rate-limiting", "main"),
            listed(50, "something-else", "main"),
        ];

        assert_eq!(numbers(&chain(&open, 41)), [41]);
    }

    /// And a fork is no link: its head is in another repository, so nothing here
    /// sits on it and the chain stops where it would have been.
    #[test]
    fn a_link_whose_head_is_in_a_fork_is_not_followed() {
        let mut forked = listed(42, "stage-03", "stage-02");
        forked.fork = true;

        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            forked,
        ];

        assert_eq!(numbers(&chain(&open, 41)), [40, 41]);
    }

    /// Two pull requests on one base is a tree rather than a stack, and what to
    /// sync of it is not something to guess at.
    #[test]
    fn a_chain_that_forks_in_two_stops_where_it_forks() {
        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            listed(42, "stage-02-again", "stage-01"),
        ];

        assert_eq!(numbers(&chain(&open, 40)), [40]);
    }

    /// A number the list does not hold is a pull request merged, closed or off
    /// the end of the page — and no chain at all.
    #[test]
    fn a_pull_request_that_is_not_open_is_no_chain() {
        let open = vec![listed(40, "stage-01", "main")];

        assert!(chain(&open, 41).is_empty());
    }

    /// The chain a walk hands back, owned — which is what a take-up carries from
    /// the reading to the recording.
    fn walked(open: &[Numbered], from: i64) -> Vec<Numbered> {
        chain(open, from).into_iter().cloned().collect()
    }

    /// The Timeline gets the chain from the bottom and is told whose the
    /// neighbours are.
    #[test]
    fn the_note_says_the_chain_and_who_holds_what() {
        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            listed(42, "stage-03", "stage-02"),
        ];

        let chain = walked(&open, 41);
        let said = vec![
            (
                40,
                Some(Whose {
                    branch: "stage-01".to_owned(),
                    closed: false,
                }),
            ),
            (42, None),
        ];

        assert_eq!(
            found(&chain, 41, &said),
            "Pull request #41 is one of a stack of 3, which this wrap-up waits on whole — from \
             the bottom: #40 (`stage-01`), #41 (`stage-02`), #42 (`stage-03`). The rest of the \
             chain is recorded here to be watched rather than taken up: #40 belongs to the \
             Conversation on `stage-01`, #42 belongs to no Conversation.",
        );
    }

    /// And where the neighbours made way, the same note says which Conversations
    /// were closed to do it.
    ///
    /// Which is the half the human cannot see from anywhere else: they pressed
    /// Start on one Conversation, and a stack of three moved.
    #[test]
    fn the_note_says_which_neighbours_were_closed_to_make_way() {
        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            listed(42, "stage-03", "stage-02"),
        ];

        let chain = walked(&open, 41);
        let said = vec![
            (
                40,
                Some(Whose {
                    branch: "stage-01".to_owned(),
                    closed: true,
                }),
            ),
            (
                42,
                Some(Whose {
                    branch: "stage-03".to_owned(),
                    closed: true,
                }),
            ),
        ];

        assert_eq!(
            found(&chain, 41, &said),
            "Pull request #41 is one of a stack of 3, which this wrap-up waits on whole — from \
             the bottom: #40 (`stage-01`), #41 (`stage-02`), #42 (`stage-03`). The rest of the \
             chain is recorded here to be watched rather than taken up: #40 belonged to the \
             Conversation on `stage-01`, which had finished with it and was closed to make way, \
             #42 belonged to the Conversation on `stage-03`, which had finished with it and was \
             closed to make way.",
        );
    }

    /// And the neighbours are every link but the one walked from, from the
    /// bottom — which is what is asked to make way and what is recorded to be
    /// watched.
    #[test]
    fn the_neighbours_are_every_link_but_the_one_walked_from() {
        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            listed(42, "stage-03", "stage-02"),
        ];

        let walked = Walked::Chain(walked(&open, 41));

        assert_eq!(numbers(&walked.neighbours(41)), [40, 42]);

        assert!(
            Walked::Chain(vec![listed(41, "rate-limiting", "main")])
                .neighbours(41)
                .is_empty(),
            "a lone pull request has none",
        );
        assert!(
            Walked::Nothing.neighbours(41).is_empty(),
            "and a Process that walks no chain has none to ask",
        );
        assert!(
            Walked::Unread("gh is not logged in".to_owned())
                .neighbours(41)
                .is_empty(),
            "and a chain that could not be read names nobody to close",
        );
    }

    /// And a chain that could not be read says so, rather than leaving the
    /// record a lone pull request's.
    ///
    /// The two are the same silence otherwise, and nothing walks again — so
    /// what this says is what the human has to go on: that the question was
    /// asked, what `gh` said about it, and that the wrap-up is over the one
    /// pull request.
    #[test]
    fn a_chain_that_could_not_be_read_is_said_apart_from_a_lone_pull_request() {
        let said = unread(41, "gh is not logged in");

        assert_eq!(
            said,
            "Verkstead could not list this repository's open pull requests, so whether #41 \
             is one of a stack is not known: gh is not logged in. This wrap-up is over #41 \
             alone — if there is a chain above or below it, nothing here is waiting on it.",
        );
    }
}
