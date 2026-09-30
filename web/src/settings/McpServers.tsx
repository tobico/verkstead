//! The MCP servers declared for this installation, on the settings page: a name
//! and a URL each, said once here and attached to a Conversation from the Attach
//! button where its Brief is written.
//!
//! A session runs with no MCP servers at all today, on purpose: a Built Root has
//! the account's own taken out of it for every harness, the way it has the
//! account's plugins and hooks. This section is the one deliberate way in — a
//! server nobody's home directory happened to hold, said in Verkstead's own
//! `config.yaml`, and given to a session only where its Conversation asked for
//! it. See ADR-0021, and **MCP server** in `CONTEXT.md`.
//!
//! **Declared once for the installation, attached per Conversation.** What is
//! here is the declaration and nothing else: a Conversation refers to one by
//! name, and nothing in this pane knows or cares which Conversations have.
//!
//! **HTTP only.** There is no command, no arguments and no transport to choose
//! on this form. A stdio server is a child process the agent starts inside its
//! own sandbox, which is a hole in the sandbox rather than a setting, and it was
//! turned down in the grilling this was settled in.
//!
//! **Static headers are the whole of the authentication**, OAuth having been
//! turned down there too: there is no browser at three in the morning. So a
//! declaration carries headers, and **every value is a secret** — it goes where
//! the GitHub token goes, and the page is never shown one again. What comes back
//! about a header is its name and whether anything is kept to send in it, which
//! is exactly what comes back about the token.
//!
//! Which is why a header travels as an *action* rather than as a value, one per
//! header: keep what is there, set a new one, or clear it. **A value box left
//! blank keeps what is there** — a page that read an empty write-only box as
//! *clear this* would take a key away every time somebody corrected a URL, which
//! is the token's rule said once per header.
//!
//! **The name is the identity, and a server is never renamed.** It is what a
//! chip refers to and what the agent sees in front of the server's tool names,
//! so it is lowercase letters, digits and hyphens and unique among the
//! declarations. The form offers a name field on a *new* row alone: a declared
//! one draws its name as words, because renaming one would be every chip
//! pointing at it losing what it pointed at. Changing a name is removing the
//! declaration and making another. The URL is the half that is edited.
//!
//! Two halves in two panes, like every section beside it: a card in the middle
//! pane saying how many are declared, and the rows that rewrite them in the
//! details pane it opens, at `/settings/mcp-servers`. Both read the one settings
//! query the rest of the page reads, and the save goes through the one settings
//! endpoint — so the author, the token and the rest ride along as they stand.
//!
//! The declarations travel as an action rather than a value, the way the ignore
//! rules do and for the same reason: a save is *refused* over a name that is not
//! a name or one another declaration already has. So the rows are sent only once
//! somebody has touched one, and a save about an email address is one the server
//! cannot turn down over a name it never showed anybody.
//!
//! **A row nobody wrote anything in is not a declaration**, and comes off the
//! page as the save goes out — which is how one is deleted where Remove is not
//! pressed, and what keeps a row somebody added and left from turning down the
//! save it rode along with.
//!
//! A refusal is the whole request refused: nothing is written, and what comes
//! back names the row and the box it is about. So it is drawn at that row, with
//! everything the human typed left where they left it.
//!
//! **And a save that lands speaks to every declaration it wrote down**, once,
//! and each row says what came of that under its URL — the way the Git pane
//! says who the token it just saved authenticates as. It is a report rather
//! than a refusal: a server that would not answer is drawn as one that is
//! declared and was not reached, because it may be reachable from inside a
//! session's network and never from here. Nothing is tried when a server is
//! attached or when a session launches, so this is the only place the page ever
//! hears whether one answers.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Index, Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import type {
  HeaderEdit,
  McpServersEdit,
  ServerField,
  ServerRefused,
  ServerTried,
  SettingsSaved,
  SettingsView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldConfig } from "./held";
import styles from "./McpServers.module.css";

/// What the section is called, in the one place both panes read it from.
const TITLE = "MCP servers";

/// One row of the pane as it is being edited: a declaration, and whether it is
/// one the server already holds.
///
/// That second half is what decides whether the name is a field or words. It is
/// carried on the row rather than worked out by looking the name up in what was
/// read, because a row added and not yet saved has no name to look up — and a
/// name typed to match one already declared would otherwise stop being editable
/// under the hand that was typing it.
type Editing = {
  name: string;
  url: string;
  fresh: boolean;
  headers: EditingHeader[];
};

