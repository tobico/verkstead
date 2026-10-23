//! Drawing the pictures in a Diff — an SVG, a PNG, any file the renderer marked
//! as one — over the placeholders the server left for them (ADR-0026).
//!
//! What it looks for is what `crates/render` wrote where a picture's hunks would
//! be: a `figure.diffPicture` per side, naming the repository, the path, the
//! blob id and the media type. The markup names the bytes and never carries
//! them, so each is fetched here — once its fold scrolls near, and once per id
//! for the life of the page — and put in an `img` over an object URL.
//!
//! **An `img`, never inline markup.** The Diff is injected unsanitised, and an
//! SVG inlined into it would run whatever script a session wrote into one.
//! Inside an `img` nothing runs and nothing it references is fetched.
//!
//! **A picture that cannot be fetched falls back to what the patch holds.** An
//! SVG's hunks travel under the pictures, hidden by the stylesheet while the
//! pictures stand; a failure takes the pictures away, which is what shows them,
//! with the server's sentence over them. A raster has no hunks, so its figure
//! becomes the sentence. A picture that fetched but will not draw is left to the
//! browser's own broken-image mark.
//!
//! **A share carries its pictures.** It is one file with no server behind it,
//! so the server writes each picture it could read into its placeholder as a
//! `data:` URI, and that is drawn without a fetch. One it could not read is a
//! placeholder like any other, and with no reader to ask it falls back.
//!
//! **A press opens a picture at full size**, over the page in a dialog painted
//! as `Modal.tsx`'s card is: the same object URL, so nothing is fetched again,
//! drawn at its own size and no larger than the window, under its side's word
//! and its path. A picture that fell back is a sentence, and opens nothing.
//!
//! None of it is Solid, for `diagrams.ts`'s reason: one routine run over the
//! injected markup by both the Set sheet and the commit pane, wherever either is
//! drawn — so the dialog is `Modal.tsx`'s native `dialog` written out by hand.

import { faXmark } from "@fortawesome/free-solid-svg-icons";

import { icon } from "./diagrams";

/// One placeholder's worth: what the bytes are fetched by, and what to draw
/// them as.
export type Picture = { repo: string; blob: string; path: string; type: string };

/// How a picture's bytes are fetched — the server's endpoint, in the page; a
/// stub, in a test. A rejection is the reason it is not there, and an Error
/// named `RefusedError` carries the server's own sentence for it.
export type Reader = (picture: Picture) => Promise<Blob>;

/// What a page hands over: the markup to draw in, and how to read a picture.
/// With no reader there is nowhere to fetch from, and every picture falls back.
export type Drawing = { root: ParentNode; read?: Reader };

/// The renderer's names, which are global — see `styles/diff.css`.
const PLACEHOLDER = "figure.diffPicture";
const SECTION = "diffPictures";
const CODE = "diffPictureCode";
const NOTE = "diffNote";

/// And this module's own, painted beside them: the box a picture is drawn in,
/// which is also the button that opens it, and the dialog it opens into.
const BOX = "diffPictureBox";
const FULL = "diffPictureFull";

/// Said where a picture was and is not any more, when the server gave no
/// sentence of its own: the blob gone with its worktree, or no server to ask.
export const GONE = "This picture is no longer there.";

/// And where the read failed without the server saying anything.
const UNREAD = "This picture couldn't be read.";

/// How far ahead of the visible page a picture is fetched: about one more
/// screenful, so a picture is there by the time it is scrolled to.
const NEAR = "100% 0px";

/// Every picture this page has fetched, by blob id, as the object URL it is
/// drawn from. The same id is the same bytes, whichever repository or device
/// it came out of, so the cache holds for the life of the page.
///
/// A read that failed is taken back out, so a later drawing may ask again.
const fetched = new Map<string, Promise<string>>();

/// Draw every picture placeholder under `root` as its fold scrolls near.
/// Returns what stops: nothing still in flight is drawn after it.
export function drawPictures(how: Drawing): () => void {
  const figures = [...how.root.querySelectorAll<HTMLElement>(PLACEHOLDER)];
  let gone = false;

  // The picture open at full size, where one is. Never two: the dialog puts
  // the page behind it out of reach.
  let open: HTMLDialogElement | null = null;

  const opened = (dialog: HTMLDialogElement) => {
    open = dialog;
    dialog.addEventListener("close", () => {
      if (open === dialog) open = null;
    });
  };

  const load = (figure: HTMLElement) => {
    const picture = placed(figure);

    // A share's placeholder carries its picture, read in as the file was
    // written: drawn as it is, with nothing to fetch.
    const carried = inlined(figure);
    if (picture !== null && carried !== null) {
      draw(figure, picture, carried, opened);
      return;
    }

    if (how.read === undefined || picture === null) {
      fail(figure, GONE);
      return;
    }

    url(how.read, picture).then(
      (src) => {
        if (!gone) draw(figure, picture, src, opened);
      },
      (error: unknown) => {
        if (!gone) fail(figure, said(error));
      },
    );
  };

  // Nothing to draw, and so nothing to watch for.
  if (figures.length === 0) {
    return () => {};
  }

  // A page that has gone takes its open picture with it: the dialog stands on
  // the body, outside anything the page unmounted.
  const stop = () => {
    gone = true;
    open?.close();
  };

  // A browser with no observer to ask is one that is drawn all at once rather
  // than not at all.
  if (typeof IntersectionObserver === "undefined") {
    figures.forEach(load);
    return stop;
  }

  const watching = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const figure = figures.find((one) => one === entry.target);
        if (!entry.isIntersecting || figure === undefined) continue;

        watching.unobserve(figure);
        load(figure);
      }
    },
    { rootMargin: NEAR },
  );

  figures.forEach((figure) => watching.observe(figure));

  return () => {
    stop();
    watching.disconnect();
  };
}

