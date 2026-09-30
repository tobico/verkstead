//! The MCP servers declared for this installation, on the settings page: what
//! the card says of them, what the rows in its pane hold, and what a Save puts
//! on the wire.
//!
//! Two halves mounted apart, because that is what they are: a card in the middle
//! pane saying how many are declared, and the rows that rewrite them in the
//! details pane it opens.
//!
//! What these are mostly about is the two rules the name carries, because they
//! are what makes this section unlike the plain ones beside it. **A declared
//! server's name is not a field**: it is what a chip refers to and what the agent
//! sees in front of the server's tool names, so renaming one would be every chip
//! pointing at it losing what it pointed at. Only a row somebody has just added
//! offers a name to type. And **a save is refused** over a name that is not
//! lowercase letters, digits and hyphens or one another declaration already has —
//! which is the server's answer, drawn at the row and the box it names, with
//! everything the human typed left where they left it.
//!
//! The save sends the whole of the settings edit — the author as it stands, the
//! token untouched, the rules and the paths where the read left them — because
//! the server writes both files in one request, and that is what these check is
//! not lost.
//!
//! The read is a fixture the server's own tests wrote, so what the page is drawn
//! from is the shape the endpoint really answers with: `settings.json` is the
//! Verkstead that has been told everything, and `settings-unset.json` is the one
//! nobody has been to — which is the empty section a fresh installation draws.

import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  McpServer,
  ServerRefused,
  ServerTried,
  SettingsSaved,
  SettingsView,
} from "../src/api/types";
import {
  McpServersCard,
  McpServersPane,
} from "../src/settings/McpServers";
import styles from "../src/settings/McpServers.module.css";
import { json, serving, whenever } from "./serving";
import told from "./fixtures/settings.json" with { type: "json" };
import unset from "./fixtures/settings-unset.json" with { type: "json" };

const TOLD = told as SettingsView;
const UNSET = unset as SettingsView;

/// The first of the two declarations the told fixture carries, and the one
/// beside it: two, because the list is a list and a page that only ever drew
/// one would say nothing about the row below it.
const DECLARED = TOLD.mcp_servers[0]!;
const BESIDE = TOLD.mcp_servers[1]!;

/// The rest of `config.yaml` as every save from this pane sends it: what the read
/// said, left exactly where it was.
const REST = {
  git_author: TOLD.git_author,
  github_token: "Keep",
  // The rules ride along as an action rather than a value: nothing this pane
  // does says anything about them — see [`IgnoredCommentsEdit`].
  ignored_comments: "Keep",
  // And the languages, for the reason the rules do: one request writes the whole
  // of `config.yaml`, so a list left out would be a list emptied. See
  // [`heldLanguages`].
  languages: [
    { name: "rust", enabled: true, size: "50G" },
    { name: "gleam", enabled: true, size: "8G" },
  ],
  cleanup: {
    trim: { enabled: true, days: "5" },
    delete: { enabled: true, days: "90" },
  },
  conflict_resolution: TOLD.conflict_resolution,
  share_on_done: TOLD.share_on_done,
  sandbox_binds: ["/var/cache/verkstead-node", "/var/cache/verkstead-cargo"],
  instructions: TOLD.instructions,
};

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

function mountCard(open = false) {
  const press = vi.fn();
  return {
    ...mounting(() => <McpServersCard open={open} press={press} />),
    press,
  };
}

function mountPane() {
  const back = vi.fn();
  return { ...mounting(() => <McpServersPane back={back} />), back };
}

function theSettings(
  standing: SettingsView,
  ...answers: Array<() => Promise<Response>>
) {
  return serving(whenever("/api/ui/settings", json(standing)), ...answers);
}

