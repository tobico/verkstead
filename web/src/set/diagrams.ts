//! Drawing the Diagrams on a Set's page — the one thing the browser renders for
//! itself, and the reason it is worth the exception: nothing in Rust draws a
//! mermaid graph.
//!
//! What it looks for is what the markdown renderer left behind: a `pre.mermaid`
//! holding its own source, escaped. That block is also the fallback — for a
//! bundle that never arrived, and for a diagram that will not draw — so a diagram
//! this cannot render is one it leaves alone, and the human reads the source the
//! agent wrote rather than mermaid's complaint about it.
//!
//! The bundle is asked for only by a page whose Set carries a Diagram, and asked
//! for as an import rather than as a script tag the page names: the promise is
//! the wait, so there is nothing here to coordinate two loads with.
//!
//! And every drawing that lands can be expanded: a button floating at its top
//! right, and a lightbox it opens into — the diagram over the page, letterboxed,
//! and big enough to study on the phone it is usually read on. It is here rather
//! than in a component because this is the one place a Diagram is drawn, so a
//! Set's page, a commit, a document and a share all get it without any of them
//! saying anything.
//!
//! Which is also why none of it is Solid: the share build folds this module into
//! one chunk with no framework in it, so the button and the dialog are built by
//! hand — `Icon.tsx` and `IconButton.tsx` written out in `document.createElement`,
//! and `Modal.tsx`'s native `dialog` opened with `showModal`. What paints them is
//! `styles/markdown.css`, a global sheet, which reaches a share for the same
//! reason this module does.

import {
  faDownLeftAndUpRightToCenter,
  faUpRightAndDownLeftFromCenter,
} from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/free-solid-svg-icons";

/// What this module needs of mermaid, which is two calls.
///
/// Written out rather than taken from the package so that a test can stand in
/// for the renderer: what is worth asserting about drawing a page is which blocks
/// were drawn and what became of one that would not, and none of that wants three
/// megabytes of mermaid in the room.
export type Renderer = {
  initialize(config: Config): void;
  render(id: string, text: string): Promise<{ svg: string }>;
};

/// Everything mermaid is told before it draws — see [`configured`].
export type Config = Record<string, unknown>;

/// How the renderer is fetched. The one place the bundle is named.
export type Bundle = () => Promise<Renderer>;

/// What a page hands over, and by default nothing: the bundle is mermaid's own
/// and the page is the document.
export type Drawing = { bundle?: Bundle; root?: ParentNode };

/// The block the markdown renderer wrote, and what a drawn one becomes.
const SOURCE = "pre.mermaid";
const DRAWN = "diagram";

/// What is built around a drawing that landed: the box it shares with the button
/// floating over it, that button, and the dialog it is expanded into. Named here
/// and painted in `styles/markdown.css`, there being no component either of them
/// belongs to.
const BOX = "diagram-drawing";
const BUTTON = "diagram-button";
const LIGHTBOX = "diagram-lightbox";

/// The drawing's own two measurements, put on the box for the stylesheet to size
/// it by: how wide mermaid drew it, and the shape it has to keep. What they are
/// spent on is the stylesheet's — the card caps the box at the first of them, and
/// the lightbox letterboxes with both.
const SIZE = "--diagram-size";
const RATIO = "--diagram-ratio";

/// Where an `svg` element has to be made, `createElement` making HTML.
const SVG = "http://www.w3.org/2000/svg";

/// The whole of how the page decides which colours it is in — there is no switch
/// and no stored preference, so this is also the only warning of a change.
const SCHEME = "(prefers-color-scheme: dark)";

/// Mermaid itself, dynamically imported so that only a page with a Diagram on it
/// pays for the bundle.
const bundled: Bundle = () => import("mermaid").then((module) => module.default);

/// One diagram that drew: the figure standing where its source block was, the box
/// inside it that the drawing shares with its button, that button, the source
/// itself — which the block took with it when it went — and the number the drawing
/// is named by.
///
/// Kept because the colour scheme can flip after a diagram is drawn, and drawing
/// it again is the only way to follow.
type Drawn = {
  figure: HTMLElement;
  box: HTMLElement;
  expand: HTMLButtonElement;
  text: string;
  n: number;
};

/// The diagram that is expanded, while one is: the dialog standing over the page,
/// and which drawing it is showing a copy of.
type Expanded = { dialog: HTMLDialogElement; of: Drawn };

/// How big a drawing is, in the pixels mermaid measured it at.
type Size = { width: number; height: number };

