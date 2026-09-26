//! What the Remote access pane reads: whether this machine has a Tailscale,
//! whether it is up, what it is called on the tailnet and whether the tailnet
//! name is already in front of the workbench — and the one thing on it that is
//! pressed, which is putting that name there and taking it off again.
//!
//! Every one of those is a fact about the machine the tests happen to be running
//! on, which is the one machine this suite must not be asking about: a pane with
//! four things to say needs four machines to say them about. So `tailscale` is a
//! shell script here, the way `gh` is in the settings suite — the server runs
//! the program it is given and reads what it printed, and what it is given is a
//! script that prints what one of those four machines would.
//!
//! The press needs a fifth, and a different kind of one: a machine that answers
//! differently after it has been pressed. That one keeps its serve in a file and
//! reads it back out — see [`switchable`] — because what makes the switch worth
//! pressing is precisely that the read afterwards says something else. It keeps
//! the operator grant in a second file, so a `tailscale set --operator=…` run
//! against it is a refusal lifted rather than a command that goes nowhere.
//!
//! Which is what the escalation is tested through. A press refused for want of
//! that grant is answered two ways — the line shown, or the line taken through
//! the platform's own password dialog — and which of them is a fact about what
//! started the server. So the dialog is a handle here too, the way `tailscale`
//! is: see [`Dialog`], which is one somebody answers and one somebody dismisses.
//!
//! And the last of it is not about Tailscale at all: the login link, which is
//! the served address with the Workbench Key on the end of it, and **Reset
//! key**, which is the press that changes what that end says. Those two need a
//! sixth server — one standing behind its own gate, see [`app_keyed`] — because
//! what says a key was re-issued is the old one being refused, and every other
//! server here answers whoever asks.
//!
//! Unix only, for that reason and no other: the cases are shapes of stdout
//! rather than anything about a platform, and a Windows run would be a second
//! machine reading the same JSON.
#![cfg(unix)]

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::{RemoteView, ServePress};
use verkstead_server::key::WorkbenchKey;
use verkstead_server::remote::{Elevate, Raised, Tailscale};
use verkstead_server::{open_database, router_reading_tailscale, router_reading_tailscale_keyed};

/// The port the workbench is served on in this suite, and so the port a serve
/// has to be proxying to for the pane to call it this workbench's.
const PORT: u16 = 8422;

/// The Workbench Key every server here holds, stated rather than invented for
/// the reason [`WHO`] is: the login link the pane draws is the served address
/// with this on the end of it, and a suite whose expected link came out of
/// thirty-two random bytes would be a suite pinning nothing.
const KEY: &str = "a-stated-workbench-key";

/// And the link that makes, which is what a served machine reads back.
fn login_link() -> String {
    format!("https://workbench.tailnet-name.ts.net/?key={KEY}")
}

/// A machine on a tailnet, serving the workbench: `tailscale status` says the
/// daemon is running and names this node, and `tailscale serve status` says
/// HTTPS on that name is proxied to the workbench's port.
const SERVING: &str = r#"
case "$1" in
  status) printf '%s' '{"BackendState":"Running","Self":{"DNSName":"workbench.tailnet-name.ts.net."}}' ;;
  serve) printf '%s' '{"TCP":{"443":{"HTTPS":true}},"Web":{"workbench.tailnet-name.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:8422"}}}}}' ;;
esac
"#;

/// The same machine with no serve on it, which is what `tailscale serve status`
/// prints where nobody has ever run one.
const NOT_SERVING: &str = r#"
case "$1" in
  status) printf '%s' '{"BackendState":"Running","Self":{"DNSName":"workbench.tailnet-name.ts.net."}}' ;;
  serve) printf '%s' 'null' ;;
esac
"#;

/// A machine whose daemon is not running, which is what both commands do about
/// it: a non-zero exit, and the line naming the service to start.
const NO_DAEMON: &str = r#"
echo "failed to connect to local tailscaled; it doesn't appear to be running (sudo systemctl start tailscaled ?)" >&2
exit 1
"#;

/// And one whose daemon is running and which has joined no tailnet.
const NOT_LOGGED_IN: &str = r#"printf '%s' '{"BackendState":"NeedsLogin","Self":{}}'"#;

/// And one whose `tailscale` answers something this build has never seen.
const UNRECOGNISED: &str = r#"printf '%s' 'this is not the JSON you are looking for'"#;

