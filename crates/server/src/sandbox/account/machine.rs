//! This machine's own account database: making the session account, taking it
//! away, and resolving the one that is there.
//!
//! The Win32 half of [`super`], and the whole of what touches the machine.
//! What an account is *called* is arithmetic over a Data Directory and is next
//! door; everything here is a call, and none of it is built where there is no
//! such call to make.
//!
//! **Two of these need elevation and one does not**, which is the split the
//! whole design turns on. Creating a local account and deleting one are an
//! administrator's, so [`create`] and [`remove`] are run once from an elevated
//! terminal by a verb of Verkstead's own — and refuse with a line naming what
//! they need rather than with error 5 when they are not. Reading one is
//! nobody's privilege at all, so [`Account::on_this_machine`] is what the
//! server does, never elevated, every time it starts a session.
//!
//! **What the account is made with** (ADR-0014, *Amended: the Sandbox is an
//! account*): an ordinary user account, in no group but `Users`, holding a long
//! random password that does not expire and that it cannot change. It is a name
//! to run as rather than one anybody signs in with, and [`kept_off_the_screen`]
//! and [`denied`] are the two halves of saying so.
//!
//! **And taking it away takes the profile directory with it.** Starting a
//! process as the account loads its profile, which makes `C:\Users\<account>`
//! at the first session and leaves it there — the account being deleted does
//! not take it. So [`remove`] deletes the profile first, while there is still a
//! SID to name it by, and the account after: a machine that has run sessions
//! and then had Verkstead taken off it should look as it did.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::ptr;

