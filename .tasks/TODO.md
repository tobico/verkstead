# The Agent interface

A harness becomes a **Plugin** of the Agent **Category**: a `provides` block in
a `plugin.yaml` holding everything about the harness that is data, and a wasm
component exporting the three pieces that are code — `parse-transcript`,
`write-configuration` and `login`. The server reaches every harness through one
Rust trait, which by the end of this stage has six implementations: the five
built-ins moved behind it with their behaviour byte-for-byte unchanged, and a
wasm-backed one that runs whatever a manifest and a component declare.

A third-party harness installed from a repository then launches a real session.
Its Profile is made on a form that draws one field per declared account path.
Its Pairing is picked on the composer under the mark it ships. `verkstead guide`
inside its sandbox prints the *Running the ask* section its manifest wrote. Its
transcript is drawn on the Timeline out of what its `parse-transcript` export
answered. `AgentType` is no longer a closed word list anywhere, and the viewer's
hand-kept tables keyed by it are gone. Nothing crosses yet: the five built-ins
stay in-process Rust until stages 04 to 06.

Roadmap stage: [03: The Agent interface](docs/roadmaps/plugins/03-the-agent-interface.md)

## Tasks

- [x] 01: The trait, launch half — [details](01-the-trait-launch-half.md)
- [x] 02: The trait, account half — [details](02-the-trait-account-half.md)
- [x] 03: The id as a string, the account as paths — [details](03-the-id-as-a-string.md)
- [x] 04: The `agent` provides-block and the WIT — [details](04-the-agent-provides-block.md)
- [x] 05: A plugin harness launches — [details](05-a-plugin-harness-launches.md)
- [x] 06: Its Transcript drawn — [details](06-its-transcript-drawn.md)
- [x] 07: Its login — [details](07-its-login.md)
- [x] 08: Models and the Guide — [details](08-models-and-the-guide.md)
- [x] 09: Detection, install, disable and remove — [details](09-detection-install-disable-remove.md)
