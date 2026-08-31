//! The rule that ends a wrap-up, and the condition it passes through on the way.
//!
//! A Conversation leaves Wrapping for **Done** when four kinds of thing are
//! true together: every pull request's checks are green, the self-review's
//! Question Set has been answered, nothing said on any of the pull requests is
//! left unaddressed, and GitHub can merge every one of them. Any one of them
//! missing keeps it where it is.
//!
//! Four kinds rather than four things, because a Conversation ends on a pull
//! request per repository it was worked in and each of them has a suite, a
//! conversation and a base of its own: the review is one review across the whole
//! of it, and the checks, the comments and the merge are one settlement each per
//! pull request. Which pull requests those are is read off the record every time
//! the rule is asked — a companion's found a poll after the Conversation's own is
//! three more things to wait on, and a wrap-up that had already counted its four
//! would have finished in between. See [`store::finish_wrap_up`].
//!
//! Verkstead decides that itself. There is nobody at the workbench to press
//! anything, which is the whole of what running unattended means — and each of
//! the four is already a fact Verkstead knows rather than an opinion somebody
//! would have to form.
//!
//! What it does **not** wait for is the merge itself. Stages stack on unmerged
//! predecessors, so a Conversation that stayed in Wrapping until its pull request
//! landed would hold up every stage behind it — and merging is the human act this
//! pipeline is built around rather than a step in it. Done means Verkstead has
//! finished with the work, not that it is on `main`. Which is why *can be merged*
//! is what the fourth settlement is about: a pull request nobody can land is work
//! Verkstead has not finished with, and one nobody has landed yet is not.
//!
//! **And never over a stop.** Every other stop a wrap-up can take leaves
//! something unsettled behind it — red checks, a review nobody finished, a batch
//! nobody answered — so the rule never had to ask whether the run was stopped.
//! A companion whose pull request was never found leaves nothing unsettled,
//! because nothing was recorded to be unsettled about: the pull requests that
//! were found could all go green and the Conversation would sail to Done past
//! its own Notice. So this asks too, the way every watcher already asks before
//! it dispatches anything — see [`crate::stopping::stopped`].
//!
//! A loop rather than a call from each of the watchers, and deliberately so: the
//! things that settle are in three different places — a poll of GitHub, another
//! poll of GitHub, and the endpoint that takes a Response — and a wrap-up left
//! for ever because one of them forgot to ask would be the failure nobody
//! notices. Asking costs a few reads of a table.
//!
//! Which is also why the condition on the way is noticed here — see
//! [`narrowing`]. A wrap-up whose review, comments and merge have settled and
//! whose checks have not, with nothing running in its Worktree, is **Waiting on
//! checks**: a label on the card and a line on the Timeline, drawn off the same
//! facts this loop already reads on a cadence, and nothing on the Lifecycle. A
//! conflicted pull request is not that, and says nothing: what it is waiting on
//! is a resolution rather than a suite.
//!
//! ## And what the settle itself does
//!
//! One thing beyond the move, and only where the human has asked for it: the
//! record is **shared to the pull requests** — published as a secret gist and
//! linked in a comment on every one of them, exactly as the Share Pane's own
//! press does it. See [`share_to_pull_requests`].
//!
//! It is this settle rather than the state that runs it. A Conversation steered
//! to Done by hand never comes through here, and nothing is published for one:
//! what the switch turns on is the *wrap-up* handing the work over, which is
//! the moment there is finished work to hand.

use verkstead_render::{ShareCommented, SharePublished};
use verkstead_schema::Nudge;

use crate::AppState;
use crate::store;

