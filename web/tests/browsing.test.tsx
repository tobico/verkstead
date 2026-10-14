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
//! - **Where a browse begins.** A field standing empty asks for no path at all,
//!   and what the server answers that with is its own home. A starting point
//!   rather than a ceiling: the way back out of it is a row like any other, so
//!   nothing above the home is out of reach.
//! - **What a field looking for a repository does with one.** It marks it and
//!   stops there — the Repos' form is being filled in with a repository, so one
//!   is the end of that browse rather than another level of it.
//! - **What a field shows of what came back.** The endpoint lists the whole of a
//!   directory and each field says which of it a human sees: the files, where
//!   the field names one, and the dotfiles, where the field points at one. Rows
//!   a field cannot hold are rows offered for nothing, and a file it can hold is
//!   where its browse arrives rather than another level of it.
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
import fieldStyles from "../src/PathField.module.css";
import type { DirectoryListing } from "../src/api/types";
import chrome from "../src/picking.module.css";
import {
  browse,
  browsing,
  held,
  leading,
  listed,
  listingAt as at,
  marked,
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
/// first and then by name, dotfiles among them. The server's own home, which is
/// what a field standing empty is answered with.
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

/// And the top of the machine, which is where a browse out of the home goes on
/// to: nothing here stops at any level.
const ROOT: DirectoryListing = {
  Listed: {
    path: "/",
    entries: [{ name: "home", path: "/home", kind: "Directory" }],
  },
};

/// The levels, each answered for however often it is asked — and the home twice
/// over, being both a directory somebody may type and what the server hands back
/// to a field that has typed nothing.
function theFilesystem(...also: Array<ReturnType<typeof whenever>>) {
  return serving(
    whenever(at(null), json(HOME)),
    whenever(at("/"), json(ROOT)),
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

/// The field, with its value held where a form would hold it — and as whichever
/// kind of field the test is about: whether a repository is what it is looking
/// for, and which of what came back it shows.
function mounted(
  at = "",
  how: {
    repositories?: boolean;
    files?: boolean;
    dotfiles?: boolean;
  } = {},
) {
  const [value, setValue] = createSignal(at);

  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  render(() => (
    <QueryClientProvider client={queries}>
      <label for="where">Where</label>
      <PathField
        id="where"
        repositories={how.repositories}
        files={how.files}
        dotfiles={how.dotfiles}
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

  it("asks for no path at all when the field is empty", async () => {
    theFilesystem();
    mounted();
    await browsed();

    // Which the server answers with its own home, and it draws like any other
    // directory: the way back out at the top, and the dotfile left out.
    expect(rows(WHERE)).toEqual(["Up to /home", "src", "work"]);
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
    const backdrop = field().parentElement!.querySelector<HTMLElement>(
      `.${chrome.backdrop}`,
    )!;

    // And clear rather than washed, unlike the rows a choice comes out of: the
    // typing goes on while these are down, so the page under them is the thing
    // the human is still looking at.
    expect(backdrop.classList.contains(chrome.clear!)).toBe(true);

    fireEvent.click(backdrop);

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
      screen.getByText("There's nothing at that path."),
    );
    expect(offered(WHERE)).toEqual([]);
  });

  it("says a directory it could not read in the server's own words", async () => {
    theFilesystem(
      whenever(
        at("/root"),
        json({ Unreadable: { why: "The server can't read it: denied." } }),
      ),
    );
    mounted("/root/");

    browse(WHERE);

    await waitFor(() =>
      screen.getByText("The server can't read it: denied."),
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

    await waitFor(() => screen.getByText(/That directory couldn't be read/));
  });
});

describe("where a browse begins", () => {
  /// The home the empty field is answered with, drawn like the directory it is:
  /// there is no boundary left for it to be the edge of.
  it("opens on the server's own home", async () => {
    theFilesystem();
    mounted();
    await browsed();

    expect(rows(WHERE)).toEqual(["Up to /home", "src", "work"]);
  });

  /// And walks out of it: the row above the home goes there like any other, so
  /// a repository or an account above it is still something a browse reaches.
  it("offers the way out of the home like any other level", async () => {
    theFilesystem();
    const { value } = mounted();
    await browsed();

    tap(WHERE, "Up to /home");

    expect(value()).toBe("/home");
    await waitFor(() => expect(rows(WHERE)).toEqual(["Up to /", "ada"]));

    tap(WHERE, "Up to /");

    expect(value()).toBe("/");
    await waitFor(() => expect(rows(WHERE)).toEqual(["home"]));
  });
});

describe("a field looking for a repository", () => {
  /// Marked, because it is the thing the Repos' form is being filled in with:
  /// the row a human is looking for should not read as one more directory.
  it("draws a repository marked and the directories beside it plain", async () => {
    theFilesystem();
    mounted("/home/ada/src/", { repositories: true });
    await browsed();

    expect(rows(WHERE)).toEqual(["Up to /home/ada", "assets", "verkstead"]);
    expect(marked(WHERE)).toEqual(["verkstead"]);
  });

  /// And a leaf: the browse was going here, so a tap writes it and stops rather
  /// than asking the server what is inside a repository nobody wants to see.
  it("writes a repository into the field without opening it", async () => {
    const fetching = theFilesystem();
    const { value } = mounted("/home/ada/src/", {
      repositories: true,
    });
    await browsed();

    tap(WHERE, "verkstead");

    expect(value()).toBe("/home/ada/src/verkstead");
    expect(askedFor(fetching, at("/home/ada/src/verkstead"))).toBe(0);

    // Still on the directory holding it, filtered to the row that was taken —
    // and still down: closing is the human's here as it is everywhere else in
    // this dropdown.
    expect(rows(WHERE)).toEqual(["Up to /home/ada", "verkstead"]);
    expect(browsing(WHERE)).toBe(true);
  });

  /// And nowhere else: a field with nothing to say about a `.git` treats a
  /// repository as the directory it also is, and drills into it.
  it("leaves a repository a plain directory in a field not looking for one", async () => {
    theFilesystem(
      whenever(
        at("/home/ada/src/verkstead"),
        json({
          Listed: {
            path: "/home/ada/src/verkstead",
            entries: [
              { name: "crates", path: "/home/ada/src/verkstead/crates", kind: "Directory" },
            ],
          },
        }),
      ),
    );
    mounted("/home/ada/src/");
    await browsed();

    expect(marked(WHERE)).toEqual([]);

    tap(WHERE, "verkstead");

    await waitFor(() =>
      expect(rows(WHERE)).toEqual(["Up to /home/ada/src", "crates"]),
    );
  });
});

describe("what a field shows of what came back", () => {
  /// The fixture is a directory of four things — two directories, one of them a
  /// dotfile, a repository and a file — so what a field draws out of it is what
  /// says which of them it holds. The default is the one every field before
  /// this task had: directories, and not the hidden ones.
  it("shows the files beside the directories where a file is what it names", async () => {
    theFilesystem();
    mounted("/home/ada/src/", { files: true });
    await browsed();

    expect(rows(WHERE)).toEqual([
      "Up to /home/ada",
      "assets",
      "verkstead",
      "README.md",
    ]);
  });

  /// And a file is where such a browse arrives: there is nothing under one to
  /// look at, so the row carries no way further in — which is the difference
  /// the eye is given between the two kinds of row.
  it("draws a file as a row that leads nowhere", async () => {
    theFilesystem();
    mounted("/home/ada/src/", { files: true });
    await browsed();

    expect(leading(WHERE)).toEqual(["assets", "verkstead"]);
  });

  it("writes a file into the field without opening it", async () => {
    const fetching = theFilesystem();
    const { value } = mounted("/home/ada/src/", { files: true });
    await browsed();

    tap(WHERE, "README.md");

    expect(value()).toBe("/home/ada/src/README.md");
    expect(askedFor(fetching, at("/home/ada/src/README.md"))).toBe(0);

    // Still in the directory that holds it, filtered to the row that was
    // taken — and still down, closing being the human's here as everywhere
    // else in this dropdown.
    expect(rows(WHERE)).toEqual(["Up to /home/ada", "README.md"]);
    expect(browsing(WHERE)).toBe(true);
  });

  /// A field pointing at a dotfile shows them, and a directory that is one is
  /// still a directory: it is drilled into like any other.
  it("shows the dotfiles the endpoint always lists where the field asks for them", async () => {
    theFilesystem();
    mounted("/home/ada/src/", { dotfiles: true });
    await browsed();

    expect(rows(WHERE)).toEqual([
      "Up to /home/ada",
      ".config",
      "assets",
      "verkstead",
    ]);
  });

  /// The two are separate decisions, and a field that has made both — which is
  /// what a config file's field is — shows everything the server listed.
  it("shows the hidden files where the field asks for both", async () => {
    theFilesystem();
    mounted("/home/ada/src/", { files: true, dotfiles: true });
    await browsed();

    expect(rows(WHERE)).toEqual([
      "Up to /home/ada",
      ".config",
      "assets",
      "verkstead",
      "README.md",
    ]);
  });

  /// And a field that has made neither — every field of tasks 02 and 03 — is
  /// left as it was: the directories, without the hidden one.
  it("leaves a field that asked for neither showing the plain directories", async () => {
    theFilesystem();
    mounted("/home/ada/src/");
    await browsed();

    expect(rows(WHERE)).toEqual(["Up to /home/ada", "assets", "verkstead"]);
    expect(leading(WHERE)).toEqual(["assets", "verkstead"]);
  });
});

/// Where the rows are put, which is the one thing about them this component
/// says rather than borrows whole: the listbox's control is its own box, and
/// this one's is the input and the press beside it together.
///
/// jsdom lays nothing out, so the boxes are the ones this describe hands back
/// and what is asserted is the arithmetic over them — the same way the
/// listbox's own placing is asked about, in `picking.test.tsx`.
describe("where the rows are put", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  /// Off the row rather than off the input inside it: a list that stopped at the
  /// input's edge would stop short of the press it was dropped from, and the
  /// paths these rows read are long enough to want every pixel of the box.
  it("takes the width of the row, not of the input inside it", async () => {
    window.innerWidth = 1000;
    window.innerHeight = 1000;

    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
      function (this: Element): DOMRect {
        // The row: the whole of the control, press included.
        if (this.classList.contains(fieldStyles.field!)) {
          return { top: 100, bottom: 140, left: 24, width: 300 } as DOMRect;
        }

        // And the input, which is what is left of it after the press.
        if (this.tagName === "INPUT") {
          return { top: 100, bottom: 140, left: 24, width: 262 } as DOMRect;
        }

        return { top: 0, bottom: 0, height: 0, width: 0 } as DOMRect;
      },
    );

    theFilesystem();
    mounted("/home/ada/");
    await browsed();

    expect(listed(WHERE).style.width).toBe("300px");
    expect(listed(WHERE).style.left).toBe("24px");
    expect(listed(WHERE).style.top).toBe("140px");
  });
});