/// And one header of one: its name, what has been typed into its value box, and
/// what the server said about the value it already holds.
///
/// `declared` is the name this header was read back under, or `null` on one
/// somebody has just added. What it is for is knowing whether `set` is still
/// about *this* header: a name rewritten is a header the declaration no longer
/// names, so whatever was kept under the old one is gone and the box has nothing
/// to keep.
///
/// `cleared` is the human having asked for the value to be taken away, which is
/// the one thing a blank box cannot say — blank is *keep*, so clearing is a
/// press of its own.
type EditingHeader = {
  name: string;
  value: string;
  declared: string | null;
  set: boolean;
  cleared: boolean;
};

/// Whether anything is kept to send in this header, which is what its value box
/// says in place of a value.
///
/// Only while the name is the one it was read back under: a header renamed is a
/// header nothing is kept for, whatever was kept for the name it used to have.
function kept(header: EditingHeader): boolean {
  return header.set && header.name.trim() === header.declared;
}

/// And what is to become of its value, which is the whole of what a save says
/// about one.
///
/// Typed wins, because typing is unambiguous. A blank box is `Keep` — the
/// token's rule, said once per header — unless the human pressed the press that
/// says otherwise.
function becomes(header: EditingHeader): HeaderEdit {
  if (header.value !== "") {
    return { Set: { value: header.value } };
  }

  return header.cleared ? "Clear" : "Keep";
}

/// A header as it is drawn when the pane first reads one off the server: the
/// name it is declared under, and no value, because a value never comes back.
function reading(header: { name: string; set: boolean }): EditingHeader {
  return {
    name: header.name,
    value: "",
    declared: header.name,
    set: header.set,
    cleared: false,
  };
}

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// A count with the word it counts, so a line reads as English rather than as
/// `1 servers`.
function counted(many: number, one: string, more: string): string {
  return `${many} ${many === 1 ? one : more}`;
}

