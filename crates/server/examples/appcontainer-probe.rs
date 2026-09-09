//! What a Windows machine really allows an AppContainer, asked by attempting.
//!
//! [ADR-0014](../../../docs/adr/0014-windows-sessions.md) decides that a
//! Windows session runs inside an AppContainer, and three of the claims that
//! decision rests on were made from documentation rather than from a machine:
//! that a desktop AppContainer is refused the loopback interface, that `node`
//! and `pwsh` run under one at all, and that a ConPTY opened outside and handed
//! in works. A rendering built on a claim the machine contradicts would be a
//! stage rebuilt, so the container stage opens with this: a program the human
//! runs on their own Windows 11 machine, whose whole output is one screen they
//! can paste back.
//!
//! **It starts its process with the capability, which it did not always do.**
//! A profile is registered with the internet client and a token is built with
//! one, and those are two calls: this probe passed the first and left the
//! second empty, so every network answer it gave was a container holding no
//! capability at all rather than the container a session runs in. The loopback
//! answers were the same either way — inside is refused `127.0.0.1` and the
//! machine's own address whether the capability is held or not — but the reach
//! a session depends on was never asked about here until the day a session
//! could not reach it.
//!
//! **It asserts nothing.** Every question here is asked by trying it and
//! reporting what happened, and a `no` is as much of an answer as a `yes` —
//! there is no failure mode where this program is right and the machine is
//! wrong. So it always exits 0 where it got as far as printing, and what it
//! could not do at all it says on the line for it.
//!
//! **It is two programs.** Run with no arguments it is the outside half: it
//! makes a throwaway profile, lays out a playground of directories with the
//! access-control entries the real rendering would write, and starts the inside
//! half in the container. Run with `--inside` it is that inside half, which
//! tries each thing and prints one line each. Nothing is shared between them
//! but the command line, so what the inside half reports is what a process in
//! an AppContainer can really do.
//!
//! **What it leaves behind is nothing.** The profile is deleted, every entry it
//! wrote is revoked, and the playground goes — including on the paths it had to
//! touch outside its own directory, which are the ones the machine would
//! otherwise keep: the directory this program is in, and each tool's.
//!
//! Run it as:
//!
//! ```text
//! cargo run -p verkstead-server --example appcontainer-probe
//! ```

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "this probe asks a Windows machine about AppContainers, and this is not one — \
         run it on the Windows 11 machine whose answer is wanted"
    );
}

#[cfg(windows)]
fn main() {
    let inside: Vec<String> = std::env::args().collect();

    match inside.get(1).map(String::as_str) {
        Some(probe::INSIDE) => probe::inside(&inside[2..]),
        _ => probe::outside(),
    }
}

#[cfg(windows)]
mod probe {
    use std::ffi::{OsStr, OsString};
    use std::io::Write;
    use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, GENERIC_READ, GENERIC_WRITE, HANDLE, HANDLE_FLAG_INHERIT,
        INVALID_HANDLE_VALUE, LocalFree, SetHandleInformation, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        ConvertStringSidToSidW, DENY_ACCESS, EXPLICIT_ACCESS_W, GetNamedSecurityInfoW,
        NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS, SDDL_REVISION_1, SE_FILE_OBJECT, SET_ACCESS,
        SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::GetTokenInformation;
    use windows_sys::Win32::Security::Isolation::{
        CreateAppContainerProfile, DeleteAppContainerProfile,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, FreeSid, NO_INHERITANCE, PSECURITY_DESCRIPTOR, PSID,
        SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
        SUB_CONTAINERS_AND_OBJECTS_INHERIT, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX, ReadFile,
    };
    use windows_sys::Win32::System::Console::{COORD, ClosePseudoConsole, CreatePseudoConsole};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows_sys::Win32::System::Threading::{
        CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
        DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
        GetExitCodeProcess, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
        OpenProcessToken, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
        STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess, WaitForSingleObject,
    };

    /// The word that says this run is the inside half.
    pub const INSIDE: &str = "--inside";

    /// The capability the ADR grants and the only one: the internet client, so
    /// that a session can reach the network it is meant to and no part of the
    /// local one it is not.
    ///
    /// Written as a SID rather than derived from a name, because that is what
    /// it is: `S-1-15-3-1` is `internetClient` on every Windows there is.
    const INTERNET_CLIENT: &str = "S-1-15-3-1";

    /// What is dialled to ask whether a session can reach the internet: the
    /// model's API, which is the reach a session most obviously depends on and
    /// the one whose absence sent somebody looking at their DNS.
    const OUTSIDE: &str = "api.anthropic.com:443";

    /// What an enabled group is, in the attributes half of a capability.
    const SE_GROUP_ENABLED: u32 = 0x0000_0004;

    /// How long anything here waits for a process it started, or for a
    /// connection to answer.
    ///
    /// Everything this starts either answers at once or is telling us
    /// something, so the wait is short: a machine that takes longer than this
    /// to refuse a connection has answered *refused slowly*, which is worth
    /// seeing rather than worth sitting through.
    const PATIENCE: Duration = Duration::from_secs(10);

