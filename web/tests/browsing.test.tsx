//! The browse dropdown a path is written with: what steers it, what a tap on a
//! row does, and how a browse ends.
//!
//! Driven straight rather than through a page, the way the listbox's own tests
//! are — what is asked here is the control's own, so no settings query and no
//! save is in the way of the answer. The three fields that draw it on the
//! settings page are asked about where those fields are, in `paths.test.tsx`.
//!
//! Three things are worth a test rather than a reading of the source:
//!
//! - **What the rows are.** The dropdown is the entries of the deepest
//!   directory the field's own text names, filtered by whatever follows the
//!   last separator. Nothing else says which directory is being looked at, so
//!   a wrong reading of the text is a dropdown showing somebody another
//!   directory entirely.
//! - **What a tap does.** It writes the path into the field *and* opens it.
//!   Both, in one press: a tap that only wrote would leave the browse where it
//!   was, and one that only opened would leave the field saying nothing about
//!   where the human had got to.
//! - **How a browse ends.** The human closes it and the field keeps whatever it
//!   holds. There is no picking here, so a close that changed the field would
//!   be the one way to leave with something nobody typed or tapped.
//!
//! The listing of `/home/ada/src` is the fixture the server's own tests wrote,
//! so what the rows are drawn from is the shape the endpoint really answers
//! with — a repository among the directories, a file and a dotfile among the
//! rows to be left out. The levels above it are written here, being nothing but
//! more of the same shape.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PathField } from "../src/PathField";
import type { DirectoryListing } from "../src/api/types";
import chrome from "../src/picking.module.css";
import {
  browse,
  browsing,
  held,
  listed,
  listingAt as at,
  offered,
  pathField,
  rows,
  tap,
  walked,
} from "./fields";
import { askedFor, json, serving, whenever } from "./serving";
import listing from "./fixtures/directories.json" with { type: "json" };

/// `/home/ada/src` as the server answered for it: two directories, one of them
/// a repository, and the file and the dotfile a field showing directories
/// leaves out.
const SRC = listing as DirectoryListing;

/// The level above it, written the way the server writes one — directories
/// first and then by name, dotfiles among them.
const HOME: DirectoryListing = {
  Listed: {
    path: "/home/ada",
    entries: [
      { name: ".cache", path: "/home/ada/.cache", kind: "Directory" },
      { name: "src", path: "/home/ada/src", kind: "Directory" },
      { name: "work", path: "/home/ada/work", kind: "Directory" },
      { name: "notes.md", path: "/home/ada/notes.md", kind: "File" },
    ],
  },
};

/// And the top of the anywhere scope, which is what an empty field asks for.
const ROOT: DirectoryListing = {
  Listed: {
    path: "/",
    entries: [{ name: "home", path: "/home", kind: "Directory" }],
  },
};