use anyhow::{Context, Result, anyhow, bail};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, LocalFree, NTSTATUS,
};
use windows_sys::Win32::NetworkManagement::NetManagement::{
    NERR_UserExists, NetUserAdd, NetUserDel, NetUserSetInfo, UF_DONT_EXPIRE_PASSWD,
    UF_PASSWD_CANT_CHANGE, UF_SCRIPT, USER_INFO_1, USER_INFO_1003, USER_PRIV_USER,
};
use windows_sys::Win32::Security::Authentication::Identity::{
    LSA_OBJECT_ATTRIBUTES, LSA_UNICODE_STRING, LsaAddAccountRights, LsaClose,
    LsaNtStatusToWinError, LsaOpenPolicy, POLICY_CREATE_ACCOUNT, POLICY_LOOKUP_NAMES,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{
    GetTokenInformation, LookupAccountNameW, PSID, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::DeleteProfileW;

use super::super::starting::{Handle, wide};
use super::{MAKE_IT, Missing, named, password};
use crate::settings::{Secrets, Settings};

/// Where Windows keeps the accounts it is not to draw on the sign-in screen.
///
/// A value of nought under this key is the documented way to say *this is not
/// somebody who signs in*, and it is the half of "denied interactive logon"
/// that can be said without denying Verkstead itself — see [`denied`], which is
/// the other half and the reason.
const OFF_THE_SCREEN: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList";

/// Every way of logging on that this account is denied outright.
///
/// **Every one there is, except the one Verkstead itself needs.**
/// `CreateProcessWithLogonW` performs an *interactive* logon — that is what
/// `LOGON_WITH_PROFILE` is — so `SeDenyInteractiveLogonRight` would refuse the
/// only thing this account exists to be, and would refuse it with the same
/// error a wrong password gives. What is left is every way a person or a
/// service actually reaches a machine: over the network, over Remote Desktop,
/// as a scheduled task, and as a service. Together with
/// [`kept_off_the_screen`] that is the whole of *a name to run as rather than
/// one anybody signs in with* — what remains is a local interactive logon by
/// somebody who already knows a thirty-six character password that is only ever
/// written to a file readable by one account.
/// What `NetUserAdd` says when the account is already on the machine.
///
/// Said again here under a name a pattern may carry: Win32's own spelling of
/// it is mixed case, and a mixed-case name in a `match` arm is a binding rather
/// than a comparison — which would quietly swallow every other answer the call
/// could give.
const ALREADY_THERE: u32 = NERR_UserExists;

const DENIED: &[&str] = &[
    "SeDenyNetworkLogonRight",
    "SeDenyRemoteInteractiveLogonRight",
    "SeDenyBatchLogonRight",
    "SeDenyServiceLogonRight",
];

/// The account this Data Directory's Windows sessions run as, resolved.
///
/// All three halves of one: the name, the password `CreateProcessWithLogonW`
/// will be given, and the SID [`super::super::granting`]'s entries are written
/// for.
pub struct Account {
    name: String,
    password: String,
    sid: Sid,
}

/// The name and the SID, and never the password.
///
/// Derived, this would put a password into every log line and every failed
/// assertion that carried an account — which is a secret leaving the file it is
/// kept 0600 in by the most ordinary route there is.
impl std::fmt::Debug for Account {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Account")
            .field("name", &self.name)
            .field("sid", &self.sid)
            .finish_non_exhaustive()
    }
}

impl Account {
    /// The account `data_dir`'s sessions run as, or which half of it is not
    /// there.
    ///
    /// **The server's own call, and a read from end to end.** Nothing here
    /// needs a privilege an ordinary process of the human's has not got: a name
    /// is arithmetic, a SID is a lookup, and the password is a file this
    /// process already owns. What it hands back where either half is absent is
    /// a [`Missing`] that says which, in words somebody can act on — see
    /// [`Missing`], and the verb it names.
    pub fn on_this_machine(data_dir: &Path, secrets: &Secrets) -> Result<Account, Missing> {
        Account::called(
            named(data_dir),
            secrets.session_account_password().unwrap_or_default(),
        )
    }

    /// And the same read of an account a description already carries the two
    /// portable halves of — see [`super::Logon`], which is the name and the
    /// password and never the SID.
    ///
    /// **What a session start asks**, and the reason a sandbox holds a `Logon`
    /// rather than an `Account`: the name is arithmetic over a Data Directory
    /// and the password is a file, so both are settled wherever a description is
    /// built, and only the SID is a question for the machine the session runs
    /// on. This is where that question is put.
    pub fn resolving(logon: &super::Logon) -> Result<Account, Missing> {
        Account::called(logon.name().to_owned(), logon.password())
    }

    /// The same read, of an account said rather than worked out.
    ///
    /// Named apart from [`Account::on_this_machine`] and
    /// [`Account::resolving`] so that both refusals can be asked for: an
    /// account this machine has never heard of is any name at all, and an
    /// account it *has* heard of is one the suite cannot make without an
    /// elevation it has not got — so the second half is asked about an account
    /// every Windows machine already has.
    fn called(name: String, password: &str) -> Result<Account, Missing> {
        let sid = sid_of(&name).map_err(|why| Missing::Account {
            name: name.clone(),
            why,
        })?;

        // Empty and absent are one answer here, and the same one: a password
        // nobody holds and a password somebody blanked by hand are both an
        // account nothing can start a process as — see
        // [`crate::settings::Secrets`], which reads a blank back as nothing in
        // the first place.
        if password.is_empty() {
            return Err(Missing::Password { name });
        }

        Ok(Account {
            name,
            password: password.to_owned(),
            sid,
        })
    }

    /// What the account is called on this machine, which is what the record
    /// beside the entries carries.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The password a process is started as it with.
    pub fn password(&self) -> &str {
        &self.password
    }

    /// And the SID every access-control entry written for it names.
    pub fn sid(&self) -> &Sid {
        &self.sid
    }
}

/// An account's SID, held as the bytes it is and as the spelling a person
/// reads.
///
/// The bytes because that is what `SetEntriesInAclW` takes, and the text
/// because that is what a record under the Data Directory writes down and what
/// a log line says.
pub struct Sid {
    bytes: Vec<u8>,
    text: String,
}

/// As the spelling and nothing else: the bytes are the same thing said in a
/// way nobody reads, and a log line or a failed assertion wants the one a
/// person can compare against what `whoami /user` printed.
impl std::fmt::Debug for Sid {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_tuple("Sid").field(&self.text).finish()
    }
}

