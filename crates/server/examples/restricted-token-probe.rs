//! What a Windows machine allows a *restricted token*, asked by attempting.
//!
//! The sibling of [`appcontainer-probe`](appcontainer-probe.rs), and the reason
//! there is a second one. That probe settled what an AppContainer allows, and
//! its second run — recorded in [ADR-0014](../../../docs/adr/0014-windows-sessions.md),
//! *What the probe answered the second time* — settled that an AppContainer
//! cannot host the agent it exists to sandbox: a path cannot be *resolved*
//! inside one however well it is granted, and msys2 will not start in one at
//! all. So the boundary is being rendered again, on the mechanism ADR-0014
//! considered and set aside, and this asks that mechanism the same questions by
//! trying them.
//!
//! **What a restricted token is, for the purposes of this program.** A process
//! started with one is the human's own account with a second access check
//! stapled on: every open is checked once against the token's own groups, as
//! ever, and again against a *restricted SID list*, and both have to say yes.
//! So what a session reaches is the intersection — everything the human could
//! reach, narrowed to what one of a handful of named SIDs is granted. The
//! machine is there and refused, which is the same shape the Mac's boundary has
//! and the shape ADR-0014 wanted; what makes it different from an AppContainer
//! is that the *identity the process runs as* is unchanged, so nothing that
//! resolves a path or makes a named object is talking to a stranger.
//!
//! **There are two families of it, and this asks about five shapes across
//! them.** A token can be narrowed in *who it is* — a restricted SID list, of
//! the identity alone, or beside the SIDs the machine grants its own
//! directories to, or beside this logon's as well — and it can be narrowed in
//! *how high it stands*, which is the integrity level, where a read is refused
//! nothing and a write is refused anything standing higher. The first family is
//! the stronger promise: the human's files are not even readable. The second is
//! the weaker one and buys an ordinary token for it, which is what a toolchain
//! written for an ordinary account turns out to need.
//!
//! **What a grant is differs with the family**, and both are an entry Verkstead
//! writes on a real path of the human's. Under a list it is an access-control
//! entry naming the identity, and it makes a directory *reachable*. Under a
//! lowered token it is a mandatory label taking the directory down to meet the
//! session, and it makes one *writable* — everything stays readable either way.
//!
//! **And it opens up the three things that are not files**, because a
//! restricted process is refused more than a directory: its own token, the
//! window station it is on and the desktop under that are all securable and all
//! granted to the human's account rather than to anything a restricted list
//! holds. Every sandbox on this mechanism writes an entry on those three, so
//! this does too — and shuts them again, which matters more here than the
//! playground does: they are the human's own logon's.
//!
//! **It asserts nothing**, exactly as its sibling asserts nothing: every
//! question is asked by trying it, a `no` is as much of an answer as a `yes`,
//! and it exits 0 wherever it got as far as printing.
//!
//! **It is two programs.** Run with no arguments it is the outside half: it
//! derives an identity, lays out a playground with the entries a rendering would
//! write, and starts the inside half under each token. Run with `--inside` it
//! is that inside half, which tries each thing and prints one line each.
//!
//! **What it leaves behind is nothing**: every entry it wrote is revoked and
//! the playground goes. There is no identity to delete — see `Identity`.
//!
//! Run it as:
//!
//! ```text
//! cargo run -p verkstead-server --example restricted-token-probe
//! ```

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "this probe asks a Windows machine about restricted tokens, and this is not one — \
         run it on the Windows machine whose answer is wanted"
    );
}

#[cfg(windows)]
fn main() {
    let argv: Vec<String> = std::env::args().collect();

    match argv.get(1).map(String::as_str) {
        Some(probe::INSIDE) => probe::inside(&argv[2..]),
        _ => probe::outside(),
    }
}

