//! Whether a fresh Verkstead can do anything yet, asked of the server over the
//! viewer's namespace: the mode, the machine it is standing on, and what is
//! missing from it.
//!
//! Every one of those is a fact about the machine the tests happen to be
//! running on, which is the one machine this suite must not be asking about: a
//! wizard that has to say something about a box with no sandbox and a box with
//! everything cannot be asked about either on the box the suite is on. So the
//! machine is stated — a `PATH` of the suite's own making, with a `bwrap` that
//! is a shell script in it — the way `tailscale` is a script in the Remote
//! access suite and `gh` is one in the settings suite. What is asserted is what
//! the server made of it.
//!
//! **The accounts are stated the same way.** What the machine holds is looked
//! for in the home the server was started with, and the suite's is a directory
//! of its own with the four shapes made under it by hand — an account being a
//! set of paths rather than anything the agent has to have been run to make.
//!
//! The other half of the objective is not on the machine at all: a Profile is a
//! row in the store and an author is a line in `config.yaml`. Both go in
//! **before** the router is stood up, which is what makes a start over them a
//! start that has them — see [`served`]. The verdict is reached once, at
//! startup, so a suite that wrote them afterwards would be asking about a
//! server that came up without them.
//!
//! **Which is the whole of the last test here.** Deleting the last Profile once
//! the server is up is a step that stops standing met under a mode that stays
//! off — see ADR-0016, where a live predicate was rejected for exactly the page
//! it would put over work somebody has.
//!
//! Unix only: what a `bwrap` did is a shell script here, and a script is what a
//! machine with a shell can be handed. Every arm that is not this platform's is
//! a unit test in the module itself, which is where a stated platform can be
//! asked about without a machine to run one on.
#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    Dependency, DependencyState, DependencyView, Distro, OnboardingView, PrefillView,
    ProfileAccount, Seen, Source,
};
use verkstead_server::github::Gh;
use verkstead_server::onboarding::Machine;
use verkstead_server::platform::{Environment, Platform};
use verkstead_server::store::{Account, AgentType, ProfileFacts};
use verkstead_server::{open_database, router_onboarding_asking_github, store};

/// A program that is there and does nothing, which is the whole of what a row
/// probed by a `PATH` walk asks of one.
const A_PROGRAM: &str = "#!/bin/sh\nexit 0\n";

/// A `bwrap` that makes the namespace it was asked for, which is the one row
/// answered by a run rather than a walk.
const MAKES_A_NAMESPACE: &str = A_PROGRAM;

/// And one that is installed and will not: the words a machine with
/// unprivileged user namespaces switched off refuses in, which is the line
/// worth putting under the row.
const REFUSED: &str = "#!/bin/sh\n\
                       echo 'bwrap: No permissions to creating new namespace, \
                       likely because the kernel does not allow non-privileged \
                       user namespaces' >&2\n\
                       exit 1\n";

/// And what that machine said, as the row carries it.
const REFUSAL: &str = "bwrap: No permissions to creating new namespace, likely because \
                       the kernel does not allow non-privileged user namespaces";

/// And a `git` that is configured: one that answers `git config --global --get`
/// for the two keys the git step is prefilled from.
///
/// A script rather than the machine's own `git`, for the reason the `bwrap` is
/// one: what the step offers is whatever this box's `~/.gitconfig` happens to
/// say, and a suite that read it would assert something different on every box.
const CONFIGURED: &str = "#!/bin/sh\n\
                          test \"$1 $2 $3\" = 'config --global --get' || exit 2\n\
                          case \"$4\" in\n\
                          user.name) echo 'Ada Lovelace';;\n\
                          user.email) echo 'ada@example.com';;\n\
                          *) exit 1;;\n\
                          esac\n";

/// What this machine says it is, which is the tab the wizard opens on.
const OS_RELEASE: &str = "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n";

/// Everything a machine needs: a sandbox that works, `git`, and one harness.
/// No `gh`, which is the row that never gates.
const EVERYTHING: &[(&str, &str)] = &[
    ("bwrap", MAKES_A_NAMESPACE),
    ("git", A_PROGRAM),
    ("claude", A_PROGRAM),
];

/// A Data Directory with a database in it, and nothing said yet about what this
/// start is to find there.
async fn ready() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool)
}