    /// The outside half: make a container, lay out what the real rendering
    /// would lay out, and ask the inside half what it can do.
    pub fn outside() {
        say("appcontainer-probe 1 — paste everything from this line down");
        say(&format!("machine        = {}", machine()));

        let Some(exe) = std::env::current_exe().ok() else {
            say("profile        = not attempted: this program cannot say where it is");
            return;
        };

        let name = format!("verkstead-probe-{}", std::process::id());

        // Deleted first, in case a run before this one ended somewhere it could
        // not tidy up after itself: a profile that is already there is a
        // creation that fails for a reason that says nothing about this
        // machine.
        unsafe { DeleteAppContainerProfile(wide(OsStr::new(&name)).as_ptr()) };

        let profile = match Profile::created(&name) {
            Ok(profile) => profile,
            Err(error) => {
                say(&format!("profile        = refused: {error}"));
                say(
                    "nothing below could be asked without one, so this is the whole of the \
                     answer: a per-user install cannot make an AppContainer on this machine",
                );
                return;
            }
        };

        say(&format!("profile        = created as {name}"));
        say(&format!("profile-sid    = {}", profile.sid_text));

        let mut written = Written::new(profile.sid);

        // Where the whole playground goes. Under the machine's own temporary
        // directory, which is inside the human's profile and so is exactly the
        // shape the real rendering has to work in: a container is granted the
        // leaves and nothing above them, and whether it can reach them at all
        // is the first thing being asked here.
        let playground = std::env::temp_dir().join(&name);
        let _ = std::fs::remove_dir_all(&playground);

        if let Err(error) = lay_out(&playground) {
            say(&format!("playground     = not made: {error}"));
            return;
        }

        say(&format!("playground     = {}", playground.display()));

        // The playground itself walkable and **nothing under it made reachable
        // by that**, which is the whole reason this one is `grant_here`: an
        // inheriting grant here would reach every path below, and the ungranted
        // one this lays out to ask about would be granted after all. The first
        // run of this probe made exactly that mistake and reported an ungranted
        // directory as readable.
        //
        // Nothing *above* the playground is granted anything at all, which is
        // the other question: a machine where an AppContainer needs traverse on
        // every ancestor is a machine where the rendering has to grant the
        // human's whole profile on the way to a Worktree.
        written.grant_here(&playground, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE);
        written.grant(
            &playground.join("granted-rw"),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
        );
        written.grant(
            &playground.join("target"),
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        );
        // And the one thing a description says a session must *not* reach,
        // rendered the way a platform with nothing to mount over it has to
        // render it: an entry that refuses, inside a tree that allows.
        written.deny(&playground.join("granted-rw").join("denied"));

        // And the program itself, because a container that cannot read the
        // image cannot start it — which would make every line below say
        // *refused* for a reason that has nothing to do with what is being
        // asked.
        let held = exe.parent().unwrap_or(Path::new(".")).to_owned();
        written.grant(&held, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE);

        say(&format!("entries        = {}", written.refusals()));

        // Something outside to dial, on both of the addresses the ADR wonders
        // about: the loopback, and this machine's own.
        let listening = TcpListener::bind("0.0.0.0:0").ok();
        let port = listening
            .as_ref()
            .and_then(|listening| listening.local_addr().ok())
            .map(|address| address.port());

        if let Some(listening) = listening {
            std::thread::spawn(move || {
                while let Ok((connected, _)) = listening.accept() {
                    drop(connected);
                }
            });
        }

        let loopback = port.map_or_else(|| String::from("-"), |port| format!("127.0.0.1:{port}"));
        let own = match (own_address(), port) {
            (Some(address), Some(port)) => format!("{address}:{port}"),
            _ => String::from("-"),
        };

        say(&format!("dialling       = {loopback} and {own}"));

        // And a named pipe whose descriptor grants this container and nothing
        // else beyond the account running the probe — the shape
        // `crates/server/src/pipe.rs` creates every instance of its own with,
        // and the whole of what a session inside a container would ask through.
        let pipe = format!(r"\\.\pipe\{name}");
        let piped = Pipe::granting(&pipe, &profile.sid_text);

        say(&format!(
            "pipe           = {}",
            match &piped {
                Ok(_) => format!("open at {pipe}"),
                Err(error) => format!("not opened: {error}"),
            }
        ));

        // The inside half, in the container, with its output read back.
        let mut argv = vec![OsString::from(INSIDE)];
        argv.push(playground.clone().into_os_string());
        argv.push(OsString::from(&pipe));
        argv.push(OsString::from(&loopback));
        argv.push(OsString::from(&own));

        let ran = started(&profile, exe.as_os_str(), &argv, None, true);

        match &ran {
            Ok(outcome) => {
                for line in outcome.output.lines() {
                    say(line);
                }

                if outcome.output.trim().is_empty() {
                    say(&format!(
                        "inside         = started and said nothing, exiting {:?}",
                        outcome.exit
                    ));
                }
            }
            Err(error) => {
                say(&format!("inside         = would not start: {error}"));
                say(
                    "which is the answer to every question below it as well: a program the \
                     container was granted read and execute on could not be started at all",
                );
            }
        }

        drop(piped);

        // The console question, which is its own run: a pseudoconsole opened
        // out here by the same calls `crates/server/src/terminal` makes, and the
        // process started on it from inside the container. What is being asked
        // is whether the handing-in survives the boundary, so the marker is
        // read back off the console's own pipe rather than off a file.
        say(&format!("conpty         = {}", conpty(&profile)));

        // The tools, each asked twice: as the machine keeps it, and again with
        // its own directory granted. The first answer is what says whether
        // Program Files really is readable by every container; the second is
        // what says whether a per-user install can be granted instead.
        //
        // **Windows PowerShell is in the list beside `pwsh`**, and it is the
        // one that cannot be missing: ADR-0014 opens a Conversation Terminal on
        // `pwsh` where somebody installed PowerShell 7 and on Windows
        // PowerShell where nobody has, so the fallback is on every machine and
        // is what has to work. `node` and `npm` are there because an agent is
        // ordinarily an npm install, which is the per-user case the grant is
        // for — a machine without them leaves that question open rather than
        // answered, and says so.
        // **`bash` is in the list because an agent's own shell is**, and it is
        // the one this probe was extended for: Claude's shell tool on Windows
        // is Git for Windows' bash, which is msys2, and a session that cannot
        // run it has no shell at all whatever else works.
        for tool in ["node", "npm", "pwsh", "powershell", "git", "bash"] {
            say(&format!(
                "tool {tool:10} = {}",
                ran_tool(&profile, tool, &mut written)
            ));
        }

        // And the two things an agent does with a path that nothing above it
        // does: run a batch file the way `sandbox::open` runs one, and resolve
        // a path the way a program written in node resolves one.
        say(&format!(
            "batch          = {}",
            batch(&profile, &playground)
        ));
        say(&format!(
            "resolving      = {}",
            resolving(&profile, &playground)
        ));

        say(&format!(
            "sccache        = {}",
            sccache(&profile, &mut written)
        ));

        // And everything this wrote, taken back: the entries first, because the
        // profile's SID is what names them and a deleted profile leaves them
        // standing as a number nothing can resolve.
        let revoked = written.revoke_all();
        let removed = std::fs::remove_dir_all(&playground);

        drop(profile);
        let deleted = unsafe { DeleteAppContainerProfile(wide(OsStr::new(&name)).as_ptr()) };

        say(&format!(
            "cleanup        = {revoked} entries revoked, playground {}, profile {}",
            if removed.is_ok() { "removed" } else { "left" },
            if deleted >= 0 { "deleted" } else { "left" },
        ));
        say("appcontainer-probe end — paste up to this line");
    }