/// The declarations as they stand, as the card that opens the section.
///
/// What is on the card is what somebody scanning the page is after: how many are
/// declared, and what that comes to for a Conversation. The names are behind the
/// press — a card is read past rather than read.
export function McpServersCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.serversCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            <p class={styles.standing}>
              <Show
                when={told().mcp_servers.length > 0}
                fallback={
                  <>
                    Nothing is declared, so a conversation has no server to
                    attach.
                  </>
                }
              >
                {counted(told().mcp_servers.length, "server", "servers")}{" "}
                declared, to attach to a conversation from its Attach button.
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the rows that rewrite them, which is the details pane the card opens.
///
/// One Save under the lot rather than a press per row: the server writes the
/// declarations as one list, so a row edited is the list sent, and a per-row
/// press would be sending the others along without saying so. There is no
/// Cancel — a details pane is left by opening something else or by the way back
/// a narrow window draws, and what was typed and never saved goes with it.
export function McpServersPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // The declarations as they are being edited, or `null` while nobody has
  // touched a row — the rows follow the server until then, the way every field
  // on this page does. `null` is also what says the save has nothing to say
  // about them, which is what keeps a save from another section one that cannot
  // be refused: see [`McpServersEdit`].
  const [rows, setRows] = createSignal<Editing[] | null>(null);

  // And what the last save was turned down over, by the row and the box it
  // named. Emptied as the next save goes out, because it describes what was
  // sent rather than what is on the page.
  const [refusals, setRefusals] = createSignal<ServerRefused[]>([]);

  // And what came of speaking to each declaration the last save wrote down,
  // by name. Emptied as the next save goes out, for the reason the refusals
  // above are: it describes what was saved rather than what is on the page.
  //
  // By name rather than by row, unlike those: a try only ever happens on a save
  // that landed, so every one of these is about a declaration with a name that
  // is a name — and the rows go back to following the server the moment one
  // lands, so the row a name is on is the row the server declared it on.
  const [tries, setTries] = createSignal<ServerTried[]>([]);

  const told = (): SettingsView | undefined => settings.data;

  /// Everything else in `config.yaml` this pane is not about, as it stands — see
  /// [`heldConfig`]. A save writes the whole file, so a section left out would
  /// be a section emptied.
  const held = () => heldConfig(told());

  /// The rows as they are drawn: what has been edited, or what is declared while
  /// nothing has been.
  const drawn = (): Editing[] =>
    rows() ??
    (told()?.mcp_servers ?? []).map((server) => ({
      name: server.name,
      url: server.url,
      fresh: false,
      headers: server.headers.map(reading),
    }));

  /// One box rewritten, which is what makes the next save a `Set` of the whole
  /// list — the server writes the declarations as one list, so a row edited is
  /// the list sent.
  const rewrite = (at: number, part: Partial<Editing>) =>
    setRows(
      drawn().map((row, which) => (which === at ? { ...row, ...part } : row)),
    );

  /// The headers of the row at `at`, rewritten by `change` — which is
  /// [`rewrite`] said a level down, and how each of the three presses below
  /// changes one.
  const rewriteHeaders = (
    at: number,
    change: (headers: EditingHeader[]) => EditingHeader[],
  ) =>
    setRows(
      drawn().map((row, which) =>
        which === at ? { ...row, headers: change(row.headers) } : row,
      ),
    );

  /// One header's own box rewritten.
  const rewriteHeader = (
    at: number,
    which: number,
    part: Partial<EditingHeader>,
  ) =>
    rewriteHeaders(at, (headers) =>
      headers.map((header, index) =>
        index === which ? { ...header, ...part } : header,
      ),
    );

  /// A header added to a row, empty, for a name and a value to be typed into.
  const addHeader = (at: number) => {
    setRefusals([]);
    rewriteHeaders(at, (headers) => [
      ...headers,
      { name: "", value: "", declared: null, set: false, cleared: false },
    ]);
  };

  /// And one taken off, which is the declaration no longer naming it — and, the
  /// next time this is saved, the value kept for it gone with it.
  const removeHeader = (at: number, which: number) => {
    setRefusals([]);
    rewriteHeaders(at, (headers) =>
      headers.filter((_, index) => index !== which),
    );
  };

  /// A row added, empty, for the human to write a declaration into — and the one
  /// row on the page whose name is a field, this being the only moment a name is
  /// chosen.
  ///
  /// Its errors are dropped with it and with every other row's: a refusal names
  /// a row by where it stood, so a list that has gained or lost one is a list
  /// they no longer point into. Typing in a box leaves them, because the rows
  /// are still the rows the server was talking about.
  const add = () => {
    setRefusals([]);
    setRows([...drawn(), { name: "", url: "", fresh: true, headers: [] }]);
  };

  const remove = (at: number) => {
    setRefusals([]);
    setRows(drawn().filter((_, which) => which !== at));
  };

  /// Why the row standing at `at` was turned down, against one of its boxes —
  /// `null` where the save was not refused over that half of it.
  const trouble = (at: number, field: ServerField): string | null =>
    refusals().find((was) => was.server === at && was.field === field)?.why ??
    null;

  /// And what came of speaking to the row standing at `at` when it was last
  /// saved — `null` on a row no save has spoken to, which is every row until
  /// one lands and every row of a save that was turned down.
  const tried = (at: number): ServerTried["outcome"] | null => {
    const row = drawn()[at];

    return (
      tries().find((was) => was.server === row?.name.trim())?.outcome ?? null
    );
  };

  /// And that answer read the two ways the row draws it: what a server that
  /// answered said about itself, and the words one that did not was refused in
  /// — one of the two at a time, and neither until a save has spoken to it.
  const reached = (at: number): { named: string | null } | null => {
    const outcome = tried(at);

    return outcome && "Reached" in outcome ? outcome.Reached : null;
  };

  const unreached = (at: number): string | null => {
    const outcome = tried(at);

    return outcome && "Refused" in outcome ? outcome.Refused.why : null;
  };

  /// Whether a row is a declaration at all, which is whether anything was
  /// written in either box. Emptiness is decided the way the server decides it —
  /// by what is left after the spaces — so the two halves agree about which rows
  /// are declarations.
  const written = (row: Editing): boolean =>
    row.name.trim() !== "" || row.url.trim() !== "";

  /// The rows nobody wrote anything in, taken off the page as the save goes out
  /// — which is how a declaration is deleted without pressing Remove, and what
  /// stops a row somebody added and never filled in refusing the save it rode
  /// along with.
  ///
  /// Taken off the page rather than merely left out of the request, so that what
  /// is drawn and what was sent are one list: a refusal names a declaration by
  /// where it stood in what was sent, and a request that quietly left rows out
  /// would have those numbers pointing at the wrong boxes.
  const tidied = () => {
    const edited = rows();

    if (edited !== null) {
      setRows(edited.filter(written));
    }
  };

  /// What the save says about the declarations: nothing at all until somebody
  /// has touched a row, and the whole list in the order it is drawn once they
  /// have — the blank rows having been taken off it by [`tidied`] first.
  ///
  /// The row's own `fresh` flag does not travel. It is how this pane knows which
  /// name to draw as a field, and the server's business is the list as it is to
  /// stand.
  const edit = (): McpServersEdit => {
    const edited = rows();

    return edited === null
      ? "Keep"
      : {
          Set: {
            servers: edited.map(({ name, url, headers }) => ({
              name,
              url,
              // Each header by name, with what is to become of the value sent
              // in it: the names are what the declaration carries, and the
              // values go where a secret goes. A blank one nobody typed into is
              // a `Keep`.
              headers: headers.map((header) => ({
                name: header.name,
                value: becomes(header),
              })),
            })),
          },
        };
  };

  const save = useMutation(() => ({
    mutationFn: () =>
      saveSettings({
        git_author: told()?.git_author ?? { name: "", email: "" },
        // Untouched. This pane has no business with the credentials, and a blank
        // token field read as *clear this* is exactly what `Keep` is here to
        // stop.
        github_token: "Keep",
        share_on_done: told()?.share_on_done ?? false,
        // And the ignore rules left exactly where they are: this pane has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody.
        ignored_comments: "Keep",
        // And the one thing this pane is about — see [`edit`].
        mcp_servers: edit(),
        ...held(),
      }),
    onSuccess: (saved: SettingsSaved) => {
      // A refusal is the whole save refused — nothing was written — so nothing
      // here is spent and nothing is read back: what the human typed stays in
      // front of them, with the errors drawn at the rows they name.
      if (saved.refused_servers.length > 0) {
        setRefusals(saved.refused_servers);
        return;
      }

      // The rows go back to following the server, which has just been told what
      // they said: what is drawn from here on is what was written down, and a
      // row that was new is a declaration now.
      setRows(null);
      setRefusals([]);

      // And what came of speaking to each of them, which is what the rows say
      // beside themselves until the next save.
      setTries(saved.tried);

      // Taken from the answer rather than asked for again: what a save comes
      // back with *is* a fresh read of the two files, so a second read would
      // learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();

    // Whatever the last save was turned down over went with the press that is
    // being made now: the answer to this one says what is wrong with what is
    // being sent this time.
    setRefusals([]);

    // And so did what the last one made of speaking to them, for the same
    // reason: the answer to this press says what these declarations are, and a
    // line left standing would be about the ones before them.
    setTries([]);

    // And a row nobody wrote anything in is not a declaration, so it goes before
    // the list is read off the page — see [`tidied`].
    tidied();

    save.mutate();
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={told()}>
          <form class={styles.servers} onSubmit={commit}>
            {/* What the section is for, in the one line a pane on this page says
                its own in. Both halves of it: what a declaration is, and what
                has to happen before a session sees one. */}
            <Note>
              Declared here for the whole installation, over HTTP, and given to a
              session only where its conversation has attached one. A name is
              lowercase letters, digits and hyphens, and cannot be changed
              afterwards. A header value is kept the way the GitHub token is: it
              is never shown again, and a value box left blank keeps what is
              there.
            </Note>

            <Show
              when={drawn().length > 0}
              fallback={<Empty>No servers are declared.</Empty>}
            >
              <ul class={styles.rows}>
                <Index each={drawn()}>
                  {(row, at) => (
                    <li class={styles.row}>
                      {/* The name is a field on a new row and words on a
                          declared one: a server is referred to by name, so
                          renaming one is deleting it and declaring another. */}
                      <Show
                        when={row().fresh}
                        fallback={
                          <p class={styles.name}>
                            <code>{row().name}</code>
                          </p>
                        }
                      >
                        <label for={`mcp-name-${at}`}>Name</label>
                        <input
                          id={`mcp-name-${at}`}
                          type="text"
                          autocapitalize="off"
                          autocorrect="off"
                          spellcheck={false}
                          placeholder="docs"
                          value={row().name}
                          onInput={(ev) =>
                            rewrite(at, { name: ev.currentTarget.value })
                          }
                        />
                      </Show>
                      <Show when={trouble(at, "Name")}>
                        {(why) => (
                          <ErrorLine class={styles.trouble}>{why()}</ErrorLine>
                        )}
                      </Show>

                      <label for={`mcp-url-${at}`}>URL</label>
                      <input
                        id={`mcp-url-${at}`}
                        type="url"
                        inputmode="url"
                        autocapitalize="off"
                        autocorrect="off"
                        spellcheck={false}
                        placeholder="https://mcp.example.com/docs"
                        value={row().url}
                        onInput={(ev) =>
                          rewrite(at, { url: ev.currentTarget.value })
                        }
                      />
                      <Show when={trouble(at, "Url")}>
                        {(why) => (
                          <ErrorLine class={styles.trouble}>{why()}</ErrorLine>
                        )}
                      </Show>

                      {/* And what came of speaking to it when it was saved,
                          under the box it is about: a declaration is tried
                          once, here, and it is saved either way — a server
                          that cannot be reached from this machine may be
                          reachable from inside a session's network. */}
                      <Show when={reached(at)}>
                        {(answered) => (
                          <p class={styles.reached}>
                            <Show
                              when={answered().named}
                              fallback={<>It answered when it was saved.</>}
                            >
                              {(named) => (
                                <>
                                  It answered when it was saved, calling itself{" "}
                                  <span class={styles.calls}>{named()}</span>.
                                </>
                              )}
                            </Show>
                          </p>
                        )}
                      </Show>
                      <Show when={unreached(at)}>
                        {(why) => (
                          <ErrorLine class={styles.unreached}>
                            It is declared, but it was not reached when it was
                            saved. {why()}
                          </ErrorLine>
                        )}
                      </Show>

                      {/* And the headers it is spoken to with: an API key or a
                          bearer token, which is the whole of the
                          authentication there is. A value goes in and never
                          comes back, so the box beside a header that has one
                          says so and is empty. */}
                      <p class={styles.headersHead}>Headers</p>

                      <Show
                        when={row().headers.length > 0}
                        fallback={
                          <p class={styles.none}>
                            None, so requests to it carry nothing.
                          </p>
                        }
                      >
                        <ul class={styles.headers}>
                          <Index each={row().headers}>
                            {(header, which) => (
                              <li class={styles.header}>
                                <label for={`mcp-header-${at}-${which}`}>
                                  Name
                                </label>
                                <input
                                  id={`mcp-header-${at}-${which}`}
                                  type="text"
                                  autocapitalize="off"
                                  autocorrect="off"
                                  spellcheck={false}
                                  placeholder="Authorization"
                                  value={header().name}
                                  onInput={(ev) =>
                                    rewriteHeader(at, which, {
                                      name: ev.currentTarget.value,
                                    })
                                  }
                                />

                                <label for={`mcp-value-${at}-${which}`}>
                                  Value
                                </label>
                                <input
                                  id={`mcp-value-${at}-${which}`}
                                  type="password"
                                  autocapitalize="off"
                                  autocorrect="off"
                                  spellcheck={false}
                                  autocomplete="off"
                                  placeholder={
                                    kept(header())
                                      ? "Kept as it is"
                                      : "Nothing is sent in it"
                                  }
                                  value={header().value}
                                  onInput={(ev) =>
                                    rewriteHeader(at, which, {
                                      value: ev.currentTarget.value,
                                      // Typing is unambiguous, so it settles a
                                      // clearing somebody asked for and then
                                      // thought better of.
                                      cleared: false,
                                    })
                                  }
                                />

                                {/* What a blank box cannot say. Blank is
                                    *keep* — that is what stops a corrected URL
                                    taking a key away — so taking a value away
                                    is a press of its own. */}
                                <Show
                                  when={
                                    kept(header()) && header().value === ""
                                  }
                                >
                                  <p class={styles.standingBy}>
                                    <Show
                                      when={header().cleared}
                                      fallback={
                                        <>
                                          A value is kept for it. Typing one
                                          replaces it.
                                        </>
                                      }
                                    >
                                      Its value will be taken away when this is
                                      saved.
                                    </Show>
                                  </p>

                                  <QuietButton
                                    class={styles.clear}
                                    onClick={() =>
                                      rewriteHeader(at, which, {
                                        cleared: !header().cleared,
                                      })
                                    }
                                  >
                                    {header().cleared
                                      ? "Keep its value"
                                      : "Clear its value"}
                                  </QuietButton>
                                </Show>

                                <QuietButton
                                  class={styles.remove}
                                  onClick={() => removeHeader(at, which)}
                                >
                                  Remove header
                                </QuietButton>
                              </li>
                            )}
                          </Index>
                        </ul>
                      </Show>

                      <QuietButton
                        class={styles.add}
                        onClick={() => addHeader(at)}
                      >
                        Add a header
                      </QuietButton>

                      <QuietButton
                        class={styles.remove}
                        onClick={() => remove(at)}
                      >
                        Remove
                      </QuietButton>
                    </li>
                  )}
                </Index>
              </ul>
            </Show>

            <QuietButton class={styles.add} onClick={add}>
              Add a server
            </QuietButton>

            <div class={styles.buttons}>
              <button type="submit" disabled={save.isPending}>
                Save
              </button>
            </div>

            {/* A server that could not write the files, which is the one thing
                here that is an error rather than an answer. */}
            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The settings could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </form>
        </Match>
      </Choose>
    </>
  );
}