/// And a server over it, probing a machine whose `PATH` is one directory of the
/// suite's own making, holding `programs` — each a name and the script it is.
///
/// Stood up last, deliberately: the verdict is reached at startup, so whatever
/// this start is meant to have is put in the Data Directory before this is
/// called and nothing is written into it afterwards but the one delete the last
/// test makes.
fn served(dir: &Path, pool: &SqlitePool, programs: &[(&str, &str)]) -> Router {
    served_holding(dir, pool, programs, Held::default())
}

/// The same, over a server whose environment holds what `held` says and whose
/// host `gh` is logged in as `held` says.
///
/// The other half of what the git step is drawn from: its token comes out of
/// the server's own environment or out of the login the machine's `gh` has, and
/// neither of those is a thing to ask the box the suite is running on.
fn served_holding(dir: &Path, pool: &SqlitePool, programs: &[(&str, &str)], held: Held) -> Router {
    let bin = bin(dir);

    for (name, script) in programs {
        program(&bin.join(name), script);
    }

    let machine = Machine::stated(
        Platform::Linux,
        OsString::from(bin.as_os_str()),
        // The server's own `PATH` is that one directory and the one nothing
        // composes in — an `/opt/foo/bin`, which is where a program a session
        // cannot open is seen.
        joined(&[&bin, &beyond(dir)]),
        None,
        Some(OS_RELEASE.to_owned()),
        &Environment {
            home: Some(home(dir)),
            gh_token: held.gh_token.map(str::to_owned),
            github_token: held.github_token.map(str::to_owned),
            ..Environment::default()
        },
    );

    router_onboarding_asking_github(pool.clone(), dir.to_owned(), machine, host_gh(dir, held))
}

/// What a start is holding that is neither on its `PATH` nor in its Data
/// Directory: the two token variables in its own environment, and whatever the
/// machine's own `gh` is logged in as.
#[derive(Debug, Default, Clone, Copy)]
struct Held {
    gh_token: Option<&'static str>,
    github_token: Option<&'static str>,
    host_login: Option<&'static str>,
}

/// The `gh` this server reaches GitHub through: one that answers `gh auth
/// token` with the login it is holding, or one that is logged into nothing.
///
/// Written outside the `PATH` the probes walk, because it is not that question:
/// whether this machine has a `gh` is a row on the dependencies step, and this
/// is the `gh` the server itself runs.
fn host_gh(dir: &Path, held: Held) -> Gh {
    let path = dir.join("host-gh");

    program(
        &path,
        &match held.host_login {
            Some(token) => {
                format!("#!/bin/sh\ntest \"$*\" = 'auth token' || exit 2\necho {token}\n")
            }
            None => "#!/bin/sh\nexit 1\n".to_owned(),
        },
    );

    Gh::running(vec![path.to_string_lossy().into_owned()])
}

/// The one directory on a session's `PATH` here: where this suite's programs
/// are, which is what a row that is there names.
fn bin(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    bin
}

/// The directory on the server's own `PATH` that a session's is not composed
/// with: where a program the human really has can be seen and no session can
/// open it.
///
/// Made whether anything is put in it or not, a `PATH` entry naming nothing
/// being a line in somebody's shell profile rather than an error.
fn beyond(dir: &Path) -> PathBuf {
    let beyond = dir.join("opt/foo/bin");
    std::fs::create_dir_all(&beyond).unwrap();

    beyond
}

/// Those two directories as one `PATH`.
fn joined(entries: &[&Path]) -> OsString {
    let written: Vec<String> = entries
        .iter()
        .map(|entry| entry.to_string_lossy().into_owned())
        .collect();

    OsString::from(written.join(":"))
}

/// Where a row that is there says the program is: the `PATH` entry this suite
/// writes its programs into, with the name on the end of it.
fn found(dir: &Path, program: &str) -> DependencyState {
    DependencyState::Present {
        at: Some(bin(dir).join(program).to_string_lossy().into_owned()),
        target: None,
    }
}

/// And what a row that is not there says where the name was seen nowhere at
/// all.
const MISSING: DependencyState = DependencyState::Absent {
    trouble: None,
    seen: None,
};

