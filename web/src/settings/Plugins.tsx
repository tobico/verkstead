//! The **Plugins** section (ADR-0030): the card at the foot of the settings
//! list, after Repos, and the pane behind it.
//!
//! **Two groups, Installed above Built-in**, one row per Plugin whatever it
//! provides. The Built-in group is drawn only once it has a row (Set 1772, Q4):
//! the Skill Pack `verkstead` first, then Codex and the six Language plugins,
//! each Language plugin saying the languages it provides. A built-in has no
//! Remove, and is always on, so its row has no checkbox — but for a Language
//! plugin, whose tick is whether sessions get its languages, an Agent plugin,
//! whose tick is whether its harness is offered, and a Forge, whose tick is
//! whether a Repo's remote is matched against it. An installer's own
//! language — a `languages:`
//! entry of this device's `config.yaml` that no plugin provides — is a row
//! under Installed, saying where it comes from, with no Update or Remove: the
//! entry is the file's.
//!
//! **A row** reads the plugin's name, version and author, the Category of what
//! it provides, where it came from, its checkbox, and its trouble where it
//! didn't load on this device — which counts it as disabled and stops nothing
//! else. The checkbox is the cluster's: unticked here is unticked everywhere. A
//! local plugin on another device is drawn greyed, naming that device, and
//! nothing of it is loaded here.
//!
//! **Add a plugin…** takes a browsed path, the field the Agent Profiles use,
//! or a git URL typed into it, and reads its manifest into the **consent
//! card**: who it is, what it provides and every capability it asks for.
//! **Install** is the grant, and the whole of it (ADR-0030, *Capabilities and
//! trust*); the press carries back what the card showed, so a manifest asking
//! for more by then is refused rather than installed. **Update** on a local
//! row reads the manifest again, and on a git row pulls; either shows the card
//! first where it now asks for more. Nothing fetches on its own.
//!
//! **A dependency that isn't there** is the dependent's trouble, and its row
//! offers the press that puts it right: **Install** from the repository the
//! dependency names, through the same consent card, or **Update** on the
//! dependency's own row. A disabled one is ticked on its own row, which the
//! trouble says.
//!
//! **Settings** on a row whose manifest declares fields opens its pane — see
//! `PluginSettings.tsx`.
//!
//! **Unticking is allowed over what stands on a plugin; Remove is refused by
//! it** (Set 1769, Q45). Unticking one that ticked plugins depend on asks
//! first, naming the ones that go with it. **Remove** asks too, saying what
//! goes, and the server refuses it while installed plugins depend on it or
//! agent profiles use its harness, counting them on the row. A built-in has no
//! Remove.
//!
//! **An Agent plugin's row says how many profiles use its harness** (Set 1764,
//! Q20): a disabled one's stay, offered on no picker. And where its harness's
//! binary is not on this machine, the row says so, with **Install** where the
//! manifest declares commands for it — the wizard's own install run, which is
//! the only thing that runs them.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  type JSX,
} from "solid-js";

import { Check } from "../Check";
import { PaneSticky } from "../Panes";
import { PathField } from "../PathField";
import {
  enablePlugin,
  installPlugin,
  installPluginHarness,
  loadPlugins,
  readPlugin,
  removePlugin,
  updatePlugin,
} from "../api/client";
import type {
  PluginCapability,
  PluginConsent,
  PluginHarness,
  PluginNeed,
  PluginRow,
  PluginsView,
  PluginUpdated,
} from "../api/types";
import { useReading } from "../freshness";
import { RUNNING } from "../setup/steps";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./Plugins.module.css";
import { SettingsCard } from "./SettingsCard";

/// The plugins as this device reads them, for the card and the pane.
function usePlugins() {
  return useReading(() => ({
    queryKey: ["plugins"],
    queryFn: loadPlugins,
    // Read again at the wizard's run cadence while a harness is installing,
    // the run moving nothing a Nudge says.
    refetchInterval: (query) => {
      const view = query.state.data as PluginsView | undefined;

      return view?.installed.some(
        (row) => row.harness?.install.install === "Installing",
      )
        ? RUNNING
        : false;
    },
    freshness: { reconcile: "uuid" },
  }));
}

