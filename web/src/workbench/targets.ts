//! What a **Target** field holds, read on this side of the wire.
//!
//! Two questions, and the same two shapes answer both: a
//! `github.com/<owner>/<repo>/pull/<n>` URL and a bare `#<n>` are the ways a
//! pull request is named, and anything else is a branch.
//!
//! - [`namesPullRequest`] asks what the *field* holds, which is what decides
//!   whether the base picker is drawn beside it: a pull request brings
//!   GitHub's own base, and a branch's base is what its pull request will be
//!   opened against.
//! - [`pullRequestIn`] scans *prose* for the first name in it, which is what
//!   fills an empty Target from the Brief.
//!
//! **A second copy of a reading the server has** — `crates/server/src/targets.rs`,
//! which is the one that decides anything. It is here because the compose page
//! holds everything on the device and nothing reaches the server until a press,
//! so a page that could not read its own box would have to ask the server what
//! it is holding. What this copy decides is what is *drawn*; what a target
//! turns out to be is settled at Start, by the server, and refused there by
//! name.

/// The two shapes as the whole of a field: a field holds the name of one
/// thing, so prose with a number in it is a branch nobody has rather than the
/// pull request somewhere inside it.
///
/// The URL keeps whatever GitHub hung off the end of it — a fragment, a query,
/// the `/files` tab, a trailing slash — because those are what is in the
/// address bar when somebody copies one.
const FIELD =
  /^(?:(?:https?:\/\/)?(?:www\.)?github\.com\/[\w.-]+\/[\w.-]+\/pull\/(\d+)(?:[/?#].*)?|#(\d+))$/i;

/// And the same two anywhere in a sentence, for the Brief.
///
/// One expression over both alternatives rather than two scanned and compared,
/// so that a URL carrying its own fragment is one name rather than two — the
/// server's own reason, in the server's own expression. The `#` has to start a
/// word: `owner/repo#41` names another repository, and a heading's `#` is
/// followed by a space.
const NAMED =
  /(?:https?:\/\/)?(?:www\.)?github\.com\/([\w.-]+)\/([\w.-]+)\/pull\/(\d+)|(?:^|[^\w/])#(\d+)/i;

/// Whether what is in a Target field names a pull request, which is the whole
/// of what the page does with it: everything else in there is a branch.
export function namesPullRequest(target: string): boolean {
  return FIELD.test(target.trim());
}

/// And the first pull request a Brief names, written the way somebody typing
/// it into the field would have written it — the URL where the prose carried
/// one, `#<n>` where it carried a bare number.
///
/// `null` where the prose names none. What comes back goes into an empty
/// Target and never over one somebody typed, which is the caller's rule to
/// keep: see `composing.ts`, and the server's own `save_brief`.
export function pullRequestIn(brief: string): string | null {
  const found = NAMED.exec(brief);
  if (found === null) {
    return null;
  }

  const [, owner, repo, numbered, hashed] = found;

  return owner !== undefined && repo !== undefined && numbered !== undefined
    ? `https://github.com/${owner}/${repo}/pull/${numbered}`
    : `#${hashed}`;
}
