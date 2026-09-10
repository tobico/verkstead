//! The identity a Windows session runs as: a **local account of Verkstead's
//! own**, and what it takes to have one.
//!
//! **Why an account rather than anything narrower** (ADR-0014, *Amended: the
//! Sandbox is an account*). Three mechanisms were tried before this one and
//! each broke the agent — an AppContainer identity refuses a path *resolution*
//! however well the path is granted, and a restricted SID list, a deny-only
//! account SID and an integrity ceiling each break node, both PowerShells or
//! `bash`. What they have in common is that the process is not quite somebody:
//! a stranger's identity, a second access check, or a ceiling. An ordinary
//! local account is none of those, because from the machine's point of view
//! there is nothing unusual about it — and what a session reaches is then
//! exactly what that account has been granted, which is the human's own files
//! not at all until an entry says so.
//!
//! **What an account is here** is three things said together: the name it goes
//! by on this machine, the password Verkstead keeps for it, and the SID that
//! [`super::granting`]'s entries are written for. The first is arithmetic, the
//! second is in `secrets.yaml` beside the GitHub token, and the third is what
//! the machine answers when it is asked about the first.
//!
//! **The name is fingerprinted, and short.** Two Verksteads on one machine keep
//! their accounts apart the way they keep their pipes apart — see
//! [`crate::pipe`], whose `verkstead-{:016x}` is the scheme this follows. It
//! cannot follow it exactly: a local account name is capped at [`MOST`]
//! characters and that one is 26, so both the prefix and the number of hex
//! digits come down. What is left is still a name off the Data Directory and
//! nothing else, so it is the same name every time for one directory and a
//! different one for the next — and it is written down in the record beside the
//! entries, so a server that did not make the account can still say which one
//! its entries are for.
//!
//! **The password is Verkstead's to keep** and nobody's to type.
//! `CreateProcessWithLogonW` needs one, and there is no passwordless route to
//! another account's token without a privilege only the operating system holds
//! — so one is generated with the operating system's own generator when the
//! account is made, and kept in the file that is already written 0600 and is
//! already in no sandbox. Which is why [`crate::settings::Settings::save_secrets`]
//! no longer empties that file whenever there is no GitHub token: the first
//! person to clear a token would otherwise have taken this away with it.
//!
//! **Making one needs elevation, and reading one does not.** Creating a local
//! account is an administrator's call, so it is a verb of Verkstead's own run
//! once from an elevated terminal rather than anything a server does — see
//! [`machine::create`] and [`machine::remove`], which are the two halves of
//! that, and which refuse with a line naming what they need rather than a Win32
//! error code when they are run without it. The server is never elevated and
//! only ever *reads*: [`machine::Account::on_this_machine`] resolves the name
//! to a SID and hands back a [`Missing`] saying plainly which half is not
//! there, which is what will later refuse a session.
//!
//! **The arithmetic is every platform's and the calls are Windows'.** What an
//! account is *called* is a fact about a path, so [`named`] and [`password`]
//! are built and tested wherever the tests run — the way
//! [`super::granting::entries`] is — and [`machine`], which is the whole of
//! what touches the machine's own account database, is compiled where there is
//! one.

// The Win32 half: the account database this machine keeps, and everything that
// is a call into it rather than a fact about a Data Directory.
#[cfg(windows)]
pub mod machine;

use std::fmt;
use std::path::Path;

/// The most characters a local account name may have on this platform.
///
/// Windows' own cap, and the whole reason the name below is not the pipe's.
/// Checked rather than assumed — see the test at the foot of this module — so
/// that a prefix somebody lengthens is caught here rather than by `NetUserAdd`
/// on a machine nobody is watching.
pub const MOST: usize = 20;

/// The account a process is started as, as a description carries one: the name
/// this machine knows it by and the password `CreateProcessWithLogonW` is
/// given.
///
/// **What crosses the seam**, the way a container's SID is what crosses it —
/// see [`super::rendering::Rendering::as_account`]. A [`machine::Account`] is
/// three things and one of them is a SID, which is a block of bytes this
/// machine resolved and which nothing off it can read; a rendering is a
/// description that is copied, compared and built on machines that have no such
/// account at all. So what travels is the two words a logon is made of, and the
/// SID stays where it was resolved.
///
/// **The password is in it because there is nowhere else for it to be.**
/// `CreateProcessWithLogonW` takes one and there is no passwordless route to
/// another account's token, and the two places that start a process are handed
/// a rendering and nothing else — see [`super::starting`] and
/// [`crate::terminal`]. What that costs is guarded rather than avoided: the
/// [`fmt::Debug`] below is written rather than derived, so a rendering in a log
/// line or in a failed assertion says the name and never the password.
#[derive(Clone, PartialEq, Eq)]
pub struct Logon {
    name: String,
    password: String,
}

