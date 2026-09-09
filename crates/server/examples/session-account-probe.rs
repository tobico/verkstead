//! Whether a session can run as a **local account of its own**, asked by
//! attempting.
//!
//! The third of the boundary probes, and the last mechanism this platform has
//! to offer. The two before it settled what does not work, and both settled it
//! the same way — see
//! [ADR-0014](../../../docs/adr/0014-windows-sessions.md):
//!
//! - [`appcontainer-probe`](appcontainer-probe.rs) — an AppContainer refuses a
//!   path *resolution* however well the path is granted, so an agent that
//!   checks a path before reading it refuses every file it has; and msys2 will
//!   not start in one.
//! - [`restricted-token-probe`](restricted-token-probe.rs) — a restricted SID
//!   list, and a deny-only account SID beside it, each give a real boundary and
//!   each break node, both PowerShells and msys2. Only lowering the *integrity*
//!   leaves the toolchain standing, and that costs bash, because msys2 makes its
//!   shared objects under `\BaseNamedObjects` and nothing below medium may write
//!   there.
//!
//! What every one of those has in common is that the process is not quite the
//! human: a stranger's identity, a second access check, or a ceiling. This asks
//! the remaining question — **what if it is somebody else entirely?** A local
//! account of Verkstead's own, with an ordinary token at medium integrity, is
//! not narrowed in any of the ways that broke the toolchain. What a session
//! reaches is then what that account is granted, which is the human's own files
//! not at all until an entry says so.
//!
//! **The price is an elevated step, once.** A local account cannot be created
//! by a per-user install, which is the constraint ADR-0014 wrote the named pipe
//! to avoid — so this probe is in three parts, and the two that touch the
//! machine's accounts say so and refuse to run without the rights:
//!
//! ```text
//! # in an elevated terminal, once
//! cargo run -p verkstead-server --example session-account-probe -- --set-up
//!
//! # as the human, which is how Verkstead itself runs
//! cargo run -p verkstead-server --example session-account-probe
//!
//! # in an elevated terminal again
//! cargo run -p verkstead-server --example session-account-probe -- --tear-down
//! ```
//!
//! **The middle one is the whole point and must be unelevated.** What is being
//! asked is whether *Verkstead* — an ordinary process of the human's, holding
//! no privilege a per-user install could not get — can start a process as that
//! account and hand it a pseudoconsole. `CreateProcessAsUserW` needs a
//! privilege no such process holds; `CreateProcessWithLogonW` needs only the
//! password, and whether it will carry an attribute list is the one claim this
//! whole design rests on. Run the middle part elevated and it will answer for a
//! Verkstead nobody will ship.
//!
//! **What it leaves behind is nothing**: the account is deleted by
//! `--tear-down`, every access-control entry is revoked by the run that wrote
//! it, and the playground goes with it.
//!
//! **It asserts nothing**, as its siblings assert nothing: every question is
//! asked by trying it, and a `no` is as much of an answer as a `yes`.

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "this probe asks a Windows machine about a local account, and this is not one — \
         run it on the Windows machine whose answer is wanted"
    );
}