/// Draw every Diagram on the page, and keep them in the page's own colours for as
/// long as it is up. Returns what stops watching for a change of scheme.
export function drawDiagrams(how: Drawing = {}): () => void {
  const bundle = how.bundle ?? bundled;
  const root = how.root ?? document;

  // A `MediaQueryList` answers live, so the scheme is asked rather than read.
  const dark = window.matchMedia(SCHEME);

  const drawn: Drawn[] = [];

  // Mermaid stamps the id it is handed all the way through the SVG it gives back,
  // so no two drawings on the page may share one — and for the moment between a
  // redraw finishing and the drawing it replaces going away, the pair are both
  // here. The generation is what tells them apart.
  let generation = 0;

  // Mermaid measures a diagram by laying it out in the document, and it has one
  // place it does that in — so a pass over the page waits for the pass before it
  // rather than overlapping with it.
  let pass: Promise<unknown> = Promise.resolve();

  // Set once the page has gone: whatever is still queued stops, and a bundle that
  // lands after it has nothing left to draw on.
  let gone = false;

  // The drawing that is expanded, where one is. Never two: a dialog opened with
  // `showModal` puts the page behind it out of reach, so there is no second expand
  // button to press. What this is held for is the redraw, which has to find the
  // copy the lightbox is showing.
  let expanded: Expanded | null = null;

  const queue = (work: () => Promise<void>) => {
    // The same either way: a pass that threw is finished too, and the next one is
    // owed its turn regardless.
    pass = pass.then(work, work);
  };

  const id = (n: number) => `diagram-${generation}-${n}`;

  // Mermaid's drawing step, and the one place its two ways of not drawing —
  // refusing the source, and failing on it — become the same nothing.
  const rendered = async (renderer: Renderer, text: string, n: number) => {
    try {
      const { svg } = await renderer.render(id(n), text);
      return svg;
    } catch {
      return null;
    }
  };

  const drawAll = async (renderer: Renderer) => {
    // Mermaid settles its theme at init and bakes it into the SVG it hands back,
    // so every pass over the page opens by telling it what the page looks like
    // now.
    renderer.initialize(configured(dark.matches));

    const sources = [...root.querySelectorAll(SOURCE)];

    // In turn rather than all at once, for the reason the queue exists.
    for (const [n, source] of sources.entries()) {
      if (gone) return;

      // `textContent` un-escapes the source back to what the agent wrote. It is
      // kept as well as drawn: the block holding it is about to be replaced, and
      // a redraw has nowhere else to read it from.
      const text = source.textContent ?? "";
      const svg = await rendered(renderer, text, n);

      // Unparseable, or mermaid gave up drawing it. The source block stays
      // exactly as it came, and there is nothing here to redraw later either.
      if (svg === null) continue;

      // A `div` in place of the `pre`, because the stylesheet washes and boxes
      // every `pre` in rendered markdown as the code it usually is, and a drawn
      // diagram is not that.
      const figure = document.createElement("div");
      figure.className = DRAWN;

      // And a box inside it holding the drawing and the button over it. It is
      // what the button is positioned against, which is the whole of why it is
      // here: the button floats at the top right of the *drawing*, and the drawing
      // is centred in a figure that is usually wider than it.
      const box = document.createElement("div");
      box.className = BOX;

      const diagram: Drawn = {
        figure,
        box,
        expand: button(faUpRightAndDownLeftFromCenter, "Expand"),
        text,
        n,
      };
      diagram.expand.addEventListener("click", () => expand(diagram));

      put(diagram, svg);
      figure.append(box);
      source.replaceWith(figure);

      drawn.push(diagram);
    }
  };

  /// Every diagram that drew, drawn again for the scheme the page is in now. The
  /// ones that did not are still source blocks, and the stylesheet themes those
  /// along with the rest of the page, so they need nothing from here.
  const redrawAll = async (renderer: Renderer) => {
    renderer.initialize(configured(dark.matches));
    generation += 1;

    for (const diagram of drawn) {
      if (gone) return;

      const svg = await rendered(renderer, diagram.text, diagram.n);

      // A redraw that fails leaves the drawing that is already there: it is in
      // last scheme's colours, which is a worse page than this one and a much
      // better one than a hole where the diagram was.
      if (svg === null) continue;

      put(diagram, svg);

      // A lightbox open on this diagram is showing a copy of the drawing that has
      // just gone, so it takes a copy of the one that replaced it.
      if (expanded?.of === diagram) refresh(expanded);
    }
  };

  /// The copy the lightbox is showing, made again: on the way open, and again
  /// whenever the drawing under it is drawn again. A scheme that flipped while the
  /// lightbox was up would leave the expanded diagram in last scheme's colours
  /// otherwise, which is the one thing the page behind it would not be.
  const refresh = (up: Expanded) => up.dialog.replaceChildren(copied(up.of));

  /// One diagram expanded: the lightbox over the page, with a copy of the drawing
  /// letterboxed in it.
  const expand = (of: Drawn) => {
    const dialog = document.createElement("dialog");
    dialog.className = LIGHTBOX;
    // There is nothing inside the dialog but the diagram, so there is no heading
    // in it for a screen reader to name it by.
    dialog.setAttribute("aria-label", "Diagram");

    // Every press closes: the collapse button, the drawing, and the blanker around
    // it. One listener is all three — the button's press passes through here on
    // its way up, and a press on the blanker is reported as a press on the dialog
    // itself, which is the thing `Modal.tsx` says about the other dialog in the
    // app. Escape is the platform's, and so is the focus going back to the button
    // this was opened from.
    dialog.addEventListener("click", () => dialog.close());

    dialog.addEventListener("close", () => {
      expanded = null;
      dialog.remove();
    });

    const up = { dialog, of };
    refresh(up);

    // On the body rather than beside the diagram: a modal dialog is drawn in the
    // top layer wherever it stands, and the pane a Diagram is in is scrolled,
    // clipped and stacked in ways none of this should have to know about.
    document.body.append(dialog);
    dialog.showModal();
    expanded = up;
  };

  // Named before there is anything to redraw with, so that the cleanup below has
  // the same listener to take off that was put on.
  let redraw = () => {};

  void bundle().then(
    (renderer) => {
      // The page went while the bundle was still coming. Nothing was drawn and
      // nothing is watching.
      if (gone) return;

      // Listening before the first pass rather than after it, so that a flip
      // arriving while the page is still being drawn is a flip like any other.
      redraw = () => queue(() => redrawAll(renderer));
      dark.addEventListener("change", redraw);
      queue(() => drawAll(renderer));
    },
    () => {
      // A bundle that never arrives leaves every source block standing, which is
      // a readable page and not a broken one — so there is nothing to say here.
    },
  );

  return () => {
    gone = true;
    dark.removeEventListener("change", redraw);

    // A page that has gone takes its lightbox with it. The dialog stands on the
    // body rather than inside anything the page unmounted, so nothing else here
    // would have taken it off — and closing it rather than removing it is how the
    // focus finds its way back out of the top layer.
    expanded?.dialog.close();
  };
}

