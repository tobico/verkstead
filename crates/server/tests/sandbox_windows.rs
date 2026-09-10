//! What a session on Windows can reach, asked by running a probe behind a
//! Conversation's own boundary — started as the local account of Verkstead's
//! own that every session on this platform runs as.

//!
//! The third arm of `tests/sandbox.rs` and `tests/sandbox_macos.rs`, settled the
//! same way both of those are: **nothing here reads what the rendering wrote.**
//! The access-control entries *are* what is being tested, and a test that read
//! one back would be asserting itself — it would go on passing while Windows
//! changed what an entry meant, or while an inherited allow quietly outranked a
//! deny somewhere above it. What settles whether the rest of the machine is
//! still reachable is a program behind the boundary attempting each access and
//! saying what the operating system said.

//!
//! **The vocabulary is the Mac's** — `write`, `read`, `refused`, `absent` — and
//! for the Mac's reason (ADR-0012). A path a session may not reach on Linux is
//! `absent`, because the mount namespace it is in never had one; the same path
//! here is `refused`, really there and every open of it denied. So a probe that
//! finds `absent` on this platform has found something odd, and one that finds
//! `refused` has found the boundary working. The one place `absent` is right is
//! a name nobody ever made, and this suite asks for one on purpose: the two look
//! identical to anything coarser, and only one of them is a boundary.
//!
//! **Two of the vocabulary's kinds name no path here, so there is nothing to
//! attempt.** `ProcessTable` is a directory on the platform that keeps one in
//! the filesystem and nothing on this one, and `Devices` is the machine's own —
//! neither comes to an access-control entry at all, which is a claim about what
//! a description comes to rather than about what a session can open, and is held
//! where the entries are worked out (see the server's `sandbox::granting`).

//! Every kind that does name a path is attempted below.
//!
//! **What the probe says, it says on one short line.** A console is a grid and
//! what comes off a pseudoconsole is a drawing of one, so a path printed on it
//! is a path with a line break in the middle — which is why
//! `tests/sessions_windows.rs` reads its evidence off files. Nothing here is on
//! a console at all: [`off_a_console`] hands the probe three pipes, so what it
//! prints arrives as it was written. The lines are `name=word` all the same,
//! because a word is the whole of what a classification is and the paths are
//! this file's already.
//!
//! **What this costs the `windows-2025` job.** Nothing here waits on anything:
//! [`off_a_console`] blocks until the probe has exited, so there is no deadline
//! to run out of and no `PATIENCE` to raise — unlike
//! `tests/sessions_windows.rs`, whose every assertion is a poll. What the suite
//! costs the job is therefore wall time and nothing else, and each test's is one
//! logon as the session account, the entries of one description written and
//! taken off, and one Windows PowerShell started as it. Measured on the job:
//! *(unmeasured — the first `windows-2025` run of this branch is what fills this
//! in; see the pull request the stage finishes with)*.
//!
//! **And it needs the session account**, which is an administrator's call to
//! make and so not one this suite can make for itself: a machine where
//! `verkstead session-account create` has never been run fails here with the
//! line that names it, rather than passing quietly.
//!
//! Windows only, which is where "everywhere" stops for this one: what it starts
//! is a logon as another account, and the two Unixes have no such call to make.

#![cfg(windows)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::attachments::Attachments;
use verkstead_server::boundaries;
use verkstead_server::build_cache::BuildCache;

use verkstead_server::handoffs::Handoffs;
use verkstead_server::platform;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::account::Logon;
use verkstead_server::sandbox::account::machine::Account;
use verkstead_server::sandbox::entries;

use verkstead_server::sandbox::{Bind, Executable, Homes, Reachable, Sandbox, off_a_console};

use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::store;
use verkstead_server::store::Lifecycle;

/// Where the server this Conversation belongs to would be listening, which is
/// what a session inside is told to put its Question Sets to.
///
/// Nothing listens on it and nothing here asks: what a Windows session really
/// asks through is the named pipe, and `crates/cli/tests/sandbox_windows.rs` is
/// where that is proved end to end.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// The shell every Windows machine has, which is what the probe is written in.
///
/// Named without a path and without an extension, because resolving it is part
/// of what is being asked: a session's `PATH` is the description's, and a probe
/// that named an image outright would have proved the boundary without ever
/// proving that a session can find a program at all.
const POWERSHELL: &str = "powershell";

/// And how it is started: the script is a file rather than a command line, for
/// the reason `tests/sessions_windows.rs` starts its stand-in that way —
/// `-File` hands what follows to the script rather than appending it, and
/// nothing between here and there re-reads the quoting.
///
/// `-ExecutionPolicy Bypass` because the script is one this test wrote a moment
/// ago, which is not a thing a machine's signing policy has anything useful to
/// say about.
const HOW: [&str; 4] = [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
];

/// What the probe is called inside the Worktree it is written into.
///
/// The Worktree because it is the one directory every description grants and
/// every session starts in — a probe written anywhere else would need a reach
/// of its own described before it could ask what the description reaches.
const PROBE: &str = "verkstead-boundary-probe.ps1";

/// What each fixture leaves in a directory for the probe to read, so that a
/// `read` is a file really opened rather than an empty directory listed.
const MARKER: &str = "what-is-in-here.txt";

/// And what is in it, which nothing asserts: what is being asked is whether the
/// bytes could be reached at all.
const SAID: &str = "the description named this directory\n";

/// The account's own skills, which a description names in order to say a session
/// finds nothing at them.
const THEIR_SKILL: &str = "# what the account would have been grilled by\n";

/// And what stands in for the server's own image: a file that is a file, which
/// is the whole of what [`Executable::at`] asks of one.
///
/// Never run — what a session does with the binary it asks with is
/// `crates/cli/tests/sandbox_windows.rs`'s to prove, that being the crate with
/// a real one to hand. What it is here is a path the description grants
/// read-only, and the one file in a directory it grants nothing else of.
const SAYS_WHICH_BUILD: &str = "the server's own image would be here\n";

