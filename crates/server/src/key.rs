//! The Workbench Key: the one secret that says a request is the human's browser
//! rather than something a session started.
//!
//! A session's network is the host's own — see [`crate::sandbox`], and the
//! probe in `tests/sandbox.rs` that proves it — so the loopback address the
//! agent contract is served on is the same address the workbench is served on.
//! Nothing about the socket can tell the two apart. What can is a secret the
//! browser holds and no sandbox mounts: the Data Directory is not bound into
//! one, so a key kept there is a key a session cannot read (ADR-0015).
//!
//! **A file of its own, beside the settings files rather than inside one.**
//! A settings save writes the whole of `secrets.yaml` out of what the page was
//! told — see [`crate::settings::Settings::save_secrets`] — so a key kept there
//! would be a key an ordinary settings save rewrote, and every device logged
//! out by somebody tidying up a token. What is in that file is what somebody
//! configured; this is not configured by anybody.
//!
//! **The link is the address with `?key=…`.** Opening it sets the cookie and
//! redirects to the same path without the parameter, so the secret is out of the
//! URL bar, out of the history entry and out of any referrer before the page
//! that was asked for is drawn. A cookie carrying the current key is the whole
//! of being logged in, and a wrong one is worth exactly as much as none.
//!
//! **What stays open**, because none of it is a question about anybody's work:
//! `/api/v1/health`, the Conversation-scoped session API — which is a session's
//! own and scoped already — and the three static files a phone installs the
//! viewer from. A browser fetches a web manifest without credentials unless the
//! document's link tag says otherwise, and a service worker under a gate is a
//! push notification that never arrives, so gating those would cost
//! installability for nothing: there is nothing about anybody's work in an icon.
//!
//! The Share Viewer needs no exemption at all. It is a self-contained file
//! published to a secret gist — see [`crate::sharing`] — and reads nothing of
//! this server.
//!
//! One consequence worth naming: a path under `/api/` that no route answers to
//! reaches the fallback, so on the keyed router it is refused here rather than
//! missed there. Nothing is hidden by that — everything real under `/api/` is a
//! route, and what the fallback would have said is a 404 carrying nothing — and
//! an agent that mistyped an endpoint is still told plainly that it did not get
//! what it asked for.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use axum::Router;
use axum::extract::{Request, State};
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Method, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::settings::write_atomically;

/// What the key's file is called inside the Data Directory. Fixed rather than
/// configurable, for the reason the database's name and the settings files'
/// names are: the directory is what an operator points Verkstead at, and what is
/// in it is Verkstead's to name.
const KEY_FILE: &str = "workbench.key";

/// And what it is written as: readable and writable by the account Verkstead
/// runs under, and by nothing else on the machine. The same mode `secrets.yaml`
/// gets, for the same reason — this is the credential that stands in front of
/// every one of them.
const KEY_MODE: u32 = 0o600;

/// How much randomness the secret is. Thirty-two bytes from the operating
/// system's own generator: the same size as the session identifiers picked in
/// [`crate::sessions`] and twice over, because this one is guessed at from the
/// tailnet rather than from nowhere.
const KEY_BYTES: usize = 32;

/// What the cookie is called. Named for what it holds rather than for the
/// product, so that a person reading their browser's storage can see what it is.
const COOKIE_NAME: &str = "workbench_key";

/// And what the parameter on the link is called.
pub const QUERY: &str = "key";

/// How long the browser is asked to keep it: ten years, which is the longest
/// anything here has to mean. There is no expiry on the key itself — one secret
/// with a Reset press behind it is the whole of the scheme — so a cookie that
/// expired would be a device logged out for no reason anybody asked for.
const FOR_GOOD: u64 = 10 * 365 * 24 * 60 * 60;

/// The secret, and the file it is kept in.
///
/// A handle rather than a bare string, because the file is what the secret *is*:
/// a second Verkstead over the same Data Directory reads the same key, and a
/// re-issue has somewhere to write.
#[derive(Debug, Clone)]
pub struct WorkbenchKey {
    path: PathBuf,

    /// Held in memory as well as on disk, because it is compared against on
    /// every request the browser makes and a file read per request would be a
    /// syscall on the way to every page. Behind a lock so that re-issuing it has
    /// somewhere to put the new one.
    secret: Arc<RwLock<String>>,
}