/// A server reading a machine whose `tailscale` is `script`.
async fn app_reading(script: &str) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let tailscale = Tailscale::running(
        vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            script.to_owned(),
            // `sh -c` gives `$0` the script's own name, so what Verkstead passes
            // lands in `$1` onwards.
            "tailscale".to_owned(),
        ],
        PORT,
    )
    // Keyed, because the login link the pane draws is the served address with
    // the Workbench Key on the end of it — see [`KEY`].
    .keyed(WorkbenchKey::stated(dir.path(), KEY).unwrap());

    (dir, router_reading_tailscale(pool, tailscale))
}

/// And a server on a machine with no `tailscale` at all, which is a program
/// that is not there rather than one that answers badly.
async fn app_without_tailscale() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let tailscale = Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
        .keyed(WorkbenchKey::stated(dir.path(), KEY).unwrap());

    (dir, router_reading_tailscale(pool, tailscale))
}

async fn remote(app: &Router) -> RemoteView {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/remote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// A machine with nothing to run reads as a machine with nothing installed —
/// which is the one state whose answer is somewhere else entirely, and so the
/// one the pane has an install pointer for.
#[tokio::test]
async fn a_machine_with_no_tailscale_says_so() {
    let (_dir, app) = app_without_tailscale().await;

    assert_eq!(remote(&app).await, RemoteView::Absent);
}

/// A machine that has `tailscale` and no daemon answering is its own third
/// thing rather than a serve that is off: nothing about it says whether this
/// machine would be served, and turning a switch on would not be the fix.
#[tokio::test]
async fn a_daemon_that_is_not_answering_is_not_a_serve_that_is_off() {
    let (_dir, app) = app_reading(NO_DAEMON).await;

    let RemoteView::Down { trouble } = remote(&app).await else {
        panic!("a machine whose daemon is down should read as down");
    };

    // In the machine's own words, because they are the useful half: the line
    // tailscale prints is the one naming the service to start.
    assert!(
        trouble.contains("tailscaled"),
        "the daemon's own complaint should come back: {trouble}"
    );
}

/// And so is one whose daemon is up and which has joined no tailnet: there is no
/// node to name and no address to serve on, which is the same thing to do about
/// it and a different sentence to say.
#[tokio::test]
async fn a_machine_that_has_joined_no_tailnet_reads_as_down_too() {
    let (_dir, app) = app_reading(NOT_LOGGED_IN).await;

    let RemoteView::Down { trouble } = remote(&app).await else {
        panic!("a machine that is not on a tailnet should read as down");
    };

    assert!(
        trouble.contains("NeedsLogin"),
        "what the daemon reports should come back: {trouble}"
    );
}

/// A machine that is up names itself, and says where the workbench answers on
/// the tailnet.
#[tokio::test]
async fn a_machine_that_is_up_and_serving_names_the_node_and_the_address() {
    let (_dir, app) = app_reading(SERVING).await;

    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::On {
                address: "https://workbench.tailnet-name.ts.net".to_owned(),
            },
            link: Some(login_link()),
        }
    );
}

/// And one that is up with no serve on it says so, which is the state the
/// switch beside it turns on.
#[tokio::test]
async fn a_machine_that_is_up_and_serving_nothing_reads_as_off() {
    let (_dir, app) = app_reading(NOT_SERVING).await;

    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::Off,
            link: None,
        }
    );
}

/// An answer this build cannot read says exactly that. Tailscale is whatever
/// the host has and the JSON it prints is documented as subject to change, so a
/// shape nobody here has seen must not be folded into the nearest state with
/// room for it.
#[tokio::test]
async fn an_answer_this_build_cannot_read_is_neither_up_nor_down() {
    let (_dir, app) = app_reading(UNRECOGNISED).await;

    assert!(matches!(remote(&app).await, RemoteView::Unreadable { .. }));
}

