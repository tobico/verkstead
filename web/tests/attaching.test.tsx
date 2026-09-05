//! The files the human puts on a draft: the paperclip that picks one, the row
//! of pills the composer draws them as, and the × that takes one off again.
//!
//! And the drop that does the same thing without a picker: the whole box is
//! the target, it highlights while a drag carrying files is over it, and a
//! folder dragged along with them is skipped without a word.
//!
//! And what becomes of the row once the Brief has frozen: it moves off the
//! composer, which is gone, onto the Brief's own pane — the same pills, with
//! each of them saying how large its file is and none of them offering
//! anything to press.
//!
//! Over the golden fixtures like every other component test here, which is what
//! makes the pills a drawing of what the server really says: the drafting
//! Conversation in `conversation.json` carries two attached files, so what these
//! read is the row the app builds from the endpoint's own answer.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  AttachmentView,
  BriefEvent,
  ConversationView,
} from "../src/api/types";
import { sized } from "../src/Attaching";
import pill from "../src/Attaching.module.css";
import briefPane from "../src/workbench/Brief.module.css";
import composer from "../src/workbench/Composer.module.css";
import shell from "../src/Panes.module.css";
import {
  ATTACHMENT_REMOVAL_REFUSAL,
  ATTACH_REFUSAL,
} from "../src/workbench/Composer";
import { OPEN, drawn, mount, theWorkbench } from "./bench";
import { carrying, carryingNothing, drag, dropOn } from "./dragging";
import { json, serving, whenever } from "./serving";
import adopting from "./fixtures/conversation-adopting.json" with { type: "json" };
import grilling from "./fixtures/conversation-grilling.json" with { type: "json" };

/// The adopting draft, whose Brief arrives frozen: the one composer in the
/// fixtures where the files are settled rather than the human's.
const ADOPTING = adopting as ConversationView;

/// The two files the drafting fixture is holding.
const ATTACHED: AttachmentView[] = OPEN.attachments;

/// A Conversation whose work has started, which is where the row is read as a
/// record: its Brief froze when the grilling did, and the file on it froze with
/// it.
const FROZEN = grilling as ConversationView;

afterEach(() => vi.unstubAllGlobals());

/// The composer, opened on a Conversation whose record is the Brief and nothing
/// else — so the landing opens this pane with nothing to press.
async function openComposer(at: ConversationView = OPEN): Promise<HTMLElement> {
  const { container } = mount(`/conversations/${at.id}`);
  return drawn(container, `.${shell.detailsPane} .${composer.composer}`);
}

/// Every pill the row is drawing, in the order it has them.
function pills(pane: ParentNode): HTMLElement[] {
  return [...pane.querySelectorAll<HTMLElement>(`.${pill.attachment}`)];
}

/// The names on them.
function names(pane: ParentNode): string[] {
  return pills(pane).map((one) =>
    one.querySelector(`.${pill.attachmentName}`)!.textContent!.trim(),
  );
}

/// The hidden picker the paperclip reaches, and one file chosen through it.
function choose(pane: ParentNode, ...files: File[]): void {
  const picker = pane.querySelector<HTMLInputElement>('input[type="file"]')!;

  Object.defineProperty(picker, "files", {
    configurable: true,
    value: files,
  });

  fireEvent.change(picker);
}

/// Every upload the page made, whatever it was of: an attach is the one POST
/// under `/attachments/`, so a page that attached nothing has none of them.
function attaches(
  fetching: ReturnType<typeof serving>,
): Array<RequestInit | undefined> {
  return fetching.mock.calls
    .filter(
      ([asked, init]) =>
        String(asked).includes("/attachments/") && init?.method === "POST",
    )
    .map(([, init]) => init);
}

/// What the page put on the wire as a POST to `path`.
function writes(
  fetching: ReturnType<typeof serving>,
  path: string,
): Array<RequestInit | undefined> {
  return fetching.mock.calls
    .filter(([asked, init]) => String(asked) === path && init?.method === "POST")
    .map(([, init]) => init);
}

