//! What a URL says, with nothing asked of GitHub: which repository a remote
//! is, where its API lives, and the pull request a piece of prose names.
//!
//! **All of it is string work**, carried over from the server's own
//! `matching.rs` and `targets.rs` rather than re-decided, so that a Repo reads
//! the same repository across the boundary as it did in process. The
//! expressions the server wrote with `regex` are scanned by hand here: a
//! component is bytes in every Verkstead binary, and a regex engine is a lot
//! of them for two shapes.

/// The repository a remote names: the host it is on, and the owner and name
/// under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// Lowercased, as a host is compared everywhere.
    pub host: String,
    pub owner: String,
    pub name: String,
}

impl Repository {
    /// `owner/name`, which is what a URL is checked against and what the
    /// REST paths are built from.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Where this host's REST API is.
    ///
    /// github.com's is a host of its own; a GitHub Enterprise Server answers
    /// under the host itself, at `/api/v3` — see the *Hosts added to a Forge*
    /// part of the stage brief, which is how a self-hosted instance is reached
    /// without a plugin being written for it.
    pub fn rest(&self) -> (String, String) {
        match self.is_dot_com() {
            true => ("api.github.com".to_owned(), String::new()),
            false => (self.host.clone(), "/api/v3".to_owned()),
        }
    }

    /// And where its GraphQL API is: the same host, `/graphql` under it.
    pub fn graphql(&self) -> (String, String) {
        match self.is_dot_com() {
            true => ("api.github.com".to_owned(), "/graphql".to_owned()),
            false => (self.host.clone(), "/api/graphql".to_owned()),
        }
    }

    fn is_dot_com(&self) -> bool {
        self.host == "github.com" || self.host == "www.github.com"
    }
}

/// The repository the remote `origin` is, or `None` where it names none — a
/// path on this machine, a `file://` URL, or a URL with no owner and name
/// under its host.
///
/// **Any host.** Which Forge a Repo is on is settled before any of this is
/// asked: the server reads the remote's host against the hosts every enabled
/// Forge declares, and only then asks that Forge about the Repo. So what is
/// left to work out here is the host to speak to and the repository under it,
/// and a host nobody declared never gets this far.
pub fn repository(origin: &str) -> Option<Repository> {
    let bare = normalised(origin);
    let host = host_of(&bare)?;
    let (_, path) = placed(&bare).split_once('/')?;
    let (owner, name) = path.split_once('/')?;

    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| Repository {
        host,
        owner: owner.to_owned(),
        name: name.to_owned(),
    })
}

/// And the `owner/name` the remote `origin` is **on github.com**, which is the
/// question the workbench asks when it draws which GitHub repository a Repo is
/// — `None` for anything else.
///
/// Narrower than [`repository`] on purpose: this answers *is this one of
/// GitHub's own*, and a repository on a host the human added to this Forge is
/// that host's rather than github.com's.
pub fn dot_com_repository(origin: &str) -> Option<String> {
    let found = repository(origin)?;
    found.is_dot_com().then(|| found.slug())
}

/// A remote with the spellings that are spellings rather than repositories
/// taken out: the `scp` form written as `ssh://`, and the trailing `.git` and
/// slash off.
fn normalised(url: &str) -> String {
    let spelled = as_ssh(url.trim());
    let bare = spelled.trim_end_matches('/');
    let bare = bare.strip_suffix(".git").unwrap_or(bare);

    bare.trim_end_matches('/').to_owned()
}

/// The `scp`-style spelling as its `ssh://` form, and everything else
/// untouched — git's own tests for the same thing: anything with a scheme,
/// anything with no colon, a colon with a slash before it (a local path) and a
/// single letter before it (a Windows drive) are left alone.
fn as_ssh(url: &str) -> String {
    if url.contains("://") {
        return url.to_owned();
    }

    let Some((host, path)) = url.split_once(':') else {
        return url.to_owned();
    };

    if host.is_empty() || host.contains('/') || host.chars().count() == 1 {
        return url.to_owned();
    }

    format!("ssh://{host}/{}", path.trim_start_matches('/'))
}

