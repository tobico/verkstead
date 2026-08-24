//! Closing a Set unanswered: the human declaring that nobody is ever going to
//! answer it, so it stops being something that is waiting on them.
//!
//! The one irreversible act in the whole UI, and the only one confirmed in as
//! many words. It settles the Set the way a Response does — the agent's wait
//! ends, and the Set stops waiting — except that there is no Response,
//! because there never was one.

import { fireEvent, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Archived } from "../src/api/types";
import { draftKey } from "../src/set/sheet";
import { answering, posts } from "./reading";
import { json, readable } from "./serving";
import archivedSet from "./fixtures/set-archived.json" with { type: "json" };
import waiting from "./fixtures/set-answering.json" with { type: "json" };

vi.mock("../src/set/diagrams", () => ({ drawDiagrams: () => () => {} }));

const WAITING = readable(waiting);

/// The same Set once it has been closed: what the server answers with when the
/// page reads it back where it stands.
const ARCHIVED = { ...readable(archivedSet), id: WAITING.id };
const KEY = draftKey(WAITING.id);

const archived = (outcome: Archived) => json(outcome);

/// The button reading `text`.
function press(page: ParentNode, text: string) {
  const button = [...page.querySelectorAll("button")].find(
    (found) => found.textContent === text,
  );
  expect(button, `expected a button reading "${text}"`).toBeTruthy();
  fireEvent.click(button!);
}