/// Ask whether `conversation_id`'s wrap-up is over, until it is or there is
/// nothing left to ask about.
///
/// Nothing here is refused for. This runs unattended with nobody watching, and
/// what it has to say it says on the Timeline or in the log.
pub(crate) async fn watch(state: AppState, conversation_id: i64) {
    loop {
        // Before the rule rather than inside it, because what a stop means here
        // is the same as what it means in front of a launch: the run does not
        // advance past one, and Done is as much an advance as a session is.
        // Started again by the press that clears it — see [`crate::resume`],
        // which starts the whole of a wrap-up over.
        //
        // And before the narrowing below for the same reason: a run that has
        // stopped is not a wrap-up waiting on its checks, so it says nothing.
        if crate::stopping::stopped(&state, conversation_id).await {
            tracing::info!(
                conversation_id,
                "driving has stopped, so the wrap-up is not being finished",
            );
            return;
        }

        // Before the rule itself, because the two are readings of the same
        // facts a moment apart and this is the one that has something to say
        // about a wrap-up that is *not* over: the narrowing is what is left
        // when everything but the checks has settled.
        narrowing(&state, conversation_id).await;

        match store::finish_wrap_up(&state.pool, conversation_id).await {
            Ok(store::Finished::StillWaiting) => {}
            Ok(store::Finished::Done) => {
                tracing::info!(
                    conversation_id,
                    "every pull request's checks are green, every one of them merges and \
                     nothing said on any of them is left unaddressed, and the review is \
                     answered, so the work is done",
                );

                // The sidebar keeps the news until the human has looked at it,
                // which is what a push nobody was there for needs behind it: a
                // notification read on a phone and swiped away is a milestone
                // the laptop would otherwise never mention. Stamped here rather
                // than wherever Done is reached, because it is this push it
                // marks the trail of — a steer to Done is the human's own act,
                // pushes nothing and stamps nothing.
                //
                // Before the Nudge, so that the sidebar the Nudge sends every
                // open page back to read is one this has already written to.
                if let Err(error) = store::stamp_unseen(&state.pool, conversation_id).await {
                    tracing::error!(
                        error = ?error,
                        conversation_id,
                        "stamping a finished Conversation unseen failed",
                    );
                }

                // The Timeline has a move on it, and an open page should say so
                // without being reloaded.
                state.nudges.announce(Nudge::Conversation {
                    conversation: conversation_id,
                });

                // And the devices are told: nobody pressed anything to get here
                // and nobody was watching it happen, which is exactly what a
                // milestone notification is for. Behind the move, which the
                // store has already made.
                crate::push::told(&state.pool, conversation_id, crate::push::News::Done);

                // And the record is handed to whoever is reviewing the work,
                // where the human has asked for that to happen by itself. This
                // settle rather than the state: a Conversation steered to Done
                // never comes through here, and nothing is shared for it.
                //
                // Awaited, and after the news above rather than in front of it:
                // a publish is a gist made, a clone and a push, and the sidebar
                // saying the work is done should not be waiting on GitHub.
                share_to_pull_requests(&state, conversation_id).await;

                // And a settled wrap-up is what lets the next roadmap stage
                // start, which is the whole of what makes a staged roadmap
                // execute itself — see [`crate::continuing`]. Asked of every
                // Conversation rather than of the ones somebody thought were
                // stages: whether this is a stage of anything is read off the
                // branch, and one that has written to no roadmap starts nothing.
                //
                // Here rather than anywhere else because this is the one place
                // that knows a wrap-up has just ended, and awaited rather than
                // spawned: this loop has nothing left to do after it, and the
                // work it is waiting on is a git read and a session starting.
                crate::continuing::carry_on(state, conversation_id).await;
                return;
            }
            // Closed out from under the watchers, or finished by something else
            // — Resume starts the whole wrap-up watching again, so two of these
            // can be running at once and the second finds the move made.
            Ok(store::Finished::NotWrapping) => {
                tracing::debug!(
                    conversation_id,
                    "the Conversation is not wrapping up any more, so nothing is left to settle",
                );
                return;
            }
            Ok(store::Finished::NoSuchConversation) => {
                tracing::error!(conversation_id, "there is no Conversation left to settle");
                return;
            }
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "asking whether a wrap-up was over failed");
            }
        }

        tokio::time::sleep(state.sessions.pace().checks).await;
    }
}