/// A drawing into the box it shares with its button: the SVG mermaid handed over,
/// the button back on top of it, and the two measurements the stylesheet sizes the
/// box by.
///
/// The same button rather than another one like it, because a redraw comes through
/// here too: it is what the lightbox hands the focus back to, and a button that
/// were replaced under an open dialog would be a button the focus could not go
/// back to.
function put(diagram: Drawn, svg: string): void {
  diagram.box.innerHTML = svg;
  sized(diagram.box);
  diagram.box.append(diagram.expand);
}

/// The copy of a drawing the lightbox shows.
///
/// A copy rather than the drawing itself, because the page under the blanker is
/// still the page: what the letterboxed bands show through to is the card the
/// diagram was expanded from, and a card with a hole in it where its diagram used
/// to be is not that.
///
/// Ids and all, which mermaid stamps through the SVG and through the `style`
/// element inside it. The two are the same drawing to the pixel, so every rule and
/// every `url(#…)` in either of them points at the same shapes whichever of the
/// pair it finds first. What a copy may not be is stale, which is why one is made
/// again every time the drawing under it is.
function copied(of: Drawn): HTMLElement {
  const box = document.createElement("div");
  box.className = BOX;

  const drawing = of.box.querySelector("svg");
  if (drawing !== null) {
    const copy = drawing.cloneNode(true) as SVGElement;

    // The one thing the copy is not allowed to keep. Mermaid writes the size it
    // measured onto the SVG as an inline `max-width`, which is a ceiling no
    // stylesheet can raise — and the lightbox is the one place a diagram is
    // deliberately drawn bigger than mermaid measured it.
    copy.style.maxWidth = "none";

    box.append(copy);
    sized(box);
  }

  box.append(button(faDownLeftAndUpRightToCenter, "Collapse"));
  return box;
}

