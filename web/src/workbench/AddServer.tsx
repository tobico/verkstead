//! Declaring an MCP server from the Attach menu, without leaving the Brief for
//! the settings page: the modal *Add a server…* opens, and the attach that
//! follows it.
//!
//! **The settings page's declaration, made from somewhere else.** What the card
//! asks is what a new row on the MCP servers pane asks — a name, a URL, and the
//! headers it is spoken to with — and what the press sends is that pane's save:
//! every declaration as it stands with its header values kept, the new one at
//! the end, and every other section of the settings left exactly where it was.
//! See [`savingServers`] in `settings/McpServers.tsx`, which both of them call.
//!
//! **And confirmed is attached.** Once the save lands, the name it was saved
//! under is handed to the caller, which puts it on the work the way picking a
//! row of the menu would have — a request on a draft's composer, a chip held in
//! the page on the compose page. So the card is one press for what was two
//! trips, and the rest of what happens to the server is what happens to any
//! other.
//!
//! A refusal is the whole save refused and nothing written, exactly as it is on
//! the settings pane — so the card stays up with what was typed in it, and says
//! why at the box the refusal names.
//!
//! **And a server that did not answer holds the card up too.** A save speaks to
//! what it wrote down, and one that was not reached is still declared — it may
//! be reachable from inside a session's network and never from here — so the
//! card says so where the URL was typed and waits: *Attach anyway* goes on with
//! it, and *Try again* saves what the boxes say now over the declaration it
//! just made, which is where a mistyped URL is put right.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  Index,
  Show,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { Modal } from "../Modal";
import { QuietButton } from "../QuietButton";
import { loadSettings, saveSettings } from "../api/client";
import type {
  HeaderEdit,
  ServerField,
  SettingsSaved,
} from "../api/types";
import { useReading } from "../freshness";
import { ErrorLine, Note } from "../notices";
import { savingServers, standing } from "../settings/McpServers";
import styles from "./AddServer.module.css";

/// The card, and the way it is opened — made once by whoever draws an Attach
/// menu, because the menu's rows are built on the way open and thrown away on
/// the way shut (see `Menu.tsx`): a modal drawn among them would go with the
/// press that opened it. So the caller draws [`modal`] beside the menu, and the
/// row calls [`open`].
export function addingServer(
  /// What becomes of the server once it is declared: put on the work, by name.
  attach: (name: string) => void,
): { open: () => void; modal: () => JSX.Element } {
  const [up, setUp] = createSignal(false);

  return {
    open: () => setUp(true),
    modal: () => (
      <Show when={up()}>
        <AddServer
          close={() => setUp(false)}
          added={(name) => {
            setUp(false);
            attach(name);
          }}
        />
      </Show>
    ),
  };
}

/// One header as it is being typed: a name, and the value to send in it.
type Typed = { name: string; value: string };

