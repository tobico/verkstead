//! The compose page: the composer standing on nothing, and what the two presses
//! under it do with what it is holding.
//!
//! Everything about it that is *drawing* is the composer's own and is asked
//! about there — the box, the row of options along its edge, the panel behind
//! the first of them. What is asked here is the half that is different: the
//! device holding a draft nobody has created, and the create that replays it
//! through the endpoints a Conversation is configured with.
//!
//! The files are that half twice over. The pill they are drawn as, the paperclip
//! they are picked through and the drop the box takes are one piece the composer
//! draws too, and are asked about in `attaching.test.tsx`; that this page draws
//! that piece as well, holds what it is given until a press, and sends it up with
//! the replay when one is made, is this page's own doing and is asked about here.

import { fireEvent, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  Created,
  DirectoryListing,
  Process,
  ProfileEntry,
  RepoEntry,
  RepoPairingsView,
  RepoView,
  SettingsView,
  TakenUp,
  TargetRecorded,
} from "../src/api/types";
import menu from "../src/Menu.module.css";
import pill from "../src/Attaching.module.css";
import composer from "../src/workbench/Composer.module.css";
import sidebar from "../src/workbench/Conversations.module.css";
import setup from "../src/workbench/Setup.module.css";
import { ADOPT_REFUSAL } from "../src/workbench/Adoption";
import { ATTACH_REFUSAL } from "../src/workbench/Composer";
import { BRANCH_REFUSAL, TARGET, TARGET_REFUSAL } from "../src/workbench/Setup";
// The Processes the picker offers and the words they are said in, read rather
// than spelled out again: what the row offers is that list and nothing else.
import { OFFERED, PROCESS, ROLES } from "../src/workbench/processes";
import {
  CREATE_REFUSAL,
  REFUSAL as REPO_REFUSAL,
} from "../src/repos/RepoList";
import { repoParent, setRepoParent } from "../src/device";
import {
  COMPOSING,
  blank,
  keep,
  leaveRefusals,
  stored,
  type Composed,
} from "../src/workbench/composing";
import {
  NO_PAIRINGS,
  OPEN,
  PROFILES,
  REPOS,
  drawn,
  mount,
  openAgent,
  theWorkbench,
} from "./bench";
import { carrying, drag, dropOn } from "./dragging";
import { browse, held, listingAt } from "./fields";
import {
  actionRows,
  offered,
  opened,
  pick,
  picker,
  press,
  rows,
  showing,
} from "./pickers";
import { askedFor, hangs, json, serving, whenever } from "./serving";
import abandoned from "./fixtures/abandoned-roadmaps.json" with { type: "json" };
import listing from "./fixtures/directories.json" with { type: "json" };
import made from "./fixtures/repo.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };

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

/// Where one file goes up, the name in the path exactly as the composer sends
/// it.
const upload = (name: string) =>
  `/api/ui/conversations/${OPEN.id}/attachments/${name}`;

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
  return [...container.querySelectorAll(`.${pill.attachmentName}`)].map(
    (name) => name.textContent!.trim(),
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
  grilling: { profile: grilling, model: grilling.models[0]! },
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

/// What the Agent trigger is reading while it is shut: the words of its own
/// value line, and the ` +N` after them.
function agentReads(): string {
  return (
    document.querySelector(
      `.${setup.agentOption} > button .${setup.optionValue}`,
    )?.textContent ?? ""
  );
}

/// Wait until every role the Process uses is standing on something, which is
/// what *Start work* waits on as well.
///
/// The press carries a grilling with it, so it draws inert until every one of
/// them is answered — and the answer they arrive with is the repo's own memory,
/// which is a read of its own. A test pressing before it lands would be pressing
/// a button that has nothing to do but say what is missing.
///
/// Read off the trigger rather than off the pickers, which is exactly what the
/// trigger is for: it says *Not chosen* while any role the Process uses is
/// empty, so the panel does not have to be opened to find out.
async function rolesAnswered(): Promise<void> {
  await waitFor(() => {
    expect(
      document.querySelector(`.${setup.agentOption}`),
      "the Agent option has not been drawn yet",
    ).toBeTruthy();
    expect(agentReads()).not.toBe("Not chosen");
  });
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
/// what this page does about a repo: the dropdown in the row until one is
/// picked, and the same control behind the trigger it became every time after.
/// Both are the app's own listbox — the two rows at their foot press rather than
/// pick, which is nothing a native option can do — so the walk is the same
/// either way and only the panel has to be opened first.
async function pickRepo(container: ParentNode, id: number): Promise<void> {
  const listed = container.querySelector(`.${setup.repoSelect}`) !== null;

  if (!listed) {
    await openRepo(container);
  }

  // Waited for the rows to have landed, the control being drawn before the list
  // it offers has arrived.
  await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
  pick("Repo", REPOS.find((repo) => repo.id === id)!.name);
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
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
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
    await openAgent(container);

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
    await openAgent(container);

    await waitFor(() =>
      expect(showing("Grilling")).toBe("Fable 5 — fable"),
    );

    // One of the three made the human's own, which is what the switch must not
    // touch.
    pick("Grilling", "Claude Code Sonnet 5 — sonnet");
    expect(showing("Grilling")).toBe("Sonnet 5 — sonnet");

    await pickRepo(container, REPOS[0]!.id);

    // The two nobody touched are the other repo's memory now; the one they did
    // is still theirs.
    await waitFor(() =>
      expect(showing("Review")).toBe("Fable 5 — fable"),
    );
    expect(showing("Implementation")).toBe("Sonnet 5 — sonnet");
    expect(showing("Grilling")).toBe("Sonnet 5 — sonnet");
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
    await openAgent(container);

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
    await openAgent(container);

    // Waited for: the list of profiles is a read of its own, and the panel is
    // drawn once it has arrived.
    await waitFor(() => expect(screen.getByLabelText("Grilling")).toBeTruthy());
    expect(screen.getByLabelText("Implementation")).toBeTruthy();
    expect(screen.getByLabelText("Review")).toBeTruthy();
    expect(PROFILES.length).toBeGreaterThan(0);
  });
});