#[cfg(windows)]
mod probe {
    use std::ffi::{OsStr, OsString};
    use std::io::Write;
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
        LocalFree, SetHandleInformation, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
        EXPLICIT_ACCESS_W, GetNamedSecurityInfoW, NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS,
        SDDL_REVISION_1, SE_FILE_OBJECT, SE_KERNEL_OBJECT, SE_WINDOW_OBJECT, SET_ACCESS,
        SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        ACL, CreateRestrictedToken, DACL_SECURITY_INFORMATION, DISABLE_MAX_PRIVILEGE,
        NO_INHERITANCE, PSECURITY_DESCRIPTOR, PSID, SID_AND_ATTRIBUTES,
        SUB_CONTAINERS_AND_OBJECTS_INHERIT, TOKEN_ADJUST_DEFAULT, TOKEN_ASSIGN_PRIMARY,
        TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    };
    use windows_sys::Win32::System::Console::{COORD, ClosePseudoConsole, CreatePseudoConsole};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW,
        DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
        GetExitCodeProcess, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
        OpenProcessToken, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION, ResumeThread,
        STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess, WaitForSingleObject,
    };

    /// The word that says this run is the inside half.
    pub const INSIDE: &str = "--inside";

    /// The SIDs the machine grants its own directories to, which a token that
    /// is to start a process at all has to keep one of.
    ///
    /// `Everyone` and `Users` are what `C:\`, the system directory and Program
    /// Files are readable by; `RESTRICTED` is the one the operating system
    /// writes precisely for this mechanism, on the files a restricted process
    /// is expected to be able to load. None of the three is on a human's own
    /// profile, which is granted to their account and to SYSTEM and the
    /// administrators — which is the whole reason this shape of list is worth
    /// asking about.
    const WORLD: [&str; 3] = ["S-1-1-0", "S-1-5-32-545", "S-1-5-12"];

    /// And the ones this logon's own objects are granted to, which the list
    /// above turns out not to be enough without.
    ///
    /// `Authenticated Users` and `INTERACTIVE` beside the three above; the
    /// logon SID itself is not here because it cannot be written down — it is
    /// read off this process's own token, one number per sign-in.
    const SESSION: [&str; 5] = ["S-1-1-0", "S-1-5-32-545", "S-1-5-12", "S-1-5-11", "S-1-5-4"];

    /// What a handle has to be opened with to read and rewrite the list on the
    /// object behind it.
    const READ_CONTROL: u32 = 0x0002_0000;
    const WRITE_DAC: u32 = 0x0004_0000;

    /// The two levels a token is lowered to below, as the SIDs an integrity
    /// level is one of.
    const LOW: &str = "S-1-16-4096";
    const MEDIUM_LOW: &str = "S-1-16-6144";

    /// And the label that says a directory stands as low as a lowered session
    /// does, which is what lets one write there.
    ///
    /// Written as SDDL because that is the one spelling of a mandatory label a
    /// person can read: a label ACE (`ML`), inherited by the directories and
    /// files under it (`OICI`), refusing a write from anything lower (`NW`), at
    /// the low level (`LW`).
    const LABELLED_LOW: &str = "S:(ML;OICI;NW;;;LW)";

    /// What a group is when it is the integrity level rather than a group.
    const SE_GROUP_INTEGRITY: u32 = 0x0000_0020;

    /// How long anything here waits for a process it started, or for a
    /// connection to answer.
    const PATIENCE: Duration = Duration::from_secs(10);

    /// The outside half: derive an identity, lay out a playground, and ask each
    /// shape of token what it can do.
    pub fn outside() {
        say("restricted-token-probe 1 — paste everything from this line down");
        say(&format!("machine        = {}", machine()));

        let Ok(exe) = std::env::current_exe() else {
            say("identity       = not attempted: this program cannot say where it is");
            return;
        };

        let name = format!("verkstead-restricted-probe-{}", std::process::id());

        // The identity a session's grants are written for and its token is
        // restricted to — see [`Identity`], which is where the whole of why it
        // is derived rather than registered is.
        let identity = match Identity::derived(&name) {
            Ok(identity) => identity,
            Err(error) => {
                say(&format!("identity       = would not be made: {error}"));
                return;
            }
        };

        say(&format!("identity       = {} as {name}", identity.text));

        let playground = std::env::temp_dir().join(&name);
        let mut written = Written::new(identity.psid());

        if let Err(error) = lay_out(&playground) {
            say(&format!("playground     = would not be laid out: {error}"));
            return;
        }

        say(&format!("playground     = {}", playground.display()));

        written.grant(
            &playground.join("granted-rw"),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
        );
        written.grant(
            &playground.join("granted-ro"),
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        );
        written.grant(
            &playground.join("target"),
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        );
        // The one a lowered session may write, which is granted *as well* so
        // that the difference between the two families is the label and
        // nothing else: a shape with a list reaches it because it is granted,
        // and a lowered shape writes it because it stands as low.
        written.grant(
            &playground.join("labelled-rw"),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
        );

        // The playground itself walked through but not opened into, so that the
        // ungranted directory under it really is ungranted.
        written.grant_here(&playground, FILE_GENERIC_EXECUTE);

        say(&format!(
            "label          = {}",
            match labelled_low(&playground.join("labelled-rw")) {
                Ok(()) => String::from("written on the one directory a lowered session may write"),
                Err(error) => format!("would not be written: {error}"),
            }
        ));

        say(&format!("entries        = {}", written.refusals()));

        // And the same three paths granted again to this logon's own SID, which
        // is what a shape that denies the human's account has left to reach
        // them by: an entry naming that account grants a token where it is
        // deny-only exactly nothing.
        //
        // Which costs the human nothing they had not already got — every
        // process of their own sign-in holds this SID, and these are their own
        // directories — and it is what a rendering on that shape would have to
        // write, so it is what is asked about.
        let logon = own_token().and_then(|own| logon_of(&own));
        let mut by_logon = None;

        if let Ok(Some(logon)) = &logon {
            let mut theirs = Written::new(logon.sid);

            for (under, rights) in [
                (
                    "granted-rw",
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
                ),
                ("granted-ro", FILE_GENERIC_READ | FILE_GENERIC_EXECUTE),
                ("target", FILE_GENERIC_READ | FILE_GENERIC_EXECUTE),
                (
                    "labelled-rw",
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
                ),
            ] {
                theirs.grant(&playground.join(under), rights);
            }

            theirs.grant_here(&playground, FILE_GENERIC_EXECUTE);

            say(&format!("logon-entries  = {}", theirs.refusals()));

            by_logon = Some(theirs);
        } else {
            say(
                "logon-entries  = this logon's own SID would not be read, so the \
                 deny-only shape below has nothing granted to it",
            );
        }

        // Something to dial, so that "refused" and "there was nothing there"
        // are not the same answer.
        let listener = TcpListener::bind("127.0.0.1:0").ok();
        let loopback = listener
            .as_ref()
            .and_then(|listener| listener.local_addr().ok())
            .map_or_else(String::new, |address| address.to_string());

        say(&format!("dialling       = {loopback}"));

        let stations = Stations::opened_up(&identity);
        say(&format!("stations       = {}", stations.said));

        // The environment a session is really handed — cleared, with a fresh
        // profile of its own where the human's would be, exactly as
        // `sandbox::windows_names` says one.
        //
        // **Which is not a detail.** A tool that keeps its configuration under
        // `LOCALAPPDATA` and is told the human's is a tool sent somewhere this
        // boundary refuses, and it reports that as a fault of its own rather
        // than as the boundary — so asking these questions with the probe's own
        // environment would be asking them wrongly.
        let handed = match session_environment(&playground) {
            Ok(handed) => handed,
            Err(error) => {
                say(&format!("environment    = would not be laid out: {error}"));
                return;
            }
        };

        // The three shapes, in the order the rendering would prefer them: the
        // tightest first, so that a machine which allows it says so before the
        // wider one is read.
        for shape in [
            Shape::Alone,
            Shape::WithTheWorld,
            Shape::WithTheLogon,
            Shape::Lowered,
            Shape::JustBelow,
            Shape::DenyingTheUser,
        ] {
            say(&format!("--- {} ---", shape.said()));

            let token = match Token::restricted(&identity, shape) {
                Ok(token) => token,
                Err(error) => {
                    say(&format!("token          = would not be made: {error}"));
                    continue;
                }
            };

            say("token          = made");

            let asked = [
                OsString::from(INSIDE),
                playground.clone().into_os_string(),
                OsString::from(&loopback),
                home().unwrap_or_default().into_os_string(),
            ];

            // **A shape whose own inside half cannot start is a shape nothing else
            // can be asked of**, so the questions below it are skipped rather than
            // asked and answered wrongly. A process that cannot load its own image
            // says so by printing nothing and ending with the code the loader gave
            // it, which is what this reads.
            let ran = match started(&token, exe.as_os_str(), &asked, None, true, None) {
                Ok(outcome) if outcome.output.trim().is_empty() => {
                    say(&format!(
                        "inside         = printed nothing and {}, so nothing below \
                         it was asked",
                        ended(outcome.exit)
                    ));

                    false
                }
                Ok(outcome) => {
                    for line in outcome.output.lines() {
                        say(line);
                    }

                    true
                }
                Err(error) => {
                    say(&format!(
                        "inside         = would not start: {error}, so nothing below \
                         it was asked"
                    ));

                    false
                }
            };

            if !ran {
                continue;
            }

            say(&format!("conpty         = {}", conpty(&token)));

            // The tools, each as the machine keeps it. Nothing is granted for
            // them: what is being asked is whether the token's second check
            // lets a program on this machine's own disk run at all.
            //
            // `bash` is the one this whole rendering turns on — it is what
            // Claude's shell tool runs, and it is what an AppContainer would
            // not start.
            for tool in ["node", "pwsh", "powershell", "git", "bash", "reg"] {
                say(&format!(
                    "tool {tool:10} = {}",
                    ran_tool(&token, tool, &handed)
                ));
            }

            say(&format!(
                "resolving      = {}",
                resolving(&token, &playground, &handed)
            ));
            say(&format!("sccache        = {}", sccache(&token, &handed)));
        }

        let shut = stations.shut(&identity);
        let revoked =
            written.revoke_all() + by_logon.as_mut().map_or(0, |theirs| theirs.revoke_all());
        let removed = std::fs::remove_dir_all(&playground);

        say(&format!(
            "cleanup        = {revoked} entries revoked, {shut} of this logon's own \
             shut again, playground {}",
            if removed.is_ok() { "removed" } else { "left" },
        ));

        say("restricted-token-probe end — paste up to this line");
    }

    /// The inside half: one line per thing tried.
    pub fn inside(argv: &[String]) {
        let playground = PathBuf::from(argv.first().cloned().unwrap_or_default());
        let loopback = argv.get(1).cloned().unwrap_or_default();
        let home = PathBuf::from(argv.get(2).cloned().unwrap_or_default());

        say(&format!(
            "grant-write    = {}",
            match std::fs::write(playground.join("granted-rw").join("written"), b"inside\n") {
                Ok(()) => String::from("wrote"),
                Err(error) => format!("refused: {error}"),
            }
        ));

        // The same write again, into the directory that has been labelled down
        // as well as granted: for a shape with a list this says nothing the
        // line above did not, and for a lowered one it is the whole boundary —
        // the one place a session may write.
        say(&format!(
            "label-write    = {}",
            match std::fs::write(playground.join("labelled-rw").join("written"), b"inside\n") {
                Ok(()) => String::from("wrote"),
                Err(error) => format!("refused: {error}"),
            }
        ));

        say(&format!(
            "grant-read     = {}",
            reading(&playground.join("granted-ro").join("readable"))
        ));

        say(&format!(
            "grant-none     = {}",
            reading(&playground.join("ungranted").join("readable"))
        ));

        // The junction, which is how a Windows session finds its account: the
        // grant is on what it points at and there is none on the name.
        say(&format!(
            "junction       = {}",
            reading(&playground.join("link").join("readable"))
        ));

        // The machine, which this boundary leaves in plain sight: a file every
        // account on it can read.
        say(&format!(
            "system-read    = {}",
            reading(&PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts"))
        ));

        // And the human's own account, which it is the whole point of this
        // boundary to refuse: their profile listed, which their own SID is
        // granted and nothing wider is.
        say(&format!(
            "human-profile  = {}",
            match std::fs::read_dir(&home) {
                Ok(mut entries) => match entries.next() {
                    Some(Err(error)) => format!("refused: {error}"),
                    _ => String::from("listed"),
                },
                Err(error) => format!("refused: {error}"),
            }
        ));

        // And their profile *written*, which is the question the second family
        // of shapes exists to answer: a lowered session may read the human's
        // files and the promise is only that it cannot change them. Removed
        // again where it lands, so that a probe which found the boundary open
        // has not left anything in it.
        let touched = home.join(".verkstead-probe-write");

        say(&format!(
            "human-write    = {}",
            match std::fs::write(&touched, b"probe\n") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&touched);

                    String::from("wrote — and removed it again")
                }
                Err(error) => format!("refused: {error}"),
            }
        ));

        say(&format!("loopback       = {}", dialled(&loopback)));

        let _ = std::io::stdout().flush();
    }

    /// What is done to a token to make it a session's, which is two families
    /// of thing rather than one.
    ///
    /// **The first three narrow *who* the process is** — a restricted SID list,
    /// and reach is the intersection of the human's own and what that list is
    /// granted. **The last two narrow *how high* it is** — the integrity level,
    /// where nothing is refused a read and a write is refused to anything
    /// standing higher than the process does. They are asked in one program
    /// because they are one call away from each other and because the first
    /// family, as the lines above record, gives a boundary the toolchain will
    /// not run inside.
    ///
    /// The second family is the weaker promise of the two and is deliberately
    /// so: the human's own files stay *readable* and only their being *written*
    /// is refused. What it buys is an ordinary token — the process is the human
    /// with a ceiling on it rather than a stranger — which is exactly what
    /// node, msys2 and a managed runtime turn out to need.
    #[derive(Clone, Copy)]
    enum Shape {
        /// The session's identity and nothing else.
        Alone,

        /// And the identity beside the SIDs the machine's own directories are
        /// granted to.
        WithTheWorld,

        /// And beside those, the SIDs this logon's own objects are granted to.
        ///
        /// **The check is over every securable thing rather than over files**,
        /// which is what the shape above turns out to miss: a window station, a
        /// desktop, the object directory a shell makes its pipes in, the
        /// registry a runtime reads itself out of and the device a random
        /// number comes from are all securable, and none of them is granted to
        /// `Everyone`. They are granted to this logon — which is what a logon
        /// SID is, one per sign-in and on every object the sign-in owns — and
        /// to the interactive and authenticated groups beside it. None of the
        /// three is on a human's profile directory, so this is wider without
        /// being wider *there*, which is the only place it would matter.
        WithTheLogon,

        /// No list at all, and the token lowered to **low** integrity instead.
        ///
        /// The level a browser's renderer runs at, and the one thing on this
        /// platform that a per-user process can impose on itself. A write is
        /// refused to anything standing higher, which is everything on an
        /// ordinary machine — a directory nobody has labelled is medium — so
        /// what a session may write is exactly what has been labelled down to
        /// meet it, and that is a label Verkstead writes on the Worktree and
        /// the fresh profile the way it would have written a grant.
        Lowered,

        /// And lowered only as far as **medium low**, which is the same
        /// arithmetic one notch up.
        ///
        /// Worth asking separately because low is not only a number: the
        /// platform special-cases it — a redirected `LocalLow`, a refusal to
        /// send window messages upwards, a handful of interfaces that check for
        /// it by name. Medium low is below medium and below nothing else, so a
        /// write to the human's own files is refused just the same while none
        /// of that special-casing applies.
        JustBelow,

        /// A third thing again: the human's **own SID turned deny-only**, and
        /// nothing else touched.
        ///
        /// **The token stays medium and stays unrestricted**, which is what the
        /// two families above each give up something for — the first makes the
        /// process a stranger and the second stands it below the machine, and
        /// msys2 needs it to be neither. A deny-only SID is a third dial: the
        /// SID stays in the token and stops granting anything, so an entry that
        /// names the human's account no longer opens a door while an entry
        /// naming a group they are in still does.
        ///
        /// Which is exactly the shape of a Windows profile. A human's own
        /// directories are granted to their account, to SYSTEM and to the
        /// administrators and to nothing wider, so turning that one SID
        /// deny-only refuses the profile **to reads as well as writes** — a
        /// stronger promise than the lowered shapes make — while the system
        /// directories, which are granted to `Users` and `Everyone`, stay
        /// readable and the object namespace a shell needs stays open.
        ///
        /// What a grant is here is an entry naming a group the token still
        /// holds; this probe uses the logon's own SID for it.
        DenyingTheUser,
    }

    impl Shape {
        fn said(self) -> &'static str {
            match self {
                Shape::Alone => "the identity alone",
                Shape::WithTheWorld => "the identity beside Everyone, Users and RESTRICTED",
                Shape::WithTheLogon => "and beside this logon's own SIDs as well",
                Shape::Lowered => "no list at all, lowered to low integrity",
                Shape::JustBelow => "no list at all, lowered to medium low",
                Shape::DenyingTheUser => "no list, medium, the human's own SID deny-only",
            }
        }

        /// The SIDs beyond the identity itself.
        fn beside(self) -> &'static [&'static str] {
            match self {
                Shape::Alone => &[],
                Shape::WithTheWorld => &WORLD,
                Shape::WithTheLogon => &SESSION,
                Shape::Lowered | Shape::JustBelow | Shape::DenyingTheUser => &[],
            }
        }

        /// Whether the identity goes in the list at all, which the shapes that
        /// have no list do not want.
        fn wants_a_list(self) -> bool {
            !matches!(
                self,
                Shape::Lowered | Shape::JustBelow | Shape::DenyingTheUser
            )
        }

        /// Whether the human's own SID is turned deny-only.
        fn denies_the_user(self) -> bool {
            matches!(self, Shape::DenyingTheUser)
        }

        /// Whether this logon's own SID is in the list, which is the one that
        /// cannot be written down: it is a different number every sign-in.
        fn wants_the_logon(self) -> bool {
            matches!(self, Shape::WithTheLogon)
        }

        /// The level the token is lowered to, where it is lowered at all.
        fn lowered_to(self) -> Option<&'static str> {
            match self {
                Shape::Lowered => Some(LOW),
                Shape::JustBelow => Some(MEDIUM_LOW),
                _ => None,
            }
        }
    }

    /// The SID a session's grants are written for, and the SID its token is
    /// restricted to.
    ///
    /// **Derived rather than registered, and that is a finding rather than a
    /// convenience.** The obvious identity was the one the AppContainer
    /// rendering already mints — `CreateAppContainerProfile` hands back a SID
    /// that is this machine's and needs no elevation — and this machine will
    /// not have it: `CreateRestrictedToken` refuses a SID under the package
    /// authority, and refuses a capability SID beside it, with *The parameter
    /// is incorrect*. What it takes is an ordinary one, so what is written here
    /// is a name rather than a registration: a SID under the **service**
    /// authority, whose sub-authorities are a hash of the name Verkstead would
    /// have called the profile.
    ///
    /// Which is exactly what a SID is for. An entry names a principal, and
    /// nothing requires that principal to resolve — `icacls` prints an
    /// unresolvable SID as itself. Service SIDs are the right namespace for one:
    /// Windows derives them the same way, from a name rather than from an
    /// account, and no human is ever issued one, so a derived SID collides with
    /// a person on no machine anywhere.
    ///
    /// **And nothing has to be deleted.** A profile is a registration and had to
    /// be swept after a crash; a derived SID is arithmetic, so what a server
    /// that died leaves behind is entries alone — which the record under the
    /// Data Directory already describes.
    struct Identity {
        sid: Local,
        text: String,
    }

    impl Identity {
        fn derived(name: &str) -> Result<Identity, String> {
            // Five sub-authorities under the service authority, which is the
            // shape Windows' own service SIDs have. The hash is this probe's
            // own and only has to be a hash: what the rendering derives one
            // with is the fingerprint it already names a pipe and a profile by.
            let mut parts = [0u32; 5];
            let mut seed = 0xcbf2_9ce4_8422_2325u64;

            for (place, part) in parts.iter_mut().enumerate() {
                for byte in name.bytes().chain(std::iter::once(place as u8)) {
                    seed ^= u64::from(byte);
                    seed = seed.wrapping_mul(0x100_0000_01b3);
                }

                *part = (seed >> 16) as u32;
            }

            let text = format!(
                "S-1-5-80-{}-{}-{}-{}-{}",
                parts[0], parts[1], parts[2], parts[3], parts[4]
            );

            let mut sid: PSID = ptr::null_mut();
            let read =
                unsafe { ConvertStringSidToSidW(wide(OsStr::new(&text)).as_ptr(), &mut sid) };

            if read == 0 {
                return Err(format!(
                    "the derived SID {text} would not resolve: {}",
                    std::io::Error::last_os_error()
                ));
            }

            Ok(Identity {
                sid: Local(sid),
                text,
            })
        }

        fn psid(&self) -> PSID {
            self.sid.0
        }
    }

    /// One securable thing of this logon's opened up to `sid`, and how to shut
    /// it again.
    ///
    /// **A restricted process is refused more than files.** Its own token, the
    /// window station it is on and the desktop under that are all securable and
    /// all granted to the human's own account rather than to anything in a
    /// restricted list — so a program that asks who it is, or that draws
    /// anything, or that loads a runtime which does either, is refused before
    /// it reaches a file at all. Which is why every sandbox built on this
    /// mechanism writes an entry on those three, and why this asks whether
    /// doing so is what the toolchain was missing.
    fn granted_on(handle: HANDLE, kind: i32, sid: PSID, rights: u32) -> Result<(), String> {
        use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SetSecurityInfo};

        let mut existing: *mut ACL = ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

        let read = unsafe {
            GetSecurityInfo(
                handle,
                kind,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut existing,
                ptr::null_mut(),
                &mut descriptor,
            )
        };

        if read != 0 {
            return Err(format!("reading the list failed with {read}"));
        }

        let access = EXPLICIT_ACCESS_W {
            grfAccessPermissions: rights,
            grfAccessMode: SET_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: sid.cast(),
            },
        };

        let mut wanted: *mut ACL = ptr::null_mut();
        let made = unsafe { SetEntriesInAclW(1, &access, existing, &mut wanted) };

        if made != 0 {
            unsafe { LocalFree(descriptor) };

            return Err(format!("building the list failed with {made}"));
        }

        let set = unsafe {
            SetSecurityInfo(
                handle,
                kind,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                wanted,
                ptr::null_mut(),
            )
        };

        unsafe {
            LocalFree(wanted.cast());
            LocalFree(descriptor);
        }

        if set != 0 {
            return Err(format!("writing the list failed with {set}"));
        }

        Ok(())
    }

    /// `path` labelled down to the level a lowered session stands at, so that
    /// one may write there.
    ///
    /// **This is what a grant is in the other family.** A restricted list makes
    /// a directory reachable by naming an identity on it; a lowered token makes
    /// one writable by lowering the directory to meet it. Both are an entry
    /// Verkstead writes on a real path of the human's and takes back afterwards
    /// — what differs is which list it goes in, and that the label leaves the
    /// path readable by everything as it was.
    fn labelled_low(path: &Path) -> Result<(), String> {
        use windows_sys::Win32::Security::{GetSecurityDescriptorSacl, LABEL_SECURITY_INFORMATION};

        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

        let read = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide(OsStr::new(LABELLED_LOW)).as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        };

        if read == 0 {
            return Err(format!(
                "the label would not be read: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut label: *mut ACL = ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;

        let found = unsafe {
            GetSecurityDescriptorSacl(descriptor, &mut present, &mut label, &mut defaulted)
        };

        if found == 0 || present == 0 {
            unsafe { LocalFree(descriptor) };

            return Err(String::from("the label came back with nothing in it"));
        }

        let name = wide(path.as_os_str());

        let set = unsafe {
            SetNamedSecurityInfoW(
                name.as_ptr().cast_mut(),
                SE_FILE_OBJECT,
                // The label alone, and deliberately not the audit list beside
                // it: they live in the same place, and asking for that one
                // needs a privilege no per-user process holds — which is what
                // the first run of this asked for and was refused with 1314.
                LABEL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                label,
            )
        };

        unsafe { LocalFree(descriptor) };

        if set != 0 {
            return Err(format!("writing the label failed with {set}"));
        }

        Ok(())
    }

    /// The window station and the desktop this process is on, opened up to
    /// `identity` for as long as this is held.
    struct Stations {
        station: HANDLE,
        desktop: HANDLE,
        said: String,
    }

    impl Stations {
        fn opened_up(identity: &Identity) -> Stations {
            use windows_sys::Win32::System::StationsAndDesktops::{
                GetProcessWindowStation, GetThreadDesktop,
            };
            use windows_sys::Win32::System::Threading::GetCurrentThreadId;

            const GENERIC_ALL: u32 = 0x1000_0000;

            let station: HANDLE = unsafe { GetProcessWindowStation() }.cast();
            let desktop: HANDLE = unsafe { GetThreadDesktop(GetCurrentThreadId()) }.cast();

            let mut trouble = Vec::new();

            // Both are window objects as far as the security API is concerned;
            // there is no separate code for a desktop.
            for (handle, what) in [(station, "the window station"), (desktop, "the desktop")] {
                if handle.is_null() {
                    trouble.push(format!("{what} would not open"));
                    continue;
                }

                if let Err(error) =
                    granted_on(handle, SE_WINDOW_OBJECT, identity.psid(), GENERIC_ALL)
                {
                    trouble.push(format!("{what}: {error}"));
                }
            }

            let said = if trouble.is_empty() {
                String::from("the window station and the desktop were opened up")
            } else {
                format!("could not be opened up: {}", trouble.join("; "))
            };

            Stations {
                station,
                desktop,
                said,
            }
        }

        /// And shut again, which matters more here than anywhere: these are the
        /// human's own logon's, and a probe that left an entry on their desktop
        /// would have changed the machine it was only asking about.
        fn shut(&self, identity: &Identity) -> usize {
            use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SetSecurityInfo};

            let mut cleared = 0;

            for handle in [self.station, self.desktop] {
                let kind = SE_WINDOW_OBJECT;
                if handle.is_null() {
                    continue;
                }

                let mut existing: *mut ACL = ptr::null_mut();
                let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

                let read = unsafe {
                    GetSecurityInfo(
                        handle,
                        kind,
                        DACL_SECURITY_INFORMATION,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        &mut existing,
                        ptr::null_mut(),
                        &mut descriptor,
                    )
                };

                if read != 0 {
                    continue;
                }

                let access = EXPLICIT_ACCESS_W {
                    grfAccessPermissions: 0,
                    grfAccessMode: REVOKE_ACCESS,
                    grfInheritance: NO_INHERITANCE,
                    Trustee: TRUSTEE_W {
                        pMultipleTrustee: ptr::null_mut(),
                        MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                        TrusteeForm: TRUSTEE_IS_SID,
                        TrusteeType: TRUSTEE_IS_UNKNOWN,
                        ptstrName: identity.psid().cast(),
                    },
                };

                let mut wanted: *mut ACL = ptr::null_mut();

                if unsafe { SetEntriesInAclW(1, &access, existing, &mut wanted) } == 0 {
                    let set = unsafe {
                        SetSecurityInfo(
                            handle,
                            kind,
                            DACL_SECURITY_INFORMATION,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            wanted,
                            ptr::null_mut(),
                        )
                    };

                    if set == 0 {
                        cleared += 1;
                    }

                    unsafe { LocalFree(wanted.cast()) };
                }

                unsafe { LocalFree(descriptor) };
            }

            cleared
        }
    }

    /// This logon's own SID, read off `token`'s groups — the one group that
    /// says of itself that it is the logon.
    ///
    /// The buffer is kept beside the SID because the SID points into it: what
    /// `GetTokenInformation` hands back is one block with the groups laid out
    /// in it, and a pointer out of a freed block is a pointer to nothing.
    struct Logon {
        sid: PSID,

        #[allow(dead_code)]
        block: Vec<u8>,
    }

    fn logon_of(token: &Held) -> Result<Option<Logon>, String> {
        use windows_sys::Win32::Security::GetTokenInformation;
        use windows_sys::Win32::Security::{TOKEN_GROUPS, TokenGroups};

        /// What a group says about itself when it is the logon's own.
        const SE_GROUP_LOGON_ID: u32 = 0xC000_0000;

        let mut wanted = 0u32;

        unsafe { GetTokenInformation(token.0, TokenGroups, ptr::null_mut(), 0, &mut wanted) };

        if wanted == 0 {
            return Err(format!(
                "the token's groups would not size themselves: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut block = vec![0u8; wanted as usize];

        let read = unsafe {
            GetTokenInformation(
                token.0,
                TokenGroups,
                block.as_mut_ptr().cast(),
                wanted,
                &mut wanted,
            )
        };

        if read == 0 {
            return Err(format!(
                "the token's groups would not be read: {}",
                std::io::Error::last_os_error()
            ));
        }

        let groups = block.as_ptr().cast::<TOKEN_GROUPS>();
        let count = unsafe { (*groups).GroupCount } as usize;
        let first = unsafe { (&raw const (*groups).Groups).cast::<SID_AND_ATTRIBUTES>() };

        for place in 0..count {
            let group = unsafe { *first.add(place) };

            if group.Attributes & SE_GROUP_LOGON_ID == SE_GROUP_LOGON_ID {
                return Ok(Some(Logon {
                    sid: group.Sid,
                    block,
                }));
            }
        }

        Ok(None)
    }

    /// This process's own token, opened well enough for everything below.
    fn own_token() -> Result<Held, String> {
        let mut own: HANDLE = ptr::null_mut();

        let opened = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE
                    | TOKEN_ASSIGN_PRIMARY
                    | TOKEN_QUERY
                    | TOKEN_ADJUST_DEFAULT
                    | READ_CONTROL
                    | WRITE_DAC,
                &mut own,
            )
        };

        if opened == 0 {
            return Err(format!(
                "this process's own token would not open: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(Held(own))
    }

    /// Whose account this token is, read off it — the SID a shape that denies
    /// the user turns deny-only, and the one every directory of a human's
    /// profile is granted to and nothing wider.
    fn user_of(token: &Held) -> Result<Option<Logon>, String> {
        use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_USER, TokenUser};

        let mut wanted = 0u32;

        unsafe { GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut wanted) };

        if wanted == 0 {
            return Err(format!(
                "the token's user would not size itself: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut block = vec![0u8; wanted as usize];

        let read = unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                block.as_mut_ptr().cast(),
                wanted,
                &mut wanted,
            )
        };

        if read == 0 {
            return Err(format!(
                "the token's user would not be read: {}",
                std::io::Error::last_os_error()
            ));
        }

        let sid = unsafe { (*block.as_ptr().cast::<TOKEN_USER>()).User.Sid };

        Ok(Some(Logon { sid, block }))
    }

    /// A restricted token, made from this process's own.
    ///
    /// **Made from this process's own is what makes it startable.** A token
    /// handed to `CreateProcessAsUserW` ordinarily needs a privilege no
    /// per-user install holds; a *restricted* version of the caller's own token
    /// is the documented exception, and the whole reason this mechanism is open
    /// to an install that cannot ask for elevation. Whether the exception holds
    /// on a real machine is one of the things this program is run to find out.
    struct Token(HANDLE);

    impl Token {
        fn restricted(identity: &Identity, shape: Shape) -> Result<Token, String> {
            let mut own: HANDLE = ptr::null_mut();

            let opened = unsafe {
                OpenProcessToken(
                    GetCurrentProcess(),
                    // The last two are what lets the restricted token below have an
                    // entry written on it: a handle can only rewrite a list it was
                    // opened well enough to read.
                    TOKEN_DUPLICATE
                        | TOKEN_ASSIGN_PRIMARY
                        | TOKEN_QUERY
                        | TOKEN_ADJUST_DEFAULT
                        | READ_CONTROL
                        | WRITE_DAC,
                    &mut own,
                )
            };

            if opened == 0 {
                return Err(format!(
                    "this process's own token would not open: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let own = Held(own);
            let mut held = Vec::new();
            let mut restricted = Vec::new();

            if shape.wants_a_list() {
                restricted.push(SID_AND_ATTRIBUTES {
                    Sid: identity.psid(),
                    Attributes: 0,
                });
            }

            for text in shape.beside() {
                let mut sid: PSID = ptr::null_mut();

                let read =
                    unsafe { ConvertStringSidToSidW(wide(OsStr::new(text)).as_ptr(), &mut sid) };

                if read == 0 {
                    return Err(format!(
                        "the SID {text} would not resolve: {}",
                        std::io::Error::last_os_error()
                    ));
                }

                held.push(Local(sid));
                restricted.push(SID_AND_ATTRIBUTES {
                    Sid: sid,
                    Attributes: 0,
                });
            }

            // And this logon's own, read off the token rather than named: it is
            // a different number every sign-in, and the group carrying it says
            // so about itself.
            let groups = shape
                .wants_the_logon()
                .then(|| logon_of(&own))
                .transpose()?
                .flatten();

            if let Some(groups) = &groups {
                restricted.push(SID_AND_ATTRIBUTES {
                    Sid: groups.sid,
                    Attributes: 0,
                });
            }

            // And the human's own SID turned deny-only, where that is what this
            // shape is — read off the token, because whose account this is is
            // the token's to say.
            let mine = shape
                .denies_the_user()
                .then(|| user_of(&own))
                .transpose()?
                .flatten();

            let disable = mine
                .as_ref()
                .map(|mine| {
                    vec![SID_AND_ATTRIBUTES {
                        Sid: mine.sid,
                        Attributes: 0,
                    }]
                })
                .unwrap_or_default();

            let mut made: HANDLE = ptr::null_mut();

            // `DISABLE_MAX_PRIVILEGE` drops every privilege but the one every
            // process keeps, which is what a session has any business with.
            let built = unsafe {
                CreateRestrictedToken(
                    own.0,
                    DISABLE_MAX_PRIVILEGE,
                    disable.len() as u32,
                    if disable.is_empty() {
                        ptr::null()
                    } else {
                        disable.as_ptr()
                    },
                    0,
                    ptr::null(),
                    restricted.len() as u32,
                    restricted.as_ptr(),
                    &mut made,
                )
            };

            drop(held);

            if built == 0 {
                return Err(format!(
                    "CreateRestrictedToken said no: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let token = Token(made);

            // The token itself opened up to the identity, so that a program
            // inside can ask who it is — see [`granted_on`]. `SE_KERNEL_OBJECT`
            // is what a token is one of.
            const TOKEN_ALL_ACCESS: u32 = 0x000F_01FF;

            if let Err(error) =
                granted_on(token.0, SE_KERNEL_OBJECT, identity.psid(), TOKEN_ALL_ACCESS)
            {
                return Err(format!(
                    "the token would not be opened up to itself: {error}"
                ));
            }

            // And lowered, where lowering is what this shape is. A token can
            // always be moved *down* by whoever holds it, which is the whole
            // reason this family is open to a per-user install: nothing here
            // asks the machine for anything it could refuse on privilege
            // grounds.
            if let Some(level) = shape.lowered_to() {
                token.lowered_to(level)?;
            }

            Ok(token)
        }

        /// This token moved down to `level`.
        fn lowered_to(&self, level: &str) -> Result<(), String> {
            use windows_sys::Win32::Security::{
                SetTokenInformation, TOKEN_MANDATORY_LABEL, TokenIntegrityLevel,
            };

            let mut sid: PSID = ptr::null_mut();

            let read =
                unsafe { ConvertStringSidToSidW(wide(OsStr::new(level)).as_ptr(), &mut sid) };

            if read == 0 {
                return Err(format!(
                    "the level {level} would not resolve: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let sid = Local(sid);

            let label = TOKEN_MANDATORY_LABEL {
                Label: SID_AND_ATTRIBUTES {
                    Sid: sid.0,
                    Attributes: SE_GROUP_INTEGRITY,
                },
            };

            let set = unsafe {
                SetTokenInformation(
                    self.0,
                    TokenIntegrityLevel,
                    ptr::from_ref(&label).cast(),
                    size_of::<TOKEN_MANDATORY_LABEL>() as u32,
                )
            };

            if set == 0 {
                return Err(format!(
                    "the token would not be lowered to {level}: {}",
                    std::io::Error::last_os_error()
                ));
            }

            Ok(())
        }
    }

    impl Drop for Token {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// The playground: a directory of each kind, and a junction to one of them.
    fn lay_out(playground: &Path) -> std::io::Result<()> {
        for made in [
            "granted-rw",
            "granted-ro",
            "ungranted",
            "target",
            "labelled-rw",
        ] {
            std::fs::create_dir_all(playground.join(made))?;
        }

        for holding in ["granted-ro", "ungranted", "target"] {
            std::fs::write(playground.join(holding).join("readable"), b"readable\n")?;
        }

        // Made by the machine's own tool rather than by hand, so that what is
        // there is what a rendering would have made.
        let mut mklink = std::process::Command::new("cmd.exe");
        mklink
            .args(["/d", "/c", "mklink", "/J"])
            .arg(playground.join("link"))
            .arg(playground.join("target"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let _ = mklink.status();

        Ok(())
    }

    /// The environment a session is handed: a fresh profile inside the granted
    /// playground, and the four names nothing on this platform runs without.
    fn session_environment(playground: &Path) -> std::io::Result<Vec<(String, String)>> {
        let home = playground.join("granted-rw").join("home");
        let roaming = home.join("AppData").join("Roaming");
        let local = home.join("AppData").join("Local");
        let temporary = local.join("Temp");

        for made in [&home, &roaming, &local, &temporary] {
            std::fs::create_dir_all(made)?;
        }

        let machines = |name: &str| std::env::var(name).unwrap_or_default();

        Ok(vec![
            (String::from("HOME"), home.display().to_string()),
            (String::from("USERPROFILE"), home.display().to_string()),
            (String::from("APPDATA"), roaming.display().to_string()),
            (String::from("LOCALAPPDATA"), local.display().to_string()),
            (String::from("TEMP"), temporary.display().to_string()),
            (String::from("TMP"), temporary.display().to_string()),
            (String::from("PATH"), machines("PATH")),
            (String::from("PATHEXT"), machines("PATHEXT")),
            (String::from("SystemRoot"), machines("SystemRoot")),
            (String::from("SystemDrive"), machines("SystemDrive")),
            (String::from("ComSpec"), machines("ComSpec")),
            (String::from("TERM"), String::from("xterm-256color")),
        ])
    }

    /// A file read, or why it could not be.
    fn reading(path: &Path) -> String {
        match std::fs::read(path) {
            Ok(_) => String::from("read"),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// A connection attempted, or why it could not be.
    fn dialled(address: &str) -> String {
        let Ok(address) = address.parse::<SocketAddr>() else {
            return String::from("not attempted: there was no such address to dial");
        };

        match TcpStream::connect_timeout(&address, PATIENCE) {
            Ok(_) => format!("connected to {address}"),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// What each tool is asked, which is whatever that tool answers.
    fn asks(tool: &str) -> Vec<OsString> {
        match tool {
            "pwsh" | "powershell" => ["-NoProfile", "-NonInteractive", "-Command", "exit 0"]
                .iter()
                .map(OsString::from)
                .collect(),
            "bash" => ["-c", "exit 0"].iter().map(OsString::from).collect(),
            // The human's own registry hive, which is granted to their account
            // and to nothing wider — so whether a restricted process may read it
            // is the same question the profile directory answers, asked of the
            // place every runtime looks before it does anything else.
            "reg" => ["query", r"HKCU\Environment"]
                .iter()
                .map(OsString::from)
                .collect(),
            _ => vec![OsString::from("--version")],
        }
    }

    /// One tool, run under `token` as the machine keeps it.
    fn ran_tool(token: &Token, tool: &str, handed: &[(String, String)]) -> String {
        let Some(path) = on_the_path(tool) else {
            return String::from("not on this machine's PATH, so this went unasked");
        };

        match started(
            token,
            path.as_os_str(),
            &asks(tool),
            None,
            true,
            Some(handed),
        ) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("ran"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// What node makes of a path under this token — the question an
    /// AppContainer failed, and the reason there is a second boundary.
    fn resolving(token: &Token, playground: &Path, handed: &[(String, String)]) -> String {
        let Some(node) = on_the_path("node") else {
            return String::from("node is not on this machine's PATH, so this went unasked");
        };

        let asked_about = [
            playground.join("granted-rw"),
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Users"),
        ];

        let listed = asked_about
            .iter()
            .map(|path| format!("{:?}", path.display().to_string()))
            .collect::<Vec<_>>()
            .join(", ");

        let script = format!(
            "const fs = require('fs'); \
             const tried = (f) => {{ try {{ f(); return 'ok'; }} catch (e) {{ return '!' + \
             (e.code || String(e)); }} }}; \
             for (const p of [{listed}]) {{ \
               console.log(p + ' lstat=' + tried(() => fs.lstatSync(p)) \
                 + ' realpath=' + tried(() => fs.realpathSync(p)) \
                 + ' native=' + tried(() => fs.realpathSync.native(p))); \
             }}"
        );

        let asked = [OsString::from("-e"), OsString::from(script)];

        match started(token, node.as_os_str(), &asked, None, true, Some(handed)) {
            Ok(outcome) => outcome
                .output
                .replace(char::is_control, " ")
                .trim()
                .to_owned(),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// Whether an sccache client under this token reaches a server outside,
    /// which is the whole of whether the shared Rust build cache survives.
    fn sccache(token: &Token, handed: &[(String, String)]) -> String {
        let Some(path) = on_the_path("sccache") else {
            return String::from("not installed, so this went unasked");
        };

        unsafe {
            std::env::set_var("SCCACHE_SERVER_PORT", "4228");
            std::env::set_var("SCCACHE_IDLE_TIMEOUT", "0");
        }

        let mut server = std::process::Command::new(&path);
        server
            .arg("--start-server")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        if server.status().is_err() {
            return String::from("a server outside would not start, so nothing was asked inside");
        }

        match started(
            token,
            path.as_os_str(),
            &[OsString::from("--show-stats")],
            None,
            true,
            Some(handed),
        ) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("reached its server"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("would not start: {error}"),
        }
    }

    /// Whether a pseudoconsole opened out here works for a process started with
    /// this token — the claim stage 01's terminal rests on, asked again because
    /// the token it is handed is not the one it was asked about with.
    fn conpty(token: &Token) -> String {
        let Some(shell) = on_the_path("cmd") else {
            return String::from("there is no cmd.exe on this machine's PATH");
        };

        // Two pipes, four ends: the console reads what is typed at it and
        // writes what is printed on it, and this program holds the other end of
        // each.
        let (typed_at, typing) = match anonymous() {
            Ok(ends) => ends,
            Err(error) => return format!("no pipe for the console: {error}"),
        };
        let (printed, printing) = match anonymous() {
            Ok(ends) => ends,
            Err(error) => return format!("no pipe for the console: {error}"),
        };

        let mut console: isize = 0;

        let made = unsafe {
            CreatePseudoConsole(
                COORD { X: 80, Y: 25 },
                typed_at.0,
                printing.0,
                0,
                &mut console,
            )
        };

        // The console holds its own copies of the two ends it was given.
        drop(typed_at);
        drop(printing);

        if made != 0 {
            return format!("CreatePseudoConsole said {made:#010x}");
        }

        let marker = format!("console-{}", std::process::id());
        let asked = [
            OsString::from("/d"),
            OsString::from("/c"),
            OsString::from(format!("echo {marker}")),
        ];

        let outcome = started(token, shell.as_os_str(), &asked, Some(console), false, None);

        // **Read only once both copies of the printing end have gone**, which
        // is what makes the read end rather than wait forever: this program's
        // own is dropped above, and the console's is only let go of by closing
        // the console. The input end goes here rather than earlier, so that
        // nothing inside met an input stream already at its end.
        drop(typing);
        unsafe { ClosePseudoConsole(console) };

        let said = drained(&printed);

        match outcome {
            Err(error) => format!("the process would not start on it: {error}"),
            Ok(_) if said.contains(&marker) => {
                String::from("the marker came back off the console, so a console handed in works")
            }
            Ok(outcome) => format!(
                "started and exited {}, but the marker never came back; the console said {:?}",
                ended(outcome.exit),
                said.chars().take(200).collect::<String>()
            ),
        }
    }

    /// How a process that did not exit 0 ended, and what it printed on the way.
    fn said_by(outcome: &Outcome) -> String {
        let said = outcome.output.replace(char::is_control, " ");
        let said = said.trim();

        if said.is_empty() {
            format!("exited {}, saying nothing", ended(outcome.exit))
        } else {
            format!("exited {}, saying: {said}", ended(outcome.exit))
        }
    }

    /// An exit as a person reads one.
    fn ended(exit: Option<u32>) -> String {
        exit.map_or_else(
            || String::from("without ending inside this probe's patience"),
            |code| format!("{code:#010x}"),
        )
    }

    /// How a process that ended ended, and what it printed on the way.
    struct Outcome {
        exit: Option<u32>,
        output: String,
    }

    /// One process, started with `token`.
    fn started(
        token: &Token,
        program: &OsStr,
        argv: &[OsString],
        console: Option<isize>,
        capture: bool,
        handed: Option<&[(String, String)]>,
    ) -> Result<Outcome, String> {
        let mut attributes = if console.is_some() {
            Some(Attributes::of(1).map_err(|error| format!("no attribute list: {error}"))?)
        } else {
            None
        };

        if let (Some(attributes), Some(console)) = (attributes.as_mut(), console) {
            attributes
                .carrying(
                    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                    console as *const std::ffi::c_void,
                    size_of::<isize>(),
                )
                .map_err(|error| format!("the console would not go on the list: {error}"))?;
        }

        let piped = if capture {
            Some(anonymous().map_err(|error| format!("no pipe to read it on: {error}"))?)
        } else {
            None
        };

        if let Some((_, writing)) = &piped {
            let set = unsafe {
                SetHandleInformation(writing.0, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT)
            };

            if set == 0 {
                return Err(format!(
                    "the pipe could not be made inheritable: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;

        if let Some(attributes) = attributes.as_mut() {
            startup.lpAttributeList = attributes.list();
        }

        if let Some((_, writing)) = &piped {
            startup.StartupInfo.hStdOutput = writing.0;
            startup.StartupInfo.hStdError = writing.0;
        }

        let mut block = handed.map(environment_block);
        let mut line = command_line(program, argv);
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let mut flags = CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED;

        if attributes.is_some() {
            flags |= EXTENDED_STARTUPINFO_PRESENT;
        }

        let created = unsafe {
            CreateProcessAsUserW(
                token.0,
                ptr::null(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                i32::from(piped.is_some()),
                flags,
                block
                    .as_mut()
                    .map_or(ptr::null(), |block| block.as_mut_ptr().cast()),
                ptr::null(),
                &raw const startup.StartupInfo,
                &mut information,
            )
        };

        if created == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let process = Held(information.hProcess);
        let thread = Held(information.hThread);

        unsafe { ResumeThread(thread.0) };
        drop(thread);

        // This end's copy of the write end goes, or the read below would never
        // see an end of file.
        let reading = piped.map(|(reading, writing)| {
            drop(writing);
            reading
        });

        // **Waited on before it is read, which is the other way round from how
        // it reads.** A read off the pipe runs until the last copy of its write
        // end is closed, and the last copy is the child's — so a child that
        // never ends is a read that never returns, and the wait is the only
        // thing here with a limit on it. What that costs is a child which
        // prints more than a pipe holds before it exits, which would block; a
        // probe's answers are one line each, so it does not.
        let waited = unsafe { WaitForSingleObject(process.0, PATIENCE.as_millis() as u32) };
        let timed_out = waited == WAIT_TIMEOUT;

        if timed_out {
            unsafe { TerminateProcess(process.0, 1) };
        }

        let output = reading.as_ref().map(drained).unwrap_or_default();

        if timed_out {
            return Ok(Outcome { exit: None, output });
        }

        let mut code = 0u32;
        let asked = unsafe { GetExitCodeProcess(process.0, &mut code) };

        Ok(Outcome {
            exit: (asked != 0).then_some(code),
            output,
        })
    }

    /// An environment as `CreateProcessAsUserW` reads one: NUL between each
    /// name and value, and two at the end.
    fn environment_block(handed: &[(String, String)]) -> Vec<u16> {
        let mut block = Vec::new();

        for (name, value) in handed {
            block.extend(wide(OsStr::new(&format!("{name}={value}"))));
        }

        block.push(0);
        block
    }

    /// Everything read off `handle` until there is no more of it.
    fn drained(handle: &Held) -> String {
        let mut said = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            match read_some(handle, &mut buffer) {
                Some(0) | None => break,
                Some(read) => said.extend_from_slice(&buffer[..read]),
            }
        }

        String::from_utf8_lossy(&said).into_owned()
    }

    /// One read off a handle, or nothing where the other end has gone.
    fn read_some(handle: &Held, buffer: &mut [u8]) -> Option<usize> {
        use windows_sys::Win32::Storage::FileSystem::ReadFile;

        let mut read = 0u32;

        let ok = unsafe {
            ReadFile(
                handle.0,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut read,
                ptr::null_mut(),
            )
        };

        if ok == 0 {
            let error = std::io::Error::last_os_error();

            if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                return Some(0);
            }

            return None;
        }

        Some(read as usize)
    }

    /// A pipe, both ends held.
    fn anonymous() -> std::io::Result<(Held, Held)> {
        let mut reading: HANDLE = ptr::null_mut();
        let mut writing: HANDLE = ptr::null_mut();

        let made = unsafe { CreatePipe(&mut reading, &mut writing, ptr::null(), 0) };

        if made == 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok((Held(reading), Held(writing)))
    }

    /// Every access-control entry this program wrote, so that every one of them
    /// can be taken back.
    struct Written {
        sid: PSID,
        paths: Vec<PathBuf>,
        refused: Vec<String>,
    }

    impl Written {
        fn new(sid: PSID) -> Written {
            Written {
                sid,
                paths: Vec::new(),
                refused: Vec::new(),
            }
        }

        /// `path` reachable at `rights`, and everything under it with it.
        fn grant(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
        }

        /// And `path` alone, with nothing under it made reachable by its being
        /// so — which is what a directory only walked through gets.
        fn grant_here(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, NO_INHERITANCE);
        }

        fn wrote(&mut self, path: &Path, rights: u32, inheritance: u32) {
            match self.entry(path, rights, SET_ACCESS, inheritance) {
                Ok(()) => self.paths.push(path.to_owned()),
                Err(error) => self.refused.push(format!("{} ({error})", path.display())),
            }
        }

        fn refusals(&self) -> String {
            if self.refused.is_empty() {
                return String::from("every entry this probe asked for was written");
            }

            format!("could not be written: {}", self.refused.join("; "))
        }

        fn revoke_all(&mut self) -> usize {
            let paths = std::mem::take(&mut self.paths);
            let mut cleared = 0;

            for path in &paths {
                if self.entry(path, 0, REVOKE_ACCESS, NO_INHERITANCE).is_ok() {
                    cleared += 1;
                }
            }

            cleared
        }

        fn entry(
            &self,
            path: &Path,
            rights: u32,
            mode: i32,
            inheritance: u32,
        ) -> Result<(), String> {
            let name = wide(path.as_os_str());

            let mut existing: *mut ACL = ptr::null_mut();
            let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

            let read = unsafe {
                GetNamedSecurityInfoW(
                    name.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut existing,
                    ptr::null_mut(),
                    &mut descriptor,
                )
            };

            if read != 0 {
                return Err(format!("reading the list failed with {read}"));
            }

            let access = EXPLICIT_ACCESS_W {
                grfAccessPermissions: rights,
                grfAccessMode: mode,
                grfInheritance: inheritance,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: ptr::null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: self.sid.cast(),
                },
            };

            let mut wanted: *mut ACL = ptr::null_mut();
            let made = unsafe { SetEntriesInAclW(1, &access, existing, &mut wanted) };

            if made != 0 {
                unsafe { LocalFree(descriptor) };

                return Err(format!("building the list failed with {made}"));
            }

            let set = unsafe {
                SetNamedSecurityInfoW(
                    name.as_ptr().cast_mut(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    wanted,
                    ptr::null_mut(),
                )
            };

            unsafe {
                LocalFree(wanted.cast());
                LocalFree(descriptor);
            }

            if set != 0 {
                return Err(format!("writing the list failed with {set}"));
            }

            Ok(())
        }
    }

    /// An attribute list, held for as long as the call that reads it.
    struct Attributes(Vec<usize>);

    impl Attributes {
        fn of(count: usize) -> Result<Attributes, String> {
            let mut size = 0usize;

            unsafe {
                InitializeProcThreadAttributeList(ptr::null_mut(), count as u32, 0, &mut size)
            };

            if size == 0 {
                return Err(String::from("the list would not size itself"));
            }

            let mut room = vec![0usize; size.div_ceil(size_of::<usize>())];

            let made = unsafe {
                InitializeProcThreadAttributeList(
                    room.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST,
                    count as u32,
                    0,
                    &mut size,
                )
            };

            if made == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }

            Ok(Attributes(room))
        }

        fn carrying(
            &mut self,
            attribute: usize,
            value: *const std::ffi::c_void,
            size: usize,
        ) -> Result<(), String> {
            use windows_sys::Win32::System::Threading::UpdateProcThreadAttribute;

            let updated = unsafe {
                UpdateProcThreadAttribute(
                    self.list(),
                    0,
                    attribute,
                    value,
                    size,
                    ptr::null_mut(),
                    ptr::null(),
                )
            };

            if updated == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }

            Ok(())
        }

        fn list(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
            self.0.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST
        }
    }

    impl Drop for Attributes {
        fn drop(&mut self) {
            unsafe { DeleteProcThreadAttributeList(self.list()) };
        }
    }

    /// A handle closed as it goes.
    struct Held(HANDLE);

    impl Drop for Held {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// A SID the local allocator owns, freed as it goes.
    struct Local(PSID);

    impl Drop for Local {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    /// A command line as `CommandLineToArgvW` reads one back.
    fn command_line(program: &OsStr, argv: &[OsString]) -> Vec<u16> {
        let mut line = Vec::new();

        quoted(program, &mut line);

        for word in argv {
            line.push(u16::from(b' '));
            quoted(word, &mut line);
        }

        line.push(0);
        line
    }

    /// One word of a command line, quoted the way that reader reads one.
    fn quoted(word: &OsStr, line: &mut Vec<u16>) {
        const QUOTE: u16 = b'"' as u16;
        const BACKSLASH: u16 = b'\\' as u16;

        line.push(QUOTE);

        let mut backslashes = 0;

        for unit in word.encode_wide() {
            match unit {
                BACKSLASH => backslashes += 1,
                QUOTE => {
                    for _ in 0..=backslashes {
                        line.push(BACKSLASH);
                    }

                    backslashes = 0;
                }
                _ => backslashes = 0,
            }

            line.push(unit);
        }

        for _ in 0..backslashes {
            line.push(BACKSLASH);
        }

        line.push(QUOTE);
    }

    /// Where a program is, as this machine resolves its name.
    fn on_the_path(program: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;

        for directory in std::env::split_paths(&path) {
            for extension in ["exe", "cmd", "bat", "com"] {
                let candidate = directory.join(format!("{program}.{extension}"));

                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// The human's own profile directory.
    fn home() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }

    /// The machine, said the way a paste wants it.
    fn machine() -> String {
        let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| String::from("(unnamed)"));
        let architecture =
            std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| String::from("(unknown)"));

        format!("{name} on {architecture}")
    }

    /// A path as Win32 wants it.
    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// One line of the paste.
    fn say(line: &str) {
        println!("{line}");
        let _ = std::io::stdout().flush();
    }
}
