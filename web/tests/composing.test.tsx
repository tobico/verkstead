//! The compose page: the composer standing on nothing, and what the two presses
//! under it do with what it is holding.
//!
//! Everything about it that is *drawing* is the composer's own and is asked
//! about there — the box, the row of options along its edge, the panel behind
//! the first of them. What is asked here is the half that is different: the
//! device holding a draft nobody has created, and the create that replays it
//! through the endpoints a Conversation is configured with.
//!
//! The files are that half twice over. The pill they are drawn as and the
//! paperclip they are picked through are the composer's own and are asked about
//! in `attaching.test.tsx`; that they are held in the page until a press, and
//! go up with the replay when one is made, is this page's own doing and is
//! asked about here.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  ProfileEntry,
  RepoPairingsView,
} from "../src/api/types";
import menu from "../src/Menu.module.css";
import composer from "../src/workbench/Composer.module.css";
import sidebar from "../src/workbench/Conversations.module.css";
import setup from "../src/workbench/Setup.module.css";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import { ATTACH_REFUSAL } from "../src/workbench/Composer";
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
import { offered, pick, picker, showing } from "./pickers";
import { json, serving, whenever } from "./serving";
import abandoned from "./fixtures/abandoned-roadmaps.json" with { type: "json" };

/// The roadmaps nothing is driving, as the server answers for them: three of
/// them in one repo, the last found on a branch that has not merged.
const ABANDONED = abandoned as AbandonedRepo[];

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

/// Where among everything the page put on the wire its first POST to `path`
/// came, or `-1` where it never wrote there at all.
///
/// The one reading that says what happened *before* what: a file has to be on
/// the Conversation before the work starts, the Brief freezing when it does and
/// a file arriving after that being refused for being late.
function order(fetching: ReturnType<typeof serving>, path: string): number {
  return fetching.mock.calls.findIndex(
    ([asked, init]) => String(asked) === path && init?.method === "POST",
  );
}

/// What one Repo remembers, as the endpoint writes it: a pairing per role, off
/// the fixture's own profiles so the rows a test names are rows the picker
/// really offers.
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

/// Every registered Repo remembering something, which is what a workbench that
/// has grilled anything looks like — and what the three role pickers stand on
/// when nobody has touched them.
///
/// Served under the presses rather than in the bench, because it is what makes
/// *Start work* pressable: the press carries a grilling with it, and a grilling
/// with no roles answered is one the server would refuse. A test about a Repo
/// with nothing to remember serves the bench's own `NO_PAIRINGS` instead.
const REMEMBERED = REPOS.map((repo) =>
  whenever(
    `/api/ui/repos/${repo.id}/pairings`,
    json(remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!)),
  ),
);

