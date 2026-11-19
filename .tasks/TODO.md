# The Activity interface, and follow-up rounds across

A plugin may provide an **Activity**: a wasm component the server starts with a
Conversation's facts, hands events to, lends a fixed set of host functions, and
watches end in one of the **Outcomes** its manifest declares. This stage defines
the `activity` provides-block and its WIT world. It puts every session launch
the runner makes behind the bindgen-generated trait of that world's host
functions, and hosts a component over the same trait. Then **follow-up rounds**
cross as the first built-in component: a Conversation steered into Follow-up, or
a Tinker started from a Draft, runs its rounds under the component, and the
sessions suite passes unchanged.

Nothing anybody can see on the workbench moves. What can be shown is a
`plugin.yaml` naming an Activity, the Plugins page listing *Follow-up rounds*
under Built-in, and a session launched by a component that reached the server
through nothing but the host functions. Settled in Set 1803:

- **host side only** (Q2): uncrossed drivers are not re-cast as in-process Activities;
- **one built-in Activities plugin with a block per Activity, each block naming its own component** (Q3);
- **no Forge or own-Set host functions until stage 09** (Q4);
- **the Activity writes its own prompt** (Q5);
- **a `stop` host function** (Q6);
- **`pushed` stays the host's** (Q7).

Roadmap stage: [08: The Activity interface and follow-up rounds across](docs/roadmaps/plugins/08-the-activity-interface.md)

## Tasks

- [ ] 01: The `activity` world and block — [details](01-the-activity-world-and-block.md)
- [ ] 02: Every launch through the host trait — [details](02-every-launch-through-the-host-trait.md)
- [ ] 03: A component hosted end to end — [details](03-a-component-hosted-end-to-end.md)
- [ ] 04: Follow-up rounds as a component — [details](04-follow-up-rounds-as-a-component.md)
- [ ] 05: Restart without state — [details](05-restart-without-state.md)