/// A machine on a tailnet whose serve is whatever this suite last did to it.
///
/// The four cases above are machines that answer the same thing however often
/// they are asked, which is all a read needs. A press is the other half: what
/// makes it worth pressing is that the next read says something different, so
/// this `tailscale` keeps its serve in a file and both halves go through it.
///
/// `--bg` is refused until the operator file is there, which is what the grant
/// is here: `tailscale serve` from a process that is neither root nor the
/// tailnet's operator is denied by the daemon, and the file stands for the
/// `sudo tailscale set --operator=…` somebody runs in a terminal to lift it.
fn switchable(state: &std::path::Path) -> String {
    let state = state.display();

    format!(
        r#"
case "$1" in
  status) printf '%s' '{{"BackendState":"Running","Self":{{"DNSName":"workbench.tailnet-name.ts.net."}}}}' ;;
  serve)
    case "$2" in
      status)
        if [ -e "{state}/serving" ]; then
          printf '%s' '{{"TCP":{{"443":{{"HTTPS":true}}}},"Web":{{"workbench.tailnet-name.ts.net:443":{{"Handlers":{{"/":{{"Proxy":"http://127.0.0.1:8422"}}}}}}}}}}'
        else
          printf '%s' 'null'
        fi ;;
      --bg)
        if [ -e "{state}/operator" ]; then
          : > "{state}/serving"
        else
          echo "Access denied: serve config denied" >&2
          exit 1
        fi ;;
      --https=443) rm -f "{state}/serving" ;;
    esac ;;
  set)
    case "$2" in
      --operator=*) : > "{state}/operator" ;;
    esac ;;
esac
"#
    )
}

/// The user this suite's server runs as, so the grant it hands back is a line
/// this file can write down. Whoever is running `cargo test` is nobody's
/// business here — see [`Tailscale::as_user`].
const WHO: &str = "ada";

/// A server whose `tailscale` is the switchable machine above, with the state
/// directory that machine keeps its serve in — and with no way to ask for the
/// operator grant, which is the daemon and every server but the desktop app's.
async fn app_pressing() -> (tempfile::TempDir, Router) {
    app_pressing_through(None).await
}

/// And the same with `escalation` where the desktop app's password dialog goes.
async fn app_pressing_through(escalation: Option<Arc<dyn Elevate>>) -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let script = switchable(dir.path());

    app_asking(dir, script, escalation).await
}

/// A machine that takes the operator grant and refuses the serve anyway.
///
/// Which is not a contradiction: the grant is one command and the serve is
/// another, and Tailscale has more than one reason to refuse the second. What it
/// is here for is the press that has already asked — the dialog was answered,
/// and there is still nothing to do but show the line.
const STUBBORN: &str = r#"
case "$1" in
  status) printf '%s' '{"BackendState":"Running","Self":{"DNSName":"workbench.tailnet-name.ts.net."}}' ;;
  serve)
    case "$2" in
      status) printf '%s' 'null' ;;
      *) echo "Access denied: serve config denied" >&2; exit 1 ;;
    esac ;;
esac
"#;

/// A server whose `tailscale` is `script`, running as [`WHO`] and asking for the
/// operator grant through `escalation` where it was handed one.
async fn app_asking(
    dir: tempfile::TempDir,
    script: String,
    escalation: Option<Arc<dyn Elevate>>,
) -> (tempfile::TempDir, Router) {
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let tailscale = Tailscale::running(
        vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            script,
            // `sh -c` gives `$0` the script's own name, so what Verkstead passes
            // lands in `$1` onwards.
            "tailscale".to_owned(),
        ],
        PORT,
    )
    .as_user(WHO.to_owned())
    .keyed(WorkbenchKey::stated(dir.path(), KEY).unwrap());

    let tailscale = match escalation {
        Some(escalation) => tailscale.escalating(escalation),
        None => tailscale,
    };

    (dir, router_reading_tailscale(pool, tailscale))
}

/// The platform's own password dialog, as a suite has one: what it was asked to
/// run, and whether anybody answered it.
///
/// The real one is `pkexec`, `osascript` or a UAC prompt, and which of those it
/// is is `verkstead_server::elevate`'s business — its own unit tests are what
/// assert the three commands. What this
/// side of the seam has to get right is the two answers: one that runs the
/// command, which is somebody typing their password, and one that runs nothing,
/// which is somebody pressing Cancel.
#[derive(Debug)]
struct Dialog {
    /// Whether it is answered.
    answered: bool,

    /// And what crossed the seam, kept for the tests to read: what goes through
    /// is a command to run rather than the line the pane shows, and the `sudo`
    /// on the front of that line is exactly what must not be here.
    asked: Mutex<Vec<Vec<String>>>,
}

