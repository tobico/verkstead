//! The push relay: a member's news reaching the phones its cluster-mates push
//! to, with the device leading the title (ADR-0020, *The opened device relays*).
//!
//! **Two Verksteads, a real dial and a real push service.** `tests/merging.rs` is
//! the sidebar merged out of a link and `tests/freshness.rs` is the Nudge stream
//! held down one; this is the one thing in a cluster that goes the *other* way —
//! a device telling its members something rather than being asked. So nothing
//! here hands A a title by the back door: B says the work is done, B's own dial
//! carries the sentence to A over the Peer Listener and behind the Member Gate,
//! and what every assertion reads is a push that a real push service took, read
//! back with the keys the phone subscribed with. `tests/push_delivery.rs` is the
//! same service stood up for the Question Set half of the same module.
//!
//! **The one thing said by hand is the news itself.** Every arm of Verkstead's
//! news is written by a session running — a stop, a session gone idle, an account
//! out of window, a roadmap moving on — and the one a suite can say by itself is
//! a Conversation reaching Done, which is
//! [`verkstead_server::push::the_work_is_done`]. It is the real entry point and
//! not a stand-in: there is one place a piece of news is sent from, so which arm
//! says it decides the sentence and nothing else. What each arm's sentence reads
//! like is asked of `News::title` in the server's own unit tests.

#![cfg(unix)]

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Path as UrlPath, State};
use axum::http::{Request, StatusCode, header};
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use http_body_util::BodyExt;
use p256::SecretKey;
use p256::elliptic_curve::rand_core::OsRng;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::RelayedNews;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::dialling::Peers;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, push, routers_answering_devices_telling, store};
use verkstead_store::{Linking, record_member};
use web_push_native::Auth;

/// The device the phone is installed from, which is where a member's news has to
/// arrive.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device whose work the news is about.
const B: &str = "0011223344556677889900aabbccddee";

/// And a third, for the one thing a cluster of three can be asked: whether news
/// is passed on.
const C: &str = "ffeeddccbbaa00998877665544332211";

/// And a fourth that is nobody's member, for the question about the gate.
const STRANGER: &str = "99887766554433221100ffeeddccbbaa";

/// What B is written down as on A: the name and the OS word A holds against B's
/// row, which is what a title about B's work leads with.
///
/// Stated rather than read off a machine, because they are the membership row's —
/// a member's name is what it last said about itself at an exchange. And
/// deliberately not this box's own hostname: every device in this suite is one
/// machine, so a title that took the hub's own name for a member's would read
/// right for the wrong reason.
const B_MACHINE: &str = "the-laptop";
const B_OS: &str = "macOS 15.1";

/// And what A and C are written down as, for the dials that go the other way.
const A_MACHINE: &str = "the-desktop";
const C_MACHINE: &str = "the-tablet";

/// The Repo every Conversation here is against, and the branch the one piece of
/// news is about — which is what a title about a piece of work is read by.
const REPO: &str = "verkstead";
const BRANCH: &str = "pwa-and-push";

/// The port the workbench is taken to be on, which nothing here asks about.
const PORT: u16 = 8422;

/// How long one address has to answer here, rather than the two seconds a running
/// server gives one. Spent only where a dial is meant to reach nobody.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test will wait for a push to arrive: generous, because it is only
/// ever paid in full when the assertion is about to fail.
const WAITING: Duration = Duration::from_secs(10);

/// And how often it looks while it waits.
const LOOKING: Duration = Duration::from_millis(20);

/// How long to keep watching after everything expected has arrived, to catch a
/// push that should never have been sent at all.
const SETTLING: Duration = Duration::from_millis(400);

/// A machine with no Tailscale on it: a program that is not there, which is what
/// having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], PORT)
}

// ---------------------------------------------------------------------------
// A phone, and the push service it is reached through
// ---------------------------------------------------------------------------

