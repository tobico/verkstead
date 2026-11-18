//! What the Plugins section says (ADR-0030): a row per plugin with its name,
//! version, author, what it provides and where it came from; its trouble where
//! it didn't load; a local plugin on another device greyed and naming it; the
//! checkbox; and **Add a plugin…** reading the consent card before **Install**
//! grants what it showed.
//!
//! The reads are fixtures the server's own suite wrote off the real endpoints —
//! see `crates/server/tests/suite/plugins.rs`.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { describe, expect, it, vi } from "vitest";

import type { PluginConsent, PluginsView } from "../src/api/types";
import { PluginsCard, PluginsPane } from "../src/settings/Plugins";
import consent from "./fixtures/plugin-consent.json" with { type: "json" };
import plugins from "./fixtures/plugins.json" with { type: "json" };
import { json, serving, whenever } from "./serving";

/// One plugin that loaded, GitLab, and one whose manifest broke, Broken.
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

    await waitFor(() => screen.getByText("1 in trouble"));
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
    expect(row.querySelector("button")).toBeNull();
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

describe("Add a plugin…", () => {
  it("shows the consent card, and Install grants what it showed", async () => {
    const fetching = serving(
      whenever("/api/ui/plugins", json(PLUGINS)),
      whenever("/api/ui/plugins/read", json(CONSENT), "POST"),
      whenever("/api/ui/plugins", json(PLUGINS), "POST"),
    );
    pane();

    fireEvent.click(await waitFor(() => screen.getByText("Add a plugin…")));
    fireEvent.input(screen.getByLabelText("Directory"), {
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
      path: "/home/you/src/card",
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
    fireEvent.input(screen.getByLabelText("Directory"), {
      target: { value: "/home/you/src/shipped" },
    });
    fireEvent.click(screen.getByText("Read manifest"));

    await waitFor(() => screen.getByText(/is a built-in plugin's/));
    expect(screen.queryByText("Install")).toBeNull();
  });
});
