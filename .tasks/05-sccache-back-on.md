# 05. sccache back on for Windows

## What to build

The shared Rust build cache works on Windows again. It was switched off for that
platform for one reason — a container is refused the loopback interface its
client reaches the Compile Server over — and the session account is refused
nothing of the sort: the probe dialled loopback from inside and connected, and
sccache's own client reached a server started outside.

So the platform switch flips, the Compile Server is started on Windows the way
it is on the other two, and a Windows session gets `RUSTC_WRAPPER` pointing at
the sccache bound in read-only beside Verkstead's own binary rather than only
the shared `CARGO_HOME` it has had until now. The Compile Server gets a boundary
of its own on this platform as it does on the others.

One thing the earlier probe found is worth keeping in view while this is built:
sccache's client failed inside a container *before* it reached the network,
unable to find its configuration directory, because the environment it was
handed pointed at a profile the boundary refused. Under the account it works
because the fresh profile is granted — so this is a task where the environment a
session is handed matters as much as the switch.

The viewer says, on its build-cache settings pane and in its setup instructions,
that sessions on this machine cannot use sccache and why. That copy stops being
true here, and the wider documentation sweep in task 06 is where the rest of the
AppContainer wording goes.

## Acceptance criteria

- [ ] The platform switch says Windows compiles through an sccache, and the
  Compile Server is started there
- [ ] A Windows session is handed `RUSTC_WRAPPER` and the sccache it names is
  reachable inside
- [ ] A Rust repository's session builds through the Compile Server, and the
  cache is warmer afterwards than before
- [ ] The build-cache settings pane and the setup instructions no longer say
  Windows sessions cannot reach one
