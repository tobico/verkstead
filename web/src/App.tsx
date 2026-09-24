//! The application: everything under the API routes the agents use.

import { Navigate, Route, Router, useParams } from "@solidjs/router";
import { QueryClient, QueryClientProvider } from "@tanstack/solid-query";
import { Show, onCleanup, onMount, type JSX } from "solid-js";

import styles from "./App.module.css";
import { Toasts } from "./Toasts";
import { loadOnboarding, retrying } from "./api/client";
import { useReading } from "./freshness";
import { Empty } from "./notices";
import { listenForNudges } from "./nudge";
import { SettingsPage, panes } from "./settings/SettingsPage";
import { SetupPage } from "./setup/SetupPage";
import { SETUP } from "./setup/steps";
import { ComposePage } from "./workbench/Compose";
import { forget } from "./workbench/eager";
import { pathTo } from "./workbench/openings";
import { Workbench } from "./workbench/Workbench";

/// One client for the whole app, made once rather than per render: it is where
/// the cache lives, and a page that rebuilt it would have no cache at all.
///
/// Coming back reads afresh whatever has gone stale, which is half of what
/// stands in for the poll that used to (ADR-0009). The other half is the
/// catch-up in `nudge.ts`, on the same moment and wider: this setting re-reads
/// what a staleTime has let go, and the catch-up re-reads everything, because a
/// page that was away cannot know what it missed.
///
/// Said out loud though it is the default, because it used to be off, on the
/// reasoning that coming back to a tab is not new information about a Set. For
/// an installed app it is precisely that: the phone was away, nothing was
/// listening while it was, and the list the human is now looking at stopped
/// being true while they were gone.
const queries = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: true,
      // And the ordinary three attempts, minus the read that gave up on a
      // deadline of its own — see [`retrying`], which is where the reasoning is.
      retry: retrying,
    },
  },
});

export function App(): JSX.Element {
  // Held here rather than by a page, because the whole app is what a Nudge is
  // about: the stream outlives every navigation between the lists and a Set,
  // and a page that opened its own would drop it on the way to the next.
  onMount(() => onCleanup(listenForNudges(queries)));

  return (
    <QueryClientProvider client={queries}>
      <Gate />
    </QueryClientProvider>
  );
}

/// Which of the two apps this is: the wizard, or the workbench.
///
/// The gate sits here, around the router, rather than inside any page — see
/// ADR-0016. While onboarding mode is on there is nowhere to be but `/setup`,
/// and the way to say that once is to give the router a table with one page in
/// it and a redirect for everything else. A guard written into each page would
/// be the same sentence said eight times, and the ninth page added would be the
/// one nobody said it on.
///
/// Which also settles the other half: while the mode is off there is no
/// `/setup` route at all, so the path falls to the catch-all exactly as any
/// other path nothing answers does. The wizard is a first run rather than a
/// page to visit.
///
/// **Nothing at all until the verdict has landed.** The mode is one small read
/// off the machine this page is served by, and the alternative to waiting for
/// it is drawing the workbench and then taking it away — a fresh install's
/// first sight of Verkstead being a flash of somebody else's empty lists. A
/// read that fails is not a verdict either: the app draws as it always did,
/// because a server that cannot say whether it is set up is one that has been
/// answering everything else for months.
///
/// Exported for the reason [`Shell`] is: what a test about the route tables has
/// to mount is the gate the app really has, and [`App`] carries a query client
/// of its own that outlives every render — a verdict cached by one test would
/// be the wrong app drawn for a moment in the next, with a redirect fired out
/// of it.
export function Gate(): JSX.Element {
  const onboarding = useReading(() => ({
    queryKey: ["onboarding"],
    queryFn: loadOnboarding,
    // The same query the wizard reads, under the same key: one read of the
    // machine between the two of them, and the interval that keeps it fresh is
    // the wizard's own — see `SetupPage.tsx`.
    freshness: { reconcile: "dependency" },
  }));

  return (
    <Show when={!onboarding.isPending}>
      <Show when={onboarding.data?.mode} fallback={<Verkstead />}>
        <Onboarding />
      </Show>
    </Show>
  );
}

/// The wizard and nothing else: `/setup`, and every other URL redirected onto
/// it.
///
/// A replace rather than a push, so that the back button does not walk into a
/// page that will only redirect again.
function Onboarding(): JSX.Element {
  return (
    <Router root={Shell}>
      <Route path={SETUP} component={SetupPage} />
      <Route path="*" component={() => <Navigate href={SETUP} />} />
    </Router>
  );
}

