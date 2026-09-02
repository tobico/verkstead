//! The compose page: the composer standing on nothing, and what the two presses
//! under it do with what it is holding.
//!
//! Everything about it that is *drawing* is the composer's own and is asked
//! about there — the box, the row of options along its edge, the panel behind
//! the first of them. What is asked here is the half that is different: the
//! device holding a draft nobody has created, and the create that replays it
//! through the endpoints a Conversation is configured with.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it } from "vitest";

import type { ProfileEntry, RepoPairingsView } from "../src/api/types";
import composer from "../src/workbench/Composer.module.css";
import sidebar from "../src/workbench/Conversations.module.css";
import setup from "../src/workbench/Setup.module.css";
import { BRANCH_REFUSAL } from "../src/workbench/Setup";
import {
  COMPOSING,
  blank,
  keep,
  leaveRefusals,
  stored,
  type Composed,
} from "../src/workbench/composing";
import { OPEN, PROFILES, REPOS, drawn, mount, theWorkbench } from "./bench";
import { pick, showing } from "./pickers";
import { json, serving, whenever } from "./serving";

/// What the page put on the wire when it wrote to `path`, and how often it did.
///
/// The same two readings `workbench.test.tsx` takes, written again here rather
/// than shared: what a create does is a sequence of writes, so both files ask
/// the same two questions of the same mock and neither owns it.
function sent(
  fetching: ReturnType<typeof serving>,
  path: string,
): unknown {
  const written = fetching.mock.calls.filter(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
  expect(written[0], `expected the page to have written to ${path}`).toBeTruthy();
  return JSON.parse(String(written[0]![1]?.body));
}

function writes(fetching: ReturnType<typeof serving>, path: string): number {
  return fetching.mock.calls.filter(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  ).length;
}

/// The endpoints a create walks through, all answering yes — with whatever the
/// test is about handed in last, an answer named twice being the later of the
/// two.
function creating(...answers: Parameters<typeof serving>) {
  return theWorkbench(
    whenever(
      "/api/ui/conversations",
      json({ Started: { id: OPEN.id } }),
      "POST",
    ),
    whenever(`/api/ui/conversations/${OPEN.id}/brief`, json("Saved"), "POST"),
    whenever(`/api/ui/conversations/${OPEN.id}/branch`, json("Renamed"), "POST"),
    whenever(`/api/ui/conversations/${OPEN.id}/base`, json("Recorded"), "POST"),
    whenever(`/api/ui/conversations/${OPEN.id}/grill`, json("Started"), "POST"),
    ...answers,
    // Everything else the page touches on the way — the seen mark above all,
    // which is fired and forgotten.
    json(null),
  );
}

/// The compose page, drawn and waited for.
async function composing(container: ParentNode): Promise<HTMLTextAreaElement> {
  return drawn<HTMLTextAreaElement>(container, `.${composer.box} textarea`);
}

/// Open the Repo panel, which is where the repo is picked and everything under
/// it is settled — unless it is open already, a press on the trigger being what
/// shuts it again.
async function openRepo(container: ParentNode): Promise<HTMLElement> {
  const trigger = await drawn<HTMLButtonElement>(
    container,
    `.${setup.repoOption} > button`,
  );

  if (trigger.getAttribute("aria-expanded") !== "true") {
    fireEvent.click(trigger);
  }

  return drawn(container, `.${setup.repoOption} > [role="group"]`);
}

/// Pick a repo out of it, which is the one thing a create cannot do without.
async function pickRepo(container: ParentNode, id: number): Promise<void> {
  await openRepo(container);

  const picker = (await waitFor(() =>
    screen.getByLabelText("Repo"),
  )) as HTMLSelectElement;

  // Waited for the list to have landed: a repo apiece, plus the placeholder
  // while nothing is picked — which is gone the second time this is called, a
  // placeholder being a state there is no way back to.
  await waitFor(() =>
    expect(picker.options.length).toBeGreaterThanOrEqual(REPOS.length),
  );

  fireEvent.change(picker, { target: { value: String(id) } });
}

describe("the compose page", () => {
  // Per device, so every test starts on a device holding nothing — and with
  // nothing left over from the create the test before it made.
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  it("is reached from the sidebar, with the older menu still beside it", async () => {
    theWorkbench();
    const { container } = mount();

    const link = await drawn<HTMLAnchorElement>(
      container,
      `.${sidebar.compose}`,
    );

    expect(link.getAttribute("href")).toBe("/compose");
    expect(link.textContent).toBe("New conversation");

    // Nothing is unreachable while the two of them stand together: the menu
    // still holds the roadmaps, and it retires when they move.
    expect(
      container.querySelector(`.${sidebar.newConversation} > button`),
    ).toBeTruthy();
  });

  it("draws the composer with no timeline beside it", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    // Two panes, exactly as a Conversation whose record is the one Event
    // stands: there is no record here at all, so there is no level between the
    // list and this.
    expect(screen.queryByLabelText("Timeline")).toBeNull();
    expect(screen.getByLabelText("Conversations")).toBeTruthy();
  });

  it("says Select until a repo is picked, and the repo after it", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    const trigger = await drawn(
      container,
      `.${setup.repoOption} .${setup.optionValue}`,
    );
    expect(trigger.textContent).toBe("Select");

    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() => expect(trigger.textContent).toBe(REPOS[1]!.name));
  });

  it("keeps what is composed on this device, and creates nothing until a press", async () => {
    const fetching = theWorkbench();
    const first = mount("/compose");

    fireEvent.input(await composing(first.container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(first.container, REPOS[1]!.id);

    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeTruthy());
    first.unmount();

    const again = mount("/compose");
    const box = await composing(again.container);

    await waitFor(() => expect(box.value).toBe("Make the widget"));
    await waitFor(() =>
      expect(
        again.container.querySelector(
          `.${setup.repoOption} .${setup.optionValue}`,
        )?.textContent,
      ).toBe(REPOS[1]!.name),
    );

    // And the whole of it was held here: nothing was started, and nothing was
    // written to any conversation.
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);
  });

  it("starts nothing until a repo is picked", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    const start = screen.getByRole("button", { name: "Start work" });
    const draft = screen.getByRole("button", { name: "Save as draft" });

    expect((start as HTMLButtonElement).disabled).toBe(true);
    expect((draft as HTMLButtonElement).disabled).toBe(true);

    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() => expect((start as HTMLButtonElement).disabled).toBe(false));
    expect((draft as HTMLButtonElement).disabled).toBe(false);
  });

  it("creates, replays every touched field, kicks off and lands in the draft", async () => {
    const fetching = creating();
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/conversations")).toEqual({
        repo_id: REPOS[1]!.id,
      }),
    );
    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toEqual({
        markdown: "Make the widget",
      }),
    );
    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(1),
    );

    // A field nobody touched is a field the server's own prefill is left to
    // answer, so nothing goes out about it.
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/branch`)).toBe(0);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toBe(0);

    // And the page lands in the Conversation it made.
    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );

    // What this device was holding is on the server by now, so it holds nothing.
    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeNull());
  });

  it("replays the branch the human named", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    fireEvent.input(await drawn(container, "#branch"), {
      target: { value: "widget-work" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/branch`)).toEqual({
        branch: "widget-work",
      }),
    );
  });

  it("saves as a draft without kicking anything off", async () => {
    const fetching = creating();
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toEqual({
        markdown: "Make the widget",
      }),
    );
    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );

    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
    expect(localStorage.getItem(COMPOSING)).toBeNull();
  });

  it("says on the draft it made what the server would not take", async () => {
    const fetching = creating(
      whenever(
        `/api/ui/conversations/${OPEN.id}/branch`,
        json("NotABranchName"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);

    fireEvent.input(await drawn(container, "#branch"), {
      target: { value: "not a branch name" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    // On the composer of the draft that was made, in the words that field's own
    // refusal is said in.
    await waitFor(() =>
      expect(
        screen.getByText(
          `The branch could not be named: ${BRANCH_REFUSAL.NotABranchName}`,
        ),
      ).toBeTruthy(),
    );

    // Nothing was lost: the brief the server did take is on the record, and the
    // kickoff is what a refusal stops — a setup the server would not take whole
    // is not the one the human asked to start work under.
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toBe(1);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
  });

  it("says that the repo has gone, and creates nothing", async () => {
    theWorkbench(
      whenever("/api/ui/conversations", json("NoSuchRepo"), "POST"),
      json(null),
    );
    const { container, history } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        screen.getByText(
          "That repo is not registered any more, so nothing was created.",
        ),
      ).toBeTruthy(),
    );

    // Still on the page, still holding what was composed.
    expect(history.get()).toBe("/compose");
  });

  /// What one Repo remembers, as the endpoint writes it: a pairing per role,
  /// off the fixture's own profiles so the rows a test names are rows the
  /// picker really offers.
  const remembering = (
    grilling: ProfileEntry,
    implementation: ProfileEntry,
    review: ProfileEntry,
  ): RepoPairingsView => ({
    grilling: { Under: { profile: grilling, model: grilling.models[0]! } },
    implementation: {
      profile: implementation,
      model: implementation.models[0]!,
    },
    review: { Under: { profile: review, model: review.models[0]! } },
  });

  /// The workbench with one Repo remembering something and the rest of them
  /// remembering nothing, which is what the bench already serves.
  const rememberedOn = (repoId: number, prefill: RepoPairingsView) =>
    theWorkbench(
      whenever(`/api/ui/repos/${repoId}/pairings`, json(prefill)),
      json(null),
    );

  it("fills the three roles with what the repo was last grilled with", async () => {
    rememberedOn(
      REPOS[1]!.id,
      remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!),
    );
    const { container } = mount("/compose");

    await composing(container);

    // Nothing to prefill from until a repo is picked: the memory is the repo's.
    await waitFor(() => expect(showing("Grilling")).toBe("Not chosen"));

    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() =>
      expect(showing("Grilling")).toBe("Claude Code Fable 5 — fable"),
    );
    expect(showing("Implementation")).toBe("Claude Code Opus 5 — opus");
    expect(showing("Review")).toBe("Claude Code Sonnet 5 — sonnet");
  });

  it("re-reads on a switch, and leaves a role the human touched alone", async () => {
    theWorkbench(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json(remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!)),
      ),
      whenever(
        `/api/ui/repos/${REPOS[0]!.id}/pairings`,
        json(remembering(PROFILES[2]!, PROFILES[2]!, PROFILES[0]!)),
      ),
      json(null),
    );
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() =>
      expect(showing("Grilling")).toBe("Claude Code Fable 5 — fable"),
    );

    // One of the three made the human's own, which is what the switch must not
    // touch.
    pick("Grilling", "No grilling");
    expect(showing("Grilling")).toBe("No grilling");

    await pickRepo(container, REPOS[0]!.id);

    // The two nobody touched are the other repo's memory now; the one they did
    // is still theirs.
    await waitFor(() =>
      expect(showing("Review")).toBe("Claude Code Fable 5 — fable"),
    );
    expect(showing("Implementation")).toBe("Claude Code Sonnet 5 — sonnet");
    expect(showing("Grilling")).toBe("No grilling");
  });

  it("sends nothing for a role left showing the prefill", async () => {
    const fetching = creating(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json(remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!)),
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/grilling-pairing`,
        json("Chosen"),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/implementation-pairing`,
        json("Chosen"),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/review-pairing`,
        json("Chosen"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() =>
      expect(showing("Grilling")).toBe("Claude Code Fable 5 — fable"),
    );

    // One picked away from what was shown, the other two left on it.
    pick("Review", "No review");

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/review-pairing`),
      ).toEqual({ pairing: null }),
    );

    // The server applies its own prefill to the Conversation it creates, so a
    // picker still showing that prefill has nothing to say: sending it back
    // would be this page claiming somebody chose it.
    expect(
      writes(fetching, `/api/ui/conversations/${OPEN.id}/grilling-pairing`),
    ).toBe(0);
    expect(
      writes(fetching, `/api/ui/conversations/${OPEN.id}/implementation-pairing`),
    ).toBe(0);
  });

  it("asks the same three roles the composer asks", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    // Waited for: the list of profiles is a read of its own, and the row is
    // drawn once it has arrived.
    await waitFor(() => expect(screen.getByLabelText("Grilling")).toBeTruthy());
    expect(screen.getByLabelText("Implementation")).toBeTruthy();
    expect(screen.getByLabelText("Review")).toBeTruthy();
    expect(PROFILES.length).toBeGreaterThan(0);
  });
});