impl WorkbenchKey {
    /// The key kept in `data_dir`: whatever is already there, or a new one
    /// written where there is nothing.
    ///
    /// **A second start reads the first's**, which is what makes a link worth
    /// keeping: a key re-issued at every start would log every device out on
    /// every restart, and there would be no point in a phone remembering one.
    ///
    /// A file that is there and empty is treated as a file that is not there.
    /// Nothing writes one — the write below is atomic — so it is a machine that
    /// lost power or a hand that emptied it, and issuing a key is the only
    /// recovery either of those has.
    pub fn issued(data_dir: &Path) -> std::io::Result<WorkbenchKey> {
        let path = data_dir.join(KEY_FILE);

        let secret = match std::fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => text.trim().to_owned(),
            _ => {
                let secret = invented()?;
                write_atomically(&path, &format!("{secret}\n"), KEY_MODE)?;
                secret
            }
        };

        Ok(WorkbenchKey {
            path,
            secret: Arc::new(RwLock::new(secret)),
        })
    }

    /// The key a fixture states, so that a login link written into one reads
    /// the same on every machine that writes it.
    ///
    /// The same file in the same place, written rather than invented — a key is
    /// a key whatever made it, and a suite that had to filter a random secret
    /// out of a payload would be a suite pinning everything but the field it is
    /// about. See `Tailscale::as_user`, which is here for the same reason.
    pub fn stated(data_dir: &Path, secret: &str) -> std::io::Result<WorkbenchKey> {
        let path = data_dir.join(KEY_FILE);

        write_atomically(&path, &format!("{secret}\n"), KEY_MODE)?;

        Ok(WorkbenchKey {
            path,
            secret: Arc::new(RwLock::new(secret.to_owned())),
        })
    }

    /// A new secret over the old one, which is **Reset key** on the Remote
    /// access pane.
    ///
    /// Everything holding the old one is logged out by this and nothing else:
    /// there is no list of devices and no expiry, so re-issuing is the whole of
    /// taking a link back — a phone that was lost, a QR somebody photographed
    /// over a shoulder, a link pasted where it should not have been.
    ///
    /// The file first and the memory after it, so that a write that failed
    /// leaves every device holding a key that still works rather than a server
    /// admitting one nothing on disk agrees with.
    ///
    /// Every handle on this key sees it: the desktop app and the server share
    /// one — see [`crate::Config::workbench_key`] — so the tray's Open opens on
    /// the new link without the app having been told anything.
    pub fn reissue(&self) -> std::io::Result<()> {
        let fresh = invented()?;

        write_atomically(&self.path, &format!("{fresh}\n"), KEY_MODE)?;

        *self
            .secret
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = fresh;

        Ok(())
    }

    /// What a cookie has to carry, and what the link hands over.
    pub fn secret(&self) -> String {
        self.secret
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// And the cookie carrying it, as a `name=value` pair.
    ///
    /// Said here rather than at each end, because the two ends have to agree
    /// exactly: what the handshake sets and what the gate reads back are one
    /// spelling, and so is what a test standing where the browser stands sends.
    pub fn cookie(&self) -> String {
        format!("{COOKIE_NAME}={}", self.secret())
    }

    /// And the whole `Set-Cookie` that puts it in a browser.
    ///
    /// The cookie is not `Secure`. The workbench is served over plain HTTP on
    /// the loopback and over HTTPS through `tailscale serve`, and a cookie the
    /// loopback could not set would be a desktop that could never log itself
    /// in. `HttpOnly` because no script here has any business reading it, and
    /// `SameSite=Lax` so that following a link from somewhere else — a QR
    /// code's browser, a chat message — arrives logged in.
    ///
    /// Said once, because two places set it: the handshake a link goes through,
    /// and the re-issue that hands the browser that asked for it the key it
    /// just made.
    pub fn set_cookie(&self) -> String {
        format!(
            "{}; Path=/; Max-Age={FOR_GOOD}; HttpOnly; SameSite=Lax",
            self.cookie()
        )
    }

    /// Where the file is, which is what says so on a settings page and in a
    /// failure to write one.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The login link against `base`: the address with the key on it, which is
    /// the whole of how a device is let in.
    ///
    /// `base` is an origin — `http://127.0.0.1:8422` for a browser on this
    /// machine, and whatever `tailscale serve` answers on for the phone that
    /// reaches it from the tailnet — because where Verkstead is reached *from*
    /// is not something the server can read off its own socket.
    ///
    /// Pointed at the root rather than at a path: the workbench opens where it
    /// always opens, and the handshake redirects there with the parameter
    /// taken off.
    ///
    /// The secret is read at the moment the link is asked for rather than kept
    /// as a string beside the handle, so a link built from a handle whose key
    /// has since been re-issued carries the new one.
    pub fn link(&self, base: &str) -> String {
        format!("{}/?{QUERY}={}", base.trim_end_matches('/'), self.secret())
    }
}