/// The same settings with another list of declarations in them, which is what a
/// save that landed answers with.
function holding(
  standing: SettingsView,
  mcp_servers: McpServer[],
  tried: ServerTried[] = [],
): SettingsSaved {
  return {
    settings: { ...standing, mcp_servers },
    verified: null,
    refused: [],
    refused_servers: [],
    // What came of speaking to each of them, which every save that landed
    // carries — empty where the test is not about that.
    tried,
  };
}

/// One declaration reached, by the name it gave for itself.
function reached(server: string, named: string | null = null): ServerTried {
  return { server, outcome: { Reached: { named } } };
}

/// And one that was not, in the words it was refused in.
function unreached(server: string, why: string): ServerTried {
  return { server, outcome: { Refused: { why } } };
}

/// And what a save that was turned down answers with: nothing written, so the
/// settings are how they stood, and one row named.
function turnedDown(...refused_servers: ServerRefused[]): SettingsSaved {
  return {
    settings: TOLD,
    verified: null,
    refused: [],
    refused_servers,
    tried: [],
  };
}

function sent(fetching: ReturnType<typeof serving>): unknown {
  const written = fetching.mock.calls.find(
    ([asked, init]) =>
      String(asked) === "/api/ui/settings" && init?.method === "POST",
  );
  expect(written, "expected the page to have saved").toBeTruthy();
  return JSON.parse(String(written![1]?.body));
}

/// The name box of the row standing at `at`, which only a row somebody added
/// has.
function nameBox(at: number): HTMLInputElement | null {
  return document.querySelector<HTMLInputElement>(`#mcp-name-${at}`);
}

function urlBox(at: number): HTMLInputElement {
  const box = document.querySelector<HTMLInputElement>(`#mcp-url-${at}`);
  expect(box, `expected a URL box on row ${at}`).not.toBeNull();
  return box!;
}

function saveButton(): HTMLButtonElement {
  return screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
}

function save() {
  fireEvent.click(saveButton());
}

/// Wait for the press to be pressable again, which is what a second save has to
/// do first: the button is disabled while one is in flight, and a click on a
/// disabled button is a click that never happened.
async function pressable() {
  await waitFor(() => expect(saveButton().disabled).toBe(false));
}

/// The name box of the header standing at `which` on the row at `at`.
function headerBox(at: number, which: number): HTMLInputElement {
  const box = document.querySelector<HTMLInputElement>(
    `#mcp-header-${at}-${which}`,
  );
  expect(box, `expected a header box at ${at}.${which}`).not.toBeNull();
  return box!;
}

/// And the write-only box beside it, which never holds a value that came back.
function valueBox(at: number, which: number): HTMLInputElement {
  const box = document.querySelector<HTMLInputElement>(
    `#mcp-value-${at}-${which}`,
  );
  expect(box, `expected a value box at ${at}.${which}`).not.toBeNull();
  return box!;
}

/// One declaration as a save sends it with every value box untouched: each
/// header named and kept, which is what stops a corrected URL taking a key away.
function keeping(server: McpServer, url: string = server.url) {
  return {
    name: server.name,
    url,
    headers: server.headers.map((header) => ({
      name: header.name,
      value: "Keep",
    })),
  };
}

describe("the card", () => {
  /// What somebody scanning the page is after: how many are declared, and what
  /// that comes to for a Conversation.
  it("says how many are declared and what they are for", async () => {
    theSettings(TOLD);
    mountCard();

    const line = await waitFor(() => screen.getByText(/2 servers declared/));
    expect(line.textContent).toContain("Attach button");
  });

  /// And a Verkstead nobody has declared one on says so, rather than saying
  /// nothing: an empty section on a settings page reads as one that is broken.
  it("says so where nothing is declared", async () => {
    theSettings(UNSET);
    mountCard();

    const line = await waitFor(() => screen.getByText(/Nothing is declared/));
    expect(line.textContent).toContain("no server to attach");
  });

  it("opens the pane on a press", async () => {
    theSettings(TOLD);
    const { container, press } = mountCard();

    const face = await waitFor(() => {
      const drawn = container.querySelector<HTMLElement>(
        `.${styles.serversCard}`,
      );
      expect(drawn, "expected the card to be drawn").not.toBeNull();
      return drawn!;
    });

    fireEvent.click(face);
    expect(press).toHaveBeenCalled();
  });
});

