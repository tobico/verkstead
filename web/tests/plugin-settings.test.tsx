//! What a Plugin's settings pane says (ADR-0030, Set 1769, Q43): the fields
//! its manifest declares, each drawn by kind with the cluster's value; a set
//! secret drawn as **Replace** and **Clear** and never shown; a save sending
//! what was typed; and a refused size drawn at its field.
//!
//! The pane is a fixture the server's own suite wrote off the real endpoint —
//! see `crates/server/tests/suite/plugins.rs`.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { describe, expect, it, vi } from "vitest";

import type {
  LanguageCleared,
  PluginLanguageCache,
  PluginSettingsSaved,
  PluginSettingsView,
} from "../src/api/types";
import { PluginSettingsPane } from "../src/settings/PluginSettings";
import settings from "./fixtures/plugin-settings.json" with { type: "json" };
import { json, serving, whenever } from "./serving";

/// GitLab's pane: Host and Groups set, Cache size not, and Token a secret
/// that is set.
const SETTINGS = settings as PluginSettingsView;
const AT = `/api/ui/plugins/${SETTINGS.uuid}/settings`;

function pane() {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>
      <PluginSettingsPane plugin={SETTINGS.uuid} back={vi.fn()} />
    </QueryClientProvider>
  ));
}

/// The box drawn under the label `label`.
async function box(label: string): Promise<HTMLInputElement> {
  return waitFor(() => screen.getByLabelText(label) as HTMLInputElement);
}

/// What the save sent, as its body.
function sent(fetching: ReturnType<typeof serving>): unknown {
  const press = fetching.mock.calls.find(
    ([asked, init]) => String(asked) === AT && init?.method === "POST",
  );
  return JSON.parse(String(press![1]!.body));
}

