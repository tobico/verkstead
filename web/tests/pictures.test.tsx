//! The pictures in a Diff: a file a browser can draw, drawn rather than read
//! (ADR-0026).
//!
//! The markup is the server's own — `tests/fixtures/set-pictured.json`, an SVG
//! edited and a PNG added, written by `cargo test` from the real renderer — and
//! the bytes come from a stub here, which is the one thing the fixture names
//! without carrying.

import { cleanup, fireEvent, render, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CommitEvent, CommitPane, SetView } from "../src/api/types";
import { forgetPictures } from "../src/set/pictures";
import { Opened } from "../src/workbench/Commit";
import { mount } from "./reading";
import { askedFor, bytes, json, readable, reads, serving, whenever } from "./serving";
import pictured from "./fixtures/set-pictured.json" with { type: "json" };

const PICTURED: SetView = readable(pictured);

/// The two sides of the edited SVG and the added PNG, as the fixture names them.
const SVG_BEFORE = "3b18e512dba79e4c8300dd08aeb37f8e728b8dad";
const SVG_AFTER = "a0423896973644771497bdc03eb99d5281615b51";
const PNG_AFTER = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

/// Where the page reads one picture from.
function at(blob: string, path: string): string {
  const query = new URLSearchParams({ repo: "verkstead", id: blob, path });
  return `/api/ui/conversations/${PICTURED.conversation}/pictures?${query}`;
}

const SVG_BEFORE_AT = at(SVG_BEFORE, "assets/mark.svg");
const SVG_AFTER_AT = at(SVG_AFTER, "assets/mark.svg");
const PNG_AFTER_AT = at(PNG_AFTER, "assets/icon.png");

/// Every picture there, as the server sends one: a download.
const SERVED = [
  whenever(SVG_BEFORE_AT, bytes("<svg><rect/></svg>", "mark.svg")),
  whenever(SVG_AFTER_AT, bytes("<svg><circle/></svg>", "mark.svg")),
  whenever(PNG_AFTER_AT, bytes("\x89PNG", "icon.png")),
];

/// The observers made while a page is up, for the test to say when a picture's
/// fold has scrolled near. Only the pictures' own: the table of contents
/// watches with one too, and is told apart by the margin it watches with.
type Watcher = { seen: (targets: Element[]) => void; watched: Element[] };
let watchers: Watcher[];

function watching() {
  watchers = [];

  class Stub {
    readonly watcher: Watcher;
    constructor(
      callback: IntersectionObserverCallback,
      options?: IntersectionObserverInit,
    ) {
      const watched: Element[] = [];
      this.watcher = {
        watched,
        seen: (targets) =>
          callback(
            targets.map(
              (target) => ({ target, isIntersecting: true }) as IntersectionObserverEntry,
            ),
            this as unknown as IntersectionObserver,
          ),
      };
      if (options?.rootMargin === "100% 0px") watchers.push(this.watcher);
    }
    observe(target: Element) {
      this.watcher.watched.push(target);
    }
    unobserve(target: Element) {
      const at = this.watcher.watched.indexOf(target);
      if (at >= 0) this.watcher.watched.splice(at, 1);
    }
    disconnect() {
      this.watcher.watched.length = 0;
    }
    takeRecords() {
      return [];
    }
  }

  vi.stubGlobal("IntersectionObserver", Stub);
}

/// Every placeholder still waiting, scrolled near.
function scrollNear() {
  for (const watcher of watchers) watcher.seen([...watcher.watched]);
}

let made: Blob[];