/// The probe itself: what a classification is, in this platform's own words.
///
/// **Attempted rather than asked of the metadata.** A read-only grant and a
/// directory the account simply has no write permission on are the same answer
/// to anything that only looks; only one of them is the surface being
/// described. So a directory is classified by creating a file in it, and then —
/// where that was refused — by listing it; and a file by opening it for writing,
/// and then by reading it.
///
/// **And absence is told from refusal by the exception's own name**, which is
/// the one thing this platform makes easy and the one thing the whole suite
/// turns on. `UnauthorizedAccessException` is the boundary — what .NET makes of
/// the `ERROR_ACCESS_DENIED` a refused open comes back with — and a
/// `FileNotFoundException` or a `DirectoryNotFoundException` is a name nobody
/// ever made.
///
/// **Anything else is reported as itself, with the machine's own number after
/// it.** A word this file does not know about is a failure worth reading rather
/// than one worth guessing at, and the `HResult` behind it is the Win32 error
/// the platform really gave — which is what tells a boundary this suite has
/// misread from an open that failed for some reason of its own. `.NET` on a
/// Unix, for one, reports a denied open as a plain `IOException`; nothing here
/// runs there, and a reader who ends up with one should be told which it was.
///
/// The innermost exception is the one asked, because a .NET method called from
/// PowerShell arrives wrapped.
///
/// **And not one cmdlet in the whole of it**, which is a rule rather than a
/// style. Windows PowerShell starts behind the boundary and parses and runs what
/// it is given, and the commands it would ordinarily import from a module at
/// startup are not there: the `windows-2025` job answered
/// `CommandNotFoundException` for `Write-Output` the first time a probe of this
/// shape ran inside one. So this is the language and the framework and nothing
/// else — `[System.IO.Path]::Combine` rather than `Join-Path`, a `[void]` cast
/// rather than `Out-Null`, `[Console]::Out` rather than `Write-Output` — which
/// is what a program has in there whatever the shell managed to load.
const CLASSIFYING: &str = r#"
$ErrorActionPreference = 'Stop'

function Innermost($caught) {
    $why = $caught.Exception
    while ($why.InnerException) { $why = $why.InnerException }
    return $why
}

function Why($caught) { return (Innermost $caught).GetType().Name }

function Unexpected($caught) {
    $why = Innermost $caught
    return ($why.GetType().Name + '/' + ('0x{0:x8}' -f $why.HResult))
}

function Report($name, $said) { [Console]::Out.WriteLine($name + '=' + $said) }

function Directory($path) {
    $probe = [System.IO.Path]::Combine($path, '.verkstead-probe')

    try {
        $handle = [System.IO.File]::Open($probe, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write)
        $handle.Close()
        [System.IO.File]::Delete($probe)
        return 'write'
    } catch {
        $why = Why $_
        if ($why -eq 'DirectoryNotFoundException') { return 'absent' }
        if ($why -ne 'UnauthorizedAccessException') { return (Unexpected $_) }
    }

    try {
        [void][System.IO.Directory]::GetFileSystemEntries($path)
        return 'read'
    } catch {
        $why = Why $_
        if ($why -eq 'DirectoryNotFoundException') { return 'absent' }
        if ($why -eq 'UnauthorizedAccessException') { return 'refused' }
        return (Unexpected $_)
    }
}

function File($path) {
    try {
        $handle = [System.IO.File]::Open($path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Write)
        $handle.Close()
        return 'write'
    } catch {
        $why = Why $_
        if ($why -eq 'FileNotFoundException') { return 'absent' }
        if ($why -eq 'DirectoryNotFoundException') { return 'absent' }
        if ($why -ne 'UnauthorizedAccessException') { return (Unexpected $_) }
    }

    try {
        [void][System.IO.File]::ReadAllBytes($path)
        return 'read'
    } catch {
        $why = Why $_
        if ($why -eq 'FileNotFoundException') { return 'absent' }
        if ($why -eq 'DirectoryNotFoundException') { return 'absent' }
        if ($why -eq 'UnauthorizedAccessException') { return 'refused' }
        return (Unexpected $_)
    }
}
"#;

/// One thing the probe is asked about: what to call it, where it is, and which
/// of the two shapes it has.
///
/// The shape is the caller's rather than something the probe works out, and
/// deliberately: asking the filesystem what is at a path is exactly the question
/// a refused path will not answer, so a probe that decided for itself would
/// classify every refusal as whichever shape it guessed.
struct Asked {
    name: &'static str,
    path: PathBuf,
    directory: bool,
}

/// A directory the probe is to classify.
fn directory(name: &'static str, path: impl Into<PathBuf>) -> Asked {
    Asked {
        name,
        path: path.into(),
        directory: true,
    }
}

/// And a file.
fn file(name: &'static str, path: impl Into<PathBuf>) -> Asked {
    Asked {
        name,
        path: path.into(),
        directory: false,
    }
}

/// A Conversation part-way through its first grilling, with one of everything a
/// description can name laid out on a real filesystem.
///
/// Everything is real, for the reason the other two suites' fixtures are: what
/// the description says is read off these paths, and a fixture that hand-built
/// them would prove the probe works rather than that the boundary does.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a Worktree that vanished mid-probe would fail obscurely.
    _watched: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    /// Where the Repo is, and the checkout beside it that no session has any
    /// business seeing.
    repo: PathBuf,
    sibling: PathBuf,

    /// Where the Profile's account is on the host, which is the far end of the
    /// junction a session finds it through.
    account: PathBuf,

    conversation: store::Conversation,
    profile: store::Profile,

    /// The record itself, kept for the tests about a boundary's lifetime: what
    /// a sweep decides is what the store says about a Conversation, so a
    /// fixture that closed the database behind it could say nothing about
    /// either.
    pool: sqlx::SqlitePool,

    /// And every Conversation of this fixture's that has had entries written
    /// for it, so that they come off the human's directories when the fixture
    /// does — see [`Grilling::drop`], which is the whole of why it is kept.
    made: std::sync::Mutex<Vec<i64>>,

    skills: Skills,
    verkstead: Executable,
    handoffs: Handoffs,
    attachments: Attachments,
    settings: Settings,

    /// The two directories Sandbox Configuration was told to add, which is how
    /// an `Own` at each reach gets into a description without being one of the
    /// things a session always has.
    writable: PathBuf,
    readable: PathBuf,
}

impl Grilling {
    /// The sandbox this Conversation's session runs in.
    fn sandbox(&self) -> Sandbox {
        self.sandbox_of(&self.conversation)
    }

    /// And the same sandbox running as an account said outright, which is what
    /// the two refusals need: a name no machine has, and a name every machine
    /// has that nothing holds a password for.
    ///
    /// The settings are read afresh rather than the fixture's own, because the
    /// second of those is asked by emptying the secrets file first.
    fn sandbox_running_as(&self, account: &str) -> Sandbox {
        self.built(
            &self.conversation,
            self.homes().running_sessions_as(account),
            &Settings::in_data_dir(self.state.path()),
        )
    }

    /// And the one any Conversation of this fixture's runs in, which is what
    /// the second Conversation below needs: everything but the Conversation is
    /// the machine's, and the machine is one machine.
    fn sandbox_of(&self, conversation: &store::Conversation) -> Sandbox {
        self.built(conversation, self.homes(), &self.settings)
    }

