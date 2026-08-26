/* @refresh reload */
import { render } from "solid-js/web";

import { App } from "./App";
import { registerWorker } from "./push/worker";

// Imported here rather than linked from the document so that vite hashes them
// into the bundle like everything else, and they are therefore cached the same
// way — see the server's `viewer` module for what that buys.
//
// These three are the whole of the app's global styling: the sheets that can
// never belong to a component — the tokens and element defaults everything is
// drawn over, and the rules that meet markup nobody here writes, the server's
// rendered markdown, mermaid's diagrams, the renderer's diffs. Everything else
// is a `*.module.css` beside the component that draws it.
//
// The base first, because the rest of the app is written over it; the other two
// after it, because where one of their rules and a component's rule match the
// same element — a fenced block inside a transcript, say — the later of the two
// wins, and that was these.
import "./styles/base.css";
import "./styles/markdown.css";
import "./styles/diff.css";

const app = document.getElementById("app");
if (!app) {
  throw new Error("index.html has no #app to mount into");
}

// Before the mount rather than after it: nothing on the page waits on the
// worker, and the notifications switch waits on the registration being in
// control, so the sooner it is asked for the sooner the switch can answer.
registerWorker();

render(() => <App />, app);