#[cfg(windows)]
fn main() {
    let argv: Vec<String> = std::env::args().collect();

    match argv.get(1).map(String::as_str) {
        Some(probe::SET_UP) => probe::set_up(),
        Some(probe::TEAR_DOWN) => probe::tear_down(),
        Some(probe::INSIDE) => probe::inside(&argv[2..]),
        Some(probe::LAUNCHER) => probe::launcher(&argv[2..]),
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
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        NetUserAdd, NetUserDel, UF_DONT_EXPIRE_PASSWD, UF_PASSWD_CANT_CHANGE, UF_SCRIPT,
        USER_INFO_1, USER_PRIV_USER,
    };
    use windows_sys::Win32::Security::Authorization::{
        EXPLICIT_ACCESS_W, GetNamedSecurityInfoW, NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS,
        SE_FILE_OBJECT, SET_ACCESS, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID,
        TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, LookupAccountNameW, NO_INHERITANCE, PSECURITY_DESCRIPTOR,
        PSID, SUB_CONTAINERS_AND_OBJECTS_INHERIT,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    };
    use windows_sys::Win32::System::Console::{COORD, ClosePseudoConsole, CreatePseudoConsole};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CREATE_UNICODE_ENVIRONMENT, CreateProcessWithLogonW, DeleteProcThreadAttributeList,
        EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
        LOGON_WITH_PROFILE, LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
        PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
        WaitForSingleObject,
    };

    /// The words that say which part of this program a run is.
    pub const SET_UP: &str = "--set-up";
    pub const TEAR_DOWN: &str = "--tear-down";
    pub const INSIDE: &str = "--inside";
    pub const LAUNCHER: &str = "--launcher";

    /// The account this makes, and where the password it made goes.
    ///
    /// **A file rather than a printed line**, so that the unelevated run in the
    /// middle needs nothing pasted into it and the password is never on a
    /// screen or in a shell's history. Under the temporary directory because
    /// this is a probe; what Verkstead would do with one is keep it beside its
    /// other secrets.
    const ACCOUNT: &str = "verkstead-probe";
    const SECRET: &str = "verkstead-probe-password.txt";

    /// How long anything here waits for a process it started, or for a
    /// connection to answer.
    const PATIENCE: Duration = Duration::from_secs(10);

    /// The elevated first part: make the account.
    pub fn set_up() {
        say("session-account-probe --set-up");

        let password = password();

        let mut name = wide(OsStr::new(ACCOUNT));
        let mut secret = wide(OsStr::new(&password));
        let mut comment = wide(OsStr::new(
            "Verkstead probe account — delete this if it outlives the probe",
        ));

        let mut wanted = USER_INFO_1 {
            usri1_name: name.as_mut_ptr(),
            usri1_password: secret.as_mut_ptr(),
            usri1_password_age: 0,
            usri1_priv: USER_PRIV_USER,
            usri1_home_dir: ptr::null_mut(),
            usri1_comment: comment.as_mut_ptr(),
            // A password that does not expire and that the account cannot
            // change: it is Verkstead's to know rather than anybody's to type.
            usri1_flags: UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE,
            usri1_script_path: ptr::null_mut(),
        };

        let made = unsafe {
            NetUserAdd(
                ptr::null(),
                1,
                ptr::from_mut(&mut wanted).cast(),
                ptr::null_mut(),
            )
        };

        match made {
            0 => say(&format!("account        = {ACCOUNT} created")),
            5 => {
                say(
                    "account        = refused with access denied, which is this part of the \
                     probe saying it needs an elevated terminal",
                );
                return;
            }
            2224 => say(&format!("account        = {ACCOUNT} was already there")),
            said => {
                say(&format!("account        = NetUserAdd said {said}"));
                return;
            }
        }

        match std::fs::write(kept(), &password) {
            Ok(()) => say(&format!("password       = written to {}", kept().display())),
            Err(error) => say(&format!("password       = would not be written: {error}")),
        }

        say("next           = run this again with no arguments, as yourself and NOT elevated");
    }

    /// The elevated third part: take it away again.
    pub fn tear_down() {
        say("session-account-probe --tear-down");

        let gone = unsafe { NetUserDel(ptr::null(), wide(OsStr::new(ACCOUNT)).as_ptr()) };

        match gone {
            0 => say(&format!("account        = {ACCOUNT} deleted")),
            5 => say(
                "account        = refused with access denied, which is this part of the probe \
                 saying it needs an elevated terminal",
            ),
            2221 => say(&format!("account        = {ACCOUNT} was not there")),
            said => say(&format!("account        = NetUserDel said {said}")),
        }

        match std::fs::remove_file(kept()) {
            Ok(()) => say("password       = removed"),
            Err(error) => say(&format!("password       = not removed: {error}")),
        }
    }

    /// The middle part, and the one that answers anything: as the human, start
    /// a process as the account and ask it what it can do.
    pub fn outside() {
        say("session-account-probe — paste everything from this line down");
        say(&format!("machine        = {}", machine()));

        let Ok(password) = std::fs::read_to_string(kept()) else {
            say(&format!(
                "account        = there is no password at {} — run this with {SET_UP} in an \
                 elevated terminal first",
                kept().display()
            ));
            return;
        };

        let Ok(exe) = std::env::current_exe() else {
            say("account        = this program cannot say where it is");
            return;
        };

        let sid = match sid_of(ACCOUNT) {
            Ok(sid) => sid,
            Err(error) => {
                say(&format!(
                    "account        = {ACCOUNT} would not resolve: {error}"
                ));
                return;
            }
        };

        say(&format!("account        = {ACCOUNT} is {}", sid.text));

        let playground = std::env::temp_dir().join("verkstead-account-probe");
        let mut written = Written::new(sid.psid());

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

        // Walked through but not opened into, so that the ungranted directory
        // under it really is ungranted.
        written.grant_here(&playground, FILE_GENERIC_EXECUTE);

        say(&format!("entries        = {}", written.refusals()));

        // **And every directory on the way to it, walked through and no more.**
        //
        // Which is a thing the AppContainer rendering never had to do and this
        // one does. *Reaching* a deep path needs no ancestor granted — every
        // ordinary account holds `SeChangeNotifyPrivilege`, which is the
        // privilege to skip the traverse check — but *resolving* one walks the
        // prefixes and asks each for its attributes, and an agent resolves a
        // path before it reads it. The human's own profile directory is on that
        // walk and is granted to their account and nothing wider, so without
        // this the walk stops there.
        //
        // `FILE_GENERIC_EXECUTE` and no inheritance is the whole of it: traverse
        // and read-attributes on that one directory. It says nothing about what
        // is inside, so the profile stays as unlistable and unreadable as it
        // was — which the lines below go on to check.
        let mut walked = 0;
        let mut refused = 0;

        for ancestor in playground.ancestors().skip(1) {
            match written.entry_at(ancestor, FILE_GENERIC_EXECUTE) {
                Ok(()) => walked += 1,
                Err(_) => refused += 1,
            }
        }

        say(&format!(
            "walking        = {walked} directories on the way in granted a step through, \
             {refused} refused this process (which is the machine's own, and grants Users \
             a step already)"
        ));

        // Something to dial, so that "refused" and "there was nothing there"
        // are not the same answer.
        let listener = TcpListener::bind("127.0.0.1:0").ok();
        let loopback = listener
            .as_ref()
            .and_then(|listener| listener.local_addr().ok())
            .map_or_else(String::new, |address| address.to_string());

        say(&format!("dialling       = {loopback}"));

        // **The inside half runs from inside the playground**, and the first
        // run of this did not: it was started from the build directory under
        // the human's own profile, which this account is refused, and the
        // answer was `Access is denied` before a question had been asked.
        // Which is the boundary working — and is also what a rendering already
        // does about it, binding Verkstead's own binary into a directory the
        // session may reach.
        let inside_exe = playground.join("granted-rw").join("probe.exe");

        if let Err(error) = std::fs::copy(&exe, &inside_exe) {
            say(&format!("inside         = would not be copied in: {error}"));
            return;
        }

        let password = password.trim().to_owned();
        let asked = [
            OsString::from(INSIDE),
            playground.clone().into_os_string(),
            OsString::from(&loopback),
            home().unwrap_or_default().into_os_string(),
        ];

        match started(&password, inside_exe.as_os_str(), &asked, None, true) {
            Ok(outcome) if outcome.output.trim().is_empty() => say(&format!(
                "inside         = printed nothing and {}",
                ended(outcome.exit)
            )),
            Ok(outcome) => {
                for line in outcome.output.lines() {
                    say(line);
                }
            }
            Err(error) => say(&format!("inside         = would not start: {error}")),
        }

        // **The claim the whole design rests on.** A pseudoconsole is an
        // attribute on an extended startup info, and whether the call that
        // starts a process as somebody else without a privilege will carry one
        // is documented nowhere this could be read off.
        say(&format!("conpty         = {}", conpty(&password)));
        say(&format!(
            "conpty-inside  = {}",
            via_launcher(&password, &inside_exe)
        ));

        for tool in ["node", "pwsh", "powershell", "git", "bash", "reg"] {
            say(&format!("tool {tool:10} = {}", ran_tool(&password, tool)));
        }

        say(&format!(
            "resolving      = {}",
            resolving(&password, &playground)
        ));

        let revoked = written.revoke_all();
        let removed = std::fs::remove_dir_all(&playground);

        say(&format!(
            "cleanup        = {revoked} entries revoked, playground {}",
            if removed.is_ok() { "removed" } else { "left" },
        ));

        say(&format!(
            "next           = run this with {TEAR_DOWN} in an elevated terminal"
        ));
        say("session-account-probe end — paste up to this line");
    }

    /// The inside half: one line per thing tried, printed as the account.
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

        say(&format!(
            "system-read    = {}",
            reading(&PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts"))
        ));

        // The human's own profile, which this boundary refuses in both
        // directions: it is granted to their account, and this is not it.
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

    /// The launcher: a process running **as the account**, which makes the
    /// pseudoconsole itself and starts something on it.
    ///
    /// **What this is for.** A pseudoconsole is an attribute on an extended
    /// startup info, and the one call that starts a process as somebody else
    /// without a privilege will not carry one — see [`conpty`], which asks that
    /// and is refused. So the console is made on the far side of the boundary
    /// instead: a small program of Verkstead's own is started as the account
    /// with plain pipes for its standard handles, and *it* calls
    /// `CreatePseudoConsole` over those and starts the session on it with an
    /// ordinary `CreateProcessW`, which carries an attribute list happily
    /// because it is starting a process as the account it already is.
    ///
    /// **Its standard handles are the console's two ends**, which is what makes
    /// it need nothing else passed to it: what is typed at the session arrives
    /// on this program's stdin and what the session prints leaves on its
    /// stdout, so the two pipes Verkstead already holds are the two pipes the
    /// console is built on and no handle has to be duplicated across the
    /// boundary.
    pub fn launcher(argv: &[String]) {
        use windows_sys::Win32::System::Console::GetStdHandle;
        use windows_sys::Win32::System::Console::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

        let marker = argv.first().cloned().unwrap_or_default();

        let typed_at = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        let printing = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

        let mut console: isize = 0;

        let made = unsafe {
            CreatePseudoConsole(COORD { X: 80, Y: 25 }, typed_at, printing, 0, &mut console)
        };

        if made != 0 {
            // Said on the one channel there is, which the far side is reading.
            let _ = std::io::stdout()
                .write_all(format!("launcher: CreatePseudoConsole said {made:#010x}\n").as_bytes());
            return;
        }

        let Some(shell) = on_the_path("cmd") else {
            let _ = std::io::stdout().write_all(b"launcher: there is no cmd.exe\n");
            return;
        };

        let asked = [
            OsString::from("/d"),
            OsString::from("/c"),
            OsString::from(format!("echo {marker}")),
        ];

        let said = match as_itself(shell.as_os_str(), &asked, console) {
            Ok(()) => String::new(),
            Err(error) => format!("launcher: the process would not start: {error}\n"),
        };

        unsafe { ClosePseudoConsole(console) };

        if !said.is_empty() {
            let _ = std::io::stdout().write_all(said.as_bytes());
        }
    }

    /// One process started by the launcher, as the account it already is — an
    /// ordinary `CreateProcessW` with the console on its attribute list.
    fn as_itself(program: &OsStr, argv: &[OsString], console: isize) -> Result<(), String> {
        use windows_sys::Win32::System::Threading::CreateProcessW;

        let mut attributes =
            Attributes::of(1).map_err(|error| format!("no attribute list: {error}"))?;

        attributes
            .carrying(
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console as *const std::ffi::c_void,
                size_of::<isize>(),
            )
            .map_err(|error| format!("the console would not go on the list: {error}"))?;

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = attributes.list();

        let mut line = command_line(program, argv);
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let created = unsafe {
            CreateProcessW(
                ptr::null(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT,
                ptr::null(),
                ptr::null(),
                &raw const startup.StartupInfo,
                &mut information,
            )
        };

        if created == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let process = Held(information.hProcess);
        drop(Held(information.hThread));

        unsafe { WaitForSingleObject(process.0, PATIENCE.as_millis() as u32) };

        Ok(())
    }

    /// Whether a console made on the far side of the boundary works, which is
    /// what the answer to [`conpty`] leaves to be settled.
    fn via_launcher(password: &str, exe: &Path) -> String {
        let (typed_at, typing) = match anonymous() {
            Ok(ends) => ends,
            Err(error) => return format!("no pipe for the console: {error}"),
        };
        let (printed, printing) = match anonymous() {
            Ok(ends) => ends,
            Err(error) => return format!("no pipe for the console: {error}"),
        };

        // The two the launcher inherits as its own standard handles — which is
        // the whole of what crosses, and is why nothing has to be duplicated
        // into a process started as somebody else.
        for end in [&typed_at, &printing] {
            let set =
                unsafe { SetHandleInformation(end.0, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) };

            if set == 0 {
                return format!(
                    "an end could not be made inheritable: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        let marker = format!("launched-{}", std::process::id());

        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = typed_at.0;
        startup.StartupInfo.hStdOutput = printing.0;
        startup.StartupInfo.hStdError = printing.0;

        let mut line = command_line(
            exe.as_os_str(),
            &[OsString::from(LAUNCHER), OsString::from(&marker)],
        );
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let created = unsafe {
            CreateProcessWithLogonW(
                wide(OsStr::new(ACCOUNT)).as_ptr(),
                ptr::null(),
                wide(OsStr::new(password)).as_ptr(),
                LOGON_WITH_PROFILE,
                ptr::null(),
                line.as_mut_ptr(),
                CREATE_UNICODE_ENVIRONMENT,
                ptr::null(),
                ptr::null(),
                &raw const startup.StartupInfo,
                &mut information,
            )
        };

        if created == 0 {
            return format!(
                "the launcher would not start: {}",
                std::io::Error::last_os_error()
            );
        }

        let process = Held(information.hProcess);
        drop(Held(information.hThread));

        // This end's copies go, so that the read below sees an end of file when
        // the launcher and its console have both let go.
        drop(typed_at);
        drop(printing);

        let waited = unsafe { WaitForSingleObject(process.0, PATIENCE.as_millis() as u32) };

        if waited == WAIT_TIMEOUT {
            unsafe { TerminateProcess(process.0, 1) };
        }

        drop(typing);

        let said = drained(&printed);

        if said.contains(&marker) {
            String::from(
                "the marker came back off a console the launcher made, so a console made inside works",
            )
        } else {
            format!(
                "the launcher ran and the marker never came back; it said {:?}",
                said.chars().take(300).collect::<String>()
            )
        }
    }

    /// Where the password this probe made is kept between its three parts.
    fn kept() -> PathBuf {
        std::env::temp_dir().join(SECRET)
    }

    /// One the account is made with: long, random, and never seen by anybody.
    ///
    /// The operating system's own generator and nothing around it, which is the
    /// same call the branch names are picked with — see the `getrandom` line in
    /// the workspace manifest.
    fn password() -> String {
        const FROM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the operating system's generator");

        // A digit and a letter of each case in front of it, so that a machine
        // with a password policy on it takes this without argument.
        let mut said = String::from("Vk1-");

        for byte in bytes {
            said.push(FROM[byte as usize % FROM.len()] as char);
        }

        said
    }

    /// The SID of a local account, which is what an entry names.
    fn sid_of(account: &str) -> Result<Sid, String> {
        let name = wide(OsStr::new(account));

        let mut room = 0u32;
        let mut domain = 0u32;
        let mut kind = 0i32;

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

        let mut sid = vec![0u8; room as usize];
        let mut said = vec![0u16; domain as usize];

        let found = unsafe {
            LookupAccountNameW(
                ptr::null(),
                name.as_ptr(),
                sid.as_mut_ptr().cast(),
                &mut room,
                said.as_mut_ptr(),
                &mut domain,
                &mut kind,
            )
        };

        if found == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let text = text_of(sid.as_ptr() as PSID).unwrap_or_else(|| String::from("(unreadable)"));

        Ok(Sid { sid, text })
    }

    /// An account's SID, held as the bytes it is.
    struct Sid {
        sid: Vec<u8>,
        text: String,
    }

    impl Sid {
        fn psid(&self) -> PSID {
            self.sid.as_ptr() as PSID
        }
    }

    /// The playground: a directory of each kind, and a junction to one of them.
    fn lay_out(playground: &Path) -> std::io::Result<()> {
        let _ = std::fs::remove_dir_all(playground);

        for made in ["granted-rw", "granted-ro", "ungranted", "target"] {
            std::fs::create_dir_all(playground.join(made))?;
        }

        for holding in ["granted-ro", "ungranted", "target"] {
            std::fs::write(playground.join(holding).join("readable"), b"readable\n")?;
        }

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
            "reg" => ["query", r"HKCU\Environment"]
                .iter()
                .map(OsString::from)
                .collect(),
            _ => vec![OsString::from("--version")],
        }
    }

    /// One tool, run as the account.
    fn ran_tool(password: &str, tool: &str) -> String {
        let Some(path) = on_the_path(tool) else {
            return String::from("not on this machine's PATH, so this went unasked");
        };

        match started(password, path.as_os_str(), &asks(tool), None, true) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("ran"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// What node makes of a path as this account — the question an AppContainer
    /// failed and a lowered token passed.
    fn resolving(password: &str, playground: &Path) -> String {
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

        match started(password, node.as_os_str(), &asked, None, true) {
            Ok(outcome) => outcome
                .output
                .replace(char::is_control, " ")
                .trim()
                .to_owned(),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// Whether a pseudoconsole opened out here works for a process started as
    /// somebody else — the claim a Windows session's terminal rests on, and the
    /// one this mechanism could not survive losing.
    fn conpty(password: &str) -> String {
        let Some(shell) = on_the_path("cmd") else {
            return String::from("there is no cmd.exe on this machine's PATH");
        };

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

        let outcome = started(password, shell.as_os_str(), &asked, Some(console), false);

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

    /// One process, started as the account.
    ///
    /// **`CreateProcessWithLogonW` rather than `CreateProcessAsUserW`**, and
    /// that is the whole reason this mechanism is open to a per-user install:
    /// the second needs `SeAssignPrimaryTokenPrivilege`, which an ordinary
    /// process of a human's does not hold and cannot ask for, and the first
    /// needs only the account's password.
    fn started(
        password: &str,
        program: &OsStr,
        argv: &[OsString],
        console: Option<isize>,
        capture: bool,
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

        // **Every handle a process started this way is to inherit has to be
        // inheritable and nothing else**: this call has no `bInheritHandles` of
        // its own, and what it does with the standard handles is the one thing
        // about it worth finding out beside the console.
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

        let mut line = command_line(program, argv);
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let mut flags = CREATE_UNICODE_ENVIRONMENT;

        if attributes.is_some() {
            flags |= EXTENDED_STARTUPINFO_PRESENT;
        }

        let created = unsafe {
            CreateProcessWithLogonW(
                wide(OsStr::new(ACCOUNT)).as_ptr(),
                // The local machine, which is what a null domain means here.
                ptr::null(),
                wide(OsStr::new(password)).as_ptr(),
                LOGON_WITH_PROFILE,
                ptr::null(),
                line.as_mut_ptr(),
                flags,
                ptr::null(),
                ptr::null(),
                &raw const startup.StartupInfo,
                &mut information,
            )
        };

        if created == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let process = Held(information.hProcess);
        drop(Held(information.hThread));

        let reading = piped.map(|(reading, writing)| {
            drop(writing);
            reading
        });

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

        fn grant(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
        }

        fn grant_here(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, NO_INHERITANCE);
        }

        /// The same, and whether it took — for the ancestors, where being
        /// refused is an answer rather than a fault: the directories above a
        /// human's profile are the machine's own and grant `Users` a step
        /// through already.
        fn entry_at(&mut self, path: &Path, rights: u32) -> Result<(), String> {
            self.entry(path, rights, SET_ACCESS, NO_INHERITANCE)?;
            self.paths.push(path.to_owned());

            Ok(())
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

    /// A SID as a person reads one.
    fn text_of(sid: PSID) -> Option<String> {
        use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;

        let mut written: *mut u16 = ptr::null_mut();

        let made = unsafe { ConvertSidToStringSidW(sid, &mut written) };

        if made == 0 || written.is_null() {
            return None;
        }

        let mut length = 0;

        while unsafe { *written.add(length) } != 0 {
            length += 1;
        }

        let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(written, length) });

        unsafe { LocalFree(written.cast()) };

        Some(text)
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