    /// One sandbox out of the three things that ever differ between them here:
    /// whose Conversation it is, whose account its sessions run as, and which
    /// settings the password comes out of.
    fn built(
        &self,
        conversation: &store::Conversation,
        homes: Homes,
        settings: &Settings,
    ) -> Sandbox {
        Sandbox::for_conversation(
            conversation,
            &self.profile,
            &homes,
            &Reachable::at(LISTENING),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.attachments,
            &settings.secrets(),
            &settings.config(),
            // Nothing is compiled inside a probe, so there is no cache for one
            // to compile into: the shared build cache is a directory granted
            // read-write like any other, and the two below are what say what
            // that comes to.
            &BuildCache::none(),
            vec![
                Bind::writable(self.writable.clone()),
                Bind::readable(self.readable.clone()),
            ],
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// A second Conversation on the same Repo, grilling in a checkout of its
    /// own — which is the other half of *one set of entries per Conversation*.
    ///
    /// Everything about it is the first one's except the two things that make
    /// it another Conversation: its own row, and its own worktree. Which is the
    /// point — a boundary that kept two Conversations apart by anything else
    /// would be one this fixture could not tell from a boundary that did not.
    async fn beside(&self, branch: &str) -> store::Conversation {
        let id = store::start_conversation(&self.pool, self.conversation.repo.id, branch)
            .await
            .unwrap()
            .expect("the second Conversation starts");

        store::set_grilling_pairing(&self.pool, id, self.profile.id, self.profile.model())
            .await
            .unwrap();

        let worktree = self
            .state
            .path()
            .join("worktrees")
            .join(format!("verkstead-{branch}"));

        let commit = git(&self.repo, &["rev-parse", "HEAD"]).trim().to_owned();

        git(
            &self.repo,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                &worktree.to_string_lossy(),
                &commit,
            ],
        );

        std::fs::write(worktree.join(MARKER), SAID).unwrap();

        store::start_grilling(&self.pool, id, &commit, &worktree, &[])
            .await
            .unwrap();

        self.made.lock().unwrap().push(id);

        store::load_conversation(&self.pool, id)
            .await
            .unwrap()
            .expect("the second Conversation is there")
    }

    /// The homes this server hands out: the fixture's own directory where the
    /// human's profile is, and its state directory to make a session's own
    /// under.
    ///
    /// The two are not the same thing, and half of what this suite asks turns on
    /// that: the first is the human's own, which a session is refused, and a
    /// session's `USERPROFILE` is a directory of Verkstead's made fresh under
    /// the second.
    ///
    /// **And the account said outright**, which is the one thing a temporary
    /// Data Directory cannot name for itself — see [`the_machines_account`].
    fn homes(&self) -> Homes {
        Homes::on(
            Platform::HERE,
            self.home.path().to_owned(),
            self.state.path(),
        )
        .running_sessions_as(the_machines_account().name())
    }

    fn worktree(&self) -> &Path {
        self.conversation
            .worktree
            .as_deref()
            .expect("a grilling Conversation has a worktree")
    }

    /// The Repo's own git directory, which is what a commit inside writes to.
    fn git_dir(&self) -> PathBuf {
        self.repo.join(".git")
    }

    /// The profile a session is given, which is what it reads `USERPROFILE` as.
    fn profile_dir(&self) -> PathBuf {
        self.state
            .path()
            .join("homes")
            .join(self.conversation.id.to_string())
    }

    /// This Conversation's handoff directory as a session finds it: under the
    /// profile, there being no mount on this platform to put it anywhere else.
    fn handoffs_inside(&self) -> PathBuf {
        self.profile_dir().join("verkstead")
    }

    /// And where the files the human attached to it are, which on this platform
    /// is also the path a session reads them at.
    fn attachments_dir(&self) -> PathBuf {
        self.state
            .path()
            .join("attachments")
            .join(self.conversation.id.to_string())
    }

    /// The human's own Documents, which nothing in a description ever names.
    fn documents(&self) -> PathBuf {
        self.home.path().join("Documents")
    }

    /// The local account this machine's Verkstead runs its sessions as, which
    /// is what every entry in this fixture's boundary is written for.
    ///
    /// Read off the machine rather than off the rendering, because that is what
    /// the two lifetime tests need it for: an entry is read back out of
    /// `icacls` under a SID or a name, and neither is a thing a description
    /// carries — see the server's `sandbox::account`.
    fn session_account(&self) -> Account {
        Account::resolving(&the_machines_account()).expect("the account resolved a moment ago")
    }

    /// The account's own skills, on the host: the one path a description
    /// refuses rather than grants.
    ///
    /// The real directory rather than the name a session finds it under. What
    /// the description names is the path inside the profile, which is inside a
    /// junction — and a junction is followed, so the entry lands here, which is
    /// where a reader with `icacls` would go looking for it.
    fn their_skills(&self) -> PathBuf {
        self.account.join(".claude").join("skills")
    }

    /// And the name a session's description says that same directory by, which
    /// is the path inside the profile: the account is junctioned in, so this is
    /// where every entry written for the refusal is written.
    ///
    /// Read beside [`Grilling::their_skills`] where a refusal has to be shown
    /// to have left the directory as it was, because the two are one directory
    /// only for as long as the junction is there — see [`Grilling::trail`].
    fn skills_inside(&self) -> PathBuf {
        self.profile_dir().join(".claude").join("skills")
    }

    /// What the machine and the record say about the one directory a
    /// description refuses, at the moment this is asked.
    ///
    /// **Said in the failure rather than worked out afterwards.** The
    /// `windows-2025` job is the only machine that answers any of this, so a
    /// refusal that did not leave the directory as it found it is a failure
    /// nobody can read a second time: what the list said before, what it said
    /// while the boundary stood, and what was written down about it are the
    /// three things a reading of that failure needs, and none of them survives
    /// the run.
    fn trail(&self, before: &str, standing: &str, written_down: &str) -> String {
        format!(
            "\n  before: {before}\
             \n  while the boundary stood: {standing}\
             \n  now: {}\
             \n  written down: {written_down}",
            self.reading(),
        )
    }

    /// That directory under both the names a description knows it by, read at
    /// once: the real one on the host, and the one inside the profile that
    /// leads to it through the junction.
    ///
    /// The pair rather than either alone, because a refusal is written under
    /// the second name and has to be shown to have left the first as it was —
    /// and the two are one directory only for as long as the junction is there.
    fn reading(&self) -> String {
        format!(
            "on the host {} / inside the profile {}",
            listed(&self.their_skills()),
            listed(&self.skills_inside()),
        )
    }

    /// And what the record under the Data Directory says about this
    /// Conversation's entries, which is what a later server takes the boundary
    /// back from — read while there is still one to read.
    fn written_down(&self) -> String {
        let record = self
            .state
            .path()
            .join("containers")
            .join(self.conversation.id.to_string());

        std::fs::read_to_string(&record)
            .unwrap_or_else(|error| format!("{}: {error}", record.display()))
    }

    /// The directories of the human's and the machine's own that this
    /// fixture's description grants — which is what a boundary's ending has to
    /// leave as it found them.
    ///
    /// Said here rather than in each test, because it is one list and two tests
    /// ask it of two different endings. Every one of them is a real directory
    /// that outlives the session: the Worktree and the git directory behind it,
    /// both halves of the Profile's account, the skills, the image a session
    /// asks with, and the two Sandbox Configuration added. What is deliberately
    /// not in it is the session's own profile under the Data Directory:
    /// Verkstead's own, and nobody's to be left alone on.
    fn granted(&self) -> Vec<PathBuf> {
        vec![
            self.worktree().to_owned(),
            self.git_dir(),
            self.account.join(".claude"),
            self.account.join(".claude.json"),
            self.skills.path().to_owned(),
            self.verkstead.path().to_owned(),
            self.writable.clone(),
            self.readable.clone(),
        ]
    }

    /// Run the probe behind this Conversation's boundary and hand back the
    /// `name=word` lines it printed.
    ///
    /// **The boundary is written here rather than by this file**, which is the
    /// point: [`Sandbox::command`] is what a session start calls, so what is
    /// probed is the boundary a session really gets rather than one this test
    /// assembled beside it. The `Closing` is held for as long as the probe runs
    /// and dropped after it.
    ///
    /// **A probe that did not run says everything it can about why.** The only
    /// machines that enforce this boundary are the `windows-2025` runner and
    /// whoever is reading this on a Windows box, so what a failure here can be
    /// diagnosed from is what this prints: how the process ended, what it said
    /// on either stream, and which program the description resolved.
    fn probe(&self, asked: &[Asked]) -> BTreeMap<String, String> {
        let script = self.worktree().join(PROBE);
        std::fs::write(&script, classifying(asked)).expect("the Worktree to be writable out here");

        let sandbox = self.sandbox();
        let mut argv: Vec<String> = vec![POWERSHELL.to_owned()];
        argv.extend(HOW.iter().map(|word| (*word).to_owned()));
        argv.push("-File".to_owned());
        argv.push(script.display().to_string());

        let (rendering, closing) = sandbox
            .command(&argv)
            .expect("this machine to have the account a session runs as");

        let output = off_a_console(&rendering, b"").expect("the probe to start as it");

        closing.close();

        let printed = String::from_utf8_lossy(&output.stdout).into_owned();

        assert!(
            output.status.success(),
            "the probe did not run behind the boundary.\n\
             how it ended: {}\n\
             what it printed: {printed:?}\n\
             what it complained: {:?}\n\
             the program the description resolved: {:?}\n\
             the account it was started as: {:?}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
            rendering.program(),
            rendering.account().map(|logon| logon.name()),
        );

        printed
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(name, said)| (name.trim().to_owned(), said.trim().to_owned()))
            .collect()
    }
}