/// And the state away from the page it is composed on: what survives a round
/// trip through this device, and what is discarded rather than half-applied.
describe("what a device holds between visits", () => {
  beforeEach(() => localStorage.clear());

  it("comes back as it was left", () => {
    const held: Composed = {
      repo: 2,
      brief: "Make the widget",
      branch: "widget-work",
      base: "release-1.4",
      companions: [
        { repo_id: 3, mode: "ReadWrite", base: "trunk", branch: "beside" },
      ],
      grilling: "1:opus",
      implementation: "2:fable",
      review: null,
    };

    keep(held);

    expect(stored()).toEqual(held);
  });

  it("holds nothing at all for a page nobody has touched", () => {
    keep(blank());

    expect(localStorage.getItem(COMPOSING)).toBeNull();
    expect(stored()).toEqual(blank());
  });

  /// A body under this key is whatever some older build of the app left there,
  /// so it is checked field by field and discarded whole rather than applied in
  /// part.
  it("discards a body that is not one of these, and drops it on the way past", () => {
    localStorage.setItem(COMPOSING, '{"brief":"Make the widget"}');

    expect(stored()).toEqual(blank());
    expect(localStorage.getItem(COMPOSING)).toBeNull();
  });

  it("discards a body that will not even parse", () => {
    localStorage.setItem(COMPOSING, "not json");

    expect(stored()).toEqual(blank());
  });

  /// A companion row missing one of the three things it settles takes the whole
  /// draft with it: half a row is a row nothing could be created from.
  it("discards a draft whose companion rows are the wrong shape", () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({ ...blank(), repo: 2, companions: [{ repo_id: 3 }] }),
    );

    expect(stored()).toEqual(blank());
  });
});