beforeEach(() => {
  forgetPictures();
  watching();
  made = [];
  // jsdom has no object URLs; what matters is which bytes, as which type,
  // went into one.
  let n = 0;
  URL.createObjectURL = (blob: Blob) => {
    made.push(blob);
    return `blob:picture-${(n += 1)}`;
  };
  vi.stubGlobal("matchMedia", () => ({ matches: false, addEventListener() {}, removeEventListener() {} }));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

/// The Set's pane, open, with the given answers behind it.
async function opened(...answers: Parameters<typeof serving>) {
  cleanup();
  const fetching = serving(
    whenever(`/api/ui/sets/${PICTURED.id}`, json(reads(PICTURED))),
    ...answers,
  );
  const { container } = mount(String(PICTURED.id));
  await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());
  return { container, fetching };
}

/// The fold of one file.
function fold(page: ParentNode, path: string): HTMLDetailsElement {
  const found = [...page.querySelectorAll<HTMLDetailsElement>("details.diffFile")].find(
    (one) => one.querySelector(".diffPath")?.textContent === path,
  );
  expect(found, `expected a fold for ${path}`).toBeTruthy();
  return found!;
}

describe("a picture in a Set's Diff", () => {
  it("is fetched only once its fold scrolls near", async () => {
    const { container, fetching } = await opened(...SERVED);

    expect(container.querySelectorAll("figure.diffPicture")).toHaveLength(3);
    expect(askedFor(fetching, SVG_BEFORE_AT)).toBe(0);
    expect(askedFor(fetching, SVG_AFTER_AT)).toBe(0);
    expect(askedFor(fetching, PNG_AFTER_AT)).toBe(0);

    scrollNear();

    await waitFor(() => expect(container.querySelectorAll(".diffPictureBox img")).toHaveLength(3));
    expect(askedFor(fetching, SVG_BEFORE_AT)).toBe(1);
    expect(askedFor(fetching, SVG_AFTER_AT)).toBe(1);
    expect(askedFor(fetching, PNG_AFTER_AT)).toBe(1);
  });

  it("draws an edit as a Before and an After, side by side", async () => {
    const { container } = await opened(...SERVED);
    scrollNear();

    const svg = fold(container, "assets/mark.svg");
    await waitFor(() => expect(svg.querySelectorAll("img")).toHaveLength(2));

    const figures = [...svg.querySelectorAll(".diffPictures > figure.diffPicture")];
    expect(figures.map((figure) => figure.querySelector("figcaption")?.textContent)).toEqual([
      "Before",
      "After",
    ]);
    expect(figures.map((figure) => figure.querySelector("img")?.getAttribute("src"))).toEqual([
      expect.stringMatching(/^blob:/),
      expect.stringMatching(/^blob:/),
    ]);
  });

  it("draws an added file as its one picture", async () => {
    const { container } = await opened(...SERVED);
    scrollNear();

    const png = fold(container, "assets/icon.png");
    await waitFor(() => expect(png.querySelector("img")).toBeTruthy());

    expect(png.querySelectorAll("figure.diffPicture")).toHaveLength(1);
    expect(png.querySelector("figcaption")?.textContent).toBe("After");
    expect(png.querySelector(".diffNote"), "drawn, rather than said to be binary").toBeNull();
  });

  it("goes through an img on the chequerboard, never as inline markup", async () => {
    const { container } = await opened(...SERVED);
    scrollNear();

    await waitFor(() => expect(container.querySelectorAll(".diffPictureBox > img")).toHaveLength(3));
    expect(container.querySelector(".diffPictures svg"), "nothing of an SVG runs in the page").toBeNull();

    // And the bytes are put in as the type the placeholder named, which is what
    // lets an `img` draw an SVG at all.
    expect(made.map((blob) => blob.type).sort()).toEqual([
      "image/png",
      "image/svg+xml",
      "image/svg+xml",
    ]);
  });

  it("keeps the SVG's hunks under the pictures, for the stylesheet to hide", async () => {
    const { container } = await opened(...SERVED);

    const svg = fold(container, "assets/mark.svg");
    expect(svg.querySelector(".diffPictures + .diffPictureCode .diffHunk")).toBeTruthy();
  });

  it("is fetched once per id for the life of the page", async () => {
    const first = await opened(...SERVED);
    scrollNear();
    await waitFor(() => expect(first.container.querySelectorAll("img")).toHaveLength(3));

    const second = await opened(...SERVED);
    scrollNear();
    await waitFor(() => expect(second.container.querySelectorAll(".diffPictureBox img")).toHaveLength(3));

    expect(askedFor(second.fetching, SVG_BEFORE_AT), "already fetched by the first page").toBe(0);
    expect(askedFor(second.fetching, PNG_AFTER_AT)).toBe(0);
  });

  it("falls back to an SVG's hunks when it is no longer there", async () => {
    const { container } = await opened(
      whenever(SVG_BEFORE_AT, bytes("<svg/>", "mark.svg")),
      whenever(SVG_AFTER_AT, json({ error: "This picture is no longer there." }, 404)),
      whenever(PNG_AFTER_AT, bytes("\x89PNG", "icon.png")),
    );
    scrollNear();

    const svg = fold(container, "assets/mark.svg");
    await waitFor(() => expect(svg.querySelector(".diffPictures")).toBeNull());

    expect(svg.querySelector(".diffNote")?.textContent).toBe("This picture is no longer there.");
    expect(svg.querySelector("img"), "either side failing takes the pair").toBeNull();
    expect(
      svg.querySelector(".diffNote + .diffPictureCode .diffLine.add"),
      "the code is under the line, no longer behind a picture",
    ).toBeTruthy();
  });

  it("says a raster is no longer there in one line, in the server's words", async () => {
    const { container } = await opened(
      whenever(SVG_BEFORE_AT, bytes("<svg/>", "mark.svg")),
      whenever(SVG_AFTER_AT, bytes("<svg/>", "mark.svg")),
      whenever(
        PNG_AFTER_AT,
        json({ error: "This picture is larger than 2 MB, so it isn't drawn." }, 422),
      ),
    );
    scrollNear();

    const png = fold(container, "assets/icon.png");
    await waitFor(() => expect(png.querySelector("figure")).toBeNull());

    expect([...png.querySelectorAll(".diffNote")].map((note) => note.textContent)).toEqual([
      "This picture is larger than 2 MB, so it isn't drawn.",
    ]);
  });
});

describe("a picture in a commit's pane", () => {
  const COMMIT: CommitEvent = {
    id: 7,
    at: "2025-02-01T09:00:00Z",
    sha: "0123456789abcdef0123456789abcdef01234567",
    subject: "feat: a rounder mark",
    files: 2,
    insertions: 1,
    deletions: 1,
    repo: null,
    merge: false,
  } as CommitEvent;

  const PANE: CommitPane = {
    summary: null,
    diagrams: false,
    diff: PICTURED.diff[0]!.diff,
  };

  it("is drawn the way the Set's is, through the same routine", async () => {
    const read = vi.fn((picture: { blob: string }) =>
      Promise.resolve(new Blob([picture.blob])),
    );

    const { container } = render(() => (
      <Opened commit={COMMIT} pane={PANE} back={() => {}} instead={<p />} read={read} />
    ));

    expect(read).not.toHaveBeenCalled();
    scrollNear();

    await waitFor(() => expect(container.querySelectorAll(".diffPictureBox img")).toHaveLength(3));
    expect(read.mock.calls.map(([picture]) => picture.blob).sort()).toEqual(
      [SVG_BEFORE, SVG_AFTER, PNG_AFTER].sort(),
    );
  });

  it("falls back where there is nothing to read from", async () => {
    const { container } = render(() => (
      <Opened commit={COMMIT} pane={PANE} back={() => {}} instead={<p />} />
    ));
    scrollNear();

    await waitFor(() => expect(container.querySelector(".diffPictures")).toBeNull());
    expect(container.querySelector(".diffPictureCode .diffHunk")).toBeTruthy();
  });
});

/// The picture open at full size, where one is.
function full(): HTMLDialogElement | null {
  return document.querySelector<HTMLDialogElement>("dialog.diffPictureFull[open]");
}

/// The box a side of a file is drawn in, once it is drawn.
async function drawn(page: ParentNode, path: string, word: string): Promise<HTMLButtonElement> {
  const file = fold(page, path);
  let box: HTMLButtonElement | undefined;
  await waitFor(() => {
    box = [...file.querySelectorAll<HTMLButtonElement>("button.diffPictureBox")].find(
      (one) => one.closest("figure")?.querySelector("figcaption")?.textContent === word,
    );
    expect(box, `expected ${word} of ${path} drawn`).toBeTruthy();
  });
  return box!;
}

describe("a picture pressed", () => {
  it("opens at full size in a Set's Diff, with its word and path", async () => {
    const { container, fetching } = await opened(...SERVED);
    scrollNear();

    const before = await drawn(container, "assets/mark.svg", "Before");
    const src = before.querySelector("img")!.getAttribute("src");
    const diff = container.querySelector("#diff")!.innerHTML;

    fireEvent.click(before);

    const dialog = full();
    expect(dialog).toBeTruthy();
    expect(dialog!.querySelector(".diffPictureFullWord")?.textContent).toBe("Before");
    expect(dialog!.querySelector(".diffPictureFullPath")?.textContent).toBe("assets/mark.svg");
    expect(dialog!.querySelector(".diffPictureBox img")?.getAttribute("src"), "the bytes already fetched").toBe(src);

    expect(container.querySelector("#diff")!.innerHTML, "the Diff behind it is unchanged").toBe(diff);
    expect(askedFor(fetching, SVG_BEFORE_AT), "nothing fetched again").toBe(1);
  });

  it("opens either side of a pair", async () => {
    const { container } = await opened(...SERVED);
    scrollNear();

    fireEvent.click(await drawn(container, "assets/mark.svg", "After"));

    expect(full()!.querySelector(".diffPictureFullWord")?.textContent).toBe("After");
  });

  it("opens at full size in a commit's pane", async () => {
    const read = vi.fn((picture: { blob: string }) => Promise.resolve(new Blob([picture.blob])));
    const { container } = render(() => (
      <Opened
        commit={{ id: 7, sha: "0".repeat(40), subject: "feat: a mark", repo: null } as CommitEvent}
        pane={{ summary: null, diagrams: false, diff: PICTURED.diff[0]!.diff }}
        back={() => {}}
        instead={<p />}
        read={read}
      />
    ));
    scrollNear();

    fireEvent.click(await drawn(container, "assets/icon.png", "After"));

    expect(full()!.querySelector(".diffPictureFullPath")?.textContent).toBe("assets/icon.png");
    expect(read, "once, to draw it in the Diff").toHaveBeenCalledTimes(3);
  });

  const ways: [string, (dialog: HTMLDialogElement) => void][] = [
    ["Escape", () => fireEvent.keyDown(document, { key: "Escape" })],
    ["a press on the backdrop", (dialog) => fireEvent.click(dialog)],
    ["its control", (dialog) => fireEvent.click(dialog.querySelector("button[aria-label='Close']")!)],
  ];

  for (const [way, close] of ways) {
    it(`closes on ${way}, handing the focus back to the picture`, async () => {
      const { container } = await opened(...SERVED);
      scrollNear();

      const before = await drawn(container, "assets/mark.svg", "Before");
      fireEvent.click(before);
      const dialog = full()!;

      close(dialog);

      expect(full()).toBeNull();
      expect(document.querySelector("dialog.diffPictureFull"), "taken off the page").toBeNull();
      expect(document.activeElement).toBe(before);
    });
  }

  it("stays open on a press inside its card", async () => {
    const { container } = await opened(...SERVED);
    scrollNear();

    fireEvent.click(await drawn(container, "assets/mark.svg", "Before"));
    fireEvent.click(full()!.querySelector("img")!);

    expect(full()).toBeTruthy();
  });

  it("opens nothing where the picture was not drawn", async () => {
    const { container } = await opened(
      whenever(SVG_BEFORE_AT, bytes("<svg/>", "mark.svg")),
      whenever(SVG_AFTER_AT, bytes("<svg/>", "mark.svg")),
      whenever(PNG_AFTER_AT, json({ error: "This picture is no longer there." }, 404)),
    );
    scrollNear();

    const png = fold(container, "assets/icon.png");
    await waitFor(() => expect(png.querySelector("figure")).toBeNull());

    expect(png.querySelector("button"), "a sentence is not a press").toBeNull();
    fireEvent.click(png.querySelector(".diffNote")!);
    expect(full()).toBeNull();
  });
});
