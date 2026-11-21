//! The Build caches part of the Sandbox pane, which has no language rows: a
//! language's switch, size and cache are its plugin's, on the Plugins page —
//! see `plugin-settings.test.tsx` and `plugins.test.tsx`. What the part keeps
//! is the line saying there is no sccache where the server can see one.
//!
//! The part mounted on its own, as `BuildCaches`, because that is what it is:
//! one of the two parts the Sandbox pane draws under its head. What the Sandbox
//! card says of the caches and the pane that holds this part are
//! `sandbox.test.tsx`'s.
//!
//! The read is a fixture the server's own tests wrote — the descriptors
//! Verkstead ships, and one an installer wrote into `config.yaml` — so what the
//! page is drawn from is the shape the endpoint really answers with.
//!
//! And the figures a language's cache is said in, which `LanguageCache.tsx`
//! draws in a Language plugin's pane.

import { render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SettingsView } from "../src/api/types";
import { bytesSaid, usageSaid } from "../src/settings/LanguageCache";
import { BuildCaches } from "../src/settings/Languages";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };

const TOLD = told as SettingsView;

/// The same settings with an sccache the server did find, which no fixture
/// carries: the routers those are written from run no sessions, so they have
/// none to hand out.
function compiling(standing: SettingsView): SettingsView {
  return {
    ...standing,
    languages: standing.languages.map((language) => ({
      ...language,
      compiling: language.compiling ? "Cached" : null,
    })),
  };
}

/// The part, over a query client of its own, as the page holds it.
function mountPane() {
  const queries = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(() => (
    <QueryClientProvider client={queries}>
      <BuildCaches />
    </QueryClientProvider>
  ));
}

function theSettings(standing: SettingsView) {
  return serving(whenever("/api/ui/settings", json(standing)));
}

const POINTED = "A language's build cache is in its plugin's settings, on the Plugins page.";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the Build caches part", () => {
  /// The part says what it is under a heading of its own, because the pane's
  /// head is the Sandbox's and the binds stand beside it under theirs.
  it("heads the part Build Caches", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => screen.getByText(POINTED));
    expect(container.querySelector("h3")?.textContent).toBe("Build Caches");
    expect(container.querySelector("h1")).toBeNull();
  });

  /// No row for any language — a built-in's or an installer's own — and no
  /// box: each is a Language plugin, ticked on the Plugins page.
  it("draws no language rows, an installer's own included", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByText(POINTED));
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    for (const label of ["Rust", "Go", "C/C++", "Gleam"]) {
      expect(screen.queryByText(label), label).toBeNull();
    }
  });

  /// No sccache where the server can see one is said at the head of the
  /// part: it is the server's, and no language's.
  it("says when there is no sccache", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => screen.getByText(/The server can't see an sccache/));
  });

  /// And nothing where it found one.
  it("says nothing about sccache where the server found one", async () => {
    theSettings(compiling(TOLD));
    mountPane();

    await waitFor(() => screen.getByText(POINTED));
    expect(screen.queryByText(/The server can't see an sccache/)).toBeNull();
  });
});

const GB = 2 ** 30;

describe("saying how much a store holds", () => {
  /// In the units a human writes a size in, so a figure reads against the
  /// size beside it without a conversion.
  it("says bytes in KB, MB, GB and TB", () => {
    expect(bytesSaid(0)).toBe("0 bytes");
    expect(bytesSaid(1023)).toBe("1023 bytes");
    expect(bytesSaid(1536)).toBe("1.5KB");
    expect(bytesSaid(20 * 2 ** 20)).toBe("20MB");
    expect(bytesSaid(30 * GB)).toBe("30GB");
    expect(bytesSaid(2 * 2 ** 40)).toBe("2.0TB");
  });

  /// And what is held against the size, in the size's unit — or its own,
  /// where it would round to nothing in that one.
  it("says usage against a size in the size's unit", () => {
    expect(usageSaid(3.2 * GB, "30G")).toBe("3.2/30GB");
    expect(usageSaid(0.4 * GB, "10G")).toBe("0.4/10GB");
    expect(usageSaid(12.4 * GB, "30G")).toBe("12/30GB");
    expect(usageSaid(0, "10G")).toBe("0.0/10GB");
    expect(usageSaid(20 * 2 ** 20, "10G")).toBe("20MB/10GB");
    expect(usageSaid(300 * 2 ** 20, "500M")).toBe("300/500MB");
    expect(usageSaid(GB, "lots")).toBe("1.0GB/lots");
  });
});
