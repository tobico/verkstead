# 01. Rust as an embedded descriptor

## What to build

The descriptors Verkstead ships become one YAML file embedded in the binary, in
the grammar an installer writes, and a session's environment is built out of
what that file says rather than out of the build cache module's own constants.
Rust is the only entry in it, and what it says is exactly what a session gets
today.

A descriptor is **data**: a label, the manifests that detect the language in a
Repo, the variables a session is given, and the **capabilities** it names.
Behaviour is built into the server and switched on by name — Rust's descriptor
names the sccache capability, and the **Compile Server** stays Rust code. Most
of the build cache module is that half and survives as it is.

**This is where the grammar's spelling is settled** — the key names, how a
capability is named, and the placeholders. Write it as the documentation's
worked example, because everything an installer ever writes is in this grammar
and stage 05 of the roadmap adds an eviction unit to the same entry.

**Two placeholders, not one.** One names the **Build Cache**, which is where
Rust's own store goes. The second names a directory beside the **Worktrees**,
because the two are not on one filesystem on every machine — they are not on
this one — and the package managers stage 02 brings hardlink out of their store
into the project, falling back to a full copy across a filesystem boundary. A
placeholder's directory is made and granted writable to a Sandbox **only where
a loaded descriptor names it**, so with Rust alone built in the second grants
nothing and no Sandbox changes shape.

**The Compile Server starts on the switch rather than on detection.** It comes
up wherever a language naming the sccache capability is enabled and there is an
sccache to run, whatever the Repo holds. Detection survives for the setup
card's warning and for nothing else: a Repo whose manifest is not at its root is
handed the wrapper variable all the same, and with no server of Verkstead's up
the sccache client inside starts one in its own Sandbox — which is the hazard
the Compile Server exists to remove.

Nothing about `config.yaml` changes in this task. The next one is where the
built-ins can be overridden.

## Acceptance criteria

- [ ] A session's environment is byte for byte what it was on all three
      platforms, with the variables now coming from the embedded descriptor.
- [ ] The Compile Server starts for any session where Rust is on and there is an
      sccache — a Repo with no `Cargo.toml` at its root included — and on no
      machine without one. The setup card still warns only for a Repo that
      builds Rust.
- [ ] The grammar carries both placeholders, and the one beside the Worktrees
      grants nothing while no loaded descriptor names it, so the sandbox suite
      passes unchanged. What changes in the Windows sessions suite is its
      assertions about when the Compile Server starts and nothing else.