/// What the stylesheet sizes a box by, read off the drawing inside it: how wide
/// mermaid drew the diagram, and the shape it has to keep.
///
/// Said as custom properties rather than as a width, because the two places a box
/// is drawn want different things of the same two numbers — the card caps the box
/// at the width, the lightbox letterboxes the window with both — and a stylesheet
/// is where that belongs. A drawing neither number can be read off is left to the
/// stylesheet's own fallbacks, which are the width it was given and no scaling up.
///
/// Asked of the box before the button is put on it, the button carrying an SVG of
/// its own that this must not measure the diagram from.
function sized(box: HTMLElement): void {
  const size = measured(box.querySelector("svg"));
  if (size === null) return;

  box.style.setProperty(SIZE, `${size.width}px`);
  box.style.setProperty(RATIO, `${size.width / size.height}`);
}

/// How big mermaid drew a diagram, in the pixels it measured it at.
///
/// Two attributes and a `viewBox`, and it takes all three because mermaid writes
/// the width as `100%` whenever a diagram has been told it may fill what it is
/// given — which is every diagram here. The height is a length either way and the
/// `viewBox` is the shape, so the width is the one that has to be worked back out.
///
/// Nothing where none of it can be read, which is not a drawing mermaid made.
function measured(svg: Element | null): Size | null {
  if (svg === null) return null;

  // A length rather than whatever else the attribute may say: `100%` is a number
  // followed by something, and reading it as one would have every such diagram
  // measured at a hundred pixels.
  const length = (name: string): number | null => {
    const said = (svg.getAttribute(name) ?? "").trim();
    const written = /^([\d.]+)(?:px)?$/.exec(said);
    const measure = written === null ? Number.NaN : Number(written[1]);

    return measure > 0 ? measure : null;
  };

  const viewBox = (svg.getAttribute("viewBox") ?? "").trim();
  const box = viewBox.split(/[\s,]+/).map(Number);
  const shape =
    box.length === 4 && box[2]! > 0 && box[3]! > 0 ? box[2]! / box[3]! : null;

  const width = length("width");
  const height = length("height");

  if (width !== null && height !== null) return { width, height };
  if (shape === null) return null;
  if (height !== null) return { width: height * shape, height };
  if (width !== null) return { width, height: width / shape };

  return null;
}

/// One button floating over a drawing: the one that expands it, and the one that
/// collapses it again.
///
/// Both are this button, which is the whole of why there is a function here: they
/// stand in the same corner, they appear the same way, and the only things telling
/// them apart are the shape inside and the word that is read aloud. Not
/// [`IconButton`](../IconButton.tsx), for two reasons: nothing in this module may
/// be a Solid component, and that button carries a selected state this one has
/// none of.
///
/// An icon says nothing when it is read aloud, so the label handed in here is the
/// whole of what the button is called.
function button(of: IconDefinition, label: string): HTMLButtonElement {
  const pressed = document.createElement("button");
  pressed.type = "button";
  pressed.className = BUTTON;
  pressed.setAttribute("aria-label", label);
  pressed.append(icon(of));

  return pressed;
}

/// One Font Awesome icon, drawn as the SVG it is — [`Icon`](../Icon.tsx) written
/// out, for the reason the button above is not `IconButton`. An icon there is data,
/// a viewBox and a path, so what is needed is an `svg` around it and nothing else.
///
/// Hidden from a screen reader, which has the button's own label to read instead.
function icon(of: IconDefinition): SVGElement {
  const drawn = document.createElementNS(SVG, "svg");
  drawn.setAttribute("viewBox", `0 0 ${of.icon[0]} ${of.icon[1]}`);
  drawn.setAttribute("aria-hidden", "true");

  // The path, or paths: a duotone icon is drawn in two, and Font Awesome hands
  // either over in the same slot.
  for (const path of [of.icon[4]].flat()) {
    if (typeof path !== "string") continue;

    const shape = document.createElementNS(SVG, "path");
    shape.setAttribute("d", path);
    drawn.append(shape);
  }

  return drawn;
}