/// The login link for a browser on the machine Verkstead is running on: what
/// the startup line carries, and what the desktop app opens.
///
/// The address as it was given, unless that is the unspecified one — bound to
/// `0.0.0.0` the server answers on every interface this machine has, and what
/// a browser *here* is pointed at is the loopback rather than a literal
/// `0.0.0.0` a URL bar has nothing to do with.
pub fn login_link(listen: SocketAddr, key: &WorkbenchKey) -> String {
    key.link(&format!("http://{}", browsable(listen)))
}

/// `listen` as an address a browser on this machine can be pointed at.
fn browsable(listen: SocketAddr) -> SocketAddr {
    match listen.ip() {
        IpAddr::V4(address) if address.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen.port())
        }
        IpAddr::V6(address) if address.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listen.port())
        }
        _ => listen,
    }
}

/// Thirty-two bytes of the operating system's own randomness, spelled so that it
/// survives a URL, a QR code and a person reading it aloud off a screen: base64
/// with the URL alphabet and no padding, whose every character is one no URL and
/// no cookie has to escape.
fn invented() -> std::io::Result<String> {
    let mut bytes = [0u8; KEY_BYTES];

    getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;

    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// What stands in front of a router: a key, or nothing at all.
///
/// **The gate is something a router is given rather than something every router
/// has.** The suites are built on constructors that answer every request, and
/// several hundred requests across them assume it; the router the binary serves
/// is the one that is keyed, and the suite that asks about the gate itself gets
/// a keyed constructor of its own — see [`crate::router_keyed`].
#[derive(Debug, Clone, Default)]
pub(crate) struct Gate(Option<WorkbenchKey>);

impl Gate {
    /// A gate that is not there: everything through, which is what every router
    /// stood up for a question about something else wants.
    pub(crate) fn open() -> Gate {
        Gate(None)
    }

    /// And one that answers 401 to anything that has not shown `key`.
    pub(crate) fn keyed(key: WorkbenchKey) -> Gate {
        Gate(Some(key))
    }

    /// The key this gate stands on, where it stands on one.
    ///
    /// What the state behind it is given, so that **Reset key** has the same
    /// handle to re-issue that the gate is checking against — a re-issue
    /// through a second handle would be a gate still admitting the old secret.
    pub(crate) fn held(&self) -> Option<WorkbenchKey> {
        self.0.clone()
    }

    /// Put it in front of everything in `router`.
    ///
    /// Called twice on the way to the served router, because the two things
    /// behind the gate are added in two places: the viewer's own namespace is
    /// merged into the API router, and the fallback that answers every page of
    /// the workbench is put on afterwards.
    pub(crate) fn guarding<S>(&self, router: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        match &self.0 {
            None => router,
            Some(key) => router.layer(axum::middleware::from_fn_with_state(key.clone(), gate)),
        }
    }
}

/// The gate itself: the handshake, then the cookie, then a refusal.
async fn gate(State(key): State<WorkbenchKey>, request: Request, next: Next) -> Response {
    if installable(request.uri().path()) {
        return next.run(request).await;
    }

    let secret = key.secret();

    // The handshake, on the one kind of request a link can be: a browser
    // following one sends GET, and a redirect answered to anything else would be
    // a body quietly dropped on the way to a second request.
    if matches!(*request.method(), Method::GET | Method::HEAD)
        && let Some(offered) = offered_in(request.uri())
        && same(&offered, &secret)
    {
        return welcomed(&key, request.uri());
    }

    if carried_by(&request, &secret) {
        return next.run(request).await;
    }

    refused()
}

/// The files a phone installs the viewer from, which are open — see this
/// module's own documentation for why.
///
/// **The icons are a prefix, and so have to name a file.** The other two are one
/// exact path each; the icons are a directory, because there is one per size and
/// the document names them by hand. What makes that safe is the second half of
/// the test. A path whose last segment carries no extension is a route rather
/// than a file, and the viewer answers a route with the workbench's own document
/// — so an exemption on the prefix alone would hand `/icons/anything` the very
/// page `/` is refused for. With it, a miss under here is a file that is not
/// there, which the viewer answers with a 404.
fn installable(path: &str) -> bool {
    path == "/sw.js"
        || path == "/manifest.webmanifest"
        || (path.starts_with("/icons/") && names_a_file(path))
}

/// Whether `path` was asking for a file rather than naming a route: a dot in the
/// last segment.
///
/// The same test the viewer sorts the two by — see `viewer::names_a_file` — and
/// said again here rather than shared, because what the two of them have to
/// agree about is a judgement rather than a helper: this one decides what is
/// let past the gate, and one that drifted from the viewer's would be an
/// exemption for a path the viewer answers with a page.
fn names_a_file(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'))
}

/// The key on the link, where there is one.
///
/// Split by hand rather than decoded, because the alphabet the secret is spelled
/// in — see [`invented`] — has no character a URL escapes: what a browser sends
/// is what was written.
fn offered_in(uri: &Uri) -> Option<String> {
    uri.query()?.split('&').find_map(|pair| {
        pair.strip_prefix(QUERY)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::to_owned)
    })
}