describe("the pane", () => {
  it("draws a row for each declaration, with its URL in a field", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => expect(urlBox(0).value).toBe(DECLARED.url));
    expect(screen.getByText(DECLARED.name)).toBeTruthy();

    expect(urlBox(1).value).toBe(BESIDE.url);
    expect(screen.getByText(BESIDE.name)).toBeTruthy();
  });

  /// The rule this section is built around: the name of a declaration that is
  /// already made is words rather than a box, because everything that refers to
  /// a server refers to it by that name.
  it("offers no name field on a declaration already made", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => urlBox(0));
    expect(nameBox(0)).toBeNull();
  });

  /// And a fresh installation draws the section rather than failing at it.
  it("says the list is empty where nothing is declared", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    expect(screen.queryByText(/Could not read the settings/)).toBeNull();
  });

  /// What the section is for, said where somebody is about to type into it —
  /// including the one rule that cannot be undone later.
  it("says what a declaration is and that a name is fixed", async () => {
    theSettings(TOLD);
    mountPane();

    const note = await waitFor(() =>
      screen.getByText(/Declared here for the whole installation/),
    );
    expect(note.textContent).toContain("over HTTP");
    expect(note.textContent).toContain("cannot be changed afterwards");
  });

  /// Adding one is the only moment a name is chosen, so it is the one row with a
  /// name field on it.
  it("offers a name on a row somebody has just added", async () => {
    theSettings(UNSET);
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));

    await waitFor(() => expect(nameBox(0)).not.toBeNull());
    expect(nameBox(0)!.value).toBe("");
    expect(urlBox(0).value).toBe("");
  });

  it("sends the declaration that was added, with the rest of the file", async () => {
    const declared = {
      name: "tickets",
      url: "https://mcp.example.com/tickets",
      headers: [],
    };
    const fetching = theSettings(UNSET, json(holding(UNSET, [declared])));
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(0)).not.toBeNull());

    fireEvent.input(nameBox(0)!, { target: { value: declared.name } });
    fireEvent.input(urlBox(0), { target: { value: declared.url } });
    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        // The read this pane sent from is the one nobody has been to, so what
        // rides along is that file rather than the told one.
        git_author: UNSET.git_author,
        languages: [{ name: "rust", enabled: true, size: "" }],
        cleanup: {
          trim: { enabled: true, days: "" },
          delete: { enabled: false, days: "" },
        },
        conflict_resolution: UNSET.conflict_resolution,
        share_on_done: UNSET.share_on_done,
        sandbox_binds: [],
        instructions: "",
        mcp_servers: { Set: { servers: [declared] } },
      }),
    );
  });

  /// The URL is the half that is edited, and editing it sends the whole list
  /// with that one changed: the server writes the declarations as one list.
  it("sends the whole list with a rewritten URL in it", async () => {
    const url = "https://docs.example.com/mcp";
    const moved = { ...DECLARED, url };
    const fetching = theSettings(TOLD, json(holding(TOLD, [moved, BESIDE])));
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: url } });
    save();

    // The one that was typed in, and the one beside it exactly as it was read:
    // what travels is the list as it is to stand. Every header of both of them
    // is kept, which is the whole point of a value box that is blank until
    // somebody types in it: correcting a URL does not take a key away.
    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        mcp_servers: {
          Set: { servers: [keeping(DECLARED, url), keeping(BESIDE)] },
        },
      }),
    );
  });

  /// Remove is how one is taken off, and an empty list is the last one taken
  /// away rather than nothing said.
  it("sends an empty list where the last declaration was removed", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, [])));
    mountPane();
    await waitFor(() => urlBox(0));

    // Every one of them, because what this is about is the list arriving empty
    // rather than the list arriving shorter: an empty one is the human having
    // taken the last declaration away, and it has to be told apart from a save
    // that says nothing about them at all.
    // One at a time and read again each time: a press takes the row it was on
    // off the page, so a handful of them collected first would be presses on
    // elements the page no longer holds.
    for (let left = TOLD.mcp_servers.length; left > 0; left -= 1) {
      fireEvent.click(screen.getAllByRole("button", { name: "Remove" })[0]!);
    }

    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        mcp_servers: { Set: { servers: [] } },
      }),
    );
  });

  /// A save that has not touched a row says nothing about the declarations at
  /// all, which is what keeps it one the server cannot turn down.
  it("says Keep where nobody has touched a row", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({ ...REST, mcp_servers: "Keep" }),
    );
  });

  /// A row somebody added and never filled in is not a declaration, so it comes
  /// off the page as the save goes out — which is what stops it being sent and
  /// refused.
  it("drops a row nobody wrote anything in as the save goes out", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(TOLD.mcp_servers.length)).not.toBeNull());

    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        mcp_servers: { Set: { servers: TOLD.mcp_servers.map((s) => keeping(s)) } },
      }),
    );
  });

  /// The refusal, drawn at the box it names with what was typed left where it
  /// was: the whole request was turned down, so nothing is read back.
  it("draws a refused name at its own row and keeps what was typed", async () => {
    const fetching = theSettings(
      UNSET,
      json(
        turnedDown({
          server: 0,
          field: "Name",
          why: "a name is lowercase letters, digits and hyphens",
        }),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(0)).not.toBeNull());

    fireEvent.input(nameBox(0)!, { target: { value: "Docs Server" } });
    fireEvent.input(urlBox(0), { target: { value: "https://example.com" } });
    save();

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    await waitFor(() =>
      screen.getByText(/a name is lowercase letters, digits and hyphens/),
    );

    // Still on the page, and still a field: a refused row is one the human is
    // about to correct.
    expect(nameBox(0)!.value).toBe("Docs Server");
    expect(urlBox(0).value).toBe("https://example.com");
  });

  /// And a refusal about the other half is drawn at the other box.
  it("draws a refused URL at its own row", async () => {
    const fetching = theSettings(
      UNSET,
      json(
        turnedDown({
          server: 0,
          field: "Url",
          why: "a server is reached over HTTP, so it needs a URL",
        }),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(0)).not.toBeNull());

    fireEvent.input(nameBox(0)!, { target: { value: "docs" } });
    save();

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    await waitFor(() => screen.getByText(/so it needs a URL/));
  });

  /// A refusal names a row by where it stood, so a list that has gained or lost
  /// one is a list those numbers no longer point into.
  it("drops the refusals when a row is added or removed", async () => {
    theSettings(
      UNSET,
      json(
        turnedDown({
          server: 0,
          field: "Name",
          why: "a name is lowercase letters, digits and hyphens",
        }),
      ),
    );
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(0)).not.toBeNull());

    fireEvent.input(nameBox(0)!, { target: { value: "Docs" } });
    save();

    await waitFor(() =>
      screen.getByText(/a name is lowercase letters, digits and hyphens/),
    );

    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));

    await waitFor(() =>
      expect(
        screen.queryByText(/a name is lowercase letters, digits and hyphens/),
      ).toBeNull(),
    );
  });

  /// And the rows follow the server again once a save lands: what was new is a
  /// declaration now, so its name stops being a field.
  it("lets go of what was typed once the save lands", async () => {
    const declared = {
      name: "tickets",
      url: "https://mcp.example.com/tickets",
      headers: [],
    };
    const fetching = theSettings(UNSET, json(holding(UNSET, [declared])));
    mountPane();

    await waitFor(() => screen.getByText("No servers are declared."));
    fireEvent.click(screen.getByRole("button", { name: "Add a server" }));
    await waitFor(() => expect(nameBox(0)).not.toBeNull());

    fireEvent.input(nameBox(0)!, { target: { value: declared.name } });
    fireEvent.input(urlBox(0), { target: { value: declared.url } });
    save();

    await waitFor(() => expect(sent(fetching)).toBeTruthy());
    await waitFor(() => expect(nameBox(0)).toBeNull());
    expect(screen.getByText(declared.name)).toBeTruthy();
    expect(urlBox(0).value).toBe(declared.url);
  });

  /// A save that would not land says so and leaves what was typed where it is.
  it("says a save that failed, and keeps what was typed", async () => {
    theSettings(TOLD, () =>
      Promise.resolve(new Response("nope", { status: 503 })),
    );
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://moved.example" } });
    save();

    await waitFor(() => screen.getByText(/The settings could not be saved/));
    expect(urlBox(0).value).toBe("https://moved.example");
  });

  /// A header is drawn by name with an empty box beside it, whatever is kept for
  /// it: a value goes in and never comes back, so what the box says in place of
  /// one is whether there is one.
  it("draws each header by name and never a value", async () => {
    theSettings(TOLD);
    mountPane();

    await waitFor(() => urlBox(0));

    expect(headerBox(0, 0).value).toBe(DECLARED.headers[0]!.name);
    expect(valueBox(0, 0).value).toBe("");
    expect(valueBox(0, 0).placeholder).toContain("Kept");
    expect(screen.getByText(/A value is kept for it/)).toBeTruthy();

    // And the one beside it, which the declaration names with nothing kept to
    // send in it.
    expect(headerBox(0, 1).value).toBe(DECLARED.headers[1]!.name);
    expect(valueBox(0, 1).placeholder).toContain("Nothing is sent");

    // A declaration with no headers says so rather than drawing nothing.
    expect(screen.getByText(/None, so requests to it carry nothing/)).toBeTruthy();
  });

  /// The three actions, in one save: a value typed is a `Set`, a box left alone
  /// is a `Keep`, and one cleared is a `Clear`.
  it("sends a typed value, an untouched one and a cleared one apart", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(valueBox(0, 0), {
      target: { value: "Bearer sk-averysecretkey" },
    });
    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        mcp_servers: {
          Set: {
            servers: [
              {
                name: DECLARED.name,
                url: DECLARED.url,
                headers: [
                  {
                    name: DECLARED.headers[0]!.name,
                    value: { Set: { value: "Bearer sk-averysecretkey" } },
                  },
                  { name: DECLARED.headers[1]!.name, value: "Keep" },
                ],
              },
              keeping(BESIDE),
            ],
          },
        },
      }),
    );
  });

  /// And the one thing a blank box cannot say, which is why it is a press:
  /// taking a value away is asked for rather than typed.
  it("sends Clear where the value was cleared", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.click(screen.getByRole("button", { name: "Clear its value" }));

    // And the row says what is about to happen to it, the press having no other
    // effect until the save goes out.
    await waitFor(() => screen.getByText(/will be taken away when this is saved/));

    save();

    await waitFor(() =>
      expect(sent(fetching)).toEqual({
        ...REST,
        mcp_servers: {
          Set: {
            servers: [
              {
                name: DECLARED.name,
                url: DECLARED.url,
                headers: [
                  { name: DECLARED.headers[0]!.name, value: "Clear" },
                  { name: DECLARED.headers[1]!.name, value: "Keep" },
                ],
              },
              keeping(BESIDE),
            ],
          },
        },
      }),
    );
  });

  /// A clearing thought better of, which is the press again — and typing a value
  /// settles it too, because typing is unambiguous.
  it("takes a clearing back when its value is typed again", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.click(screen.getByRole("button", { name: "Clear its value" }));
    await waitFor(() => screen.getByText(/will be taken away/));

    fireEvent.input(valueBox(0, 0), { target: { value: "Bearer sk-again" } });
    save();

    await waitFor(() => {
      const written = sent(fetching) as {
        mcp_servers: { Set: { servers: Array<{ headers: unknown[] }> } };
      };

      expect(written.mcp_servers.Set.servers[0]!.headers[0]).toEqual({
        name: DECLARED.headers[0]!.name,
        value: { Set: { value: "Bearer sk-again" } },
      });
    });
  });

  /// A header added is a name with a value to set, and one removed is a header
  /// the declaration no longer names — the list travelling as the list, the way
  /// the declarations themselves do.
  it("sends a header added and leaves out one removed", async () => {
    const fetching = theSettings(TOLD, json(holding(TOLD, TOLD.mcp_servers)));
    mountPane();
    await waitFor(() => urlBox(0));

    // The second of the two, so that what is left is the first rather than
    // whatever the page happened to draw last.
    fireEvent.click(
      screen.getAllByRole("button", { name: "Remove header" })[1]!,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Add a header" })[0]!);
    fireEvent.input(headerBox(0, 1), { target: { value: "X-Api-Key" } });
    fireEvent.input(valueBox(0, 1), { target: { value: "sk-anotherkey" } });

    save();

    await waitFor(() => {
      const written = sent(fetching) as {
        mcp_servers: { Set: { servers: Array<{ headers: unknown[] }> } };
      };

      expect(written.mcp_servers.Set.servers[0]!.headers).toEqual([
        { name: DECLARED.headers[0]!.name, value: "Keep" },
        { name: "X-Api-Key", value: { Set: { value: "sk-anotherkey" } } },
      ]);
    });
  });

  /// A header renamed is a header nothing is kept for, whatever was kept for the
  /// name it used to have: the box says so rather than going on claiming a value
  /// that belongs to a name the declaration no longer carries.
  it("stops claiming a kept value once the header is renamed", async () => {
    theSettings(TOLD);
    mountPane();
    await waitFor(() => urlBox(0));

    expect(valueBox(0, 0).placeholder).toContain("Kept");

    fireEvent.input(headerBox(0, 0), { target: { value: "X-Api-Key" } });

    await waitFor(() =>
      expect(valueBox(0, 0).placeholder).toContain("Nothing is sent"),
    );
  });

  /// And what the section says about all of it, where somebody is about to type
  /// a key into it.
  it("says a header value is kept the way the token is", async () => {
    theSettings(TOLD);
    mountPane();

    const note = await waitFor(() =>
      screen.getByText(/Declared here for the whole installation/),
    );

    expect(note.textContent).toContain("never shown again");
    expect(note.textContent).toContain("keeps what is there");
  });
});

