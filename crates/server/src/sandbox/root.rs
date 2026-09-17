//! Claude's built root: the `.claude` a session is given in place of the
//! account's whole one.
//!
//! **An allowlist, and nothing outside it.** What a Claude session needs of its
//! account is its login and two entries under `projects/` — the Repo's main
//! checkout's, which holds Claude's per-Repo memory, and the Worktree's, where
//! the session's transcript is written. Everything else under `~/.claude` is
//! the human's own way of working: plugins, hooks, commands, a global
//! `CLAUDE.md`, the history, and every other repository's transcripts. None of
//! that is a session's, so none of it is in the root, and whatever Claude adds
//! next is absent from it too without anybody having to notice.
//!
//! **The root is Verkstead's own directory**, under the Conversation's profile
//! in the Data Directory, emptied and made again as each session starts — see
//! [`super::Access::Built`]. What goes into it is joined rather than copied, so
//! a login from inside and a memory written inside both land in the account:
//! a bind on Linux, a symlink on a Mac, and on Windows a hard link for the login
//! and a junction for each entry.

use std::io;
use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// The file Claude keeps a login in, inside `~/.claude`.
///
/// Not there at all on a Mac whose login is in the Keychain, which is a root
/// with no credentials in it — see [`Root::joined`].
const CREDENTIALS: &str = ".credentials.json";

/// The directory of per-path entries Claude keeps memory and transcripts in.
const PROJECTS: &str = "projects";

/// The settings file Claude reads for the user, inside `~/.claude`.
const SETTINGS: &str = "settings.json";

/// Of the account's own settings, the keys a root's settings carry over.
///
/// **An allowlist rather than a denylist**, because a denylist drifts every time
/// Claude adds a key. These two are how an API-key login reaches the model:
/// `apiKeyHelper` is the command that prints the key, and `env` is where an
/// account keeps `ANTHROPIC_API_KEY` and the variables beside it. Everything
/// else is how the human works — `hooks`, `enabledPlugins`, `permissions`,
/// `statusLine` — and none of it is a session's.
const CARRIED: [&str; 2] = ["apiKeyHelper", "env"];

/// How long an entry's name is before Claude cuts it and puts a hash on the end.
const LONGEST: usize = 200;

/// What a Claude session is given of its account: the account's own directory,
/// and the names of the `projects/` entries joined from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Root {
    /// The account's own `~/.claude`, which the Profile names.
    account: PathBuf,

    /// The entries joined, in the order they are said: the Repo's main
    /// checkout's first and the Worktree's after it. One where the two are the
    /// same name.
    entries: Vec<String>,
}

impl Root {
    /// The root for a session in `worktree`, whose Repo's common git directory
    /// is `git_dir`, logged in as the account at `account`.
    ///
    /// **Each path as Claude will read it inside**, because the name is the
    /// path's. The main checkout is what Claude reads back out of the
    /// Worktree's own `.git` file, which git writes resolved — the same answer
    /// git gave for `git_dir`. The Worktree is the directory Claude was started
    /// in, as the machine reports it: resolved on a Mac and on Windows, where a
    /// session starts in the host's own path, and as stored on Linux, where a
    /// bind makes the Worktree at the path it was stored under and nothing on
    /// the way to it is a link.
    ///
    /// **And plain**, because resolving a path on Windows writes `\\?\` in
    /// front of it, and Claude names an entry from the path it was started in,
    /// which has no such prefix — see [`super::plainly`].
    pub(crate) fn of(platform: Platform, account: &Path, git_dir: &Path, worktree: &Path) -> Root {
        let worktree = match platform {
            Platform::Linux => worktree.to_owned(),
            Platform::MacOs | Platform::Windows => resolved(worktree),
        };

        let mut entries = Vec::new();

        for path in [main_checkout(git_dir), worktree] {
            let entry = entry_named(&super::plainly(&path));

            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }

        Root {
            account: account.to_owned(),
            entries,
        }
    }

    /// The account's credentials file, whether or not there is one.
    pub(crate) fn credentials(&self) -> PathBuf {
        self.account.join(CREDENTIALS)
    }

    /// Where the credentials file is in a root at `root`.
    pub(crate) fn credentials_in(root: &Path) -> PathBuf {
        root.join(CREDENTIALS)
    }