/// What kind of work the page is composing, which is the one control in the row
/// that is *not* remembered per repo.
///
/// What the control looks like and what it offers are the composer's own and are
/// asked about in `workbench.test.tsx`. What is asked here is the half a compose
/// page owns: where it stands before anybody touches it, that a pick lands on
/// the device, and what the create does — or does not do — with what is held.
describe("the process a compose page is composing under", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// Where the row reads it, which is the order the row is read in: the
  /// repository, what kind of work it is, and then who runs it.
  it("stands between the repo and the agent, on Develop", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    const row = await drawn(container, `.${setup.options}`);
    // Waited for: the profiles are a read of their own, and the Agent option at
    // the far end of the row is drawn once they have landed.
    await waitFor(() =>
      expect(row.querySelector(`.${setup.agentOption}`)).toBeTruthy(),
    );

    const at = (selector: string) =>
      [...row.children].findIndex((option) => option.matches(selector));

    expect(at(`.${setup.repoSelect}`)).toBe(0);
    expect(at(`.${setup.processChoice}`)).toBe(1);
    expect(at(`.${setup.agentOption}`)).toBe(2);
    expect(row.children).toHaveLength(3);

    // Develop for every repo, remembered nowhere: it is the one thing about a
    // conversation likeliest to differ from the last.
    expect(showing("Process")).toBe("Develop");
    await pickRepo(container, REPOS[1]!.id);
    expect(showing("Process")).toBe("Develop");
  });

  /// Four rows for now. A Process is offered only once its stage has landed, as
  /// an agent type is offered only once it can launch the real thing.
  it("offers the processes that have landed and no others", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(screen.getByLabelText("Process")).toBeTruthy());

    expect(rows("Process")).toEqual(OFFERED.map((process) => PROCESS[process]));
    expect(OFFERED).toEqual(["Develop", "Tinker", "Investigate", "Review"]);
  });

  /// The server applies its own reading to the Conversation it creates — no row
  /// at all is Develop — so a picker nobody touched has nothing to say. Sending
  /// it back would be this page claiming somebody chose it.
  it("sends nothing for a process left on Develop", async () => {
    const fetching = creating(
      whenever(
        `/api/ui/conversations/${OPEN.id}/process`,
        json("Picked"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();
    await waitFor(() => expect(showing("Process")).toBe("Develop"));

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(1),
    );
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/process`)).toBe(0);
  });

  /// And exactly one where it was touched, even where what was picked is what
  /// the control was already showing: what the human touched is the page's to
  /// say, and the record is what says it afterwards.
  it("sends one request for a process the human picked", async () => {
    const fetching = creating(
      whenever(
        `/api/ui/conversations/${OPEN.id}/process`,
        json("Picked"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();

    await waitFor(() => expect(screen.getByLabelText("Process")).toBeTruthy());
    pick("Process", PROCESS.Develop);

    // Held on the device the moment it is picked, so a reload loses nothing.
    expect(stored().process).toBe("Develop");

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/process`),
      ).toEqual({ process: "Develop" }),
    );
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/process`)).toBe(1);
  });
});

/// The Target field on this page: the same field the composer draws, held on
/// the device like everything else in the panel and replayed onto the draft a
/// press makes.
describe("the target a compose page is pointed at", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// A page on Review, against the repo it would be composed against, which is
  /// where a reload leaves one.
  function composedAsReview(over: Partial<Composed> = {}): void {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({
        ...blank(),
        repo: REPOS[1]!.id,
        process: "Review" satisfies Process,
        ...over,
      }),
    );
  }

  it("is drawn under Review and under no other process", async () => {
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);
    await openRepo(container);

    await waitFor(() => expect(screen.getByLabelText("Branch")).toBeTruthy());
    expect(screen.queryByLabelText("Target")).toBeNull();

    pick("Process", PROCESS.Review);

    const field = (await waitFor(() =>
      screen.getByLabelText("Target"),
    )) as HTMLInputElement;
    expect(field.placeholder).toBe(TARGET);
  });

  /// Filled from the box while it is empty, by the reading the server does
  /// when a Brief is saved — so a URL written into the brief shows up in the
  /// field rather than the field standing empty over what a press would find.
  it("fills itself from the brief while it is empty", async () => {
    composedAsReview();
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    const box = await composing(container);
    await openRepo(container);
    await waitFor(() => screen.getByLabelText("Target"));

    fireEvent.input(box, {
      target: {
        value: "Wrap up https://github.com/tobico/verkstead/pull/41 today.\n",
      },
    });

    await waitFor(() =>
      expect(
        (screen.getByLabelText("Target") as HTMLInputElement).value,
      ).toBe("https://github.com/tobico/verkstead/pull/41"),
    );
    expect(stored().target).toBe(
      "https://github.com/tobico/verkstead/pull/41",
    );
  });

  /// And never over what was typed into it: a branch somebody named survives a
  /// URL arriving in the box afterwards.
  it("leaves a target somebody typed exactly as it was", async () => {
    composedAsReview({ target: "rate-limiting" });
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    const box = await composing(container);
    await openRepo(container);
    await waitFor(() => screen.getByLabelText("Target"));

    fireEvent.input(box, {
      target: { value: "Like https://github.com/tobico/verkstead/pull/41.\n" },
    });

    await waitFor(() => expect(stored().brief).toContain("Like"));
    expect(
      (screen.getByLabelText("Target") as HTMLInputElement).value,
    ).toBe("rate-limiting");
  });

  /// The base picker follows what the field holds, exactly as it does on a
  /// draft's own composer.
  it("takes the base picker away where the target is a pull request", async () => {
    composedAsReview({ target: "https://github.com/tobico/verkstead/pull/41" });
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await openRepo(container);

    await waitFor(() => screen.getByLabelText("Target"));
    expect(screen.queryByLabelText("Base branch")).toBeNull();
  });

  it("keeps it where the target is a branch", async () => {
    composedAsReview({ target: "rate-limiting" });
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await openRepo(container);

    await waitFor(() => screen.getByLabelText("Target"));
    expect(screen.getByLabelText("Base branch")).toBeTruthy();
  });

  /// And the press waits on it: a Review with a brief and both roles and
  /// nothing named is inert, and says what it is waiting on.
  it("holds the press inert while nothing is named", async () => {
    composedAsReview({ brief: "Wrap the limiter up." });
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await rolesAnswered();

    const start = screen.getByRole("button", { name: "Start work" });
    await waitFor(() =>
      expect(start.getAttribute("aria-disabled")).toBe("true"),
    );
    expect(start.getAttribute("title")).toBe(
      "Starting needs a brief, a target, and both roles picked and working.",
    );
  });

  /// And the draft a press makes is holding what the page held, replayed
  /// through the endpoint the composer's own field uses.
  it("puts the target on the draft it creates", async () => {
    composedAsReview({ brief: "Wrap the limiter up.", target: "#41" });
    const fetching = creating(
      whenever(
        `/api/ui/conversations/${OPEN.id}/process`,
        json("Picked"),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/target`,
        json("Recorded"),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/take-up`,
        json("TakenUp" satisfies TakenUp),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/target`)).toEqual(
        { target: "#41" },
      ),
    );

    // And the kickoff is the take-up, the Review's press being the wrap-up
    // over what the target names.
    await waitFor(() =>
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/take-up`),
      ).toBe(1),
    );
  });

  /// And a target the server would not take is carried to that draft's own
  /// composer, the way every other refused field is.
  it("says on the draft it made what the server would not take", async () => {
    composedAsReview({ brief: "Wrap the limiter up.", target: "#41" });
    const fetching = creating(
      whenever(
        `/api/ui/conversations/${OPEN.id}/process`,
        json("Picked"),
        "POST",
      ),
      whenever(
        `/api/ui/conversations/${OPEN.id}/target`,
        json("NotDrafting" satisfies TargetRecorded),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        screen.getByText(
          `The target could not be named: ${TARGET_REFUSAL.NotDrafting}`,
        ),
      ).toBeTruthy(),
    );

    // And the kickoff is what the refusal stops, exactly as a refused branch
    // name stops it.
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/take-up`)).toBe(0);
  });
});

