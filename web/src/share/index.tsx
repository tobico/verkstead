/* @refresh reload */
import { render } from "solid-js/web";

import { boarded } from "./bundle";
import { Share } from "./Share";

// The same three global sheets the app mounts, and imported here for the same
// reason: vite carries them into the bundle rather than leaving them as a link
// to a file. Which in a share is not an optimisation but the whole arrangement
// — the build inlines what it is handed, and a stylesheet the document *linked*
// would be a request to somewhere the moment the file was opened.
//
// In the order the app has them: the tokens and element defaults everything is
// drawn over, then the two that meet markup nobody here writes — the server's
// rendered markdown, and the renderer's diffs — so that where one of those and a
// component's rule match the same element, the later of the two wins.
import "../styles/base.css";
import "../styles/markdown.css";
import "../styles/diff.css";

// No service worker, and nothing that would register one: a share is a file
// rather than an installed app, and a worker asking to control the paths under
// wherever the recipient happens to have put it is the opposite of what this is.
function mount(): void {
  const app = document.getElementById("app");
  if (!app) {
    throw new Error("share.html has no #app to mount into");
  }

  render(() => <Share shared={boarded()} />, app);
}

// Waited for rather than assumed. The build folds this script into the document
// and vite writes the tag into the head, where an inline script runs the moment
// it is parsed — `defer` means nothing on one — so at this point the body and
// the `#app` inside it may not exist yet. The app's own entry has no such
// problem: its tag names a file, and a module that names a file is deferred.
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", mount, { once: true });
} else {
  mount();
}
