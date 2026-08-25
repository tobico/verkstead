# 03. Status components

## What to build

The app-wide status vocabulary — `.empty` (16 components), `.error` (17) and
`.note` (7), each styled once in `main.css` and refined per context — becomes
three small components instead of shared classes: `Empty`, `ErrorLine` and
`Note`, together in `web/src/notices.tsx` with a colocated
`notices.module.css`. This is the migration's first module, and it sets the
conventions every later one follows:

- Class names in the module are camelCase and identical in CSS and TS — no
  Vite `localsConvention`, the file itself is the single spelling.
- Contextual refinement follows the existing Menu/Modal pattern: the component
  accepts a `class` prop, and a parent that wants different margins or colors
  styles that class in its own module. The old `.add-repo .error`-style
  descendant refinements in `main.css` become exactly that, or die where the
  audit shows nothing needs them. Parents not yet migrated may keep a plain
  class on the prop until their own task; the base styling must come from the
  component either way.

Swap every raw `class="empty"`, `class="error"` and `class="note"` site over to
the components and delete the moved rules from `main.css`. Tests that assert on
these classes import `notices.module.css` and build their selectors from it.

## Acceptance criteria

- [ ] No `class="empty|error|note"` string remains in `web/src`; the base rules
      are gone from `main.css`.
- [ ] The components render the same elements with the same visual result,
      including per-context refinements.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
