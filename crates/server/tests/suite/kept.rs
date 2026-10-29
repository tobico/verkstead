//! A kept Verkstead — an **Annex**, whose `config.yaml` names its **Keeper**
//! (ADR-0024) — and the wizard it never raises.
//!
//! An Annex's workbench is a viewer nobody opens: the human drives its work
//! from the Keeper's. So Onboarding Mode, which only ever redirects that
//! viewer, never comes on over one — a mode nothing would ever clear being
//! worse than none — and the viewer opened on it lands on the compose page,
//! which is what a reading with the mode off draws (`web/src/App.tsx`).
//!
//! The machine is stated rather than probed, for the reason the onboarding
//! suite states one: what this box has installed is not a thing a test of the
//! mode may stand on. Here it has nothing at all — no `PATH`, no home, no
//! Profile and no author — which is the machine furthest from the objective,
//! and so the one where the mode would be on if anything could put it there.
//! Nothing it is handed is a script, so this suite runs on every platform.

use std::ffi::OsString;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use verkstead_render::OnboardingView;
use verkstead_server::onboarding::Machine;
use verkstead_server::platform::{Environment, Platform};
use verkstead_server::{open_database, router_onboarding};

/// A `kept_by` as the Keeper writes one: its Device Id, and its certificate's
/// fingerprint in the spelling a fingerprint is printed in.
const KEPT: &str = "kept_by:\n  device: 86f1933fecb070cbee865fbb84819d14\n  \
                    fingerprint: \"3F:0A:9C:4B:21:D7:E0:66\"\n";

/// A server over `dir`, on a machine with nothing on it.
///
/// Whatever `config.yaml` is to say is written before this is called: the
/// verdict is settled once, at startup.
async fn served(dir: &Path) -> Router {
    let pool = open_database(&dir.join("verkstead.db")).await.unwrap();

    let machine = Machine::stated(
        Platform::Linux,
        OsString::new(),
        OsString::new(),
        None,
        None,
        &Environment::default(),
    );

    router_onboarding(pool, dir.to_owned(), machine)
}

/// What the viewer reads before it draws anything.
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

    serde_json::from_slice(&body).expect("the reading the viewer is drawn from")
}

/// A kept server with nothing set up is not in Onboarding Mode, and a server
/// over the same nothing without `kept_by` is — so it is the Keeper's name in
/// the file that took the wizard away, and not the machine.
#[tokio::test]
async fn a_kept_server_with_nothing_set_up_comes_up_without_the_wizard() {
    let unkept = tempfile::tempdir().unwrap();
    let not_kept = reading(&served(unkept.path()).await).await;

    assert!(
        not_kept.mode,
        "with nothing set up and nobody keeping it, a Verkstead is the wizard as it \
         always was",
    );

    let kept = tempfile::tempdir().unwrap();
    std::fs::write(kept.path().join("config.yaml"), KEPT).unwrap();

    let annex = reading(&served(kept.path()).await).await;

    assert!(
        !annex.mode,
        "an Annex's viewer is one nobody opens, and a mode nothing will clear is worse \
         than none — so it opens on the compose page",
    );
    assert!(
        !annex.steps.dependencies && !annex.steps.accounts && !annex.steps.git,
        "and the steps still say what they say: it is the mode that is off, not the \
         objective that is met — {:?}",
        annex.steps,
    );
}