/// The three classes an agent can put on a node to say what it did to the thing
/// the node stands for, in the colours the Diff marks the same three things in —
/// so the picture and the patch read as one account of the delta rather than two.
///
/// Handed to mermaid as CSS of its own rather than written in the stylesheet
/// beside every other rule about the page, for a reason that is entirely
/// mermaid's: what it is given it namespaces under the id of the drawing it is
/// drawing — `.node.new rect` becomes `#diagram-0-1 .node.new rect` — and an id
/// out-ranks any selector a stylesheet could write about a node from outside. The
/// colours are still the stylesheet's; only the rules that spend them are here.
function marks(ink: (name: string) => string): string {
  // What the base theme fills and outlines, because which of them a node is drawn
  // as is the diagram's business and not this module's.
  const shapes = ["rect", "circle", "ellipse", "polygon", "path"];

  // Each mark is the Diff's own pattern for a line: the wash behind it and the
  // saturated ink at its edge. `modified` is the one the Diff has no colour of its
  // own for — it marks lines, and a changed line there is an added one beside a
  // removed one — so it takes the wash that means "look at this", outlined in the
  // accent to tell it from a note, which is filled the same way.
  const mark = (name: string, wash: string, edge: string) => {
    const selector = shapes.map((shape) => `.node.${name} ${shape}`).join(", ");
    return `${selector} { fill: ${wash}; stroke: ${edge}; }`;
  };

  // Nothing about a node nobody tagged: every selector above is qualified by the
  // class it marks, so an untagged one is left to the theme.
  return [
    mark("new", ink("--added-wash"), ink("--added")),
    mark("modified", ink("--marked"), ink("--accent")),
    mark("removed", ink("--removed-wash"), ink("--removed")),
  ].join("\n");
}

/// What mermaid is told before it draws, read off the document every time it is
/// asked for rather than written out here. The stylesheet is the only thing that
/// knows what colour the page is — it has two schemes and picks between them by
/// media query — so asking the document is both how a diagram comes out on the
/// page's own palette and how it comes out on the right one of the two.
///
/// `base` because it is mermaid's only theme that is all overrides: every other
/// one brings a palette with it, and a second palette on the page is the thing
/// this is here to avoid. `darkMode` is not a colour but it decides the direction
/// mermaid derives the shades it still works out for itself, and on this page that
/// is the same question the media query answers.
function configured(dark: boolean): Config {
  const page = getComputedStyle(document.documentElement);
  const ink = (name: string) => page.getPropertyValue(name).trim();

  return {
    // `strict` because every diagram here was written by an agent and is
    // therefore untrusted: it is what has mermaid sanitize the labels it draws
    // and refuse the click handlers and inline styles a diagram can ask for.
    //
    // `startOnLoad` off because the deciding is ours. Mermaid's own pass would
    // find these same blocks and replace an unparseable one with an error
    // graphic, and the source block is the error state.
    //
    // `suppressErrorRendering` for the half of that mermaid does even when it is
    // asked one diagram at a time: a diagram it cannot parse it draws as a bomb
    // captioned "Syntax error in text", and it draws it into the document before
    // reporting the failure, so leaving the source block alone afterwards is not
    // enough to keep the page quiet. With this on, it reports and draws nothing.
    startOnLoad: false,
    securityLevel: "strict",
    suppressErrorRendering: true,

    theme: "base",
    themeCSS: marks(ink),
    themeVariables: {
      darkMode: dark,

      // Behind the drawing: a Diagram is inside a card, never on the paper.
      background: ink("--card"),

      // A node. The fenced block's wash, so a Diagram and the code around it are
      // washed alike; the softer ink for the outline, because the hairline the
      // page draws boxes with is too faint to hold a shape at this size; and the
      // page's own ink for the label inside.
      primaryColor: ink("--code-wash"),
      primaryBorderColor: ink("--ink-soft"),
      primaryTextColor: ink("--ink"),

      // The two shades mermaid reaches for when one fill is not enough. Given
      // shades of the page rather than left alone: unset, the base theme arrives
      // at them by rotating the first fill's hue, which is how a colour that is
      // on no palette at all ends up on the page.
      secondaryColor: ink("--hunk"),
      secondaryTextColor: ink("--ink"),
      tertiaryColor: ink("--card"),
      tertiaryTextColor: ink("--ink"),

      // Everything drawn between the nodes, and every label that is not inside
      // one. An edge label lands on top of the line it belongs to, so it needs
      // the page behind it to be read at all.
      lineColor: ink("--ink-soft"),
      textColor: ink("--ink"),
      edgeLabelBackground: ink("--card"),

      // A subgraph: the wash the Diff's hunks get, boxed in the page's own
      // hairline. A grouping should read as a grouping and not as one more node.
      clusterBkg: ink("--hunk"),
      clusterBorder: ink("--edge"),

      // A note is the one thing in a diagram that is there to be looked at, which
      // is the one thing `--marked` is the colour for.
      noteBkgColor: ink("--marked"),
      noteBorderColor: ink("--edge"),
      noteTextColor: ink("--ink"),

      // The page's own type, asked for rather than named, so there is one place
      // the font stack is written down. A label in a node is not prose and reads
      // better a notch down from it — mermaid wants that as a length, and it is
      // the same notch down the stylesheet takes its smaller text.
      fontFamily: getComputedStyle(document.body).fontFamily,
      fontSize: "14px",
    },
  };
}