    /// The inside half: everything tried from within the container, one line
    /// each.
    ///
    /// Nothing here knows it is in a container. It opens what it was told to
    /// open and reports what happened, which is the whole point: the answers
    /// are the operating system's rather than this program's.
    pub fn inside(argv: &[String]) {
        let playground = PathBuf::from(argv.first().cloned().unwrap_or_default());
        let pipe = argv.get(1).cloned().unwrap_or_default();
        let loopback = argv.get(2).cloned().unwrap_or_default();
        let own = argv.get(3).cloned().unwrap_or_default();

        let granted = playground.join("granted-rw");

        say(&format!(
            "grant-write    = {}",
            match std::fs::write(granted.join("written-from-inside"), b"inside\n") {
                Ok(()) => String::from("wrote"),
                Err(error) => format!("refused: {error}"),
            }
        ));

        say(&format!(
            "grant-read     = {}",
            reading(&playground.join("target").join("readable"))
        ));

        say(&format!(
            "grant-none     = {}",
            reading(&playground.join("ungranted").join("readable"))
        ));

        say(&format!(
            "deny-entry     = {}",
            reading(&granted.join("denied").join("readable"))
        ));

        // The junction, which is how a Windows session finds its account: the
        // grant is on the directory the junction points at and there is none on
        // the junction, so what this says is which end the machine checks.
        say(&format!(
            "junction       = {}",
            reading(&playground.join("link").join("readable"))
        ));

        say(&format!("loopback       = {}", dialled(&loopback)));
        say(&format!("own-address    = {}", dialled(&own)));
        say(&format!("internet       = {}", reached(OUTSIDE)));

        say(&format!(
            "named-pipe     = {}",
            match opened(&pipe) {
                Ok(()) => String::from("opened"),
                Err(error) => format!("refused: {error}"),
            }
        ));

        let _ = std::io::stdout().flush();
    }

