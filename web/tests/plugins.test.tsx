//! What the Plugins section says (ADR-0030): a row per plugin with its name,
//! version, author, what it provides and where it came from; its trouble where
//! it didn't load; a local plugin on another device greyed and naming it; the
//! checkbox; and **Add a plugin…** reading the consent card before **Install**
//! grants what it showed.
//!
//! The reads are fixtures the server's own suite wrote off the real endpoints —
//! see `crates/server/tests/suite/plugins.rs`.

import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { describe, expect, it, vi } from "vitest";

import type { PluginConsent, PluginsView } from "../src/api/types";
import { PluginsCard, PluginsPane } from "../src/settings/Plugins";
import consent from "./fixtures/plugin-consent.json" with { type: "json" };
import plugins from "./fixtures/plugins.json" with { type: "json" };
import { json, serving, whenever } from "./serving";

/// One plugin that loaded, GitLab, one whose manifest broke, Broken, one from
/// git, Shared skills, and one whose dependency, Base, isn't installed.
const PLUGINS = plugins as PluginsView;
const CONSENT = consent as PluginConsent;

/// The page with GitLab a local plugin on another device.
const ELSEWHERE: PluginsView = {
  ...PLUGINS,
  installed: [{ ...PLUGINS.installed[0]!, elsewhere: "wolf" }],
};

function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

const pane = () => mounting(() => <PluginsPane back={vi.fn()} />);

/// The row drawn for `name`.
async function theRow(name: string): Promise<HTMLElement> {
  return waitFor(() => screen.getByText(name).closest("li")!);
}

describe("the card", () => {
  it("says how many are in trouble, as a warning", async () => {
    serving(whenever("/api/ui/plugins", json(PLUGINS)));

    mounting(() => <PluginsCard open={false} press={vi.fn()} />);

    await waitFor(() => screen.getByText("2 in trouble"));
  });

  it("says none are installed on an empty page", async () => {
    serving(
      whenever("/api/ui/plugins", json({ installed: [], built_in: [] })),
    );

    mounting(() => <PluginsCard open={false} press={vi.fn()} />);

    await waitFor(() => screen.getByText("None installed"));
  });
});

describe("a row", () => {
  it("reads the name, version, author, what it provides and its source", async () => {
    serving(whenever("/api/ui/plugins", json(PLUGINS)));
    pane();

    const row = await theRow("GitLab");

    expect(row.textContent).toContain("1.4.0 · Ada Lovelace");
    expect(row.textContent).toContain("Provides forge, skills.");
    expect(row.textContent).toContain("/home/you/src/gitlab");
    expect(row.querySelector("input[type=checkbox]")).toHaveProperty(
      "checked",
      true,
    );
  });

  it("reads a git plugin's repository and pinned revision, with Update", async () => {
    serving(whenever("/api/ui/plugins", json(PLUGINS)));
    pane();

    const row = await theRow("Shared skills");

    expect(row.textContent).toContain(
      "file:///home/you/src/sources/skills at 133d813",
    );
    expect(row.textContent).toContain("Update");
  });

  it("draws its trouble where it didn't load", async () => {
    serving(whenever("/api/ui/plugins", json(PLUGINS)));
    pane();

    const row = await theRow("Broken");

    expect(row.textContent).toContain("`version` isn't a semantic version.");
  });

  it("draws a local plugin on another device greyed, naming it, with no Update", async () => {
    serving(whenever("/api/ui/plugins", json(ELSEWHERE)));
    pane();

    const row = await theRow("GitLab");

    expect(row.textContent).toContain("On wolf.");
    expect(
      [...row.querySelectorAll("button")].map((button) => button.textContent),
    ).toEqual(["Remove"]);
  });

  it("is disabled by unticking it", async () => {
    const unticked: PluginsView = {
      ...PLUGINS,
      installed: PLUGINS.installed.map((row) =>
        row.name === "GitLab" ? { ...row, enabled: false } : row,
      ),
    };
    const fetching = serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever(
        `/api/ui/plugins/${PLUGINS.installed[0]!.uuid}/enabled`,
        json(unticked),
        "POST",
      ),
    );
    pane();

    const row = await theRow("GitLab");
    fireEvent.click(row.querySelector("input[type=checkbox]")!);

    await waitFor(() =>
      expect(row.querySelector("input[type=checkbox]")).toHaveProperty(
        "checked",
        false,
      ),
    );
    const press = fetching.mock.calls.find(([asked]) =>
      String(asked).endsWith("/enabled"),
    );
    expect(JSON.parse(String(press![1]!.body))).toEqual({ enabled: false });
  });
});