impl Sid {
    /// The SID as Win32 passes one about.
    ///
    /// Borrowed from the bytes this holds, so it is good for exactly as long as
    /// the [`Sid`] is.
    pub fn psid(&self) -> PSID {
        self.bytes.as_ptr() as PSID
    }

    /// And as a person reads one: `S-1-5-21-…`.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// What making the account came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// There was no such account and now there is.
    Created,

    /// The account was already on the machine and `secrets.yaml` already had
    /// its password: there was nothing to do, and running the verb twice is not
    /// a failure.
    AlreadyThere,

    /// The account was already on the machine and nothing knew its password —
    /// a Data Directory restored without its secrets, or a `secrets.yaml`
    /// emptied by hand. A password nobody holds is an account nothing can use,
    /// so a new one is set and written down.
    PasswordSetAgain,
}

/// The account `data_dir`'s sessions will run as, made on this machine and its
/// password written down beside the other secrets.
///
/// **Elevated, or refused in words.** Everything below the first line is an
/// administrator's call, and a refusal that says `Access is denied` is one
/// nobody can act on.
///
/// **And made twice is not a failure.** A machine somebody runs this on again
/// is one that already has the account, which is said rather than refused — see
/// [`Made`].
pub fn create(data_dir: &Path) -> Result<(String, Made)> {
    elevated_enough("create")?;

    let name = named(data_dir);
    let settings = Settings::in_data_dir(data_dir);
    let secret = password();

    let made = match added(&name, &secret, data_dir) {
        0 => {
            written_down(&settings, &secret)?;

            Made::Created
        }
        ALREADY_THERE => match settings.secrets().session_account_password() {
            Some(_) => Made::AlreadyThere,
            None => {
                set_password(&name, &secret)?;
                written_down(&settings, &secret)?;

                Made::PasswordSetAgain
            }
        },
        // Belt and braces: the elevation was checked above, so this is a
        // machine whose policy refuses the call for some other reason — and it
        // is still the same sentence that helps.
        ERROR_ACCESS_DENIED => bail!(
            "this machine refused to create the local account {name} with access denied — \
             `{MAKE_IT}` has to be run from an elevated terminal, and on a machine whose \
             policy allows a local account to be made at all",
        ),
        said => bail!(
            "this machine would not create the local account {name}: {}",
            std::io::Error::from_raw_os_error(said as i32),
        ),
    };

    // Said every time rather than only on the run that created the account:
    // both are idempotent, and a machine where one of them was interrupted
    // half way is one the verb can be run again on to finish.
    let sid =
        sid_of(&name).map_err(|why| anyhow!("the account {name} would not resolve: {why}"))?;

    denied(&sid)
        .with_context(|| format!("denying {name} every way of logging on it may not use"))?;
    kept_off_the_screen(&name).with_context(|| format!("keeping {name} off the sign-in screen"))?;

    Ok((name, made))
}

/// What taking the account away came to: which of the three things that could
/// have been there was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Removed {
    /// Whether there was an account to delete.
    pub account: bool,

    /// Whether it had a profile directory under `C:\Users` — which it will have
    /// as soon as one session has run, and will not before.
    pub profile: bool,

    /// And whether `secrets.yaml` was still holding its password.
    pub password: bool,
}