/// The endpoints a create walks through, all answering yes — with whatever the
/// test is about handed in last, an answer named twice being the later of the
/// two.
function creating(...answers: Parameters<typeof serving>) {
  return theWorkbench(
    ...REMEMBERED,
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

/// Wait until all three role pickers are standing on something, which is what
/// *Start work* waits on as well.
///
/// The press carries a grilling with it, so it draws inert until every role is
/// answered — and the answer they arrive with is the repo's own memory, which is
/// a read of its own. A test pressing before it lands would be pressing a button
/// that has nothing to do but say what is missing.
async function rolesAnswered(): Promise<void> {
  for (const role of ["Grilling", "Implementation", "Review"]) {
    await waitFor(() => expect(showing(role)).not.toBe("Not chosen"));
  }
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

/// Pick a repo, which is the one thing a create cannot do without.
///
/// Two controls wear that name, and which of them is standing is the whole of
/// what this page does about a repo: a listbox in the row until one is picked,
/// and the panel's own `<select>` — behind the trigger the listbox became —
/// every time after. So the first pick walks the rows and the rest change the
/// field, which is what the human does too.
async function pickRepo(container: ParentNode, id: number): Promise<void> {
  const listed = container.querySelector(`.${setup.repoSelect}`) !== null;

  if (listed) {
    // Waited for the rows to have landed, the control being drawn before the
    // list it offers has arrived.
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
    pick("Repo", REPOS.find((repo) => repo.id === id)!.name);
    return;
  }

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

  it("is the one way into a new conversation from the sidebar", async () => {
    theWorkbench();
    const { container } = mount();

    const link = await drawn<HTMLAnchorElement>(
      container,
      `.${sidebar.compose}`,
    );

    expect(link.getAttribute("href")).toBe("/compose");
    expect(link.textContent).toBe("New conversation");

    // And the button stands alone: the menu beside it — one press, one repo,
    // and a conversation created before a word was written — went when this
    // page took over the last of what it offered, so the pane drops nothing at
    // all any more.
    expect(
      screen.getByLabelText("Conversations").querySelector('[aria-haspopup="menu"]'),
    ).toBeNull();
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

  /// The Repo slot is two controls, one after the other: an ordinary dropdown
  /// while nothing is picked, and the panel that arranges what was picked from
  /// the moment something is. A panel of questions about a repository nobody has
  /// chosen is a form about nothing.
  it("offers the repos as a dropdown, and becomes the panel once one is picked", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.repoSelect}`);

    // The invitation rather than the placeholder every other picker says: this
    // one has not been answered rather than had its answer taken away.
    expect(showing("Repo")).toBe("Select");
    expect(container.querySelector(`.${setup.repoOption}`)).toBeNull();

    await pickRepo(container, REPOS[1]!.id);

    const trigger = await drawn(
      container,
      `.${setup.repoOption} .${setup.optionValue}`,
    );
    await waitFor(() => expect(trigger.textContent).toBe(REPOS[1]!.name));
    expect(container.querySelector(`.${setup.repoSelect}`)).toBeNull();
  });

  /// Which of the two is standing follows the id this device is holding, and the
  /// name on the trigger follows the repos — two different reads, so the panel
  /// can be standing before there is a name for it to say. The moment the read
  /// has not landed in looks exactly like the repo having been deregistered
  /// since, and neither is a blank line under the label.
  it("says the invitation again where the held repo has no name to draw", async () => {
    keep({ ...blank(), repo: 9999 });
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    const trigger = await drawn(
      container,
      `.${setup.repoOption} .${setup.optionValue}`,
    );
    expect(trigger.textContent).toBe("Select");

    // And still saying it once the repos have landed and none of them answers
    // to the id — the panel's own picker being where that is put right.
    await openRepo(container);
    const choice = (await waitFor(() =>
      picker("Repo"),
    )) as unknown as HTMLSelectElement;
    await waitFor(() =>
      expect(choice.options.length).toBeGreaterThanOrEqual(REPOS.length),
    );
    expect(trigger.textContent).toBe("Select");
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

  /// Neither press is ever truly `disabled` for a thing that is missing, only
  /// for a press in flight: a disabled button is one no browser will hover and
  /// no keyboard will reach, so the `title` saying what is missing would go the
  /// same way as the press it never takes. Inert, `aria-disabled`, and answering
  /// a press with nothing is what says it instead.
  it("creates nothing until a repo is picked, and says why on both presses", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);

    const start = screen.getByRole("button", { name: "Start work" });
    const draft = screen.getByRole("button", { name: "Save as draft" });

    for (const press of [start, draft]) {
      expect((press as HTMLButtonElement).disabled).toBe(false);
      expect(press.getAttribute("aria-disabled")).toBe("true");
      expect(press.classList.contains(composer.inert!)).toBe(true);

      // And why, on the press itself: one thing is missing, and it is the first
      // control in the row above.
      expect(press.getAttribute("title")).toBe("No repo selected");
    }

    // Pressed, either of them does nothing at all — which is what the press
    // being taken at all has to be worth.
    fireEvent.click(draft);
    fireEvent.click(start);
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);

    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() => expect(draft.getAttribute("aria-disabled")).toBe("false"));
    expect(draft.classList.contains(composer.inert!)).toBe(false);
    expect(draft.getAttribute("title")).toBeNull();
    expect(start.getAttribute("title")).not.toBe("No repo selected");
  });

  /// And the two presses part company there. Creating is the whole of what the
  /// quieter one does, so a repo is the whole of what it waits on; the other
  /// carries a grilling with it and waits on what one waits on — a brief, and
  /// the three roles answered.
  ///
  /// Inert rather than disabled, exactly as the composer's own start is: a
  /// disabled button is one a browser will not hover, and hovering is how what
  /// is missing gets read.
  it("draws Start inert until there is a brief and three roles, and says what is missing", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();

    // The roles are answered by the repo's own memory; the brief is not.
    const start = screen.getByRole("button", { name: "Start work" });
    expect(start.getAttribute("aria-disabled")).toBe("true");
    expect((start as HTMLButtonElement).disabled).toBe(false);
    expect(start.classList.contains(composer.inert!)).toBe(true);

    // What it is waiting on, on the press itself rather than under the box —
    // and nothing at all said on the page.
    expect(start.getAttribute("title")).toBe(
      "Starting needs a brief, and every role picked and working.",
    );
    expect(
      screen.queryByText(
        "Starting needs a brief, and every role picked and working.",
      ),
    ).toBeNull();

    // And pressed, it creates nothing at all — the whole of what was wrong with
    // a press that made the conversation and then reported the grilling
    // refused.
    fireEvent.click(start);
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);

    // And nothing about the other press: creating is all it does, and it can.
    expect(
      screen
        .getByRole("button", { name: "Save as draft" })
        .getAttribute("aria-disabled"),
    ).toBe("false");

    // A brief typed is the last of it, and the press goes live.
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });

    await waitFor(() =>
      expect(start.getAttribute("aria-disabled")).toBe("false"),
    );
    expect(start.classList.contains(composer.inert!)).toBe(false);
  });

  it("creates, replays every touched field, kicks off and lands in the draft", async () => {
    const fetching = creating();
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();

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

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();

    // The branch is inside the panel the trigger became once a repo was
    // picked, which is where the rest of *which code* is settled.
    await openRepo(container);
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
    await rolesAnswered();

    // The branch is inside the panel the trigger became once a repo was
    // picked, which is where the rest of *which code* is settled.
    await openRepo(container);
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
    creating(whenever("/api/ui/conversations", json("NoSuchRepo"), "POST"));
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();

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
      expect(showing("Grilling")).toBe("Fable 5 — fable"),
    );
    expect(showing("Implementation")).toBe("Opus 5 — opus");
    expect(showing("Review")).toBe("Sonnet 5 — sonnet");
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
      expect(showing("Grilling")).toBe("Fable 5 — fable"),
    );

    // One of the three made the human's own, which is what the switch must not
    // touch.
    pick("Grilling", "No grilling");
    expect(showing("Grilling")).toBe("No grilling");

    await pickRepo(container, REPOS[0]!.id);

    // The two nobody touched are the other repo's memory now; the one they did
    // is still theirs.
    await waitFor(() =>
      expect(showing("Review")).toBe("Fable 5 — fable"),
    );
    expect(showing("Implementation")).toBe("Sonnet 5 — sonnet");
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

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() =>
      expect(showing("Grilling")).toBe("Fable 5 — fable"),
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

/// The other way work gets into the pipeline, which is this page as well: a
/// roadmap somebody staged before Verkstead was driving anything, loaded into
/// the composer and started from it.
///
/// This is where the sidebar's New-conversation menu ended up. What was a group
/// of rows under the repos is a dropdown under the box, and what a press does is
/// the difference worth asking about: the menu created a conversation on the
/// spot, and this creates nothing until one of the two presses under the box.
describe("adopting a roadmap from the compose page", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// The workbench with roadmaps to adopt, and the two endpoints a press walks
  /// through — the adoption started, and the stage adopted.
  function adopting(...answers: Parameters<typeof serving>) {
    return theWorkbench(
      // Adopting needs all three roles answered exactly as grilling does — the
      // stages after this one inherit them — so the repos remember something
      // here too, and *Start work* is a press with an answer to give.
      ...REMEMBERED,
      whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)),
      whenever("/api/ui/adoptions", json({ Started: { id: OPEN.id } }), "POST"),
      whenever(
        `/api/ui/conversations/${OPEN.id}/adopt`,
        json("Adopted" satisfies Adopted),
        "POST",
      ),
      ...answers,
      json(null),
    );
  }

  /// The dropdown under the box, dropped — and the rows it holds.
  async function roadmapRows(
    container: ParentNode,
  ): Promise<HTMLButtonElement[]> {
    fireEvent.click(
      await drawn(container, `.${composer.adopt} > .${menu.trigger}`),
    );
    await drawn(container, `.${composer.roadmapRow}`);
    return [
      ...container.querySelectorAll<HTMLButtonElement>(
        `.${composer.roadmapRow}`,
      ),
    ];
  }

  /// Load the one at `at`, which is what pressing its row does.
  async function loadRoadmap(container: ParentNode, at: number): Promise<void> {
    fireEvent.click((await roadmapRows(container))[at]!);
    await drawn(container, `.${composer.loaded}`);
  }

  /// Every roadmap there is to adopt, flat, in the order the rows come down.
  const flat = ABANDONED.flatMap((held) =>
    held.roadmaps.map((roadmap) => ({ repo: held.repo, roadmap })),
  );

  it("names each roadmap, its repo, the next stage and where it was found", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    const rows = await roadmapRows(container);
    expect(rows.length).toBe(flat.length);

    for (const [n, held] of flat.entries()) {
      const said = rows[n]!.textContent!;
      expect(said).toContain(held.roadmap.name);
      expect(said).toContain(held.repo);
      expect(said).toContain(held.roadmap.stage);
      expect(said).toContain(held.roadmap.stage_title);
    }

    // Where the roadmap was found, said only when it is somewhere other than
    // the default branch: that branch is what the stage gets built on.
    expect(rows[2]!.textContent).toContain("tobi/steer");
    expect(rows[0]!.textContent).not.toContain("on ");
  });

  /// Nothing to adopt is nothing to offer, and a brief already being written is
  /// nothing to replace: the dropdown loads what would stand in the box, and
  /// one offering to replace a half-written brief would be offering to lose it.
  it("is drawn only with roadmaps to adopt and an empty box", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    // The bench serves no roadmaps at all, which is what a workbench whose
    // repositories are all being driven looks like.
    await composing(container);
    expect(container.querySelector(`.${composer.adopt}`)).toBeNull();
  });

  it("goes when a brief is being written, and comes back when it is cleared", async () => {
    adopting();
    const { container } = mount("/compose");
    const box = await composing(container);

    await drawn(container, `.${composer.adopt}`);

    fireEvent.input(box, { target: { value: "Make the widget" } });
    await waitFor(() =>
      expect(container.querySelector(`.${composer.adopt}`)).toBeNull(),
    );

    fireEvent.input(box, { target: { value: "" } });
    await drawn(container, `.${composer.adopt}`);
  });

  /// Loading one creates nothing: the row is written into what this device is
  /// holding, and the box locks to the stage that would be started.
  it("locks the box to the roadmap, and creates nothing", async () => {
    const fetching = adopting();
    const { container } = mount("/compose");

    await composing(container);
    await loadRoadmap(container, 0);

    const card = await drawn(container, `.${composer.loaded}`);
    expect(card.textContent).toContain(ABANDONED[0]!.roadmaps[0]!.name);
    expect(card.textContent).toContain(ABANDONED[0]!.roadmaps[0]!.stage_title);

    // No field left to write in, and nothing on the wire.
    expect(container.querySelector(`.${composer.box} textarea`)).toBeNull();
    expect(writes(fetching, "/api/ui/adoptions")).toBe(0);
  });

  /// The repo is the roadmap's own and settled, the branch and the base are the
  /// stage's, and what is left is what adopting actually asks for: the pairings
  /// and the repos alongside.
  it("fixes the repo and the base, and leaves the pairings and companions live", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    await loadRoadmap(container, 0);

    await waitFor(() =>
      expect(
        container.querySelector(`.${setup.repoOption} .${setup.optionValue}`)
          ?.textContent,
      ).toBe(ABANDONED[0]!.repo),
    );

    await openRepo(container);
    expect((screen.getByLabelText("Repo") as HTMLSelectElement).disabled).toBe(
      true,
    );
    expect(container.querySelector("#branch")).toBeNull();
    expect(screen.queryByLabelText("Base branch")).toBeNull();

    // The two that are still the human's to settle.
    expect(screen.getByLabelText("Works alongside")).toBeTruthy();
    expect(screen.getByLabelText("Grilling")).toBeTruthy();
  });

  /// And put down again, which gives the box back what was in it: a roadmap is
  /// loaded *over* the brief rather than in place of it, so nothing that was
  /// written is lost by taking one up.
  /// The box is locked to a card while a roadmap is loaded, so there is nothing
  /// being written for a file to be handed over with — and a control that has
  /// nothing to do is not drawn.
  it("offers no paperclip while a roadmap is loaded", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    expect(screen.getByRole("button", { name: "Attach a file" })).toBeTruthy();

    await loadRoadmap(container, 0);

    expect(
      screen.queryByRole("button", { name: "Attach a file" }),
    ).toBeNull();
  });

  it("clears back to the brief this device was holding", async () => {
    // A device holding both, which is what the state is shaped to hold: the
    // brief that was written, and the roadmap standing over it.
    keep({
      ...blank(),
      brief: "Make the widget",
      adopting: {
        repo_id: ABANDONED[0]!.repo_id,
        repo: ABANDONED[0]!.repo,
        roadmap: ABANDONED[0]!.roadmaps[0]!.name,
        title: ABANDONED[0]!.roadmaps[0]!.title,
        stage: ABANDONED[0]!.roadmaps[0]!.stage,
        stage_title: ABANDONED[0]!.roadmaps[0]!.stage_title,
        base: ABANDONED[0]!.roadmaps[0]!.base,
      },
    });
    adopting();
    const { container } = mount("/compose");

    // The roadmap is what the box is showing, and the brief is nowhere on the
    // page until it is put down.
    await drawn(container, `.${composer.loaded}`);
    expect(container.querySelector(`.${composer.box} textarea`)).toBeNull();

    fireEvent.click(
      screen.getByRole("button", {
        name: `Clear ${ABANDONED[0]!.roadmaps[0]!.name}`,
      }),
    );

    const box = await composing(container);
    await waitFor(() => expect(box.value).toBe("Make the widget"));
  });

  /// Held on the device like everything else on this page: a reload lands on
  /// the roadmap that was loaded rather than on a blank box.
  it("keeps the loaded roadmap on this device", async () => {
    adopting();
    const first = mount("/compose");

    await composing(first.container);
    await loadRoadmap(first.container, 1);
    first.unmount();

    const again = mount("/compose");
    const card = await drawn(again.container, `.${composer.loaded}`);
    expect(card.textContent).toContain(ABANDONED[0]!.roadmaps[1]!.name);
  });

  /// The press: the adoption started against the repo and the roadmap, every
  /// touched field put on what it made, and the stage adopted.
  it("starts the adoption, applies what was touched and adopts", async () => {
    const fetching = adopting(
      whenever(
        `/api/ui/conversations/${OPEN.id}/review-pairing`,
        json("Chosen"),
        "POST",
      ),
    );
    const { container, history } = mount("/compose");

    await composing(container);
    await loadRoadmap(container, 2);
    await rolesAnswered();

    pick("Review", "No review");

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/adoptions")).toEqual({
        repo_id: ABANDONED[0]!.repo_id,
        roadmap: ABANDONED[0]!.roadmaps[2]!.name,
        // The branch the roadmap was found on, so the conversation starts fixed
        // to it: a roadmap on an unmerged branch is only on that branch.
        base: ABANDONED[0]!.roadmaps[2]!.base,
      }),
    );
    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/review-pairing`),
      ).toEqual({ pairing: null }),
    );
    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/adopt`)).toBe(1),
    );

    // The three the roadmap answers for itself are never asked: the stage's
    // brief arrives with the adoption, the stage is worked on its own slug, and
    // the base went out with the start.
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toBe(0);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/branch`)).toBe(0);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/base`)).toBe(0);

    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );
    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeNull());
  });

  /// And the quieter press, which does everything but the last of it: the
  /// conversation is there to be looked at, and adopting is a press on its own
  /// page — which is what the menu's row did, minus the start.
  it("saves as a draft without adopting", async () => {
    const fetching = adopting();
    const { container, history } = mount("/compose");

    await composing(container);
    await loadRoadmap(container, 0);

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/adoptions")).toEqual({
        repo_id: ABANDONED[0]!.repo_id,
        roadmap: ABANDONED[0]!.roadmaps[0]!.name,
        // Off the default branch, so there is no base to fix.
        base: null,
      }),
    );
    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );

    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/adopt`)).toBe(0);
  });

  /// A refusal is carried to the conversation it is about, exactly as every
  /// other one this page's replay meets is.
  it("says on the conversation it made what refused the adoption", async () => {
    adopting(
      whenever(
        `/api/ui/conversations/${OPEN.id}/adopt`,
        json("NoGrillingProfile" satisfies Adopted),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await loadRoadmap(container, 0);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        screen.getByText(
          `The stage could not be adopted: ${ADOPT_REFUSAL.NoGrillingProfile}`,
        ),
      ).toBeTruthy(),
    );
  });
});

