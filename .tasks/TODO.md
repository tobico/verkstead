# CSS modules

Migrate the viewer's styling from the single 5,382-line `web/src/main.css` to
CSS modules — one colocated `*.module.css` per component file — plus a small set
of global sheets for what can never be hashed: base tokens and element defaults,
and the classes on HTML the frontend never writes (server-rendered markdown,
diffs and syntax highlighting; mermaid's and xterm's own DOM). Class names we
write become camelCase everywhere, `crates/render` included; library-generated
names (`tok-*` from syntect, mermaid, xterm) keep their generated names as a
documented exception. Tests import the modules they assert on. Every task
leaves typecheck, lint and tests green and the UI visually unchanged; `main.css`
shrinks each step until the last task deletes it.

## Tasks

- [x] 01: Carve the global sheets — [details](01-carve-global-sheets.md)
- [x] 02: Rename the renderer's own classes — [details](02-rename-renderer-classes.md)
- [x] 03: Status components — [details](03-status-components.md)
- [x] 04: Pane chrome component — [details](04-pane-chrome-component.md)
- [x] 05: Root and small components — [details](05-root-and-small-components.md)
- [ ] 06: The set/ directory — [details](06-set-directory.md)
- [ ] 07: Timeline and event rows — [details](07-timeline-and-event-rows.md)
- [ ] 08: Detail panes — [details](08-detail-panes.md)
- [ ] 09: Workbench shell — [details](09-workbench-shell.md)
- [ ] 10: Settings, profiles, repos — [details](10-settings-profiles-repos.md)
- [ ] 11: Delete main.css and audit — [details](11-delete-main-css-and-audit.md)
