//! The Diff on a Set's page: the evidence for what the agent is asking about,
//! rendered and highlighted by the server and folded per file by the browser.
//!
//! The payload every Set here is drawn from comes out of `tests/fixtures/`,
//! which `cargo test` writes from the real `/api/ui/sets/{id}` — so the markup
//! being injected, the anchors on it and the paths beside it are the server's
//! own answers rather than a mock's agreement with this file.

import { waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SetView } from "../src/api/types";
// The wrap control, which is the one on/off switch this page draws twice.
import app from "../src/App.module.css";
// The Diff section itself, whose two classes are the whole of what wrapping is.
import styles from "../src/set/Diff.module.css";
import toggle from "../src/Switch.module.css";
import { mount, reading, texts } from "./reading";
import { json, readable, reads, serving, whenever } from "./serving";
import alongside from "./fixtures/set-alongside.json" with { type: "json" };
import answered from "./fixtures/set-answered.json" with { type: "json" };
import answering from "./fixtures/set-answering.json" with { type: "json" };

/// The Set with a Diff attached, and one without: two files, one of them a Rust
/// file the server has highlighted.
const WAITING = readable(answering);
const UNDIFFED = readable(answered);

/// And one asked with a read-write companion beside the work: the same two
/// files, and a third out of the other repository.
const ALONGSIDE = readable(alongside);

/// Where the wrap setting is kept — the key `src/device.ts` writes, asked for
/// here by the name a browser would find it under.
const WRAP = "verkstead.diff-wrap";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

/// The Diff section of a page, which every test here has to have found.
function diffOf(page: ParentNode): HTMLElement {
  const diff = page.querySelector<HTMLElement>(`section.${styles.diff}`);
  expect(diff, "expected the Diff section").toBeTruthy();
  return diff as HTMLElement;
}

describe("the attached Diff", () => {
  it("is put in as the server rendered it, one fold per file", async () => {
    const diff = diffOf(await reading(WAITING));

    const folds = diff.querySelectorAll<HTMLDetailsElement>("details.diffFile");
    expect(folds, "one fold per file, whatever git knew of it").toHaveLength(2);
    expect([...folds].map((fold) => fold.id)).toEqual(["diff-1", "diff-2"]);
    expect(texts(diff, "details.diffFile .diffPath")).toEqual([
      "src/limits.rs",
      "notes.txt",
    ]);
  });

  it("keeps the colouring the server put on it", async () => {
    // The browser gets no diff parser and no highlighter: both are the
    // server's, and this is the whole of what arrives.
    const diff = diffOf(await reading(WAITING));

    expect(diff.querySelector(".diffLine.add")).toBeTruthy();
    expect(diff.querySelector(".diffLine.del")).toBeTruthy();
    expect(
      diff.querySelector("span[class^='tok-']"),
      "expected the Rust file highlighted server-side",
    ).toBeTruthy();
  });

  it("is named by a heading a jump can land on", async () => {
    const diff = diffOf(await reading(WAITING));

    expect(diff.id).toBe("diff");
    expect(diff.querySelector(`h2.${app.sectionHeading}`)!.textContent).toBe(
      "Diff",
    );
  });

  it("folds and unfolds each file", async () => {
    const diff = diffOf(await reading(WAITING));
    const [first] = diff.querySelectorAll<HTMLDetailsElement>("details.diffFile");

    // Open as the server wrote them: a Diff is evidence, and evidence nobody
    // can see until they open it is not being shown.
    expect(first!.open).toBe(true);

    // The fold is the browser's own `details`, so what a test can ask is that
    // the page is still one — a summary to press, and the file's lines inside
    // it.
    expect(first!.querySelector("summary .diffPath")!.textContent).toBe(
      "src/limits.rs",
    );
    first!.open = false;
    expect(first!.open, "and it takes being shut again").toBe(false);
  });

  /// A fold is DOM state and nothing else, and a Nudge re-reads an open Set —
  /// its answers change, so it cannot be frozen the way a commit's diff can.
  /// The Set's query reconciles each read into what is drawn instead, so a
  /// Diff whose markup did not change is left the element it was, folds and
  /// all — even when the read brought something else about the Set back
  /// different, which is what a Nudge fires for.
  it("keeps a fold the reader closed across a Nudge", async () => {
    // The re-read comes back changed somewhere that is not the Diff, so the
    // test can wait for the change to be drawn rather than guess when the
    // read landed.
    const RETITLED: SetView = { ...WAITING, title: "Rate limiting, again" };
    let standing = WAITING;
    serving(
      whenever(`/api/ui/sets/${WAITING.id}`, () => json(reads(standing))()),
    );
    const { container, client } = mount(String(WAITING.id));
    await waitFor(() => expect(container.querySelector("h1")).toBeTruthy());

    const first = container.querySelector<HTMLDetailsElement>(
      "details.diffFile",
    )!;
    first.open = false;

    standing = RETITLED;
    await client.invalidateQueries();

    // The read landed — the new title is up — and the fold is the element it
    // was, still shut.
    await waitFor(() =>
      expect(container.querySelector("h1")!.textContent).toBe(RETITLED.title),
    );
    expect(container.querySelector("details.diffFile")).toBe(first);
    expect(first.open).toBe(false);
  });

  it("shows none of its chrome on a Set that has no Diff", async () => {
    const page = await reading(UNDIFFED);

    expect(page.querySelector(`section.${styles.diff}`)).toBeNull();
    expect(page.querySelector("#diff")).toBeNull();
    expect(page.querySelector("#diff-1")).toBeNull();
    expect(
      texts(page, `h2.${app.sectionHeading}`),
      "with no Diff there is no heading to draw either — the closing section " +
        "is headed for the box it holds, this Set having no Postscript",
    ).toEqual(["Preface", "Questions", "Comment"]);
    expect(
      page.querySelector(`.${toggle.switch}`),
      "and nowhere for word wrap to belong: it governs a Diff, and there is none",
    ).toBeNull();
  });
});