/// What came of speaking to each declaration as it was saved, which the section
/// says beside the server the way the Git pane says who a token authenticates
/// as.
///
/// The thing worth proving here is that it is a *report* rather than a refusal:
/// a server that would not answer is drawn as one that was saved and not
/// reached, with the rows following the server exactly as they do after any
/// save that landed. The words themselves are the server's — what these check
/// is that the row is the one they are drawn at, and that they go when the next
/// press does.
describe("what came of trying them", () => {
  /// The whole of what a reached server says about itself, which is the name it
  /// gives — not the name it is declared under.
  it("says a server answered, by the name it gave", async () => {
    theSettings(
      TOLD,
      json(holding(TOLD, TOLD.mcp_servers, [reached(DECLARED.name, "Docs MCP")])),
    );
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();

    const line = await waitFor(() => screen.getByText(/It answered when it was saved/));
    expect(line.textContent).toContain("Docs MCP");
  });

  /// And one that named itself nothing is still a server that answered, said in
  /// fewer words.
  it("says one that named itself nothing answered all the same", async () => {
    theSettings(
      TOLD,
      json(holding(TOLD, TOLD.mcp_servers, [reached(DECLARED.name)])),
    );
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();

    const line = await waitFor(() => screen.getByText(/It answered when it was saved/));
    expect(line.textContent).not.toContain("calling itself");
  });

  /// A server that would not answer is a report and not a refusal: the words say
  /// it is declared, and the rows have gone back to following the server, which
  /// is what every save that landed does.
  it("says one that was not reached is declared all the same", async () => {
    theSettings(
      TOLD,
      json(
        holding(TOLD, TOLD.mcp_servers, [
          unreached(DECLARED.name, "It did not answer within 5 seconds."),
        ]),
      ),
    );
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();

    const line = await waitFor(() =>
      screen.getByText(/It is declared, but it was not reached/),
    );
    expect(line.textContent).toContain("It did not answer within 5 seconds.");

    // And the row follows the server again, which is what says nothing was
    // turned down: what is drawn is what was written rather than what was typed.
    await waitFor(() => expect(urlBox(0).value).toBe(DECLARED.url));
  });

  /// Each line is drawn at the row it is about, which is the whole point of
  /// saying it beside the server rather than under the button.
  it("draws each answer at the row it is about", async () => {
    theSettings(
      TOLD,
      json(
        holding(TOLD, TOLD.mcp_servers, [
          reached(DECLARED.name, "Docs MCP"),
          unreached(BESIDE.name, "It did not answer: connection refused."),
        ]),
      ),
    );
    const { container } = mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();

    await waitFor(() =>
      expect(container.querySelectorAll(`.${styles.reached}`).length).toBe(1),
    );

    const rows = container.querySelectorAll(`.${styles.row}`);

    expect(rows[0]!.querySelector(`.${styles.reached}`)).not.toBeNull();
    expect(rows[0]!.querySelector(`.${styles.unreached}`)).toBeNull();
    expect(rows[1]!.querySelector(`.${styles.unreached}`)).not.toBeNull();
  });

  /// And nothing is said before a save has spoken to them: the section is drawn
  /// from a read, and a read says nothing about whether anything answers.
  it("says nothing about a declaration no save has tried", async () => {
    theSettings(TOLD);
    const { container } = mountPane();

    await waitFor(() => urlBox(0));

    expect(container.querySelector(`.${styles.reached}`)).toBeNull();
    expect(container.querySelector(`.${styles.unreached}`)).toBeNull();
  });

  /// And what the last press learned goes with the next one: the answer to this
  /// save says what these declarations are, and a line left standing would be
  /// about the ones before them.
  it("drops what the last save learned as the next goes out", async () => {
    const fetching = theSettings(
      TOLD,
      json(holding(TOLD, TOLD.mcp_servers, [reached(DECLARED.name, "Docs MCP")])),
      // The second press is answered without the page ever being told what came
      // of it, which is the save still in flight.
      () => new Promise<Response>(() => {}),
    );
    mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();
    await waitFor(() => screen.getByText(/It answered when it was saved/));
    await pressable();

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/other" } });
    save();

    await waitFor(() =>
      expect(screen.queryByText(/It answered when it was saved/)).toBeNull(),
    );

    expect(fetching).toHaveBeenCalled();
  });

  /// And a save that was turned down says nothing either: nothing was written,
  /// so there was no declaration to speak to.
  it("says nothing where the save was turned down", async () => {
    theSettings(
      TOLD,
      json(turnedDown({ server: 0, field: "Name", why: "that is taken" })),
    );
    const { container } = mountPane();
    await waitFor(() => urlBox(0));

    fireEvent.input(urlBox(0), { target: { value: "https://docs.internal/mcp" } });
    save();

    await waitFor(() => screen.getByText("that is taken"));

    expect(container.querySelector(`.${styles.reached}`)).toBeNull();
    expect(container.querySelector(`.${styles.unreached}`)).toBeNull();
  });
});
