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
//! Worktree is where the work happens, and it is the one the *one Conversation
//! per piece of work* refusal is about. The rest are recorded so that they are
//! watched — the wrap-up waits on Mergeable and the checks for every recorded
//! pull request — and in this workbench they usually belong to a Conversation
//! each, that being what a stacked stage is. So the refusal reads the pull
//! request a Conversation was pointed at rather than every row recorded beside
//! it — see [`store::conversation_on_pull_request`] — and the note below says
//! which of the stack somebody else holds, that being a thing to be told rather
//! than to find out later.
//!
//! **A chain that leaves the Repo is not followed.** A head in a fork is not a
//! branch this Conversation could ever push a fix to, which is the refusal the
//! pull request it was pointed at already gets — see
//! [`crate::conversations::resolve`] — so a fork is no link at all and the chain
//! stops where one would be.
//!
//! **And it runs where the pull request is recorded**, which is two doors rather
//! than one. A take-up over a pull request records it at the press and walks
//! from there. A take-up over a bare branch records nothing, sends one
//! `submitting` session for the pull request nobody opened, and the walk runs
//! where *that* is recorded — the same walk, a few minutes later, with a note of
//! its own because the take-up's was written before there was anything to walk
//! from.

use crate::AppState;
use crate::github::{self, Numbered};
use crate::store;

/// Walk the chain from the pull request `conversation_id` is on, record every
/// link of it, and say in words what the stack is.
///
/// `None` where there is nothing to say: a Process that does not walk, a
/// Conversation with no pull request recorded yet, a `gh` that would not answer,
/// or a lone pull request — which is the ordinary case and reads exactly as it
/// read before there were stacks.
///
/// Called before the watchers start, which is what watches the neighbours: a
/// wrap-up starts one checks watcher and one comments watcher per *recorded*
/// pull request, so a chain recorded first is a chain watched without anything
/// here spawning a thing. See [`crate::wrapping::watching`].
///
/// Nothing is refused for and nothing is returned but the words. A stack that
/// could not be read is a wrap-up over the one pull request it was pointed at,
/// which is what every Fix Merge Issues was until this stage: the press it is
/// answering has already succeeded, and there is nothing here worth undoing it
/// for.
pub(crate) async fn walked(state: &AppState, conversation_id: i64) -> Option<String> {
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

    // Off the runtime's threads, as every other reach into GitHub is: `gh` is a
    // process, and running one blocks.
    let asked = tokio::task::spawn_blocking({
        let gh = state.github.clone();
        let path = repo.path.clone();

        move || github::open_pull_requests(&gh, &path)
    })
    .await;

    let open = match asked {
        Ok(Ok(open)) => open,
        Ok(Err(trouble)) => {
            tracing::warn!(
                conversation_id,
                repo = repo.name,
                why = trouble.why(),
                "the Repo's open pull requests could not be listed, so nothing was walked",
            );

            return None;
        }
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking gh for the Repo's open pull requests failed");
            return None;
        }
    };

    let chain = chain(&open, own.number);

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

    for link in &chain {
        if link.number == own.number {
            continue;
        }

        said.push((
            link.number,
            held(state, conversation_id, repo.id, link.number).await,
        ));
    }

    for link in &chain {
        if link.number == own.number {
            continue;
        }

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

    Some(found(&chain, own.number, &said))
}

/// Which Conversation holds `number`, where one that is not this Conversation
/// does — by the name it goes under, which is its branch.
///
/// The branch rather than the id, because that is what a Conversation is called
/// once anybody has named one, and it is the name the human will find it under
/// in the sidebar. It is not always the pull request's own head branch: a push
/// that followed a repository's branch-naming rule went under a name of its own,
/// and the Conversation kept the one it was cut with.
async fn held(state: &AppState, conversation_id: i64, repo_id: i64, number: i64) -> Option<String> {
    let other = match store::conversation_on_pull_request(&state.pool, repo_id, number).await {
        Ok(Some(other)) if other != conversation_id => other,
        Ok(_) => return None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, number, "reading which Conversation holds a pull request of the stack failed");
            return None;
        }
    };

    match store::load_conversation(&state.pool, other).await {
        Ok(Some(conversation)) => Some(conversation.branch),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, other, "reading the Conversation that holds a pull request of the stack failed");
            None
        }
    }
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
fn found(chain: &[&Numbered], own: i64, said: &[(i64, Option<String>)]) -> String {
    let listed: Vec<String> = chain
        .iter()
        .map(|link| format!("#{} (`{}`)", link.number, link.head))
        .collect();

    let whose: Vec<String> = said
        .iter()
        .map(|(number, held)| match held {
            Some(branch) => format!("#{number} belongs to the Conversation on `{branch}`"),
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

    /// The Timeline gets the chain from the bottom and is told whose the
    /// neighbours are.
    #[test]
    fn the_note_says_the_chain_and_who_holds_what() {
        let open = vec![
            listed(40, "stage-01", "main"),
            listed(41, "stage-02", "stage-01"),
            listed(42, "stage-03", "stage-02"),
        ];

        let chain = chain(&open, 41);
        let said = vec![(40, Some("stage-01".to_owned())), (42, None)];

        assert_eq!(
            found(&chain, 41, &said),
            "Pull request #41 is one of a stack of 3, which this wrap-up waits on whole — from \
             the bottom: #40 (`stage-01`), #41 (`stage-02`), #42 (`stage-03`). The rest of the \
             chain is recorded here to be watched rather than taken up: #40 belongs to the \
             Conversation on `stage-01`, #42 belongs to no Conversation.",
        );
    }
}