impl Drop for Grilling {
    /// Every entry this fixture had written for it, off the machine again.
    ///
    /// **Which a fixture has to say now that a boundary's life is its
    /// Conversation's.** An entry is not a session's to take back — see the
    /// server's `sandbox::entries` — so a test that rendered a description and
    /// walked away would leave one on the runner for every fixture the suite
    /// stood up, each of them on a directory that has since gone. This is that
    /// close and that sweep, said once for the whole suite.
    ///
    /// The tests that take one back themselves are none the worse for it: a
    /// boundary that has already gone is nothing left to take.
    fn drop(&mut self) {
        for conversation in self.made.lock().unwrap().iter() {
            boundaries::remove(self.state.path(), *conversation);
        }
    }
}

/// The whole script the probe runs: how a classification is made, and then one
/// line per thing this test wants classified.
fn classifying(asked: &[Asked]) -> String {
    let mut script = CLASSIFYING.to_owned();

    for one in asked {
        script.push_str(&format!(
            "Report '{}' ({} '{}')\r\n",
            one.name,
            if one.directory { "Directory" } else { "File" },
            one.path.display().to_string().replace('\'', "''"),
        ));
    }

    script
}

/// Stand a Conversation up, with one of everything a description names really
/// on the disk.
async fn grilling() -> Grilling {
    let watched = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // The human's own profile, with something in it that is nobody else's: this
    // is what stands where `C:\Users\<them>` does, and "their Documents are
    // refused" is only a claim if there is something in there to refuse.
    std::fs::create_dir_all(home.path().join("Documents")).unwrap();
    std::fs::write(
        home.path().join("Documents").join(MARKER),
        "the human's own\n",
    )
    .unwrap();

    let repo = repository(watched.path().join("verkstead"));
    let sibling = repository(watched.path().join("something-else"));

    let pool = store::open_database(&state.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo_row = store::register_repo(&pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("the Repo registers");

    // The account a session runs under: the pair of files a Claude Profile is,
    // with skills of the account's own inside the directory half. Really there,
    // so that a probe finding them refused has found the rule covering them
    // rather than an empty name.
    let account = watched.path().join("account");
    let claude_dir = account.join(".claude");
    let config_file = account.join(".claude.json");
    std::fs::create_dir_all(claude_dir.join("skills")).unwrap();
    std::fs::write(claude_dir.join(MARKER), SAID).unwrap();
    std::fs::write(claude_dir.join("skills").join("theirs.md"), THEIR_SKILL).unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: Some("work".to_owned()),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
            models: vec!["claude-opus-5".to_owned()],
        },
    )
    .await
    .unwrap()
    .expect("the Profile saves");

    let id = store::start_conversation(&pool, repo_row.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Conversation starts");

    store::set_grilling_pairing(&pool, id, profile.id, profile.model())
        .await
        .unwrap();

    // The worktree git itself made, where the server puts one.
    let worktree = state.path().join("worktrees/verkstead-rate-limiting");
    std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "rate-limiting",
            &worktree.to_string_lossy(),
            &commit,
        ],
    );

    store::start_grilling(&pool, id, &commit, &worktree, &[])
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    let skills =
        Skills::installed(Platform::HERE, state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let attachments = Attachments::under(state.path());
    let settings = Settings::in_data_dir(state.path());

    // And the account's password written into this Data Directory's own
    // secrets, which is the other half of running as the machine's account —
    // see [`the_machines_account`]. The name travels on the `Homes`, and the
    // password is read out of the file a description was built against.
    settings
        .save_secrets(
            &settings
                .secrets()
                .with_session_account_password(Some(the_machines_account().password().to_owned())),
        )
        .expect("a secrets file to be writable under a temporary Data Directory");

    let settings = Settings::in_data_dir(state.path());

    // A file the human attached, which is what makes the attachments directory
    // one the description names at all.

    let attached = state.path().join("attachments").join(id.to_string());
    std::fs::create_dir_all(&attached).unwrap();
    std::fs::write(attached.join(MARKER), SAID).unwrap();

    // And the handoff directory, made the way a sandbox makes one — asked for
    // here so that the marker below has somewhere to land.
    let handoff_dir = handoffs
        .directory(id)
        .expect("the handoff directory to be one this machine can make");
    std::fs::write(handoff_dir.join(MARKER), SAID).unwrap();

    // The server's own image, in a directory holding one other file: what the
    // description grants is the image, and the file beside it is how this suite
    // asks whether anything else in that directory came with it.
    let image = state.path().join("image");
    std::fs::create_dir_all(&image).unwrap();
    std::fs::write(image.join("verkstead.exe"), SAYS_WHICH_BUILD).unwrap();
    std::fs::write(
        image.join("beside-the-image.txt"),
        "the host put this here\n",
    )
    .unwrap();

    let verkstead = Executable::at(Platform::HERE, image.join("verkstead.exe"), state.path())
        .expect("the image was just written");

    // And the two Sandbox Configuration was told to add, one at each reach.
    let writable = watched.path().join("a-configured-bind");
    let readable = watched.path().join("a-configured-read-only-bind");

    for bind in [&writable, &readable] {
        std::fs::create_dir_all(bind).unwrap();
        std::fs::write(bind.join(MARKER), SAID).unwrap();
    }

    Grilling {
        _watched: watched,
        state,
        home,
        repo,
        sibling,
        account,
        // The Conversation this fixture is, seeded here because a boundary is

        // made for it the moment a description is rendered — see
        // [`Grilling::drop`].
        made: std::sync::Mutex::new(vec![conversation.id]),
        conversation,
        profile,
        pool,
        skills,
        verkstead,
        handoffs,
        attachments,
        settings,
        writable,
        readable,
    }
}