    /// Where the settings file is in a root at `root`.
    pub(crate) fn settings_in(root: &Path) -> PathBuf {
        root.join(SETTINGS)
    }

    /// The settings file a root is given: Verkstead's own, written as each
    /// session starts, and neither joined nor written back.
    ///
    /// Read off the account's `settings.json` as it is at this moment, so a key
    /// the human changes reaches the next session. An account with no such
    /// file, or one that does not read as JSON, still gets the bypass key — see
    /// [`settings`].
    ///
    /// Blocking: one read.
    pub(crate) fn settings(&self) -> Vec<u8> {
        settings(std::fs::read(self.account.join(SETTINGS)).ok().as_deref())
    }

    /// Make each joined entry in the account where it is not there yet.
    ///
    /// **Made in the account rather than in the root**, because what is written
    /// under one is the account's: memory a later session reads, and the
    /// transcript Verkstead follows. An entry that is not there is the ordinary
    /// case for a Repo nobody has run Claude in yet, and a join of nothing is a
    /// session that will not start.
    ///
    /// Blocking.
    pub(crate) fn made_in_account(&self) -> io::Result<()> {
        for entry in &self.entries {
            std::fs::create_dir_all(self.account.join(PROJECTS).join(entry))?;
        }

        Ok(())
    }

    /// Everything joined into a root a session finds at `inside`: each as the
    /// account's own path and the path a session finds it at.
    ///
    /// The credentials file first, and only where the account has one. A join
    /// of a file that is not there is nothing on a Mac, a hard link that fails on
    /// Windows and a bind that will not start on Linux — so a root for an
    /// account with no file has none, and the file a session logs in and writes
    /// is handed back as it ends instead.
    /// See [`super::Sandbox::command`].
    ///
    /// Blocking: one `stat`.
    pub(crate) fn joined(&self, inside: &Path) -> Vec<(PathBuf, PathBuf)> {
        let mut joined = Vec::new();

        let credentials = self.credentials();

        if credentials.is_file() {
            joined.push((credentials, Root::credentials_in(inside)));
        }

        for entry in &self.entries {
            joined.push((
                self.account.join(PROJECTS).join(entry),
                inside.join(PROJECTS).join(entry),
            ));
        }

        joined
    }
}