/// And the app as it has always been.
function Verkstead(): JSX.Element {
  return (
    <Router root={Shell}>
      {/* The workbench has the root: it is what Verkstead is for, and what a
          device with a window opens on. The Conversation in the URL is a
          record of which one is open rather than a document of its own — the
          same page draws both. */}
      <Route path="/" component={Workbench} />
      {/* And each of that Conversation's details panes under it, so what is
          open survives a reload and can be linked to. Nested rather than
          written out as six routes of their own, because the workbench is
          one page across all of them: a route the router swaps for another
          takes its component down with it, and everything the middle pane was
          holding — a Brief half typed into above all — would go every time a
          card was pressed. A parent route stays up while the leaf under it
          changes, and these leaves draw nothing: what they are is what the
          path says, and the page reads that off the URL.

          The `events/` segment keeps the ids apart from the panes named by a
          word beside them — see `openings.ts`. */}
      <Route path="/conversations/:id" component={Workbench}>
        <Route path="/" />
        <Route path="/events/:event" />
        <Route path="/backlog" />
        <Route path="/share" />
        <Route path="/code" />
        <Route path="/steer" />
        <Route path="/roadmaps/:name" />
      </Route>
      {/* And where Code's pane stood while it was the Terminal pane, which is
          a redirect rather than a page: a link somebody kept and a browser
          that remembered the old path still land on the pane, under the same
          Conversation. Outside the route above rather than another leaf of
          it, because those leaves draw nothing — the page reads what is open
          off the URL — so a redirect written as one of them would never be
          rendered to do its redirecting. */}
      <Route path="/conversations/:id/terminal" component={Moved} />
      {/* And the composer before there is anything for it to be about: the
          same page, working what the device is holding rather than a record.
          A page of its own rather than a pane of the workbench, because there
          is no Conversation for the workbench to be about — what it shares
          with it is the sidebar and the frame, both of which it draws. */}
      <Route path="/compose" component={ComposePage} />
      {/* Everything the human configures, on one page: the GitHub token and
          the git author Verkstead was told, the Agent Profiles a session runs
          under, and the Repos a Conversation is started against. The Repos
          and the Profiles had routes of their own until they were folded in
          here; those paths are no such page now, rather than redirects to
          this one.

          With a details pane of its own for each thing on it that is opened
          rather than read, nested for the reason the Conversation's are: the
          settings are one page across all of them, and a route the router
          swapped for another would take the middle pane down with it every
          time a card was pressed.

          Which panes there are is that page's own — see `panes` in
          `SettingsPage.tsx`. Written out here, this list was a second opinion
          about where a card leads, and a section added to the page without a
          line added here answered its own path with the catch-all below. */}
      <Route path="/settings" component={SettingsPage}>
        {panes()}
      </Route>
      <Route path="*" component={NoSuchPage} />
    </Router>
  );
}

/// What every page sits in. The column the stylesheets set its width on: the
/// measure every page but one is read at is `main`'s own in `styles/base.css`,
/// and the workbench's exception to it is this shell's, in `App.module.css`.
///
/// With the toast layer beside it rather than inside it: what a toast is drawn
/// over is the page, so it belongs outside the column the page is read at — and
/// it is here, once, because an outcome outlives the control that learned it and
/// no page owns one. See [`Toasts`].
///
/// Exported so that a test mounting a page can mount the shell it really sits
/// in: a press whose outcome is a toast has nowhere to say it otherwise, and a
/// test that supplied its own layer would be asking about a layer the app does
/// not have.
export function Shell(props: { children?: JSX.Element }): JSX.Element {
  // And what a press has said ahead of the server goes with it, for the reason
  // the toasts do: nothing there outlives its own request while the app is up,
  // so this is about the app itself ending — see `eager.ts`.
  onCleanup(forget);

  return (
    <>
      <main class={styles.shell}>{props.children}</main>
      <Toasts />
    </>
  );
}

/// Where the Terminal pane stood, sending whoever asked for it on to Code
/// under the same Conversation — see `Code.tsx`.
///
/// A replace rather than a push, the way the wizard's catch-all is: the old
/// path is a page nobody should be able to walk back into, and a push would
/// put one entry of it between the pane and wherever the human came from.
///
/// The one path of the workbench's that is a redirect rather than nothing. A
/// pane that moved within the settings is no such page instead, because those
/// are sections that were folded into one another; this one is the same pane
/// under a new name, still holding the shells the old path opened.
///
/// Exported for the reason [`Shell`] is: a test mounting the workbench mounts
/// the route table the app really has, and a redirect written a second time in
/// the bench would be a test of a redirect the app does not ship.
export function Moved(): JSX.Element {
  const params = useParams();

  return <Navigate href={pathTo(params.id!, "code")} />;
}

function NoSuchPage(): JSX.Element {
  return <Empty>No such page.</Empty>;
}