describe("a Diff of more than one repository", () => {
  it("draws a block per repository, each named and in the order composed", async () => {
    const diff = diffOf(await reading(ALONGSIDE));

    expect(
      texts(diff, `h3.${styles.repo}`),
      "the work's own repository first, then the companion it was asked beside",
    ).toEqual(["verkstead", "askance"]);
  });

  it("anchors the folds across the blocks rather than restarting at each", async () => {
    const diff = diffOf(await reading(ALONGSIDE));

    const folds = diff.querySelectorAll<HTMLDetailsElement>("details.diffFile");
    expect(
      [...folds].map((fold) => fold.id),
      "the ids are one page's, so a jump lands on the fold it names",
    ).toEqual(["diff-1", "diff-2", "diff-3"]);
    expect(texts(diff, "details.diffFile .diffPath")).toEqual([
      "src/limits.rs",
      "notes.txt",
      "src/set.rs",
    ]);
  });

  it("is one Diff section, whatever it is made of", async () => {
    const page = await reading(ALONGSIDE);

    expect(page.querySelectorAll(`section.${styles.diff}`)).toHaveLength(1);
    expect(
      texts(page, `h2.${app.sectionHeading}`).filter((name) => name === "Diff"),
      "one heading over the lot: the blocks are inside it",
    ).toEqual(["Diff"]);
  });

  it("leaves one repository's changes unlabeled", async () => {
    const diff = diffOf(await reading(WAITING));

    expect(
      diff.querySelector(`h3.${styles.repo}`),
      "a Diff of one block is the work's own repository, and the label earns " +
        "its place when repos mix",
    ).toBeNull();
  });
});

describe("word wrap", () => {
  /// The switch beside the Diff's heading.
  function wrapSwitch(page: ParentNode): HTMLInputElement {
    const found = page.querySelector<HTMLInputElement>(
      `section.${styles.diff} .${toggle.switch} input`,
    );
    expect(
      found,
      "expected the wrap switch beside the Diff's heading",
    ).toBeTruthy();
    return found as HTMLInputElement;
  }

  it("is offered beside the Diff heading, and off until it is asked for", async () => {
    const page = await reading(WAITING);
    const flip = wrapSwitch(page);

    expect(flip.getAttribute("role")).toBe("switch");
    expect(flip.checked).toBe(false);
    expect(diffOf(page).className).toBe(styles.diff);
  });

  it("wraps the Diff, and remembers it for the next one", async () => {
    const page = await reading(WAITING);

    wrapSwitch(page).click();

    expect(
      diffOf(page).className,
      "wrapping is a class and nothing more: the Diff arrived rendered",
    ).toBe(`${styles.diff} ${styles.wrapped}`);
    expect(
      localStorage.getItem(WRAP),
      "the setting governs every Diff, so it is the device that remembers it",
    ).toBe("on");
  });

  it("comes back on for a device that asked for it", async () => {
    localStorage.setItem(WRAP, "on");

    const page = await reading(WAITING);

    expect(diffOf(page).className).toBe(`${styles.diff} ${styles.wrapped}`);
    expect(wrapSwitch(page).checked).toBe(true);
  });

  it("leaves nothing behind when it is turned back off", async () => {
    localStorage.setItem(WRAP, "on");
    const page = await reading(WAITING);

    wrapSwitch(page).click();

    expect(diffOf(page).className).toBe(styles.diff);
    expect(
      localStorage.getItem(WRAP),
      "the absence is already the default, so off is nothing kept",
    ).toBeNull();
  });

  it("costs the page nothing when the device has no storage to remember it in", async () => {
    // A browser that refuses storage costs the human their settings and nothing
    // else: the Diff is still drawn, and the switch still works for this visit.
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("refused");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("refused");
    });

    const page = await reading(WAITING);
    expect(diffOf(page).className).toBe(styles.diff);

    wrapSwitch(page).click();
    expect(diffOf(page).className).toBe(`${styles.diff} ${styles.wrapped}`);

    vi.restoreAllMocks();
  });
});
