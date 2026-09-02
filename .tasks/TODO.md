# Path selector

Every path in the workbench is typed blind into a bare text field today. This
adds one shared field component that keeps the typed input exactly as it is and
extends it with a drill-in browse dropdown: the dropdown shows the entries of
the deepest directory the field currently names, a tap writes that path into
the field and opens it, and the human closes the dropdown once the field holds
what they want. Behind it, a new server endpoint lists one directory per
request in two scopes — inside the Watched Paths for fields the server refuses
outside them, and anywhere for fields it does not.

This consciously revises the written "typed rather than picked" stance: the
module headers that state it are rewritten as each of their fields adopts the
component.

## Tasks

- [x] 01: Directory listing endpoint — [details](01-directory-listing-endpoint.md)
- [x] 02: Browse dropdown on the Paths section — [details](02-browse-dropdown-paths-section.md)
- [x] 03: The Repos' form browses — [details](03-repos-form-browses.md)
- [ ] 04: Agent Profile account paths browse — [details](04-profile-account-paths-browse.md)
