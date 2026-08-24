//! Telling the devices what happened while nobody was watching.
//!
//! Two kinds of thing are worth a phone lighting up. **Needs-you**: a Question
//! Set has arrived, a Hold has stood a while with nobody coming back to it, a
//! run has stopped on something Verkstead cannot resolve, or the account it was
//! spending ran out of window. And **milestones**: the work is on a pull
//! request, a roadmap has moved on to its next stage or run out of stages, or a
//! Conversation has reached Done. One push per subscribed device in every case,
//! encrypted for that device's own keys and signed with the VAPID identity the
//! store generated on first run. The body is small on purpose: enough for the
//! service worker to draw the notification and to know which page to open, and
//! nothing that would put a Question — or the substance of the work — in a
//! notification the phone shows on a lock screen.
//!
//! What tells one from another is its title, which is why all of them are
//! written in the same place: see [`News::title`]. A phone that lights up with
//! the same sentence whatever happened is a phone the human learns to ignore.
//!
//! Sending always happens behind the thing it is announcing, never in front of
//! it. Delivery goes out through the browser vendors' push services, which is
//! the one place Verkstead reaches the public internet, and none of it is
//! reliable enough to make the record depend on: a service that cannot be
//! reached costs a notification, and never the Set, the pull request or the
//! Interruption it was about.
//!
//! A Hold's push is a reminder and nothing more. It ends no Hold — only the
//! hand-back does that — and it leaves nothing on the Timeline, which records
//! the work rather than the watching. That holds for every push here: each is
//! sent from the one place that already knows the thing happened, and none of
//! them writes anything down.
//!
//! A push service is also the only thing that can tell us a device has gone —
//! the app was uninstalled, the subscription expired — and it says so with a
//! `404` or a `410`. Those are pruned. Everything else, a timeout or a `503`,
//! is a notification lost and the device is left alone.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use serde::Serialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;
use verkstead_schema::QuestionSet;
use verkstead_store::{PushSubscription, VapidKeys};
use web_push_native::{Auth, WebPushBuilder};

use crate::AppState;
use crate::hold::Which;

/// How long a push service should hold a notification for a phone that is off
/// or out of signal. Half a day: a Question Set older than that has either been
/// answered at a desk or is not going to be answered from a lock screen.
const HOLD_FOR: Duration = Duration::from_secs(12 * 60 * 60);

/// How long to wait on one push service before giving up on it. A push service
/// that is this slow has cost this device its notification either way, and the
/// remaining devices are still waiting their turn.
const REACHABLE_WITHIN: Duration = Duration::from_secs(10);

/// How long a VAPID signature stays good for. RFC 8292 caps it at 24 hours;
/// each push is signed as it is sent, so this only has to outlive the request.
const SIGNATURE_GOOD_FOR: i64 = 12 * 60 * 60;

/// The contact the VAPID claim carries: whom a push service would complain to
/// about this server's traffic. It wants a `mailto:` or an `https:` URI, and
/// the project is the only thing that is true of every Verkstead — this is a
/// single-user tool with nobody's address configured in it.
const CONTACT: &str = "https://github.com/tobico/verkstead";

/// The one JWT header these signatures ever use, so it is spelled out rather
/// than serialized: ES256 is what VAPID is defined in terms of.
const JWT_HEADER: &str = r#"{"typ":"JWT","alg":"ES256"}"#;

/// How long a Hold stands before the human's devices are told about it, on a
/// server nobody has said otherwise to.
///
/// Long enough that a human who typed and is still typing is not interrupted by
/// news of their own keyboard, and short enough that one who put the phone down
/// mid-intervention is told while the run they stalled still matters. What a
/// server actually keeps to is [`crate::Pace::holding`].
pub(crate) const HELD_A_WHILE: Duration = Duration::from_secs(5 * 60);

/// What the service worker is handed: enough to draw the notification, and where
/// tapping it goes.
#[derive(Debug, Serialize)]
struct Notice<'a> {
    /// The page the notification opens — a Set's own page for a Set, and the
    /// held Conversation for a Hold.
    ///
    /// Said by the server rather than worked out by the worker, because what a
    /// push is about is the server's to know: a phone woken by a Hold that
    /// landed on a Question Set it is not about would be worse than no
    /// notification at all.
    path: String,

    title: &'a str,

    /// Which repository this is about, when it is known — the one thing that
    /// tells two notifications apart at a glance on a lock screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<&'a str>,
}