impl Dialog {
    /// One somebody types their password into.
    fn answered() -> Arc<Dialog> {
        Arc::new(Dialog {
            answered: true,
            asked: Mutex::new(Vec::new()),
        })
    }

    /// And one somebody dismisses.
    fn dismissed() -> Arc<Dialog> {
        Arc::new(Dialog {
            answered: false,
            asked: Mutex::new(Vec::new()),
        })
    }

    /// What it has been asked to raise, in the order it was asked.
    fn asked(&self) -> Vec<Vec<String>> {
        self.asked.lock().unwrap().clone()
    }
}

impl Elevate for Dialog {
    fn raise(&self, command: &[String]) -> Raised {
        self.asked.lock().unwrap().push(command.to_vec());

        if !self.answered {
            return Raised::Refused {
                why: "User canceled.".to_owned(),
            };
        }

        // Answered, so the command runs — with the privilege it wanted, which
        // this machine's `tailscale` stands in for by taking the grant from
        // whoever asks.
        let (program, arguments) = command.split_first().unwrap();
        let told = std::process::Command::new(program)
            .args(arguments)
            .output()
            .unwrap();

        match told.status.success() {
            true => Raised::Done,
            false => Raised::Refused {
                why: String::from_utf8_lossy(&told.stderr).into_owned(),
            },
        }
    }
}

/// Press the switch, and read what came of it.
async fn press(app: &Router, on: bool) -> ServePress {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ui/remote/serve")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"on":{on}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// The grant, run in a terminal — which here is the file the machine above
/// looks for.
fn granted(dir: &tempfile::TempDir) {
    std::fs::write(dir.path().join("operator"), "").unwrap();
}

/// Serving is what makes the address readable, and the switch's position comes
/// off the machine rather than off what was pressed: what a press answers with
/// is the reading, made again.
#[tokio::test]
async fn a_serve_switched_on_makes_the_address_readable() {
    let (dir, app) = app_pressing().await;
    granted(&dir);

    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::Off,
            link: None,
        },
        "nothing is served until the switch is pressed"
    );

    assert_eq!(
        press(&app, true).await,
        ServePress::Done {
            reading: RemoteView::Up {
                node: "workbench.tailnet-name.ts.net".to_owned(),
                serve: verkstead_render::ServeView::On {
                    address: "https://workbench.tailnet-name.ts.net".to_owned(),
                },
                link: Some(login_link()),
            },
        }
    );

    // And the read the pane makes on its own says the same, which is the half
    // that says the press changed the machine rather than only the answer.
    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::On {
                address: "https://workbench.tailnet-name.ts.net".to_owned(),
            },
            link: Some(login_link()),
        }
    );
}

/// And off takes the address away again — including a serve this workbench
/// never set up, because the position was read off the machine and the switch
/// turns off whatever it read.
#[tokio::test]
async fn a_serve_switched_off_takes_the_address_away() {
    let (dir, app) = app_pressing().await;

    // Set up by hand rather than by a press: the file this machine keeps its
    // serve in, written without anybody having been to the pane.
    std::fs::write(dir.path().join("serving"), "").unwrap();

    let RemoteView::Up { serve, .. } = remote(&app).await else {
        panic!("the machine should be up");
    };
    assert!(
        matches!(serve, verkstead_render::ServeView::On { .. }),
        "a serve set up by hand reads as on: {serve:?}"
    );

    assert_eq!(
        press(&app, false).await,
        ServePress::Done {
            reading: RemoteView::Up {
                node: "workbench.tailnet-name.ts.net".to_owned(),
                serve: verkstead_render::ServeView::Off,
                link: None,
            },
        }
    );
}

/// A serve Tailscale will not take from this user comes back as the line that
/// makes it take one — for this machine's own user, exactly as it is to be
/// typed — and a press after it has been run serves.
#[tokio::test]
async fn a_refused_serve_reads_back_the_grant_and_the_next_press_serves() {
    let (dir, app) = app_pressing().await;

    let ServePress::Ungranted { grant, trouble } = press(&app, true).await else {
        panic!("a serve refused for want of the operator grant should say so");
    };

    assert_eq!(grant, format!("sudo tailscale set --operator={WHO}"));

    // And what the machine said with it, because the grant is this build's
    // reading of a refusal and the refusal itself is the machine's.
    assert!(
        trouble.contains("Access denied"),
        "the refusal's own words should come back: {trouble}"
    );

    // Nothing was served, so the switch is still off.
    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::Off,
            link: None,
        }
    );

    // The line run in a terminal, and the same press again — which is the whole
    // of what a re-try is.
    granted(&dir);

    assert_eq!(
        press(&app, true).await,
        ServePress::Done {
            reading: RemoteView::Up {
                node: "workbench.tailnet-name.ts.net".to_owned(),
                serve: verkstead_render::ServeView::On {
                    address: "https://workbench.tailnet-name.ts.net".to_owned(),
                },
                link: Some(login_link()),
            },
        }
    );
}

