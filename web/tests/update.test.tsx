//! The Update Notice: the banner the settings page draws when the server says a
//! newer Verkstead has been released than the one serving it.
//!
//! Mounted through the settings page rather than on its own, because where it
//! sits is half of what it is: a banner above everything that page configures,
//! in the reading column that page is in. What the server answers with is
//! `UpdateNotice` from `src/api/types.ts`, which `cargo test` writes out of the
//! Rust the endpoint fills in — so a payload written here that the server would
//! never send does not typecheck.

import { screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  ConversationEntry,
  ProfileEntry,
  RepoEntry,
  SettingsView,
  ShowingArchived,
  UpdateNotice,
} from "../src/api/types";
import shell from "../src/Panes.module.css";
import profileList from "../src/profiles/ProfileList.module.css";
import notifications from "../src/push/Notifications.module.css";
import repoList from "../src/repos/RepoList.module.css";
import credentials from "../src/settings/Credentials.module.css";
import { SettingsPage } from "../src/settings/SettingsPage";
import page from "../src/settings/SettingsPage.module.css";
import head from "../src/workbench/PaneHead.module.css";
import styles from "../src/update/UpdateNotice.module.css";
import { mount } from "./listing";
import { json, serving, whenever } from "./serving";
import profiles from "./fixtures/profiles.json" with { type: "json" };
import conversations from "./fixtures/conversations.json" with { type: "json" };
import repos from "./fixtures/repos.json" with { type: "json" };
import settings from "./fixtures/settings.json" with { type: "json" };

const REPOS = repos as RepoEntry[];

/// Where the README says how to update. Stage 06 of the public-release roadmap
/// writes that section; the anchor is the one agreed with it.
const UPDATING = "https://github.com/tobico/verkstead#updating";

/// What the server says about updating, whenever the page asks.
const saying = (notice: UpdateNotice) =>
  whenever("/api/ui/update", json(notice));

/// Everything under the banner, which every test here serves the same way: the
/// banner is what is under test, and everything it sits above is only what it
/// sits above.
const beneath = () => [
  whenever("/api/ui/settings", json(settings as SettingsView)),
  whenever("/api/ui/profiles", json(profiles as ProfileEntry[])),
  whenever("/api/ui/repos", json(REPOS)),
  // The conversations pane rides along on this page, and reads its own three.
  whenever("/api/ui/conversations", json(conversations as ConversationEntry[])),
  whenever("/api/ui/conversations/archived", json({ showing: false } as ShowingArchived)),
  whenever("/api/ui/abandoned-roadmaps", json([])),
];

const banner = () => document.querySelector(`.${styles.notice}`);

/// The page having drawn what it configures, which is what every test here
/// waits for before reading the banner. Waited for on the settings pane's own
/// list rather than by a repo's name: the conversations pane beside it names
/// the repo every row is in, so a name is no longer one element on this page.
const loaded = () =>
  waitFor(() =>
    expect(
      document.querySelector(`.${repoList.repos} .${repoList.row}`),
    ).not.toBeNull(),
  );

/// The settings pane, which is the middle one of the three the page stands on:
/// the banner and everything under it are in there, and the conversations pane
/// beside it is the workbench's own.
const settingsPane = (container: ParentNode): HTMLElement =>
  container.querySelectorAll<HTMLElement>(`.${shell.panes} > section`)[1]!;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("the Update Notice", () => {
  it("asks the server whether there is a newer Verkstead", async () => {
    const fetching = serving(...beneath(), saying("Current"));
    mount(SettingsPage);

    await loaded();
    expect(fetching).toHaveBeenCalledWith("/api/ui/update", expect.anything());
  });

  it("names the release when the server says there is one", async () => {
    serving(...beneath(), saying({ Available: { version: "0.4.0" } }));
    mount(SettingsPage);

    await waitFor(() => expect(banner()).not.toBeNull());
    expect(banner()!.textContent).toContain("0.4.0");
  });

  it("links the updating instructions, and offers nothing that installs", async () => {
    serving(...beneath(), saying({ Available: { version: "0.4.0" } }));
    const { container } = mount(SettingsPage);

    await waitFor(() => expect(banner()).not.toBeNull());

    // The link is the whole of what the Notice offers: nothing here changes the
    // server, and nothing installs on the human's behalf.
    const links = [...banner()!.querySelectorAll("a")];
    expect(links.map((link) => link.getAttribute("href"))).toEqual([UPDATING]);
    expect(banner()!.querySelectorAll("button, input, form")).toHaveLength(0);
    // And it added nothing outside itself either: what the settings pane can be
    // pressed and typed into is what it configures — the credentials card, the
    // profiles, the repos and the notifications switch — all of which were there
    // before the banner and are not the banner's. The conversations pane beside
    // it has plenty to press and none of it is this page's.
    const pressable = [...settingsPane(container).querySelectorAll("button, input, form")];
    expect(
      pressable.filter(
        (found) =>
          !found.closest(`.${head.head}`) &&
          !found.closest(`.${profileList.profiles}`) &&
          !found.closest(`.${repoList.repos}`) &&
          !found.closest(`.${notifications.notifications}`),
      ),
    ).toHaveLength(0);
  });

  it("stands above the page, in the column the page is read in", async () => {
    serving(...beneath(), saying({ Available: { version: "0.4.0" } }));
    const { container } = mount(SettingsPage);

    await waitFor(() => expect(banner()).not.toBeNull());

    const column = container.querySelector(`.${page.settings}`)!;
    const configured = column.querySelector(`.${credentials.githubCard}`)!;
    // Inside the page's column rather than beside it, and before everything the
    // page is for.
    expect(banner()!.parentElement).toBe(column);
    expect(
      banner()!.compareDocumentPosition(configured) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("draws nothing when the server says there is nothing to update to", async () => {
    serving(...beneath(), saying("Current"));
    mount(SettingsPage);

    await loaded();
    expect(banner()).toBeNull();
  });

  it("draws nothing when the server could not be asked", async () => {
    serving(
      ...beneath(),
      whenever(
        "/api/ui/update",
        json({ error: "the update check could not be read" }, 500),
      ),
    );
    mount(SettingsPage);

    // The page is exactly as it was: a page that cannot reach the endpoint says
    // nothing about updating, and nothing about the failure either.
    await loaded();
    expect(banner()).toBeNull();
    expect(screen.queryByText(/update/i)).toBeNull();
  });

  it("draws nothing while the answer is still in flight", async () => {
    let deliver: () => void;
    const held = new Promise<void>((resolve) => {
      deliver = resolve;
    });

    serving(
      ...beneath(),
      whenever("/api/ui/update", () =>
        held.then(
          () =>
            new Response(
              JSON.stringify({ Available: { version: "0.4.0" } }),
            ),
        ),
      ),
    );
    mount(SettingsPage);

    await loaded();
    expect(banner()).toBeNull();

    deliver!();
    await waitFor(() => expect(banner()).not.toBeNull());
  });

  it("asks at its own cadence rather than the page's", async () => {
    const fetching = serving(...beneath(), saying("Current"));
    mount(SettingsPage);
    await loaded();

    const asking = () =>
      fetching.mock.calls.filter(([path]) => path === "/api/ui/update").length;
    expect(asking()).toBe(1);

    // The page has refetched several times over by now; a release cannot arrive
    // in the ten seconds a Set can, and the server is answering out of a memory
    // it refreshes daily.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(asking()).toBe(1);
  });
});