/// Take the account away, and its profile directory and its password with it.
///
/// **In that order, and the order matters.** A profile is named by a SID, so it
/// is deleted while the account is still there to resolve one; the account
/// after it; and the password last, because a password cleared before a failed
/// deletion would leave an account nothing could use and nothing could name.
///
/// Nothing here is a failure for not having been there. A Data Directory whose
/// account was never made, or was deleted by hand, is one this says it found
/// nothing of — which is what a verb run twice should do.
pub fn remove(data_dir: &Path) -> Result<(String, Removed)> {
    elevated_enough("remove")?;

    let name = named(data_dir);
    let settings = Settings::in_data_dir(data_dir);
    let mut removed = Removed::default();

    if let Ok(sid) = sid_of(&name) {
        // Before the account goes, because the entries name it: the grants that
        // stand for the installation are the ones no boundary ever takes off —
        // see [`super::super::Surface::standing`] — so the account being
        // removed is the one moment there is to take them off, and after
        // `NetUserDel` there is no SID left to say whose they were.
        //
        // Said and not insisted on, the way the rest of this verb is: an entry
        // that would not come off is an entry naming an identity this machine
        // is about to stop having, which grants nobody anything. What it costs
        // is a line on somebody's directory list.
        let standing = super::super::granting::standing(&super::super::standing_grants(data_dir));

        super::super::granting::writing::strip(&standing, &[], sid.text());

        removed.profile = profile_deleted(&sid)?;
    }

    match unsafe { NetUserDel(ptr::null(), wide(OsStr::new(&name)).as_ptr()) } {
        0 => removed.account = true,
        // NERR_UserNotFound: there was nothing to delete, which is not a
        // failure — see this function's own documentation.
        2221 => {}
        ERROR_ACCESS_DENIED => bail!(
            "this machine refused to delete the local account {name} with access denied — \
             `verkstead session-account remove` has to be run from an elevated terminal",
        ),
        said => bail!(
            "this machine would not delete the local account {name}: {}",
            std::io::Error::from_raw_os_error(said as i32),
        ),
    }

    put_back_on_the_screen(&name)
        .with_context(|| format!("taking {name} off the list of accounts not to draw"))?;

    if settings.secrets().session_account_password().is_some() {
        settings
            .save_secrets(&settings.secrets().with_session_account_password(None))
            .context("taking the password out of secrets.yaml")?;

        removed.password = true;
    }

    Ok((name, removed))
}

/// The SID of a local account, or what the machine said when it was asked.
///
/// The one call the server makes here, and the one that says whether the
/// elevated verb has been run on this machine at all.
pub fn sid_of(account: &str) -> Result<Sid, String> {
    let name = wide(OsStr::new(account));

    let mut room = 0u32;
    let mut domain = 0u32;
    let mut kind = 0i32;

    // Asked twice, which is how every Win32 call that hands back a variable
    // amount is asked: once for how much, and once for it. The first is
    // expected to fail, and what it leaves behind is the size.
    unsafe {
        LookupAccountNameW(
            ptr::null(),
            name.as_ptr(),
            ptr::null_mut(),
            &mut room,
            ptr::null_mut(),
            &mut domain,
            &mut kind,
        )
    };

    if room == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    let mut bytes = vec![0u8; room as usize];
    let mut said = vec![0u16; domain as usize];

    let found = unsafe {
        LookupAccountNameW(
            ptr::null(),
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut room,
            said.as_mut_ptr(),
            &mut domain,
            &mut kind,
        )
    };

    if found == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    let text = written(bytes.as_ptr() as PSID).unwrap_or_else(|| String::from("(unreadable)"));

    Ok(Sid { bytes, text })
}

/// Whether this process is running elevated, or a line saying it has to be.
///
/// Asked of the token rather than of the account: a member of Administrators
/// running unelevated holds a filtered token and every call below would refuse
/// it, so what matters is what this process *is* rather than what its owner
/// could become.
fn elevated_enough(verb: &str) -> Result<()> {
    if elevated()? {
        return Ok(());
    }

    bail!(
        "`verkstead session-account {verb}` has to be run from an elevated terminal: it \
         {} on this machine, which is an administrator's call. Start Windows PowerShell or \
         Command Prompt with Run as administrator and run it there.",
        match verb {
            "create" => "creates a local account",
            _ => "deletes a local account and its profile directory",
        },
    )
}