/// The files handed over with what is being composed: picked before there is
/// anything to attach them to, held in the page until a press, and sent through
/// the route a draft's own paperclip sends them through once there is a
/// Conversation.
describe("the files a compose page holds", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// Where one file goes up, the name in the path exactly as the composer sends
  /// it.
  const upload = (name: string) =>
    `/api/ui/conversations/${OPEN.id}/attachments/${name}`;

  /// What the server says when it takes one: the record it made, which is what
  /// the composer of the draft this page lands on draws its pill from.
  const attached = (id: number, name: string) =>
    json({ Attached: { attachment: { id, name, bytes: 4, origin: "Brief" } } });

  /// The hidden picker the paperclip reaches, and files chosen through it —
  /// which is what a browser does when somebody picks them.
  function choose(container: ParentNode, ...files: File[]): void {
    const picker = container.querySelector<HTMLInputElement>(
      'input[type="file"]',
    )!;

    Object.defineProperty(picker, "files", { configurable: true, value: files });
    fireEvent.change(picker);
  }

  /// The names on the pills the page is drawing.
  function pills(container: ParentNode): string[] {
    return [
      ...container.querySelectorAll(`.${composer.attachmentName}`),
    ].map((name) => name.textContent!.trim());
  }

  /// What went up to `path`, bodies and all — the count is not the question
  /// here, the body being the file itself rather than a JSON field.
  function bodies(
    fetching: ReturnType<typeof serving>,
    path: string,
  ): Array<RequestInit | undefined> {
    return fetching.mock.calls
      .filter(([asked, init]) => String(asked) === path && init?.method === "POST")
      .map(([, init]) => init);
  }

  it("draws a pill per chosen file, and sends nothing until a press", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    choose(container, new File(["note"], "notes.md"), new File(["x"], "shot.png"));

    await waitFor(() => expect(pills(container)).toEqual(["notes.md", "shot.png"]));

    // There is no Conversation to attach anything to, so nothing has been
    // attached: the press is still the first thing that reaches the server.
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);
    expect(bodies(fetching, upload("notes.md"))).toHaveLength(0);
  });

  /// At the near edge of the row the two presses are at the far edge of, which
  /// is where it stands on the composer beside this one as well.
  it("stands the paperclip left of Save as draft", async () => {
    creating();
    const { container } = mount("/compose");

    await composing(container);

    const row = container.querySelector(`.${composer.presses}`)!;
    const clip = screen.getByRole("button", { name: "Attach a file" });
    const draft = screen.getByRole("button", { name: "Save as draft" });

    expect(row.contains(clip)).toBe(true);
    expect(
      clip.compareDocumentPosition(draft) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// The × takes the held file out of the page. Nothing is asked of the server,
  /// there being nothing on it to take anything off.
  it("drops a held file from its own ×", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    choose(container, new File(["note"], "notes.md"), new File(["x"], "shot.png"));
    await waitFor(() => expect(pills(container)).toHaveLength(2));

    fireEvent.click(screen.getByRole("button", { name: "Remove notes.md" }));

    await waitFor(() => expect(pills(container)).toEqual(["shot.png"]));
    expect(fetching.mock.calls.filter(([, init]) => init?.method === "POST")).toHaveLength(
      0,
    );
  });

  it("sends what it holds when the draft is saved, and lands in the draft", async () => {
    const file = new File(["note"], "notes.md");
    const fetching = creating(
      whenever(upload("notes.md"), attached(9, "notes.md"), "POST"),
    );
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    choose(container, file);
    await waitFor(() => expect(pills(container)).toEqual(["notes.md"]));

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    // The file itself as the body, through the route the composer's own
    // paperclip uses — one more field of the replay.
    await waitFor(() => expect(bodies(fetching, upload("notes.md"))).toHaveLength(1));
    expect(bodies(fetching, upload("notes.md"))[0]!.body).toBe(file);

    // And the page lands in the Conversation it made, holding nothing at all.
    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );
    expect(localStorage.getItem(COMPOSING)).toBeNull();
  });

  /// Every file up before the work starts, because the Brief freezes when it
  /// does: one arriving after would be refused for having been late rather than
  /// for anything the human did.
  it("finishes the uploads before it starts the work", async () => {
    const fetching = creating(
      whenever(upload("notes.md"), attached(9, "notes.md"), "POST"),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();
    choose(container, new File(["note"], "notes.md"));
    await waitFor(() => expect(pills(container)).toEqual(["notes.md"]));

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(1),
    );
    expect(order(fetching, upload("notes.md"))).toBeGreaterThan(-1);
    expect(order(fetching, upload("notes.md"))).toBeLessThan(
      order(fetching, `/api/ui/conversations/${OPEN.id}/grill`),
    );
  });

  /// A `File` is a handle the browser gave the page rather than text a device
  /// can write down, so a reload keeps what was typed and loses what was picked
  /// — and says nothing about it, there being nothing to be done from here.
  it("keeps the brief through a reload and holds no file", async () => {
    creating();
    const first = mount("/compose");

    fireEvent.input(await composing(first.container), {
      target: { value: "Make the widget" },
    });
    choose(first.container, new File(["note"], "notes.md"));
    await waitFor(() => expect(pills(first.container)).toEqual(["notes.md"]));

    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeTruthy());
    first.unmount();

    const again = mount("/compose");
    const box = await composing(again.container);

    await waitFor(() => expect(box.value).toBe("Make the widget"));
    expect(pills(again.container)).toEqual([]);
    expect(
      again.container.querySelector(`.${composer.attachments}`),
    ).toBeNull();
  });

  /// A refused upload is one more refusal for the draft the page lands on,
  /// which is the shape a refused field already takes: the rest of the replay
  /// stands, and the kickoff is what a refusal stops.
  it("says on the new draft what could not be attached", async () => {
    const fetching = creating(
      whenever(upload("huge.bin"), json("TooLarge"), "POST"),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();
    choose(container, new File(["x"], "huge.bin"));
    await waitFor(() => expect(pills(container)).toEqual(["huge.bin"]));

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        screen.getByText(
          `huge.bin could not be attached: ${ATTACH_REFUSAL.TooLarge}`,
        ),
      ).toBeTruthy(),
    );

    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toBe(1);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
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
      adopting: null,
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
