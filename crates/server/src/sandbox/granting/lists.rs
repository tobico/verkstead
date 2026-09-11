//! An access-control list read as the bytes it is, and the one question a step
//! asks of one before it is written: can the session account walk through this
//! directory already?
//!
//! **Read the same on every platform**, for [`super`]'s own reason. What a list
//! says is a fact about its bytes, and only fetching one off a directory is a
//! Win32 call — which [`super::writing`] makes, handing the bytes here. So a
//! test on a Linux box can hand this the list Windows ships on `C:\` and ask
//! what a boundary would write there, the way [`super::entries`]'s own tests
//! ask what a boundary is.
//!
//! **And the reading of an entry is here with it**: an allow or a deny is a
//! header, a mask and a SID, and [`super::writing`] reads the same three out of
//! the same bytes for every list it rebuilds.

/// The two ACE types whose SID is where this reads one.
///
/// An allow and a deny are the same shape — a header, a mask, and the SID after
/// them — which is what makes [`whose`] one function rather than two. Anything
/// else in a list is copied as it stands and never asked about: an object ACE
/// carries two more fields before its SID and is a thing directories in Active
/// Directory have rather than files on a disk.
pub(crate) const ALLOWED: u8 = 0;
pub(crate) const DENIED: u8 = 1;

/// How far past an ACE's start its SID begins, on both of the types above: four
/// bytes of header and four of mask.
pub(crate) const THE_SID: usize = 8;

/// And the one flag of a header [`super::writing`] reads: the bit that says an
/// entry is the parent's rather than this path's own.
///
/// Written here rather than taken from Win32's own `INHERITED_ACE`, which is
/// the same bit in the flag *word* a call is handed: a header keeps its flags
/// in one byte, and a constant that had to be narrowed at every use would be a
/// conversion standing in front of a bit test.
pub(crate) const INHERITED: u8 = 0x10;

/// And the flag that says an entry is on a directory only to be handed down to
/// what is made under it, granting nothing on the directory itself — which is
/// how `C:\` gives `Authenticated Users` the right to change what is under it
/// without giving them anything of the drive.
const INHERIT_ONLY: u8 = 0x08;

/// The rights a step is written with — see [`super::writing`], whose `stepped`
/// says why each of the three — spelled as the numbers a list holds, because
/// the Win32 names for them are one platform's. That file checks this spelling
/// against those names, being where both are in reach.
pub(crate) const STEP: u32 = FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE;

const FILE_TRAVERSE: u32 = 0x0000_0020;
const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
const SYNCHRONIZE: u32 = 0x0010_0000;

/// The generic rights, and what each comes to on a file or a directory.
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const GENERIC_EXECUTE: u32 = 0x2000_0000;
const GENERIC_ALL: u32 = 0x1000_0000;

const FILE_GENERIC_READ: u32 = 0x0012_0089;
const FILE_GENERIC_WRITE: u32 = 0x0012_0116;
const FILE_GENERIC_EXECUTE: u32 = 0x0012_00a0;
const FILE_ALL_ACCESS: u32 = 0x001f_01ff;

/// The three identities every session's token carries beside the account's
/// own, as the bytes a list holds for each.
///
/// `Everyone` and `Authenticated Users` are on every logon there is; `Users` is
/// what making the account with `USER_PRIV_USER` puts it in — see
/// [`super::super::account::machine`]. Nothing narrower is read for: an entry
/// for `INTERACTIVE` or for the logon's own session grants a session as well,
/// and is read past, which only ever costs a step written that need not have
/// been.
pub(crate) const EVERYONE: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
pub(crate) const AUTHENTICATED_USERS: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 5, 11, 0, 0, 0];
pub(crate) const USERS: &[u8] = &[1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 0x21, 0x02, 0, 0];

const EVERYBODY: [&[u8]; 3] = [EVERYONE, AUTHENTICATED_USERS, USERS];

/// What a directory's list came to when the machine was asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Found {
    /// Nothing the machine would say, which says nothing about who reaches it.
    Unread,

    /// No list at all, which is a directory everybody reaches.
    Open,

    /// The entries it holds, each as the bytes it is and in the order it holds
    /// them.
    Listed(Vec<Vec<u8>>),
}

