# 09. The settings pane

## What to build

Opening a plugin's row draws a pane with the **settings fields its manifest
declares**, each drawn by kind: **text**, **size** (the human-readable sizes the
Sandbox page's build cache size already reads), **list**, and **secret**. The
values are the cluster's (ADR-0030, *Language*), written on the plugin's record
and keyed by the plugin's uuid and the field's key. Only the server's grammar
decides what a value may be, and a refused value says why.

**A secret goes where MCP header values go** (ADR-0021, ADR-0028). It is written
to `secrets.yaml` under the plugin's uuid and never returned to the viewer, which
learns only whether one is set. The record carries a **digest**, and a device
whose copy does not match fetches the value from a device that holds it, the way
`replicating/mcp_servers.rs` copies header values. Author tokens are not plugin
settings and stay in the Git section.

Fields whose plugin is in trouble are still drawn, so a setting that a later
stage's Category needs to load can be put right from the pane.

## Acceptance criteria

- [ ] A text field round-trips through the pane and is read on a second device.
- [ ] A secret is written to `secrets.yaml`, never comes back to the viewer, and
      reaches a second device through the digest-and-fetch path.
- [ ] A size that is not a size is refused, with the reason on the field.