describe("a plugin's settings pane", () => {
  it("draws each field by kind with the cluster's value", async () => {
    serving(whenever(AT, json(SETTINGS)));
    pane();

    expect((await box("Host")).value).toBe("gitlab.example.com");
    expect((await box("Cache size")).value).toBe("");
    expect((await box("Groups")).value).toBe("infra\nweb");
    screen.getByText("The GitLab instance, where it isn't gitlab.com.");
  });

  /// A size's placeholder is the server's: a Language plugin's default size,
  /// and nothing where the field has none.
  it("draws a size's placeholder as the server gives it", async () => {
    const sized = (placeholder: string | null): PluginSettingsView => ({
      ...SETTINGS,
      fields: SETTINGS.fields.map((field) =>
        field.label === "Cache size" ? { ...field, placeholder } : field,
      ),
    });

    serving(whenever(AT, json(sized("30G"))));
    const { unmount } = pane();
    expect((await box("Cache size")).placeholder).toBe("30G");
    unmount();

    serving(whenever(AT, json(sized(null))));
    pane();
    expect((await box("Cache size")).placeholder).toBe("");
  });

  it("draws a set secret as its presses, and never its value", async () => {
    serving(whenever(AT, json(SETTINGS)));
    pane();

    await waitFor(() => screen.getByRole("button", { name: "Replace" }));
    screen.getByRole("button", { name: "Clear" });
    expect(screen.queryByLabelText("Token")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Replace" }));

    const token = await box("Token");
    expect(token.type).toBe("password");
    expect(token.value).toBe("");
  });

  it("saves what was typed, keeping a secret nobody typed", async () => {
    const answer: PluginSettingsSaved = { settings: SETTINGS, refused: [] };
    const fetching = serving(
      whenever(AT, json(SETTINGS)),
      whenever(AT, json(answer), "POST"),
    );
    pane();

    fireEvent.input(await box("Cache size"), { target: { value: "2G" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText("Saved."));
    expect(sent(fetching)).toEqual({
      fields: [
        { key: "host", value: { kind: "Text", value: "gitlab.example.com" } },
        { key: "cache", value: { kind: "Size", value: "2G" } },
        { key: "groups", value: { kind: "List", values: ["infra", "web"] } },
      ],
    });
  });

  it("clears a secret with its press", async () => {
    const answer: PluginSettingsSaved = { settings: SETTINGS, refused: [] };
    const fetching = serving(
      whenever(AT, json(SETTINGS)),
      whenever(AT, json(answer), "POST"),
    );
    pane();

    fireEvent.click(
      await waitFor(() => screen.getByRole("button", { name: "Clear" })),
    );

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        fields: [{ key: "token", value: { kind: "SecretCleared" } }],
      }),
    );
  });

  it("draws a refused size at its field, with the reason", async () => {
    const why =
      "2 gigs isn't a size. Write a whole number with K, M, G or T after it, such as 30G.";
    const answer: PluginSettingsSaved = {
      settings: SETTINGS,
      refused: [{ key: "cache", why }],
    };
    serving(whenever(AT, json(SETTINGS)), whenever(AT, json(answer), "POST"));
    pane();

    fireEvent.input(await box("Cache size"), { target: { value: "2 gigs" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    const reason = await waitFor(() => screen.getByText(why));
    expect(reason.closest("[data-setting]")?.getAttribute("data-setting")).toBe(
      "cache",
    );
    expect((await box("Cache size")).value).toBe("2 gigs");
    expect(screen.queryByText("Saved.")).toBeNull();
  });

  it("still draws the fields of a plugin in trouble, under its trouble", async () => {
    serving(
      whenever(
        AT,
        json({ ...SETTINGS, trouble: "Its `plugin.yaml` doesn't parse." }),
      ),
    );
    pane();

    await waitFor(() => screen.getByText("Its `plugin.yaml` doesn't parse."));
    await box("Host");
  });

  describe("a Language plugin's build cache", () => {
    const GIB = 1024 ** 3;

    /// Rust & C/C++'s pane, its size field and its two languages: Rust's
    /// store holding `held` bytes, and C/C++ compiling through it.
    const sized = (
      held: number,
      running = { sessions: 0, terminals: 0 },
    ): PluginSettingsView => {
      const rust: PluginLanguageCache = {
        name: "rust",
        label: "Rust",
        size: "30G",
        disk_use: { Measured: { bytes: held, in_units: held } },
        stores: [
          { name: "cargo", eviction: "ByUnit" },
          { name: "sccache", eviction: "ByItsTool" },
        ],
        swept: null,
        compiling: "Cached",
        through: null,
      };
      const cpp: PluginLanguageCache = {
        name: "cpp",
        label: "C/C++",
        size: "30G",
        disk_use: "NoStore",
        stores: [],
        swept: null,
        compiling: "Cached",
        through: "Rust",
      };

      return {
        ...SETTINGS,
        name: "Rust & C/C++",
        fields: [
          {
            key: "size",
            label: "Store size limit",
            description: null,
            placeholder: "30G",
            value: { kind: "Size", value: "" },
          },
        ],
        languages: [rust, cpp],
        running,
      };
    };
    const CLEAR = "/api/ui/languages/rust/clear";

    it("draws each language's store against its size, under the fields", async () => {
      serving(whenever(AT, json(sized(3.2 * GIB))));
      pane();

      await waitFor(() =>
        screen.getByText("Cache usage: 3.2/30GB, 3.2GB of it in packages."),
      );
      screen.getByText("cargo");
      screen.getByText(
        "sccache holds its compile cache to this size by itself, so the store may hold up to twice it.",
      );
      screen.getByText(/^Not trimmed since the server started/);
      screen.getByText(
        "Compiles through the compile server, so its cache is Rust's.",
      );
      // One Clear, Rust's: C/C++ has no store of its own.
      expect(screen.getAllByRole("button", { name: /^Clear / })).toHaveLength(
        1,
      );
    });

    it("clears a store and draws the figure again", async () => {
      let cleared = false;
      const fetching = serving(
        whenever(AT, () => json(sized(cleared ? 0 : 3.2 * GIB))()),
        whenever(
          CLEAR,
          () => {
            cleared = true;
            return json({
              Cleared: { settings: {} },
            } as unknown as LanguageCleared)();
          },
          "POST",
        ),
      );
      pane();

      fireEvent.click(
        await waitFor(() =>
          screen.getByRole("button", { name: "Clear Rust's cache" }),
        ),
      );

      await waitFor(() =>
        screen.getByText("Cache usage: 0.0/30GB, 0 bytes of it in packages."),
      );
      expect(
        fetching.mock.calls.some(
          ([asked, init]) => String(asked) === CLEAR && init?.method === "POST",
        ),
      ).toBe(true);
    });

    it("draws Clear off with what is running, and a refusal the same way", async () => {
      serving(whenever(AT, json(sized(GIB, { sessions: 1, terminals: 2 }))));
      const { unmount } = pane();

      const clear = await waitFor(() =>
        screen.getByRole("button", { name: "Clear Rust's cache" }),
      );
      expect((clear as HTMLButtonElement).disabled).toBe(true);
      screen.getByText(
        "Clear waits until no session or terminal is running. 1 session and 2 terminals are running.",
      );
      unmount();

      serving(
        whenever(AT, json(sized(GIB))),
        whenever(
          CLEAR,
          json({
            Running: { sessions: 1, terminals: 0 },
          } satisfies LanguageCleared),
          "POST",
        ),
      );
      pane();

      fireEvent.click(
        await waitFor(() =>
          screen.getByRole("button", { name: "Clear Rust's cache" }),
        ),
      );
      await waitFor(() =>
        screen.getByText(
          "Clear waits until no session or terminal is running. 1 session is running.",
        ),
      );
      expect(
        (
          screen.getByRole("button", {
            name: "Clear Rust's cache",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(true);
    });
  });
});