describe("the files on a draft", () => {
  it("draws one pill per attached file, in the order the server sent them", async () => {
    theWorkbench();

    const pane = await openComposer();

    expect(names(pane)).toEqual(ATTACHED.map((attachment) => attachment.name));
  });

  /// The row is named for whoever is not looking at it, and each × is named for
  /// the file it takes: a line of names with an unnamed × on each is a control
  /// a screen reader cannot tell from the one beside it.
  it("names the row and every remove press on it", async () => {
    theWorkbench();

    await openComposer();

    expect(screen.getByRole("list", { name: "Attached files" })).toBeTruthy();
    for (const attachment of ATTACHED) {
      expect(
        screen.getByRole("button", { name: `Remove ${attachment.name}` }),
      ).toBeTruthy();
    }
  });

  /// Inside the box, between the brief and the setup row — which is what the
  /// row is *for*: the files are part of what is being written rather than
  /// something under it.
  it("stands inside the box, under the text", async () => {
    theWorkbench();

    const pane = await openComposer();
    const box = pane.querySelector(`.${composer.box}`)!;
    const row = box.querySelector(`.${pill.attachments}`)!;

    expect(row).toBeTruthy();
    expect(
      row.compareDocumentPosition(box.querySelector("textarea")!) &
        Node.DOCUMENT_POSITION_PRECEDING,
    ).toBeTruthy();
  });

  /// The paperclip is a button over the browser's own picker, and pressing it
  /// is what opens one: an `<input type="file">` in the row would be a control
  /// of the platform's choosing with a word beside it.
  it("opens the browser's picker from the paperclip", async () => {
    theWorkbench();

    const pane = await openComposer();
    const picker = pane.querySelector<HTMLInputElement>('input[type="file"]')!;
    const opened = vi.spyOn(picker, "click");

    fireEvent.click(screen.getByRole("button", { name: "Attach a file" }));

    expect(opened).toHaveBeenCalled();
  });

  /// At the near edge of the row the start press is at the far edge of, which
  /// is a fact about where it stands rather than about what it does.
  it("stands at the near edge of the row the start press is at the far edge of", async () => {
    theWorkbench();

    const pane = await openComposer();
    const row = pane.querySelector(`.${composer.presses}`)!;

    expect(row.firstElementChild!.getAttribute("aria-label")).toBe(
      "Attach a file",
    );
    expect(row.lastElementChild!.textContent).toContain("Start work");
  });

  /// One request per file, the bytes as the body and the name in the path.
  it("sends every chosen file on a request of its own", async () => {
    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/notes.md`,
        json({ Attached: { attachment: { id: 9, name: "notes.md", bytes: 4, origin: "Brief" } } }),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/shot.png`,
        json({ Attached: { attachment: { id: 10, name: "shot.png", bytes: 3, origin: "Brief" } } }),
        "POST",
      ),
    );

    const pane = await openComposer();

    choose(
      pane,
      new File(["note"], "notes.md"),
      new File(["png"], "shot.png"),
    );

    await waitFor(() => {
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/attachments/notes.md`),
      ).toHaveLength(1);
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/attachments/shot.png`),
      ).toHaveLength(1);
    });
  });

  /// A pill on its way up is drawn dimmed and carries no ×: the file has been
  /// chosen and there is nothing to press on it yet.
  it("draws a chosen file dimmed until the record comes back", async () => {
    let land: (() => void) | null = null;

    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/notes.md`,
        () =>
          new Promise<Response>((settle) => {
            land = () =>
              settle(
                new Response(
                  JSON.stringify({
                    Attached: {
                      attachment: {
                        id: 9,
                        name: "notes.md",
                        bytes: 4,
                        origin: "Brief",
                      },
                    },
                  }),
                  { headers: { "content-type": "application/json" } },
                ),
              );
          }),
        "POST",
      ),
    );
    expect(fetching).toBeTruthy();

    const pane = await openComposer();

    choose(pane, new File(["note"], "notes.md"));

    const landing = await drawn(
      pane,
      `.${pill.attachment}.${pill.landing}`,
    );
    expect(landing.textContent).toContain("notes.md");
    expect(landing.querySelector("button")).toBeNull();

    land!();

    await waitFor(() =>
      expect(
        pane.querySelector(`.${pill.attachment}.${pill.landing}`),
      ).toBeNull(),
    );
  });

  /// And it stays up until the record that replaces it is on the row. The
  /// dimmed pill is what the file is drawn as until the Conversation is read
  /// again, so a row that dropped it the moment the upload landed would be the
  /// file blinking out and back in — with nothing to say which of the two pills
  /// the human is looking at.
  it("keeps the pill on the row from the upload landing to the record arriving", async () => {
    const notes: AttachmentView = {
      id: 9,
      name: "notes.md",
      bytes: 4,
      origin: "Brief",
    };

    // The read the upload asks for, held: the whole of what this is about is
    // the window between the file landing and the row being told about it.
    const waiting: Array<(record: ConversationView) => void> = [];
    let uploaded = false;
    let read: "before" | "held" | "after" = "before";

    const answering = (record: ConversationView) =>
      new Response(JSON.stringify(record), {
        headers: { "content-type": "application/json" },
      });

    const withNotes: ConversationView = {
      ...OPEN,
      attachments: [...OPEN.attachments, notes],
    };

    theWorkbench(
      whenever(`/api/ui/conversations/${OPEN.id}`, () => {
        if (!uploaded) return Promise.resolve(answering(OPEN));
        if (read === "after") return Promise.resolve(answering(withNotes));

        read = "held";
        return new Promise<Response>((settle) =>
          waiting.push((record) => settle(answering(record))),
        );
      }),
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/notes.md`,
        () => {
          uploaded = true;
          return json({ Attached: { attachment: notes } })();
        },
        "POST",
      ),
    );

    const pane = await openComposer();
    expect(names(pane)).toEqual(ATTACHED.map((one) => one.name));

    choose(pane, new File(["note"], "notes.md"));

    // The upload has landed and the read it asked for is held: the file is
    // still on the row, and still drawn as one on its way up.
    await waitFor(() => expect(read).toBe("held"));
    expect(names(pane)).toEqual([
      ...ATTACHED.map((one) => one.name),
      "notes.md",
    ]);
    expect(pane.querySelector(`.${pill.attachment}.${pill.landing}`)).toBeTruthy();

    read = "after";
    for (const settle of waiting.splice(0)) settle(withNotes);

    // And once the record is back the same file is on the row as a record,
    // with a × of its own.
    await waitFor(() =>
      expect(
        pane.querySelector(`.${pill.attachment}.${pill.landing}`),
      ).toBeNull(),
    );
    expect(names(pane)).toEqual([
      ...ATTACHED.map((one) => one.name),
      "notes.md",
    ]);
    expect(
      screen.getByRole("button", { name: "Remove notes.md" }),
    ).toBeTruthy();
  });

  /// A refused upload is said on the composer, named for the file it was
  /// about — a choice is several files, and one sentence for the lot would not
  /// say which of them the human has to do something about.
  it("says on the composer what could not be attached", async () => {
    theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/huge.bin`,
        json("TooLarge"),
        "POST",
      ),
    );

    const pane = await openComposer();

    choose(pane, new File(["x"], "huge.bin"));

    await waitFor(() =>
      expect(pane.textContent).toContain(
        `huge.bin: ${ATTACH_REFUSAL.TooLarge}`,
      ),
    );
  });

  /// A body the server would not read at all is the same refusal said the other
  /// way: the route's own limit answers a 413, and the composer has one sentence
  /// for both.
  it("reads a body the server would not even take as the same refusal", async () => {
    theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/huge.bin`,
        json({ error: "too large" }, 413),
        "POST",
      ),
    );

    const pane = await openComposer();

    choose(pane, new File(["x"], "huge.bin"));

    await waitFor(() =>
      expect(pane.textContent).toContain(
        `huge.bin: ${ATTACH_REFUSAL.TooLarge}`,
      ),
    );
  });

  it("takes one off from its own ×", async () => {
    const fetching = theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/${ATTACHED[0]!.id}/remove`,
        json("Removed"),
        "POST",
      ),
    );

    await openComposer();

    fireEvent.click(
      screen.getByRole("button", { name: `Remove ${ATTACHED[0]!.name}` }),
    );

    await waitFor(() =>
      expect(
        writes(
          fetching,
          `/api/ui/conversations/${OPEN.id}/attachments/${ATTACHED[0]!.id}/remove`,
        ),
      ).toHaveLength(1),
    );
  });

  /// And a refused removal is one line under the row rather than one inside a
  /// pill: a pill is a name on a line, and there is nowhere in one to say a
  /// sentence.
  it("says under the row what could not be removed", async () => {
    theWorkbench(
      whenever(
        `/api/ui/conversations/${OPEN.id}/attachments/${ATTACHED[0]!.id}/remove`,
        json("NotDrafting"),
        "POST",
      ),
    );

    const pane = await openComposer();

    fireEvent.click(
      screen.getByRole("button", { name: `Remove ${ATTACHED[0]!.name}` }),
    );

    await waitFor(() =>
      expect(pane.textContent).toContain(
        ATTACHMENT_REMOVAL_REFUSAL.NotDrafting,
      ),
    );
  });

  /// Once the Brief has frozen the files are settled with it, so neither the
  /// paperclip nor a × is drawn — the row is the record and not a control any
  /// more.
  ///
  /// Asked of the adopting draft, which is the one composer in the fixtures
  /// whose Brief comes down frozen, with the drafting fixture's own files put on
  /// it: a frozen Brief with nothing attached would draw no × for having nothing
  /// to draw one on, which is not the rule being asked about.
  it("draws no way of changing them once the brief has frozen", async () => {
    const frozen: ConversationView = { ...ADOPTING, attachments: ATTACHED };

    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json({ showing: false })),
      whenever("/api/ui/repos", json([])),
      whenever("/api/ui/profiles", json([])),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever(`/api/ui/conversations/${frozen.id}`, json(frozen)),
      json([]),
    );

    const pane = await openComposer(frozen);

    expect(names(pane), "the files are still drawn").toEqual(
      ATTACHED.map((attachment) => attachment.name),
    );
    expect(
      pane.querySelector(`.${pill.attachment} button`),
      "and none of them has a × on it any more",
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Attach a file" }),
      "and there is nothing to attach with",
    ).toBeNull();
  });
});

/// The other way a file is put on a draft: dropped anywhere on the box rather
/// than picked through the paperclip. The same attaching either way — what is
/// asked here is the box being the target, the highlight while a drag is over
/// it, and the folders a drop may carry.
describe("dropping files on a draft's composer", () => {
  /// The box, which is the whole of what a file is dropped onto: the text, the
  /// pills and the setup row alike.
  async function theBox(at: ConversationView = OPEN): Promise<HTMLElement> {
    const pane = await openComposer(at);
    return pane.querySelector<HTMLElement>(`.${composer.box}`)!;
  }

  /// What the server says when it takes one.
  const attached = (id: number, name: string) =>
    json({ Attached: { attachment: { id, name, bytes: 4, origin: "Brief" } } });

  /// Where one file goes up.
  const upload = (name: string) =>
    `/api/ui/conversations/${OPEN.id}/attachments/${name}`;

  it("attaches every file dropped on it", async () => {
    const fetching = theWorkbench(
      whenever(upload("notes.md"), attached(9, "notes.md"), "POST"),
      whenever(upload("shot.png"), attached(10, "shot.png"), "POST"),
    );

    const box = await theBox();

    dropOn(
      box,
      carrying({
        files: [new File(["note"], "notes.md"), new File(["png"], "shot.png")],
      }),
    );

    await waitFor(() => {
      expect(writes(fetching, upload("notes.md"))).toHaveLength(1);
      expect(writes(fetching, upload("shot.png"))).toHaveLength(1);
    });
  });

  /// Drawn while the drag is over the box and not otherwise — and through every
  /// child of it, which is why the box counts the drag in and out rather than
  /// holding a flag: `dragenter` fires again for each element crossed, and a
  /// flag would go out the moment the drag moved from the text onto a pill.
  it("highlights the box while files are dragged over it", async () => {
    theWorkbench();

    const box = await theBox();
    const carried = carrying({ files: [new File(["note"], "notes.md")] });

    expect(box.classList.contains(composer.over!)).toBe(false);

    drag(box, "dragenter", carried);
    await waitFor(() =>
      expect(box.classList.contains(composer.over!)).toBe(true),
    );

    drag(box.querySelector("textarea")!, "dragenter", carried);
    drag(box.querySelector("textarea")!, "dragleave", carried);
    expect(
      box.classList.contains(composer.over!),
      "and still over it, the drag having only crossed into a child",
    ).toBe(true);

    drag(box, "dragleave", carried);
    await waitFor(() =>
      expect(box.classList.contains(composer.over!)).toBe(false),
    );
  });

  /// And gone the moment the files are let go, rather than left on a box that
  /// is no longer under anything.
  it("takes the highlight off on the drop", async () => {
    theWorkbench(whenever(upload("notes.md"), attached(9, "notes.md"), "POST"));

    const box = await theBox();

    dropOn(box, carrying({ files: [new File(["note"], "notes.md")] }));

    await waitFor(() =>
      expect(box.classList.contains(composer.over!)).toBe(false),
    );
  });

  /// A drag carrying a selection of text or a link is not a drop this box
  /// takes: nothing is drawn, and the browser is left to do whatever it does
  /// with one.
  it("draws nothing for a drag carrying no files", async () => {
    const fetching = theWorkbench();

    const box = await theBox();
    const carried = carryingNothing();

    drag(box, "dragenter", carried);
    const over = drag(box, "dragover", carried);

    expect(box.classList.contains(composer.over!)).toBe(false);
    expect(over.defaultPrevented, "and the drop is not taken").toBe(false);
    expect(attaches(fetching)).toHaveLength(0);
  });

  /// A folder in the drop is skipped without a word: the files beside it are
  /// attached, and what could not be is not a mistake to report.
  it("attaches the files in a drop and skips the folder beside them", async () => {
    const fetching = theWorkbench(
      whenever(upload("notes.md"), attached(9, "notes.md"), "POST"),
    );

    const box = await theBox();

    dropOn(
      box,
      carrying({
        files: [new File(["note"], "notes.md")],
        folders: ["screenshots"],
      }),
    );

    await waitFor(() =>
      expect(writes(fetching, upload("notes.md"))).toHaveLength(1),
    );

    expect(
      writes(fetching, upload("screenshots")),
      "and nothing went up for the folder",
    ).toHaveLength(0);
    expect(box.textContent, "and nothing was said about it").not.toContain(
      "screenshots",
    );
  });

  /// Once the Brief has frozen the files are settled with it, so the box takes
  /// no drop either — the paperclip and the × are already gone.
  it("takes no drop once the brief has frozen", async () => {
    const frozen: ConversationView = { ...ADOPTING, attachments: ATTACHED };

    const fetching = serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json({ showing: false })),
      whenever("/api/ui/repos", json([])),
      whenever("/api/ui/profiles", json([])),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever(`/api/ui/conversations/${frozen.id}`, json(frozen)),
      json([]),
    );

    const box = await theBox(frozen);

    dropOn(box, carrying({ files: [new File(["note"], "notes.md")] }));

    expect(box.classList.contains(composer.over!)).toBe(false);
    expect(attaches(fetching)).toHaveLength(0);
  });
});

/// And where the same files are read once the work has started: the composer is
/// gone with the drafting, so the row stands on the Brief's own pane, under the
/// document and above what the Conversation was configured with.
///
/// Over the golden fixture like the composer's own tests, the grilling
/// Conversation carrying a file that was put on it before the work started.
describe("the files on a frozen brief", () => {
  /// The Brief on a record, which is what the pane is opened at.
  function briefOn(at: ConversationView): BriefEvent {
    const found = at.timeline.find(
      (event): event is { Brief: BriefEvent } => "Brief" in event,
    );

    if (!found) throw new Error("the fixture should carry a Brief");
    return found.Brief;
  }

  /// The Brief pane, opened at its own path — which is what a cold load of it
  /// is, and what saves these from walking a Timeline they are not about.
  async function openBrief(at: ConversationView): Promise<HTMLElement> {
    serving(
      whenever("/api/ui/conversations", json([])),
      whenever("/api/ui/conversations/archived", json({ showing: false })),
      whenever("/api/ui/repos", json([])),
      whenever("/api/ui/profiles", json([])),
      whenever("/api/ui/abandoned-roadmaps", json([])),
      whenever(`/api/ui/conversations/${at.id}`, json(at)),
      json([]),
    );

    const { container } = mount(
      `/conversations/${at.id}/events/${briefOn(at).id}`,
    );

    return drawn(container, `.${shell.detailsPane}`);
  }

  /// What each pill on the pane reads: the name, and after it how large the
  /// file is. The size is the one thing this row says that the composer's did
  /// not — a record is the whole account of what the sessions were handed.
  it("lists what was attached, each with its size", async () => {
    expect(FROZEN.attachments.length).toBeGreaterThan(0);

    const pane = await openBrief(FROZEN);
    const row = await drawn(pane, `.${pill.attachments}`);

    expect(names(row)).toEqual(
      FROZEN.attachments.map((attachment) => attachment.name),
    );
    expect(
      [...row.querySelectorAll(`.${pill.attachmentSize}`)].map(
        (size) => size.textContent,
      ),
    ).toEqual(FROZEN.attachments.map((attachment) => sized(attachment.bytes)));

    // Named for whoever is not looking at it, the way the composer's row is.
    expect(screen.getByRole("list", { name: "Attached files" })).toBeTruthy();
  });

  /// Nothing to press on it, on a pane that has no control anywhere: the files
  /// froze with the Brief, and the composer that could have changed either of
  /// them is gone.
  it("offers no way of changing any of it", async () => {
    const pane = await openBrief(FROZEN);
    const row = await drawn(pane, `.${pill.attachments}`);

    expect(row.querySelector("button")).toBeNull();
    expect(screen.queryByRole("button", { name: "Attach a file" })).toBeNull();
  });

  /// Under the Brief and above the Configuration: the files are part of what
  /// the work was set up with, and the half of it that came out of the Brief
  /// rather than out of the dropdowns.
  it("stands between the brief and the configuration", async () => {
    const pane = await openBrief(FROZEN);
    await drawn(pane, `.${briefPane.configuration}`);

    const where = (selector: string) =>
      [...pane.children].findIndex((part) =>
        part.matches(`${selector}, :has(${selector})`),
      );

    const document = where(`.${briefPane.brief}`);
    const row = where(`.${pill.attachments}`);

    expect(document).toBeGreaterThan(-1);
    expect(row).toBeGreaterThan(document);
    expect(where(`.${briefPane.configuration}`)).toBeGreaterThan(row);
  });

  /// And no row at all on a Conversation nobody attached anything to, which is
  /// most of them: an empty row under the Brief would read as something having
  /// gone missing.
  it("draws no row at all where nothing was attached", async () => {
    const pane = await openBrief({ ...FROZEN, attachments: [] });
    await drawn(pane, `.${briefPane.configuration}`);

    expect(pane.querySelector(`.${pill.attachments}`)).toBeNull();
    expect(screen.queryByRole("list", { name: "Attached files" })).toBeNull();
  });

  /// A size is said in whichever unit keeps it to a few digits, in the words
  /// the session's own prompt says it in — the same list the server's own
  /// `sized` is asked, because the two are one wording said in two languages.
  /// See `crates/server/src/skills.rs`.
  it("says a size in the unit that reads, the way the prompt does", () => {
    expect(sized(0)).toBe("0 bytes");
    expect(sized(1)).toBe("1 byte");
    expect(sized(999)).toBe("999 bytes");
    expect(sized(4_096)).toBe("4.1 kB");
    expect(sized(1_240_000)).toBe("1.2 MB");
    expect(sized(32 * 1024 * 1024)).toBe("33.6 MB");
    expect(sized(2_500_000_000)).toBe("2.5 GB");
  });
});