/// The local account this machine's Verkstead runs its sessions as, name and
/// password both.
///
/// **A suite cannot make one and says so rather than passing** — the rule
/// `tests/account_windows.rs` follows, and for its reason: creating a local
/// account is an administrator's call, so a machine where the elevated verb has
/// never been run fails here with the line that names it.
///
/// **Which is why it is the *machine's* Data Directory rather than this
/// fixture's.** An account's name is a fingerprint of the Data Directory it
/// belongs to (see the server's `sandbox::account`), and every fixture here runs
/// against a temporary one — so the account they would be named after is one no
/// elevated verb was ever run for. What they run as instead is the account the
/// human really has, said outright on the `Homes` and with its password written
/// into the fixture's own secrets.
fn the_machines_account() -> Logon {
    let data_dir =
        platform::data_dir(None).expect("this machine has somewhere for a Data Directory");
    let settings = Settings::in_data_dir(&data_dir);

    match Account::on_this_machine(&data_dir, &settings.secrets()) {
        Ok(account) => Logon::of(account.name(), account.password()),
        Err(missing) => panic!(
            "this suite runs sessions as the session account and there is not one: {missing}\n\
             \n\
             The Data Directory it asked about is {}.",
            data_dir.display(),
        ),
    }
}

/// A git repository at `path`, with one commit on `main`.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "local@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Whatever The Repo Says"]);
    std::fs::write(path.join(MARKER), SAID).unwrap();
    git(&path, &["add", MARKER]);
    git(&path, &["commit", "-m", "first"]);

    path
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// What the probe said about `name`, or a failure naming everything it did say.
fn said<'a>(classified: &'a BTreeMap<String, String>, name: &str) -> &'a str {
    classified
        .get(name)
        .map(String::as_str)
        .unwrap_or_else(|| panic!("the probe said nothing about {name}. It said: {classified:?}"))
}

/// Every access kind a description can name, classified by attempting it — and
/// each of them what the description said it would be.
///
/// **One boundary and one probe**, which is not a shortcut: these are one
/// description, and a suite that wrote a boundary per path would be asking one
/// question a dozen times over on the runner this stage has to fit inside.
/// What is asserted is a path at a time all the same, so a failure names the one
/// that moved.
///
/// The kinds, in the order a session's own description says them: the profile it
/// is given and the two halves and the temporary directory inside it, which are
/// made rather than reached; its Worktree and the git directory behind it; the
/// account joined in by junction and the file half by hard link; the handoff
/// directory; the skills, the attached files and the image it asks with,
/// read-only; and the two Sandbox Configuration added, one at each reach.
#[tokio::test]
async fn every_access_kind_is_classified_as_the_description_said() {
    let fixture = grilling().await;
    let profile = fixture.profile_dir();

    let classified = fixture.probe(&[
        directory("profile", &profile),
        directory("roaming", profile.join("AppData").join("Roaming")),
        directory("local", profile.join("AppData").join("Local")),
        directory("temp", profile.join("AppData").join("Local").join("Temp")),
        directory("worktree", fixture.worktree()),
        directory("git", fixture.git_dir()),
        directory("account", profile.join(".claude")),
        file("config", profile.join(".claude.json")),
        directory("handoffs", fixture.handoffs_inside()),
        directory("bind", &fixture.writable),
        directory("skills", fixture.skills.path()),
        directory("attachments", fixture.attachments_dir()),
        directory("readable-bind", &fixture.readable),
        file("verkstead", fixture.verkstead.path()),
    ]);

    for (name, what) in [
        ("profile", "the profile a session is given, made fresh"),
        ("roaming", "the roaming half of it"),
        ("local", "the local half"),
        ("temp", "what a session throws away, inside that half"),
        ("worktree", "the Conversation's own checkout"),
        ("git", "the Repo's git directory behind it"),
        ("account", "the Profile's account, through the junction"),
        ("config", "the file half of it, through the hard link"),
        (
            "handoffs",
            "the Conversation's own directory outside the worktree",
        ),
        ("bind", "what Sandbox Configuration asked for read-write"),
    ] {
        assert_eq!(
            said(&classified, name),
            "write",
            "{what} is read-write in the description, and the probe said: {classified:?}",
        );
    }

    for (name, what) in [
        ("skills", "the skills a session is grilled by"),
        ("attachments", "the files the human attached"),
        (
            "readable-bind",
            "what Sandbox Configuration asked for read-only",
        ),
        ("verkstead", "the image a session asks with"),
    ] {
        assert_eq!(
            said(&classified, name),
            "read",
            "{what} is read-only in the description — reachable and not \
             writable, which is two claims and this is both of them. The probe \
             said: {classified:?}",
        );
    }

    // And from the host, which is the other half of what a `write` on the
    // account means: what a session's profile holds is a *name* for the account
    // rather than a copy of it, so what it writes through that name is written
    // where the Profile said and is still there when the session has gone.
    assert!(
        std::fs::symlink_metadata(profile.join(".claude"))
            .expect("the account is joined into the profile")
            .is_symlink(),
        "what a session finds its account at is the junction the rendering made, \
         a reparse point reading as a link — a copy would have taken the write \
         with it when the profile was emptied",
    );
    assert!(
        fixture.account.join(".claude").join(MARKER).is_file(),
        "and the account itself is untouched, where the Profile said it is",
    );
}

