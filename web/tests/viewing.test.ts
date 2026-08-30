//! The share viewer: the page that turns a published share into a read.
//!
//! `crates/server/share-viewer.html` is not part of this bundle and never will
//! be — it is served from a public site, Verkstead's own GitHub Pages or a copy
//! the human hosts, which is the only place it can do its job from. So it is
//! read here as text and run against this document, the way the service worker
//! beside it is: the markup goes into the page, the script is evaluated, and
//! what it does with `fetch` and the fragment is what these ask about.
//!
//! Three promises, and almost every test here is one of them. The last describe
//! is about the page's *address* rather than the page: three files spell that
//! out separately, and one of them drifting is a 404 on every share ever
//! published.
//!
//! **It draws the share whole.** The Gists API cuts a file off at a megabyte and
//! says so with `truncated`; a share is several. So the page follows the API's
//! `raw_url` and never its `content` — that is the difference between a
//! conversation and the first megabyte of one.
//!
//! **It tells nobody anything.** The gist id rides in the fragment, which no
//! browser sends to the host of the page, and the only thing fetched is
//! GitHub's — so a recipient reading a share tells whoever hosts this page
//! nothing beyond that they opened it.
//!
//! **And it does not hand the share its origin.** A share is an application and
//! has to run, so the frame allows scripts and withholds same-origin: whatever
//! else the human hosts beside this page is not the share's to reach.

import { waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";

/// The page exactly as it is handed over. Reading above `web/` is what
/// `server.fs.allow` is opened for under vitest — see `vite.config.ts`.
import SOURCE from "../../crates/server/share-viewer.html?raw";

/// And the two files that say where it is published, read the same way: the
/// workflow that puts it there, and the module that composes every link through
/// it. What the address is compared against is `HOSTED` on the settings page —
/// see the last describe below.
import WORKFLOW from "../../.github/workflows/pages.yml?raw";
import SHARING from "../../crates/server/src/sharing.rs?raw";
import { HOSTED } from "../src/settings/ShareViewer";

/// What is inside its `<body>`, which is the markup and the script it runs.
const BODY = /<body>([\s\S]*)<\/body>/.exec(SOURCE)![1]!;

/// The script on its own, to be evaluated…
const SCRIPT = /<script>([\s\S]*?)<\/script>/.exec(BODY)![1]!;

/// …and the markup without it, to be put in the document first: the script
/// reaches for elements the moment it runs, as it does in a browser that has
/// already parsed the body around it.
const MARKUP = BODY.replace(/<script>[\s\S]*?<\/script>/, "");

/// A gist as the API answers for one, holding a share.
///
/// The share itself is not in it, which is the point: `content` is as much of
/// the file as the API will hand over, and `truncated` is the API saying so —
/// on a share, always.
function gist(id = "9f1") {
  return {
    id,
    description: "A Verkstead conversation: sharing",
    files: {
      "sharing-2026-08-30.html": {
        filename: "sharing-2026-08-30.html",
        raw_url: `https://gist.githubusercontent.com/tobico/${id}/raw/sharing-2026-08-30.html`,
        truncated: true,
        content: THE_FIRST_MEGABYTE,
      },
    },
  };
}

/// What the API hands over in place of a file too big for it. Nothing here is
/// ever what the page should draw.
const THE_FIRST_MEGABYTE = "<!doctype html><html><body>cut off at a megabyte";

/// And a whole share, as the raw URL answers with it.
const SHARE =
  '<!doctype html><html><head><title>sharing</title></head><body><div id="app">the whole conversation</div></body></html>';

/// Every listener the page has put on the window, so that a page opened by one
/// test is not still listening while the next one runs. A browser gets a fresh
/// document per load and this does not.
const listening: Array<[string, EventListener]> = [];

/// Open the page at a link, with GitHub answering as `answers` says.
///
/// The fragment first, because the script reads it as it runs — which is what a
/// browser does with a link somebody followed.
function opens(
  fragment: string,
  ...answers: Array<(url: string) => Promise<Response>>
) {
  location.hash = fragment;
  document.body.innerHTML = MARKUP;

  let taken = 0;
  const fetching = vi.fn((asked: RequestInfo | URL) =>
    answers[Math.min(taken++, answers.length - 1)]!(String(asked)),
  );
  vi.stubGlobal("fetch", fetching);

  const added = window.addEventListener.bind(window);
  window.addEventListener = (
    name: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ) => {
    listening.push([name, listener as EventListener]);
    added(name, listener, options);
  };

  new Function(SCRIPT)();

  window.addEventListener = added;

  return fetching;
}

/// One answer, as GitHub would have written it.
function json(body: unknown): (url: string) => Promise<Response> {
  return () =>
    Promise.resolve(
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
}

/// And one file, as a raw URL hands it over: plain text, which is exactly why
/// a browser cannot be pointed at it.
function raw(text: string): (url: string) => Promise<Response> {
  return () =>
    Promise.resolve(
      new Response(text, {
        status: 200,
        headers: { "content-type": "text/plain; charset=utf-8" },
      }),
    );
}

/// A refusal, in GitHub's own numbers.
function refuses(status: number, statusText: string) {
  return () => Promise.resolve(new Response("", { status, statusText }));
}

const frame = () => document.getElementById("share") as HTMLIFrameElement;
const standing = () => document.getElementById("standing")!;

/// The share once it is drawn, which is what the frame is carrying.
async function drawn(): Promise<string> {
  await waitFor(() => expect(frame().hidden).toBe(false));
  return frame().srcdoc;
}

/// What the page says in place of a share, once it has stopped saying it is
/// fetching one: that line is what stands while GitHub is being asked, so a
/// test that read the first thing on the page would read it every time.
async function said(): Promise<string> {
  await waitFor(() =>
    expect(standing().textContent ?? "").not.toContain("Fetching"),
  );

  expect(standing().hidden).toBe(false);

  return standing().textContent ?? "";
}

afterEach(() => {
  for (const [name, listener] of listening.splice(0)) {
    window.removeEventListener(name, listener);
  }

  vi.unstubAllGlobals();
  location.hash = "";
  document.body.innerHTML = "";
});

describe("a link to a published share", () => {
  it("draws the share, with no download in the way", async () => {
    const fetching = opens("#9f1", json(gist()), raw(SHARE));

    expect(await drawn()).toBe(SHARE);

    // The gist the link named, asked of GitHub itself.
    expect(String(fetching.mock.calls[0]![0])).toBe(
      "https://api.github.com/gists/9f1",
    );
  });

  /// The whole reason the raw URL is followed at all: a share runs to several
  /// megabytes and the API hands over the first one.
  it("draws the whole share rather than the megabyte the API answers with", async () => {
    const fetching = opens("#9f1", json(gist()), raw(SHARE));

    const drew = await drawn();

    expect(drew).toBe(SHARE);
    expect(drew).not.toContain("cut off at a megabyte");

    // The URL the API named, rather than one composed here: what a gist's raw
    // file is called is GitHub's business, revision and all.
    expect(String(fetching.mock.calls[1]![0])).toBe(
      "https://gist.githubusercontent.com/tobico/9f1/raw/sharing-2026-08-30.html",
    );
  });

  /// Nothing but GitHub, which is the whole of what a recipient's browser gives
  /// away by reading a share through this page.
  it("reaches for nothing but GitHub", async () => {
    const fetching = opens("#9f1", json(gist()), raw(SHARE));

    await drawn();

    for (const [asked] of fetching.mock.calls) {
      expect(String(asked)).toMatch(
        /^https:\/\/(api\.github\.com|gist\.githubusercontent\.com)\//,
      );
    }
  });

  /// A share is an application and has to run. What it must not have is the
  /// origin of whatever else the human hosts beside this page.
  it("runs the share without handing it this page's origin", async () => {
    opens("#9f1", json(gist()), raw(SHARE));

    await drawn();

    expect(frame().getAttribute("sandbox")).toBe("allow-scripts");
  });

  /// The gist's own page URL is a link to the same gist, and somebody who has
  /// one in their hands will paste it after the `#`.
  it("takes the id out of a pasted gist URL", async () => {
    const fetching = opens(
      "#https://gist.github.com/tobico/9f1",
      json(gist()),
      raw(SHARE),
    );

    await drawn();

    expect(String(fetching.mock.calls[0]![0])).toBe(
      "https://api.github.com/gists/9f1",
    );
  });

  /// The page is hosted rather than updated, so a link nobody could parse must
  /// not be the end of it: what was typed is asked about, and GitHub says what
  /// it makes of it.
  it("asks about a fragment nothing can decode rather than dying on it", async () => {
    const fetching = opens("#%zz", refuses(404, "Not Found"));

    expect(await said()).toContain("404 Not Found");
    expect(String(fetching.mock.calls[0]![0])).toBe(
      "https://api.github.com/gists/%25zz",
    );
  });

  /// A fragment changes without the page loading again, so a link pasted into a
  /// tab that is already open has to be heard.
  it("follows a link pasted into a tab that is already open", async () => {
    opens("#9f1", json(gist()), raw(SHARE));
    await drawn();

    const second = "<!doctype html><html><body>another conversation";
    vi.stubGlobal(
      "fetch",
      vi.fn((asked: RequestInfo | URL) =>
        String(asked).startsWith("https://api.github.com/")
          ? json(gist("a72"))(String(asked))
          : raw(second)(String(asked)),
      ),
    );

    location.hash = "#a72";
    window.dispatchEvent(new Event("hashchange"));

    await waitFor(() => expect(frame().srcdoc).toBe(second));
  });
});

describe("a link that draws nothing", () => {
  it("says so where it names no gist at all", async () => {
    opens("");

    expect(await said()).toContain("does not name one");
    expect(frame().hidden).toBe(true);
  });

  it("says so where GitHub will not hand the gist over", async () => {
    opens("#9f1", refuses(404, "Not Found"));

    expect(await said()).toContain("404 Not Found");
    expect(frame().hidden).toBe(true);
  });

  it("says so where GitHub cannot be reached at all", async () => {
    opens("#9f1", () => Promise.reject(new Error("offline")));

    expect(await said()).toContain("offline");
  });

  it("says so where the gist holds no file", async () => {
    opens("#9f1", json({ id: "9f1", files: {} }));

    expect(await said()).toContain("no file");
  });

  it("says so where the share itself will not come", async () => {
    opens("#9f1", json(gist()), refuses(451, "Unavailable"));

    expect(await said()).toContain("451 Unavailable");
  });
});

/// Where the page is, which is a fact three files hold separately: the workflow
/// that publishes it, the server constant every link is composed through, and
/// the settings page that says which viewer this Verkstead is using.
///
/// Nothing composes the three. A viewer published to one address and linked at
/// another is a 404 on every share ever published, and the first person to find
/// out would be whoever opened a link on a pull request — so this is the
/// comparison, kept beside the page itself.
describe("where the viewer is published", () => {
  it("is the one address the workflow, the server and the settings page hold", () => {
    expect(WORKFLOW).toContain(`EXPECTED: ${HOSTED}`);
    expect(SHARING).toContain(`pub(crate) const HOSTED: &str = "${HOSTED}";`);
  });
});
