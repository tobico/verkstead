//! The share viewer on the settings page: what the card says of it, what the
//! pane hands over, and what the field puts on the wire.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying where the viewer is hosted, and the file, the steps and the field
//! in the details pane it opens.
//!
//! The section is about a job rather than a switch — take the page away, host
//! it, say where it went — so what these ask about is the whole of that: that
//! the file is reachable, that the address goes in and comes back, and that
//! saving it leaves the credentials and the build cache exactly where they were.
//! The page has one endpoint and it writes both settings files, which is what
//! makes that last one worth a test rather than a reading of the source.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with. The page the viewer
//! *is* — the one this section hands over — is `viewing.test.ts`'s subject.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsSaved, SettingsView } from "../src/api/types";
import card from "../src/CardButton.module.css";
import { ShareViewerCard, ShareViewerPane } from "../src/settings/ShareViewer";
import styles from "../src/settings/ShareViewer.module.css";
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

/// Where the fixture says the viewer is hosted, which is what the card draws and
/// what the field is filled with.
const HOSTED = TOLD.share_viewer_url;

afterEach(() => {
  vi.unstubAllGlobals();
});

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
    ...mounting(() => <ShareViewerCard open={open} press={press} />),
    press,
  };
}

/// The controls in the details pane, and what its way back asked for.
function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <ShareViewerPane back={back} />), back };
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

/// The same settings with the viewer hosted somewhere else, which is what a
/// save of this field comes back as.
function hosting(standing: SettingsView, url: string): SettingsView {
  return { ...standing, share_viewer_url: url };
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
      `.${styles.shareViewerCard}`,
    );
    expect(face, "expected the card to be drawn").not.toBeNull();
    return face!;
  });
}

function theField(): HTMLInputElement {
  return screen.getByLabelText(/Where you hosted it/) as HTMLInputElement;
}

describe("the card", () => {
  it("says where published shares are read through", async () => {
    theSettings(TOLD);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/Published shares are read through/));

    expect(container.querySelector(`.${styles.hosted}`)!.textContent).toBe(
      HOSTED,
    );
  });

  /// What leaving this alone costs, said where somebody scanning the page reads
  /// it — rather than found out by whoever opens the link on a pull request and
  /// is shown the file as source.
  it("says what a share is linked as while nobody has hosted one", async () => {
    theSettings(UNSET);
    const { container } = mountCard();

    await waitFor(() => screen.getByText(/No share viewer is hosted/));
    expect(container.querySelector(`.${styles.warning}`)).not.toBeNull();
  });

  it("opens the pane when it is pressed", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    fireEvent.click(await theCard(container));

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

describe("the pane", () => {
  /// The file is the first of the three steps and the only one Verkstead can do
  /// anything about: a link, because a link is how a browser hands over a file.
  it("hands the viewer over as a file to host", async () => {
    theSettings(UNSET);
    const { container } = mountPane();

    await waitFor(() => screen.getByText(/Download the share viewer/));

    const link = container.querySelector<HTMLAnchorElement>(
      `.${styles.download}`,
    )!;
    expect(link.getAttribute("href")).toBe("/api/ui/share-viewer.html");
    expect(link.getAttribute("download")).toBe("verkstead-share-viewer.html");
  });

  it("fills the field with the address that is configured", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => expect(theField().value).toBe(HOSTED));
  });

  it("stands empty where nobody has hosted one", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByLabelText(/Where you hosted it/));
    expect(theField().value).toBe("");
  });

  /// The round trip, and the whole of what this section is for: what was typed
  /// goes on the wire, and everything else goes with it as it stands — the
  /// server writes both files in one request, and a save here must not cost a
  /// token or a build cache size.
  it("saves the address without disturbing anything else", async () => {
    const now = hosting(TOLD, "https://ada.github.io/elsewhere/");
    const fetching = theSettings(TOLD, json(answering(now)));
    mountPane();

    await waitFor(() => expect(theField().value).toBe(HOSTED));

    fireEvent.input(theField(), {
      target: { value: "https://ada.github.io/elsewhere/" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        git_author: TOLD.git_author,
        github_token: "Keep",
        rust_build_cache: {
          enabled: TOLD.rust_build_cache.enabled,
          size: TOLD.rust_build_cache.size,
        },
        share_viewer_url: "https://ada.github.io/elsewhere/",
        ...PATHS,
      }),
    );

    // And the field follows the answer rather than what was typed into it.
    await waitFor(() =>
      expect(theField().value).toBe("https://ada.github.io/elsewhere/"),
    );
  });

  /// Clearing the box is how the setting is taken away — there is nothing else
  /// to press, and an empty address is what nothing configured looks like on
  /// both sides of the wire.
  it("sends an empty address for a field the human cleared", async () => {
    const fetching = theSettings(TOLD, json(answering(hosting(TOLD, ""))));
    mountPane();

    await waitFor(() => expect(theField().value).toBe(HOSTED));

    fireEvent.input(theField(), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(
        (sent(fetching) as { share_viewer_url: string }).share_viewer_url,
      ).toBe(""),
    );
  });

  /// What the setting buys, with the address in it: the link a comment on a
  /// pull request carries, and the `#` that keeps the gist out of the host's
  /// logs.
  it("shows what a published share will be linked as", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => screen.getByText(/A published share is linked as/));

    expect(container.querySelector(`.${styles.link}`)!.textContent).toBe(
      `${HOSTED}#the-gist-id`,
    );
  });

  it("shows no such line while nobody has hosted one", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByLabelText(/Where you hosted it/));
    expect(screen.queryByText(/A published share is linked as/)).toBeNull();
  });

  it("says so when the save fails", async () => {
    theSettings(TOLD, () =>
      Promise.resolve(
        new Response("nope", { status: 503, statusText: "Unavailable" }),
      ),
    );
    mountPane();

    await waitFor(() => expect(theField().value).toBe(HOSTED));

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => screen.getByText(/could not be saved/));
  });
});