/// Whether this process holds an elevated token.
fn elevated() -> Result<bool> {
    let mut token: HANDLE = ptr::null_mut();

    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(std::io::Error::last_os_error()).context("reading this process's own token");
    }

    let token = Handle(token);
    let mut elevation = TOKEN_ELEVATION::default();
    let mut written = 0u32;

    let read = unsafe {
        GetTokenInformation(
            token.0,
            TokenElevation,
            ptr::from_mut(&mut elevation).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut written,
        )
    };

    if read == 0 {
        return Err(std::io::Error::last_os_error())
            .context("asking this process's token whether it is elevated");
    }

    Ok(elevation.TokenIsElevated != 0)
}

/// `NetUserAdd` for the account, said once so that the flags are in one place.
///
/// `USER_PRIV_USER` is what a local account added this way must be, and is what
/// puts it in `Users` and in nothing else. The two password flags are the whole
/// of *Verkstead's to know rather than anybody's to type*: it does not expire,
/// and the account cannot change it out from under the file that holds it.
fn added(name: &str, secret: &str, data_dir: &Path) -> u32 {
    let mut name = wide(OsStr::new(name));
    let mut secret = wide(OsStr::new(secret));

    // Said on the account itself, so that a human looking at the accounts on
    // their machine is told what this is and which Verkstead's it is without
    // having to look anything up.
    let mut comment = wide(OsStr::new(&whose(data_dir)));

    let mut wanted = USER_INFO_1 {
        usri1_name: name.as_mut_ptr(),
        usri1_password: secret.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: ptr::null_mut(),
        usri1_comment: comment.as_mut_ptr(),
        usri1_flags: UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE,
        usri1_script_path: ptr::null_mut(),
    };

    unsafe {
        NetUserAdd(
            ptr::null(),
            1,
            ptr::from_mut(&mut wanted).cast(),
            ptr::null_mut(),
        )
    }
}

/// What the account says about itself on this machine.
///
/// Kept inside what a comment may be: the field is a fixed size, and a Data
/// Directory somebody nested twenty deep is not worth a refusal.
fn whose(data_dir: &Path) -> String {
    const ROOM: usize = 200;

    let said = format!(
        "Verkstead runs its Windows sessions as this account. Data Directory: {}",
        data_dir.display(),
    );

    match said.char_indices().nth(ROOM) {
        Some((at, _)) => format!("{}…", &said[..at]),
        None => said,
    }
}

/// Give the account a password again, for the one case that wants it: an
/// account on the machine that nothing holds the password of.
fn set_password(name: &str, secret: &str) -> Result<()> {
    let mut secret = wide(OsStr::new(secret));

    let wanted = USER_INFO_1003 {
        usri1003_password: secret.as_mut_ptr(),
    };

    let said = unsafe {
        NetUserSetInfo(
            ptr::null(),
            wide(OsStr::new(name)).as_ptr(),
            1003,
            ptr::from_ref(&wanted).cast(),
            ptr::null_mut(),
        )
    };

    match said {
        0 => Ok(()),
        said => Err(anyhow!(
            "the account {name} is on this machine and nothing holds its password, and this \
             machine would not set a new one: {}",
            std::io::Error::from_raw_os_error(said as i32),
        )),
    }
}

/// Put the password in `secrets.yaml`, beside whatever else is in there.
///
/// On the secrets that are already there rather than on nothing: this file
/// holds the human's GitHub token too, and a verb run at install has no
/// business taking that away — see [`crate::settings::Settings::save_secrets`].
fn written_down(settings: &Settings, secret: &str) -> Result<()> {
    settings
        .save_secrets(
            &settings
                .secrets()
                .with_session_account_password(Some(secret.to_owned())),
        )
        .with_context(|| {
            format!(
                "the account was made and its password could not be written to {} — run \
                 `verkstead session-account remove` and try again",
                settings.secrets_path().display(),
            )
        })
}