/// What became of one push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    /// The push service took it. Whether the phone ever shows it is between the
    /// two of them.
    Taken,

    /// This subscription is finished with, for good. The device uninstalled the
    /// app, or its subscription expired.
    Gone,
}

/// Tell every subscribed device about a Set, without making the agent wait for
/// it.
///
/// Returns as soon as the work is handed to the runtime: the caller's job is to
/// answer the agent, and this must not be able to delay that or to fail it.
pub(crate) fn announce(pool: &SqlitePool, id: i64, set: &QuestionSet) {
    let notice = Notice {
        path: format!("/sets/{id}"),
        title: &set.title,
        project: set.project.as_deref(),
    };

    let notice = match serde_json::to_vec(&notice) {
        Ok(notice) => notice,
        Err(error) => {
            tracing::error!(set = id, error = ?error, "the push notice could not be built");
            return;
        }
    };

    let pool = pool.clone();
    let about = format!("Set {id}");

    tokio::spawn(async move {
        if let Err(error) = notify(&pool, &about, &notice).await {
            tracing::error!(set = id, error = ?error, "telling the devices about a Set failed");
        }
    });
}

/// Tell the devices about a Hold once it has stood [`crate::Pace::holding`],
/// and say nothing at all if the keyboard has gone back before then.
///
/// Returns as soon as the waiting is handed to the runtime: what the caller is
/// doing is relaying a keystroke, and a reminder about it must not be able to
/// slow that down.
///
/// One push per Hold, because it is one wait per Hold: nothing here loops, so a
/// Hold that goes on standing is told about once and then left alone. The
/// [`Which`] is what keeps it to *this* Hold — a Conversation handed back and
/// typed into again in the meantime has a Hold of its own, with a wait of its
/// own behind it.
pub(crate) fn when_it_has_stood(state: &AppState, conversation_id: i64, which: Which) {
    let state = state.clone();
    let stood_a_while = state.sessions.pace().holding;

    tokio::spawn(async move {
        tokio::time::sleep(stood_a_while).await;

        // Handed back, or handed back and taken again: either way this Hold is
        // over, and nothing is owed about a Hold nobody is in.
        if !state.sessions.still_held(conversation_id, which) {
            return;
        }

        told(&state.pool, conversation_id, News::Waiting);
    });
}

/// What a push about a Conversation is saying.
///
/// One enum rather than a function each, because nearly all of it is common: a
/// Conversation to load, a branch to read it by, the Repo underneath and one
/// push per subscribed device. What differs is the sentence — and that is the
/// half worth keeping together, since the whole job of a title is to say which
/// of these it is to somebody glancing at a lock screen.
#[derive(Debug, Clone)]
pub(crate) enum News {
    /// A Hold has stood a while with nobody coming back to the keyboard.
    Waiting,

    /// A run stopped on something Verkstead cannot resolve, and the Conversation
    /// is blocked on the human until a Remedy is chosen — see
    /// [`crate::interruptions`]. `what` is the step it stopped at, in the words
    /// the Interruption's own evidence carries.
    Stopped { what: String },

    /// The account a run was spending ran out of window, so the run is waiting
    /// on the human or on the clock — see [`crate::limits`].
    OutOfWindow {
        profile: String,
        resets_at: Option<String>,
    },

    /// The work the Conversation was for is on a pull request, and the wrap-up
    /// has started.
    OnAPullRequest { number: i64 },

    /// The stage after the one that just settled has started, on a Conversation
    /// of its own — which is this one.
    StageStarted { label: String, roadmap: String },

    /// A roadmap that has run out of stages: the one that settled was its last.
    RoadmapComplete { roadmap: String },

    /// The Conversation has reached Done. Verkstead has finished with the work;
    /// whether it is merged is the human's.
    Done,
}