/// And a Conversation nothing has been attached to is given its own attachments
/// directory all the same, made empty as the session starts.
///
/// The third arm of the same claim the other two suites make: a session blocked
/// on an ask goes on running while the human answers it, so a file put on an
/// Answer an hour in has to land somewhere that session already reaches. Here
/// that is the directory itself — nothing is mounted on this platform, and the
/// grant on a directory is one that everything under it inherits, so a file
/// written into it after the boundary was written is a file the session reads.
///
/// The fixture's own file is taken away first, this being the one test here
/// that is about there being none.
#[tokio::test]
async fn the_attachments_directory_is_given_though_nothing_has_been_attached() {
    let fixture = grilling().await;

    std::fs::remove_dir_all(fixture.attachments_dir())
        .expect("the fixture made that directory to put its file in");

    let classified = fixture.probe(&[directory("attachments", fixture.attachments_dir())]);

    assert_eq!(
        said(&classified, "attachments"),
        "read",
        "the directory is made empty and granted at every launch — reachable \
         and not writable, as it is with a file in it. The probe said: \
         {classified:?}",
    );
}

/// And `bash` runs behind the boundary, which is the whole reason this stage
/// exists.
///
/// **The one program every mechanism before the account broke.** An AppContainer
/// would not start msys2 at all; a restricted SID list and a deny-only account
/// SID broke node and both PowerShells with it; a lowered integrity level left
/// the toolchain standing and still cost `bash`, because msys2 makes its shared
/// objects under `\BaseNamedObjects` and nothing below medium may write there
/// (ADR-0014). An ordinary local account is none of those, and this is where
/// that is attempted rather than believed.
///
/// **Started by name off the session's own `PATH`**, which is the same claim
/// the probe's `powershell` makes and one more besides: what a session finds is
/// the machine's own `bash` — Git for Windows' — reached through a `PATH` entry
/// nothing in the description grants, the way `git` and `node` are.
///
/// **And it writes into its Worktree**, because starting is the smaller half:
/// a shell that came up and could reach nothing would be a shell that ran. The
/// file is read back out here rather than off what it printed, for the reason
/// `tests/sessions_windows.rs` reads its evidence off files.
///
/// A machine with no `bash` on its `PATH` fails here rather than passing
/// quietly, which is the rule the two build-cache tests in
/// `tests/sessions_windows.rs` follow: the `windows-2025` runner has Git for
/// Windows, and so does every machine anybody develops Verkstead on.
#[tokio::test]
async fn bash_runs_behind_the_boundary_and_writes_into_the_worktree() {
    let fixture = grilling().await;

    let wrote = fixture.worktree().join("what-bash-wrote.txt");

    // A path `bash` will take as it stands: msys2 reads a drive letter and
    // backslashes, and the single quotes keep the shell out of it.
    let inside = wrote.display().to_string().replace('\\', "/");

    let (rendering, closing) = fixture
        .sandbox()
        .command(&["bash", "-c", &format!("echo {BASH_WROTE} > '{inside}'")])
        .expect("this machine to have the account a session runs as");

    let output = off_a_console(&rendering, b"").expect("bash to start as the session account");

    closing.close();

    assert!(
        output.status.success(),
        "bash should have run behind the boundary and exited well.\n\
         how it ended: {}\n\
         what it printed: {:?}\n\
         what it complained: {:?}\n\
         the program the description resolved: {:?}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        rendering.program(),
    );

    assert_eq!(
        std::fs::read_to_string(&wrote)
            .unwrap_or_else(|error| format!("{}: {error}", wrote.display()))
            .trim(),
        BASH_WROTE,
        "and what it wrote should have landed in the Worktree it was started in",
    );
}

/// What that shell writes, said as one word: it goes on a command line the
/// shell itself parses, and a marker with a space or a newline in it would be
/// this test asking about quoting rather than about `bash`.
const BASH_WROTE: &str = "bash-ran-behind-the-boundary";

/// A machine with no account for its sessions to run as refuses the session,
/// and says which half of it is not there.
///
/// **The two halves are two different things to do about**, which is why they
/// are told apart: an account this machine has never heard of is the elevated
/// verb never having been run, and an account it *has* heard of with no
/// password is that verb having been run and `secrets.yaml` since emptied. Both
/// name the verb, because that is what mends either.
///
/// **And nothing is started either way** — no unsandboxed session to fall back
/// to, which is the whole of ADR-0014 Q18. What comes back is a refusal from
/// [`Sandbox::command`], which is before there is a process at all.
///
/// Neither half needs a machine that has the account, so this is the one test
/// here that would pass on a Windows box where the verb has never been run.
#[tokio::test]
async fn a_machine_with_no_account_or_no_password_refuses_the_session() {
    let fixture = grilling().await;

    // A name no machine has, which is the first half: the lookup is what fails,
    // and it fails before the password is looked at.
    let nobody = fixture.sandbox_running_as("vk-nobody-at-all");

    let refused = nobody
        .command(&[POWERSHELL])
        .expect_err("a session whose account is on no machine should not be started")
        .to_string();

    assert!(
        refused.contains("vk-nobody-at-all"),
        "the refusal should name the account, and it said: {refused}"
    );
    assert!(
        refused.contains("verkstead session-account create"),
        "and say what to run about it, and it said: {refused}"
    );

    // And a name that resolves with nothing holding its password, which is the
    // second: asked of an account every Windows machine already has, this suite
    // having no way to make one.
    let secrets = Settings::in_data_dir(fixture.state.path());
    secrets
        .save_secrets(&secrets.secrets().with_session_account_password(None))
        .expect("a secrets file to be writable");

    let unknown = fixture.sandbox_running_as("SYSTEM");

    let refused = unknown
        .command(&[POWERSHELL])
        .expect_err("a session whose account has no password should not be started")
        .to_string();

    assert!(
        refused.contains("secrets.yaml"),
        "the refusal should say which half was missing, and it said: {refused}"
    );
    assert!(
        refused.contains("verkstead session-account create"),
        "and say what to run about it, and it said: {refused}"
    );
}

