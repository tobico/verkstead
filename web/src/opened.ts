//! Where a click on one of the app's own notifications opens, followed in place.
//!
//! **Inside the app a notification is the app's rather than a service worker's**
//! (ADR-0020, *Amended: notifications are the app's own*): Electron ships without
//! Google's push client, so the app holds the server's notices stream and draws
//! each as the platform's own notification. A click there lands where a tap on
//! the phone lands — the Timeline Event a Set is read on, the Conversation, the
//! Remote access pane — and the app says which over the bridge. What goes there
//! is this page, by routing rather than by being loaded again, so a composer half
//! typed into is still half typed into when the human comes back to it.
//!
//! **And none of it reaches a browser.** A browser on this machine or a phone on
//! the tailnet has no bridge, and so nothing to listen to: its notifications are
//! the service worker's, and a tap there is `sw.js`'s to open.

import { bridge } from "./settings/bridge";

/// Follow every clicked notification's path with `go`, for as long as the page
/// is up. Returns what stops listening — which also tells the app there is no
/// page to tell, so a click after it loads the window on the path instead.
///
/// **Only a path of the workbench's own is followed.** The app checks the same
/// before it says one, and the router would read a path with two slashes in
/// front as another host just as a browser does.
export function follow(go: (path: string) => void): () => void {
  const app = bridge();

  if (app === null) {
    return () => {};
  }

  return app.opening((path) => {
    if (path.startsWith("/") && !path.startsWith("//")) {
      go(path);
    }
  });
}