/// The name, and the password said to be there rather than said.
///
/// Derived, this would put the password into every log line and every failed
/// assertion that carried a rendering — which is a secret leaving the file it
/// is kept 0600 in by the most ordinary route there is. The same reason
/// `machine::Account`'s own is written out.
impl fmt::Debug for Logon {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Logon")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Logon {
    /// The account called `name`, whose password is `password`.
    pub fn of(name: impl Into<String>, password: impl Into<String>) -> Logon {
        Logon {
            name: name.into(),
            password: password.into(),
        }
    }

    /// What the account is called on this machine.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// And the password a logon as it is made with.
    pub fn password(&self) -> &str {
        &self.password
    }
}

/// What every session account's name begins with, so that a human looking at
/// the accounts on their machine can see whose it is.
///
/// Shorter than the pipe's `verkstead-` because [`MOST`] leaves no room for
/// that one and a fingerprint worth having. Not a word anything else here uses,
/// and not one an ordinary account is likely to be called.
const PREFIX: &str = "vk-";

/// And how many hex digits of the fingerprint follow it.
///
/// Fewer than the pipe's sixteen, for the same reason the prefix is shorter.
/// Forty-eight bits of a Data Directory's path is not a hash to build anything
/// on and is not being asked to be: what it separates is the two or three
/// Verksteads a machine might have, and what it costs to be wrong is that the
/// second one is refused a name the first is holding.
const DIGITS: usize = 12;

/// The name the account belonging to `data_dir` goes by on this machine.
///
/// Every character after the prefix comes off the Data Directory, so this is
/// the same name every time for one directory and a different one for the next
/// — and it is at most [`MOST`] characters, which is what a local account name
/// may be.
pub fn named(data_dir: &Path) -> String {
    // Through the resolved path rather than the one that was typed, for the
    // reason the pipe's name is: `.` and the absolute name of the same
    // directory are one Data Directory, and two servers pointed at it by those
    // two spellings have to arrive at one account. A directory that will not
    // resolve is one nothing has made yet — the caller makes it before asking —
    // and the name it was asked by is the honest answer.
    let settled = data_dir.canonicalize();
    let settled = settled.as_deref().unwrap_or(data_dir);

    // The low digits of the fingerprint, which are as good as any of them: FNV
    // is an avalanche the whole way, so a suffix of one is not a worse
    // separator than the whole.
    let kept = fingerprint(settled) & ((1 << (DIGITS * 4)) - 1);

    format!("{PREFIX}{kept:0DIGITS$x}")
}

/// `path` as one number, by FNV-1a over the way Windows itself spells it.
///
/// Written out rather than taken from a hashing crate or from the standard
/// library's own hasher, for the reason [`crate::pipe`]'s is: what this decides
/// is the name of an account two runs of Verkstead have to agree on, so it has
/// to be the same number in a year's time as it is today, and `DefaultHasher`
/// promises exactly the opposite.
///
/// Over the UTF-16 of the path rather than over its bytes, which is the same
/// arithmetic the pipe's name is — said portably, so that a machine with no
/// `encode_wide` can still be asked what a Windows session's account would be
/// called. The two agree on every path a Windows machine really has: a path
/// that is not valid UTF-16 is not one this platform can hand back.
fn fingerprint(path: &Path) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET;

    for unit in path.to_string_lossy().encode_utf16() {
        for byte in unit.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }

    hash
}

/// A password for a new account: long, random, and never seen by anybody.
///
/// The operating system's own generator and nothing around it, which is the
/// same call a Conversation's branch name is picked with — see the `getrandom`
/// line in the workspace manifest. Nobody types this and nobody reads it back
/// off a screen: it is written straight into `secrets.yaml` by the verb that
/// made the account.
///
/// A digit and a letter of each case in front of it, so that a machine with a
/// password policy on it takes this without argument — the alphabet below is
/// otherwise capable of producing thirty-two lowercase letters in a row, and
/// `NetUserAdd` would answer that with a number rather than with a reason.
pub fn password() -> String {
    const FROM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const LONG: usize = 32;

    let mut bytes = [0u8; LONG];
    getrandom::fill(&mut bytes).expect("the operating system's generator");

    let mut said = String::from("Vk1-");

    for byte in bytes {
        said.push(FROM[byte as usize % FROM.len()] as char);
    }

    said
}