impl News {
    /// The sentence the lock screen shows.
    ///
    /// Every title in one place, because the thing they all have to do is be
    /// told apart from each other at a glance. Two rules hold across the lot of
    /// them. What a piece of work is read by is its branch — see
    /// [`verkstead_store::ConversationRow`] — so a title names that, unless it
    /// has something the human knows the work by better: the account that ran
    /// out, or the stage of the roadmap. And nothing of the *substance* of the
    /// work goes in, which is the rule that keeps a Question out of a Set's push.
    fn title(&self, branch: &str) -> String {
        match self {
            News::Waiting => format!("{branch} is waiting for you"),
            // The step rather than how it went wrong: which part of the run
            // stopped is what decides whether the human gets up, and the
            // evidence underneath it is one tap away.
            News::Stopped { what } => format!("{branch} stopped while {what}"),
            // The account and when it comes back, because those are the two
            // things that decide whether the human does anything about it: an
            // account back in twenty minutes is one to leave alone.
            News::OutOfWindow {
                profile,
                resets_at: Some(resets_at),
            } => format!("{profile} is out of window until {resets_at}"),
            News::OutOfWindow {
                profile,
                resets_at: None,
            } => format!("{profile} is out of window"),
            News::OnAPullRequest { number } => format!("{branch} is on pull request #{number}"),
            // Read by the stage rather than by the branch: a stage is a
            // Conversation whose name in the human's head is its number in the
            // roadmap, and the branch is named after that anyway.
            News::StageStarted { label, roadmap } => {
                format!("Stage {label} of the `{roadmap}` roadmap has started")
            }
            News::RoadmapComplete { roadmap } => format!("The `{roadmap}` roadmap is complete"),
            News::Done => format!("{branch} is done"),
        }
    }

    /// What the log calls it, where a push could not be sent.
    fn about(&self) -> &'static str {
        match self {
            News::Waiting => "the Hold",
            News::Stopped { .. } => "the Interruption",
            News::OutOfWindow { .. } => "the Pause",
            News::OnAPullRequest { .. } => "the pull request",
            News::StageStarted { .. } => "the stage that started",
            News::RoadmapComplete { .. } => "the roadmap that is finished",
            News::Done => "the work being done",
        }
    }
}

/// Tell every subscribed device something that happened to a Conversation,
/// without making the thing that happened wait for it.
///
/// Returns as soon as the work is handed to the runtime, exactly as a Set's push
/// does: the caller's job is to put the Interruption, the pull request or the
/// Pause on the record, and none of this may delay that or fail it. A push
/// service that cannot be reached costs a notification and nothing else.
pub(crate) fn told(pool: &SqlitePool, conversation_id: i64, news: News) {
    let pool = pool.clone();

    tokio::spawn(async move {
        if let Err(error) = say(&pool, conversation_id, &news).await {
            tracing::error!(
                conversation_id,
                about = news.about(),
                error = ?error,
                "telling the devices about a Conversation failed",
            );
        }
    });
}

/// The notice one piece of news is worth, sent.
///
/// With the Repo underneath the title, so that a lock screen says which piece of
/// work this is about — that is the one thing that tells two notifications apart
/// where their titles read alike.
async fn say(pool: &SqlitePool, conversation_id: i64, news: &News) -> Result<()> {
    let Some(conversation) = verkstead_store::load_conversation(pool, conversation_id).await?
    else {
        // Aborted and gone between the thing happening and this being sent.
        // There is nobody left to tell anything about it.
        return Ok(());
    };

    let title = news.title(&conversation.branch);

    let notice = Notice {
        path: format!("/conversations/{conversation_id}"),
        title: &title,
        project: Some(&conversation.repo.name),
    };

    let notice = serde_json::to_vec(&notice).context("building the push notice")?;

    notify(
        pool,
        &format!("{} on {conversation_id}", news.about()),
        &notice,
    )
    .await
}

