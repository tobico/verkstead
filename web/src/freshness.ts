//! Reading the server into the page: the one way a query is made here, and the
//! decision it does not let a caller skip.
//!
//! A Nudge invalidates what the page is showing (ADR-0005), so every query is
//! re-read while the human is looking at it. A re-read that replaces its payload
//! wholesale takes the reader's own state with it — an open fold, a dropdown
//! mid-choice, a spinner mid-spin are all DOM state on elements the rebuild
//! throws away. So every query has to say one of two things about itself, and
//! this is what makes saying neither impossible: `freshness` has no default and
//! no shape but the two.
//!
//! **Merge** names the key a re-read is matched up by — flat on each element,
//! because reconcile does not look inside — and what did not change is left
//! alone, object and DOM node together. Elements the key is absent from are
//! matched by position instead, which is sound for a list only ever appended to.
//!
//! **Static** is for the payload that cannot change: read once and never again.
//! A finite `staleTime` is not this and cannot stand in for it — invalidation
//! beats staleness, so `"static"` is the only one that survives a Nudge.
//!
//! Neither is a property of the endpoint: the same query is often merged while
//! its session runs and frozen once it has stopped, which is why `freshness` is
//! a value read on every render rather than a flag set at the call site once.
//!
//! The rule is ADR-0005's and was written down once already. It was then missed
//! on seven queries out of eleven, which is why it is a type signature now
//! (ADR-0009) — with a lint wall in `eslint.config.js` keeping the hook
//! underneath out of everybody else's reach.

import {
  useQuery,
  type DefaultError,
  type QueryKey,
  type QueryOptions,
  type UseQueryResult,
} from "@tanstack/solid-query";

/// What a query says about its own re-reads. One of the two and never neither.
///
/// `{ reconcile }` is the key the merge matches elements by — `"id"` for
/// everything on this wire that carries one, and whatever the payload actually
/// carries where that is something else.
export type Freshness = "static" | { reconcile: string };

/// A query's options, minus the ones this module owns, plus the one it insists
/// on.
///
/// `reconcile` is gone because saying it here would be saying the decision
/// twice, in two places that could disagree. `staleTime` narrows to a number
/// for the same reason: the one value of it that would be an opt-out is not the
/// caller's to write. A finite one still is — a query that merges may well have
/// an interval before a fresh mount is worth a refetch.
///
/// `initialData` goes because nothing here has any: a query with data before it
/// has read anything is a different result type, and there is no such query in
/// this viewer to keep the second one for.
export type Reading<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
> = Omit<
  QueryOptions<TQueryFnData, TError, TData, TQueryKey>,
  "reconcile" | "staleTime" | "initialData"
> & {
  freshness: Freshness;
  staleTime?: number;
};

/// Read something of the server's, having said how the re-reads are to land.
export function useReading<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(
  options: () => Reading<TQueryFnData, TError, TData, TQueryKey>,
): UseQueryResult<TData, TError> {
  return useQuery<TQueryFnData, TError, TData, TQueryKey>(() => {
    const { freshness, ...rest } = options();

    return {
      ...rest,
      // Frozen queries name no key: nothing is ever merged into them, there
      // being no second read to merge.
      ...(freshness === "static"
        ? { staleTime: "static" as const }
        : { reconcile: freshness.reconcile }),
    };
  });
}
