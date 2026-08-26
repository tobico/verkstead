import { cleanup } from "@solidjs/testing-library";
import { afterEach } from "vitest";

// The library cleans up after itself only when vitest's globals are on, and
// they are not: an uncleaned render leaves the last test's DOM in the document
// for the next one's queries to find two of everything.
afterEach(cleanup);

// jsdom has no `matchMedia`, and xterm.js asks for one the moment a terminal is
// opened: it watches the device pixel ratio so it can redraw when a window moves
// between screens. Nothing here has screens or moves between them, so this
// answers the question and never changes its mind.
//
// A stub rather than a library, and here rather than in the one test file that
// draws a Screen: it is a gap in the environment rather than a fixture, and the
// next test to open a terminal should not have to discover it again.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: () => ({
    matches: false,
    media: "",
    onchange: null,
    // Both spellings: the listener pair was deprecated in favour of the event
    // one years ago, and xterm.js still reaches for whichever it finds.
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent: () => false,
  }),
});

// Two more gaps in the same environment, and the same reason for filling them
// here. xterm.js measures a character by drawing one on a canvas, and jsdom has
// no canvas to draw on; it also keeps the caret in view, and jsdom does not
// scroll. Both fail harmlessly and both print a paragraph about it on every run,
// which is noise over every other test in the file.
//
// `getContext` answers with nothing, which is what jsdom was going to answer
// anyway — the terminal falls back to a nominal character size, and its buffer,
// which is what the tests read, does not depend on one.
HTMLCanvasElement.prototype.getContext = () => null;

// And scrolling the window does nothing. Nothing in the viewer's suite asserts
// that it happened: the one place that scrolls the window is the table of
// contents' animated jump, and what its tests watch is `scrollIntoView`.
window.scrollTo = () => {};

// Another gap in the same environment, filled here for the same reason: jsdom
// has `PointerEvent` but none of the capture that goes with it. A drag takes
// hold of the pointer so that every move reaches the card it started on, even
// once the pointer has left it — see `Conversations.tsx`.
//
// Capture is about what a real pointer does between elements, and there is no
// real pointer here: a test fires the moves at the card itself, which is where
// they would have arrived. So these take the call and do nothing with it.
Element.prototype.setPointerCapture = () => {};
Element.prototype.releasePointerCapture = () => {};
Element.prototype.hasPointerCapture = () => false;

// A third gap in the same environment: jsdom has no layout, so it has no
// `ResizeObserver` either. The Screen of a live session watches the pane it is
// drawn in with one, to send its width up the socket — see `Screen.tsx`.
//
// Nothing here has a layout to change, so this observes and never reports. What
// the tests drive instead is the other thing that makes the pane measure itself:
// a repaint arriving, which is when the terminal it would be measured against
// first exists.
Object.defineProperty(window, "ResizeObserver", {
  writable: true,
  value: class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
});

// A fourth gap in the same environment, and the one the modal component is
// built on: jsdom has an `HTMLDialogElement` carrying nothing but the `open`
// attribute — no `showModal`, no `close`, and none of the behaviour a modal
// dialog is otherwise taken for granted for. A page drawing one would throw
// before a test could read it.
//
// So this is the platform's own contract, as much of it as anything here
// reaches for: opening marks the dialog open, Escape asks the topmost open one
// to close the way a browser does — a cancellable `cancel`, then a `close` —
// and closing fires `close` once. What is missing is what jsdom could not have
// had anyway: the top layer, `::backdrop`, the page behind going inert, and the
// focus being moved in and handed back.
const opened = new Set<HTMLDialogElement>();

function opens(this: HTMLDialogElement) {
  if (this.open) return;
  this.setAttribute("open", "");
  opened.add(this);
}

Object.assign(HTMLDialogElement.prototype, {
  show: opens,
  showModal: opens,
  close(this: HTMLDialogElement, value?: string) {
    if (!this.open) return;
    if (value !== undefined) this.returnValue = value;
    this.removeAttribute("open");
    opened.delete(this);
    this.dispatchEvent(new Event("close"));
  },
});

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;

  // The topmost, which with no top layer to consult is the last one opened.
  const dialog = [...opened].pop();
  if (dialog && dialog.dispatchEvent(new Event("cancel", { cancelable: true }))) {
    dialog.close();
  }
});

// An uncleaned render leaves its dialog in that set, and the next test's Escape
// would reach for a dialog no longer on the page.
afterEach(() => opened.clear());