    /// A file read, or why it could not be.
    fn reading(path: &Path) -> String {
        match std::fs::read(path) {
            Ok(_) => String::from("read"),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// A connection attempted, or why it could not be.
    ///
    /// A refusal and a timeout are two different answers and both are worth
    /// having: the first is a firewall saying no, and the second is one
    /// dropping the packet, which a session would experience as a hang rather
    /// than as an error.
    fn dialled(address: &str) -> String {
        let Ok(address) = address.parse::<SocketAddr>() else {
            return String::from("not attempted: there was no such address to dial");
        };

        match TcpStream::connect_timeout(&address, PATIENCE) {
            Ok(_) => format!("connected to {address}"),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// A host out on the internet, looked up and then dialled.
    ///
    /// **The name first, which is the half that fails.** A container refused
    /// the network does not meet a refusal at the socket — it meets a hostname
    /// that will not resolve, because the resolver is reached over the network
    /// it has not got. That is what a session sees, and so it is what is
    /// reported here: the lookup and the connection said apart.
    fn reached(host: &str) -> String {
        use std::net::ToSocketAddrs;

        let mut found = match host.to_socket_addrs() {
            Ok(found) => found,
            Err(error) => return format!("the name would not resolve: {error}"),
        };

        let Some(address) = found.next() else {
            return String::from("the name resolved to nothing at all");
        };

        match TcpStream::connect_timeout(&address, PATIENCE) {
            Ok(_) => format!("connected to {host} at {address}"),
            Err(error) => format!("resolved {address}, then refused: {error}"),
        }
    }

    /// A named pipe opened as a client, which is what `verkstead ask` does.
    fn opened(name: &str) -> Result<(), std::io::Error> {
        if name.is_empty() {
            return Err(std::io::Error::other("there was no pipe to open"));
        }

        let handle = unsafe {
            CreateFileW(
                wide(OsStr::new(name)).as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        unsafe { CloseHandle(handle) };

        Ok(())
    }

    /// The directories and files the inside half reaches for, really made.
    fn lay_out(playground: &Path) -> std::io::Result<()> {
        for under in ["granted-rw", "granted-rw/denied", "ungranted", "target"] {
            std::fs::create_dir_all(playground.join(under))?;
        }

        for under in ["granted-rw/denied", "ungranted", "target"] {
            std::fs::write(playground.join(under).join("readable"), b"outside\n")?;
        }

        // The junction, made by the machine's own tool rather than by hand: a
        // reparse point needs an ioctl and a buffer, and what is being asked
        // here is about the grant rather than about the making.
        let mut mklink = std::process::Command::new(std::env::var("ComSpec").unwrap_or_default());
        mklink
            .args(["/d", "/c", "mklink", "/J"])
            .arg(playground.join("link"))
            .arg(playground.join("target"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let _ = mklink.status();

        Ok(())
    }

    /// What each tool is asked, which is whatever that tool answers.
    ///
    /// `--version` for the three that take one, and a command that does nothing
    /// for the two shells: Windows PowerShell has no `--version` and would take
    /// it for a script to run, so a machine where it works perfectly would
    /// report it as having failed.
    fn asks(tool: &str) -> Vec<OsString> {
        match tool {
            "pwsh" | "powershell" => ["-NoProfile", "-NonInteractive", "-Command", "exit 0"]
                .iter()
                .map(OsString::from)
                .collect(),
            "bash" => ["-c", "exit 0"].iter().map(OsString::from).collect(),
            _ => vec![OsString::from("--version")],
        }
    }

    /// How a process that did not exit 0 ended, and what it printed on the way.
    ///
    /// The printing is half the answer where the exit code is a number nobody
    /// reads: `0xc0000142` is a library refusing to start and says nothing about
    /// which, and what the program said on its way down is what names it.
    fn said_by(outcome: &Outcome) -> String {
        let said = outcome.output.replace(char::is_control, " ");
        let said = said.trim();

        if said.is_empty() {
            format!("exited {}, saying nothing", ended(outcome.exit))
        } else {
            format!("exited {}, saying: {said}", ended(outcome.exit))
        }
    }

    /// Whether a batch file runs inside, which is how an npm-installed agent
    /// starts — see `sandbox::open`, whose `cmd /d /c call` this is.
    ///
    /// The file is the probe's own and in a directory granted read-write, so
    /// what this asks about is the shell and the container rather than whether
    /// some tool's own installer left a script that works.
    fn batch(profile: &Profile, playground: &Path) -> String {
        let Some(shell) = on_the_path("cmd") else {
            return String::from("there is no cmd.exe on this machine's PATH");
        };

        let script = playground.join("granted-rw").join("probe.cmd");

        if let Err(error) = std::fs::write(&script, "@echo off\r\necho batch-ran-ok\r\n") {
            return format!("the batch file would not be written: {error}");
        }

        let asked = [
            OsString::from("/d"),
            OsString::from("/c"),
            OsString::from("call"),
            script.into_os_string(),
        ];

        match started(profile, shell.as_os_str(), &asked, None, true) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("ran"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// What node makes of a path inside a container, which is what an agent
    /// written in it makes of one.
    ///
    /// **This is the question a session failed on**, and it is not the question
    /// every line above it asks. Those ask whether a path can be *opened*; this
    /// asks whether it can be *resolved* — `fs.realpathSync`, which walks a path
    /// from the volume root down, and `fs.realpathSync.native`, which asks the
    /// operating system for the final name of an open handle. An agent that
    /// checks a path is what it was when permission was given calls one of them
    /// before every read, so a container where they cannot answer is a container
    /// where the agent refuses its own files however well they are granted.
    ///
    /// Asked with `-e` rather than a script named by path, because resolving a
    /// main module is itself a `realpath`: a script would never get as far as
    /// running.
    fn resolving(profile: &Profile, playground: &Path) -> String {
        let Some(node) = on_the_path("node") else {
            return String::from("node is not on this machine's PATH, so this went unasked");
        };

        // One of each: a directory granted read-write, the machine's own
        // system directory — which every container reads with no entry at all —
        // and the directory above every human's profile, which none is granted.
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

        match started(profile, node.as_os_str(), &asked, None, true) {
            Ok(outcome) => outcome
                .output
                .replace(char::is_control, " ")
                .trim()
                .to_owned(),
            Err(error) => format!("refused: {error}"),
        }
    }

    /// One tool, asked twice: as the machine keeps it, and with its own
    /// directory granted.
    fn ran_tool(profile: &Profile, tool: &str, written: &mut Written) -> String {
        let Some(path) = on_the_path(tool) else {
            return String::from("not on this machine's PATH, so this went unasked");
        };

        let held = path.parent().unwrap_or(Path::new(".")).to_owned();
        let under_profile = home().is_some_and(|home| held.starts_with(home));

        let asked = asks(tool);

        let ungranted = match started(profile, path.as_os_str(), &asked, None, true) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("ran"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("refused: {error}"),
        };

        written.grant(&held, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE);

        let granted = match started(profile, path.as_os_str(), &asked, None, true) {
            Ok(outcome) if outcome.exit == Some(0) => String::from("ran"),
            Ok(outcome) => said_by(&outcome),
            Err(error) => format!("refused: {error}"),
        };

        format!(
            "{} | under the profile: {} | ungranted: {ungranted} | granted: {granted}",
            path.display(),
            if under_profile { "yes" } else { "no" },
        )
    }

    /// Whether an sccache client inside reaches a server outside — which is the
    /// whole of whether the shared Rust build cache survives the boundary.
    ///
    /// Asked of the real client rather than of the port it dials, because
    /// sccache is free to have changed its mind about how a client finds a
    /// server, and what the answer has to be about is sccache rather than about
    /// a number this program guessed.
    fn sccache(profile: &Profile, written: &mut Written) -> String {
        let Some(path) = on_the_path("sccache") else {
            return String::from(
                "not installed, so this went unasked — install sccache and run again if the \
                 shared build cache matters",
            );
        };

        // A server of its own, on a port of its own, so that whatever answers
        // inside is this one rather than something the machine already had.
        unsafe {
            std::env::set_var("SCCACHE_SERVER_PORT", "4227");
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

        written.grant(
            path.parent().unwrap_or(Path::new(".")),
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        );

        let asked = started(
            profile,
            path.as_os_str(),
            &[OsString::from("--show-stats")],
            None,
            true,
        );

        let said = match asked {
            Ok(outcome) => format!(
                "exited {:?}, saying: {}",
                outcome.exit,
                outcome
                    .output
                    .lines()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
            Err(error) => format!("would not start: {error}"),
        };

        let mut stop = std::process::Command::new(&path);
        let _ = stop
            .arg("--stop-server")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        format!("{} | client inside: {said}", path.display())
    }

    /// Whether a pseudoconsole opened out here survives being handed to a
    /// process inside the container.
    ///
    /// The one question in this program that the stage cannot be built without:
    /// a session *is* a program on a console, so a console that does not reach
    /// inside is a stage with nothing to render.
    fn conpty(profile: &Profile) -> String {
        let Ok((reading, printing)) = anonymous() else {
            return String::from("not attempted: a pipe could not be made");
        };

        let Ok((typing, writing)) = anonymous() else {
            return String::from("not attempted: a second pipe could not be made");
        };

        let mut console = 0;

        let opened = unsafe {
            CreatePseudoConsole(
                COORD { X: 100, Y: 30 },
                typing.0,
                printing.0,
                0,
                &mut console,
            )
        };

        if opened < 0 {
            return format!("no console: CreatePseudoConsole said {opened:#010x}");
        }

        // The console holds its own copies of the two ends it was given, and
        // ours are what would keep the read below waiting forever.
        drop(printing);
        drop(typing);

        // **`writing` is not dropped with them**, and the first run of this
        // probe is why. It is this end of the console's *input*, and a console
        // whose input has no writer left is one at end of file from the moment
        // it opens: the shell inside read end-of-input before it had printed
        // anything, and died on `STATUS_CONTROL_C_EXIT` — which read as a
        // console that does not survive the boundary and was nothing of the
        // sort. So it is held until the process has gone, exactly as a real
        // terminal holds it for as long as the session runs.
        let marker = format!("conpty-{}", std::process::id());
        let comspec = std::env::var("ComSpec").unwrap_or_else(|_| String::from("cmd.exe"));

        let started = started(
            profile,
            OsStr::new(&comspec),
            &[
                OsString::from("/d"),
                OsString::from("/c"),
                OsString::from(format!("echo {marker}")),
            ],
            Some(console),
            false,
        );

        // And now the process has gone, so the input this was holding open for
        // it can go too — and then the console, whose copy of the printing end
        // is what stands between a drained pipe and an end of file.
        drop(writing);
        unsafe { ClosePseudoConsole(console) };

        let said = drained(&reading);
        drop(reading);

        match started {
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

    /// How a process ended, in the spelling a Windows status code is read in.
    ///
    /// Decimal is what a status code is least legible as: `3221225786` says
    /// nothing and `0xC000013A` is one search away from being
    /// `STATUS_CONTROL_C_EXIT`. The two that this probe has actually seen are
    /// named outright, because they are the two a reader will meet.
    fn ended(exit: Option<u32>) -> String {
        match exit {
            None => String::from("nothing — it was still running when the wait ran out"),
            Some(0) => String::from("0"),
            Some(0xC000_013A) => String::from("0xC000013A (STATUS_CONTROL_C_EXIT)"),
            Some(0xC000_0022) => String::from("0xC0000022 (STATUS_ACCESS_DENIED)"),
            Some(code) => format!("{code:#010x}"),
        }
    }

    /// One process, started inside `profile`'s container.
    ///
    /// The whole of the mechanism the stage turns on: the security
    /// capabilities go on the same attribute list the pseudoconsole goes on,
    /// which is why both are here rather than in two functions — a list is one
    /// block of memory sized for the number of attributes it will hold.
    fn started(
        profile: &Profile,
        program: &OsStr,
        argv: &[OsString],
        console: Option<isize>,
        capture: bool,
    ) -> Result<Outcome, String> {
        let capabilities = profile.capabilities();

        let mut attributes = Attributes::of(1 + usize::from(console.is_some()))
            .map_err(|error| format!("no attribute list: {error}"))?;

        attributes
            .carrying(
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                ptr::from_ref(&capabilities).cast(),
                size_of::<SECURITY_CAPABILITIES>(),
            )
            .map_err(|error| format!("the capabilities would not go on the list: {error}"))?;

        if let Some(console) = console {
            attributes
                .carrying(
                    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                    console as *const std::ffi::c_void,
                    size_of::<isize>(),
                )
                .map_err(|error| format!("the console would not go on the list: {error}"))?;
        }

        // Where what it prints goes, when anybody is reading: a pipe whose
        // write end the child inherits, and whose read end this keeps.
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
        startup.lpAttributeList = attributes.list();
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;

        if let Some((_, writing)) = &piped {
            startup.StartupInfo.hStdOutput = writing.0;
            startup.StartupInfo.hStdError = writing.0;
        }

        let mut line = command_line(program, argv);
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let created = unsafe {
            CreateProcessW(
                ptr::null(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                i32::from(piped.is_some()),
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED,
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
        let thread = Held(information.hThread);

        unsafe { ResumeThread(thread.0) };
        drop(thread);

        // This end's copy of the write end goes, or the read below would never
        // see an end of file: the child is not the only one holding it until it
        // is.
        let reading = piped.map(|(reading, writing)| {
            drop(writing);
            reading
        });

        let output = reading.as_ref().map(drained).unwrap_or_default();

        let waited = unsafe { WaitForSingleObject(process.0, PATIENCE.as_millis() as u32) };

        if waited == WAIT_TIMEOUT {
            unsafe { TerminateProcess(process.0, 1) };

            return Ok(Outcome { exit: None, output });
        }

        let mut code = 0u32;
        let asked = unsafe { GetExitCodeProcess(process.0, &mut code) };

        Ok(Outcome {
            exit: (asked != 0).then_some(code),
            output,
        })
    }

    /// How a process this started ended, and what it printed on the way.
    struct Outcome {
        exit: Option<u32>,
        output: String,
    }

    /// Everything read off `handle` until there is no more of it.
    fn drained(handle: &Held) -> String {
        let mut said = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
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
                // The ordinary end of a pipe whose writer has gone, and the one
                // thing here that is not a failure.
                let _ = std::io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_BROKEN_PIPE as i32);
                break;
            }

            if read == 0 {
                break;
            }

            said.extend_from_slice(&buffer[..read as usize]);
        }

        String::from_utf8_lossy(&said).into_owned()
    }

    /// A pipe with no name, for reading what a child printed.
    fn anonymous() -> std::io::Result<(Held, Held)> {
        let mut reading: HANDLE = ptr::null_mut();
        let mut writing: HANDLE = ptr::null_mut();

        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: ptr::null_mut(),
            bInheritHandle: 1,
        };

        let made = unsafe { CreatePipe(&mut reading, &mut writing, &attributes, 0) };

        if made == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // The read end is this process's alone: a child that inherited it would
        // be one more thing keeping the pipe from ever reaching an end of file.
        unsafe { SetHandleInformation(reading, HANDLE_FLAG_INHERIT, 0) };

        Ok((Held(reading), Held(writing)))
    }

    /// A named pipe created granting the container, which is what a session
    /// inside one would ask Verkstead through.
    ///
    /// Both halves are held rather than read: the handle keeps an instance of
    /// the pipe standing for as long as the inside half might dial it, and the
    /// descriptor is the block it was created from, which this process frees.
    #[allow(dead_code)]
    struct Pipe(Held, Descriptor);

    impl Pipe {
        /// One instance, under `name`, granting the account this runs as
        /// everything and `container` what a client needs — the descriptor
        /// `crates/server/src/pipe.rs` writes, said again here so that what is
        /// probed is the shape that will really be used.
        fn granting(name: &str, container: &str) -> Result<Pipe, String> {
            let descriptor = Descriptor::of(&format!(
                "D:P(A;;GA;;;{})(A;;GRGW;;;{container})",
                the_account_here().map_err(|error| error.to_string())?
            ))
            .map_err(|error| error.to_string())?;

            let mut attributes = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: 0,
            };

            let handle = unsafe {
                CreateNamedPipeW(
                    wide(OsStr::new(name)).as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    1,
                    4096,
                    4096,
                    0,
                    ptr::from_mut(&mut attributes),
                )
            };

            if handle == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error().to_string());
            }

            Ok(Pipe(Held(handle), descriptor))
        }
    }

    /// The throwaway AppContainer this whole program is about.
    struct Profile {
        sid: PSID,
        sid_text: String,
        internet: PSID,

        /// The capability list every process started in here is built with,
        /// boxed so that a pointer to it stays good however this is held.
        ///
        /// **Registering the profile with a capability is not granting it.**
        /// What `CreateAppContainerProfile` is handed says what the container
        /// may be given; what `CreateProcessW` is handed is what the token
        /// really carries. Passing the first and not the second — which is what
        /// this probe did until it was asked why a session could reach nothing
        /// — asks the machine what a container holding no capability at all can
        /// reach, and the answer to that is nothing, whatever the machine would
        /// otherwise have allowed.
        capability: Box<SID_AND_ATTRIBUTES>,
    }

    impl Profile {
        /// One made, with the internet-client capability and nothing else.
        fn created(name: &str) -> Result<Profile, String> {
            let mut internet: PSID = ptr::null_mut();

            let read = unsafe {
                ConvertStringSidToSidW(wide(OsStr::new(INTERNET_CLIENT)).as_ptr(), &mut internet)
            };

            if read == 0 {
                return Err(format!(
                    "the internet-client capability would not resolve: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let capability = Box::new(SID_AND_ATTRIBUTES {
                Sid: internet,
                Attributes: SE_GROUP_ENABLED,
            });

            let name_w = wide(OsStr::new(name));
            let mut sid: PSID = ptr::null_mut();

            let made = unsafe {
                CreateAppContainerProfile(
                    name_w.as_ptr(),
                    name_w.as_ptr(),
                    name_w.as_ptr(),
                    capability.as_ref(),
                    1,
                    &mut sid,
                )
            };

            if made < 0 {
                return Err(format!(
                    "CreateAppContainerProfile said {made:#010x} ({})",
                    std::io::Error::from_raw_os_error(made & 0xffff)
                ));
            }

            let sid_text = text_of(sid).unwrap_or_else(|| String::from("(unreadable)"));

            Ok(Profile {
                sid,
                sid_text,
                internet,
                capability,
            })
        }

        /// What `CreateProcessW` is handed to put a process inside it.
        fn capabilities(&self) -> SECURITY_CAPABILITIES {
            // Held on the stack of the caller for as long as the attribute list
            // is, which is what the one call that reads it needs.
            SECURITY_CAPABILITIES {
                AppContainerSid: self.sid,
                Capabilities: ptr::from_ref(self.capability.as_ref()).cast_mut(),
                CapabilityCount: 1,
                Reserved: 0,
            }
        }
    }

    impl Drop for Profile {
        fn drop(&mut self) {
            if !self.internet.is_null() {
                unsafe { FreeSid(self.internet) };
            }

            if !self.sid.is_null() {
                unsafe { FreeSid(self.sid) };
            }
        }
    }

    /// Every access-control entry this program wrote, so that every one of them
    /// can be taken back.
    ///
    /// Which matters more here than anywhere: these are entries on the human's
    /// own directories — the one this program is in, and each tool's — and a
    /// probe that left them behind would have changed the machine it was only
    /// supposed to ask about.
    struct Written {
        sid: PSID,
        paths: Vec<PathBuf>,

        /// And the ones that would not be written, which is a fact about this
        /// machine rather than a failure to report at the end: a per-user
        /// process cannot write the access-control list of a directory it does
        /// not own, and Program Files is the case that matters.
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

        /// `path` reachable by the container at `rights`, inherited by
        /// everything under it — which is what a rendering granting a directory
        /// means by granting it.
        fn grant(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, SET_ACCESS, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
        }

        /// And `path` alone reachable, with nothing under it made reachable by
        /// its being so.
        ///
        /// Which is what a directory a session only has to *walk through* gets,
        /// and the difference matters more than it looks: an inheriting grant
        /// on a directory grants everything beneath it, so a playground laid
        /// out under one would have no ungranted path left in it to ask about.
        fn grant_here(&mut self, path: &Path, rights: u32) {
            self.wrote(path, rights, SET_ACCESS, NO_INHERITANCE);
        }

        /// And `path` refused, whatever a grant above it says.
        fn deny(&mut self, path: &Path) {
            self.wrote(
                path,
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                DENY_ACCESS,
                SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            );
        }

        /// One entry written down, or why it could not be.
        fn wrote(&mut self, path: &Path, rights: u32, mode: i32, inheritance: u32) {
            match self.entry(path, rights, mode, inheritance) {
                Ok(()) => self.paths.push(path.to_owned()),
                Err(error) => self.refused.push(format!("{} ({error})", path.display())),
            }
        }

        /// Everything that would not be written, for the line that says so.
        fn refusals(&self) -> String {
            if self.refused.is_empty() {
                return String::from("every entry this probe asked for was written");
            }

            format!("could not be written: {}", self.refused.join("; "))
        }

        /// Everything written, unwritten. Hands back how many paths it cleared.
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

        /// One entry for the container, added to whatever `path`'s access
        /// control list already says.
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

    /// A security descriptor read out of SDDL, freed when it is dropped.
    struct Descriptor(PSECURITY_DESCRIPTOR);

    impl Descriptor {
        fn of(sddl: &str) -> std::io::Result<Descriptor> {
            let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

            let read = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    wide(OsStr::new(sddl)).as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            };

            if read == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Descriptor(descriptor))
        }
    }

    impl Drop for Descriptor {
        fn drop(&mut self) {
            unsafe { LocalFree(self.0) };
        }
    }

    /// The attribute list a process is started with, sized for as many
    /// attributes as it is going to be given.
    struct Attributes(Vec<usize>);

    impl Attributes {
        fn of(count: usize) -> std::io::Result<Attributes> {
            let mut wanted = 0usize;

            unsafe {
                InitializeProcThreadAttributeList(ptr::null_mut(), count as u32, 0, &mut wanted)
            };

            if wanted == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut buffer = vec![0usize; wanted.div_ceil(size_of::<usize>())];

            let made = unsafe {
                InitializeProcThreadAttributeList(
                    buffer.as_mut_ptr().cast(),
                    count as u32,
                    0,
                    &mut wanted,
                )
            };

            if made == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Attributes(buffer))
        }

        fn carrying(
            &mut self,
            attribute: usize,
            value: *const std::ffi::c_void,
            size: usize,
        ) -> std::io::Result<()> {
            let list = self.list();

            let carried = unsafe {
                windows_sys::Win32::System::Threading::UpdateProcThreadAttribute(
                    list,
                    0,
                    attribute,
                    value,
                    size,
                    ptr::null_mut(),
                    ptr::null(),
                )
            };

            if carried == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(())
        }

        fn list(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
            self.0.as_mut_ptr().cast()
        }
    }

    impl Drop for Attributes {
        fn drop(&mut self) {
            unsafe { DeleteProcThreadAttributeList(self.list()) };
        }
    }

    /// One handle, closed when it goes.
    struct Held(HANDLE);

    impl Drop for Held {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// The account this program is running as, as a descriptor names one.
    fn the_account_here() -> std::io::Result<String> {
        let mut token: HANDLE = ptr::null_mut();

        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };

        if opened == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let token = Held(token);
        let mut wanted = 0u32;

        unsafe { GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut wanted) };

        let mut buffer = vec![0u8; wanted as usize];

        let read = unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                wanted,
                &mut wanted,
            )
        };

        if read == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let user = buffer.as_ptr().cast::<TOKEN_USER>();

        text_of(unsafe { (*user).User.Sid })
            .ok_or_else(|| std::io::Error::other("the account's SID would not be written down"))
    }

    /// A SID as a person reads one.
    fn text_of(sid: PSID) -> Option<String> {
        let mut written: *mut u16 = ptr::null_mut();

        let ok = unsafe { ConvertSidToStringSidW(sid, &mut written) };

        if ok == 0 || written.is_null() {
            return None;
        }

        let mut length = 0;
        while unsafe { *written.add(length) } != 0 {
            length += 1;
        }

        let text = OsString::from_wide(unsafe { std::slice::from_raw_parts(written, length) })
            .to_string_lossy()
            .into_owned();

        unsafe { LocalFree(written.cast()) };

        Some(text)
    }

    /// This machine's own address, as something on the network would dial it.
    ///
    /// Read off a socket rather than out of the interface table: connecting a
    /// datagram socket sends nothing and settles which interface would carry
    /// the traffic, which is the address the ADR wonders about.
    fn own_address() -> Option<String> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:53").ok()?;

        Some(socket.local_addr().ok()?.ip().to_string())
    }

    /// Where a program is on this machine's `PATH`, resolved the way the
    /// machine resolves one.
    fn on_the_path(program: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        let extensions =
            std::env::var("PATHEXT").unwrap_or_else(|_| String::from(".EXE;.CMD;.BAT"));

        for directory in std::env::split_paths(&path) {
            for extension in extensions.split(';').filter(|piece| !piece.is_empty()) {
                let candidate = directory.join(format!("{program}{extension}"));

                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// The human's own profile directory, which is what says whether a tool is
    /// a per-user install.
    fn home() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }

    /// Which machine this is, for the top of the report.
    fn machine() -> String {
        format!(
            "{} on {}",
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| String::from("(unnamed)")),
            std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| String::from("(unknown)")),
        )
    }

    /// `program` and `argv` as the one string `CreateProcessW` takes.
    fn command_line(program: &OsStr, argv: &[OsString]) -> Vec<u16> {
        let mut line: Vec<u16> = Vec::new();

        quoted(program, &mut line);

        for word in argv {
            line.push(u16::from(b' '));
            quoted(word, &mut line);
        }

        line.push(0);

        line
    }

    /// One word of it, written so that it is read back as the word it was.
    fn quoted(word: &OsStr, line: &mut Vec<u16>) {
        const QUOTE: u16 = b'"' as u16;
        const BACKSLASH: u16 = b'\\' as u16;

        let word: Vec<u16> = word.encode_wide().collect();

        if !word.is_empty() && !word.iter().any(|unit| matches!(*unit, 0x20 | 0x09 | QUOTE)) {
            line.extend_from_slice(&word);
            return;
        }

        line.push(QUOTE);

        let mut slashes = 0;

        for unit in word {
            match unit {
                BACKSLASH => slashes += 1,
                QUOTE => {
                    for _ in 0..=slashes {
                        line.push(BACKSLASH);
                    }
                    slashes = 0;
                }
                _ => slashes = 0,
            }

            line.push(unit);
        }

        for _ in 0..slashes {
            line.push(BACKSLASH);
        }

        line.push(QUOTE);
    }

    /// A string as Win32 takes one.
    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// One line of the report, on stdout and flushed, so that a run that dies
    /// part way through still shows how far it got.
    fn say(line: &str) {
        println!("{line}");
        let _ = std::io::stdout().flush();
    }
}