/// The word a refusal tells somebody to run, said once so that every refusal
/// says it the same way.
pub const MAKE_IT: &str = "verkstead session-account create";

/// Why the account this Data Directory's sessions would run as cannot be had.
///
/// Two halves and therefore two answers: an account the machine has never heard
/// of, and an account nothing knows the password of. Both are the elevated verb
/// not having been run — the second is also a `secrets.yaml` somebody has
/// emptied by hand — and both say so, because a session refused with a Win32
/// number is a session nobody can do anything about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    /// There is no such account on this machine, and what the machine said when
    /// it was asked.
    Account { name: String, why: String },

    /// The account is there and `secrets.yaml` has no password for it, so
    /// nothing can start a process as it.
    Password { name: String },
}

impl fmt::Display for Missing {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Missing::Account { name, why } => write!(
                out,
                "there is no local account {name} for this Data Directory to run sessions as \
                 ({why}) — run `{MAKE_IT}` from an elevated terminal",
            ),
            Missing::Password { name } => write!(
                out,
                "the local account {name} is on this machine and secrets.yaml has no password \
                 for it, so nothing can start a process as it — run `{MAKE_IT}` from an \
                 elevated terminal",
            ),
        }
    }
}

impl std::error::Error for Missing {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{DIGITS, MOST, Missing, PREFIX, named, password};

    #[test]
    fn a_name_is_short_enough_to_be_one() {
        let name = named(Path::new("/home/somebody/.local/share/verkstead"));

        assert!(
            name.len() <= MOST,
            "{name} is {} characters and a local account name may be {MOST}",
            name.len(),
        );
    }

    #[test]
    fn a_name_says_whose_it_is_and_then_says_which() {
        let name = named(Path::new("/home/somebody/.local/share/verkstead"));

        assert!(
            name.starts_with(PREFIX),
            "{name} should begin with {PREFIX}"
        );
        assert_eq!(name.len(), PREFIX.len() + DIGITS);
        assert!(
            name[PREFIX.len()..]
                .chars()
                .all(|char| char.is_ascii_hexdigit()),
            "{name} should be the prefix and hex digits and nothing else",
        );
    }

    #[test]
    fn one_data_directory_is_one_name() {
        assert_eq!(
            named(Path::new("/home/somebody/.local/share/verkstead")),
            named(Path::new("/home/somebody/.local/share/verkstead")),
        );
    }

    #[test]
    fn two_data_directories_are_two_names() {
        assert_ne!(
            named(Path::new("/home/somebody/.local/share/verkstead")),
            named(Path::new(
                "/home/somebody/.local/share/verkstead-the-second"
            )),
            "two Verksteads on one machine have to keep their accounts apart",
        );
    }

    /// A path that will not resolve is answered by the path itself rather than
    /// by a panic: a Data Directory nothing has made yet still has a name.
    #[test]
    fn a_directory_that_is_not_there_yet_still_has_a_name() {
        let name = named(Path::new("/nowhere/at/all/verkstead"));

        assert!(name.starts_with(PREFIX));
        assert!(name.len() <= MOST);
    }

    #[test]
    fn a_password_is_long_and_is_never_the_same_one_twice() {
        let first = password();
        let second = password();

        assert_ne!(first, second);
        assert_eq!(first.len(), "Vk1-".len() + 32);
        assert!(
            first
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-'),
            "{first} should be a password a command line can carry",
        );
        assert!(first.chars().any(|char| char.is_ascii_uppercase()));
        assert!(first.chars().any(|char| char.is_ascii_lowercase()));
        assert!(first.chars().any(|char| char.is_ascii_digit()));
    }

    /// Both refusals name the verb, because a refusal nobody can act on is a
    /// refusal that has not said anything.
    #[test]
    fn a_refusal_says_what_to_run() {
        let missing = Missing::Account {
            name: "vk-0123456789ab".to_owned(),
            why: "the machine has no such account".to_owned(),
        };

        assert!(missing.to_string().contains("vk-0123456789ab"));
        assert!(missing.to_string().contains(super::MAKE_IT));

        let missing = Missing::Password {
            name: "vk-0123456789ab".to_owned(),
        };

        assert!(missing.to_string().contains("secrets.yaml"));
        assert!(missing.to_string().contains(super::MAKE_IT));
    }
}
