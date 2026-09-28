//! Which of a member's Repos **is** this repository (ADR-0020, *Repos across
//! devices*).
//!
//! The question a cluster has to settle before anything keyed by a path crosses
//! between machines. A Repo is a directory on one machine and the same
//! repository is a different directory on the next — cloned to another place,
//! under another name, under another home — so a device that syncs what a member
//! remembers of *this* repository, or lands a branch of it there, has first to
//! say which of that machine's Repos it means. Nobody is asked to press for it:
//! the answer is one git already holds on both ends.
//!
//! **Origin URL first.** The `origin` remote's URL, compared after the
//! differences that are spellings rather than repositories are taken out — a
//! trailing `.git`, a trailing slash, and the `scp`-style spelling of an SSH URL
//! against its `ssh://` form. Those three and nothing else: every further
//! liberty taken with a URL is a way for two repositories to read as one, and a
//! wrong match here puts somebody's work in the wrong directory.
//!
//! **Then by name, where neither end has an origin.** The registered Repo's
//! name, which is its directory's own. And **a Repo with an origin never matches
//! one without**: they are two repositories until something says otherwise, and
//! a shared name is not that something — `verkstead` on a laptop that clones it
//! and `verkstead` on a machine that started it locally are one repository or
//! two, and nothing here can tell.
//!
//! **Nothing about the registration is trusted for the origin.** A Repo records
//! a path, a name and a default branch and no origin at all, so both ends ask
//! git afresh — see [`crate::repos::origin`]. Which is also why the comparison
//! happens *here*, on the device that asked, rather than on the one that
//! answered: the far end sends facts and this end applies the rule, so two
//! Verksteads one version apart cannot disagree about a trailing `.git` and
//! therefore about which repository the work is in.
//!
//! **Asked over the link from either end.** B asks which of A's Repos is this
//! one — which is what the memory sync needs, to know whose entry to pull — and
//! the same question goes the other way when a transfer has to say where a
//! branch lands. One reading, called with the device to ask; the answering half
//! is [`crate::peer::repos`], a member's own route on the Peer Listener.
//!
//! **A device that is not answering is refused by name**, rather than answered
//! *no match*. Those are two different things to say: no match sends somebody to
//! **Open repo** on a machine that may already have the repository, and a
//! machine that is asleep will answer perfectly well tomorrow. So the sentence
//! names the machine the way every other press across a link names it — see
//! [`crate::relaying::Refusal`].
//!
//! **And no match is an answer.** It is not a failure here: what to do without
//! one is the caller's, and the caller in this stage carries on with nothing
//! pulled.

use std::borrow::Cow;

use verkstead_render::RepoAcross;

use crate::device::Devices;
use crate::peer::repos::REPOS;
use crate::relaying::{self, Refusal};

/// The most a member's Repos may be before the answer is dropped: **one
/// megabyte**.
///
/// A Repo on the wire here is an id, a name, a path and a URL — some hundreds of
/// bytes — so a megabyte is thousands of repositories, which is more than one
/// person registers. What it is for is the other case, which is the one every
/// bound across a link is for: a machine on the far end that answers and then
/// writes without stopping. The mirrors' own bound, for its reason — see
/// [`crate::mirroring::MOST_THE_PROFILES_ARE`].
const MOST_THE_REPOS_ARE: usize = 1024 * 1024;