/// Send the notice to every device, and prune the ones the push services have
/// finished with.
///
/// One device at a time rather than all at once: this is a single human's
/// handful of devices, and the timeout bounds what a slow push service can cost
/// the ones behind it.
async fn notify(pool: &SqlitePool, about: &str, notice: &[u8]) -> Result<()> {
    let devices = verkstead_store::push_subscriptions(pool).await?;
    if devices.is_empty() {
        return Ok(());
    }

    let keys = verkstead_store::vapid_keys(pool).await?;
    let client = reqwest::Client::builder()
        .timeout(REACHABLE_WITHIN)
        .build()
        .context("building the push client")?;

    for device in devices {
        match send(&client, &keys, &device, notice).await {
            Ok(Delivery::Taken) => {
                tracing::debug!(about, endpoint = %device.endpoint, "a device was told");
            }
            Ok(Delivery::Gone) => {
                tracing::info!(
                    endpoint = %device.endpoint,
                    "the push service has finished with this device, so it is forgotten",
                );
                verkstead_store::forget_subscription(pool, &device.endpoint).await?;
            }
            // Logged rather than returned: the devices behind this one have
            // done nothing wrong, and there is nobody to hand the failure to —
            // the agent was answered before any of this started.
            Err(error) => {
                tracing::warn!(
                    about,
                    endpoint = %device.endpoint,
                    error = ?error,
                    "a device was not told",
                );
            }
        }
    }

    Ok(())
}

/// Put one push on the wire, and read what the push service made of it.
async fn send(
    client: &reqwest::Client,
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<Delivery> {
    let request = addressed(keys, device, notice)?;

    let response = client
        .execute(reqwest::Request::try_from(request).context("preparing the push request")?)
        .await
        .context("reaching the push service")?;

    let status = response.status();

    if status.is_success() {
        return Ok(Delivery::Taken);
    }

    // The only two answers that mean the subscription itself is over. Anything
    // else — a timeout, a 429, a 503 — is this notification lost and no word at
    // all about the device.
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
        return Ok(Delivery::Gone);
    }

    let said = response.text().await.unwrap_or_default();
    bail!("the push service answered {status}: {said}");
}

/// The notice encrypted for one device, in the request its push service takes.
fn addressed(
    keys: &VapidKeys,
    device: &PushSubscription,
    notice: &[u8],
) -> Result<http::Request<Vec<u8>>> {
    let endpoint: http::Uri = device
        .endpoint
        .parse()
        .with_context(|| format!("the stored endpoint {:?} is not a URL", device.endpoint))?;

    let public = URL_SAFE_NO_PAD
        .decode(&device.p256dh)
        .context("the stored p256dh is not base64url")?;
    let public = p256::PublicKey::from_sec1_bytes(&public)
        .context("the stored p256dh is not a P-256 key")?;

    let auth = URL_SAFE_NO_PAD
        .decode(&device.auth)
        .context("the stored auth secret is not base64url")?;
    if auth.len() != 16 {
        bail!("the stored auth secret is {} bytes, not 16", auth.len());
    }

    let mut request = WebPushBuilder::new(endpoint.clone(), public, Auth::clone_from_slice(&auth))
        .with_valid_duration(HOLD_FOR)
        .build(notice.to_vec())
        .map_err(|err| anyhow::anyhow!("encrypting the push: {err}"))?;

    let authorization = authorization(keys, &endpoint)?;
    request.headers_mut().insert(
        http::header::AUTHORIZATION,
        authorization
            .parse()
            .context("the VAPID header is not a header value")?,
    );

    Ok(request)
}

/// The `Authorization` header a push service checks this server by: a signed
/// claim about who is sending, alongside the public key it was signed with —
/// which is the key the device subscribed against.
fn authorization(keys: &VapidKeys, endpoint: &http::Uri) -> Result<String> {
    let claims = serde_json::json!({
        "aud": audience(endpoint)?,
        "exp": OffsetDateTime::now_utc().unix_timestamp() + SIGNATURE_GOOD_FOR,
        "sub": CONTACT,
    });

    let token = signed(
        &keys.private_key,
        &serde_json::to_vec(&claims).context("serialising the VAPID claims")?,
    )?;

    Ok(format!("vapid t={token}, k={}", keys.public_key))
}

/// Who a signature is for: the push service's origin, and nothing of the path
/// that identifies the device. A signature made out to one service is no use at
/// another, which is the whole point of the claim.
fn audience(endpoint: &http::Uri) -> Result<String> {
    let scheme = endpoint
        .scheme_str()
        .context("the endpoint has no scheme to make the signature out to")?;
    let authority = endpoint
        .authority()
        .context("the endpoint has no host to make the signature out to")?;

    Ok(format!("{scheme}://{authority}"))
}

