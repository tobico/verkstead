# 06. Monaco

## What to build

The editor arrives: a text tab is Monaco rather than plain text.

**Whole, as ADR-0019 decided and as the human confirmed once it could be
priced.** Every built-in language coloured, and completions and diagnostics
for TypeScript, JavaScript, JSON, CSS and HTML from the bundled workers. The
TypeScript worker is about seven megabytes of the editor's twenty-three and is
what a diet would take first; it stays.

**A chunk the page loads when Code first opens and never before**, with its
workers beside it under the hashed assets path the viewer already keeps for a
year. The current release restructured the package: the root import now
registers every language by itself, which is what *whole* asks for, and the
workers are still per-file ESM entries wanting Vite's `?worker` — there is no
bundled loader. It pulls two runtime dependencies of its own.

It follows the workbench's light and dark themes. Its settings — word wrap,
font size, minimap — are stage 03 of the roadmap; here it is VS Code's
defaults.

**The suite has to survive it.** jsdom has no layout, no canvas and no fonts,
and the vitest run has tripped on heap before; mounting the real editor in a
test is not the thing to try first. Stub the module for the suite and keep the
assertions on what the pane does around it.

**And the binary is measured.** The viewer is embedded in the one binary, so
Monaco is weight in every release and every update, not only a download the
page makes once. Build the release before and after, and write down what it
came to — ADR-0019 names that cost and this is where it becomes a number.

## Acceptance criteria

- [ ] A text tab is Monaco, following the workbench's theme, and its chunk is
      not fetched until Code first opens.
- [ ] Colouring works in a Rust file and completions in a TypeScript one.
- [ ] The vitest suite passes without mounting the real editor, and without
      going near the heap limit.
- [ ] The release binary's size before and after is measured and written down.
