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
