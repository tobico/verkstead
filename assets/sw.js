// Verkstead's service worker, served from the site root so its scope is the
// whole site: a worker can only control the paths under the one it was served
// from,
// and a worker under /pkg/ could never show a notification for /sets/12.
//
// It does no caching. Every page is rendered against live SQLite, and a cached
// copy of a Set that has since been answered is worse to the human than a
// failure to load. This listener never answers a request, which leaves the
// browser to fetch it exactly as it would with no worker at all; it is here
// because installability checks still look for a fetch handler.
self.addEventListener("fetch", () => {});

// Take over as soon as a new worker is available, rather than waiting for every
// tab to close. There is no cached state for a version skew to corrupt, and the
// stale worker would be the thing holding back a fix to push handling.
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

// Something happened while nobody was watching: something is waiting for the
// human — a Question Set, a Hold nobody came back to, a run stopped or an
// account out of window — or the work moved on past a milestone, a pull request
// opened, a roadmap stage started or a Conversation done. The worker does the
// same two things with every one of them: it shows a notification, and it tells
// every Verkstead that is already open to look again.
//
// The notification is not the negotiable half. The subscription was made with
// `userVisibleOnly`, and a push that showed nothing would cost the subscription
// itself — so it is shown even where a window is open on the very list the Nudge
// beside it is about to refresh.
self.addEventListener("push", (event) => {
  const notice = read(event.data);

  // Where the notice says to go: the Set's own page for a Set, and the
  // Conversation it is about for everything else. The server names it, because
  // what the push is about is the server's to know — see `Notice` in
  // `crates/server/src/push.rs`.
  // The workbench where a push could not say: every Conversation is there, and
  // so is every Set through the one it was asked from.
  const url = notice.path || "/";

  // The title is the notice's own, so it says which thing happened rather than
  // that something did — see `News::title` in `crates/server/src/push.rs`, where
  // all of them are written together for exactly that reason. The project goes in
  // the body, because it is what tells two of them apart at a glance.
  event.waitUntil(
    Promise.all([
      self.registration.showNotification(notice.title || "Verkstead is waiting for you", {
        body: notice.project ? `Verkstead · ${notice.project}` : "Verkstead",
        icon: "/icons/icon-192.png",
        // One notification per page it would open: a push service that delivers
        // the same push twice then replaces the notification instead of stacking
        // a second one over it.
        tag: `verkstead-${url}`,
        data: { url },
      }),
      nudge(),
    ]),
  );
});

// Tapped. What it was about is what opens — the Set, or the Conversation the news
// was about — in the Verkstead that is already there if there is one.
self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  event.waitUntil(open((event.notification.data || {}).url || "/"));
});

// Tell every open Verkstead that the pending world moved. The page reacts to
// this exactly as it does to a Nudge down the server's stream — look again at
// everything it is showing — and nothing here says what changed, because a Nudge
// never does.
//
// This is the second of the two channels the viewer's freshness is layered from.
// The stream is instant while the page is alive but dies when iOS suspends the
// PWA; the push that woke this worker survived exactly that. Each covers the
// other's gap, and neither has to work: the ten-second poll is underneath both.
//
// The message is named, so a page is never left guessing at whatever else may
// one day be posted to it. The page's half is `web/src/nudge.ts`, which cannot
// share the name with this file: a worker is a static file served from the site
// root rather than a module of the bundle.
async function nudge() {
  // includeUncontrolled, for the same reason opening a notification's Set does:
  // a page loaded before this worker took control is still an open Verkstead,
  // and it is the one that would otherwise sit on the poll alone.
  const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });

  for (const client of windows) {
    client.postMessage({ verkstead: "nudge" });
  }
}

// What the server sent, or an empty notice if it sent something unreadable —
// a notification saying a Set is waiting is still worth more than none.
function read(data) {
  if (!data) return {};

  try {
    return data.json() || {};
  } catch (_) {
    return {};
  }
}

async function open(url) {
  const target = new URL(url, self.location.origin).href;

  // includeUncontrolled: a Verkstead opened before this worker took control is
  // still the window the human means, and opening a second one over it is the
  // thing to avoid.
  const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });

  for (const client of windows) {
    if (new URL(client.url).origin !== self.location.origin) continue;

    // Navigated before it is focused, so the human is not shown whichever page
    // was open for as long as the navigation takes. A window this worker does
    // not control refuses to be navigated, and is focused where it stands
    // rather than being abandoned for a new window.
    const opened = client.url === target ? client : await client.navigate(target).catch(() => client);

    return opened.focus();
  }

  return self.clients.openWindow(target);
}