/// The host a normalised remote is on, lowercased, or `None` where it names
/// none. The user and the port come off, as they do where the server matches a
/// Repo's Forge by the same host.
fn host_of(bare: &str) -> Option<String> {
    let (scheme, rest) = bare.split_once("://")?;
    if scheme.eq_ignore_ascii_case("file") {
        return None;
    }

    let rest = match rest.split_once('@') {
        Some((user, host)) if !user.contains('/') => host,
        _ => rest,
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = match host.rsplit_once(':') {
        Some((named, port)) if port.chars().all(|c| c.is_ascii_digit()) => named,
        _ => host,
    };

    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// A normalised remote with its scheme and its user taken off, as a person
/// would say where it is: `github.com/you/thing`.
fn placed(bare: &str) -> &str {
    let after_scheme = bare.split_once("://").map_or(bare, |(_, rest)| rest);

    match after_scheme.split_once('@') {
        Some((user, rest)) if !user.contains('/') => rest,
        _ => after_scheme,
    }
}

/// A pull request a piece of prose or a Target field named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub number: i64,

    /// The `owner/repo` a URL said it is in, or `None` where the name was a
    /// bare `#<n>`.
    pub repository: Option<String>,
}

/// The first pull request `prose` names, or `None` where it names none.
///
/// **The URL shape wins where both would match at the same place**, so a URL
/// carrying its own fragment is read as one name rather than two. And **a `#`
/// has to start a word**: one after a letter, a digit or a slash is
/// `owner/repo#41` or a URL's fragment, neither of which is a number this
/// Repo's origin can be asked about.
pub fn pull_request_in(prose: &str) -> Option<Named> {
    scan(prose, false)
}

/// And the pull request a **Target** field names as the whole of it, or `None`
/// where what it holds is a branch.
///
/// Anchored, where [`pull_request_in`] scans: the field holds the name of one
/// thing. A URL keeps whatever GitHub hung off the end of it — a fragment, a
/// query, the `/files` tab, a trailing slash — because that is what is in the
/// address bar when somebody copies one.
pub fn pull_request_named(target: &str) -> Option<Named> {
    scan(target.trim(), true)
}

/// And the `owner/repo` a pull request's own URL says it is in, or `None`
/// where the URL is not one of GitHub's.
pub fn repository_in(url: &str) -> Option<String> {
    scan(url, false)?.repository
}

/// The one scan both readings are: left to right for the first of the two
/// shapes, or — `whole` — the whole of what it was handed being one of them.
fn scan(said: &str, whole: bool) -> Option<Named> {
    let bytes = said.as_bytes();

    for at in 0..=bytes.len() {
        if whole && at > 0 {
            return None;
        }

        if let Some((named, end)) = url_at(said, at)
            && (!whole || end == bytes.len() || tail(said, end))
        {
            return Some(named);
        }

        if let Some(number) = hash_at(said, at, whole)
            && (!whole || at + 1 + digits(said, at + 1) == bytes.len())
        {
            return Some(Named {
                number,
                repository: None,
            });
        }
    }

    None
}

/// The pull request URL at `at`, with where it ends, or `None` where one does
/// not start there: `[https://][www.]github.com/<owner>/<repo>/pull/<number>`,
/// matched without regard to case.
fn url_at(said: &str, at: usize) -> Option<(Named, usize)> {
    let rest = said.get(at..)?;
    let rest = strip(rest, "https://")
        .or_else(|| strip(rest, "http://"))
        .unwrap_or(rest);
    let rest = strip(rest, "www.").unwrap_or(rest);
    let rest = strip(rest, "github.com/")?;

    let (owner, rest) = name_in(rest)?;
    let rest = strip(rest, "/")?;
    let (repo, rest) = name_in(rest)?;
    let rest = strip(rest, "/pull/")?;

    let taken = digits(rest, 0);
    if taken == 0 {
        return None;
    }
    let number: i64 = rest[..taken].parse().ok()?;

    Some((
        Named {
            number,
            repository: Some(format!("{owner}/{repo}")),
        },
        said.len() - rest.len() + taken,
    ))
}

/// And the bare `#<number>` at `at`, or `None` where there is none — `whole`
/// taking the field's reading, where the `#` opens what it was handed.
fn hash_at(said: &str, at: usize, whole: bool) -> Option<i64> {
    if !said.as_bytes().get(at).is_some_and(|byte| *byte == b'#') {
        return None;
    }

    let before = said[..at].chars().next_back();
    let opens = match before {
        None => true,
        Some(_) if whole => false,
        Some(char) => !char.is_alphanumeric() && char != '_' && char != '/',
    };
    if !opens {
        return None;
    }

    let rest = said.get(at + 1..)?;
    let taken = digits(rest, 0);
    if taken == 0 {
        return None;
    }

    rest[..taken].parse().ok()
}

/// What a Target field may carry after a URL and still be that URL: the tab,
/// the query or the fragment GitHub hands out, or a trailing slash.
fn tail(said: &str, end: usize) -> bool {
    said[end..].starts_with(['/', '?', '#'])
}

/// `said` with `word` off the front, matched without regard to case, or
/// `None` where it does not start with it.
fn strip<'s>(said: &'s str, word: &str) -> Option<&'s str> {
    let taken = said.get(..word.len())?;
    taken
        .eq_ignore_ascii_case(word)
        .then(|| &said[word.len()..])
}

/// The owner or repository name at the front of `said`, and what follows it: a
/// run of word characters, dots and dashes, which is what GitHub allows and
/// what the server's own expression took.
fn name_in(said: &str) -> Option<(&str, &str)> {
    let taken = said
        .char_indices()
        .take_while(|(_, char)| {
            char.is_alphanumeric() || *char == '_' || *char == '.' || *char == '-'
        })
        .map(|(at, char)| at + char.len_utf8())
        .last()?;

    Some((&said[..taken], &said[taken..]))
}

/// How many ASCII digits `said` holds from `at`.
fn digits(said: &str, at: usize) -> usize {
    said[at..].bytes().take_while(u8::is_ascii_digit).count()
}

/// Whether what a publish failed with is the Forge turning the token down for
/// what it may do rather than turning the request down.
///
/// Two shapes, because GitHub has two ways of saying it. A fine-grained token
/// without the permission is told so — *not accessible by* — and a classic one
/// gets a `404`, which is GitHub's habit everywhere: telling somebody a
/// resource exists but is not theirs is telling them it exists.
///
/// The `404` is safe to read that way *here* and would not be anywhere else:
/// what was asked for is `POST /gists`, a collection rather than a thing, and
/// there is nothing about it for GitHub to have lost.
pub fn unscoped(said: &str) -> bool {
    let said = said.to_lowercase();

    said.contains("not accessible by")
        || said.contains("requires authentication")
        || said.contains("not found")
}
