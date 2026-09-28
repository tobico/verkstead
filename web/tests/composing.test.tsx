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

import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@solidjs/testing-library";
import { beforeEach, describe, expect, it } from "vitest";

import type {
  AbandonedRepo,
  Adopted,
  ConversationEntry,
  Created,
  DevicesView,
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
import {
  ATTACH_REFUSAL,
  SERVER_REFUSAL,
} from "../src/workbench/Composer";
import { BRANCH_REFUSAL, TARGET, TARGET_REFUSAL } from "../src/workbench/Setup";
// The Processes the picker offers and the words they are said in, read rather
// than spelled out again: what the row offers is that list and nothing else.
import { OFFERED, PROCESS, ROLES } from "../src/workbench/processes";
import {
  CREATE_REFUSAL,
  REFUSAL as REPO_REFUSAL,
} from "../src/repos/RepoList";
import { osIcon } from "../src/devices";
import {
  draftingOn,
  repoParent,
  setDraftingOn,
  setRepoParent,
} from "../src/remembered";
import {
  COMPOSING,
  blank,
  keep,
  leaveRefusals,
  stored,
  type Composed,
} from "../src/workbench/composing";
import {
  BRANCHES,
  NO_PAIRINGS,
  OPEN,
  PROFILES,
  REPOS,
    SETTINGS, SIDEBAR,
  drawn,
  mount,
  openAgent,
  theWorkbench,
} from "./bench";
import { carrying, drag, dropOn } from "./dragging";
// The path field's own rows are `browsed`, the listbox's being `rows` below:
// one page draws both, and a browse's levels are not what a picker offers.
import { browse, held, listingAt, rows as browsed } from "./fields";
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
import linked from "./fixtures/devices-linked.json" with { type: "json" };
import listing from "./fixtures/directories.json" with { type: "json" };
import made from "./fixtures/repo.json" with { type: "json" };
import told from "./fixtures/settings.json" with { type: "json" };

/// The roadmaps nothing is driving, as the server answers for them: three of
/// them in one repo, the last found on a branch that has not merged.
const ABANDONED = abandoned as AbandonedRepo[];

/// And this machine with a cluster around it: two members, one a macOS laptop
/// that answered the last dial and one a WSL that did not. Which is the reading
/// the device select is drawn off — see the `describe` below.
const LINKED = linked as DevicesView;

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

/// And what the server says when it takes one: the record it made, which is what
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
  return [...container.querySelectorAll(`.${pill.attachmentName}`)].map(
    (name) => name.textContent!.trim(),
  );
}

/// And the names on the chips beside them, which is the same row read for the
/// MCP servers rather than for the files.
function chips(container: ParentNode): string[] {
  return [...container.querySelectorAll(`.${pill.serverName}`)].map((name) =>
    name.textContent!.trim(),
  );
}

/// The Attach menu, opened.
///
/// Waited out rather than read straight away: the declarations are read when
/// the menu opens rather than when the page does — see `ServerRows` in
/// `Composer.tsx` — so the card comes down saying it is reading them and fills
/// in after.
async function attachMenu(): Promise<HTMLElement> {
  fireEvent.click(
    await waitFor(() => screen.getByRole("button", { name: "Attach" })),
  );

  const card = screen.getByRole("menu", { name: "Attach" });
  await waitFor(() =>
    expect(within(card).queryByText("Reading the settings…")).toBeNull(),
  );

  return card;
}

/// What it is offering, in the order it offers it.
function offeredIn(card: ParentNode): string[] {
  return [...card.querySelectorAll('[role="menuitem"]')].map((row) =>
    row.textContent!.trim(),
  );
}