/// A press on a machine with no `tailscale` at all fails as what it is, rather
/// than as a grant that would not help: there is no command to run and nothing
/// to grant it to.
#[tokio::test]
async fn a_press_with_no_tailscale_is_trouble_rather_than_a_grant() {
    let (_dir, app) = app_without_tailscale().await;

    assert!(matches!(
        press(&app, true).await,
        ServePress::Trouble { .. }
    ));
}

/// The desktop app's arm: a press refused for want of the operator grant raises
/// the platform's own asking, and the press that follows the grant serves.
///
/// One press from the human, therefore, where the daemon's arm takes two and a
/// terminal in between.
#[tokio::test]
async fn a_dialog_somebody_answered_takes_the_grant_and_serves() {
    let dialog = Dialog::answered();
    let (_dir, app) = app_pressing_through(Some(dialog.clone())).await;

    assert_eq!(
        press(&app, true).await,
        ServePress::Done {
            reading: RemoteView::Up {
                node: "workbench.tailnet-name.ts.net".to_owned(),
                serve: verkstead_render::ServeView::On {
                    address: "https://workbench.tailnet-name.ts.net".to_owned(),
                },
                link: Some(login_link()),
            },
        }
    );

    // And what was raised is the grant as a command — the line the pane shows,
    // without the `sudo`: what raises the privilege is the dialog rather than a
    // word in front of the command.
    let asked = dialog.asked();

    assert_eq!(asked.len(), 1, "the dialog is raised once: {asked:?}");
    assert_eq!(
        asked[0][asked[0].len() - 2..],
        ["set".to_owned(), format!("--operator={WHO}")],
        "the grant is what crosses: {asked:?}",
    );
    assert!(
        !asked[0].iter().any(|word| word == "sudo"),
        "nothing crosses with a sudo on it: {asked:?}",
    );
}

/// And a dialog somebody dismissed is not a failure to report as one: the switch
/// stays off, which is the truth about the machine, and the line stays on the
/// pane for whoever would rather type it.
#[tokio::test]
async fn a_dialog_somebody_dismissed_leaves_the_switch_off_and_the_line_shown() {
    let dialog = Dialog::dismissed();
    let (_dir, app) = app_pressing_through(Some(dialog.clone())).await;

    let ServePress::Ungranted { grant, trouble } = press(&app, true).await else {
        panic!("a dismissed dialog leaves the press refused");
    };

    assert_eq!(grant, format!("sudo tailscale set --operator={WHO}"));
    assert!(
        trouble.contains("Access denied"),
        "what Tailscale said is what is shown under the line: {trouble}"
    );

    // It was asked, which is the half that says the app tried rather than the
    // half that says it worked.
    assert_eq!(dialog.asked().len(), 1);

    // And nothing was served, so the switch the pane draws is still off.
    assert_eq!(
        remote(&app).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::Off,
            link: None,
        }
    );
}

/// A dialog answered on a machine that refuses the serve anyway leaves the line
/// standing: the grant was taken and did not help, which is still a command
/// somebody can run where they can read what the machine says back.
#[tokio::test]
async fn a_serve_still_refused_after_the_grant_reads_back_the_line() {
    let dialog = Dialog::answered();
    let dir = tempfile::tempdir().unwrap();
    let (_dir, app) = app_asking(dir, STUBBORN.to_owned(), Some(dialog.clone())).await;

    let ServePress::Ungranted { grant, trouble } = press(&app, true).await else {
        panic!("a serve refused after the grant is still a serve that was refused");
    };

    assert_eq!(grant, format!("sudo tailscale set --operator={WHO}"));
    assert!(
        trouble.contains("Access denied"),
        "what the machine said the second time: {trouble}"
    );

    // Once per press, rather than once per refusal: a dialog raised again over
    // the same press would be the app asking for something it has just been
    // given.
    assert_eq!(dialog.asked().len(), 1);
}