/// Notice a wrap-up that has narrowed to its checks, and say so once.
///
/// **Waiting on checks** is a condition of Wrapping rather than a state: the
/// review answered, the comments dealt with, every pull request merging, the
/// checks alone outstanding and nothing running in the Worktree. A conflicted
/// pull request is not it — what that waits on is a resolution rather than a
/// suite — and it says nothing. Nothing is stored about it beyond the mark
/// that says the Notice has been written — see [`store::narrowing`] — and the
/// Lifecycle is untouched. It is read off the settle facts and the sessions
/// register the same way *blocked on you* is read off a stop.
///
/// Here rather than anywhere else because this loop already reads those facts
/// on a cadence, and the narrowing is exactly what it finds when the answer to
/// *is this over* is nearly yes.
///
/// One Notice per narrowing, which is the whole of what the mark is for: leaving
/// the condition takes the mark with it, so a fix session dispatched or a
/// comment landing and the wrap-up quietening again writes a fresh line rather
/// than a duplicate of the first or nothing at all.
///
/// **No device push.** There is nothing for the human to do about it — the
/// checks are GitHub's to finish — so it is a line on the Timeline and a label
/// on the card, and neither is worth a phone lighting up.
async fn narrowing(state: &AppState, conversation_id: i64) {
    // The register rather than the record: what says a wrap-up is waiting is
    // that nobody is in it, and a fix session working a red check is a wrap-up
    // getting on with it.
    let working = state.sessions.working().contains(&conversation_id);

    match store::narrowing(&state.pool, conversation_id, working).await {
        Ok(store::Narrowing::Narrowed) => {
            tracing::info!(
                conversation_id,
                "the review is answered and nothing is left unaddressed, so the wrap-up is \
                 waiting on its checks",
            );

            if let Err(error) = store::note(
                &state.pool,
                conversation_id,
                "**Waiting on checks.** The review is answered and nothing said on the pull \
                 request is left unaddressed, so the checks going green is the whole of what \
                 this wrap-up is still waiting on.",
            )
            .await
            {
                tracing::error!(error = ?error, conversation_id, "saying that a wrap-up was down to its checks failed");

                // And the mark comes off, so the next poll is told to write it
                // again: one standing over a line that never landed would be a
                // narrowing said nowhere at all.
                if let Err(error) = store::forget_narrowing(&state.pool, conversation_id).await {
                    tracing::error!(error = ?error, conversation_id, "taking back the mark on a line that was never written failed");
                }

                return;
            }

            // The Timeline has a line on it and the card a label, and an open
            // page should say so without being reloaded. The one kind carries
            // both — a Conversation that moved is a sidebar row that reads
            // differently, which is exactly what this is.
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });
        }
        // Said already, and still true: the label goes on standing and there is
        // nothing to write.
        Ok(store::Narrowing::NoticedAlready) => {}
        // And not narrowed — which includes every state that is not Wrapping,
        // so a Conversation steered away leaves no mark behind for the round
        // after it to be quiet on.
        Ok(store::Narrowing::NotNarrowed) => {}
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "asking whether a wrap-up was down to its checks failed");
        }
    }
}

/// Hand this Conversation's record to whoever is reviewing the work, where the
/// human has asked for that to happen by itself.
///
/// The same act the Share Pane's **Share to pull request** press is, run by
/// nobody: a fresh share published as a secret gist, and the link commented on
/// every pull request the Conversation holds — see [`crate::ui::commented`],
/// which is the one spelling of it.
///
/// **Two things have to be true.** The switch on the settings page is on — it is
/// off until somebody turns it on, because what it does writes to GitHub under
/// the human's own account — and no share-to-PR comment is on this
/// Conversation's record yet.
///
/// **The gate is the fact rather than the arrival.** A Conversation the human
/// handed over themselves is already commented and stays quiet; a wrap-up that
/// settles a second time after a share that worked stays quiet too; and a share
/// that *failed* wrote no fact, so the settle after it tries again. The fact is
/// written where the comment lands rather than here, because the press writes
/// the same one — see [`store::record_share_comment`].
///
/// **Failure is a Notice and success is nothing.** A publish nobody could make
/// and a pull request the comment could not land on are both something for the
/// human to go and do, and the Timeline is where a wrap-up says what it could
/// not do. A share that worked says nothing at all: the fresh publish reads in
/// the Share Pane, which is where somebody looking for it would look.
async fn share_to_pull_requests(state: &AppState, conversation_id: i64) {
    // Read fresh, like everything else out of `config.yaml`: the switch is what
    // it says at the moment the work settles rather than at the moment the
    // wrap-up began.
    if !state.settings.config().share_on_done() {
        return;
    }

    match store::share_commented(&state.pool, conversation_id).await {
        // Said already — by the human's own press or by an earlier settle — so
        // there is nothing to say again.
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            // Nothing is shared over a fact that could not be read. The failure
            // that costs least is the quiet one: a second comment on a pull
            // request somebody is reading cannot be taken back, and a share
            // that did not happen is one press away.
            tracing::error!(error = ?error, conversation_id, "reading whether a share was ever commented failed, so nothing is being shared");
            return;
        }
    }

    let Some(bundle) = crate::ui::share_of(state, conversation_id).await else {
        say(
            state,
            conversation_id,
            "**The record could not be shared to the pull request** — it could not be read to \
             compose a share of, and the log says what failed. The Share pane's own press is \
             where to try it again.",
        )
        .await;
        return;
    };

    match crate::ui::commented(state, &bundle).await {
        Ok(said) => {
            if let Some(trouble) = unshared(&said) {
                say(state, conversation_id, &trouble).await;
            }
        }
        // The server failing at its own end rather than GitHub refusing
        // anything — a viewer this binary was built without, or a write to the
        // record that would not go. Said as what it is, with the log carrying
        // the rest.
        Err(why) => {
            say(
                state,
                conversation_id,
                &format!(
                    "**The record could not be shared to the pull request** — {why}. The Share \
                     pane's own press is where to try it again.",
                ),
            )
            .await;
        }
    }
}