/// A script at `path`, executable.
fn program(path: &Path, script: &str) {
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// The home this start was given, which is where its accounts are looked for.
///
/// Under the suite's own directory rather than the one the tests are running
/// as: what is found in it is what this test put there, on every box and on a
/// box whose own home is full of accounts.
fn home(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();

    home
}

/// An account of `agent_type` in that home: the paths a session of that type
/// mounts its account from, made by hand.
///
/// Made rather than logged in: an account is a set of paths, and what the
/// wizard answers is whether they are there.
fn an_account(dir: &Path, agent_type: AgentType) {
    let home = home(dir);

    let dirs: Vec<PathBuf> = match agent_type {
        AgentType::Claude => vec![home.join(".claude")],
        AgentType::Codex => vec![home.join(".codex")],
        AgentType::Grok => vec![home.join(".grok")],
        AgentType::OpenCode => vec![
            home.join(".config/opencode"),
            home.join(".local/share/opencode"),
        ],
    };

    for path in dirs {
        std::fs::create_dir_all(path).unwrap();
    }

    if agent_type == AgentType::Claude {
        std::fs::write(home.join(".claude.json"), "{}\n").unwrap();
    }
}

/// An Agent Profile in the store, which is the whole of what the accounts step
/// asks for.
async fn a_profile(pool: &SqlitePool, dir: &Path) -> i64 {
    let account = dir.join("account");
    std::fs::create_dir_all(&account).unwrap();

    let profile = store::create_profile(
        pool,
        &ProfileFacts {
            name: Some("Mine".to_owned()),
            account: Account::Claude {
                claude_dir: account.clone(),
                config_file: account.join("claude.json"),
            },
            models: vec!["sonnet".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("nothing else is called that");

    profile.id
}

/// And a git author in `config.yaml`, which is the whole of what the git step
/// asks for. Written rather than saved through the endpoint: what is being
/// stood up is a server that already has one.
fn an_author(dir: &Path) {
    std::fs::write(
        dir.join("config.yaml"),
        "git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n",
    )
    .unwrap();
}

async fn reading(app: &Router) -> OnboardingView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/onboarding")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).expect("the reading the wizard is drawn from")
}

/// And what the git step could be prefilled with, which is the read of its own
/// that step makes.
async fn prefill(app: &Router) -> PrefillView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/onboarding/git")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).expect("what the git step's fields are filled with")
}

/// And the wizard's last Continue, which answers with the reading made again.
async fn finished(app: &Router) -> OnboardingView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/onboarding/finished")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).expect("the reading the wizard is drawn from")
}

/// What one row of a reading says.
fn row(reading: &OnboardingView, dependency: Dependency) -> &DependencyView {
    reading
        .dependencies
        .iter()
        .find(|row| row.dependency == dependency)
        .expect("every dependency is a row")
}

/// A machine whose `bwrap` will not make a namespace comes up in onboarding
/// mode, with the sandbox row as the one thing holding it and the failed run's
/// own words under it.
///
/// Everything else about this start is met — `git`, a harness, a Profile and an
/// author — so what puts it in the mode is the sandbox and nothing else, which
/// is what makes the row the reading names the row that is wrong.
#[tokio::test]
async fn a_sandbox_that_will_not_run_is_a_start_in_onboarding_mode() {
    let (dir, pool) = ready().await;

    a_profile(&pool, dir.path()).await;
    an_author(dir.path());

    let app = served(
        dir.path(),
        &pool,
        &[
            ("bwrap", REFUSED),
            ("git", A_PROGRAM),
            ("claude", A_PROGRAM),
        ],
    );

    let reading = reading(&app).await;

    assert!(
        reading.mode,
        "a Verkstead that cannot make a sandbox can run no session, so the \
         wizard is the only page there is"
    );

    assert_eq!(
        row(&reading, Dependency::Sandbox).state,
        DependencyState::Absent {
            trouble: Some(REFUSAL.to_owned()),
            seen: None,
        },
        "the machine's own line, which is the one that names what to change"
    );

    assert!(
        !reading.steps.dependencies,
        "the step the sandbox row is on is the unmet one"
    );
    assert!(
        reading.steps.accounts && reading.steps.git,
        "and the two beside it stand met: a Profile and an author are what the \
         store and the settings file already hold"
    );

    assert_eq!(
        reading.distro,
        Distro::Ubuntu,
        "and the tab that opens is the one this machine says it is"
    );
}

/// And a start with a sandbox, `git`, a harness, one Profile and an author
/// comes up with the mode off — which is the workbench opening as it always
/// did.
#[tokio::test]
async fn a_machine_with_the_objective_met_comes_up_with_the_mode_off() {
    let (dir, pool) = ready().await;

    a_profile(&pool, dir.path()).await;
    an_author(dir.path());

    let app = served(dir.path(), &pool, EVERYTHING);
    let reading = reading(&app).await;

    assert!(!reading.mode, "there is nothing to put in front of anybody");

    assert!(reading.steps.dependencies);
    assert!(reading.steps.accounts);
    assert!(reading.steps.git);

    assert_eq!(
        row(&reading, Dependency::Sandbox).state,
        found(dir.path(), "bwrap"),
        "the `bwrap` on this machine made the namespace it was asked for, and \
         the row says which one was run"
    );
    assert_eq!(
        row(&reading, Dependency::Claude).state,
        found(dir.path(), "claude"),
        "and one harness is what the objective asks for — this one, at this path"
    );
    assert_eq!(
        row(&reading, Dependency::Codex).state,
        MISSING,
        "the three beside it are absent, and hold nothing up"
    );
    assert_eq!(
        row(&reading, Dependency::Gh).state,
        MISSING,
        "and `gh` is absent as well, GitHub being a choice rather than a dependency"
    );

    assert_eq!(
        reading.path,
        vec![dir.path().join("bin").to_string_lossy().into_owned()],
        "and the step says where a session looks, which is the list a session \
         was given rather than a sentence about one"
    );
}

/// A harness on the `PATH` the server was started with, in a directory a
/// session's own does not hold, reads absent with where it was seen.
///
/// The bug this whole feature is about, said in the row: the program is on the
/// machine, nothing a session can open is, and what is wanted is a `PATH` and a
/// restart rather than an install.
#[tokio::test]
async fn a_harness_a_session_cannot_reach_says_where_it_was_seen() {
    let (dir, pool) = ready().await;

    let out_of_reach = beyond(dir.path());
    program(&out_of_reach.join("codex"), A_PROGRAM);

    let app = served(dir.path(), &pool, EVERYTHING);
    let reading = reading(&app).await;

    assert_eq!(
        row(&reading, Dependency::Codex).state,
        DependencyState::Absent {
            trouble: None,
            seen: Some(Seen::Beyond {
                at: out_of_reach.join("codex").to_string_lossy().into_owned(),
            }),
        },
        "the row names the directory it was seen in, that being what there is \
         to do something about"
    );
    assert_eq!(
        row(&reading, Dependency::Grok).state,
        MISSING,
        "while a harness on neither list was seen nowhere, and has nothing \
         said under it but what to install"
    );
}

/// Each of the three things the objective is holds the mode on by itself, and
/// the step that reads unmet is the one it belongs to.
///
/// Three starts rather than three requests, because the mode is settled once
/// per start: what is being asked is what a machine short of one thing comes up
/// as, and that is a fact about a start.
#[tokio::test]
async fn each_of_the_three_holds_the_mode_on_by_itself() {
    // No harness at all, and the other two given: a Verkstead with nothing to
    // run a session with.
    let without_a_harness = {
        let (dir, pool) = ready().await;

        a_profile(&pool, dir.path()).await;
        an_author(dir.path());

        let app = served(
            dir.path(),
            &pool,
            &[("bwrap", MAKES_A_NAMESPACE), ("git", A_PROGRAM)],
        );

        reading(&app).await
    };

    assert!(without_a_harness.mode);
    assert!(
        !without_a_harness.steps.dependencies,
        "a machine with no harness on it can launch nothing"
    );

    // Everything on the machine, and no Profile: nothing to launch a session
    // *under*.
    let without_a_profile = {
        let (dir, pool) = ready().await;

        an_author(dir.path());

        let app = served(dir.path(), &pool, EVERYTHING);

        reading(&app).await
    };

    assert!(without_a_profile.mode);
    assert!(
        !without_a_profile.steps.accounts,
        "an account on the machine is not an Agent Profile until it is saved as one"
    );

    // And everything but the author, which is what git asks for on every commit
    // a session makes.
    let without_an_author = {
        let (dir, pool) = ready().await;

        a_profile(&pool, dir.path()).await;

        let app = served(dir.path(), &pool, EVERYTHING);

        reading(&app).await
    };

    assert!(without_an_author.mode);
    assert!(
        !without_an_author.steps.git,
        "a session that committed as nobody is a session git would refuse"
    );
}

/// The verdict is fixed for the length of a run: deleting the last Profile
/// while the server is up leaves the mode where the start put it, and the step
/// beside it says what is now true.
///
/// The two are meant to disagree. A live predicate would put a first-run page
/// over work somebody has, which is what ADR-0016 rejected it for — so what a
/// Profile that has gone gets is the settings page's own empty state, and
/// nothing here.
#[tokio::test]
async fn deleting_the_last_profile_mid_run_leaves_the_mode_where_it_was() {
    let (dir, pool) = ready().await;

    let profile = a_profile(&pool, dir.path()).await;
    an_author(dir.path());

    let app = served(dir.path(), &pool, EVERYTHING);

    assert!(
        !reading(&app).await.mode,
        "this start found the objective met"
    );

    store::delete_profile(&pool, profile).await.unwrap();

    let reading = reading(&app).await;

    assert!(
        !reading.mode,
        "the mode is off for the rest of this run, whatever has been deleted since"
    );
    assert!(
        !reading.steps.accounts,
        "and the step says what is true now: there is no Profile left to launch under"
    );
}

/// The accounts in the server's home are offered as the Profiles they would be
/// saved as, each with whether the harness that runs it is on this machine.
///
/// The two halves of the accounts step in one reading: an account whose binary
/// is there is one to tick, and an account whose binary is not is one to grey.
/// The paths are the account's own, because what the step does with a ticked
/// row is hand them back to the profile create.
#[tokio::test]
async fn the_accounts_in_the_home_are_offered_with_their_harnesses() {
    let (dir, pool) = ready().await;

    an_account(dir.path(), AgentType::Claude);
    an_account(dir.path(), AgentType::Codex);

    // Claude Code is on this machine and Codex is not, which is the whole
    // difference between the two rows.
    let app = served(dir.path(), &pool, EVERYTHING);
    let reading = reading(&app).await;

    let home = home(dir.path());

    assert_eq!(
        reading
            .accounts
            .iter()
            .map(|found| (found.account.clone(), found.harness))
            .collect::<Vec<_>>(),
        vec![
            (
                ProfileAccount::Claude {
                    claude_dir: home.join(".claude").to_string_lossy().into_owned(),
                    config_file: home.join(".claude.json").to_string_lossy().into_owned(),
                },
                true,
            ),
            (
                ProfileAccount::Codex {
                    home: home.join(".codex").to_string_lossy().into_owned(),
                },
                false,
            ),
        ],
        "one account per harness, in the order the harness rows are drawn, and \
         ticked by whether a session could be launched under it",
    );

    assert!(
        !reading.steps.accounts,
        "an account on the machine is not an Agent Profile until it is saved as \
         one, which is what the step's own Continue does",
    );
}

/// A home with nothing in it offers nothing, which is the step that says what
/// to run rather than what to tick.
#[tokio::test]
async fn a_home_with_no_account_in_it_offers_none() {
    let (dir, pool) = ready().await;

    let app = served(dir.path(), &pool, EVERYTHING);

    assert!(
        reading(&app).await.accounts.is_empty(),
        "nothing was ever logged in here",
    );
}

/// The git step's fields are prefilled from the machine and the server's own
/// environment, each labelled with where its value was found.
#[tokio::test]
async fn the_git_step_is_prefilled_with_what_the_machine_could_say() {
    let (dir, pool) = ready().await;

    let app = served_holding(
        dir.path(),
        &pool,
        &[("bwrap", MAKES_A_NAMESPACE), ("git", CONFIGURED)],
        Held {
            github_token: Some("ghp_intheenvironment"),
            host_login: Some("ghp_thehostslogin"),
            ..Held::default()
        },
    );

    let prefill = prefill(&app).await;

    assert_eq!(
        prefill.name.as_ref().map(|found| found.value.as_str()),
        Some("Ada Lovelace"),
        "the author this machine's own commits are by",
    );
    assert_eq!(
        prefill.name.map(|found| found.source),
        Some(Source::GitConfig),
        "labelled with where the human can go and check it",
    );
    assert_eq!(
        prefill.email.map(|found| (found.value, found.source)),
        Some(("ada@example.com".to_owned(), Source::GitConfig)),
    );
    assert_eq!(
        prefill.token.map(|found| (found.value, found.source)),
        Some(("ghp_intheenvironment".to_owned(), Source::GithubToken)),
        "the environment before the host's own login, and named as the \
         variable that held it",
    );
}

/// And a field Verkstead has already been told is not prefilled at all: what is
/// in front of the human is then what is written down, and a token already
/// configured is one this endpoint has no business handing back.
#[tokio::test]
async fn a_field_verkstead_already_holds_is_left_alone() {
    let (dir, pool) = ready().await;

    an_author(dir.path());

    let app = served_holding(
        dir.path(),
        &pool,
        &[("git", CONFIGURED)],
        Held {
            gh_token: Some("ghp_intheenvironment"),
            ..Held::default()
        },
    );

    let prefill = prefill(&app).await;

    assert_eq!(prefill.name, None, "config.yaml already names the author");
    assert_eq!(prefill.email, None);
    assert_eq!(
        prefill.token.map(|found| found.source),
        Some(Source::GhToken),
        "and the one field nothing has been said about is the one offered",
    );
}

/// The wizard's last Continue takes the mode off for the rest of the run, and
/// says so in the reading it answers with.
///
/// Nothing is written by it: the steps under it stand exactly as they stood,
/// and what would make them met is what the presses before this one saved. See
/// ADR-0016 — the verdict is a fact about the start, and this is what has
/// happened since.
#[tokio::test]
async fn finishing_the_wizard_takes_the_mode_off_for_the_rest_of_the_run() {
    let (dir, pool) = ready().await;

    let app = served(dir.path(), &pool, &[]);

    assert!(
        reading(&app).await.mode,
        "a machine with nothing on it came up in onboarding mode"
    );

    let answered = finished(&app).await;

    assert!(!answered.mode, "the press answers with the mode off");
    assert!(
        !answered.steps.dependencies && !answered.steps.accounts && !answered.steps.git,
        "and with the steps as they are, which is what the next start will judge"
    );

    assert!(
        !reading(&app).await.mode,
        "and every reading afterwards says the same: there is no way back into \
         the wizard until the next start",
    );
}

/// And the next start reaches the verdict afresh: the wizard finishing was a
/// fact about that process and nothing was written down.
///
/// Which is the whole of what *once, at startup* is worth: a machine that is
/// still short of the objective is a start in the mode again, and one that has
/// what it was missing is the workbench opening as it always did.
#[tokio::test]
async fn the_next_start_reaches_the_verdict_afresh() {
    let (dir, pool) = ready().await;

    let app = served(dir.path(), &pool, &[]);
    finished(&app).await;

    // A second router over the same Data Directory, which is what a restart is
    // to everything below the process.
    let restarted = served(dir.path(), &pool, &[]);

    assert!(
        reading(&restarted).await.mode,
        "the machine is still missing what it was missing, so this start is the \
         wizard again",
    );

    // And the same machine with the objective met, which is the start that
    // opens the workbench.
    a_profile(&pool, dir.path()).await;
    an_author(dir.path());

    let met = served(dir.path(), &pool, EVERYTHING);

    assert!(
        !reading(&met).await.mode,
        "what the wizard saved is what the next start reads",
    );
}

/// Where the golden fixtures are written, relative to this crate — the same
/// directory `ui_content` and `nudges` write the other endpoints' payloads to.
const FIXTURES: &str = "../../web/tests/fixtures";

/// Leave the viewer's own tests a reading of each shape the wizard is drawn
/// over, exactly as this server writes one.
///
/// Committed, and rewritten by every run of this test: the diff is the review.
/// The wizard's component tests are fed from these rather than from a payload
/// somebody typed out, so a field added on this side that nobody carried across
/// shows up as a failing fixture rather than as a page drawing the wrong thing.
///
/// Three of them, because three states are what the frame has to draw: a start
/// with nothing at all, one part way through, and one whose objective is met —
/// which is the only one of the three the wizard is not the page for.
///
/// Nothing here is read off the machine the suite is on. The `PATH`, the
/// `bwrap` and `/etc/os-release` are all [`served`]'s own, so a run today and a
/// run on another box write the same bytes.
#[tokio::test]
async fn the_viewers_own_tests_are_fed_from_here() {
    // A machine with nothing on it and a Data Directory with nothing in it:
    // every row absent, no account in the home and all three steps unmet, which
    // is a first start on a box somebody has just installed Verkstead on.
    let (dir, pool) = ready().await;
    let app = served(dir.path(), &pool, &[]);
    write("onboarding-fresh.json", &reading(&app).await, dir.path());

    // The dependencies settled and nothing else: the step that is met, the two
    // that are not, and the mode still on. Two accounts in the home, one of
    // whose harnesses is on the machine, which is the row to tick and the row
    // to grey.
    let (dir, pool) = ready().await;
    an_account(dir.path(), AgentType::Claude);
    an_account(dir.path(), AgentType::Codex);
    let app = served(dir.path(), &pool, EVERYTHING);
    natively_installed(dir.path());
    write("onboarding-part-way.json", &reading(&app).await, dir.path());

    // And the objective met, which is the reading that says the wizard is no
    // page at all.
    let (dir, pool) = ready().await;
    a_profile(&pool, dir.path()).await;
    an_author(dir.path());
    let app = served(dir.path(), &pool, EVERYTHING);
    write("onboarding-ready.json", &reading(&app).await, dir.path());

    // And the git step's own read, in the two shapes it comes in: a machine
    // that can answer every field, and one that can answer none. Both are
    // written from a stated machine for the reason the readings above are —
    // this box's own `~/.gitconfig` and `gh` are nothing a committed fixture
    // could hold true.
    let (dir, pool) = ready().await;
    let app = served_holding(
        dir.path(),
        &pool,
        &[("git", CONFIGURED)],
        Held {
            gh_token: Some("ghp_intheenvironment"),
            ..Held::default()
        },
    );
    written("onboarding-git.json", &prefill(&app).await);

    let (dir, pool) = ready().await;
    let app = served(dir.path(), &pool, &[]);
    written("onboarding-git-nothing.json", &prefill(&app).await);
}

/// The `claude` on this machine's `PATH` made into the install the vendor's own
/// installer leaves: a link into the versions directory under the home.
///
/// Which is the install this whole feature is about, and the one shape a row
/// has two paths to draw — the name a session resolved and the version it is
/// really running. Made after the router is stood up, the rows being probed at
/// every read rather than at startup.
fn natively_installed(dir: &Path) {
    let version = home(dir).join(".local/share/claude/versions/0.0.0");
    std::fs::create_dir_all(&version).unwrap();
    program(&version.join("claude"), A_PROGRAM);

    let name = bin(dir).join("claude");
    std::fs::remove_file(&name).unwrap();
    std::os::unix::fs::symlink(version.join("claude"), name).unwrap();
}

/// One fixture that has no path in it, written as it stands.
///
/// The git step's read is three values off a `git config` and an environment
/// variable — nothing about it is this box's — so there is nothing to write
/// back out the way [`write`] writes a home.
fn written<T: serde::Serialize>(name: &str, payload: &T) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let pretty = serde_json::to_string_pretty(payload).unwrap() + "\n";

    std::fs::write(dir.join(name), pretty).unwrap();
}