/// Which of the role pickers stand in the row, and what the press waits on:
/// the Process's to say, and `processes.ts`'s table to answer.
///
/// The one-role shape is exercised over an Investigate, which the picker offers
/// and this page can be moved to like any other landed Process.
describe("the pickers a compose page's process draws", () => {
  beforeEach(() => localStorage.clear());

  /// A page holding a Process and the repo it would be composed against, which
  /// is where a reload would leave one.
  function composedAs(process: Process): void {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({ ...blank(), repo: REPOS[1]!.id, process }),
    );
  }

  it("draws all three under Develop", async () => {
    composedAs("Develop");
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await openAgent(container);
    await waitFor(() => expect(screen.getByLabelText("Grilling")).toBeTruthy());
    expect(screen.getByLabelText("Implementation")).toBeTruthy();
    expect(screen.getByLabelText("Review")).toBeTruthy();
  });

  /// And two under a Tinker, which is the other Process this page can be moved
  /// to: no Grilling picker, it being a Process that is never interviewed.
  it("draws two and no grilling picker under Tinker", async () => {
    composedAs("Tinker");
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await openAgent(container);
    await waitFor(() =>
      expect(screen.getByLabelText("Implementation")).toBeTruthy(),
    );
    expect(screen.getByLabelText("Review")).toBeTruthy();
    expect(screen.queryByLabelText("Grilling")).toBeNull();

    expect(ROLES.Tinker.uses).toEqual(["implementation", "review"]);
    expect(OFFERED).toContain("Tinker");
  });

  /// And one picker is no panel at all: the control is the picker, which is
  /// what the describe below is about.
  it("draws one picker under a process that uses one role", async () => {
    composedAs("Investigate");
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(screen.getByLabelText("Agent")).toBeTruthy());

    expect(screen.queryByLabelText("Grilling")).toBeNull();
    expect(screen.queryByLabelText("Implementation")).toBeNull();
    expect(screen.queryByLabelText("Review")).toBeNull();
    expect(ROLES.Investigate.uses).toEqual(["implementation"]);
    expect(OFFERED).toContain("Investigate");
  });

  /// And the press waits on exactly those roles. Asked over a repo remembering
  /// an implementation pairing and nothing else: under Develop that is a start
  /// still waiting on two roles, and under a Process that uses one it is a
  /// start with everything it needs.
  it("waits on the roles the process uses and no others", async () => {
    const memory: RepoPairingsView = {
      grilling: null,
      implementation: {
        profile: PROFILES[1]!,
        model: PROFILES[1]!.models[0]!,
      },
      review: "Nothing",
    };

    composedAs("Investigate");
    theWorkbench(
      whenever(`/api/ui/repos/${REPOS[1]!.id}/pairings`, json(memory)),
      json(null),
    );
    const { container } = mount("/compose");

    const box = await composing(container);
    fireEvent.input(box, { target: { value: "What does the cache do?" } });

    const start = screen.getByRole("button", { name: "Start work" });
    // The one role, on the control the table gave it: a dropdown rather than a
    // trigger, so there is nothing to open to read it.
    await waitFor(() => expect(showing("Agent")).toBe("Opus 5 — opus"));

    // The one role it uses is answered, so there is nothing left to wait on —
    // and the two it does not use are not drawn to be waited on.
    await waitFor(() => expect(start.getAttribute("aria-disabled")).toBe("false"));
    expect(start.getAttribute("title")).toBeNull();
  });

  /// The same page under Develop, which waits on the two that memory left
  /// empty — and says so in the words the table counts.
  it("says what it is waiting on in the roles the process has", async () => {
    const memory: RepoPairingsView = {
      grilling: null,
      implementation: {
        profile: PROFILES[1]!,
        model: PROFILES[1]!.models[0]!,
      },
      review: "Nothing",
    };

    composedAs("Develop");
    theWorkbench(
      whenever(`/api/ui/repos/${REPOS[1]!.id}/pairings`, json(memory)),
      json(null),
    );
    const { container } = mount("/compose");

    const box = await composing(container);
    fireEvent.input(box, { target: { value: "Make the widget" } });
    await openAgent(container);

    const start = screen.getByRole("button", { name: "Start work" });
    await waitFor(() =>
      expect(showing("Implementation")).toBe("Opus 5 — opus"),
    );

    expect(start.getAttribute("aria-disabled")).toBe("true");
    expect(start.getAttribute("title")).toBe(
      "Starting needs a brief, and every role picked and working.",
    );
  });

  /// And a Process with one role says one role, which is the whole of what the
  /// table changes about these words.
  it("says one role where the process has one", async () => {
    composedAs("Investigate");
    theWorkbench(json(null));
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(screen.getByLabelText("Agent")).toBeTruthy());

    expect(
      screen.getByRole("button", { name: "Start work" }).getAttribute("title"),
    ).toBe("Starting needs a brief, and one role picked and working.");
  });
});

