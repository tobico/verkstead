//! The lint wall around the query hook.
//!
//! One rule, and it exists for one reason: every query in this viewer has to
//! say how its re-reads are to land, and the way to make sure of that is to
//! make the raw hook unreachable. `useReading` in `src/freshness.ts` is the way
//! in — it takes the same options with `freshness` added and no `reconcile`,
//! so a query that has decided nothing does not compile, and one that imported
//! its way around the decision does not lint.
//!
//! ADR-0005 wrote the rule down and review missed it on seven queries out of
//! eleven; ADR-0009 is what turned it into these two files. Everything else
//! `@tanstack/solid-query` exports — the mutations, the client, the provider —
//! is untouched: mutations write and do not hold what a reader is in the
//! middle of.
//!
//! Nothing else is linted here. `tsc --noEmit` is the viewer's other static
//! check and the one that reads types; this is deliberately a wall and not a
//! style regime.

import parser from "@typescript-eslint/parser";

/// The names that read data. Every one of them makes a query the cache holds
/// and a Nudge invalidates, so every one of them has to come through the
/// wrapper.
const READING = [
  "useQuery",
  "createQuery",
  "useQueries",
  "createQueries",
  "useInfiniteQuery",
  "createInfiniteQuery",
];

/// The one module allowed to import them, being the one that wraps them.
const WRAPPER = "src/freshness.ts";

export default [
  {
    // The viewer's own sources and its tests. A test may hold a query as
    // readily as a component, and one that made its own would be a test
    // proving something about a query nothing in the app is.
    files: ["src/**/*.ts", "src/**/*.tsx", "tests/**/*.ts", "tests/**/*.tsx"],

    // `tsc` is what typechecks; this parser is here to read TypeScript and JSX
    // into an AST the core rule can see the imports in, and no more than that.
    languageOptions: { parser },

    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tanstack/solid-query",
              importNames: READING,
              message:
                "Read through `useReading` from src/freshness.ts, which makes " +
                "the query name a reconcile key or declare its payload " +
                "static — see ADR-0005 and ADR-0009.",
            },
          ],
        },
      ],
    },
  },
  {
    files: [WRAPPER],
    rules: { "no-restricted-imports": "off" },
  },
];
