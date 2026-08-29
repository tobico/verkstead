//! The shared Rust build cache on the settings page: what the switch says, what
//! flipping it puts on the wire, and the size beside it.
//!
//! Two saves and one endpoint. The switch is its own press, because a switch
//! that needed a second one is not a switch; the size is typed, so it waits for
//! a Save. Both send the whole of the settings edit — the author as it stands
//! and the token untouched — because the server writes both files in one
//! request, and that is what these check is not lost.
//!
//! The read is a fixture the server's own tests wrote, so what the page is
//! drawn from is the shape the endpoint really answers with.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import { BuildCache } from "../src/settings/BuildCache";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The same settings with an sccache the server did find, which no fixture
/// carries: the routers those are written from run no sessions, so they have
/// none to hand out.
function compiling(standing: SettingsView): SettingsView {
  return {
    ...standing,
    rust_build_cache: { ...standing.rust_build_cache, compiles_cached: true },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={client}>
      <BuildCache />
    </QueryClientProvider>
  ));
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// What a save answers with, which is the settings as they now stand.
function answering(standing: SettingsView): SettingsSaved {
  return { settings: standing, verified: null };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

function theSwitch(): HTMLInputElement {
  return screen.getByRole("switch") as HTMLInputElement;
}

describe("the build cache as it stands", () => {
  /// The whole shape of the feature: nobody has to find this section for their
  /// sessions to share a cache.
  it("reads as on where nothing has been configured", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() => expect(theSwitch().checked).toBe(true));
  });

  /// What the human cannot fix from the browser, said where they would
  /// otherwise wonder why nothing got faster.
  it("says when there is no sccache for the server to compile through", async () => {
    theSettings(UNSET);
    mount();

    await waitFor(() => screen.getByText(/No sccache is installed/));

    // And no size field, because the size is sccache's own and there is no
    // sccache to read it.
    expect(screen.queryByLabelText(/How large/)).toBeNull();
  });

  /// And nothing about it while the cache is switched off, because the half of
  /// that warning that says the downloads are still shared is only true while
  /// there is a cache to share them.
  it("says nothing about sccache while the cache is switched off", async () => {
    theSettings({
      ...UNSET,
      rust_build_cache: { ...UNSET.rust_build_cache, enabled: false },
    });
    mount();

    await waitFor(() => expect(theSwitch().checked).toBe(false));

    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  it("says nothing about sccache where the server found one", async () => {
    theSettings(compiling(UNSET));
    mount();

    await waitFor(() => screen.getByLabelText(/How large/));

    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  /// An unconfigured size is the default drawn as a placeholder rather than as
  /// text somebody typed — the field says what will happen without claiming
  /// anybody chose it.
  it("draws a size nobody configured as the placeholder", async () => {
    theSettings(compiling(UNSET));
    mount();

    const field = (await waitFor(() =>
      screen.getByLabelText(/How large/),
    )) as HTMLInputElement;

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("30G");
  });

  it("draws a size somebody configured as the value", async () => {
    theSettings(compiling(TOLD));
    mount();

    const field = (await waitFor(() =>
      screen.getByLabelText(/How large/),
    )) as HTMLInputElement;

    expect(field.value).toBe(TOLD.rust_build_cache.size);
  });
});

describe("changing the build cache", () => {
  /// A switch is its own save. What goes with it is the author as it stands and
  /// `Keep` for the token: one request writes both files, so a flip here must
  /// not be able to take the credentials with it.
  it("saves the moment the switch is flipped, and leaves the credentials alone", async () => {
    const off: SettingsView = {
      ...TOLD,
      rust_build_cache: { ...TOLD.rust_build_cache, enabled: false },
    };
    const fetching = theSettings(TOLD, json(answering(off)));
    mount();

    await waitFor(() => expect(theSwitch().checked).toBe(true));
    fireEvent.click(theSwitch());

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
        rust_build_cache: {
          enabled: false,
          size: TOLD.rust_build_cache.size,
        },
      }),
    );

    // And the switch follows the answer rather than the press.
    await waitFor(() => expect(theSwitch().checked).toBe(false));
  });

  /// The size is typed, so it waits for a press: nothing is committed while
  /// somebody is halfway through writing `30`.
  it("sends a size only when it is saved", async () => {
    const bigger: SettingsView = {
      ...compiling(TOLD),
      rust_build_cache: {
        ...compiling(TOLD).rust_build_cache,
        size: "80G",
        size_configured: true,
      },
    };
    const fetching = theSettings(compiling(TOLD), json(answering(bigger)));
    mount();

    const field = await waitFor(() => screen.getByLabelText(/How large/));
    fireEvent.input(field, { target: { value: "80G" } });

    expect(
      fetching.mock.calls.some(([, init]) => init?.method === "POST"),
      "typing is not saving",
    ).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
        rust_build_cache: { enabled: true, size: "80G" },
      }),
    );
  });

  /// Clearing the field asks for the default back, which is what an empty size
  /// means to the server — and the placeholder is what says so.
  it("sends an empty size for a field the human cleared", async () => {
    const fetching = theSettings(
      compiling(TOLD),
      json(answering(compiling(UNSET))),
    );
    mount();

    const field = await waitFor(() => screen.getByLabelText(/How large/));
    fireEvent.input(field, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(
        (sent(fetching) as { rust_build_cache: { size: string } })
          .rust_build_cache.size,
      ).toBe(""),
    );
  });

  /// A save that would not land keeps the page honest about it: a settings page
  /// that quietly saved nothing is how a machine ends up not being what it says.
  it("says so when the save fails", async () => {
    serving(
      whenever("/api/ui/settings", json(TOLD)),
      () =>
        Promise.resolve(
          new Response("nope", { status: 503, statusText: "Service Unavailable" }),
        ),
    );
    mount();

    await waitFor(() => expect(theSwitch().checked).toBe(true));
    fireEvent.click(theSwitch());

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});