/// The one **Agent** control, which is the row's last option: a trigger reading
/// who the work would run under, and behind it the role pickers stacked under
/// their own names.
///
/// What the control *is* is the composer's own and is asked about in
/// `workbench.test.tsx` — both pages draw the one piece, which is the whole
/// point of it. What is asked here is the half a compose page owns: that the row
/// holds it at all, and that its reading is composed off what the pickers are
/// showing rather than off anything this device has stored.
describe("the agent control on a compose page", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// The row reads Repo, Process, Agent, and the pickers are what the last of
  /// them drops — one flat card hung off its trigger rather than a dialog over
  /// the page, exactly as the Repo option beside it is.
  it("holds the role pickers behind one Agent trigger", async () => {
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);

    const row = await drawn(container, `.${setup.options}`);
    await waitFor(() =>
      expect(row.querySelector(`.${setup.agentOption}`)).toBeTruthy(),
    );

    // Three labels along the row, and nothing of the pickers until the last of
    // them is pressed.
    expect(
      [...row.querySelectorAll(`.${setup.optionLabel}`)].map(
        (label) => label.textContent,
      ),
    ).toEqual(["Repo", "Process", "Agent"]);
    expect(screen.queryByLabelText("Grilling")).toBeNull();
    expect(screen.queryByLabelText("Implementation")).toBeNull();
    expect(screen.queryByLabelText("Review")).toBeNull();
    expect(container.querySelector("dialog")).toBeNull();

    const panel = await openAgent(container);
    await waitFor(() => expect(picker("Grilling")).toBeTruthy());

    // Stacked under their role names, in the order the table says the Process
    // uses them.
    expect(
      [...panel.querySelectorAll(`.${setup.optionLabel}`)].map(
        (label) => label.textContent,
      ),
    ).toEqual(["Grilling", "Implementation", "Review"]);
    expect(panel.getAttribute("role")).toBe("group");
  });

  /// The repo in the bench remembers three genuinely different accounts, which
  /// is the case the counting is for: the Implementation Pairing is read out and
  /// the other two are counted, the way the Repo trigger counts the repos the
  /// work runs alongside.
  it("reads the implementation pairing and counts the roles beside it", async () => {
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() => expect(agentReads()).toBe("Opus 5 — opus +2"));
  });

  /// And says it once where there is nothing else to say: a role on the same
  /// Pairing adds nothing, and neither does a role picked away — *No review* is
  /// an answer rather than another account.
  it("reads the pairing once where the other roles match or are skipped", async () => {
    theWorkbench(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json({
          ...remembering(PROFILES[1]!, PROFILES[1]!, PROFILES[1]!),
          review: "Skipped",
        } satisfies RepoPairingsView),
      ),
      json(null),
    );
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);

    await waitFor(() => expect(agentReads()).toBe("Opus 5 — opus"));
  });

  /// One for the one role that is somewhere else — and off what the pickers are
  /// showing, so a pick inside the panel is read on the trigger the moment it is
  /// made.
  it("counts one for a review picked onto a different pairing", async () => {
    theWorkbench(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json(remembering(PROFILES[1]!, PROFILES[1]!, PROFILES[1]!)),
      ),
      json(null),
    );
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);
    await waitFor(() => expect(agentReads()).toBe("Opus 5 — opus"));

    await openAgent(container);
    // The row reads out in full where the list does; the trigger drops the
    // backend's name, its mark having said it already.
    pick("Review", "Claude Code Fable 5 — fable");

    expect(agentReads()).toBe("Opus 5 — opus +1");
  });

  /// And *Not chosen* while a role the Process uses is empty, which is what the
  /// start press will refuse on. Nothing is stored for a picker nobody has
  /// touched, so the trigger stands on the repo's own memory the same way the
  /// pickers do — and says nothing at all until it lands.
  it("reads not chosen until the repo's memory has landed", async () => {
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);

    // No repo, so no memory to read: there is nothing to prefill a role from.
    await waitFor(() => expect(agentReads()).toBe("Not chosen"));
    expect(stored().implementation).toBeNull();

    await pickRepo(container, REPOS[1]!.id);
    await waitFor(() => expect(agentReads()).toBe("Opus 5 — opus +2"));
  });

  /// A role the Process does not use is not one of them, and the table is what
  /// says which: a Review uses Implementation and Review, so a repo remembering
  /// nothing for the grilling is nothing the trigger is waiting on.
  it("says nothing about a role the process does not use", async () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({
        ...blank(),
        repo: REPOS[1]!.id,
        process: "Review" satisfies Process,
      }),
    );
    theWorkbench(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json({
          grilling: null,
          implementation: {
            profile: PROFILES[1]!,
            model: PROFILES[1]!.models[0]!,
          },
          review: { Under: { profile: PROFILES[1]!, model: PROFILES[1]!.models[0]! } },
        } satisfies RepoPairingsView),
      ),
      json(null),
    );
    const { container } = mount("/compose");

    await composing(container);

    await waitFor(() => expect(agentReads()).toBe("Opus 5 — opus"));
    expect(ROLES.Review.uses).toEqual(["implementation", "review"]);
  });
});