/// The three levels, each answered for however often it is asked.
function theFilesystem(...also: Array<ReturnType<typeof whenever>>) {
  return serving(
    whenever(at(null), json(ROOT)),
    whenever(at("/home"), json({ Listed: { path: "/home", entries: [
      { name: "ada", path: "/home/ada", kind: "Directory" },
    ] } })),
    whenever(at("/home/ada"), json(HOME)),
    whenever(at("/home/ada/src"), json(SRC)),
    ...also,
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/// The field, with its value held where a form would hold it.
function mounted(at = "") {
  const [value, setValue] = createSignal(at);

  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  render(() => (
    <QueryClientProvider client={queries}>
      <label for="where">Where</label>
      <PathField
        id="where"
        scope="anywhere"
        value={value()}
        write={setValue}
      />
    </QueryClientProvider>
  ));

  return { value };
}

/// The one field every test here drives, by the label that names it.
const WHERE = "Where";

const field = (): HTMLInputElement => pathField(WHERE);

/// Drop the rows, and wait for the level they come out of.
async function browsed(): Promise<void> {
  browse(WHERE);
  await waitFor(() => expect(offered(WHERE).length).toBeGreaterThan(0));
}

describe("what the field says it is", () => {
  it("is a text field the label reaches, drawn as a combobox", () => {
    theFilesystem();
    mounted();

    expect(field().tagName).toBe("INPUT");
    expect(field().getAttribute("role")).toBe("combobox");
    expect(field().getAttribute("aria-expanded")).toBe("false");
  });

  it("names the list it drops, and only while it is down", async () => {
    theFilesystem();
    mounted("/home/ada/");

    expect(field().getAttribute("aria-controls")).toBeNull();

    await browsed();

    expect(listed(WHERE).getAttribute("role")).toBe("listbox");
    expect(browsing(WHERE)).toBe(true);

    fireEvent.keyDown(field(), { key: "Escape" });

    expect(field().getAttribute("aria-controls")).toBeNull();
  });
});

describe("what the rows are", () => {
  /// The field's own text is the whole of what says which directory this is,
  /// and the fixture is what says which of its entries a directory field draws.
  it("shows the directories of the level the text names", async () => {
    theFilesystem();
    mounted("/home/ada/src/");
    await browsed();

    // The repository is one of the directories — a field with nothing to say
    // about a `.git` treats it as what it also is — and the file and the
    // dotfile are not rows here at all.
    expect(rows(WHERE)).toEqual(["Up to /home/ada", "assets", "verkstead"]);
  });

  it("takes the deepest directory the text names, not the text", async () => {
    theFilesystem();
    mounted("/home/ada/sr");
    await browsed();

    expect(rows(WHERE)).toEqual(["Up to /home", "src"]);
  });

  /// Typing steers it: the segment after the last separator is a path halfway
  /// through being written, and it filters the rows of the level above it.
  it("filters by what has been typed of the last segment", async () => {
    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    expect(rows(WHERE)).toEqual(["Up to /home", "src", "work"]);

    fireEvent.input(field(), { target: { value: "/home/ada/w" } });

    expect(rows(WHERE)).toEqual(["Up to /home", "work"]);
  });

  it("asks for the top of the scope when the field is empty", async () => {
    theFilesystem();
    mounted();
    await browsed();

    // Nothing above `/`, so no way back out of it.
    expect(rows(WHERE)).toEqual(["home"]);
  });

  /// One directory per level and no walking: the filter moves over rows already
  /// read, so a segment typed a character at a time costs one request and not
  /// one per character.
  it("asks the server once for a level, however much is typed in it", async () => {
    const fetching = theFilesystem();
    mounted("/home/ada/");
    await browsed();

    expect(askedFor(fetching, at("/home/ada"))).toBe(1);

    fireEvent.input(field(), { target: { value: "/home/ada/w" } });
    fireEvent.input(field(), { target: { value: "/home/ada/wo" } });

    expect(askedFor(fetching, at("/home/ada"))).toBe(1);
  });
});

describe("what a tap on a row does", () => {
  it("writes the path into the field and opens it", async () => {
    theFilesystem();
    const { value } = mounted("/home/ada/");
    await browsed();

    tap(WHERE, "src");

    // Both halves of the one press: the field says where the human has got to,
    // and the rows say what is there.
    expect(value()).toBe("/home/ada/src");
    expect(held(WHERE)).toBe("/home/ada/src");
    await waitFor(() =>
      expect(rows(WHERE)).toEqual(["Up to /home/ada", "assets", "verkstead"]),
    );
  });

  it("shallows both again from the row back out", async () => {
    theFilesystem();
    const { value } = mounted("/home/ada/src/");
    await browsed();

    tap(WHERE, "Up to /home/ada");

    expect(value()).toBe("/home/ada");
    await waitFor(() =>
      expect(rows(WHERE)).toEqual(["Up to /home", "src", "work"]),
    );
  });

  /// The rows stay down: a browse ends when the human says it does, and a tap
  /// is a step of one rather than the end of it.
  it("leaves the rows down", async () => {
    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    tap(WHERE, "src");

    expect(browsing(WHERE)).toBe(true);
  });
});

describe("how a browse ends", () => {
  it("closes on the backdrop, leaving the field as it stands", async () => {
    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    tap(WHERE, "src");
    await waitFor(() => expect(rows(WHERE)).toContain("assets"));

    // The listbox's own backdrop, which is what a press anywhere but on the
    // rows lands on.
    fireEvent.click(
      field().parentElement!.querySelector<HTMLElement>(`.${chrome.backdrop}`)!,
    );

    expect(browsing(WHERE)).toBe(false);
    expect(held(WHERE)).toBe("/home/ada/src");
  });

  it("closes on Escape the same way", async () => {
    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    fireEvent.input(field(), { target: { value: "/home/ada/wo" } });
    fireEvent.keyDown(field(), { key: "Escape" });

    expect(browsing(WHERE)).toBe(false);
    expect(held(WHERE)).toBe("/home/ada/wo");
  });
});

describe("the keyboard", () => {
  it("drops the rows on the way down into them", async () => {
    theFilesystem();
    mounted("/home/ada/");

    fireEvent.keyDown(field(), { key: "ArrowDown" });

    await waitFor(() => expect(browsing(WHERE)).toBe(true));
  });

  it("walks the rows and says which one it is on", async () => {
    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    expect(walked(WHERE)).toBe("Up to /home");

    fireEvent.keyDown(field(), { key: "ArrowDown" });
    expect(walked(WHERE)).toBe("src");

    fireEvent.keyDown(field(), { key: "End" });
    expect(walked(WHERE)).toBe("work");

    fireEvent.keyDown(field(), { key: "ArrowUp" });
    expect(walked(WHERE)).toBe("src");
  });

  it("takes the walked row on Enter", async () => {
    theFilesystem();
    const { value } = mounted("/home/ada/");
    await browsed();

    fireEvent.keyDown(field(), { key: "ArrowDown" });
    fireEvent.keyDown(field(), { key: "Enter" });

    expect(value()).toBe("/home/ada/src");
  });
});

describe("what the server said instead of rows", () => {
  /// Every one of these is the ordinary state of a field halfway through being
  /// typed into, so the dropdown says it where its rows would be rather than
  /// drawing anything as a failure.
  it("says a path with nothing at it, and offers nothing", async () => {
    theFilesystem(whenever(at("/home/ad"), json("Missing")));
    mounted("/home/ad/");

    browse(WHERE);

    await waitFor(() =>
      screen.getByText("There is nothing at that path."),
    );
    expect(offered(WHERE)).toEqual([]);
  });

  it("says a directory it could not read in the server's own words", async () => {
    theFilesystem(
      whenever(
        at("/root"),
        json({ Unreadable: { why: "the server cannot read it: denied" } }),
      ),
    );
    mounted("/root/");

    browse(WHERE);

    await waitFor(() =>
      screen.getByText("the server cannot read it: denied"),
    );
  });

  it("says a read that never landed as the failure it is", async () => {
    theFilesystem(
      whenever(at("/home/ada"), () =>
        Promise.resolve(
          new Response("nope", { status: 503, statusText: "Unavailable" }),
        ),
      ),
    );
    mounted("/home/ada/");

    browse(WHERE);

    await waitFor(() => screen.getByText(/Could not read that directory/));
  });
});