/// And what the description does not name is refused rather than absent, with
/// a name nobody ever made told apart from both.
///
/// The whole point of there being a boundary at all, and the one thing a coarser
/// test would pass without: the human's own Documents, which no description
/// mentions; another checkout on the same machine, which is somebody else's
/// work; Verkstead's own record of every Conversation, which is nobody's
/// business inside; the account's own skills, which a description mentions in
/// order to say a session finds nothing there; and the file the host left beside
/// the image, which says that what is granted on that `PATH` entry is the one
/// file rather than the directory holding it.
///
/// **`absent` is asked for on purpose.** A path that was never made and a path
/// that is there and wholly denied are the same answer to anything that only
/// looks, and a suite that could not tell them apart would pass just as happily
/// against a description that named nothing at all.
#[tokio::test]
async fn what_no_description_names_is_refused_and_a_name_nobody_made_is_absent() {
    let fixture = grilling().await;
    let profile = fixture.profile_dir();

    let classified = fixture.probe(&[
        directory("their-skills", profile.join(".claude").join("skills")),
        directory("documents", fixture.documents()),
        directory("sibling", &fixture.sibling),
        file("verksteads-own", fixture.state.path().join("verkstead.db")),
        file(
            "beside-the-image",
            fixture
                .verkstead
                .path()
                .parent()
                .expect("the image is in a directory")
                .join("beside-the-image.txt"),
        ),
        file("never-a-file", fixture.worktree().join("nobody-wrote-this")),
        directory(
            "never-a-directory",
            fixture.worktree().join("nobody-made-this"),
        ),
    ]);

    for (name, what) in [
        (
            "their-skills",
            "the account's own skills, which the description names as nothing \
             at all",
        ),
        (
            "documents",
            "the human's own Documents, which no description names",
        ),
        (
            "sibling",
            "another checkout on the machine, which is somebody else's work",
        ),
        (
            "verksteads-own",
            "Verkstead's own record of every Conversation",
        ),
        (
            "beside-the-image",
            "what the host left beside the image, the description having \
             granted the one file rather than the directory",
        ),
    ] {
        assert_eq!(
            said(&classified, name),
            "refused",
            "{what} is refused from inside — refused rather than absent, the \
             machine being there and denied. The probe said: {classified:?}",
        );
    }

    for name in ["never-a-file", "never-a-directory"] {
        assert_eq!(
            said(&classified, name),
            "absent",
            "a name nobody ever made, inside a directory the description grants, \
             is absent rather than refused — which is what says the refusals \
             above are about a boundary rather than about a machine with \
             nothing on it. The probe said: \
             {classified:?}",
        );
    }

    // And from the host, which is the other half of the same claim: what was
    // refused is still there and still says what it said. A boundary that
    // worked by taking something away would be no boundary.
    assert_eq!(
        std::fs::read_to_string(
            fixture
                .account
                .join(".claude")
                .join("skills")
                .join("theirs.md")
        )
        .expect("the account's own skills are the account's"),
        THEIR_SKILL,
    );
    assert!(
        fixture.documents().join(MARKER).is_file(),
        "and so are the human's own Documents",
    );
}

/// A live Conversation's Worktree is reachable from another Conversation's
/// session, and a stopped one's is not — asked by attempting both.
///
/// **This is the cost the account stage accepted, asserted rather than allowed
/// for** (ADR-0014, *Amended: the Sandbox is an account*). There is one account
/// for the installation, because making one needs elevation and one per
/// Conversation would need elevation per Conversation — so two Conversations
/// running at once are one identity, and what the second was granted the first
/// can reach. A suite that did not say so out loud would leave the reader of
/// this file believing the boundary is narrower than it is.
///
/// **And the bound on it, in the same run.** What keeps that cost to *live*
/// work is that a Conversation's entries come off with its Worktree — see the
/// server's `sandbox::entries` — so the same directory, once its Conversation
/// has stopped, is refused to exactly the session that could read it a moment
/// ago. Either half alone is passable by accident: a machine that granted
/// everything would pass the first, and one that had granted nothing would pass
/// the second.
///
/// **The second Conversation's session really runs its description**, which is
/// what makes both halves worth asserting: a checkout nothing had ever been
/// granted would be refused by a machine that had never heard of either
/// Conversation.
#[tokio::test]
async fn a_live_conversations_worktree_is_reachable_and_a_stopped_ones_is_not() {
    let fixture = grilling().await;
    let theirs = fixture.beside("something-of-their-own").await;

    let worktree = theirs
        .worktree
        .clone()
        .expect("the second Conversation has a worktree");

    // Its boundary written on the machine, and held for as long as the probe
    // below runs: what is being asked is what this session may reach while that
    // Conversation is still working.
    let (_theirs, closing) = fixture
        .sandbox_of(&theirs)
        .command(&[POWERSHELL])
        .expect("this machine to have the account a session runs as");

    let asked = [
        directory("theirs", &worktree),
        file("their-file", worktree.join(MARKER)),
        directory("ours", fixture.worktree()),
    ];

    let while_live = fixture.probe(&asked);

    assert_eq!(
        said(&while_live, "ours"),
        "write",
        "a session reaches its own Conversation's checkout, which is what says \
         the rest of this is about whose it is. The probe said: {while_live:?}",
    );

    for (name, what) in [
        ("theirs", "another live Conversation's checkout"),
        ("their-file", "and a file inside it"),
    ] {
        assert_eq!(
            said(&while_live, name),
            "write",
            "{what} is reachable while that Conversation is working, there being \
             one account for the installation — which is the cost this stage \
             accepted. The probe said: {while_live:?}",
        );
    }

    // And that Conversation stopping, which is what takes its entries off: the
    // close, said the way the server says it — see `boundaries::remove`.
    closing.close();
    boundaries::remove(fixture.state.path(), theirs.id);

    let once_stopped = fixture.probe(&asked);

    assert_eq!(
        once_stopped.get("ours").map(String::as_str),
        Some("write"),
        "this session's own checkout is untouched by another Conversation \
         ending. The probe said: {once_stopped:?}",
    );

    for (name, what) in [
        ("theirs", "a stopped Conversation's checkout"),
        ("their-file", "and a file inside it"),
    ] {
        assert_eq!(
            said(&once_stopped, name),
            "refused",
            "{what} is refused once its entries have come off — refused rather \
             than absent, the directory being still there. The probe said: \
             {once_stopped:?}",
        );
    }
}