/// Whether the request's cookies carry the key.
///
/// Every `Cookie` header rather than the first: a browser sends one, but nothing
/// in the protocol says it has to.
fn carried_by(request: &Request, secret: &str) -> bool {
    request
        .headers()
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|cookie| cookie.trim().split_once('='))
        .any(|(name, value)| name == COOKIE_NAME && same(value, secret))
}

/// Whether two secrets are the same, without saying how far along they differed.
///
/// The whole of what is being guarded is a string somebody may guess at over a
/// tailnet, so the comparison takes the same time whatever it is given: a
/// short-circuit here is a way to learn the key one character at a time.
fn same(offered: &str, secret: &str) -> bool {
    let (offered, secret) = (offered.as_bytes(), secret.as_bytes());

    offered.len() == secret.len()
        && offered
            .iter()
            .zip(secret)
            .fold(0u8, |seen, (a, b)| seen | (a ^ b))
            == 0
}

/// The answer to a link that carried the key: the cookie set, and the same path
/// again without it.
///
/// The parameter is taken off and whatever else was on the query is put back, so
/// that a link into somewhere the workbench reads its own parameters lands where
/// it was pointed rather than at a stripped version of it.
///
/// `See Other` rather than `Found`, which says in the status what the redirect is
/// for: the resource asked for is over there, and it is fetched with a GET.
///
/// What the cookie is set with is [`WorkbenchKey::set_cookie`], which the
/// re-issue sets the same one from.
fn welcomed(key: &WorkbenchKey, uri: &Uri) -> Response {
    let kept: Vec<&str> = uri
        .query()
        .into_iter()
        .flat_map(|query| query.split('&'))
        .filter(|pair| !pair.starts_with(&format!("{QUERY}=")))
        .collect();

    let landing = match kept.is_empty() {
        true => uri.path().to_owned(),
        false => format!("{}?{}", uri.path(), kept.join("&")),
    };

    (
        StatusCode::SEE_OTHER,
        [(SET_COOKIE, key.set_cookie()), (LOCATION, landing)],
    )
        .into_response()
}

/// And the answer to everything else.
///
/// No `WWW-Authenticate`: the credential is a cookie a link sets rather than
/// anything the browser knows how to ask for, and a challenge here would open a
/// password dialog nobody can fill in.
fn refused() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "verkstead is reached through its own link, which carries the workbench key\n",
    )
        .into_response()
}