/// The modal itself.
function AddServer(props: {
  /// Said when the card has been left without a declaration — Cancel, Escape,
  /// or a press on the backdrop.
  close: () => void;
  /// And what a save that landed does with the name it saved.
  added: (name: string) => void;
}): JSX.Element {
  const queries = useQueryClient();

  // The settings as they stand, which the save is made over: every other
  // declaration goes back as it is, and so does every other section. The one
  // read every reader of them makes, by the one key.
  const settings = useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" } as const,
  }));

  const id = createUniqueId();

  const [name, setName] = createSignal("");
  const [url, setUrl] = createSignal("");
  const [headers, setHeaders] = createSignal<Typed[]>([]);

  // Why the last save was turned down, at the box it named — or, for a refusal
  // about something other than this declaration, in a line of its own.
  const [trouble, setTrouble] = createSignal<Partial<Record<ServerField, string>>>(
    {},
  );
  const [elsewhere, setElsewhere] = createSignal<string | null>(null);

  // The name this card has already declared, where a save landed and the
  // server it wrote down did not answer — and why not. A save after that one is
  // a rewrite of that declaration rather than a second of the same name.
  const [declared, setDeclared] = createSignal<string | null>(null);
  const [unreached, setUnreached] = createSignal<string | null>(null);

  /// What is declared already, as the save sends it back — less the one this
  /// card made, which the boxes are now the account of.
  const others = () =>
    standing(settings.data).filter((one) => one.name !== declared());

  /// What is about to be written about one header's value: a typed one set, and
  /// a blank one nothing — there being nothing kept for a header that is new.
  const becomes = (header: Typed): HeaderEdit =>
    header.value === "" ? "Keep" : { Set: { value: header.value } };

  const save = useMutation(() => ({
    mutationFn: () =>
      saveSettings(
        savingServers(settings.data, {
          Set: {
            servers: [
              ...others(),
              {
                name: name().trim(),
                url: url().trim(),
                // A header row nobody wrote anything in is not a header, as a
                // declaration row nobody wrote in is not a declaration on the
                // settings pane.
                headers: headers()
                  .filter((one) => one.name.trim() !== "" || one.value !== "")
                  .map((one) => ({ name: one.name.trim(), value: becomes(one) })),
              },
            ],
          },
        }),
      ),
    onSuccess: (saved: SettingsSaved) => {
      // Where the new declaration stood in what was sent, which is how a
      // refusal names it: after every one already declared.
      const at = others().length;

      const mine = saved.refused_servers.filter((one) => one.server === at);
      const other = [
        ...saved.refused_servers
          .filter((one) => one.server !== at)
          .map((one) => one.why),
        ...saved.refused.map((one) => one.why),
        ...saved.refused_sizes.map((one) => one.why),
      ];

      if (mine.length > 0 || other.length > 0) {
        setTrouble(
          Object.fromEntries(mine.map((one) => [one.field, one.why])),
        );
        setElsewhere(other.length > 0 ? other.join(" ") : null);
        return;
      }

      // Taken from the answer rather than read again: a save comes back with a
      // fresh read of the files, so the menu's next opening offers what is
      // declared now.
      queries.setQueryData(["settings"], saved.settings);

      const called = name().trim();
      const tried = saved.tried.find((one) => one.server === called)?.outcome;

      if (tried && "Refused" in tried) {
        setDeclared(called);
        setUnreached(tried.Refused.why);
        return;
      }

      props.added(called);
    },
  }));

  const ready = () =>
    settings.data !== undefined && name().trim() !== "" && url().trim() !== "";

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    if (!ready() || save.isPending) return;

    setTrouble({});
    setElsewhere(null);
    setUnreached(null);
    save.mutate();
  };

  return (
    <Modal class={styles.adding!} open close={props.close} labelledBy={id}>
      <h3 id={id}>Add an MCP server</h3>

      <form class={styles.form} onSubmit={commit}>
        <Note class={styles.note}>
          Declared for the whole installation, as in Settings, and attached to
          this work. A header value is stored, and never shown again.
        </Note>

        <label for={`${id}-name`}>Name</label>
        <input
          id={`${id}-name`}
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder="docs"
          value={name()}
          onInput={(ev) => {
            setName(ev.currentTarget.value);
            setTrouble((was) => ({ ...was, Name: undefined }));
          }}
        />
        <Show when={trouble().Name}>
          {(why) => <ErrorLine class={styles.failure}>{why()}</ErrorLine>}
        </Show>

        <label class={styles.second} for={`${id}-url`}>
          URL
        </label>
        <input
          id={`${id}-url`}
          type="url"
          inputmode="url"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder="https://mcp.example.com/docs"
          value={url()}
          onInput={(ev) => {
            setUrl(ev.currentTarget.value);
            setTrouble((was) => ({ ...was, Url: undefined }));
          }}
        />
        <Show when={trouble().Url}>
          {(why) => <ErrorLine class={styles.failure}>{why()}</ErrorLine>}
        </Show>
        {/* Under the box it is most likely about, in the words the settings
            pane says it in. */}
        <Show when={unreached()}>
          {(why) => (
            <ErrorLine class={styles.failure}>
              It's declared, but it couldn't be reached when it was saved. {why()}
            </ErrorLine>
          )}
        </Show>

        {/* The headers it is spoken to with — an API key or a bearer token,
            which is the whole of the authentication there is. None until one
            is asked for: most servers a team runs for itself want none. */}
        <Show when={headers().length > 0}>
          <ul class={styles.headers}>
            <Index each={headers()}>
              {(header, at) => (
                <li class={styles.header}>
                  <div class={styles.headerField}>
                    <label for={`${id}-header-${at}`}>Header</label>
                    <input
                      id={`${id}-header-${at}`}
                      type="text"
                      autocapitalize="off"
                      autocorrect="off"
                      spellcheck={false}
                      placeholder="Authorization"
                      value={header().name}
                      onInput={(ev) => {
                        const typed = ev.currentTarget.value;
                        setHeaders((was) =>
                          was.map((one, which) =>
                            which === at ? { ...one, name: typed } : one,
                          ),
                        );
                      }}
                    />
                  </div>
                  <div class={styles.headerField}>
                    <label for={`${id}-value-${at}`}>Value</label>
                    <input
                      id={`${id}-value-${at}`}
                      type="password"
                      autocapitalize="off"
                      autocorrect="off"
                      spellcheck={false}
                      autocomplete="off"
                      value={header().value}
                      onInput={(ev) => {
                        const typed = ev.currentTarget.value;
                        setHeaders((was) =>
                          was.map((one, which) =>
                            which === at ? { ...one, value: typed } : one,
                          ),
                        );
                      }}
                    />
                  </div>
                  {/* A mark rather than a word, as a companion row's is; the
                      screen reader gets the sentence. */}
                  <button
                    type="button"
                    class={styles.forget}
                    aria-label={`Remove header ${at + 1}`}
                    onClick={() =>
                      setHeaders((was) => was.filter((_, which) => which !== at))
                    }
                  >
                    ×
                  </button>
                </li>
              )}
            </Index>
          </ul>
        </Show>

        <QuietButton
          class={styles.addHeader}
          onClick={() => setHeaders((was) => [...was, { name: "", value: "" }])}
        >
          Add a header
        </QuietButton>

        <div class={styles.buttons}>
          <Show
            when={unreached() !== null}
            fallback={
              <button type="submit" disabled={!ready() || save.isPending}>
                {save.isPending ? "Adding…" : "Add server"}
              </button>
            }
          >
            {/* The declaration stands either way, so going on with it is the
                press that does the thing, and saving it again is the quieter
                one beside it. */}
            <button
              type="button"
              disabled={save.isPending}
              onClick={() => props.added(declared()!)}
            >
              Attach anyway
            </button>
            <button
              type="submit"
              class={`${styles.cancel} secondary`}
              disabled={!ready() || save.isPending}
            >
              {save.isPending ? "Trying…" : "Try again"}
            </button>
          </Show>
          {/* Drawn as well as the ways out the modal already has: Escape and a
              press on the backdrop are for a keyboard and a cursor, and this is
              the one a thumb has. */}
          <button
            type="button"
            class={`${styles.cancel} secondary`}
            onClick={() => props.close()}
          >
            Cancel
          </button>
        </div>

        <Show when={elsewhere()}>
          {(why) => (
            <ErrorLine class={styles.failure}>
              The settings couldn't be saved as they stand: {why()}
            </ErrorLine>
          )}
        </Show>
        <Show when={settings.isError}>
          <ErrorLine class={styles.failure}>
            The settings couldn't be read: {settings.error?.message}
          </ErrorLine>
        </Show>
        <Show when={save.isError}>
          <ErrorLine class={styles.failure}>
            The server couldn't be added: {save.error?.message}
          </ErrorLine>
        </Show>
      </form>
    </Modal>
  );
}