/// Open the standing menu — the badge is its title — and choose the one thing
/// in it, which is the offer to close the Set unanswered.
function reachForArchive(page: ParentNode) {
  const trigger = page.querySelector(".standing > .menu-trigger");
  expect(trigger, "expected the badge to open the standing menu").toBeTruthy();
  fireEvent.click(trigger!);
  press(page, "Archive unanswered");
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("the offer to close a Set unanswered", () => {
  it("folds behind the badge saying whether anyone is still listening", async () => {
    const { page } = await answering(WAITING);

    // The badge is the menu's title, and the offer is nowhere on the page
    // until the menu is asked for: archiving is almost never the right thing
    // to do to a Set.
    const trigger = page.querySelector(".standing > .menu-trigger")!;
    expect(trigger.querySelector(".liveness")!.textContent).toBe(
      "agent waiting",
    );
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(page.querySelector("button.archive")).toBeNull();

    fireEvent.click(trigger);

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(page.querySelector("button.archive")!.textContent).toBe(
      "Archive unanswered",
    );

    // And it is the one menu the rest of the UI drops, rather than a second
    // one built here — which is what says it takes Escape, takes a press away
    // from it and stands off the page the way every other menu does.
    expect(page.querySelector(".standing > .menu-drop button.archive")).toBe(
      page.querySelector("button.archive"),
    );
  });

  it("says the agent has gone when that is what the server said", async () => {
    const { page } = await answering({
      ...WAITING,
      standing: { Waiting: "disconnected" },
    });

    const badge = page.querySelector(".standing .liveness")!;
    expect(badge.className).toBe("liveness disconnected");
    expect(badge.textContent).toBe("agent disconnected");
  });

  /// A Deferred Ask has nobody on the other end and never had, so the badge
  /// says that rather than reporting an agent that has gone. The offer behind
  /// it is the same offer: the Set is still the human's to close.
  it("says nobody is waiting on a Set that was deferred", async () => {
    const { page } = await answering({
      ...WAITING,
      standing: { Waiting: "deferred" },
    });

    const badge = page.querySelector(".standing .liveness")!;
    expect(badge.className).toBe("liveness deferred");
    expect(badge.textContent).toBe("no agent waiting");

    reachForArchive(page);
    expect(page.querySelector(".confirm")).toBeTruthy();
  });

  it("is not offered on a Set that has already settled", async () => {
    const { page } = await answering({
      ...WAITING,
      standing: { ArchivedUnanswered: "2026-08-03T09:07:11.000Z" },
    });

    expect(
      page.querySelector(".standing"),
      "nothing is waiting on a settled Set, and there is nothing left to close",
    ).toBeNull();
  });

  it("archives nothing until the human has confirmed it", async () => {
    const { page, fetching } = await answering(WAITING, archived("Closed"));

    reachForArchive(page);

    const asking = page.querySelector("dialog.confirm")!;
    expect((asking as HTMLDialogElement).open, "opened as a modal").toBe(true);
    // The one irreversible act in the UI has to be asked about as one — and it
    // has to say where the Set stays, because it is not being deleted.
    expect(asking.querySelector(".note")!.textContent).toContain(
      "cannot be undone",
    );
    expect(asking.querySelector(".note")!.textContent).toContain(
      "Conversation's timeline",
    );
    expect(posts(fetching), "nothing has been sent").toHaveLength(0);

    press(page, "Keep it pending");
    expect(page.querySelector(".confirm")).toBeNull();
    expect(posts(fetching), "and still nothing was sent").toHaveLength(0);
  });
});

describe("closing a Set unanswered", () => {
  it("settles it as archived and reads it back where it stands", async () => {
    const { page, fetching, history, settles } = await answering(
      WAITING,
      archived("Closed"),
    );
    // What the page reads back once the Set is closed: it stays put, so the
    // sheet it redraws is the record of a Set nobody ever answered.
    settles(ARCHIVED);

    reachForArchive(page);
    // The dialog's own button, which is the second one reading this.
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(posts(fetching)).toHaveLength(1));
    const [path, init] = posts(fetching)[0]!;
    expect(path).toBe(`/api/ui/sets/${WAITING.id}/archive`);
    expect(init?.method).toBe("POST");

    // The Set was not discarded, it was closed — so the page stays on it and
    // says so, which is the confirmation that nothing was lost. The way out
    // leads where it always did: the Conversation this Set was asked from.
    await waitFor(() => expect(page.querySelector(".archived-at")).toBeTruthy());
    expect(history.get()).toBe(`/sets/${WAITING.id}`);
    expect(page.querySelector("a.back")!.getAttribute("href")).toBe(
      `/conversations/${WAITING.conversation}`,
    );
  });

  it("drops the draft, which this Set can never take a Response from now", async () => {
    const { page, fetching } = await answering(WAITING, archived("Closed"));

    fireEvent.input(
      page.querySelector<HTMLTextAreaElement>("#set-comment")!,
      { target: { value: "nobody is coming back for this" } },
    );
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeTruthy());

    reachForArchive(page);
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(posts(fetching)).toHaveLength(1));
    await waitFor(() => expect(localStorage.getItem(KEY)).toBeNull());
  });

  it("says why it was not archived, when it was not", async () => {
    for (const [outcome, said] of [
      ["AlreadyAnswered", "it stands as the decision that was made"],
      ["AlreadyArchived", "This Set has already been archived."],
      ["NoSuchSet", "This Set is no longer here."],
    ] as Array<[Archived, string]>) {
      const { page, fetching } = await answering(WAITING, archived(outcome));

      reachForArchive(page);
      fireEvent.click(
        page.querySelector(".confirm-actions button:last-child") as HTMLElement,
      );

      await waitFor(() => expect(posts(fetching)).toHaveLength(1));
      await waitFor(() =>
        expect(page.querySelector(".meta .error")!.textContent).toContain(
          said,
        ),
      );
    }
  });

  it("says so in the server's own wording when it did not get through", async () => {
    const { page, fetching } = await answering(
      WAITING,
      json({ error: "the Question Set could not be archived" }, 503),
    );

    reachForArchive(page);
    fireEvent.click(
      page.querySelector(".confirm-actions button:last-child") as HTMLElement,
    );

    await waitFor(() => expect(posts(fetching)).toHaveLength(1));
    await waitFor(() =>
      expect(page.querySelector(".meta .error")!.textContent).toContain(
        "the Question Set could not be archived",
      ),
    );
  });
});