/// What a placeholder names, or `null` for one missing any of it.
function placed(figure: HTMLElement): Picture | null {
  const { repo, blob, path, type } = figure.dataset;

  if (!repo || !blob || !path || !type) return null;
  return { repo, blob, path, type };
}

/// The picture a share wrote into its placeholder, as the `data:` URI it is
/// drawn from — or `null` where it carries none, and so has to be fetched.
/// Nothing but a `data:` URI is taken: a placeholder is never a way to have
/// the page fetch something.
function inlined(figure: HTMLElement): string | null {
  const src = figure.dataset.src;
  return src?.startsWith("data:") ? src : null;
}

/// The object URL a picture is drawn from, fetched the first time its id is
/// asked for and shared by every ask after.
function url(read: Reader, picture: Picture): Promise<string> {
  const known = fetched.get(picture.blob);
  if (known !== undefined) return known;

  // The bytes arrive as a download; it is the type put on them here, off the
  // placeholder, that lets an `img` draw an SVG at all.
  const asked = read(picture).then((bytes) =>
    URL.createObjectURL(new Blob([bytes], { type: picture.type })),
  );

  fetched.set(picture.blob, asked);
  asked.catch(() => fetched.delete(picture.blob));

  return asked;
}

/// The picture, in its box on the chequerboard under its side's word. The box
/// is a button, and a press on it opens the picture at full size.
function draw(
  figure: HTMLElement,
  picture: Picture,
  src: string,
  opened: (dialog: HTMLDialogElement) => void,
): void {
  // Read before the box goes in, which has no text but would be in the way.
  const word = (figure.textContent ?? "").trim();

  const box = document.createElement("button");
  box.type = "button";
  box.className = BOX;
  box.setAttribute("aria-label", `Open ${word} ${picture.path}`.replace(/\s+/g, " "));
  box.append(img(src, word, picture.path));
  box.addEventListener("click", () => opened(full(box, word, picture.path, src)));

  figure.append(box);
}

function img(src: string, word: string, path: string): HTMLImageElement {
  const drawn = document.createElement("img");
  drawn.src = src;
  drawn.alt = `${word} ${path}`.trim();
  return drawn;
}

/// One picture at full size: a dialog over the page, opened from `from` and
/// handing the focus back to it on the way shut.
///
/// Every way out is `Modal.tsx`'s: Escape is the platform's, a press on the
/// backdrop lands on the dialog itself — which is why the card inside carries
/// the padding — and the card's own control closes it.
function full(from: HTMLElement, word: string, path: string, src: string): HTMLDialogElement {
  const dialog = document.createElement("dialog");
  dialog.className = FULL;
  dialog.setAttribute("aria-label", `${word} ${path}`.trim());

  const card = document.createElement("div");
  card.className = `${FULL}Card`;

  const head = document.createElement("div");
  head.className = `${FULL}Head`;

  const caption = document.createElement("p");
  caption.className = `${FULL}Caption`;
  const side = document.createElement("span");
  side.className = `${FULL}Word`;
  side.textContent = word;
  const where = document.createElement("span");
  where.className = `${FULL}Path`;
  where.textContent = path;
  caption.append(side, where);

  const close = document.createElement("button");
  close.type = "button";
  close.className = `${FULL}Close`;
  close.setAttribute("aria-label", "Close");
  close.append(icon(faXmark));
  close.addEventListener("click", () => dialog.close());

  head.append(caption, close);

  const box = document.createElement("div");
  box.className = `${BOX} ${FULL}Box`;
  box.append(img(src, word, path));

  card.append(head, box);
  dialog.append(card);

  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });

  // The browser hands the focus back on its own; said here as well, for a
  // dialog that is taken off the body before the browser gets to.
  dialog.addEventListener("close", () => {
    dialog.remove();
    if (from.isConnected) from.focus();
  });

  // On the body rather than in the Diff, for `diagrams.ts`'s reason: the pane
  // is scrolled and clipped in ways this should not have to know about, and
  // the box in the Diff keeps its place.
  document.body.append(dialog);
  dialog.showModal();

  return dialog;
}

/// The sentence a failed read is drawn as: the server's own, where it said one.
function said(error: unknown): string {
  return error instanceof Error && error.name === "RefusedError" && error.message
    ? error.message
    : UNREAD;
}

/// A picture that is not there, drawn as what the patch still holds.
///
/// Where the file's hunks travelled with it — an SVG — the whole section goes,
/// either side failing, and the sentence stands over the code it uncovers. A
/// raster's figure becomes the sentence, and a pair that both failed for the
/// same reason says it once.
function fail(figure: HTMLElement, sentence: string): void {
  const section = figure.closest(`.${SECTION}`);

  // Already fallen back, by the other side of the pair.
  if (section === null || !section.isConnected) return;

  if (section.nextElementSibling?.classList.contains(CODE)) {
    section.replaceWith(note(sentence));
    return;
  }

  figure.replaceWith(note(sentence));

  const notes = [...section.querySelectorAll(`.${NOTE}`)];
  const alike = notes.every((one) => one.textContent === sentence);
  if (section.querySelector("figure") === null && alike) {
    section.replaceWith(note(sentence));
  }
}

function note(sentence: string): HTMLElement {
  const line = document.createElement("p");
  line.className = NOTE;
  line.textContent = sentence;
  return line;
}

/// Forget every picture fetched, which a page never does: for the tests, whose
/// pages share this one module and so this one cache.
export function forgetPictures(): void {
  fetched.clear();
}
