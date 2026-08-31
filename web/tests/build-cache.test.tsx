//! The shared Rust build cache on the settings page: what the card says of it,
//! what the switch in its pane puts on the wire, and the size beside it.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying how the cache stands, and the controls that change it in the
//! details pane it opens. Each is mounted on its own, and the pair together only
//! where the round trip is what is being asked about.
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
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { BuildCacheCard, BuildCachePane } from "../src/settings/BuildCache";
import styles from "../src/settings/BuildCache.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;

/// The paths the fixture holds, as a save puts them back on the wire: the
/// settings' own entries, and a Repo's bind in the `name=path` grammar the file
/// keeps them in. Every section's save carries them, because one request writes
/// the whole of `config.yaml` — a list left out would be a list emptied.
const PATHS = {
  watched_paths: ["/home/ada/src"],
  sandbox_binds: [
    "/var/cache/verkstead-node",
    "verkstead=/var/cache/verkstead-cargo",
  ],
};
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

/// And the same settings with the cache switched off.
function off(standing: SettingsView): SettingsView {
  return {
    ...standing,
    rust_build_cache: { ...standing.rust_build_cache, enabled: false },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/// Whatever half of the cache a test is about, over one query client: both
/// halves read the same two files, so a test mounting the pair is reading them
/// once, exactly as the page does.
function mounting(what: () => JSX.Element) {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>{what()}</QueryClientProvider>
  ));
}

/// The card in the middle pane, and what pressing it asked for.
function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <BuildCacheCard open={open} press={press} />),
    press,
  };
}

/// The controls in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <BuildCachePane back={back} />), back };
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

/// The card itself, once it is drawn — waited for, because it stands on a read.
async function theCard(container: ParentNode): Promise<HTMLElement> {
  return await waitFor(() => {
    const face = container.querySelector<HTMLElement>(
      `.${styles.buildCacheCard}`,
    );
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

function theSwitch(): HTMLInputElement {
  return screen.getByRole("switch") as HTMLInputElement;
}

describe("the card", () => {
  /// The whole shape of the feature, said in the one line somebody scanning the
  /// page reads: nobody has to open anything for their sessions to share a
  /// cache.
  it("says the crates are downloaded once for the machine", async () => {
    theSettings(compiling(UNSET));
    mountCard();

    await waitFor(() => screen.getByText(/downloaded/));
    expect(screen.getByText(/downloaded/).textContent).toContain(
      "and compiled once for this machine",
    );
  });

  it("says so where the cache is switched off", async () => {
    theSettings(off(TOLD));
    mountCard();

    await waitFor(() => screen.getByText(/^Off,/));
  });

  /// What the human cannot fix from the browser, said where they would
  /// otherwise wonder why nothing got faster — and on the card rather than
  /// only in the pane, because whoever needs to read it is whoever is not
  /// editing.
  it("warns when there is no sccache for the server to compile through", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/No sccache is installed/));
    expect(container.querySelector(`.${styles.warning}`)).not.toBeNull();
  });

  /// And nothing about it while the cache is switched off, because the half of
  /// that warning that says the downloads are still shared is only true while
  /// there is a cache to share them.
  it("says nothing about sccache while the cache is switched off", async () => {
    theSettings(off(UNSET));
    mountCard();

    await waitFor(() => screen.getByText(/^Off,/));
    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  it("says nothing about sccache where the server found one", async () => {
    theSettings(compiling(UNSET));
    mountCard();

    await waitFor(() => screen.getByText(/downloaded/));
    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    const face = await theCard(container);
    fireEvent.click(face);

    expect(press).toHaveBeenCalled();
  });

  it("reads as open while its pane is", async () => {
    theSettings(TOLD);
    const { container } = mountCard(true);

    const face = await theCard(container);
    expect(face.classList).toContain(card.open);
    expect(face.getAttribute("aria-pressed")).toBe("true");
  });

  it("says so when the server could not be read at all", async () => {
    serving(() =>
      Promise.resolve(
        new Response("nope", { status: 500, statusText: "Server Error" }),
      ),
    );
    mountCard();

    await waitFor(() => screen.getByText(/Could not read the settings/));
  });
});

describe("the cache as the pane draws it", () => {
  /// The switch says where the cache stands rather than whether anybody has
  /// touched it, which is what an unconfigured cache being on means.
  it("reads as on where nothing has been configured", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => expect(theSwitch().checked).toBe(true));
  });

  it("says when there is no sccache for the server to compile through", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByText(/No sccache is installed/));

    // And no size field, because the size is sccache's own and there is no
    // sccache to read it.
    expect(screen.queryByLabelText(/How large/)).toBeNull();
  });

  it("says nothing about sccache while the cache is switched off", async () => {
    theSettings(off(UNSET));
    mountPane();

    await waitFor(() => expect(theSwitch().checked).toBe(false));

    expect(screen.queryByText(/No sccache is installed/)).toBeNull();
  });

  /// An unconfigured size is the default drawn as a placeholder rather than as
  /// text somebody typed — the field says what will happen without claiming
  /// anybody chose it.
  it("draws a size nobody configured as the placeholder", async () => {
    theSettings(compiling(UNSET));
    mountPane();

    const field = (await waitFor(() =>
      screen.getByLabelText(/How large/),
    )) as HTMLInputElement;

    expect(field.value).toBe("");
    expect(field.placeholder).toBe("30G");
  });

  it("draws a size somebody configured as the value", async () => {
    theSettings(compiling(TOLD));
    mountPane();

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
    const fetching = theSettings(TOLD, json(answering(off(TOLD))));
    mountPane();

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
        share_viewer_url: TOLD.share_viewer_url,
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        conflict_resolution: TOLD.conflict_resolution,
        ...PATHS,
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
    mountPane();

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
        share_viewer_url: TOLD.share_viewer_url,
        // Untouched by this form, and sent back as it stands: one request
        // writes the whole of `config.yaml`.
        conflict_resolution: TOLD.conflict_resolution,
        ...PATHS,
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
    mountPane();

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

  /// What the pane saved is what the card goes back to saying, because the
  /// answer is a fresh read of the files that both halves are drawn from.
  it("says on the card what the pane switched off", async () => {
    theSettings(TOLD, json(answering(off(TOLD))));
    mounting(() => (
      <>
        <BuildCacheCard open press={() => {}} />
        <BuildCachePane back={() => {}} />
      </>
    ));

    await waitFor(() => expect(theSwitch().checked).toBe(true));
    fireEvent.click(theSwitch());

    await waitFor(() => screen.getByText(/^Off,/));
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
    mountPane();

    await waitFor(() => expect(theSwitch().checked).toBe(true));
    fireEvent.click(theSwitch());

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});