/// Whether a directory whose list is `found` already lets the account whose SID
/// is `account` walk through it and read its attributes — which is the whole of
/// what a step would grant it.
///
/// **Read the way the machine's own access check reads a list**: in order, each
/// of the three rights settled by the first entry that speaks to it for an
/// identity the account's token carries — the account itself, and the three in
/// [`EVERYONE`]'s company. An allow after a deny grants nothing the deny
/// refused; rights granted by two entries between them are granted; and an
/// entry that is only handed down grants nothing here.
///
/// **A list that would not read is no**, because nothing about who reaches a
/// directory can be said of it, and the step is written as it always was.
/// **No list at all is yes**: a directory with none is one everybody reaches,
/// and a step written there would be a list where there was none — which is
/// everybody it does not name refused.
pub(crate) fn steps_through(found: &Found, account: &[u8]) -> bool {
    let list = match found {
        Found::Unread => return false,
        Found::Open => return true,
        Found::Listed(list) => list,
    };

    let (mut allowed, mut denied) = (0u32, 0u32);

    for ace in list {
        let Some(theirs) = whose(ace) else {
            continue;
        };

        if ace[1] & INHERIT_ONLY != 0 || !carried(theirs, account) {
            continue;
        }

        let unsettled = specific(mask(ace)) & STEP & !(allowed | denied);

        if ace[0] == ALLOWED {
            allowed |= unsettled;
        } else {
            denied |= unsettled;
        }
    }

    allowed == STEP
}

/// Whether the SID an entry names is one the account's token carries.
///
/// The bytes rather than the spelling, because that is what a list holds. A SID
/// says how many parts it has in its second byte, so one that begins with the
/// whole of another is that other and nothing longer.
fn carried(theirs: &[u8], account: &[u8]) -> bool {
    std::iter::once(account)
        .chain(EVERYBODY)
        .any(|sid| !sid.is_empty() && theirs.starts_with(sid))
}

/// What an entry's mask comes to on a file or a directory, generic rights and
/// all.
///
/// An entry held on a directory is ordinarily written in specific rights, and a
/// generic one is what an inherit-only entry carries for whatever it is handed
/// down to — but nothing forbids one on the directory itself, and a right read
/// past for being spelled generically is a step written for nothing.
fn specific(mask: u32) -> u32 {
    [
        (GENERIC_READ, FILE_GENERIC_READ),
        (GENERIC_WRITE, FILE_GENERIC_WRITE),
        (GENERIC_EXECUTE, FILE_GENERIC_EXECUTE),
        (GENERIC_ALL, FILE_ALL_ACCESS),
    ]
    .into_iter()
    .filter(|(generic, _)| mask & generic != 0)
    .fold(mask, |so_far, (_, on_a_file)| so_far | on_a_file)
}

/// Whether an entry is one the path was handed from above rather than one of
/// its own.
pub(crate) fn inherited(ace: &[u8]) -> bool {
    ace.get(1).is_some_and(|flags| flags & INHERITED != 0)
}

/// What one entry of a list allows or denies.
pub(crate) fn mask(ace: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&ace[4..THE_SID]);

    u32::from_le_bytes(bytes)
}

/// And whose it is, where it is one of the two shapes whose SID is where this
/// reads one — see [`ALLOWED`] and [`DENIED`].
pub(crate) fn whose(ace: &[u8]) -> Option<&[u8]> {
    (matches!(ace[0], ALLOWED | DENIED) && ace.len() > THE_SID).then(|| &ace[THE_SID..])
}

/// Lists as a Windows machine ships them, for the tests here and in [`super`]
/// that ask what a boundary writes with no machine to write on — each one what
/// `icacls` says of that directory on a Windows 11 machine.
#[cfg(test)]
pub(crate) mod shipped {
    use super::*;