/// Which of `device`'s Repos is `here`, the Repo of this device's own.
///
/// `Ok(None)` is *that device holds nothing that is this repository*, which is an
/// answer rather than a failure. `Err` is *the question was never answered* — the
/// machine did not take it, or this device could not put it — and it names the
/// machine.
///
/// **The whole of that device's registry is read and matched here**, one reading
/// rather than a question per Repo: a device asks once, and the rule that decides
/// runs on the end that is going to act on the answer.
///
/// **This end's origin is read at the moment it is asked for**, off the
/// repository rather than off the row, like the far end's. It goes in a blocking
/// task because that is what shelling out to git is; a match is asked for once
/// before a launch rather than on every page, so the thread it borrows costs
/// nothing.
pub async fn across(
    devices: &Devices,
    device: &str,
    here: &crate::store::Repo,
) -> Result<Option<RepoAcross>, Refusal> {
    let read = here.path.clone();
    let ours = match tokio::task::spawn_blocking(move || crate::repos::origin(&read)).await {
        Ok(ours) => ours,

        Err(why) => {
            return Err(Refusal::ours(format!(
                "this device could not read its own repository's origin: {why:#}",
            )));
        }
    };

    let theirs: Vec<RepoAcross> = relaying::word_from(
        Some(devices),
        device,
        relaying::asking(REPOS.to_owned()),
        MOST_THE_REPOS_ARE,
    )
    .await?;

    Ok(theirs
        .into_iter()
        .find(|there| same(ours.as_deref(), &here.name, there)))
}

/// Whether the repository whose origin is `ours` and whose name is `name` is the
/// one `there` stands for.
///
/// The rule itself, in three arms — and the third is the one worth stating: a
/// repository with an origin and one without are two repositories, whatever they
/// are called. A name is a directory's, two machines name directories alike all
/// the time, and *this clone and that one* is not something a shared name can
/// establish.
fn same(ours: Option<&str>, name: &str, there: &RepoAcross) -> bool {
    match (ours, there.origin.as_deref()) {
        (Some(ours), Some(theirs)) => normalised(ours) == normalised(theirs),
        (None, None) => name == there.name,
        _ => false,
    }
}

/// A URL with the differences that are spellings rather than repositories taken
/// out.
///
/// Three of them, which is the whole list:
///
/// - the **`scp`-style spelling** of an SSH URL — `git@github.com:you/thing`,
///   which is what `git clone` is usually given — against the `ssh://` form of
///   the same thing;
/// - a **trailing slash**, which a URL copied out of a browser's address bar
///   carries and a remote added by hand does not;
/// - and a **trailing `.git`**, which GitHub offers both with and without.
///
/// Nothing else. The host's case is left alone, a `https://` is never held to be
/// an `ssh://` of the same path, and no query or fragment is stripped: each of
/// those is a liberty that could make two repositories read as one, and the cost
/// of a wrong match is somebody's work landing in the wrong directory. The cost
/// of a missed one is a sentence saying so.
///
/// The slash is trimmed on both sides of the `.git`, so that
/// `…/thing.git/` and `…/thing/` come to the same place.
fn normalised(url: &str) -> String {
    let spelled = as_ssh(url.trim());
    let bare = spelled.trim_end_matches('/');
    let bare = bare.strip_suffix(".git").unwrap_or(bare);

    bare.trim_end_matches('/').to_owned()
}