describe("a dependency that isn't there", () => {
  const DEPENDENT = PLUGINS.installed.find((row) => row.name === "Dependent")!;

  it("is the dependent's trouble, naming it", async () => {
    serving(whenever("/api/ui/plugins", json(PLUGINS)));
    pane();

    const row = await theRow("Dependent");

    expect(row.textContent).toContain("It needs Base, which isn't installed.");
  });

  it("installs from its repository through the consent card", async () => {
    const fetching = serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever("/api/ui/plugins/read", json(CONSENT), "POST"),
      whenever("/api/ui/plugins", json(PLUGINS), "POST"),
    );
    pane();

    const row = await theRow("Dependent");
    fireEvent.click(
      await waitFor(() => screen.getByRole("button", { name: "Install Base" })),
    );

    await waitFor(() => screen.getByText("Dependent needs it."));
    expect(row.textContent).toContain("Card 1.4.0");
    const read = fetching.mock.calls.find(([asked]) =>
      String(asked).endsWith("/read"),
    );
    expect(JSON.parse(String(read![1]!.body))).toEqual({
      source: "https://example.com/base.git",
    });

    fireEvent.click(screen.getByRole("button", { name: "Install" }));

    await waitFor(() => expect(screen.queryByText("Card 1.4.0")).toBeNull());
    const press = fetching.mock.calls.find(
      ([asked, init]) =>
        String(asked) === "/api/ui/plugins" && init?.method === "POST",
    );
    expect(JSON.parse(String(press![1]!.body))).toEqual({
      source: "https://example.com/base.git",
      granted: CONSENT.capabilities,
    });
  });

  it("offers the update where it is outdated, pressing it on the dependency", async () => {
    const outdated: PluginsView = {
      ...PLUGINS,
      installed: PLUGINS.installed.map((row) =>
        row.uuid === DEPENDENT.uuid
          ? {
              ...row,
              trouble: "It needs GitLab ^2, and 1.4.0 is installed.",
              needs: [
                {
                  uuid: PLUGINS.installed[0]!.uuid,
                  name: "GitLab",
                  fix: { kind: "Update" },
                },
              ],
            }
          : row,
      ),
    };
    const fetching = serving(
      whenever("/api/ui/plugins", json(outdated)),
      whenever(
        `/api/ui/plugins/${PLUGINS.installed[0]!.uuid}/update`,
        json({ kind: "Updated", plugins: PLUGINS }),
        "POST",
      ),
    );
    pane();

    await theRow("Dependent");
    fireEvent.click(
      await waitFor(() =>
        screen.getByRole("button", { name: "Update GitLab" }),
      ),
    );

    await waitFor(() =>
      expect(
        fetching.mock.calls.some(([asked]) =>
          String(asked).endsWith("/update"),
        ),
      ).toBe(true),
    );
  });
});