    pub(crate) const SYSTEM: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
    pub(crate) const ADMINISTRATORS: &[u8] =
        &[1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 0x20, 0x02, 0, 0];
    pub(crate) const CREATOR_OWNER: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0];

    /// A human's own account, `S-1-5-21-1-2-3-1001`.
    pub(crate) const ADA: &[u8] = &[
        1, 5, 0, 0, 0, 0, 0, 5, 21, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0xe9, 0x03, 0, 0,
    ];

    /// And the session account made beside it, `S-1-5-21-1-2-3-1003`.
    pub(crate) const SESSIONS: &[u8] = &[
        1, 5, 0, 0, 0, 0, 0, 5, 21, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0xeb, 0x03, 0, 0,
    ];

    /// Handed down to what is made under a directory, files and directories
    /// both, as well as held on it.
    pub(crate) const BOTH_WAYS: u8 = 0x03;

    /// And the half of that which reaches a directory made underneath.
    const TO_DIRECTORIES: u8 = 0x02;

    pub(crate) const FULL: u32 = 0x001f_01ff;
    pub(crate) const READ_AND_EXECUTE: u32 = 0x0012_00a9;
    const MODIFY: u32 = 0x0013_01bf;
    const APPEND: u32 = 0x0000_0004;
    const ADD_AND_WRITE_ATTRIBUTES: u32 = 0x0000_0116;

    /// One entry: a kind, the flags, a mask and whose it is.
    pub(crate) fn ace(kind: u8, flags: u8, mask: u32, sid: &[u8]) -> Vec<u8> {
        let mut ace = vec![kind, flags, 0, 0];

        ace.extend_from_slice(&mask.to_le_bytes());
        ace.extend_from_slice(sid);

        let size = u16::try_from(ace.len()).expect("an entry this size");
        ace[2..4].copy_from_slice(&size.to_le_bytes());

        ace
    }

    /// `C:\`.
    pub(crate) fn drive_root() -> Vec<Vec<u8>> {
        vec![
            ace(ALLOWED, BOTH_WAYS, FULL, ADMINISTRATORS),
            ace(ALLOWED, BOTH_WAYS, FULL, SYSTEM),
            ace(ALLOWED, BOTH_WAYS, READ_AND_EXECUTE, USERS),
            ace(
                ALLOWED,
                BOTH_WAYS | INHERIT_ONLY,
                MODIFY,
                AUTHENTICATED_USERS,
            ),
            ace(ALLOWED, 0, APPEND, AUTHENTICATED_USERS),
        ]
    }

    /// `C:\Users`, which `Users` and `Everyone` are each granted on the
    /// directory itself, beside what they are handed below it.
    pub(crate) fn users() -> Vec<Vec<u8>> {
        vec![
            ace(ALLOWED, BOTH_WAYS, FULL, SYSTEM),
            ace(ALLOWED, BOTH_WAYS, FULL, ADMINISTRATORS),
            ace(ALLOWED, 0, READ_AND_EXECUTE, USERS),
            ace(
                ALLOWED,
                BOTH_WAYS | INHERIT_ONLY,
                GENERIC_READ | GENERIC_EXECUTE,
                USERS,
            ),
            ace(ALLOWED, 0, READ_AND_EXECUTE, EVERYONE),
            ace(
                ALLOWED,
                BOTH_WAYS | INHERIT_ONLY,
                GENERIC_READ | GENERIC_EXECUTE,
                EVERYONE,
            ),
        ]
    }

    /// `C:\ProgramData`.
    pub(crate) fn program_data() -> Vec<Vec<u8>> {
        vec![
            ace(ALLOWED, BOTH_WAYS, FULL, SYSTEM),
            ace(ALLOWED, BOTH_WAYS, FULL, ADMINISTRATORS),
            ace(ALLOWED, BOTH_WAYS | INHERIT_ONLY, FULL, CREATOR_OWNER),
            ace(ALLOWED, BOTH_WAYS, READ_AND_EXECUTE, USERS),
            ace(ALLOWED, TO_DIRECTORIES, ADD_AND_WRITE_ATTRIBUTES, USERS),
        ]
    }

    /// A human's own profile, whose list is protected: nothing of `C:\Users`
    /// reaches it, and it gives the machine, its administrators and its human
    /// everything and nobody else anything.
    pub(crate) fn profile() -> Vec<Vec<u8>> {
        vec![
            ace(ALLOWED, BOTH_WAYS, FULL, SYSTEM),
            ace(ALLOWED, BOTH_WAYS, FULL, ADMINISTRATORS),
            ace(ALLOWED, BOTH_WAYS, FULL, ADA),
        ]
    }

    /// What a directory made under one holding `list` holds: every entry handed
    /// down to directories, held on it now, and marked as taken from above.
    pub(crate) fn handed_down(list: &[Vec<u8>]) -> Vec<Vec<u8>> {
        list.iter()
            .filter(|ace| ace[1] & TO_DIRECTORIES != 0)
            .map(|ace| {
                let mut down = ace.clone();
                down[1] = (down[1] & !INHERIT_ONLY) | INHERITED;
                down
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::shipped::*;
    use super::*;

    /// An entry's SID is read where an entry keeps one, which is the offset
    /// every list built or read here turns on.
    #[test]
    fn an_entry_is_read_as_a_header_a_mask_and_a_sid() {
        let mut ace = vec![ALLOWED, 3, 20, 0];
        ace.extend_from_slice(&0x001f_01ffu32.to_le_bytes());
        ace.extend_from_slice(&[1, 2, 3, 4]);

        assert_eq!(mask(&ace), 0x001f_01ff);
        assert_eq!(whose(&ace), Some(&[1u8, 2, 3, 4][..]));

        // And an entry of a shape whose SID is somewhere else is one nothing
        // here claims to know — see [`ALLOWED`].
        assert_eq!(whose(&[5, 3, 8, 0, 0, 0, 0, 0, 9]), None);
    }

    /// The drive root as Windows ships it is one every session walks through
    /// already — and it is the entry for `Users` that says so.
    #[test]
    fn the_drive_root_as_windows_ships_it_is_walked_through_already() {
        assert!(
            steps_through(&Found::Listed(drive_root()), SESSIONS),
            "`Users` may read and execute on the drive root, which is a walk and an \
             attribute read and more",
        );

        let without_users = drive_root()
            .into_iter()
            .filter(|ace| whose(ace) != Some(USERS))
            .collect();

        assert!(
            !steps_through(&Found::Listed(without_users), SESSIONS),
            "and nothing else on it says so: `Authenticated Users` holds one entry that is \
             only handed down to what is under the drive and another that appends and \
             nothing else",
        );
    }

    /// And the directory the profiles are in, and the one the machine keeps
    /// its programs' data in — the other two public directories most paths are
    /// found under.
    #[test]
    fn the_public_directories_under_the_drive_are_walked_through_already() {
        assert!(
            steps_through(&Found::Listed(users()), SESSIONS),
            "`C:\\Users` grants `Users` and `Everyone` a read on the directory itself",
        );
        assert!(
            steps_through(&Found::Listed(program_data()), SESSIONS),
            "`C:\\ProgramData` grants `Users` a read and hands it down",
        );
        assert!(
            steps_through(&Found::Listed(handed_down(&program_data())), SESSIONS),
            "and a directory under it holds that read as one taken from above, which is a \
             read all the same",
        );
    }

    /// A human's own profile gives nobody else anything, and neither does what
    /// is under it — which is what a step exists for.
    #[test]
    fn a_humans_own_profile_is_not_walked_through_until_a_step_says_so() {
        assert!(
            !steps_through(&Found::Listed(profile()), SESSIONS),
            "the profile gives the machine, its administrators and its human everything, \
             and a session nothing",
        );
        assert!(
            !steps_through(&Found::Listed(handed_down(&profile())), SESSIONS),
            "and what is under it takes exactly that from it",
        );
        assert!(
            steps_through(&Found::Listed(profile()), ADA),
            "while the human it belongs to walks through it: what is read for is the \
             account asked about, rather than whoever",
        );

        let mut stepped = profile();
        stepped.push(ace(ALLOWED, 0, STEP, SESSIONS));

        assert!(
            steps_through(&Found::Listed(stepped), SESSIONS),
            "and a step an earlier boundary wrote for the account is the account walking \
             through it already",
        );
    }

    /// A list is read the way the machine's own access check reads one: in
    /// order, each right settled by the first entry that speaks to it.
    #[test]
    fn a_list_is_read_in_order_with_each_right_settled_by_the_first_entry_to_speak_to_it() {
        let read = |list: Vec<Vec<u8>>| steps_through(&Found::Listed(list), SESSIONS);

        assert!(
            !read(vec![
                ace(DENIED, 0, FILE_TRAVERSE, EVERYONE),
                ace(ALLOWED, 0, READ_AND_EXECUTE, USERS),
            ]),
            "a walk refused before it is granted is refused",
        );
        assert!(
            read(vec![
                ace(ALLOWED, 0, READ_AND_EXECUTE, USERS),
                ace(DENIED, 0, FILE_TRAVERSE, EVERYONE),
            ]),
            "and one granted before it is refused is granted",
        );
        assert!(
            read(vec![
                ace(ALLOWED, 0, FILE_TRAVERSE | SYNCHRONIZE, AUTHENTICATED_USERS),
                ace(ALLOWED, 0, FILE_READ_ATTRIBUTES, EVERYONE),
            ]),
            "rights granted by two entries between them are granted",
        );
        assert!(
            read(vec![ace(ALLOWED, 0, GENERIC_READ | GENERIC_EXECUTE, USERS)]),
            "and a right spelled generically is the right it comes to on a directory",
        );
        assert!(
            !read(vec![ace(
                ALLOWED,
                0,
                FILE_TRAVERSE | FILE_READ_ATTRIBUTES,
                USERS
            )]),
            "a walk and an attribute read with nothing to open the directory with is short \
             of a step — see `stepped` for why it takes all three",
        );
        assert!(
            !read(vec![ace(
                ALLOWED,
                BOTH_WAYS | INHERIT_ONLY,
                READ_AND_EXECUTE,
                USERS
            )]),
            "and an entry that is only handed down grants nothing on the directory it is on",
        );
        assert!(
            !read(vec![ace(ALLOWED, BOTH_WAYS, FULL, ADA)]),
            "nor does one for somebody the account is not",
        );
    }

    /// And the two answers that are not a list at all.
    #[test]
    fn no_list_is_walked_through_and_a_list_that_would_not_read_is_not() {
        assert!(
            steps_through(&Found::Open, SESSIONS),
            "a directory with no list is one everybody reaches",
        );
        assert!(
            !steps_through(&Found::Unread, SESSIONS),
            "and one whose list would not read is one nothing can be said about, so its step \
             is written as it always was",
        );
    }
}