/// And taken back again, which is what a test that only wanted to read the rows
/// does with it: the trigger toggles, so a card left hanging is a card the next
/// press shuts instead of opening.
function shutMenu(): void {
  fireEvent.keyDown(document, { key: "Escape" });
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

/// The member of the cluster the tests below draft onto, and the one beside it:
/// this device first and then the membership in the order it lists them — a
/// reachable laptop, and a WSL that did not answer the last dial.
const MEMBER = LINKED.members[0]!.identity;
const AWAY = LINKED.members[1]!.identity;

/// Where a call for that member stands. The prefix takes the place of
/// `/api/ui`, so the far end sees the path the browser would have written
/// locally — see `src/api/client.ts`, which is the one place a path is composed.
const at = (path: string) => `/api/ui/members/${MEMBER.device}${path}`;

/// The Repos registered on the member, which are not this device's: a Repo id is
/// one Verkstead's own, so the two lists collide by construction and the names
/// are what says which of them a dropdown is drawn off.
const THEIRS: RepoEntry[] = REPOS.map((repo) => ({
  ...repo,
  name: `${repo.name}-on-the-laptop`,
}));

/// The workbench with a member linked, answering for its own registry and for
/// what each of its Repos was last grilled with.
///
/// At module scope rather than inside the select's own `describe`, because two
/// of them stand a cluster up: the select and the reading it changes, and the two
/// rows at the foot of the Repo dropdown, which are the same reading one press
/// further on.
function theCluster(...answers: Parameters<typeof serving>) {
  return theWorkbench(
    ...REMEMBERED,
    whenever("/api/ui/devices", json(LINKED)),
    whenever(at("/repos"), json(THEIRS)),
    whenever(at("/profiles"), json(PROFILES)),
    ...THEIRS.map((repo) =>
      whenever(at(`/repos/${repo.id}/pairings`), json(NO_PAIRINGS)),
    ),
    ...answers,
    json(null),
  );
}

/// Draw the compose page over a cluster and pick the member, which is where
/// everything about the two rows on another device starts.
///
/// Waited for the pick to have landed rather than merely made: the select is
/// drawn off a read of the membership, and what a pick changes is what every
/// control under it is about — so a test that went on at once would be filling in
/// the dropdown this device is still holding.
async function draftingOnTheMember(container: ParentNode): Promise<void> {
  await composing(container);
  await drawn(container, `.${setup.deviceSelect}`);
  pick("Device", MEMBER.name);
  await waitFor(() => expect(showing("Device")).toBe(MEMBER.name));
}

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

/// And the same endpoints on the member, which is where a page drafting onto it
/// walks: the same sequence under the Relay's prefix, and nothing of it on this
/// device.
///
/// Its Repos remember something too, for [`REMEMBERED`]'s reason one machine
/// along: the press carries a grilling, and the roles it needs answered are
/// prefilled from the memory the *picked* device holds.
function creatingOnTheMember(...answers: Parameters<typeof serving>) {
  return theCluster(
    ...THEIRS.map((repo) =>
      whenever(
        at(`/repos/${repo.id}/pairings`),
        json(remembering(PROFILES[0]!, PROFILES[1]!, PROFILES[2]!)),
      ),
    ),
    whenever(at("/conversations"), json({ Started: { id: OPEN.id } }), "POST"),
    whenever(at(`/conversations/${OPEN.id}/brief`), json("Saved"), "POST"),
    whenever(at(`/conversations/${OPEN.id}/branch`), json("Renamed"), "POST"),
    whenever(at(`/conversations/${OPEN.id}/base`), json("Recorded"), "POST"),
    whenever(at(`/conversations/${OPEN.id}/grill`), json("Started"), "POST"),
    // And the draft the page lands on, which is the member's own record read
    // through the same prefix — the page it lands on is an ordinary page of a
    // member's Conversation.
    whenever(at(`/conversations/${OPEN.id}`), json(OPEN)),
    ...answers,
  );
}

/// Where one file goes up on the member, which is [`upload`] one machine along.
const uploadedThere = (name: string) =>
  at(`/conversations/${OPEN.id}/attachments/${name}`);

/// Pick one of the member's repos, the dropdown being drawn off its registry
/// rather than this device's.
async function pickRepoThere(container: ParentNode, id: number): Promise<void> {
  if (container.querySelector(`.${setup.repoSelect}`) === null) {
    await openRepo(container);
  }

  await waitFor(() => expect(offered("Repo")).toHaveLength(THEIRS.length));
  pick("Repo", THEIRS.find((repo) => repo.id === id)!.name);
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

/// A native `<select>` by the label that names it, once the row a test is about
/// is really among its options.
///
/// Waited for the option rather than for the element: the branches and the repos
/// alongside are reads of their own, so the control is on the page before what it
/// offers is — and a `<select>` set to a value it has no option for keeps the one
/// it had, which is a change nothing hears about.
function offering(label: string, value: string): Promise<HTMLSelectElement> {
  return waitFor(() => {
    const select = screen.getByLabelText(label) as HTMLSelectElement;
    if (![...select.options].some((option) => option.value === value)) {
      throw new Error(`the "${label}" select has no option for ${value} yet`);
    }
    return select;
  });
}

/// What the Create repo card's two fields are labelled, which is how they are
/// found — on this device and on a member both, it being the one card.
const WHERE = "Where it goes";
const CALLED = "What it is called";

/// And what the Open repo card's one field is labelled.
const PATH = "Absolute path of a git repository";

/// Fill the two in and send them.
function make(parent: string, name: string): void {
  fireEvent.input(screen.getByLabelText(WHERE), { target: { value: parent } });
  fireEvent.input(screen.getByLabelText(CALLED), { target: { value: name } });
  fireEvent.click(screen.getByRole("button", { name: "Create" }));
}

/// And what stands where the GitHub tick would have, on a Verkstead with no
/// token saved — which is what says the card has read the settings and is taking
/// creates.
function noRemote(): HTMLElement | null {
  return screen.queryByText(/needs a remote/i);
}

/// What Verkstead has been told, which the Create card asks exactly one thing
/// of: whether a GitHub token is saved. The fixture's has one.
const TOKENED = told as SettingsView;

/// And the same with none, which is what a Verkstead nobody has told anything
/// looks like — and what every test that is not about the tick reads.
const UNTOKENED: SettingsView = { ...TOKENED, github_token: null };

describe("the compose page", () => {
  // Per device, so every test starts on a device holding nothing — and with
  // nothing left over from the create the test before it made.
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(null, 0, []);
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

describe("the device the compose page is drafting onto", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(null, 0, []);
  });

  /// Every device the select offers, and the mark each row wears.
  const marks = (): (string | null | undefined)[] =>
    offered("Device").map((row) =>
      row.querySelector("svg path")?.getAttribute("d"),
    );

  /// What one operating system's mark is drawn as, for comparing against those.
  const mark = (os: string) => [osIcon(os).icon[4]].flat()[0];

  /// With nothing linked the row is the row it has always been. Which is nearly
  /// every Verkstead, and the reason this is the first thing asked: a select
  /// naming the one machine there is would be a control with one answer.
  it("draws no select at all where there is no other device", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.repoSelect}`);

    expect(container.querySelector(`.${setup.deviceSelect}`)).toBeNull();
    expect(screen.queryByLabelText("Device")).toBeNull();
  });

  it("lists this device and each member with its mark, the unreachable one too", async () => {
    theCluster();
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);

    // This device to begin with, nothing having been picked in this browser.
    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));

    expect(rows("Device")).toEqual([
      LINKED.this.name,
      MEMBER.name,
      AWAY.name,
    ]);

    // The mark a device wears wherever it is drawn: the WSL reads as Linux,
    // which is the whole point of the word — and it is listed like any other,
    // the list being the membership rather than a reachability probe.
    expect(marks()).toEqual([
      mark(LINKED.this.os),
      mark(MEMBER.os),
      mark(AWAY.os),
    ]);
  });

  /// The select stands left of the Repo, because the Repo is one of the picked
  /// device's: the question above *which repository* is *whose registry*.
  it("stands at the head of the setup row", async () => {
    theCluster();
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);

    const options = container.querySelector(`.${setup.options}`)!;
    expect(options.firstElementChild!.className).toContain(setup.deviceSelect!);
  });

  /// The whole of what a pick changes: every control under the row reads the
  /// picked device, which is one reading rather than anything composed anew.
  it("makes the repos, the branches and the pairings the picked device's", async () => {
    const fetching = theCluster(
      ...THEIRS.map((repo) =>
        whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);
    pick("Device", MEMBER.name);

    // The Repo dropdown is the member's registry, which is what the names say.
    await waitFor(() =>
      expect(offered("Repo")).toHaveLength(THEIRS.length),
    );
    expect(rows("Repo")).toEqual(THEIRS.map((repo) => repo.name));

    pick("Repo", THEIRS[1]!.name);

    // And the base picker under it, and the prefill the three role pickers show
    // — both asked of the member, and neither asked of this device. The panel
    // has to be open for the base picker to be on the page at all: it is drawn
    // inside the Repo panel, with the branch and the companions it belongs
    // beside.
    await openRepo(container);
    await waitFor(() =>
      expect(askedFor(fetching, at(`/repos/${THEIRS[1]!.id}/branches`))).toBe(1),
    );
    await waitFor(() =>
      expect(askedFor(fetching, at(`/repos/${THEIRS[1]!.id}/pairings`))).toBe(1),
    );
    expect(askedFor(fetching, `/api/ui/repos/${THEIRS[1]!.id}/branches`)).toBe(0);

    // And the Profiles the three pairing pickers are made of, which is the last
    // of the reads under the row: an account is a directory on one machine, so
    // whose accounts is the same question as whose Repos.
    await waitFor(() =>
      expect(askedFor(fetching, at("/profiles"))).toBeGreaterThan(0),
    );
    expect(screen.getByLabelText("Works alongside")).toBeTruthy();
  });

  /// The prefill is the picked device's memory of that Repo, which is what a
  /// draft created there would arrive showing — so it is read off the member and
  /// what the pickers stand on is its answer rather than this device's.
  it("shows the prefill the picked device remembers", async () => {
    theCluster(
      whenever(
        at(`/repos/${THEIRS[0]!.id}/pairings`),
        json(remembering(PROFILES[2]!, PROFILES[1]!, PROFILES[0]!)),
      ),
      ...THEIRS.map((repo) =>
        whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);
    pick("Device", MEMBER.name);

    await waitFor(() => expect(offered("Repo")).toHaveLength(THEIRS.length));
    pick("Repo", THEIRS[0]!.name);

    await openAgent(container);
    await waitFor(() =>
      expect(showing("Grilling")).toBe("Sonnet 5 — sonnet"),
    );
  });

  /// A Repo id, a companion's Repo id and a Pairing's Profile id are each one
  /// device's own, so the pick takes them with it. The words the human wrote do
  /// not move: a brief is a brief on any machine.
  it("drops what named the other machine and keeps what was written", async () => {
    theCluster(
      ...REPOS.map((repo) =>
        whenever(`/api/ui/repos/${repo.id}/branches`, json(BRANCHES)),
      ),
    );
    const { container } = mount("/compose");

    const brief = await composing(container);
    fireEvent.input(brief, { target: { value: "the piece of work" } });
    choose(container, new File(["notes"], "notes.md"));

    await pickRepo(container, REPOS[0]!.id);
    await openRepo(container);
    fireEvent.input(await drawn<HTMLInputElement>(container, "#branch"), {
      target: { value: "a-branch-of-my-own" },
    });
    fireEvent.change(await offering("Base branch", BRANCHES[1]!), {
      target: { value: BRANCHES[1] },
    });
    fireEvent.change(
      await offering("Works alongside", String(REPOS[1]!.id)),
      { target: { value: String(REPOS[1]!.id) } },
    );
    await drawn(container, `.${setup.companion}`);
    await openAgent(container);
    pick("Grilling", "Claude Code Sonnet 5 — sonnet");

    await waitFor(() => {
      const held = stored();
      expect(held.base).toBe(BRANCHES[1]);
      expect(held.companions).toHaveLength(1);
      expect(held.grilling).not.toBeNull();
    });

    await drawn(container, `.${setup.deviceSelect}`);
    pick("Device", MEMBER.name);

    // The repo, the base, the companions and the three pairings are gone with
    // the machine they named: the Repo slot is the invitation again, which is
    // the state a page that has picked nothing is in.
    await drawn(container, `.${setup.repoSelect}`);
    expect(showing("Device")).toBe(MEMBER.name);
    expect(showing("Repo")).toBe("Select");
    expect(container.querySelector(`.${setup.companion}`)).toBeNull();

    const left = stored();
    expect(left.repo).toBeNull();
    expect(left.base).toBeNull();
    expect(left.companions).toEqual([]);
    expect(left.grilling).toBeNull();
    expect(left.implementation).toBeNull();
    expect(left.review).toBeNull();

    // And what the human wrote stayed where it was.
    expect(
      (container.querySelector(`.${composer.box} textarea`) as HTMLTextAreaElement)
        .value,
    ).toBe("the piece of work");
    expect(left.brief).toBe("the piece of work");
    expect(left.branch).toBe("a-branch-of-my-own");

    // The files with them: a file picked here is a handle this page is holding
    // rather than anything a machine has a name for, so there is nothing about
    // it that a move could invalidate.
    expect(pills(container)).toEqual(["notes.md"]);
  });

  /// Remembered in the browser rather than in the compose draft beside it, so
  /// the pick outlives a draft of nothing and the create that drops one.
  it("comes back to the device it was left on", async () => {
    theCluster(
      ...THEIRS.map((repo) =>
        whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const first = mount("/compose");

    await composing(first.container);
    await drawn(first.container, `.${setup.deviceSelect}`);
    pick("Device", MEMBER.name);
    await waitFor(() => expect(showing("Device")).toBe(MEMBER.name));

    cleanup();
    const second = mount("/compose");

    await composing(second.container);
    await drawn(second.container, `.${setup.deviceSelect}`);
    await waitFor(() => expect(showing("Device")).toBe(MEMBER.name));

    // And the Repo dropdown under it is the member's, which is the half that
    // makes the remembered pick worth anything.
    await waitFor(() =>
      expect(rows("Repo")).toEqual(THEIRS.map((repo) => repo.name)),
    );
  });

  /// The pick is in this browser and the membership is on the server, so the two
  /// come apart: a device unlinked while this page was shut is a page that would
  /// otherwise be pointed at a machine nothing can reach.
  it("reads as this device where the pick has left the cluster", async () => {
    theCluster();
    setDraftingOn("d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0");
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);

    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));

    // And the browser is put back too, so the correction is made once rather
    // than on every visit.
    expect(draftingOn()).toBeNull();

    // And this device's own registry under it, rather than the lost machine's.
    await waitFor(() => expect(rows("Repo")).toEqual(REPOS.map((r) => r.name)));
  });

  /// And the same correction where the select is not drawn at all: the last
  /// member unlinked takes the control away, and a page left pointed at that
  /// member would have no control to put it right with.
  /// And a draft left on that machine goes back with it. The repo it names is a
  /// number in the lost device's registry, so a page that read as this device and
  /// kept it would be drafting against whatever repository happens to hold that
  /// number here.
  it("takes a stored draft off a device that has left the cluster", async () => {
    const lost = "d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0";

    // What this device answers for an id that is no member of its cluster: a
    // refusal before anything is dialled, which is what the reads under the row
    // get for as long as the page is pointed at the machine that has gone.
    theCluster(
      whenever(
        `/api/ui/members/${lost}/repos`,
        json({ error: "no device by that id is a member" }, 404),
      ),
      whenever(
        `/api/ui/members/${lost}/repos/${REPOS[1]!.id}/pairings`,
        json({ error: "no device by that id is a member" }, 404),
      ),
    );
    keep({
      ...blank(),
      device: lost,
      repo: REPOS[1]!.id,
      brief: "the piece of work",
    });
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);

    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));
    await waitFor(() => expect(stored().device).toBeNull());
    expect(stored().repo).toBeNull();

    // The Repo slot is the invitation again, over this device's own registry —
    // and what the human wrote is where they left it.
    await drawn(container, `.${setup.repoSelect}`);
    expect(showing("Repo")).toBe("Select");
    expect(stored().brief).toBe("the piece of work");
  });

  it("reads as this device where the cluster has gone altogether", async () => {
    setDraftingOn(MEMBER.device);
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);

    await waitFor(() => expect(draftingOn()).toBeNull());
    expect(container.querySelector(`.${setup.deviceSelect}`)).toBeNull();
    await waitFor(() => expect(rows("Repo")).toEqual(REPOS.map((r) => r.name)));
  });

  /// The list work is loaded from is this device's own — the roadmaps nothing is
  /// driving — so a row of it names a Repo on this machine.
  it("puts the work back on this device when a roadmap is loaded, and settles", async () => {
    theCluster(whenever("/api/ui/abandoned-roadmaps", json(ABANDONED)));
    const { container } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);
    pick("Device", MEMBER.name);
    await waitFor(() => expect(showing("Device")).toBe(MEMBER.name));

    fireEvent.click(
      await drawn(container, `.${composer.actions} > .${menu.trigger}`),
    );
    fireEvent.click(
      await waitFor(() => {
        const level = container.querySelector<HTMLButtonElement>(
          `.${menu.nested}`,
        );
        if (!level || level.disabled) throw new Error("still greyed");
        return level;
      }),
    );
    fireEvent.click(await drawn(container, `.${composer.roadmapRow}`));
    await drawn(container, `.${composer.loaded}`);

    // Back here, and settled while the card is over the box: the stage is in a
    // repository on this machine, so moving the work off would be moving it
    // away from what it is continuing.
    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));
    expect(draftingOn()).toBeNull();
    expect((screen.getByLabelText("Device") as HTMLButtonElement).disabled).toBe(
      true,
    );

    // And a select again the moment it is cleared.
    fireEvent.click(await drawn(container, `.${composer.clear}`));
    await waitFor(() =>
      expect(
        (screen.getByLabelText("Device") as HTMLButtonElement).disabled,
      ).toBe(false),
    );
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
    leaveRefusals(null, 0, []);
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

  /// All five now, Fix Merge Issues having landed last. A Process is offered
  /// only once its stage has landed, as an agent type is offered only once it
  /// can launch the real thing.
  it("offers the processes that have landed and no others", async () => {
    theWorkbench();
    const { container } = mount("/compose");

    await composing(container);
    await waitFor(() => expect(screen.getByLabelText("Process")).toBeTruthy());

    expect(rows("Process")).toEqual(OFFERED.map((process) => PROCESS[process]));
    expect(OFFERED).toEqual([
      "Develop",
      "Tinker",
      "Investigate",
      "Review",
      "FixMergeIssues",
    ]);
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
    leaveRefusals(null, 0, []);
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

  /// And under a **Fix Merge Issues**, which is the other Process pointed at
  /// work already somewhere else: the field follows the pick on this page as it
  /// does on a draft's own composer, off the same list.
  it("follows a pick onto Fix merge issues and off it again", async () => {
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    await composing(container);
    await pickRepo(container, REPOS[1]!.id);
    await openRepo(container);

    await waitFor(() => expect(screen.getByLabelText("Branch")).toBeTruthy());

    pick("Process", PROCESS.FixMergeIssues);
    const field = (await waitFor(() =>
      screen.getByLabelText("Target"),
    )) as HTMLInputElement;
    expect(field.placeholder).toBe(TARGET);

    pick("Process", PROCESS.Develop);
    await waitFor(() => expect(screen.queryByLabelText("Target")).toBeNull());
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

  /// And it keeps up with a name being typed out, which is the only way a name
  /// ever reaches this box: `#4` is a whole name on the way to `#412`, so a fill
  /// that stopped at the first one would leave the field naming a pull request
  /// nobody meant.
  it("follows a name being typed a character at a time", async () => {
    composedAsReview();
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    const box = await composing(container);
    await openRepo(container);
    await waitFor(() => screen.getByLabelText("Target"));

    const typed = "Wrap up #412 today.";
    for (let upto = 1; upto <= typed.length; upto += 1) {
      fireEvent.input(box, { target: { value: typed.slice(0, upto) } });
    }

    await waitFor(() =>
      expect((screen.getByLabelText("Target") as HTMLInputElement).value).toBe(
        "#412",
      ),
    );
    expect(stored().target).toBe("#412");
  });

  /// The same for a URL, which is longer and goes the same way.
  it("follows a url being typed a character at a time", async () => {
    composedAsReview();
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    const box = await composing(container);
    await openRepo(container);
    await waitFor(() => screen.getByLabelText("Target"));

    const typed = "See https://github.com/tobico/verkstead/pull/412 please.";
    for (let upto = 1; upto <= typed.length; upto += 1) {
      fireEvent.input(box, { target: { value: typed.slice(0, upto) } });
    }

    await waitFor(() =>
      expect((screen.getByLabelText("Target") as HTMLInputElement).value).toBe(
        "https://github.com/tobico/verkstead/pull/412",
      ),
    );
  });

  /// And it lets go the moment the human types in the field, whatever the box
  /// says afterwards: correcting its own fill is not the same as overwriting
  /// theirs.
  it("stops following once the field has been typed in", async () => {
    composedAsReview();
    theWorkbench(...REMEMBERED, json(null));
    const { container } = mount("/compose");

    const box = await composing(container);
    await openRepo(container);
    const field = (await waitFor(() =>
      screen.getByLabelText("Target"),
    )) as HTMLInputElement;

    fireEvent.input(box, { target: { value: "Wrap up #41" } });
    await waitFor(() => expect(field.value).toBe("#41"));

    fireEvent.input(field, { target: { value: "rate-limiting" } });
    await waitFor(() => expect(stored().target).toBe("rate-limiting"));

    fireEvent.input(box, { target: { value: "Wrap up #412 today." } });
    await waitFor(() => expect(stored().brief).toContain("today"));
    expect(field.value).toBe("rate-limiting");
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

  /// And a **Fix Merge Issues** page is the same press over the same field: it
  /// waits on a brief, a target and the one role its table names, and the
  /// kickoff behind it is the take-up rather than a grill start.
  it("takes up what a fix merge issues page is pointed at", async () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({
        ...blank(),
        repo: REPOS[1]!.id,
        process: "FixMergeIssues" satisfies Process,
        target: "#41",
      }),
    );

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

    const box = await composing(container);

    // Nothing written yet, so the press is inert and says the three things it
    // is waiting on — one role, this Process using one.
    const start = screen.getByRole("button", { name: "Start work" });
    await waitFor(() =>
      expect(start.getAttribute("aria-disabled")).toBe("true"),
    );
    expect(start.getAttribute("title")).toBe(
      "Starting needs a brief, a target, and one role picked and working.",
    );

    fireEvent.input(box, { target: { value: "The limiter will not merge." } });
    await waitFor(() => expect(showing("Agent")).toBeTruthy());
    await waitFor(() =>
      expect(start.getAttribute("aria-disabled")).toBe("false"),
    );

    fireEvent.click(start);

    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/process`)).toEqual(
        { process: "FixMergeIssues" satisfies Process },
      ),
    );
    await waitFor(() =>
      expect(sent(fetching, `/api/ui/conversations/${OPEN.id}/target`)).toEqual(
        { target: "#41" },
      ),
    );
    await waitFor(() =>
      expect(
        writes(fetching, `/api/ui/conversations/${OPEN.id}/take-up`),
      ).toBe(1),
    );
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
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
    leaveRefusals(null, 0, []);
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
    leaveRefusals(null, 0, []);
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

/// The press, once a member is picked: the Conversation is made over there, every
/// field and every file of the replay goes there, the kickoff is that device's,
/// and the page lands on the URL a member's Conversation already stands at.
///
/// Which is the whole of what this half adds. The replay is the replay — the same
/// endpoints in the same order, and each of them refused in its own words — so
/// what is asked here is *where* every one of them went and where the page ended
/// up, rather than the sequence itself: that is the first `describe` in this
/// file's, over this device.
describe("starting the work on the picked device", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(null, 0, []);
  });

  /// Every row of the sidebar as it reads aloud, which is where the device a
  /// Conversation is on is said: the merged list carries the name on the row.
  const listed = (container: ParentNode): string[] =>
    [...container.querySelectorAll(`.${sidebar.open}`)].map(
      (card) => card.getAttribute("aria-label") ?? "",
    );

  it("creates on the member, replays every field there and lands on its URL", async () => {
    const fetching = creatingOnTheMember();
    const { container, history } = mount("/compose");

    await draftingOnTheMember(container);
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepoThere(container, THEIRS[1]!.id);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    // The Conversation started against a Repo of the member's, on the member.
    await waitFor(() =>
      expect(sent(fetching, at("/conversations"))).toEqual({
        repo_id: THEIRS[1]!.id,
      }),
    );
    await waitFor(() =>
      expect(sent(fetching, at(`/conversations/${OPEN.id}/brief`))).toEqual({
        markdown: "Make the widget",
      }),
    );
    await waitFor(() =>
      expect(writes(fetching, at(`/conversations/${OPEN.id}/grill`))).toBe(1),
    );

    // And nothing of it on this device: a Conversation half made here and half
    // there is two records neither of which is the work.
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/brief`)).toBe(0);
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);

    // The page lands on the member's URL for it, which is the path a member's
    // Conversation has always stood at.
    await waitFor(() =>
      expect(
        history
          .get()
          .startsWith(`/devices/${MEMBER.device}/conversations/${OPEN.id}`),
      ).toBe(true),
    );

    // What was being composed is on that machine by now, so this browser holds
    // none of it — and goes on drafting onto the member, the pick outliving the
    // draft it was made for.
    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeNull());
    expect(draftingOn()).toBe(MEMBER.device);
  });

  /// The branch and the base are the two fields under the Repo that a member's
  /// answer decides, so both go the same way as the start above them.
  it("names the branch and records the base on the member", async () => {
    const fetching = creatingOnTheMember(
      ...THEIRS.map((repo) =>
        whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepoThere(container, THEIRS[1]!.id);

    await openRepo(container);
    fireEvent.input(await drawn(container, "#branch"), {
      target: { value: "widget-work" },
    });
    fireEvent.change(await offering("Base branch", BRANCHES[1]!), {
      target: { value: BRANCHES[1] },
    });

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    await waitFor(() =>
      expect(sent(fetching, at(`/conversations/${OPEN.id}/branch`))).toEqual({
        branch: "widget-work",
      }),
    );
    await waitFor(() =>
      expect(sent(fetching, at(`/conversations/${OPEN.id}/base`))).toEqual({
        branch: BRANCHES[1],
      }),
    );
    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/branch`)).toBe(0);
  });

  /// A `File` is a handle this page is holding rather than anything a machine has
  /// a name for, so it travels with the Brief it was picked beside — and it goes
  /// up before the work starts, attachments freezing when it does.
  it("puts the files it holds on the member, before the work is kicked off", async () => {
    const fetching = creatingOnTheMember(
      whenever(uploadedThere("notes.md"), attached(9, "notes.md"), "POST"),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    choose(container, new File(["notes"], "notes.md"));
    await pickRepoThere(container, THEIRS[0]!.id);
    await rolesAnswered();

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(writes(fetching, uploadedThere("notes.md"))).toBe(1),
    );
    expect(order(fetching, uploadedThere("notes.md"))).toBeLessThan(
      order(fetching, at(`/conversations/${OPEN.id}/grill`)),
    );

    // And nowhere near this device, which has no Conversation for it to go on.
    expect(writes(fetching, upload("notes.md"))).toBe(0);
  });

  /// *Save as draft* stops after the fields wherever it is pressed, and the draft
  /// it leaves is one of the member's — which is how the merged sidebar draws it:
  /// the row says whose machine the work is on.
  it("saves a draft there, and the row reads under that device", async () => {
    // The sidebar as the hub answers it, which gains the member's new draft the
    // moment there is one: the page reads the list again when a create lands.
    let merged: ConversationEntry[] = SIDEBAR;
    const fetching = creatingOnTheMember(
      whenever("/api/ui/conversations", () => json(merged)()),
    );
    const { container, history } = mount("/compose");

    await draftingOnTheMember(container);
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepoThere(container, THEIRS[0]!.id);

    merged = [
      {
        ...SIDEBAR[0]!,
        id: OPEN.id,
        branch: "",
        branch_named: false,
        rank: `Zz-${MEMBER.device}`,
        repo: THEIRS[0]!.name,
        state: "Draft",
        device: {
          id: MEMBER.device,
          name: MEMBER.name,
          os: MEMBER.os,
          reachable: true,
        },
      },
      ...SIDEBAR,
    ];

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    await waitFor(() =>
      expect(sent(fetching, at(`/conversations/${OPEN.id}/brief`))).toEqual({
        markdown: "Make the widget",
      }),
    );

    // Nothing kicked off, here or there: the press creates and stops.
    expect(writes(fetching, at(`/conversations/${OPEN.id}/grill`))).toBe(0);

    await waitFor(() =>
      expect(
        history
          .get()
          .startsWith(`/devices/${MEMBER.device}/conversations/${OPEN.id}`),
      ).toBe(true),
    );

    // And it is in the merged list under the machine that will do the work,
    // which every row of that list says out loud.
    await waitFor(() =>
      expect(
        listed(container).some((row) =>
          row.startsWith(`Draft, ${MEMBER.name},`),
        ),
      ).toBe(true),
    );
  });

  /// Ids collide by construction — every Verkstead issues a Conversation 1 — so
  /// what a create was refused is left against the Conversation *and* the device.
  /// A refusal keyed by the number alone would be drawn on the composer of an
  /// unrelated draft the moment two devices are in play.
  it("says what the member refused on the draft it made, and not on the same number here", async () => {
    const said = `The branch could not be named: ${BRANCH_REFUSAL.NotABranchName}`;

    creatingOnTheMember(
      whenever(
        at(`/conversations/${OPEN.id}/branch`),
        json("NotABranchName"),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepoThere(container, THEIRS[0]!.id);
    await rolesAnswered();

    await openRepo(container);
    fireEvent.input(await drawn(container, "#branch"), {
      target: { value: "not a branch name" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    // On the composer of the draft that was made, which is the member's page.
    await waitFor(() => expect(screen.getByText(said)).toBeTruthy());

    // And on nothing else. This device's own Conversation 1 is another piece of
    // work entirely, and what refused a create on the laptop is no business of
    // its composer.
    cleanup();
    const here = mount(`/conversations/${OPEN.id}`);

    await composing(here.container);
    expect(screen.queryByText(said)).toBeNull();
  });

  /// The Repo the trigger names and the rows the companions are offered as are
  /// read off the picked device's registry, because that is the registry the work
  /// will be done against.
  it("names the repo and the companion rows off the member's registry", async () => {
    creatingOnTheMember(
      ...THEIRS.map((repo) =>
        whenever(at(`/repos/${repo.id}/branches`), json(BRANCHES)),
      ),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await pickRepoThere(container, THEIRS[1]!.id);

    const trigger = await drawn(
      container,
      `.${setup.repoOption} .${setup.optionValue}`,
    );
    await waitFor(() => expect(trigger.textContent).toBe(THEIRS[1]!.name));

    // And the repos it could run alongside are the rest of that same registry.
    await openRepo(container);
    const alongside = await offering("Works alongside", String(THEIRS[0]!.id));
    const names = [...alongside.options].map((option) => option.textContent);
    expect(names).toContain(THEIRS[0]!.name);
    expect(names.some((name) => REPOS.some((repo) => repo.name === name))).toBe(
      false,
    );

    fireEvent.change(alongside, { target: { value: String(THEIRS[0]!.id) } });
    const row = await drawn(container, `.${setup.companion}`);
    expect(row.textContent).toContain(THEIRS[0]!.name);
  });

  /// And with the select left where it starts, both presses are the presses they
  /// were before there was a select at all: a cluster around this device changes
  /// nothing about work being drafted on it.
  it("creates here and lands here with this device left picked", async () => {
    const fetching = theCluster(
      whenever(
        "/api/ui/conversations",
        json({ Started: { id: OPEN.id } }),
        "POST",
      ),
      whenever(`/api/ui/conversations/${OPEN.id}/brief`, json("Saved"), "POST"),
      whenever(`/api/ui/conversations/${OPEN.id}/grill`, json("Started"), "POST"),
    );
    const { container, history } = mount("/compose");

    await composing(container);
    await drawn(container, `.${setup.deviceSelect}`);
    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));

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
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(1),
    );

    // The local path, with no device in it — which is the URL this app wrote
    // before there was a cluster and the URL it writes for its own work still.
    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );
    expect(writes(fetching, at("/conversations"))).toBe(0);
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
    leaveRefusals(null, 0, []);
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
    leaveRefusals(null, 0, []);
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

  /// The tick beside the two fields, where a token is saved for it to be drawn
  /// by — this describe's own subject, where the labels and the fill-in above it
  /// are shared with the cluster's copy of the same card.
  const ON_GITHUB = "Create it on GitHub too, privately";

  /// The tick as the card is drawing it, or `null` where it is not drawn at all.
  function tick(): HTMLInputElement | null {
    return screen.queryByLabelText(ON_GITHUB) as HTMLInputElement | null;
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
    setRepoParent(null, "/home/ada/src");
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
    expect(repoParent(null)).toBe("/home/ada/src");
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
    expect(repoParent(null)).toBe("");
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

/// And the two rows on whichever device the select names: **Open repo** and
/// **Create repo** browsing and making directories on the machine that will do
/// the work.
///
/// Everything under the select reads the picked device already — the field
/// browses through the Relay, the registration and the `git init` go out under
/// it — so what is asked here is that being true from this page end to end,
/// against a second machine. And the one thing that was not: where the last repo
/// went, which is a fact about the machine it went on rather than about the
/// browser, and which a single key answered with a path on this one.
describe("the two repo rows on the picked device", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(null, 0, []);
  });

  /// What the member's filesystem looks like, which is nothing like this
  /// machine's: the fixture's `/home/ada/src` is this device's own, and the whole
  /// point of a browse going out under the member is that what comes back is the
  /// other machine's directories.
  const OVER_THERE: DirectoryListing = {
    Listed: {
      path: "/mnt/code",
      entries: [
        { kind: "Directory", name: "projects", path: "/mnt/code/projects" },
        { kind: "Directory", name: "widgets", path: "/mnt/code/widgets" },
      ],
    },
  };

  /// And this machine's own, which is the fixture's — served alongside so that a
  /// browse that went to the wrong end is an assertable reading rather than a
  /// call nothing answered.
  const HERE = listing as DirectoryListing;

  /// And the level above each of them, because a field being typed into asks for
  /// one: the text `/mnt/code` names `/mnt` with `code` half-written in it, which
  /// is how a browse narrows as somebody types — see `src/PathField.tsx`. A
  /// filesystem has those levels, so the stand-in for one does too.
  const ABOVE_THEIRS: DirectoryListing = {
    Listed: {
      path: "/mnt",
      entries: [{ kind: "Directory", name: "code", path: "/mnt/code" }],
    },
  };

  const ABOVE_HERE: DirectoryListing = {
    Listed: {
      path: "/home/ada",
      entries: [{ kind: "Directory", name: "src", path: "/home/ada/src" }],
    },
  };

  /// A repository opened on the member: none of the ones registered there, so a
  /// draft landing on it is unmistakably the answer's doing rather than the
  /// list's.
  const THEIR_OPENED: RepoEntry = {
    id: 5151,
    name: "widgets-over-there",
    path: "/mnt/code/widgets",
    default_branch: "main",
  };

  /// And one a create made there, in the same place and for the same reason.
  const THEIR_MADE: RepoView = {
    ...(made as RepoView),
    id: 5252,
    name: "widgets",
    path: "/mnt/code/widgets",
  };

  /// And one made here, for the test that leaves the select alone.
  const MADE_HERE: RepoView = {
    ...(made as RepoView),
    id: 5353,
    name: "widgets",
    path: "/home/ada/src/widgets",
  };

  /// The workbench with a member linked, both filesystems answering, and
  /// whatever the test is about handed in last.
  ///
  /// The settings among them because the Create card holds its press until they
  /// answer: what the tick would ask of GitHub is nothing this describe is
  /// about, so there is no token and the card says so.
  const theirs = (...answers: Parameters<typeof serving>) =>
    theCluster(
      whenever("/api/ui/settings", json(UNTOKENED)),
      whenever(listingAt(null, MEMBER.device), json(OVER_THERE)),
      whenever(listingAt("/mnt/code", MEMBER.device), json(OVER_THERE)),
      whenever(listingAt("/mnt", MEMBER.device), json(ABOVE_THEIRS)),
      whenever(listingAt(null), json(HERE)),
      whenever(listingAt("/home/ada/src"), json(HERE)),
      whenever(listingAt("/home/ada"), json(ABOVE_HERE)),
      ...answers,
    );

  /// Open the Create repo card off the dropdown's foot and wait for it to have
  /// settled what it says about GitHub, the way the human filling it in does:
  /// it takes no create until the settings have answered.
  async function createCard(registry: RepoEntry[]): Promise<void> {
    await waitFor(() => expect(offered("Repo")).toHaveLength(registry.length));
    press("Repo", "Create repo");

    await waitFor(() => expect(screen.getByLabelText(WHERE)).toBeTruthy());
    await waitFor(() => expect(noRemote()).toBeTruthy());
  }

  /// And the Open repo card beside it, which asks the settings nothing and so
  /// has nothing to wait for but itself.
  async function openCard(registry: RepoEntry[]): Promise<void> {
    await waitFor(() => expect(offered("Repo")).toHaveLength(registry.length));
    press("Repo", "Open repo");

    await waitFor(() => expect(screen.getByLabelText(PATH)).toBeTruthy());
  }

  /// Type a path into that one and send it.
  function register(path: string): void {
    fireEvent.input(screen.getByLabelText(PATH), { target: { value: path } });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
  }

  /// **Open repo**, end to end on the member: the browse is its filesystem, the
  /// registration lands on its registry, and the dropdown behind the card is
  /// that registry read again with what just arrived on it.
  it("browses the member's directories and lands a repo on its registry", async () => {
    // The registry as a registry behaves: what has been registered, which is one
    // more thing the moment the press lands.
    let landed = false;

    const fetching = theirs(
      whenever(at("/repos"), () =>
        json(landed ? [...THEIRS, THEIR_OPENED] : THEIRS)(),
      ),
      whenever(
        at("/repos"),
        () => {
          landed = true;
          return json({ Added: THEIR_OPENED })();
        },
        "POST",
      ),
      whenever(at(`/repos/${THEIR_OPENED.id}/pairings`), json(NO_PAIRINGS)),
      whenever(at(`/repos/${THEIR_OPENED.id}/branches`), json(BRANCHES)),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await openCard(THEIRS);

    // The field standing empty asks for the server's own home, and through the
    // Relay that is the member's home: what comes down is the other machine's
    // directories rather than this one's.
    browse(PATH);
    await waitFor(() =>
      expect(browsed(PATH)).toEqual(["Up to /mnt", "projects", "widgets"]),
    );
    expect(askedFor(fetching, listingAt(null, MEMBER.device))).toBe(1);
    expect(askedFor(fetching, listingAt(null))).toBe(0);

    register(THEIR_OPENED.path);

    // The registration went to the member's registry and to no other, and the
    // draft is on the Repo that came back.
    await waitFor(() => expect(stored().repo).toBe(THEIR_OPENED.id));
    expect(sent(fetching, at("/repos"))).toEqual({ path: THEIR_OPENED.path });
    expect(writes(fetching, "/api/ui/repos")).toBe(0);

    // The card is spent and the row it was opened from has become the panel —
    // whose list is the member's registry read again, now holding what landed,
    // and settled on it.
    await waitFor(() => expect(screen.queryByLabelText(PATH)).toBeNull());
    await openRepo(container);
    await waitFor(() => expect(rows("Repo")).toContain(THEIR_OPENED.name));
    expect(showing("Repo")).toBe(THEIR_OPENED.name);
  });

  /// **Create repo** on the member, and the one thing that was genuinely wrong:
  /// where the last repo went is a fact about the machine it went on, so there is
  /// an answer per device and the browse opens in the picked one's.
  it("makes a repo on the member, and opens where the last one there went", async () => {
    // Two memories, one per machine: this browser has been making repositories
    // in this device's own ~/src since before there was a cluster, and the
    // member's code lives somewhere else entirely.
    setRepoParent(null, "/home/ada/src");
    setRepoParent(MEMBER.device, "/mnt/code");

    const fetching = theirs(
      whenever(at("/repos/new"), json({ Made: THEIR_MADE }), "POST"),
      whenever(at(`/repos/${THEIR_MADE.id}/pairings`), json(NO_PAIRINGS)),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await createCard(THEIRS);

    // The member's parent rather than this device's — and, because that is a
    // path handed over rather than one being typed, the browse opens inside it.
    expect(held(WHERE)).toBe("/mnt/code");

    browse(WHERE);
    await waitFor(() =>
      expect(askedFor(fetching, listingAt("/mnt/code", MEMBER.device))).toBe(1),
    );
    expect(askedFor(fetching, listingAt("/home/ada/src"))).toBe(0);

    make("/mnt/code", "widgets");

    // The directory and the repository were made over there, and the draft is on
    // what came back.
    await waitFor(() => expect(stored().repo).toBe(THEIR_MADE.id));
    expect(sent(fetching, at("/repos/new"))).toEqual({
      parent: "/mnt/code",
      name: "widgets",
      github: false,
    });
    expect(writes(fetching, "/api/ui/repos/new")).toBe(0);

    // And what it came home with is remembered against the member, this device's
    // own answer left exactly where it was.
    expect(repoParent(MEMBER.device)).toBe("/mnt/code");
    expect(repoParent(null)).toBe("/home/ada/src");
  });

  /// And a device nothing has been made on yet starts where a first run starts,
  /// which is the case one key got wrong: a path on this machine, offered as
  /// somewhere to put a repository over there.
  it("starts at the member's own home where nothing has been made on it", async () => {
    setRepoParent(null, "/home/ada/src");

    const fetching = theirs();
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await createCard(THEIRS);

    expect(held(WHERE)).toBe("");

    browse(WHERE);
    await waitFor(() =>
      expect(browsed(WHERE)).toEqual(["Up to /mnt", "projects", "widgets"]),
    );
    expect(askedFor(fetching, listingAt(null, MEMBER.device))).toBe(1);
    expect(askedFor(fetching, listingAt("/home/ada/src"))).toBe(0);
  });

  /// A refusal from over there is the far end's own outcome, said in the words
  /// this app has for it wherever it is met: the hop is not something the human
  /// is told about, and nothing was added to the four sentences.
  it("says a create's refusal in its own words, not as a failed call", async () => {
    theirs(
      whenever(
        at("/repos/new"),
        json("AlreadyThere" satisfies Created),
        "POST",
      ),
    );
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await createCard(THEIRS);
    make("/mnt/code", "widgets");

    await waitFor(() => screen.getByText(CREATE_REFUSAL.AlreadyThere));

    // And the card stays up holding what was typed, because what answers a
    // refusal is correcting it.
    expect(held(WHERE)).toBe("/mnt/code");
    expect(
      (screen.getByLabelText(CALLED) as HTMLInputElement).value,
    ).toBe("widgets");
    expect(stored().repo).toBeNull();
    // And nothing is remembered: there is no repository over there to put the
    // next one beside.
    expect(repoParent(MEMBER.device)).toBe("");
  });

  /// The other row refuses the same way, which is what says this is the far
  /// end's answer reaching the card rather than the Relay's account of a call.
  it("says an open's refusal in its own words too", async () => {
    theirs(whenever(at("/repos"), json("NotARepository"), "POST"));
    const { container } = mount("/compose");

    await draftingOnTheMember(container);
    await openCard(THEIRS);
    register("/mnt/code/notes");

    await waitFor(() => screen.getByText(REPO_REFUSAL.NotARepository));
    expect(held(PATH)).toBe("/mnt/code/notes");
    expect(stored().repo).toBeNull();
  });

  /// And with this device picked — which is what an untouched browser is on,
  /// cluster or no cluster — both rows are the rows they were before there was a
  /// select at all: this device's home, this device's registry, this device's
  /// memory of where the last one went.
  it("leaves both rows on this device where nothing is picked", async () => {
    setRepoParent(null, "/home/ada/src");
    setRepoParent(MEMBER.device, "/mnt/code");

    const fetching = theirs(
      whenever("/api/ui/repos/new", json({ Made: MADE_HERE }), "POST"),
      whenever(`/api/ui/repos/${MADE_HERE.id}/pairings`, json(NO_PAIRINGS)),
    );
    const { container } = mount("/compose");

    await composing(container);
    // Drawn, and left alone.
    await drawn(container, `.${setup.deviceSelect}`);
    await waitFor(() => expect(showing("Device")).toBe(LINKED.this.name));

    await createCard(REPOS);

    expect(held(WHERE)).toBe("/home/ada/src");

    browse(WHERE);
    await waitFor(() =>
      expect(askedFor(fetching, listingAt("/home/ada/src"))).toBe(1),
    );
    expect(
      askedFor(fetching, listingAt("/home/ada/src", MEMBER.device)),
    ).toBe(0);

    make("/home/ada/src", "widgets");

    await waitFor(() => expect(stored().repo).toBe(MADE_HERE.id));
    expect(sent(fetching, "/api/ui/repos/new")).toEqual({
      parent: "/home/ada/src",
      name: "widgets",
      github: false,
    });
    expect(writes(fetching, at("/repos/new"))).toBe(0);

    // And the member's own memory of where its code goes is untouched by a
    // repository made here.
    expect(repoParent(null)).toBe("/home/ada/src");
    expect(repoParent(MEMBER.device)).toBe("/mnt/code");
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
    leaveRefusals(null, 0, []);
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
  /// nothing to do is not drawn. The drop goes with it: the Attach menu and the
  /// box are two ways into the one piece, and neither is offered here.
  it("offers no attach control and takes no drop while a roadmap is loaded", async () => {
    adopting();
    const { container } = mount("/compose");

    await composing(container);
    expect(screen.getByRole("button", { name: "Attach" })).toBeTruthy();

    await loadRoadmap(container, 0);

    expect(screen.queryByRole("button", { name: "Attach" })).toBeNull();

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
    leaveRefusals(null, 0, []);
  });

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
    const clip = screen.getByRole("button", { name: "Attach" });
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

/// The MCP servers handed over with what is being composed: picked out of the
/// same Attach menu the draft's composer grew, held in the page beside the
/// files until a press, and put on the Conversation by name once the press has
/// made one.
///
/// The menu itself is `Composer.tsx`'s and is asked about in
/// `attaching-servers.test.tsx`. What is asked here is this page's own half:
/// that it draws that menu too, that a pick is a chip and nothing else while
/// there is nothing to attach it to, and that the replay carries what it is
/// holding.
describe("the MCP servers a compose page holds", () => {
  beforeEach(() => {
    localStorage.clear();
    leaveRefusals(null, 0, []);
  });

  /// Where one goes on, the name in the path exactly as the composer sends it.
  const attach = (name: string) =>
    `/api/ui/conversations/${OPEN.id}/mcp-servers/${name}`;

  /// The settings with another list of declarations in them — the fixture's
  /// own two otherwise.
  const declaring = (...names: string[]): SettingsView => ({
    ...SETTINGS,
    mcp_servers: names.map((name) => ({
      name,
      url: `https://mcp.example.com/${name}`,
      headers: [],
    })),
  });

  /// Pick one out of the menu, which is what a press on its row does.
  async function pickServer(name: string): Promise<void> {
    fireEvent.click(
      within(await attachMenu()).getByRole("menuitem", { name }),
    );
  }

  /// The same menu the composer's Attach opens, drawn from the same
  /// declarations: *Attach file* first, and the servers under it.
  it("opens the menu the composer's Attach opens", async () => {
    creating();
    const { container } = mount("/compose");

    await composing(container);

    expect(offeredIn(await attachMenu())).toEqual([
      "Attach file",
      ...SETTINGS.mcp_servers.map((one) => one.name),
    ]);
  });

  /// And with nothing declared it is still a menu, with the way to the section
  /// where a declaration is made — this page falling back to a plain button
  /// would be the control the human had to learn twice, drawn twice.
  it("holds Attach file and the way to the settings where none is declared", async () => {
    creating(whenever("/api/ui/settings", json(declaring())));
    const { container } = mount("/compose");

    await composing(container);

    expect(offeredIn(await attachMenu())).toEqual([
      "Attach file",
      "Declare an MCP server…",
    ]);
    expect(
      screen
        .getByRole("menuitem", { name: "Declare an MCP server…" })
        .getAttribute("href"),
    ).toBe("/settings/mcp-servers");
  });

  /// A pick draws its chip at once and nothing reaches the server: there is no
  /// Conversation to attach anything to, and the press is still the first thing
  /// that goes on the wire.
  it("draws a chip per picked server, and sends nothing until a press", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    await pickServer("docs");

    await waitFor(() => expect(chips(container)).toEqual(["docs"]));
    expect(writes(fetching, "/api/ui/conversations")).toBe(0);
    expect(writes(fetching, attach("docs"))).toBe(0);

    // And the menu is gone with the press, as it is on the composer: the row
    // that was pressed is about to go.
    expect(screen.queryByRole("menu", { name: "Attach" })).toBeNull();
  });

  /// Taken out of the menu by the pick, and put back by the ×, which is the
  /// whole of what the two controls are to each other.
  it("takes the row out of the menu, and the × puts it back", async () => {
    creating();
    const { container } = mount("/compose");

    await composing(container);
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));

    expect(offeredIn(await attachMenu())).toEqual(["Attach file", "tickets"]);
    shutMenu();

    fireEvent.click(screen.getByRole("button", { name: "Remove docs" }));

    await waitFor(() => expect(chips(container)).toEqual([]));
    expect(offeredIn(await attachMenu())).toEqual([
      "Attach file",
      ...SETTINGS.mcp_servers.map((one) => one.name),
    ]);
  });

  /// Nothing at all is asked of the server on the way to either: a chip is
  /// drawn and taken away in the page, there being nothing on the server for
  /// them to be about.
  it("asks nothing of the server for a pick or a ×", async () => {
    const fetching = creating();
    const { container } = mount("/compose");

    await composing(container);
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));
    fireEvent.click(screen.getByRole("button", { name: "Remove docs" }));
    await waitFor(() => expect(chips(container)).toEqual([]));

    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
    ).toHaveLength(0);
  });

  /// The press is what attaches them: one request per name, through the route
  /// the composer's own menu attaches through.
  it("attaches what it holds when the draft is saved, and lands in the draft", async () => {
    const fetching = creating(
      whenever(attach("docs"), json("Attached"), "POST"),
      whenever(attach("tickets"), json("Attached"), "POST"),
    );
    const { container, history } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await pickServer("docs");
    await pickServer("tickets");
    await waitFor(() => expect(chips(container)).toEqual(["docs", "tickets"]));

    fireEvent.click(screen.getByRole("button", { name: "Save as draft" }));

    await waitFor(() => expect(writes(fetching, attach("docs"))).toBe(1));
    expect(writes(fetching, attach("tickets"))).toBe(1);

    await waitFor(() =>
      expect(history.get().startsWith(`/conversations/${OPEN.id}`)).toBe(true),
    );
  });

  /// And every one of them on before the work starts, for the reason the files
  /// go up before it: the servers freeze when the Brief does, and one arriving
  /// after the grilling started would be refused for being late.
  it("finishes attaching before it starts the work", async () => {
    const fetching = creating(
      whenever(attach("docs"), json("Attached"), "POST"),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(1),
    );
    expect(order(fetching, attach("docs"))).toBeGreaterThan(-1);
    expect(order(fetching, attach("docs"))).toBeLessThan(
      order(fetching, `/api/ui/conversations/${OPEN.id}/grill`),
    );
  });

  /// Whichever Process the page is composing under: a server is the
  /// Conversation's whatever its sessions are sent to do, so nothing about the
  /// picker decides whether one goes on — and the kickoff it goes in front of
  /// is whichever kickoff that Process has.
  it("attaches them under a process whose kickoff is a take-up", async () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({
        ...blank(),
        repo: REPOS[1]!.id,
        brief: "The limiter will not merge.",
        process: "FixMergeIssues" satisfies Process,
        target: "#41",
      }),
    );

    const fetching = creating(
      whenever(attach("docs"), json("Attached"), "POST"),
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
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));

    // Waited on the press rather than on the Agent control: this Process uses
    // one role, which stands in the row as the picker itself with no panel
    // anywhere over it — see the agent dropdown's own tests.
    const start = screen.getByRole("button", { name: "Start work" });
    await waitFor(() =>
      expect(start.getAttribute("aria-disabled")).toBe("false"),
    );

    fireEvent.click(start);

    await waitFor(() =>
      expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/take-up`)).toBe(
        1,
      ),
    );
    expect(order(fetching, attach("docs"))).toBeGreaterThan(-1);
    expect(order(fetching, attach("docs"))).toBeLessThan(
      order(fetching, `/api/ui/conversations/${OPEN.id}/take-up`),
    );
  });

  /// A name is text a device could write down, and this one is not written
  /// down: what it refers to is a declaration on the settings page, and one
  /// picked out of a menu a moment ago is one the settings still hold. So the
  /// chips go the way the pills go, and the brief stays.
  it("keeps the brief through a reload and holds no server", async () => {
    creating();
    const first = mount("/compose");

    fireEvent.input(await composing(first.container), {
      target: { value: "Make the widget" },
    });
    await pickServer("docs");
    await waitFor(() => expect(chips(first.container)).toEqual(["docs"]));

    await waitFor(() => expect(localStorage.getItem(COMPOSING)).toBeTruthy());
    first.unmount();

    const again = mount("/compose");
    const box = await composing(again.container);

    await waitFor(() => expect(box.value).toBe("Make the widget"));
    expect(chips(again.container)).toEqual([]);
  });

  /// And a page left without a press has attached nothing to anything: what was
  /// picked never reached the server, and there is no Conversation it could
  /// have reached it about.
  it("attaches nothing where the page is left without a press", async () => {
    const fetching = creating();
    const { container, unmount } = mount("/compose");

    await composing(container);
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));

    unmount();

    expect(
      fetching.mock.calls.filter(([, init]) => init?.method === "POST"),
    ).toHaveLength(0);
  });

  /// A name the server will not take is one more refusal for the draft the page
  /// lands on, worded where the rest of a replay's refusals are — and it stops
  /// the kickoff, exactly as a refused file does.
  it("says on the new draft what could not be attached", async () => {
    const fetching = creating(
      whenever(attach("docs"), json("NoSuchServer"), "POST"),
    );
    const { container } = mount("/compose");

    fireEvent.input(await composing(container), {
      target: { value: "Make the widget" },
    });
    await pickRepo(container, REPOS[1]!.id);
    await rolesAnswered();
    await pickServer("docs");
    await waitFor(() => expect(chips(container)).toEqual(["docs"]));

    fireEvent.click(screen.getByRole("button", { name: "Start work" }));

    await waitFor(() =>
      expect(
        screen.getByText(
          `docs could not be attached: ${SERVER_REFUSAL.NoSuchServer}`,
        ),
      ).toBeTruthy(),
    );

    expect(writes(fetching, `/api/ui/conversations/${OPEN.id}/grill`)).toBe(0);
  });
});

/// And the state away from the page it is composed on: what survives a round
/// trip through this device, and what is discarded rather than half-applied.
describe("what a device holds between visits", () => {
  beforeEach(() => localStorage.clear());

  it("comes back as it was left", () => {
    const held: Composed = {
      device: null,
      repo: 2,
      brief: "Make the widget",
      branch: "widget-work",
      target: "#41",
      filled: "#41",
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

  /// The device is held with the ids that are only ids on it — the repo, the
  /// companions and the pairings — so a draft read back knows which machine its
  /// repository is a number on.
  it("comes back knowing which device it was being composed for", () => {
    keep({ ...blank(), device: MEMBER.device, repo: 2 });

    expect(stored().device).toBe(MEMBER.device);
  });

  /// A body from a build before the compose page could draft anywhere else names
  /// no device at all, which is not a fault: it reads as whatever this browser is
  /// drafting onto, the two having been written together by the build that wrote
  /// them.
  it("reads a body that names no device as the one this browser is drafting onto", () => {
    setDraftingOn(MEMBER.device);
    const { device: _, ...before } = { ...blank(), repo: 2 };
    localStorage.setItem(COMPOSING, JSON.stringify(before));

    expect(stored().device).toBe(MEMBER.device);
    expect(stored().repo).toBe(2);
  });

  /// Where it does say, what it says is the answer. A draft written for this
  /// device reads as this device however the browser has drafted since, that
  /// draft's Repo id being a number here.
  it("reads a body that says this device as this device", () => {
    keep({ ...blank(), repo: 2 });
    setDraftingOn(MEMBER.device);

    expect(stored().device).toBeNull();
  });

  it("discards a draft whose device is not a device", () => {
    localStorage.setItem(
      COMPOSING,
      JSON.stringify({ ...blank(), device: 7, repo: 2 }),
    );

    expect(stored()).toEqual(blank());
  });

  /// A pick and nothing else is a page holding nothing: the pick is the
  /// browser's and is kept there, so the next visit comes back to that machine
  /// with no draft under it.
  it("holds nothing for a page that has picked a device and nothing else", () => {
    setDraftingOn(MEMBER.device);

    keep(blank());

    expect(localStorage.getItem(COMPOSING)).toBeNull();
    expect(stored().device).toBe(MEMBER.device);
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