/// One push, as the push service received it.
#[derive(Debug, Clone)]
struct Received {
    /// The last segment of the path it was posted to, which is the phone it was
    /// meant for.
    phone: String,
    body: Vec<u8>,
}

/// A phone as its browser would have described it, plus the private half the
/// browser would have kept — which is what lets a test read a push back.
struct Phone {
    name: String,
    endpoint: String,
    secret: SecretKey,
    auth: Vec<u8>,
}

impl Phone {
    fn new(service: &str, name: &str) -> Phone {
        // Deterministic rather than random, so a failure names the same phone
        // twice running. Sixteen bytes because that is what the auth secret is.
        let auth: Vec<u8> = name.bytes().cycle().take(16).collect();

        Phone {
            name: name.to_owned(),
            endpoint: format!("{service}/{name}"),
            secret: SecretKey::random(&mut OsRng),
            auth,
        }
    }

    /// The public half, in the encoding `PushManager.subscribe` hands back.
    fn p256dh(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.secret.public_key().to_encoded_point(false).as_bytes())
    }

    fn auth(&self) -> String {
        URL_SAFE_NO_PAD.encode(&self.auth)
    }

    /// What this phone was shown, as its service worker would read it — see
    /// `assets/sw.js`, which reads exactly these three.
    fn read(&self, push: &Received) -> Notice {
        let plain = web_push_native::decrypt(
            push.body.clone(),
            &self.secret,
            &Auth::clone_from_slice(&self.auth),
        )
        .expect("a push for this phone has to decrypt with this phone's keys");

        serde_json::from_slice(&plain).expect("the notice has to be JSON")
    }
}

/// What the service worker draws a notification from: the sentence, the
/// repository under it, and where a tap goes.
#[derive(Debug, serde::Deserialize)]
struct Notice {
    title: String,
    path: String,
    #[serde(default)]
    project: Option<String>,
}

/// A push service on a loopback port, and the pushes it has taken. It accepts
/// everything: what is being asked here is what was sent and to whom.
async fn push_service() -> (String, Arc<Mutex<Vec<Received>>>) {
    let received = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/{phone}", post(take))
        .with_state(received.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{address}"), received)
}

async fn take(
    State(received): State<Arc<Mutex<Vec<Received>>>>,
    UrlPath(phone): UrlPath<String>,
    body: Bytes,
) -> StatusCode {
    received.lock().unwrap().push(Received {
        phone,
        body: body.to_vec(),
    });

    StatusCode::CREATED
}

/// Wait until `phone` has been pushed to, and hand over what it was shown.
async fn shown(received: &Arc<Mutex<Vec<Received>>>, phone: &Phone) -> Notice {
    let waited = tokio::time::timeout(WAITING, async {
        loop {
            let taken = received.lock().unwrap().clone();

            if let Some(push) = taken.iter().find(|push| push.phone == phone.name) {
                return phone.read(push);
            }

            drop(taken);
            tokio::time::sleep(LOOKING).await;
        }
    })
    .await;

    waited.unwrap_or_else(|_| panic!("nothing was ever pushed to {}", phone.name))
}

/// Give anything still on its way time to arrive, for an assertion about a push
/// that should never be sent.
async fn settle() {
    tokio::time::sleep(SETTLING).await;
}

/// How many pushes a phone has been sent.
fn count(received: &Arc<Mutex<Vec<Received>>>, phone: &Phone) -> usize {
    received
        .lock()
        .unwrap()
        .iter()
        .filter(|push| push.phone == phone.name)
        .count()
}

/// An address nothing is listening on: bound to claim a free port, then dropped.
/// Which is a machine that is switched off.
async fn nowhere() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("127.0.0.1:{}", address.port())
}

// ---------------------------------------------------------------------------
// One Verkstead
// ---------------------------------------------------------------------------

/// One Verkstead: its store, its identity, and both of the routers standing over
/// its one state — with its cluster left where a notification can reach it, which
/// is what a running server does as it comes up.
struct Verkstead {
    device: Device,
    pool: SqlitePool,