/// The other shape of the same control, on this page: where the table says the
/// Process is run under one role, the **Agent** is the flat Pairing dropdown
/// itself, standing in the row with no trigger over it and no panel behind it.
///
/// What the shape *is* is the composer's own and is asked about in
/// `workbench.test.tsx`. What is asked here is the half a compose page owns: that
/// the one picker stands on the repo's memory the way the pickers in the panel
/// do, and that a pick through it is replayed onto the Implementation role of
/// the Conversation a press creates.
///
/// Asked over an Investigate, which is the one landed Process run under a single
/// role: the device is holding one, the picker draws a chosen
/// Process it cannot offer, and the shape is read off the table rather than off
/// what has landed.
describe("the agent dropdown on a compose page", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// A page holding a one-role Process and the repo it would be composed
  /// against, which is where a reload would leave one.
  function investigating(): void {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({
        ...blank(),
        repo: REPOS[1]!.id,
        process: "Investigate" satisfies Process,
      }),
    );
  }

  it("stands in the row as the picker itself, with no panel anywhere", async () => {
    investigating();
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    const row = await drawn(container, `.${setup.options}`);
    await waitFor(() => expect(picker("Agent")).toBeTruthy());

    // The row still reads Repo, Process, Agent — the last of the three drawn as
    // a listbox rather than as a panel's trigger.
    expect(
      [...row.querySelectorAll(`.${setup.optionLabel}`)].map(
        (label) => label.textContent,
      ),
    ).toEqual(["Repo", "Process", "Agent"]);
    expect(container.querySelector(`.${setup.agentOption}`)).toBeNull();
    expect(row.children).toHaveLength(3);

    expect(ROLES.Investigate.control).toBe("dropdown");
  });

  /// The Implementation role, whichever shape asks for it: the same picker on
  /// the same prefill, which is the repo's own memory until somebody touches
  /// it. The id says so as plainly as anything can — it is the implementation
  /// picker with the row's own label over it.
  it("stands on the repo's memory, as the picker in the panel does", async () => {
    investigating();
    theWorkbench(
      whenever(
        `/api/ui/repos/${REPOS[1]!.id}/pairings`,
        json(remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!)),
      ),
      json(null),
    );
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(picker("Agent")).toBeTruthy());

    expect(picker("Agent").id).toBe("implementation-pairing");
    // The implementation memory and not another role's, which is what the two
    // other accounts the repo remembers are there to tell it from.
    await waitFor(() => expect(showing("Agent")).toBe("Opus 5 — opus"));
    expect(stored().implementation, "nobody touched it").toBeNull();
  });

  /// And a pick through it is the Implementation role's, replayed onto the
  /// Conversation the press creates and onto no other role.
  it("replays a pick onto the implementation role alone", async () => {
    investigating();
    const fetching = creating(
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
      target: { value: "What does the cache do?" },
    });
    await waitFor(() => expect(showing("Agent")).toBe("Opus 5 — opus"));

    // The row reads out in full where the list does; the closed control drops
    // the backend's name, its mark having said it already.
    pick("Agent", "Claude Code Sonnet 5 — sonnet");
    expect(showing("Agent")).toBe("Sonnet 5 — sonnet");

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        sent(fetching, `/api/ui/conversations/${OPEN.id}/implementation-pairing`),
      ).toEqual({
        profile_id: PROFILES[2]!.id,
        model: PROFILES[2]!.models[0]!,
      }),
    );

    // The two roles the Process does not use were never drawn, so there was
    // nothing to touch and nothing to replay.
    for (const role of ["grilling", "review"]) {
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/${role}-pairing`),
      ).toBe(0);
    }
  });
});

/// The two rows at the foot of the Repo dropdown, and the one this stage wires:
/// **Open repo**, which registers a repository that already exists and lands the
/// draft on it.
///
/// The rows are the one control's, so the panel behind a picked repo has them
/// too — that half is `workbench.test.tsx`'s, over a draft where a pick is a
/// move. What is asked here is the half a compose page owns: the pick lands in
/// the state this device is holding, and a refusal is answered inside the modal.
describe("registering a repo from the Repo dropdown", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// A repository nothing has registered yet — not one of the fixture's, so
  /// landing on it is unmistakably the answer's doing rather than the list's.
  const OPENED: RepoEntry = {
    id: 4242,
    name: "widgets",
    path: "/srv/repos/widgets",
    default_branch: "main",
  };

  /// The workbench with the registration answered however the test says.
  const registering = (answer: () => Promise<Response>) =>
    theWorkbench(whenever("/api/ui/repos", answer, "POST"));

  /// Open the modal off the dropdown's foot, and hand back the field inside it.
  async function openRepoModal(): Promise<HTMLInputElement> {
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
    press("Repo", "Open repo");

    return (await waitFor(() =>
      screen.getByLabelText(/absolute path/i),
    )) as HTMLInputElement;
  }

  /// Type a path into it and send it.
  function register(field: HTMLInputElement, path: string): void {
    fireEvent.input(field, { target: { value: path } });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
  }

  it("draws both rows behind a rule, and neither is a repo to pick", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));

    expect(actionRows("Repo")).toEqual(["Create repo", "Open repo"]);
    // Every repo and nothing else: what the control offers is the rows above
    // the rule, and the two under it are not repositories.
    expect(rows("Repo")).toEqual(REPOS.map((repo) => repo.name));
    expect(opened("Repo").querySelector('[role="separator"]')).toBeTruthy();
  });

  it("puts the draft on the repo it registered", async () => {
    const fetching = registering(json({ Added: OPENED }));
    const { container } = mount("/compose");

    await composing(container);
    register(await openRepoModal(), OPENED.path);

    // The path went out as the settings pane's own registration does.
    await waitFor(() =>
      expect(fetching).toHaveBeenCalledWith(
        "/api/ui/repos",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ path: OPENED.path }),
        }),
      ),
    );

    // And the draft is on it: the id the answer carried is what this device is
    // holding now, which is the same thing picking a registered one writes.
    await waitFor(() =>
      expect(stored().repo).toBe(OPENED.id),
    );
    // The modal is spent, and the row it was opened from has become the panel.
    await waitFor(() =>
      expect(screen.queryByLabelText(/absolute path/i)).toBeNull(),
    );
  });

  /// A path already registered is not a dead end here: this row is a way *onto*
  /// a repository, and the outcome carries the one it found.
  it("lands on the repo a path already registered names", async () => {
    registering(json({ AlreadyRegistered: REPOS[1]! }));
    const { container } = mount("/compose");

    await composing(container);
    register(await openRepoModal(), REPOS[1]!.path);

    await waitFor(() => expect(stored().repo).toBe(REPOS[1]!.id));
  });

  /// And a refusal keeps the modal up with the reason under the field, because a
  /// refusal is answered by correcting the path — the same rule the settings
  /// pane draws by, in the same words.
  it("says why a path was refused, and stays up to say it", async () => {
    registering(json("NotARepository"));
    const { container } = mount("/compose");

    await composing(container);
    const field = await openRepoModal();
    register(field, "/srv/repos/notes");

    await waitFor(() => screen.getByText(REPO_REFUSAL.NotARepository));
    expect(screen.getByLabelText(/absolute path/i)).toBeTruthy();
    expect(field.value).toBe("/srv/repos/notes");
    expect(stored().repo).toBeNull();
  });
});

/// The other row at that foot: **Create repo**, which *makes* a repository
/// rather than taking on one that is already there.
///
/// The pick lands the same way the registration's does — that half is asked
/// above — so what is asked here is what a create has of its own: the two fields
/// going out as two, the parent this device remembers, the tick that puts the
/// same repository on GitHub, and a refusal that keeps the card up.
describe("making a repo from the Repo dropdown", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(0, []);
  });

  /// The repository the create made, as the server answers for it: not one of
  /// the fixture's, so a draft landing on it is unmistakably the answer's doing.
  /// Under the directory the browse fixture lists, so the parent it comes home
  /// with is one a browse could have reached.
  const MADE: RepoView = {
    ...(made as RepoView),
    id: 4343,
    name: "widgets",
    path: "/home/ada/src/widgets",
  };

  /// The one directory there is to browse, which is the fixture's own.
  const LISTING = listing as DirectoryListing;

  /// What Verkstead has been told, which this card asks exactly one thing of:
  /// whether a GitHub token is saved. The fixture's has one.
  const TOKENED = told as SettingsView;

  /// And the same with none, which is what a Verkstead nobody has told anything
  /// looks like — and what every test here that is not about the tick reads.
  const UNTOKENED: SettingsView = { ...TOKENED, github_token: null };

  /// The workbench with the create answered however the test says, that
  /// directory served both by name and as the server's home, and the settings
  /// saying whether there is a token.
  ///
  /// The settings as an answer rather than as a view, because one of the states
  /// this card has is the read not having landed — see the test that holds it
  /// with [`hangs`].
  const creating = (
    answer: () => Promise<Response>,
    settings: () => Promise<Response> = json(UNTOKENED),
  ) =>
    theWorkbench(
      whenever("/api/ui/repos/new", answer, "POST"),
      whenever("/api/ui/settings", settings),
      // What the repo it made was last grilled with, which the page asks for the
      // moment the draft lands on it — the fixture's repos are served this
      // already, and the one this create makes is not one of them.
      whenever(`/api/ui/repos/${MADE.id}/pairings`, json(NO_PAIRINGS)),
      whenever(listingAt("/home/ada/src"), json(LISTING)),
      whenever(listingAt(null), json(LISTING)),
      json(null),
    );

  /// Open the modal off the dropdown's foot, and wait for it to have settled
  /// what it is going to say about GitHub.
  ///
  /// The card asks the settings one thing — whether a token is saved — and says
  /// neither the tick nor the note until they answer, taking no create in the
  /// meantime. So a test that is about to fill it in waits for that answer, the
  /// way the human filling it in does.
  ///
  /// `settled` off for the one test that is about the wait itself.
  async function createRepoModal(settled = true): Promise<void> {
    await waitFor(() => expect(offered("Repo").length).toBe(REPOS.length));
    press("Repo", "Create repo");

    await waitFor(() => expect(screen.getByLabelText(WHERE)).toBeTruthy());

    if (settled) {
      await waitFor(() => expect(tick() ?? noRemote()).toBeTruthy());
    }
  }

  /// What the two fields are labelled, which is how they are found.
  const WHERE = "Where it goes";
  const CALLED = "What it is called";

  /// And the tick beside them, where a token is saved for it to be drawn by.
  const ON_GITHUB = "Create it on GitHub too, privately";

  /// The tick as the card is drawing it, or `null` where it is not drawn at all.
  function tick(): HTMLInputElement | null {
    return screen.queryByLabelText(ON_GITHUB) as HTMLInputElement | null;
  }

  /// And what stands where it would have on a Verkstead with no token saved.
  function noRemote(): HTMLElement | null {
    return screen.queryByText(/needs a remote/i);
  }

  /// Fill them in and send them.
  function make(parent: string, name: string): void {
    fireEvent.input(screen.getByLabelText(WHERE), {
      target: { value: parent },
    });
    fireEvent.input(screen.getByLabelText(CALLED), {
      target: { value: name },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
  }

  it("puts the draft on the repo it made", async () => {
    const fetching = creating(json({ Made: MADE }));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();
    make("/home/ada/src", "widgets");

    // The two halves went out as two: joining them into a path here would be
    // the browser building one out of a separator the server never agreed to.
    await waitFor(() =>
      expect(sent(fetching, "/api/ui/repos/new")).toEqual({
        parent: "/home/ada/src",
        name: "widgets",
        // Nothing is asked of GitHub on a Verkstead with no token saved: there
        // is nothing to make the repository as.
        github: false,
      }),
    );

    // And the draft is on what it made, which is what picking a registered one
    // writes.
    await waitFor(() => expect(stored().repo).toBe(MADE.id));
    // The modal is spent, and the row it was opened from has become the panel.
    await waitFor(() => expect(screen.queryByLabelText(WHERE)).toBeNull());
  });

  /// Where this device keeps its code is a fact about the machine in front of
  /// you, so the parent is remembered here and the browse opens *inside* it —
  /// among what is in it rather than among its siblings, which is what the same
  /// text typed by hand would mean.
  it("opens the browse in the parent the last repo on this device went in", async () => {
    setRepoParent("/home/ada/src");
    const fetching = creating(json({ Made: MADE }));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();

    expect(held(WHERE)).toBe("/home/ada/src");
    // The memory alone says nothing to the server: what is read is the browse
    // the human asked for.
    expect(askedFor(fetching, listingAt("/home/ada/src"))).toBe(0);

    browse(WHERE);
    await waitFor(() =>
      expect(askedFor(fetching, listingAt("/home/ada/src"))).toBe(1),
    );
    expect(askedFor(fetching, listingAt("/home/ada"))).toBe(0);
  });

  /// And where there is none — which a first run always is — the field stands
  /// empty and the browse opens at the server's own home, which is where an
  /// unbounded browse already opens.
  it("opens at the server's home where this device has made none", async () => {
    const fetching = creating(json({ Made: MADE }));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();

    expect(held(WHERE)).toBe("");
    browse(WHERE);

    await waitFor(() => expect(askedFor(fetching, listingAt(null))).toBe(1));
  });

  /// What it comes home with is the parent the server resolved rather than the
  /// text that was typed: that is the directory the repository is actually in,
  /// and so the one the next create should open in.
  it("remembers where it put one, for the next one", async () => {
    creating(json({ Made: MADE }));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();
    make("/home/ada/src/", "widgets");

    await waitFor(() => expect(stored().repo).toBe(MADE.id));
    expect(repoParent()).toBe("/home/ada/src");
  });

  /// A refusal keeps the modal up with the reason under the fields, for the
  /// registration's reason: what answers one is correcting what was typed, and a
  /// modal that closed on one would take the correction away with it.
  it("says why a create was refused, and stays up to say it", async () => {
    creating(json("AlreadyThere" satisfies Created));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();
    make("/home/ada/src", "widgets");

    await waitFor(() => screen.getByText(CREATE_REFUSAL.AlreadyThere));
    expect(held(WHERE)).toBe("/home/ada/src");
    expect(
      (screen.getByLabelText(CALLED) as HTMLInputElement).value,
    ).toBe("widgets");
    expect(stored().repo).toBeNull();
    expect(repoParent()).toBe("");
  });

  /// And the one refusal that is not a word this app has: what git would not do,
  /// in git's own words, because nothing here could put it better.
  it("says what git would not do, in git's words", async () => {
    creating(json({ Refused: "git init: permission denied" } satisfies Created));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();
    make("/home/ada/src", "widgets");

    await waitFor(() => screen.getByText("git init: permission denied"));
    expect(stored().repo).toBeNull();
  });

  /// Where a token is saved the tick is drawn and starts on: somebody who has
  /// saved one has said what they mean to do with it, and the pipeline this
  /// repository is about to go through ends in a push.
  it("asks for it on GitHub too where a token is saved", async () => {
    const fetching = creating(json({ Made: MADE }), json(TOKENED));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();

    await waitFor(() => expect(tick()?.checked).toBe(true));
    make("/home/ada/src", "widgets");

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/repos/new")).toEqual({
        parent: "/home/ada/src",
        name: "widgets",
        github: true,
      }),
    );
    await waitFor(() => expect(stored().repo).toBe(MADE.id));
  });

  /// And taking it off is a repository made here only, which is a thing somebody
  /// may well mean: the tick is on by default rather than compulsory.
  it("leaves GitHub alone where the tick is taken off", async () => {
    const fetching = creating(json({ Made: MADE }), json(TOKENED));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();

    await waitFor(() => expect(tick()).toBeTruthy());
    fireEvent.click(tick()!);
    make("/home/ada/src", "widgets");

    await waitFor(() =>
      expect(sent(fetching, "/api/ui/repos/new")).toMatchObject({
        github: false,
      }),
    );
  });

  /// With no token there is nothing to draw a tick for, and a sentence stands
  /// where it would have: the local repository is still worth making, and the
  /// token can be saved afterwards — but the work on it cannot be finished
  /// without a remote, which is worth knowing now rather than halfway through
  /// the first conversation.
  it("says a remote is needed where no token is saved", async () => {
    creating(json({ Made: MADE }));
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();

    await waitFor(() => screen.getByText(/needs a remote/i));
    expect(tick()).toBeNull();
  });

  /// And says neither of those things until it has been told which it is.
  ///
  /// Nothing else on this page reads the settings, so the read this card starts
  /// is always in flight when it opens. A card that took *not answered yet* for
  /// *no token* would put the sentence above in front of everybody who has one,
  /// every time, and replace it with the tick a moment later.
  it("says nothing about GitHub until the settings have answered", async () => {
    const fetching = creating(json({ Made: MADE }), hangs());
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal(false);

    // The fields are there, so this is the card drawn rather than the card
    // still coming.
    expect(screen.getByLabelText(CALLED)).toBeTruthy();
    expect(tick()).toBeNull();
    expect(screen.queryByText(/needs a remote/i)).toBeNull();

    // And it takes no create either: one sent now would ask nothing of GitHub
    // without the card ever having said so, which is the sentence going missing
    // rather than being wrong.
    fireEvent.input(screen.getByLabelText(WHERE), {
      target: { value: "/home/ada/src" },
    });
    fireEvent.input(screen.getByLabelText(CALLED), {
      target: { value: "widgets" },
    });

    const press = screen.getByRole("button", {
      name: "Create",
    }) as HTMLButtonElement;
    expect(press.disabled).toBe(true);

    fireEvent.click(press);
    expect(askedFor(fetching, "/api/ui/repos/new")).toBe(0);
  });

  /// A GitHub failure after the local repository exists is not a failed create:
  /// the directory, the commit and the registration all stand. So the card stops
  /// being a form and says what failed, and the Repo goes onto the draft on the
  /// way out — landing one takes this card away with the dropdown it was opened
  /// from, so a card that landed it at once would vanish with the reason unread.
  it("says what GitHub would not do, and lands the repo it made", async () => {
    creating(
      json({
        MadeWithoutRemote: {
          repo: MADE,
          why: "`gh` said: Name already exists on this account",
        },
      } satisfies Created),
      json(TOKENED),
    );
    const { container } = mount("/compose");

    await composing(container);
    await createRepoModal();
    await waitFor(() => expect(tick()).toBeTruthy());
    make("/home/ada/src", "widgets");

    // `gh`'s own words, the way git's are for a git that would not commit — and
    // the fields are gone, there being nothing left to fill in.
    await waitFor(() =>
      screen.getByText("`gh` said: Name already exists on this account"),
    );
    expect(screen.queryByLabelText(WHERE)).toBeNull();
    expect(stored().repo).toBeNull();

    // And the one press out lands it: the repository is registered whatever
    // GitHub said, so the draft goes on it the way a clean create's does.
    fireEvent.click(screen.getByRole("button", { name: "Done" }));

    await waitFor(() => expect(stored().repo).toBe(MADE.id));
    await waitFor(() =>
      expect(screen.queryByText(/Name already exists/)).toBeNull(),
    );
  });
});

/// The other way work gets into the pipeline, which is this page as well: a
/// roadmap somebody staged before Verkstead was driving anything, loaded into
/// the composer and started from it.
///
/// This is where the sidebar's New-conversation menu ended up. What was a group
/// of rows under the repos is *the* level of the *Other actions* menu under the
/// box — the only one, since a Review takes a pull request up now — and what a
/// press does is the difference worth asking about: the menu created a
/// conversation on the spot, and this creates nothing until one of the two
/// presses under the box.
describe("continuing a roadmap from the compose page", () => {
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

  /// The menu under the box, dropped.
  async function otherActions(container: ParentNode): Promise<HTMLElement> {
    fireEvent.click(
      await drawn(container, `.${composer.actions} > .${menu.trigger}`),
    );
    return screen.getByRole("menuitem", { name: CONTINUE });
  }

  /// And the level that holds the roadmaps, opened — and the rows it holds.
  ///
  /// Waited on rather than pressed straight away: the menu is drawn before the
  /// roadmaps have been read, and the level ungreys as they land. Which is what
  /// greying it is for — the control is there from the first paint, and what it
  /// offers settles under the hand rather than the control arriving.
  async function roadmapRows(
    container: ParentNode,
  ): Promise<HTMLButtonElement[]> {
    await otherActions(container);
    fireEvent.click(
      await waitFor(() => {
        const level = container.querySelector<HTMLButtonElement>(
          `.${menu.nested}`,
        );
        if (!level || level.disabled) {
          throw new Error(`${CONTINUE} is still greyed`);
        }
        return level;
      }),
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

  /// What the level reads as, and what the way back out of it says.
  const CONTINUE = "Continue a roadmap";

  /// Every roadmap there is to continue, flat, in the order the rows come down.
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

  /// Nothing to continue is a level greyed rather than a menu gone: what there
  /// is to do here is not a list the human can see, so a control that came and
  /// went with one would change shape between one visit and the next.
  it("draws the menu with nothing to continue, and greys the level", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    // The bench serves no roadmaps at all, which is what a workbench whose
    // repositories are all being driven looks like.
    await composing(container);
    const level = await otherActions(container);

    expect((level as HTMLButtonElement).disabled).toBe(true);

    // And it opens nothing: the card is still on its first level.
    fireEvent.click(level);
    expect(container.querySelector(`.${composer.roadmapRow}`)).toBeNull();
    expect(screen.getByRole("menuitem", { name: CONTINUE })).toBeTruthy();
  });

  /// A brief already being written is nothing to replace: a row loads what would
  /// stand in the box, and a menu offering to replace a half-written brief would
  /// be offering to lose it.
  it("goes when a brief is being written, and comes back when it is cleared", async () => {
    adopting();
    const { container } = mount("/compose");
    const box = await composing(container);

    await drawn(container, `.${composer.actions}`);

    fireEvent.input(box, { target: { value: "Make the widget" } });
    await waitFor(() =>
      expect(container.querySelector(`.${composer.actions}`)).toBeNull(),
    );

    fireEvent.input(box, { target: { value: "" } });
    await drawn(container, `.${composer.actions}`);
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

    // No field left to write in, no menu over a box that is holding something,
    // and nothing on the wire.
    expect(container.querySelector(`.${composer.box} textarea`)).toBeNull();
    expect(container.querySelector(`.${composer.actions}`)).toBeNull();
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

    // The two that are still the human's to settle — the roles behind the
    // Agent trigger, which is where every one of them stands.
    expect(screen.getByLabelText("Works alongside")).toBeTruthy();
    await openAgent(container);
    expect(screen.getByLabelText("Grilling")).toBeTruthy();
  });

  /// And put down again, which gives the box back what was in it: a roadmap is
  /// loaded *over* the brief rather than in place of it, so nothing that was
  /// written is lost by taking one up.
  /// The box is locked to a card while a roadmap is loaded, so there is nothing
  /// being written for a file to be handed over with — and a control that has
  /// nothing to do is not drawn. The drop goes with it: the paperclip and the
  /// box are two ways into the one piece, and neither is offered here.
  it("offers no paperclip and takes no drop while a roadmap is loaded", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    expect(screen.getByRole("button", { name: "Attach a file" })).toBeTruthy();

    await loadRoadmap(container, 0);

    expect(
      screen.queryByRole("button", { name: "Attach a file" }),
    ).toBeNull();

    const box = container.querySelector(`.${composer.box}`)!;
    dropOn(box, carrying({ files: [new File(["note"], "notes.md")] }));

    expect(box.classList.contains(composer.over!)).toBe(false);
    expect(container.querySelector(`.${pill.attachments}`)).toBeNull();
  });

  /// And a file picked *before* the roadmap was loaded goes with the box it was
  /// picked for: the row is not drawn over a locked box either. Held rather
  /// than dropped, the way everything else a roadmap stands over is held —
  /// clearing the card gives the file back with the brief.
  it("puts a file picked beforehand away with the box, and gives it back when the roadmap is cleared", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    choose(container, new File(["note"], "notes.md"));
    await waitFor(() => expect(pills(container)).toEqual(["notes.md"]));

    await loadRoadmap(container, 0);
    expect(pills(container), "the row goes with the box").toEqual([]);

    fireEvent.click(
      screen.getByRole("button", {
        name: `Clear ${ABANDONED[0]!.roadmaps[0]!.name}`,
      }),
    );

    await composing(container);
    await waitFor(() =>
      expect(pills(container), "and comes back with it").toEqual(["notes.md"]),
    );
  });

  /// And nothing it is holding goes up with an adoption. The paperclip is not
  /// offered over a loaded roadmap, so a file that went up with one would be a
  /// file the page never offered to take — on a Conversation whose Brief
  /// arrives frozen, with no × left to take it off again.
  it("sends no held file with the adoption", async () => {
    const fetching = adopting(
      whenever(
        `/api/ui/conversations/${OPEN.id}/review-pairing`,
        json("Chosen"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    choose(container, new File(["note"], "notes.md"));
    await waitFor(() => expect(pills(container)).toEqual(["notes.md"]));

    await loadRoadmap(container, 2);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/adopt`)).toBe(1),
    );
    expect(writes(fetching, upload("notes.md"))).toBe(0);
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

    await openAgent(container);
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
          `The stage could not be started: ${ADOPT_REFUSAL.NoGrillingProfile}`,
        ),
      ).toBeTruthy(),
    );
  });

  /// And it is the only level. *Wrap up a pull request* stood beside it until
  /// the Review Process took that path over, and the menu is still a menu: one
  /// level, greyed rather than gone while there is nothing to continue, because
  /// what it replaced was a dropdown that came and went with a list.
  it("offers continuing a roadmap and nothing else", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    await otherActions(container);

    expect(screen.getAllByRole("menuitem").length).toBe(1);
    expect(screen.getByRole("menuitem", { name: CONTINUE })).toBeTruthy();
  });

  /// And nothing asks GitHub what is open. The level that did is gone, and with
  /// it the one reading on this page that waited on somebody else's server: the
  /// page opens on what Verkstead itself knows.
  it("asks for no open pull requests when the page opens", async () => {
    const fetching = adopting();
    const { container } = mount("/compose");

    await composing(container);
    await otherActions(container);

    expect(
      fetching.mock.calls.filter(([asked]) =>
        String(asked).includes("pull-request"),
      ),
    ).toEqual([]);
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

  /// What the server says when it takes one: the record it made, which is what
  /// the composer of the draft this page lands on draws its pill from.
  const attached = (id: number, name: string) =>
    json({ Attached: { attachment: { id, name, bytes: 4, origin: "Brief" } } });

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
      again.container.querySelector(`.${pill.attachments}`),
    ).toBeNull();
  });

  /// A refused upload is one more refusal for the draft the page lands on,
  /// which is the shape a refused field already takes: the rest of the replay
  /// stands, and the kickoff is what a refusal stops.
  /// The other way one is put on: dropped anywhere on the box rather than picked
  /// through the paperclip, which is the same piece doing the same thing — held
  /// in the page either way, there being nothing on the server to send them to.
  it("holds every file dropped on the box", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    const box = container.querySelector(`.${composer.box}`)!;

    dropOn(
      box,
      carrying({
        files: [new File(["note"], "notes.md"), new File(["x"], "shot.png")],
      }),
    );

    await waitFor(() =>
      expect(pills(container)).toEqual(["notes.md", "shot.png"]),
    );

    // Still nothing on the wire: a drop is a choice, and the press is what
    // sends what has been chosen.
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);
  });

  /// Highlighted while the drag is over it and not otherwise, exactly as the
  /// composer's own box is — and a folder in the drop is skipped without a
  /// word.
  it("highlights the box, and skips a folder dropped with the files", async () => {
    creating();
    const { container } = mount("/compose");

    await composing(container);
    const box = container.querySelector(`.${composer.box}`)!;
    const carried = carrying({
      files: [new File(["note"], "notes.md")],
      folders: ["screenshots"],
    });

    drag(box, "dragenter", carried);
    await waitFor(() =>
      expect(box.classList.contains(composer.over!)).toBe(true),
    );

    drag(box, "dragover", carried);
    drag(box, "drop", carried);

    await waitFor(() => expect(pills(container)).toEqual(["notes.md"]));
    expect(box.classList.contains(composer.over!)).toBe(false);
  });

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
      target: "",
      base: "release-1.4",
      companions: [
        { repo_id: 3, mode: "ReadWrite", base: "trunk", branch: "beside" },
      ],
      process: "Develop",
      grilling: "1:opus",
      implementation: "2:fable",
      review: null,
      adopting: null,
    };

    keep(held);

    expect(stored()).toEqual(held);
  });

  /// A body from a build before the Process was asked for has no `process` at
  /// all, which is a picker nobody touched rather than a fault — the one
  /// absence a field-by-field check has to read rather than discard.
  it("reads a body from before the process as one nobody picked", () => {
    const { process: _, ...before } = { ...blank(), repo: 2 };
    localStorage.setItem(COMPOSING, JSON.stringify(before));

    expect(stored()).toEqual({ ...blank(), repo: 2 });
  });

  /// And a word this build has never heard of is discarded with the rest of the
  /// draft, because it would go straight back out on the wire.
  it("discards a draft holding a process the wire does not know", () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({ ...blank(), repo: 2, process: "Ponder" }),
    );

    expect(stored()).toEqual(blank());
  });

  /// An untouched Process is no more worth coming back to than an untouched
  /// anything else: a page holding one and nothing else is a page worth
  /// nothing, and a page holding a picked one is worth keeping.
  it("keeps a draft whose only touched field is the process", () => {
    keep({ ...blank(), process: "Develop" });

    expect(stored()).toEqual({ ...blank(), process: "Develop" });
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