/// The card: how many are installed, warning where one is in trouble.
export function PluginsCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const plugins = usePlugins();

  const troubled = (): number =>
    plugins.data?.installed.filter((row) => row.trouble !== null).length ?? 0;

  const status = (): string => {
    const installed = plugins.data?.installed.length ?? 0;
    if (troubled() > 0) {
      return troubled() === 1 ? "1 in trouble" : `${troubled()} in trouble`;
    }
    if (installed === 0) {
      return "None installed";
    }
    return `${installed} installed`;
  };

  return (
    <SettingsCard
      class={styles.pluginsCard}
      title="Plugins"
      open={props.open}
      press={props.press}
      status={plugins.data === undefined ? "" : status()}
      warn={troubled() > 0}
    />
  );
}

/// The pane: the groups, and **Add a plugin…** under them.
export function PluginsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
  /// Opening a plugin's settings pane, from its row.
  open: (uuid: string) => void;
}): JSX.Element {
  const plugins = usePlugins();
  const queries = useQueryClient();

  // What the server said about a press on a row it wouldn't take, by the row's
  // uuid.
  const [refused, setRefused] = createSignal<{
    uuid: string;
    why: string;
  } | null>(null);

  // The consent card an Update came back with, where the manifest now asks
  // for more than was granted.
  const [reviewing, setReviewing] = createSignal<{
    uuid: string;
    consent: PluginConsent;
  } | null>(null);

  // The consent card for a dependency a row offered to install, drawn under
  // that row.
  const [fixing, setFixing] = createSignal<{
    uuid: string;
    repository: string;
    consent: PluginConsent;
  } | null>(null);

  const read = (view: PluginsView): void => {
    setRefused(null);
    queries.setQueryData(["plugins"], view);
  };

  const enable = useMutation(() => ({
    mutationFn: (press: { uuid: string; enabled: boolean }) =>
      enablePlugin(press.uuid, press.enabled),
    onSuccess: read,
    onError: (error: Error, press) =>
      setRefused({ uuid: press.uuid, why: error.message }),
  }));

  const update = useMutation(() => ({
    mutationFn: (press: {
      uuid: string;
      granted: PluginCapability[] | null;
    }) => updatePlugin(press.uuid, press.granted),
    onSuccess: (updated: PluginUpdated, press) => {
      if (updated.kind === "Consent") {
        setReviewing({ uuid: press.uuid, consent: updated.consent });
        return;
      }

      setReviewing(null);
      read(updated.plugins);
    },
    onError: (error: Error, press) =>
      setRefused({ uuid: press.uuid, why: error.message }),
  }));

  const fetching = useMutation(() => ({
    mutationFn: (press: { uuid: string; repository: string }) =>
      readPlugin(press.repository),
    onSuccess: (consent: PluginConsent, press) => {
      setRefused(null);
      setFixing({ ...press, consent });
    },
    onError: (error: Error, press) =>
      setRefused({ uuid: press.uuid, why: error.message }),
  }));

  const installing = useMutation(() => ({
    mutationFn: (press: {
      uuid: string;
      repository: string;
      granted: PluginCapability[];
    }) => installPlugin(press.repository, press.granted),
    onSuccess: (view: PluginsView) => {
      setFixing(null);
      read(view);
    },
    onError: (error: Error, press) =>
      setRefused({ uuid: press.uuid, why: error.message }),
  }));

  /// A press on a row's need: the dependency's card read off its repository,
  /// or its own row's **Update**.
  const fix = (uuid: string, need: PluginNeed): void => {
    switch (need.fix.kind) {
      case "Install":
        fetching.mutate({ uuid, repository: need.fix.repository });
        return;
      case "Update":
        update.mutate({ uuid: need.uuid, granted: null });
        return;
    }
  };

  const removing = useMutation(() => ({
    mutationFn: (uuid: string) => removePlugin(uuid),
    onSuccess: read,
    onError: (error: Error, uuid) => setRefused({ uuid, why: error.message }),
  }));

  const installingHarness = useMutation(() => ({
    mutationFn: (uuid: string) => installPluginHarness(uuid),
    onSuccess: read,
    onError: (error: Error, uuid) => setRefused({ uuid, why: error.message }),
  }));

  const pressing = (): boolean =>
    enable.isPending ||
    update.isPending ||
    fetching.isPending ||
    installing.isPending ||
    removing.isPending ||
    installingHarness.isPending;

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Plugins" />
      </PaneSticky>

      <Switch>
        <Match when={plugins.isPending}>
          <Empty>Reading the plugins…</Empty>
        </Match>
        <Match when={plugins.isError}>
          <ErrorLine>
            The plugins couldn't be read. {plugins.error?.message}
          </ErrorLine>
        </Match>
        <Match when={plugins.data}>
          {(view) => (
            <>
              <section class={styles.group}>
                <h3>Installed</h3>
                <Show
                  when={view().installed.length > 0}
                  fallback={
                    <Empty>
                      No plugins are installed yet. Add one from a directory
                      or a git repository with a <code>plugin.yaml</code>.
                    </Empty>
                  }
                >
                  <ul class={styles.list}>
                    <For each={view().installed}>
                      {(row) => (
                        <Row
                          of={row}
                          pressing={pressing()}
                          flip={(enabled) =>
                            enable.mutate({ uuid: row.uuid, enabled })
                          }
                          update={
                            row.source.kind === "Config"
                              ? undefined
                              : () =>
                                  update.mutate({ uuid: row.uuid, granted: null })
                          }
                          remove={
                            row.source.kind === "Config"
                              ? undefined
                              : () => removing.mutate(row.uuid)
                          }
                          install={() => installingHarness.mutate(row.uuid)}
                          fix={(need) => fix(row.uuid, need)}
                          open={() => props.open(row.uuid)}
                          refused={
                            refused()?.uuid === row.uuid
                              ? refused()!.why
                              : null
                          }
                        >
                          <Show
                            when={
                              reviewing()?.uuid === row.uuid
                                ? reviewing()!.consent
                                : null
                            }
                          >
                            {(consent) => (
                              <Consent
                                of={consent()}
                                lede="It now asks for more than was granted."
                                verb="Update"
                                pressing={update.isPending}
                                grant={() =>
                                  update.mutate({
                                    uuid: row.uuid,
                                    granted: consent().capabilities,
                                  })
                                }
                                cancel={() => setReviewing(null)}
                              />
                            )}
                          </Show>
                          <Show
                            when={
                              fixing()?.uuid === row.uuid ? fixing() : null
                            }
                          >
                            {(fixed) => (
                              <Consent
                                of={fixed().consent}
                                lede={`${row.name} needs it.`}
                                verb="Install"
                                pressing={installing.isPending}
                                grant={() =>
                                  installing.mutate({
                                    uuid: row.uuid,
                                    repository: fixed().repository,
                                    granted: fixed().consent.capabilities,
                                  })
                                }
                                cancel={() => setFixing(null)}
                              />
                            )}
                          </Show>
                        </Row>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              <Show when={view().built_in.length > 0}>
                <section class={styles.group}>
                  <h3>Built-in</h3>
                  <ul class={styles.list}>
                    <For each={view().built_in}>
                      {(row) => (
                        <Row
                          of={row}
                          pressing={enable.isPending}
                          flip={
                            row.languages.length > 0 ||
                            row.provides.includes("agent") ||
                            row.provides.includes("forge")
                              ? (enabled) =>
                                  enable.mutate({ uuid: row.uuid, enabled })
                              : undefined
                          }
                          open={() => props.open(row.uuid)}
                          refused={
                            refused()?.uuid === row.uuid
                              ? refused()!.why
                              : null
                          }
                        />
                      )}
                    </For>
                  </ul>
                </section>
              </Show>

              <Adding installed={read} />
            </>
          )}
        </Match>
      </Switch>
    </>
  );
}

/// One plugin's row.
function Row(props: {
  of: PluginRow;
  pressing: boolean;
  /// Its checkbox, where it has one: every installed plugin, and a built-in
  /// only where it is a Language, an Agent or a Forge plugin — the rest are
  /// always on.
  flip?: (enabled: boolean) => void;
  /// **Update**, where the row has one: a git plugin, or a local plugin on
  /// this device.
  update?: () => void;
  /// **Remove**, where the row has one: every installed plugin, and no
  /// built-in.
  remove?: () => void;
  /// A press on one of its needs, where the row has trouble a press puts
  /// right.
  fix?: (need: PluginNeed) => void;
  /// **Install** on the harness it provides, where that binary is missing.
  install?: () => void;
  /// Opening its settings pane, drawn where its manifest declares fields.
  open: () => void;
  refused: string | null;
  /// What is drawn under the row: the consent card an Update came back with,
  /// or one for a dependency it offered to install.
  children?: JSX.Element;
}): JSX.Element {
  /// The version and the author, as one quiet line beside the name — none
  /// for an installer's own language, which has neither.
  const facts = (): string =>
    props.of.source.kind === "Config"
      ? ""
      : `${props.of.version} · ${props.of.author}`;

  /// The press waiting on its confirmation: an untick that takes ticked
  /// dependents with it, or **Remove**.
  const [asking, setAsking] = createSignal<"disable" | "remove" | null>(null);

  const flip = (enabled: boolean): void => {
    if (!enabled && props.of.dependents.length > 0) {
      setAsking("disable");
      return;
    }
    props.flip?.(enabled);
  };

  return (
    <li
      class={styles.plugin}
      classList={{ [styles.elsewhere!]: props.of.elsewhere !== null }}
      data-plugin={props.of.uuid}
    >
      <div class={styles.head}>
        <Show
          when={props.flip !== undefined}
          fallback={
            <span>
              <span class={styles.name}>{props.of.name}</span>
              <Show when={facts()}>
                <span class={styles.facts}>{facts()}</span>
              </Show>
            </span>
          }
        >
          <Check
            label={
              <>
                <span class={styles.name}>{props.of.name}</span>
                <Show when={facts()}>
                  <span class={styles.facts}>{facts()}</span>
                </Show>
              </>
            }
            on={props.of.enabled}
            disabled={props.pressing || asking() !== null}
            flip={flip}
          />
        </Show>

        <Show when={props.of.settings > 0}>
          <button
            type="button"
            class={styles.press}
            onClick={() => props.open()}
          >
            Settings
          </button>
        </Show>

        <Show when={props.update !== undefined && props.of.elsewhere === null}>
          <button
            type="button"
            class={styles.press}
            disabled={props.pressing}
            onClick={() => props.update?.()}
          >
            Update
          </button>
        </Show>

        <Show when={props.remove !== undefined}>
          <button
            type="button"
            class={styles.press}
            disabled={props.pressing || asking() !== null}
            onClick={() => setAsking("remove")}
          >
            Remove
          </button>
        </Show>
      </div>

      <Show when={asking() === "disable"}>
        <Confirming
          question={`Disable ${props.of.name}?`}
          consequence={`${listed(props.of.dependents)} ${
            props.of.dependents.length === 1 ? "depends" : "depend"
          } on it, and ${
            props.of.dependents.length === 1 ? "is" : "are"
          } disabled with it.`}
          verb="Disable"
          act={() => {
            setAsking(null);
            props.flip?.(false);
          }}
          cancel={() => setAsking(null)}
        />
      </Show>

      <Show when={asking() === "remove"}>
        <Confirming
          question={`Remove ${props.of.name}?`}
          consequence={
            props.of.source.kind === "Git"
              ? "It's taken off every device, and each device's clone of it is deleted."
              : "It's taken off every device. Its directory stays."
          }
          verb="Remove"
          act={() => {
            setAsking(null);
            props.remove?.();
          }}
          cancel={() => setAsking(null)}
        />
      </Show>

      <Show
        when={props.of.languages.length > 0}
        fallback={
          <Show when={props.of.provides.length > 0}>
            <p class={styles.provides}>
              Provides {props.of.provides.join(", ")}.
            </p>
          </Show>
        }
      >
        <p class={styles.provides}>
          Provides {listed(props.of.languages)}.
        </p>
      </Show>

      <Show when={props.of.activities.length > 0}>
        <p class={styles.provides}>{activities(props.of.activities)}</p>
      </Show>

      <Show when={source(props.of)}>
        {(from) => <p class={styles.source}>{from()}</p>}
      </Show>

      <Show when={props.of.profiles > 0}>
        <p class={styles.provides} data-profiles>
          {profilesUsing(props.of)}
        </p>
      </Show>

      <Show when={props.of.harness}>
        {(harness) => (
          <Harnessed
            of={harness()}
            pressing={props.pressing}
            install={props.install}
          />
        )}
      </Show>

      <Show when={props.of.elsewhere}>
        {(device) => <Note>On {device()}. It's loaded there.</Note>}
      </Show>

      <Show when={props.of.trouble}>
        {(trouble) => <p class={styles.trouble}>{trouble()}</p>}
      </Show>

      <Show when={props.fix !== undefined && props.of.needs.length > 0}>
        <div class={styles.needs}>
          <For each={props.of.needs}>
            {(need) => (
              <button
                type="button"
                class={styles.press}
                disabled={props.pressing}
                onClick={() => props.fix?.(need)}
              >
                {need.fix.kind === "Install" ? "Install" : "Update"}{" "}
                {need.name}
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={props.refused}>{(why) => <ErrorLine>{why()}</ErrorLine>}</Show>

      {props.children}
    </li>
  );
}

/// How many agent profiles use the harness a row provides, and — where it is
/// disabled — that none of them is offered.
function profilesUsing(row: PluginRow): string {
  const used =
    row.profiles === 1
      ? "1 agent profile uses it."
      : `${row.profiles} agent profiles use it.`;

  return row.enabled
    ? used
    : `${used} ${row.profiles === 1 ? "It isn't" : "They aren't"} offered while it's disabled.`;
}

/// The harness an Agent plugin's row provides, where its binary is not on this
/// machine: what is missing, and **Install** where the manifest declares
/// commands for it.
function Harnessed(props: {
  of: PluginHarness;
  pressing: boolean;
  install?: () => void;
}): JSX.Element {
  const installing = (): boolean =>
    props.of.install.install === "Installing";

  const failed = (): string | null =>
    props.of.install.install === "Failed" ? props.of.install.why : null;

  return (
    <Show when={!props.of.present}>
      <div class={styles.needs} data-harness={props.of.id}>
        <p class={styles.missing}>
          <code>{props.of.binary}</code> isn't on this machine.
        </p>
        <Show when={props.of.installable && props.install !== undefined}>
          <button
            type="button"
            class={styles.press}
            disabled={props.pressing || installing()}
            onClick={() => props.install?.()}
          >
            {installing() ? "Installing…" : "Install"}
          </button>
        </Show>
      </div>
      <Show when={failed()}>{(why) => <ErrorLine>{why()}</ErrorLine>}</Show>
    </Show>
  );
}

/// A press's confirmation under its row: the question naming the act, what
/// follows from it, and the act's verb against **Cancel**.
function Confirming(props: {
  question: string;
  consequence: string;
  verb: string;
  act: () => void;
  cancel: () => void;
}): JSX.Element {
  return (
    <div class={styles.consent} role="group" aria-label={props.question}>
      <h3>{props.question}</h3>
      <p>{props.consequence}</p>
      <div class={styles.buttons}>
        <button type="button" onClick={() => props.act()}>
          {props.verb}
        </button>
        <button
          type="button"
          class={`${styles.cancel} secondary`}
          onClick={() => props.cancel()}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

/// `names` as a sentence lists them, with the serial comma.
function listed(names: string[]): string {
  if (names.length <= 2) {
    return names.join(" and ");
  }
  return `${names.slice(0, -1).join(", ")}, and ${names.at(-1)}`;
}

/// The Activities a plugin provides, by the names its manifest draws them as.
function activities(names: string[]): string {
  return names.length === 1
    ? `Its activity is ${names[0]}.`
    : `Its activities are ${listed(names)}.`;
}

/// Where a row's plugin came from: its directory, its repository and the
/// revision this cluster is pinned to, or this device's `config.yaml`. Nothing
/// for a built-in, whose group says it.
function source(row: PluginRow): string | null {
  switch (row.source.kind) {
    case "Local":
      return row.source.path;
    case "Git":
      return `${row.source.url} at ${row.source.revision.slice(0, 7)}`;
    case "Config":
      return "From this device's config.yaml.";
    case "BuiltIn":
      return null;
  }
}

/// What one capability is, in a few words.
function asked(capability: PluginCapability): JSX.Element {
  switch (capability.kind) {
    case "Filesystem":
      return (
        <>
          {capability.writable ? "Read and write " : "Read "}
          <code>{capability.path}</code>
        </>
      );
    case "Outbound":
      return (
        <>
          Connect to <code>{capability.host}</code>
        </>
      );
    case "Environment":
      return (
        <>
          Read the environment variable <code>{capability.name}</code>
        </>
      );
    case "Conversations":
      return <>Start conversations of its own</>;
    case "Secrets":
      return (
        <>
          Read a secret Verkstead keeps for it, as <code>{capability.name}</code>
        </>
      );
  }
}

/// The consent card: who the plugin is, what it provides, and every
/// capability it asks for, over the press that grants them.
function Consent(props: {
  of: PluginConsent;
  /// A sentence above it, where it is shown for a reason of its own.
  lede?: string;
  /// The press that grants: **Install**, or **Update**.
  verb: string;
  pressing: boolean;
  grant: () => void;
  cancel: () => void;
}): JSX.Element {
  return (
    <div class={styles.consent} data-consent={props.of.uuid}>
      <Show when={props.lede}>{(lede) => <p>{lede()}</p>}</Show>

      <h3>
        {props.of.name} {props.of.version}
      </h3>
      <p class={styles.facts}>
        {props.of.author} · {props.of.email}
      </p>

      <Show when={props.of.description}>
        {(description) => <p>{description()}</p>}
      </Show>

      <p>
        {props.of.provides.length > 0
          ? `Provides ${props.of.provides.join(", ")}.`
          : "Provides nothing yet."}
      </p>

      <Show when={props.of.activities.length > 0}>
        <p>{activities(props.of.activities)}</p>
      </Show>

      <Show
        when={props.of.capabilities.length > 0}
        fallback={<p>It asks for no capabilities.</p>}
      >
        <p>It asks to:</p>
        <ul class={styles.capabilities}>
          <For each={props.of.capabilities}>
            {(capability) => <li>{asked(capability)}</li>}
          </For>
        </ul>
      </Show>

      <div class={styles.buttons}>
        <button
          type="button"
          disabled={props.pressing}
          onClick={() => props.grant()}
        >
          {props.verb}
        </button>
        <button
          type="button"
          class={`${styles.cancel} secondary`}
          onClick={() => props.cancel()}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

/// **Add a plugin…**: a browsed path or a git URL, its consent card, and
/// **Install**.
function Adding(props: { installed: (view: PluginsView) => void }): JSX.Element {
  const [open, setOpen] = createSignal(false);
  const [path, setPath] = createSignal("");
  const [consent, setConsent] = createSignal<PluginConsent | null>(null);
  const [refused, setRefused] = createSignal<string | null>(null);

  const close = (): void => {
    setOpen(false);
    setPath("");
    setConsent(null);
    setRefused(null);
  };

  const reading = useMutation(() => ({
    mutationFn: readPlugin,
    onSuccess: (read: PluginConsent) => {
      setRefused(null);
      setConsent(read);
    },
    onError: (error: Error) => setRefused(error.message),
  }));

  const installing = useMutation(() => ({
    mutationFn: (granted: PluginCapability[]) =>
      installPlugin(path(), granted),
    onSuccess: (view: PluginsView) => {
      props.installed(view);
      close();
    },
    onError: (error: Error) => setRefused(error.message),
  }));

  return (
    <Show
      when={open()}
      fallback={
        <button
          type="button"
          class={`${styles.add} secondary`}
          onClick={() => setOpen(true)}
        >
          Add a plugin…
        </button>
      }
    >
      <form
        class={styles.adding}
        onSubmit={(ev) => {
          ev.preventDefault();
          reading.mutate(path());
        }}
      >
        <label for="plugin-path">Directory or git URL</label>
        <PathField
          id="plugin-path"
          placeholder="/home/you/src/my-plugin"
          value={path()}
          write={(asked) => {
            setPath(asked);
            setConsent(null);
            setRefused(null);
          }}
        />

        <Show when={refused()}>{(why) => <ErrorLine>{why()}</ErrorLine>}</Show>

        <Show
          when={consent()}
          fallback={
            <div class={styles.buttons}>
              <button
                type="submit"
                disabled={path().trim() === "" || reading.isPending}
              >
                Read manifest
              </button>
              <button
                type="button"
                class={`${styles.cancel} secondary`}
                onClick={close}
              >
                Cancel
              </button>
            </div>
          }
        >
          {(card) => (
            <Consent
              of={card()}
              verb="Install"
              pressing={installing.isPending}
              grant={() => installing.mutate(card().capabilities)}
              cancel={close}
            />
          )}
        </Show>
      </form>
    </Show>
  );
}