    /// The workbench its own browser asks, which is where a phone subscribes.
    workbench: Router,

    /// Where the other devices dial it, on a port the machine picked.
    address: SocketAddr,

    /// Held for the length of the test: the identity and the database live in it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    async fn answering(id: &str) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());
        let nudges = Nudges::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let cluster = Devices::of(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
        )
        .waiting(PATIENCE);

        let Routers {
            workbench,
            over_the_link,
        } = routers_answering_devices_telling(pool.clone(), cluster.clone(), nudges.clone());

        // Which is the one line of a start this suite is about: the cluster left
        // where a piece of news can find it, so that what a push tells this
        // device's own phones it also tells its members.
        push::hold_the_cluster(&pool, &cluster);

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading,
            members,
            Joins::none(),
            nudges,
            over_the_link,
        )));

        Verkstead {
            device,
            pool,
            workbench,
            address,
            _dir: dir,
        }
    }

    /// The one address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Write `other` down as one of this device's members, at `addresses`, under
    /// the name it is to be shown as.
    async fn linked_to(&self, other: &Device, name: &str, os: &str, addresses: Vec<String>) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: name.to_owned(),
                os: os.to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
            },
        )
        .await
        .unwrap();
    }

    /// A Conversation on it, on a Repo of its own, which is what a piece of news
    /// is about.
    async fn working_on(&self, branch: &str) -> i64 {
        let repo = store::register_repo(&self.pool, Path::new("/srv/verkstead"), REPO, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet")
            .id;

        store::start_conversation(&self.pool, repo, branch, self.device.id())
            .await
            .unwrap()
            .expect("the Repo is registered")
    }

    /// Put a phone on its list, the way the page does.
    async fn pushes_to(&self, phone: &Phone) {
        let args = serde_json::json!({
            "endpoint": phone.endpoint,
            "p256dh": phone.p256dh(),
            "auth": phone.auth(),
        });

        let http = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/ui/push/subscribe")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&args).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = http.status();
        let said = http.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            status,
            StatusCode::OK,
            "subscribing {}: {}",
            phone.name,
            String::from_utf8_lossy(&said),
        );
    }

    /// How it dials, for the one question that is about a call rather than about
    /// a piece of news.
    fn peers(&self) -> Peers {
        Peers::of(self.device.clone(), Members::recorded(self.pool.clone())).waiting(PATIENCE)
    }

    /// Whether the row it holds for `device` is dimmed, which is what a dial that
    /// reached nobody leaves behind.
    async fn holds_unreachable(&self, device: &str) -> bool {
        store::members(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .any(|member| member.device == device && !member.reachable)
    }
}

/// `other` as a member row pointing at where it is really listening, which is
/// what a dial made by hand is handed.
fn member_at(listening: &Verkstead, device: &str) -> verkstead_store::Member {
    verkstead_store::Member {
        device: device.to_owned(),
        name: "somewhere".to_owned(),
        os: "Linux".to_owned(),
        addresses: vec![listening.at()],
        fingerprint: listening.device.fingerprint().to_owned(),
        last_seen: "2099-01-01T00:00:00Z".to_owned(),
        reachable: true,
        renewing_from: None,
        acknowledged: None,
    }
}

// ---------------------------------------------------------------------------
// The news crossing the link
// ---------------------------------------------------------------------------