/// The settings a root is given, out of the account's own `settings.json` where
/// there is one to read.
///
/// **`skipDangerousModePermissionPrompt`**, which Claude Code 2.1.268 reads
/// from user settings. A session runs with its permissions bypassed and nobody
/// at its terminal, so a consent screen asking whether that is all right is a
/// session parked for ever — which is what a fresh account's first session was.
///
/// And the [`CARRIED`] keys of the account's own, as they are there. Nothing
/// else of it, and nothing at all of a file that is not a JSON object.
fn settings(account: Option<&[u8]>) -> Vec<u8> {
    let mut written = serde_json::Map::new();

    written.insert(
        "skipDangerousModePermissionPrompt".to_owned(),
        serde_json::Value::Bool(true),
    );

    if let Some(serde_json::Value::Object(own)) =
        account.and_then(|bytes| serde_json::from_slice(bytes).ok())
    {
        for key in CARRIED {
            if let Some(value) = own.get(key) {
                written.insert(key.to_owned(), value.clone());
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(written))
        .expect("a map of JSON values writes as JSON");
    bytes.push(b'\n');

    bytes
}

/// `path` resolved, or `path` as it stands where it cannot be.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// The Repo's main checkout, out of its common git directory.
///
/// The directory holding it where it is called `.git`, and the directory itself
/// where it is not — a bare repository, which has no checkout to be beside.
/// That is Claude's own rule, read off 2.1.268: it follows the Worktree's
/// `.git` file to `commondir` and makes the same choice.
fn main_checkout(git_dir: &Path) -> PathBuf {
    match (git_dir.file_name(), git_dir.parent()) {
        (Some(name), Some(checkout)) if name == ".git" => checkout.to_owned(),
        _ => git_dir.to_owned(),
    }
}

/// The name Claude gives the `projects/` entry for `path`.
///
/// Read off Claude Code 2.1.268. Every UTF-16 unit outside `[a-zA-Z0-9]` becomes
/// `-`, one for one — so a character outside the Basic Multilingual Plane is two
/// of them. A name longer than 200 units is cut to its first 200, then `-`, then
/// a hash of the path in base 36: `h = (h << 5) - h + unit` kept to 32 bits and
/// made positive, over the path as it was rather than as it was renamed.
///
/// The plain path, never a `\\?\` spelling: that is a different string, and so
/// a different entry.
pub(crate) fn entry_named(path: &Path) -> String {
    let units: Vec<u16> = path.to_string_lossy().encode_utf16().collect();

    let mut named: String = units
        .iter()
        .map(|unit| match char::from_u32(u32::from(*unit)) {
            Some(kept) if kept.is_ascii_alphanumeric() => kept,
            _ => '-',
        })
        .collect();

    if named.len() <= LONGEST {
        return named;
    }

    // Every character in `named` is one ASCII byte by now, so the byte length
    // is the unit length Claude cuts at.
    named.truncate(LONGEST);
    named.push('-');
    named.push_str(&base36(u64::from(hashed(&units).unsigned_abs())));

    named
}

/// Java's string hash over `units`, in 32 bits.
fn hashed(units: &[u16]) -> i32 {
    units.iter().fold(0i32, |hash, unit| {
        (hash << 5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(*unit))
    })
}

/// `number` in base 36, the digits lower-case, as JavaScript writes it.
fn base36(mut number: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    if number == 0 {
        return "0".to_owned();
    }

    let mut digits = Vec::new();

    while number > 0 {
        digits.push(DIGITS[(number % 36) as usize]);
        number /= 36;
    }

    digits.reverse();

    String::from_utf8(digits).expect("base 36 digits are ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name of the entry this very checkout's memory is under, which is
    /// the shape every entry has.
    #[test]
    fn an_entry_is_the_path_with_everything_but_letters_and_digits_as_dashes() {
        assert_eq!(
            entry_named(Path::new("/home/infi/src/verkstead")),
            "-home-infi-src-verkstead"
        );
        assert_eq!(
            entry_named(Path::new(
                "/var/lib/verkstead/worktrees/verkstead-built_roots.01"
            )),
            "-var-lib-verkstead-worktrees-verkstead-built-roots-01"
        );
    }

    /// One dash for each UTF-16 unit rather than for each character, which is
    /// what a JavaScript regular expression without the `u` flag replaces.
    #[test]
    fn a_character_outside_the_basic_plane_is_two_dashes() {
        assert_eq!(entry_named(Path::new("/tmp/é🦀")), "-tmp----");
    }

    /// A name longer than 200 is cut there, with the hash of the whole path on
    /// the end — the value here is what Claude Code 2.1.268 itself names it.
    #[test]
    fn a_path_longer_than_two_hundred_is_cut_and_hashed() {
        let path = format!("/{}", "a".repeat(250));

        let named = entry_named(Path::new(&path));

        assert_eq!(named.len(), 200 + 1 + "feo44x".len());
        assert_eq!(named, format!("-{}-feo44x", "a".repeat(199)));
    }

    /// The hash is made positive, including the one value whose negation does
    /// not fit in 32 bits.
    #[test]
    fn the_hash_is_javas_in_thirty_two_bits_and_written_positive() {
        let units: Vec<u16> = "hello".encode_utf16().collect();
        assert_eq!(hashed(&units), 99_162_322);

        let units: Vec<u16> = "polygenelubricants".encode_utf16().collect();
        assert_eq!(hashed(&units), i32::MIN);
        assert_eq!(base36(u64::from(i32::MIN.unsigned_abs())), "zik0zk");
    }

    #[test]
    fn a_main_checkout_is_the_directory_holding_its_git_directory() {
        assert_eq!(
            main_checkout(Path::new("/home/you/src/verkstead/.git")),
            Path::new("/home/you/src/verkstead")
        );
        assert_eq!(
            main_checkout(Path::new("/srv/git/verkstead.git")),
            Path::new("/srv/git/verkstead.git"),
            "and a bare repository is its own"
        );
    }

    fn read(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    /// An account with no settings of its own, or with some that do not read,
    /// is the account whose first session would otherwise park at the consent.
    #[test]
    fn the_bypass_key_is_written_whatever_the_account_has() {
        let bypass = serde_json::json!({ "skipDangerousModePermissionPrompt": true });

        assert_eq!(read(&settings(None)), bypass, "no settings.json at all");
        assert_eq!(
            read(&settings(Some(b"{ not json"))),
            bypass,
            "one that does not parse"
        );
        assert_eq!(
            read(&settings(Some(b"[1, 2]"))),
            bypass,
            "one that is not an object"
        );
        assert_eq!(
            read(&settings(Some(
                b"{\"skipDangerousModePermissionPrompt\": false}"
            ))),
            bypass,
            "and one that says otherwise is not asked"
        );
    }

    /// The two keys an API-key login needs come over as they are, and nothing
    /// else of the account's does.
    #[test]
    fn only_the_api_key_helper_and_the_environment_are_carried_over() {
        let account = serde_json::json!({
            "apiKeyHelper": "/usr/local/bin/print-key",
            "env": { "ANTHROPIC_BASE_URL": "https://proxy.example" },
            "hooks": { "Stop": [] },
            "enabledPlugins": { "the-humans@own": true },
            "permissions": { "allow": ["Bash"] },
            "statusLine": { "type": "command", "command": "true" },
        });

        assert_eq!(
            read(&settings(Some(account.to_string().as_bytes()))),
            serde_json::json!({
                "skipDangerousModePermissionPrompt": true,
                "apiKeyHelper": "/usr/local/bin/print-key",
                "env": { "ANTHROPIC_BASE_URL": "https://proxy.example" },
            })
        );
    }

    /// A Worktree and a Repo whose entries are one name join it once.
    #[test]
    fn the_repos_entry_and_the_worktrees_are_joined_once_each() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();

        let account = dir.path().join("account/.claude");
        let root = Root::of(Platform::Linux, &account, &repo.join(".git"), &worktree);

        root.made_in_account().unwrap();

        let joined = root.joined(Path::new("/inside/.claude"));
        let repo_entry = entry_named(&repo);
        let worktree_entry = entry_named(&worktree);

        assert_eq!(
            joined,
            [
                (
                    account.join("projects").join(&repo_entry),
                    PathBuf::from("/inside/.claude/projects").join(&repo_entry),
                ),
                (
                    account.join("projects").join(&worktree_entry),
                    PathBuf::from("/inside/.claude/projects").join(&worktree_entry),
                ),
            ],
            "no credentials file in the account, so none in the root"
        );
        assert!(account.join("projects").join(&repo_entry).is_dir());
        assert!(account.join("projects").join(&worktree_entry).is_dir());

        std::fs::write(account.join(CREDENTIALS), "{}\n").unwrap();
        assert_eq!(
            root.joined(Path::new("/inside/.claude"))[0],
            (
                account.join(CREDENTIALS),
                PathBuf::from("/inside/.claude/.credentials.json")
            )
        );

        let same = Root::of(Platform::Linux, &account, &worktree.join(".git"), &worktree);
        assert_eq!(same.entries, [worktree_entry]);
    }

    /// A path resolved on Windows carries `\\?\` in front of it, and its entry is
    /// named from the plain path a session is started in.
    #[test]
    fn a_verbatim_path_is_named_as_the_plain_path_it_spells() {
        let root = Root::of(
            Platform::Windows,
            Path::new(r"C:\Users\ada\.claude"),
            Path::new(r"\\?\C:\Users\ada\src\verkstead"),
            Path::new(r"\\?\C:\ProgramData\Verkstead\worktrees\verkstead-x"),
        );

        assert_eq!(
            root.entries,
            [
                "C--Users-ada-src-verkstead",
                "C--ProgramData-Verkstead-worktrees-verkstead-x"
            ]
        );
    }

    /// A Worktree reached through a link is named as a session will find itself
    /// standing in it: through the link on Linux, where the bind makes the path
    /// as it was stored, and resolved on a Mac, where the path is the host's.
    #[cfg(unix)]
    #[test]
    fn a_worktree_is_named_as_the_session_inside_will_read_its_path() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real/worktree");
        let linked = dir.path().join("linked");
        std::fs::create_dir_all(&real).unwrap();
        std::os::unix::fs::symlink(dir.path().join("real"), &linked).unwrap();

        let account = dir.path().join("account/.claude");
        let git_dir = dir.path().join("repo/.git");
        let through = linked.join("worktree");

        assert_eq!(
            Root::of(Platform::Linux, &account, &git_dir, &through).entries[1],
            entry_named(&through)
        );
        assert_eq!(
            Root::of(Platform::MacOs, &account, &git_dir, &through).entries[1],
            entry_named(&real.canonicalize().unwrap())
        );
    }
}