/// What a Notice says about a share the settle could not leave, or `None` where
/// there is nothing to say.
///
/// Nothing is said about a share that worked, which is the ordinary answer: the
/// publish reads in the Share Pane and the comment reads on the pull request,
/// and a Timeline line saying so would be a line on every finished Conversation
/// the human has ever seen.
///
/// A Conversation on no pull request is not a failure either, and cannot happen
/// to a wrap-up: it ends on one per repository it was worked in, and it is the
/// finding of them that moved it into Wrapping at all. Nothing is published for
/// one, and nothing is said about it here.
///
/// Everything else is named as what it is, because each is a different thing for
/// the human to go and do — a token to save, a token to re-issue with the `gist`
/// scope, GitHub's own refusal, or a pull request to paste the link on
/// themselves.
fn unshared(said: &ShareCommented) -> Option<String> {
    match said {
        ShareCommented::NoPullRequest => None,

        ShareCommented::NotPublished { why } => Some(format!(
            "**The record could not be shared to the pull request.** {}",
            unpublished(why),
        )),

        // Published, and said on some of them: what is left to say is which
        // missed out and why, so the human can paste the link there themselves.
        // The link is on the record either way, and the Share pane draws it.
        ShareCommented::Commented { missed, .. } if !missed.is_empty() => Some(format!(
            "**The share was published, and could not be commented on {}.** {} The link is on \
             the Share pane, to paste there by hand.",
            listed(missed.iter().map(named)),
            missed
                .iter()
                .map(|pull| format!("{}: {}.", named(pull), pull.why.trim_end_matches('.')))
                .collect::<Vec<_>>()
                .join(" "),
        )),

        ShareCommented::Commented { .. } => None,
    }
}

/// Why the publish did not happen, as a sentence: the refusal, and the one
/// thing to go and do about it.
///
/// The workbench's own words for each of them are in `web/src/workbench/`, and
/// these are the same three answers said for a Timeline rather than for a
/// toast: two of them are the token on the settings page, which is where they
/// are fixed.
fn unpublished(why: &SharePublished) -> String {
    match why {
        SharePublished::NoToken => "No GitHub token is configured, so there is nobody to publish \
             a share as. The settings page is where one is saved."
            .to_owned(),
        SharePublished::NoGistScope => "The configured GitHub token may not write gists. \
             Re-issue it with the `gist` scope and save it again on the settings page."
            .to_owned(),
        SharePublished::Refused { why } => {
            format!("GitHub refused it: {}.", why.trim_end_matches('.'))
        }
        // Never arrives: this is the shape of a press that got no further than
        // the publish, and a publish that worked is not one of them.
        SharePublished::Published { .. } => "The publish worked.".to_owned(),
    }
}

/// One pull request as a human names it: the number, and the repository where
/// it is not the Conversation's own.
fn named(pull: &verkstead_render::MissedOut) -> String {
    match &pull.repo {
        Some(repo) => format!("#{} in {repo}", pull.number),
        None => format!("#{}", pull.number),
    }
}