/// A member's news lights the phone installed from another device, with the
/// device leading the title and a tap landing on that device's own URL.
///
/// **The stage's own demonstration.** One phone is subscribed to A and another to
/// B, and the work that finishes is B's. B's phone is shown the sentence B wrote,
/// with nothing in front of it, because it is B's own work; A's phone is shown the
/// same sentence with B's name in front of it, because on A that work is a
/// member's. And the path A's phone would open is under B's device segment: a path
/// taken verbatim would have opened A's own Conversation of the same number, which
/// is the id collision a cluster addresses every row by device for.
#[tokio::test]
async fn a_members_news_lights_a_phone_on_another_device_with_the_device_leading_it() {
    let (service, received) = push_service().await;

    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;
    b.linked_to(&a.device, A_MACHINE, "Linux", vec![a.at()])
        .await;

    let on_a = Phone::new(&service, "phone-on-a");
    let on_b = Phone::new(&service, "phone-on-b");
    a.pushes_to(&on_a).await;
    b.pushes_to(&on_b).await;

    let conversation = b.working_on(BRANCH).await;

    push::the_work_is_done(&b.pool, conversation);

    let locally = shown(&received, &on_b).await;
    assert_eq!(
        locally.title,
        format!("{BRANCH} is done"),
        "B's own phone is told about B's own work, so nothing leads the title",
    );
    assert_eq!(
        locally.path,
        format!("/conversations/{conversation}"),
        "and a tap opens it where B's own browser opens it",
    );
    assert_eq!(locally.project.as_deref(), Some(REPO));

    let relayed = shown(&received, &on_a).await;
    assert_eq!(
        relayed.title,
        format!("{B_MACHINE} — {BRANCH} is done"),
        "and the phone installed from A is told which machine the work was on",
    );
    assert_eq!(
        relayed.path,
        format!("/devices/{B}/conversations/{conversation}"),
        "and a tap opens B's Conversation on A rather than A's of the same number",
    );
    assert_eq!(
        relayed.project.as_deref(),
        Some(REPO),
        "and the repository stands under a relayed title as it does under a local one",
    );
}

/// A device that is linked to nothing tells its own phones exactly what it always
/// did.
///
/// **Which is the sidebar rule said about a notification**: a Verkstead with no
/// cluster draws no device anywhere, so nothing leads its titles either — and the
/// walk over its members is a walk over nothing rather than something to check
/// for.
#[tokio::test]
async fn a_device_linked_to_nothing_tells_its_phone_what_it_always_did() {
    let (service, received) = push_service().await;

    let alone = Verkstead::answering(A).await;

    let phone = Phone::new(&service, "the-only-phone");
    alone.pushes_to(&phone).await;

    let conversation = alone.working_on(BRANCH).await;

    push::the_work_is_done(&alone.pool, conversation);

    let notice = shown(&received, &phone).await;
    assert_eq!(notice.title, format!("{BRANCH} is done"));
    assert_eq!(notice.path, format!("/conversations/{conversation}"));

    settle().await;
    assert_eq!(
        count(&received, &phone),
        1,
        "one piece of news is one notification, there being nobody else to tell",
    );
}

/// A member that is switched off when the news happens costs the notification and
/// nothing else.
///
/// **The same bargain a push service that cannot be reached is already on.** The
/// record was written before any of this started, this device's own phone is told,
/// and the machine that answered nowhere is dimmed on the Devices list the way
/// every other dial dims one — which is the whole of what it costs. Nothing is
/// queued and nothing is retried: a notification is only what reaches a pocket,
/// and the Timeline says it in full on the device the work is on either way.
#[tokio::test]
async fn a_member_that_is_switched_off_costs_the_notification_and_nothing_else() {
    let (service, received) = push_service().await;

    let b = Verkstead::answering(B).await;

    // A machine that is not there: a row for A at an address nothing is listening
    // on, which is what a device that is switched off looks like from here.
    record_member(
        &b.pool,
        &Linking {
            device: A.to_owned(),
            name: A_MACHINE.to_owned(),
            os: "Linux".to_owned(),
            addresses: vec![nowhere().await],
            fingerprint: "sha256:nothing-is-listening-there".to_owned(),
        },
    )
    .await
    .unwrap();

    let on_b = Phone::new(&service, "phone-on-b");
    b.pushes_to(&on_b).await;

    let conversation = b.working_on(BRANCH).await;

    push::the_work_is_done(&b.pool, conversation);

    let locally = shown(&received, &on_b).await;
    assert_eq!(
        locally.title,
        format!("{BRANCH} is done"),
        "the local push went out, which is what a member being off must not cost",
    );

    let dimmed = tokio::time::timeout(WAITING, async {
        while !b.holds_unreachable(A).await {
            tokio::time::sleep(LOOKING).await;
        }
    })
    .await;

    assert!(
        dimmed.is_ok(),
        "the member that answered nowhere is dimmed, as every other dial dims one",
    );

    assert_eq!(
        store::members(&b.pool).await.unwrap().len(),
        1,
        "and the row stays on the list, dimmed rather than gone",
    );
}