/// A server on the serving machine above, standing behind its own key: what the
/// half of this suite about **Reset key** stands up.
///
/// Gated where every other server here is open, because the press is about the
/// gate: what says a key was re-issued is the old one being refused, and a
/// router nothing is refused by could not say it. Requests to it carry the
/// cookie a browser would — see [`with_cookie`].
async fn app_keyed() -> (tempfile::TempDir, Router, WorkbenchKey) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    let key = WorkbenchKey::stated(dir.path(), KEY).unwrap();

    let tailscale = Tailscale::running(
        vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            SERVING.to_owned(),
            "tailscale".to_owned(),
        ],
        PORT,
    )
    .keyed(key.clone());

    let app = router_reading_tailscale_keyed(pool, tailscale, key.clone());

    (dir, app, key)
}

/// Ask for something as a browser holding `cookie` would.
async fn with_cookie(
    app: &Router,
    method: &str,
    path: &str,
    cookie: &str,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// **Reset key** re-issues the secret, and the link the pane draws redraws on
/// the new one.
///
/// Which is the whole of what the press is for: the QR code and the copyable
/// link beside it are that field, so a link that came back unchanged would be a
/// pane drawing a code for a key that no longer opens anything.
#[tokio::test]
async fn resetting_the_key_redraws_the_link_on_a_new_one() {
    let (_dir, app, key) = app_keyed().await;

    let before = key.cookie();

    assert_eq!(
        remote_as(&app, &before).await,
        RemoteView::Up {
            node: "workbench.tailnet-name.ts.net".to_owned(),
            serve: verkstead_render::ServeView::On {
                address: "https://workbench.tailnet-name.ts.net".to_owned(),
            },
            link: Some(login_link()),
        }
    );

    let response = with_cookie(&app, "POST", "/api/ui/remote/key", &before).await;

    assert_eq!(response.status(), StatusCode::OK);

    // The browser that pressed it is let back in by the answer. A reset made
    // from the phone on the tailnet is a reset made from the only device that
    // could reach this server at all, and one that logged that device out with
    // the rest would lock somebody out of their own workbench.
    let admitted = response
        .headers()
        .get("set-cookie")
        .expect("the browser that pressed it is handed the key it just made")
        .to_str()
        .unwrap()
        .to_owned();

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let redrawn: RemoteView = serde_json::from_slice(&bytes).unwrap();

    let RemoteView::Up { link, .. } = redrawn else {
        panic!("the press answers with the machine read again");
    };
    let link = link.expect("a served machine reads back a login link");

    assert_ne!(link, login_link(), "the link is on the new key");
    assert!(
        link.starts_with("https://workbench.tailnet-name.ts.net/?key="),
        "and it is still this machine's address: {link}"
    );

    // And the address the QR now carries opens a logged-in workbench, which is
    // the handshake the gate does with it.
    let after = admitted
        .split(';')
        .next()
        .expect("a Set-Cookie starts with the pair it sets")
        .to_owned();

    assert_eq!(
        format!("workbench_key={}", link.rsplit_once('=').unwrap().1),
        after,
        "the cookie the answer sets is the key the link hands over"
    );
}

/// And a device holding the previous key gets 401 on its next request, which is
/// the half that makes a lost phone recoverable.
#[tokio::test]
async fn a_device_holding_the_previous_key_is_refused() {
    let (_dir, app, key) = app_keyed().await;

    let phone = key.cookie();

    // It was let in a moment ago, which is what makes the refusal below about
    // the reset rather than about the cookie having always been wrong.
    assert_eq!(
        with_cookie(&app, "GET", "/api/ui/remote", &phone)
            .await
            .status(),
        StatusCode::OK
    );

    assert_eq!(
        with_cookie(&app, "POST", "/api/ui/remote/key", &phone)
            .await
            .status(),
        StatusCode::OK
    );

    assert_eq!(
        with_cookie(&app, "GET", "/api/ui/remote", &phone)
            .await
            .status(),
        StatusCode::UNAUTHORIZED,
        "everything holding the old key is out"
    );
}

/// The reading, asked for as a browser holding `cookie`.
async fn remote_as(app: &Router, cookie: &str) -> RemoteView {
    let response = with_cookie(app, "GET", "/api/ui/remote", cookie).await;

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