/// Deny the account every way of logging on that it will never use.
///
/// See [`DENIED`] for which those are and for the one that is deliberately not
/// among them.
fn denied(sid: &Sid) -> Result<()> {
    let policy = Policy::opened()?;

    // Held for the length of the call: an `LSA_UNICODE_STRING` is a pointer
    // into somebody else's buffer, and these are the buffers.
    let mut buffers: Vec<Vec<u16>> = DENIED.iter().map(|right| wide(OsStr::new(right))).collect();
    let rights: Vec<LSA_UNICODE_STRING> = buffers.iter_mut().map(spelt).collect();

    let said =
        unsafe { LsaAddAccountRights(policy.0, sid.psid(), rights.as_ptr(), rights.len() as u32) };

    ok(said, "denying the account the logon types it may not use")
}

/// The LSA policy of this machine, opened for long enough to write a right onto
/// an account and closed by dropping it.
struct Policy(windows_sys::Win32::Security::Authentication::Identity::LSA_HANDLE);

impl Policy {
    fn opened() -> Result<Policy> {
        let attributes = LSA_OBJECT_ATTRIBUTES::default();
        let mut handle = 0;

        let said = unsafe {
            LsaOpenPolicy(
                ptr::null(),
                &attributes,
                (POLICY_CREATE_ACCOUNT | POLICY_LOOKUP_NAMES) as u32,
                &mut handle,
            )
        };

        ok(said, "opening this machine's security policy")?;

        Ok(Policy(handle))
    }
}

impl Drop for Policy {
    fn drop(&mut self) {
        unsafe { LsaClose(self.0) };
    }
}

/// One of these calls' answers as an ordinary error: they hand back an
/// `NTSTATUS` rather than a Win32 error, and this is the translation the
/// operating system provides for exactly that.
fn ok(said: NTSTATUS, doing: &str) -> Result<()> {
    if said == 0 {
        return Ok(());
    }

    Err(std::io::Error::from_raw_os_error(
        unsafe { LsaNtStatusToWinError(said) } as i32,
    ))
    .context(doing.to_owned())
}

/// A wide string as the LSA calls want one: the units, and how many bytes of
/// them there are without and with the nothing at the end.
fn spelt(buffer: &mut Vec<u16>) -> LSA_UNICODE_STRING {
    let units = buffer.len() - 1;

    LSA_UNICODE_STRING {
        Length: (units * 2) as u16,
        MaximumLength: (buffer.len() * 2) as u16,
        Buffer: buffer.as_mut_ptr(),
    }
}

/// Say that this account is not to be drawn on the sign-in screen.
///
/// See [`OFF_THE_SCREEN`]. Written every time the verb is run rather than only
/// on the run that made the account, because it costs one registry write and
/// because a machine somebody has tidied by hand is one the verb should put
/// back.
fn kept_off_the_screen(name: &str) -> Result<()> {
    let mut key: HKEY = ptr::null_mut();

    let opened = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            wide(OsStr::new(OFF_THE_SCREEN)).as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };

    if opened != 0 {
        return Err(std::io::Error::from_raw_os_error(opened as i32))
            .context("opening the list of accounts the sign-in screen is not to draw");
    }

    let key = Key(key);
    let nought = 0u32;

    let written = unsafe {
        RegSetValueExW(
            key.0,
            wide(OsStr::new(name)).as_ptr(),
            0,
            REG_DWORD,
            ptr::from_ref(&nought).cast(),
            size_of::<u32>() as u32,
        )
    };

    if written != 0 {
        return Err(std::io::Error::from_raw_os_error(written as i32))
            .context("saying the session account is not to be drawn on the sign-in screen");
    }

    Ok(())
}