/// A device that is not a member is refused the news by the gate.
///
/// **And by the gate rather than by a line written to refuse it.** The news is a
/// member's own call, so it stands where a member's calls stand: a device A holds
/// no membership for cannot say which device it is, and news from a machine A has
/// never confirmed is a sentence anybody able to reach that port could have put on
/// A's lock screen.
#[tokio::test]
async fn a_device_that_is_not_a_member_is_refused_the_news() {
    let (service, received) = push_service().await;

    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let stranger = Verkstead::answering(STRANGER).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;

    let on_a = Phone::new(&service, "phone-on-a");
    a.pushes_to(&on_a).await;

    let news = RelayedNews {
        conversation: 1,
        said: format!("{BRANCH} is done"),
        project: Some(REPO.to_owned()),
    };

    let refused = stranger
        .peers()
        .news(&member_at(&a, STRANGER), &news)
        .await
        .expect_err("a device A holds no membership for is refused at A's gate");

    assert!(
        format!("{refused:#}").contains("403"),
        "refused by the gate rather than missed: {refused:#}",
    );

    // And the same words from B, which A has confirmed, land — so the two dials
    // differ in the gate's answer and in nothing else.
    b.peers()
        .news(&member_at(&a, B), &news)
        .await
        .expect("a member's own news gets through");

    let shown_on_a = shown(&received, &on_a).await;
    assert_eq!(
        shown_on_a.title,
        format!("{B_MACHINE} — {BRANCH} is done"),
        "and it is titled by the device A holds the certificate for, not by the body",
    );

    settle().await;
    assert_eq!(
        count(&received, &on_a),
        1,
        "the stranger's words reached nobody's lock screen",
    );
}

/// And a member's news is not passed on to that member's cluster-mates.
///
/// **The rule the Nudge streams are held under, said about a notification.** In a
/// cluster everybody holds a link to everybody, so B's news reaches C down B's own
/// link; A passing it on would be A saying that news was its own, and C would show
/// the same work twice — once under B's name and once under A's.
#[tokio::test]
async fn news_a_member_told_this_device_is_not_passed_on() {
    let (service, received) = push_service().await;

    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;

    // A is linked to both, and C to A — so a relay from A to C is a dial that
    // would get through if one were ever made.
    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;
    a.linked_to(&c.device, C_MACHINE, "Linux", vec![c.at()])
        .await;
    c.linked_to(&a.device, A_MACHINE, "Linux", vec![a.at()])
        .await;

    let on_a = Phone::new(&service, "phone-on-a");
    let on_c = Phone::new(&service, "phone-on-c");
    a.pushes_to(&on_a).await;
    c.pushes_to(&on_c).await;

    b.peers()
        .news(
            &member_at(&a, B),
            &RelayedNews {
                conversation: 7,
                said: format!("{BRANCH} is done"),
                project: Some(REPO.to_owned()),
            },
        )
        .await
        .expect("B is a member of A");

    let shown_on_a = shown(&received, &on_a).await;
    assert_eq!(shown_on_a.title, format!("{B_MACHINE} — {BRANCH} is done"));

    settle().await;
    assert_eq!(
        count(&received, &on_c),
        0,
        "A told its own phones and nobody else's: news over the link is a device's own",
    );
}
