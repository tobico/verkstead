# 04. Headers, as secrets

## What to build

A declared MCP server gains **headers** — what an API key or a bearer token is
sent in. Static headers are the only authentication there is: OAuth was turned
down in the grilling, so a server that offers nothing else cannot be used.

**Every header value is a secret**, with no plain kind. It follows the GitHub
token's precedent in full:

- Values are kept in `secrets.yaml`, never in `config.yaml`, under the server's
  name.
- **A value never comes back over the wire.** What the page is told about a
  server's headers is their names, and no more of a value than the token's view
  gives of the token.
- An edit is **an action rather than a value**, one per header: keep it, set
  it, or clear it. A value box left blank keeps what is there, so correcting a
  URL does not take a key away.
- `secrets.yaml` is written whole, so saving a server must leave the GitHub
  token and the session account's password as they were, and saving the token
  must leave the headers. The secrets there are a flat pair of fields today; a
  set of headers per server is a new shape for that file, and an empty one
  still has to read as nothing set.

**Deleting a server takes its headers with it**, so declaring the same name
again starts with none.

**The session is handed the values**, because the harness has to send them.
Task 03 writes each attached server into a Claude root; it now carries the
headers too:

```json
{"type": "http", "url": "<url>", "headers": {"Authorization": "Bearer …"}}
```

That is the limit of the secrecy, and it was said out loud in the grilling: the
values are kept from the page and the wire, **not from the agent**, which can
read its own configuration. Written literally into the root is fine for that
reason; whatever is written must not be readable more widely than the root's
other credentials are.

## Acceptance criteria

- [ ] Headers can be added to a server, and a reload shows their names and none
      of their values.
- [ ] No response from the settings endpoints holds a header's value, whole.
- [ ] A header can be kept, set or cleared on its own, and a blank value box
      keeps what was there.
- [ ] Saving a server leaves the GitHub token as it was, and saving the token
      leaves every server's headers as they were.
- [ ] Deleting a server removes its header values from `secrets.yaml`.
- [ ] A Claude session's root carries the attached server's headers, and a
      request the harness makes to the server arrives with them.