describe("Remove and the checkbox over dependents", () => {
  const GITLAB = PLUGINS.installed[0]!;

  /// The page with GitLab depended on by two ticked plugins.
  const DEPENDED: PluginsView = {
    ...PLUGINS,
    installed: PLUGINS.installed.map((row) =>
      row.uuid === GITLAB.uuid ? { ...row, dependents: ["One", "Top"] } : row,
    ),
  };

  /// The page with a built-in listed, which Verkstead ships rather than the
  /// human installing.
  const BUILT_IN: PluginsView = {
    ...PLUGINS,
    built_in: [
      {
        ...GITLAB,
        uuid: "613e2a43-80ee-4307-8623-28969e05330f",
        name: "Fixture",
      },
    ],
  };

  it("draws no Remove on a built-in row", async () => {
    serving(whenever("/api/ui/plugins", json(BUILT_IN)));
    pane();

    const row = await theRow("Fixture");

    expect(row.textContent).not.toContain("Remove");
    expect((await theRow("GitLab")).textContent).toContain("Remove");
  });

  it("removes a row once its confirmation is pressed", async () => {
    const fetching = serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever(
        `/api/ui/plugins/${GITLAB.uuid}/remove`,
        json({ ...PLUGINS, installed: PLUGINS.installed.slice(1) }),
        "POST",
      ),
    );
    pane();

    const row = await theRow("GitLab");
    fireEvent.click(within(row).getByRole("button", { name: "Remove" }));

    const asking = await waitFor(() =>
      screen.getByRole("group", { name: "Remove GitLab?" }),
    );
    expect(asking.textContent).toContain(
      "It's taken off every device. Its directory stays.",
    );
    expect(fetching.mock.calls.some(([asked]) => String(asked).endsWith("/remove"))).toBe(false);

    fireEvent.click(within(asking).getByRole("button", { name: "Remove" }));

    await waitFor(() => expect(screen.queryByText("GitLab")).toBeNull());
  });

  it("draws the refusal on the row where plugins depend on it", async () => {
    const why =
      "GitLab can't be removed while 2 plugins depend on it: One and Top. Remove them first.";
    serving(
      whenever("/api/ui/plugins", json(DEPENDED)),
      whenever(
        `/api/ui/plugins/${GITLAB.uuid}/remove`,
        json({ error: why }, 409),
        "POST",
      ),
    );
    pane();

    const row = await theRow("GitLab");
    fireEvent.click(within(row).getByRole("button", { name: "Remove" }));
    fireEvent.click(
      within(
        await waitFor(() => screen.getByRole("group", { name: "Remove GitLab?" })),
      ).getByRole("button", { name: "Remove" }),
    );

    await waitFor(() => expect(row.textContent).toContain(why));
  });

  it("names the dependents before unticking, and unticks on Disable", async () => {
    const fetching = serving(
      whenever("/api/ui/plugins", json(DEPENDED)),
      whenever(
        `/api/ui/plugins/${GITLAB.uuid}/enabled`,
        json(PLUGINS),
        "POST",
      ),
    );
    pane();

    const row = await theRow("GitLab");
    fireEvent.click(row.querySelector("input[type=checkbox]")!);

    const asking = await waitFor(() =>
      screen.getByRole("group", { name: "Disable GitLab?" }),
    );
    expect(asking.textContent).toContain(
      "One and Top depend on it, and are disabled with it.",
    );
    const pressed = () =>
      fetching.mock.calls.some(([asked]) => String(asked).endsWith("/enabled"));
    expect(pressed()).toBe(false);

    fireEvent.click(within(asking).getByRole("button", { name: "Disable" }));

    await waitFor(() => expect(pressed()).toBe(true));
    const press = fetching.mock.calls.find(([asked]) =>
      String(asked).endsWith("/enabled"),
    );
    expect(JSON.parse(String(press![1]!.body))).toEqual({ enabled: false });
  });

  it("leaves it ticked when the untick is cancelled", async () => {
    serving(whenever("/api/ui/plugins", json(DEPENDED)));
    pane();

    const row = await theRow("GitLab");
    fireEvent.click(row.querySelector("input[type=checkbox]")!);
    fireEvent.click(
      within(
        await waitFor(() => screen.getByRole("group", { name: "Disable GitLab?" })),
      ).getByRole("button", { name: "Cancel" }),
    );

    await waitFor(() =>
      expect(screen.queryByRole("group", { name: "Disable GitLab?" })).toBeNull(),
    );
    expect(row.querySelector("input[type=checkbox]")).toHaveProperty(
      "checked",
      true,
    );
  });
});

describe("Add a plugin…", () => {
  it("shows the consent card, and Install grants what it showed", async () => {
    const fetching = serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever("/api/ui/plugins/read", json(CONSENT), "POST"),
      whenever("/api/ui/plugins", json(PLUGINS), "POST"),
    );
    pane();

    fireEvent.click(await waitFor(() => screen.getByText("Add a plugin…")));
    fireEvent.input(screen.getByLabelText("Directory or git URL"), {
      target: { value: "/home/you/src/card" },
    });
    fireEvent.click(screen.getByText("Read manifest"));

    await waitFor(() => screen.getByText("Card 1.4.0"));
    expect(screen.getByText("gitlab.com")).toBeTruthy();
    expect(screen.getByText("~/.config/glab")).toBeTruthy();
    expect(screen.getByText("GITLAB_TOKEN")).toBeTruthy();

    fireEvent.click(screen.getByText("Install"));

    await waitFor(() => expect(screen.queryByText("Card 1.4.0")).toBeNull());
    const press = fetching.mock.calls.find(
      ([asked, init]) =>
        String(asked) === "/api/ui/plugins" && init?.method === "POST",
    );
    expect(JSON.parse(String(press![1]!.body))).toEqual({
      source: "/home/you/src/card",
      granted: CONSENT.capabilities,
    });
  });

  it("draws a refusal without a card", async () => {
    serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever(
        "/api/ui/plugins/read",
        json(
          {
            error:
              "Its uuid `613e2a43-80ee-4307-8623-28969e05330f` is a built-in plugin's. Give it a uuid of its own.",
          },
          409,
        ),
        "POST",
      ),
    );
    pane();

    fireEvent.click(await waitFor(() => screen.getByText("Add a plugin…")));
    fireEvent.input(screen.getByLabelText("Directory or git URL"), {
      target: { value: "/home/you/src/shipped" },
    });
    fireEvent.click(screen.getByText("Read manifest"));

    await waitFor(() => screen.getByText(/is a built-in plugin's/));
    expect(screen.queryByText("Install")).toBeNull();
  });
});