/// Closing a Conversation takes every entry written for it off the human's own
/// directories.
///
/// **Read back rather than attempted, which is the one place this suite has to
/// be.** Everything else here asks the boundary by running a program behind it;
/// what is being asserted here is that the entries are *gone*, and the account
/// they named is still on the machine — it is the installation's, not this
/// Conversation's. So the machine's own `icacls` is asked what each directory's
/// list says.
///
/// **And the entries are asserted to have been there first.** Otherwise this
/// would pass just as happily against a run where nothing was ever written.
///
/// The kinds are the ones this fixture's description names: the Worktree and the
/// git directory behind it, the account, the skills, the image and the two
/// configured binds — and beside them the one path a description refuses rather
/// than grants, which is left as it was found rather than merely un-denied.
///
/// A `PATH` entry under the human's profile is the same kind of entry as the
/// skills — a read-only grant on a real directory — and is in no description
/// here, this fixture's human profile being a temporary directory that nothing
/// on the machine's `PATH` is under. Where that rule is proved against a real
/// boundary is the server's own `sandbox::granting::writing` tests, which can
/// say what the `PATH` is without saying it to this whole process.
#[tokio::test]
async fn closing_a_conversation_takes_every_entry_written_for_it() {
    let fixture = grilling().await;

    // Read before anything is written, because what a refusal has to leave
    // behind is this and not merely the absence of a deny — see the server's
    // `sandbox::granting::writing::restored`.
    let refused = fixture.their_skills();
    let before = fixture.reading();
    let as_it_was = listed(&refused);

    let (_rendering, closing) = fixture
        .sandbox()
        .command(&[POWERSHELL])
        .expect("this machine to have the account a session runs as");

    let account = fixture.session_account();

    let standing = fixture.reading();
    let written_down = fixture.written_down();

    closing.close();

    let granted = fixture.granted();

    for path in granted.iter().chain([&refused]) {
        assert!(
            names(&account, path),
            "{} should be carrying an entry for the session account before \
             anything is taken back",
            path.display(),
        );
    }

    // What the close does about the boundary, which is the whole of this test's
    // subject — see the server's `boundaries::closing`, the one line of the
    // close that reaches it.
    boundaries::remove(fixture.state.path(), fixture.conversation.id);

    for path in granted.iter().chain([&refused]) {
        assert!(
            !names(&account, path),
            "{} should have been left as the human's own again, and it says: {}",
            path.display(),
            listed(path),
        );
    }

    // And the refused one down to the entry, which the assertion above cannot
    // see: cutting the inheritance kept what the directory was inheriting as
    // its own, and a close that put the inheritance back and left those copies
    // would grow this list by a few entries every time a Conversation ended.
    // The one thing it does not ask of the list is [`reach`]'s.
    assert_eq!(
        reach(&listed(&refused)),
        reach(&as_it_was),
        "the one directory a description refuses should leave the human \
         reaching it exactly as they did before the boundary was written, and \
         what it said all along is: {}",
        fixture.trail(&before, &standing, &written_down),
    );

    assert!(
        !fixture
            .state
            .path()
            .join("containers")
            .join(fixture.conversation.id.to_string())
            .exists(),
        "and the record they were written down in should have gone with them",
    );
}

/// And what a crash left behind is taken off the machine by the next server's
/// startup sweep.
///
/// **The case nothing else covers.** A close takes a Conversation's boundary
/// with its Worktree; a server that died took nothing at all, and what it left
/// is a set of entries on the human's own directories that nothing in any
/// memory describes any more. What the next server has to go on is what this
/// one wrote down — see the server's `sandbox::granting::remembering`.
///
/// The crash is made by letting go of the hold without taking anything back,
/// which is exactly what a process that died did — see [`entries::forgotten`].
/// The Conversation is Done rather than Closed because Done is the harder half
/// of the rule: its Worktree stays, and its boundary goes.
#[tokio::test]
async fn the_entries_a_crash_left_behind_are_swept_at_the_next_startup() {
    let fixture = grilling().await;

    let refused = fixture.their_skills();
    let before = fixture.reading();
    let as_it_was = listed(&refused);

    let (_rendering, closing) = fixture
        .sandbox()
        .command(&[POWERSHELL])
        .expect("this machine to have the account a session runs as");

    let account = fixture.session_account();

    let standing = fixture.reading();
    let written_down = fixture.written_down();

    closing.close();

    let granted = fixture.granted();

    store::set_state(&fixture.pool, fixture.conversation.id, Lifecycle::Done)
        .await
        .expect("the Conversation to have finished");

    // And the server dies holding them: the entries stay exactly where they
    // are, and this process stops knowing about them.
    entries::forgotten(fixture.state.path(), fixture.conversation.id);

    for path in granted.iter().chain([&refused]) {
        assert!(
            names(&account, path),
            "{} should still be carrying the entry a crash left on it",
            path.display(),
        );
    }

    // What the next server does before it serves anything.
    boundaries::swept(&fixture.pool, fixture.state.path()).await;

    for path in granted.iter().chain([&refused]) {
        assert!(
            !names(&account, path),
            "{} should have been left as the human's own again, and it says: {}",
            path.display(),
            listed(path),
        );
    }

    // Down to the entry on the one path a description refuses, the sweep having
    // exactly the close's job here and no more of the description to do it
    // from — see the close's own test, where the reason this is asserted
    // separately is.
    assert_eq!(
        reach(&listed(&refused)),
        reach(&as_it_was),
        "the sweep should leave the human reaching the account's own skills as \
         they did before the boundary was written, and what they said all along \
         is: {}",
        fixture.trail(&before, &standing, &written_down),
    );

    assert!(
        !fixture
            .state
            .path()
            .join("containers")
            .join(fixture.conversation.id.to_string())
            .exists(),
        "and so should the record the sweep read them off",
    );

    assert!(
        fixture.worktree().is_dir(),
        "and the Worktree should still be there: Done is not Closed, and what a \
         Follow-up steer picks the work up in is the checkout it was left in",
    );
}

/// What an access-control list says, as the machine's own tool prints it.
///
/// `icacls` rather than a call of this suite's own: what is being read back is
/// the list Windows really holds, and a reader that this file wrote would be
/// this file agreeing with itself.
fn listed(path: &Path) -> String {
    let listed = Command::new("icacls")
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .expect("icacls is part of Windows");

    format!(
        "{}{}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr),
    )
}

/// The same list read for who reaches the directory, with where each entry came
/// from left out of it.
///
/// **Because that last part is not a refusal's to give back**, and asking for it
/// is what made the two lifetime tests above unpassable on the one machine that
/// answers them. The `windows-2025` runner's temporary directory — which is
/// where this suite's stand-in for the human's account lives — is a tree of the
/// older age: a directory under it holds copies of the entries its parent hands
/// down, not marked as taken from above at all. That marking is what Windows
/// puts right the first time anything writes an access-control list in the tree,
/// and it will not be put back: writing the copies again with the inheritance on
/// leaves the directory holding both them and the entries that come down, and
/// writing them with the inheritance off leaves a directory of the human's own
/// cut off from whatever they change above it. The entries themselves come back
/// exactly — every trustee the human had, at exactly the reach they had — and
/// only the word for where each one came from is the machine's to say.
///
/// So what is compared is the reach and not the bookkeeping. Everything a
/// refusal owes the directory is still in it: a deny left standing is an entry
/// that is still there, a copy left behind is an entry more than there were, and
/// a directory whose own entries were taken off for the ones above it is a list
/// of somebody else's trustees. `(I)` is the only thing dropped, and `icacls`
/// writes it nowhere but in front of an entry it is saying that about.
fn reach(listed: &str) -> String {
    listed.replace("(I)", "")
}

/// And whether that list names the session account, by either of the two
/// spellings it can be printed in.
///
/// The SID and the account's own name both, because which of them appears is
/// the machine's business: `icacls` resolves a SID to the name it belongs to
/// wherever Windows can look it up, and prints the number itself where it
/// cannot. Neither is a spelling anything else on this machine writes — the
/// name is a Data Directory's fingerprint behind `vk-` — so a list holding
/// either is a list holding this identity.
fn names(account: &Account, path: &Path) -> bool {
    let listed = listed(path);

    listed.contains(account.sid().text()) || listed.contains(account.name())
}