/// The claims as a signed ES256 JWT.
///
/// Signed with `p256` directly rather than through a JWT crate: this is one
/// signature over two claims under a fixed header, and the keypair is already
/// a `p256` one.
fn signed(private_key: &str, claims: &[u8]) -> Result<String> {
    let scalar = URL_SAFE_NO_PAD
        .decode(private_key)
        .context("the stored private key is not base64url")?;
    let key = p256::ecdsa::SigningKey::from_slice(&scalar)
        .context("the stored private key is not a P-256 scalar")?;

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(JWT_HEADER),
        URL_SAFE_NO_PAD.encode(claims),
    );

    // Fixed-width r‖s rather than DER: JWS defines ES256 as the 64-byte form,
    // and a push service handed DER rejects the signature.
    let signature: p256::ecdsa::Signature = key.sign(signing_input.as_bytes());

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes()),
    ))
}

#[cfg(test)]
mod tests {
    use super::{JWT_HEADER, News, audience, signed};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::signature::Verifier;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::elliptic_curve::sec1::ToEncodedPoint;

    /// Every piece of news, as one Conversation would produce them.
    fn all() -> Vec<News> {
        vec![
            News::Waiting,
            News::Stopped {
                what: "implementing the work inline".to_owned(),
            },
            News::OutOfWindow {
                profile: "implementation".to_owned(),
                resets_at: Some("2026-08-24T05:00:00Z".to_owned()),
            },
            News::OutOfWindow {
                profile: "implementation".to_owned(),
                resets_at: None,
            },
            News::OnAPullRequest { number: 41 },
            News::StageStarted {
                label: "01".to_owned(),
                roadmap: "rate-limiting".to_owned(),
            },
            News::RoadmapComplete {
                roadmap: "rate-limiting".to_owned(),
            },
            News::Done,
        ]
    }

    /// The whole job of a title is to say which of these it is, to somebody
    /// glancing at a lock screen with the app shut. Two of them reading alike is
    /// a phone that says only that *something* happened.
    #[test]
    fn no_two_notifications_about_one_conversation_read_alike() {
        let mut titles: Vec<String> = all()
            .iter()
            .map(|news| news.title("rate-limiting"))
            .collect();

        let said = titles.clone();
        titles.sort();
        titles.dedup();

        assert_eq!(
            titles.len(),
            said.len(),
            "two of these read the same: {said:?}"
        );
    }

    /// And each of them fits on a lock screen, which is where every one of them
    /// is read: a title the phone cuts off mid-sentence has thrown away whichever
    /// half was at the end of it.
    #[test]
    fn a_title_is_short_enough_to_be_shown_whole() {
        for news in all() {
            let title = news.title("rate-limiting");

            assert!(
                title.len() <= 80,
                "a title a lock screen would cut off mid-sentence: {title:?}",
            );
        }
    }

    #[test]
    fn a_signature_is_one_a_push_service_can_check_against_the_public_key() {
        let secret = p256::SecretKey::random(&mut OsRng);
        let private_key = URL_SAFE_NO_PAD.encode(secret.to_bytes());

        let token = signed(&private_key, br#"{"aud":"https://push.example"}"#).unwrap();

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT is three parts: {token}");
        assert_eq!(
            URL_SAFE_NO_PAD.decode(parts[0]).unwrap(),
            JWT_HEADER.as_bytes(),
        );

        let signature =
            p256::ecdsa::Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap())
                .expect("the signature has to be the 64-byte r‖s form JWS defines");

        // Through the encoding the browser was handed, because that is the key
        // the push service checks against: the `k=` half of the header.
        let handed_out = secret.public_key().to_encoded_point(false);
        let verifying = p256::ecdsa::VerifyingKey::from_sec1_bytes(handed_out.as_bytes()).unwrap();

        verifying
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("the push service has to be able to check the signature");
    }

    #[test]
    fn a_signature_is_made_out_to_the_push_service_and_not_to_the_device() {
        let endpoint = "https://push.example/devices/abc123?token=xyz"
            .parse()
            .unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "https://push.example");
    }

    #[test]
    fn a_push_service_on_an_unusual_port_is_a_different_audience() {
        let endpoint = "http://127.0.0.1:8422/devices/abc123".parse().unwrap();

        assert_eq!(audience(&endpoint).unwrap(), "http://127.0.0.1:8422");
    }
}