/// And take it off that list again, which is what removal owes the machine.
///
/// A list that is not there, and a name that is not on it, are both nothing to
/// do rather than a failure: the value is one write of the verb that made the
/// account, and a machine where that write never happened is still a machine
/// this should leave tidy.
fn put_back_on_the_screen(name: &str) -> Result<()> {
    let mut key: HKEY = ptr::null_mut();

    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            wide(OsStr::new(OFF_THE_SCREEN)).as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };

    if opened == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }

    if opened != 0 {
        return Err(std::io::Error::from_raw_os_error(opened as i32))
            .context("opening the list of accounts the sign-in screen is not to draw");
    }

    let key = Key(key);
    let taken = unsafe { RegDeleteValueW(key.0, wide(OsStr::new(name)).as_ptr()) };

    if taken != 0 && taken != ERROR_FILE_NOT_FOUND {
        return Err(std::io::Error::from_raw_os_error(taken as i32))
            .context("taking the session account off the list of accounts not to draw");
    }

    Ok(())
}

/// A registry key, closed by dropping it.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

/// Delete the account's profile directory, and say whether there was one.
///
/// `DeleteProfileW` rather than a `remove_dir_all` of `C:\Users\<account>`,
/// because a profile is two things: the directory, and the machine's record of
/// where that directory is. Removing the first by hand leaves the second, and a
/// later account of the same name would then be given a directory with a suffix
/// on it.
fn profile_deleted(sid: &Sid) -> Result<bool> {
    let gone = unsafe {
        DeleteProfileW(
            wide(OsStr::new(sid.text())).as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    };

    if gone != 0 {
        return Ok(true);
    }

    match unsafe { GetLastError() } {
        // The account has never had a session run as it, so nothing ever
        // loaded a profile for it. Which is the ordinary case for a machine
        // where somebody ran the verb and changed their mind.
        ERROR_FILE_NOT_FOUND => Ok(false),
        said => Err(std::io::Error::from_raw_os_error(said as i32))
            .context("deleting the session account's profile directory"),
    }
}

/// A SID as a person reads one, or nothing where it would not be written down.
fn written(sid: PSID) -> Option<String> {
    let mut put: *mut u16 = ptr::null_mut();

    if unsafe { ConvertSidToStringSidW(sid, &mut put) } == 0 || put.is_null() {
        return None;
    }

    let mut length = 0;

    // Safety: what the call wrote is a string ending in a nothing, and this is
    // how long it is.
    while unsafe { *put.add(length) } != 0 {
        length += 1;
    }

    // Safety: `length` units are what was written, and they are read before the
    // block holding them is given back.
    let text = std::ffi::OsString::from_wide(unsafe { std::slice::from_raw_parts(put, length) })
        .to_string_lossy()
        .into_owned();

    unsafe { LocalFree(put.cast()) };

    Some(text)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Account, MAKE_IT, Missing, Settings, elevated, named, sid_of, whose};

    /// The account of a machine that certainly has one, so that the lookup
    /// itself is exercised without anything having to be created.
    #[test]
    fn a_local_account_that_is_there_resolves_to_a_sid() {
        let sid = sid_of("SYSTEM").expect("every Windows machine has SYSTEM");

        assert_eq!(sid.text(), "S-1-5-18");
    }

    /// And one that is not there is a refusal in words rather than a panic,
    /// which is what a Data Directory whose verb has never been run gets.
    #[test]
    fn an_account_that_is_not_there_says_so() {
        let why = sid_of("vk-000000000000").expect_err("no such account should have been found");

        assert!(!why.is_empty(), "a refusal should say something");
    }

    /// Whichever this machine answers, it answers: the suite runs unelevated
    /// and elevated, and neither is a failure here.
    #[test]
    fn this_process_can_say_whether_it_is_elevated() {
        elevated().expect("a process can always read its own token");
    }

    /// The server's own read, on a Data Directory nobody has run the verb for:
    /// a refusal naming what is missing, which is what will later refuse a
    /// session rather than a Win32 number nobody can act on.
    #[test]
    fn a_data_directory_with_no_account_is_refused_in_words() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Settings::in_data_dir(dir.path()).secrets();

        let missing = Account::on_this_machine(dir.path(), &secrets)
            .expect_err("nothing has made an account for a directory made a moment ago");

        assert!(
            matches!(missing, Missing::Account { .. }),
            "got {missing:?}",
        );
        assert!(missing.to_string().contains(&named(dir.path())));
        assert!(missing.to_string().contains(MAKE_IT));
    }

    /// And the other half of the same read: an account that is on this machine
    /// with nothing holding its password. Asked of an account every Windows
    /// machine has, because making one takes an elevation the suite has not
    /// got — what is under test is which refusal a resolved name with no
    /// password gets, rather than which account it is about.
    #[test]
    fn an_account_with_no_password_is_the_other_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Settings::in_data_dir(dir.path()).secrets();

        let missing = Account::called(
            String::from("SYSTEM"),
            secrets.session_account_password().unwrap_or_default(),
        )
        .expect_err("a directory made a moment ago holds no password");

        assert!(
            matches!(missing, Missing::Password { .. }),
            "got {missing:?}",
        );
        assert!(missing.to_string().contains("secrets.yaml"));
        assert!(missing.to_string().contains(MAKE_IT));
    }

    /// And a name that resolves *and* has a password is the account itself,
    /// carrying all three halves of one.
    #[test]
    fn a_name_that_resolves_and_a_password_are_an_account() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_session_account_password(Some("Vk1-hunter2".to_owned())),
            )
            .unwrap();

        let account = Account::called(String::from("SYSTEM"), "Vk1-hunter2")
            .expect("SYSTEM resolves and a password was said");

        assert_eq!(account.name(), "SYSTEM");
        assert_eq!(account.password(), "Vk1-hunter2");
        assert_eq!(account.sid().text(), "S-1-5-18");

        // And the same read of the two halves a description carries, which is
        // what a session start asks — see [`Account::resolving`]. The password
        // written above is the one a sandbox would have read out of the same
        // file, so this is that whole route in one line.
        let resolved = Account::resolving(&super::super::Logon::of(
            "SYSTEM",
            settings
                .secrets()
                .session_account_password()
                .expect("the password was just written"),
        ))
        .expect("the same name and the same password");

        assert_eq!(resolved.sid().text(), account.sid().text());
    }

    /// And a `Logon` with nothing in its password half is the password refusal
    /// rather than a logon attempted with an empty secret.
    ///
    /// Which is what a sandbox built on a machine whose `secrets.yaml` holds
    /// nothing carries — see `sandbox::Sandbox::session_account`, where the
    /// empty half is deliberate: a description is portable, so it says the
    /// account it *would* run as and this is where the machine says there is
    /// none to run as.
    #[test]
    fn a_logon_with_no_password_is_the_password_refusal() {
        let missing = Account::resolving(&super::super::Logon::of("SYSTEM", ""))
            .expect_err("an empty password is nobody's password");

        assert!(
            matches!(missing, Missing::Password { .. }),
            "got {missing:?}",
        );
    }

    #[test]
    fn the_comment_says_which_verkstead_this_is() {
        let said = whose(Path::new(r"C:\Users\somebody\AppData\Roaming\Verkstead"));

        assert!(said.contains("Verkstead"));
        assert!(said.contains(r"AppData\Roaming\Verkstead"));
    }

    #[test]
    fn a_comment_is_never_longer_than_the_field_it_goes_in() {
        let deep = Path::new(r"C:\").join("a".repeat(4000));
        let said = whose(&deep);

        assert!(
            said.chars().count() <= 201,
            "{} characters",
            said.chars().count()
        );
    }
}