/// What a home reads as in a fixture, whoever ran the suite.
const A_HOME: &str = "/home/you";

/// And what the rest of the directory this run was given reads as: the `PATH`
/// entry its programs are in, and the one nothing composes in beside it.
const A_MACHINE: &str = "/machine";

/// One fixture, as the server would have written it — with the one thing in it
/// that is this run's own written back out as a home anybody would recognise.
///
/// A detected account is a set of real paths and so is a program a row was
/// found at: both have to be on disk, so both are under a temporary directory
/// whose name is different every run. What the viewer's tests are drawn over is
/// the shape of a reading rather than this box's paths, so the home is written
/// as [`A_HOME`] and whatever else this run made is written under
/// [`A_MACHINE`] — which is a reading anybody would recognise as their own.
///
/// The home first, it being inside the directory the run was given: what is
/// left for the second pass is the `PATH` entry beside it.
fn write(name: &str, reading: &OnboardingView, ran_in: &Path) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    std::fs::create_dir_all(&dir).unwrap();

    let pretty = serde_json::to_string_pretty(reading).unwrap();
    let pretty = pretty.replace(&home(ran_in).to_string_lossy().into_owned(), A_HOME);
    let pretty = pretty.replace(&ran_in.to_string_lossy().into_owned(), A_MACHINE) + "\n";

    std::fs::write(dir.join(name), pretty).unwrap();
}