/// The `scp`-style spelling as its `ssh://` form, and everything else untouched.
///
/// `[user@]host:path` is how `git clone` is given an SSH remote nine times out
/// of ten, and `ssh://[user@]host/path` is the same remote written out; a device
/// holding one spelling and a device holding the other hold one repository. The
/// path's leading slashes come off with the colon, so that `host:/srv/thing` and
/// `ssh://host/srv/thing` meet as well.
///
/// **What is left alone** is anything that already has a scheme, anything with
/// no colon in it at all, and the two shapes where a colon means something else:
/// a path with a slash before its colon, which is a local path with a colon in a
/// directory name, and a single letter before it, which is a Windows drive. Those
/// are git's own tests for the same thing, and getting either wrong would turn a
/// local clone's path into a hostname.
fn as_ssh(url: &str) -> Cow<'_, str> {
    if url.contains("://") {
        return Cow::Borrowed(url);
    }

    let Some((host, path)) = url.split_once(':') else {
        return Cow::Borrowed(url);
    };

    if host.is_empty() || host.contains('/') || host.chars().count() == 1 {
        return Cow::Borrowed(url);
    }

    Cow::Owned(format!("ssh://{host}/{}", path.trim_start_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Repo on the far end with the origin and name given, for the rule to be
    /// asked about.
    fn there(name: &str, origin: Option<&str>) -> RepoAcross {
        RepoAcross {
            id: 1,
            name: name.to_owned(),
            path: format!("/srv/{name}"),
            origin: origin.map(str::to_owned),
        }
    }

    /// One origin at two paths under two names is one repository: what the row
    /// records is not what the question is asked of.
    #[test]
    fn one_origin_under_two_names_matches() {
        assert!(same(
            Some("https://github.com/you/thing.git"),
            "thing",
            &there("work", Some("https://github.com/you/thing.git")),
        ));
    }

    /// And the three spellings of one URL are one URL.
    #[test]
    fn two_spellings_of_one_url_match() {
        for (ours, theirs) in [
            (
                "git@github.com:you/thing.git",
                "ssh://git@github.com/you/thing.git",
            ),
            ("git@github.com:you/thing", "git@github.com:you/thing.git"),
            (
                "https://github.com/you/thing.git",
                "https://github.com/you/thing/",
            ),
            (
                "https://github.com/you/thing.git/",
                "https://github.com/you/thing",
            ),
            ("host:/srv/thing", "ssh://host/srv/thing"),
        ] {
            assert!(
                same(Some(ours), "thing", &there("thing", Some(theirs))),
                "{ours} and {theirs} are two spellings of one repository",
            );
        }
    }

    /// Two repositories that share a name and have different origins are two
    /// repositories.
    #[test]
    fn one_name_over_two_origins_does_not_match() {
        assert!(!same(
            Some("https://github.com/you/thing.git"),
            "thing",
            &there("thing", Some("https://github.com/someone/thing.git")),
        ));
    }

    /// And two that are not the same repository do not become one for being
    /// spelled alike in other ways: the scheme is not normalised away, and
    /// neither is the host's case.
    #[test]
    fn nothing_beyond_the_three_is_taken_out() {
        assert!(!same(
            Some("https://github.com/you/thing"),
            "thing",
            &there("thing", Some("ssh://git@github.com/you/thing"))
        ));

        assert!(!same(
            Some("https://GitHub.com/you/thing"),
            "thing",
            &there("thing", Some("https://github.com/you/thing"))
        ));
    }

    /// Neither end has an origin and the names agree: a match, that being all
    /// there is to go on.
    #[test]
    fn with_no_origin_either_side_the_names_decide() {
        assert!(same(None, "thing", &there("thing", None)));
        assert!(!same(None, "thing", &there("other", None)));
    }

    /// And one with an origin never matches one without, whatever they are
    /// called.
    #[test]
    fn an_origin_never_matches_the_want_of_one() {
        assert!(!same(
            Some("https://github.com/you/thing.git"),
            "thing",
            &there("thing", None),
        ));

        assert!(!same(
            None,
            "thing",
            &there("thing", Some("https://github.com/you/thing.git")),
        ));
    }

    /// A local path with a colon in it is a path rather than a host, and a
    /// Windows drive is a drive: both would be turned into a hostname by a
    /// conversion that only looked for a colon.
    #[test]
    fn a_colon_that_is_not_a_hosts_is_left_alone() {
        for local in ["/srv/odd:name/thing", "C:/src/thing", "C:\\src\\thing"] {
            assert_eq!(
                as_ssh(local),
                local,
                "{local} is a path on this machine rather than a host and a path",
            );
        }
    }

    /// And two spellings of one Windows path still meet, which is what leaving
    /// the drive alone has to cost nothing: both ends normalise the same way.
    #[test]
    fn one_windows_path_matches_itself() {
        assert!(same(
            Some("C:/src/thing.git"),
            "thing",
            &there("elsewhere", Some("C:/src/thing")),
        ));
    }
}