/// A few of them in a row, said the way a sentence says them.
fn listed(names: impl Iterator<Item = String>) -> String {
    let names: Vec<String> = names.collect();

    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// Put a notice on the Timeline, and tell every open page there is one.
///
/// Nothing is refused for: by the time this has anything to say, what it is
/// saying has already happened. A notice that could not be written is a line in
/// the log and no more.
async fn say(state: &AppState, conversation_id: i64, markdown: &str) {
    match store::note(&state.pool, conversation_id, markdown).await {
        Ok(true) => state.nudges.announce(Nudge::Conversation {
            conversation: conversation_id,
        }),
        Ok(false) => tracing::error!(
            conversation_id,
            "there is no Conversation left to say anything on",
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "putting a notice on a Timeline failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use verkstead_render::{CommentedOn, MissedOut, ShareView};

    use super::*;

    /// A share that went everywhere says nothing: the publish reads in the
    /// Share pane and the comment reads on the pull request, and a Timeline
    /// line about it would be one on every finished Conversation there is.
    #[test]
    fn a_share_that_worked_is_not_worth_a_notice() {
        assert_eq!(
            unshared(&ShareCommented::Commented {
                share: share(),
                on: vec![landed(41, None)],
                missed: Vec::new(),
            }),
            None,
        );
    }

    /// And neither is a Conversation on no pull request, which is not something
    /// that can happen to a wrap-up: nothing was published for it, and nothing
    /// failed.
    #[test]
    fn a_conversation_on_no_pull_request_is_not_a_failure() {
        assert_eq!(unshared(&ShareCommented::NoPullRequest), None);
    }

    /// Each way the publish can refuse is named as itself, because each is a
    /// different thing for the human to go and do.
    #[test]
    fn a_publish_that_never_happened_says_which_of_them_it_was() {
        let said = unshared(&ShareCommented::NotPublished {
            why: SharePublished::NoToken,
        })
        .expect("a publish that never happened is worth a Notice");

        assert!(
            said.contains("no GitHub token") || said.contains("No GitHub token"),
            "{said}"
        );
        assert!(said.contains("settings page"), "{said}");

        let said = unshared(&ShareCommented::NotPublished {
            why: SharePublished::NoGistScope,
        })
        .expect("a token that may not write gists is worth one too");

        assert!(said.contains("`gist` scope"), "{said}");

        let said = unshared(&ShareCommented::NotPublished {
            why: SharePublished::Refused {
                why: "gh: Not Found (HTTP 404)".to_owned(),
            },
        })
        .expect("and so is GitHub refusing it");

        assert!(said.contains("gh: Not Found (HTTP 404)"), "{said}");
    }

    /// A share that was published and could not be commented everywhere names
    /// the pull requests that missed out and what GitHub said about each, and
    /// points at the link the human can paste there themselves.
    #[test]
    fn a_pull_request_the_comment_missed_is_named_with_what_was_said_about_it() {
        let said = unshared(&ShareCommented::Commented {
            share: share(),
            on: vec![landed(41, None)],
            missed: vec![missed(
                7,
                Some("verkstead-site"),
                "gh: Not Found (HTTP 404)",
            )],
        })
        .expect("a pull request that missed out is worth a Notice");

        assert!(said.contains("#7 in verkstead-site"), "{said}");
        assert!(said.contains("gh: Not Found (HTTP 404)"), "{said}");
        assert!(said.contains("Share pane"), "{said}");

        // And the ones that worked are not named: what is left to do is about
        // the ones that did not.
        assert!(!said.contains("#41"), "{said}");
    }

    /// And several of them read as a sentence rather than as a list.
    #[test]
    fn several_that_missed_out_are_said_in_a_row() {
        let said = unshared(&ShareCommented::Commented {
            share: share(),
            on: Vec::new(),
            missed: vec![
                missed(7, Some("verkstead-site"), "gone"),
                missed(9, Some("askance"), "forbidden"),
                missed(41, None, "gone too"),
            ],
        })
        .expect("three that missed out are worth a Notice");

        assert!(
            said.contains("#7 in verkstead-site, #9 in askance and #41"),
            "{said}",
        );
    }

    fn share() -> ShareView {
        ShareView {
            url: "https://tobico.github.io/verkstead/share-viewer.html#9f1".to_owned(),
            gist: "https://gist.github.com/tobico/9f1".to_owned(),
            at: "2026-08-31T01:02:03.000Z".to_owned(),
        }
    }

    fn landed(number: i64, repo: Option<&str>) -> CommentedOn {
        CommentedOn {
            number,
            repo: repo.map(str::to_owned),
            url: format!("https://github.com/tobico/verkstead/pull/{number}#issuecomment-1"),
        }
    }

    fn missed(number: i64, repo: Option<&str>, why: &str) -> MissedOut {
        MissedOut {
            number,
            repo: repo.map(str::to_owned),
            why: why.to_owned(),
        }
    }
}
