# 02. Languages as plugins

## Goal

The seven descriptors in `languages.yaml` are six built-in **Language**
plugins, and the Plugins page has its first real rows. Rust and C++ are one
plugin, because they share the sccache Compile Server and one mechanism is one
thing to switch; Go, Node, Python, .NET and the JVM are one each. A Language
plugin is pure data: the descriptor grammar as it stands becomes the `language`
block of a `provides:` entry, and nothing a descriptor could not say before can
be said now. The Sandbox page's per-language checkbox goes — the tick on the
Plugins page is now whether sessions get the language — and the store's `size`
moves to a declared settings field on the plugin, drawn in its pane. An
installer's `languages:` entry in `config.yaml` still merges, key by key, over
the plugin of the same name. The sccache capability stays server behaviour a
descriptor switches on by name.

## Decisions in force

- **Languages are a Category** (Set 1760, Q7). The descriptor was already a
  plugin in all but name — data in one grammar, embedded for the built-ins and
  written by an installer for the rest — which is why it is the first Category
  to land: it proves the Plugins page over real rows with no component involved.
- **One switch, on the Plugins page** (Set 1762, Q11a). Two switches for one
  fact — a plugin tick and a Sandbox checkbox — was turned down; the Sandbox
  page's **Build caches** part loses its language rows, and what it keeps is
  what is not a language's.
- **One plugin per language, except the sccache pair** (Set 1762, Q11b, and the
  human's words beside it: *group the sccache languages into one, since one
  mechanism here supports multiple languages*). Rust and C++ both name the
  `sccache` capability in `languages.yaml` and nothing else does.
- **A descriptor cannot say a command to run.** `languages.yaml`'s own header
  says so, and the plugin grammar keeps it: anything that is behaviour — the
  Compile Server — is built into the server and named by a capability. A
  Language plugin asks for no WASI capability, because it runs nothing.
- **`size` is a declared settings field** of kind `size`, in sccache's own
  grammar, refused where it is not one — the Sandbox page's rule, moved
  ([ADR-0030](../../adr/0030-plugins.md), *Language*). The `rust_build_cache`
  key in `config.yaml` is still read as Rust's two, since installations from
  before wrote it.
- **The installer's override keeps merging.** A `languages:` entry in
  `config.yaml` merges over the built-in of the same name key by key, a
  variable set to `null` taken out, exactly as before; what changes is that the
  thing it merges over is a plugin's block rather than an embedded file's. An
  entry naming a language no plugin provides is drawn as a plugin of its own,
  from the installer, in the Installed group, so an installer that adds a
  language still gets its box with no code change — the one promise the
  descriptor made that this stage must keep.
- **Both go cluster-wide** (Set 1771, Q2). A language's tick and its store
  size live in each device's `config.yaml` today; from here the tick is the
  plugin record's enabled flag and the size is a plugin setting on that
  record, so both are the cluster's, like every other plugin's. A device short
  of disk turns a language off, or its cache down, for every device. Keeping
  the size per device was offered and turned down.
- **The `enabled` key and the size** in `config.yaml` are taken up into the
  record once at the first start after this lands, the way each Replicated
  record kind took up its `config.yaml` key (ADR-0028), and are not written
  again. Devices that disagree resolve **on wins** — a language any device had
  on is on — and **the larger size wins**, so the take-up never turns off or
  shrinks a cache somebody was using.

## Proposed tasks (provisional)

1. **The `language` block** — the descriptor grammar as a `provides:` entry, the
   six built-in plugins embedded, `languages.yaml` retired as a file. AC: a
   session under Rust gets the variables it got before; the embedded plugins
   pass `verkstead plugin check`; the Rust-and-C++ plugin names `sccache` once.
2. **The rows** — six built-in rows on the Plugins page, each saying which
   languages it provides. AC: the page lists them under Built-in; the tick on
   one is what a new session reads.
3. **The size field** — `size` as a declared settings field on each plugin,
   drawn in its pane, validated as before. AC: a size saved on the pane is
   what the next session's `{size}` reads; one that is not a size is refused
   with the reason; `rust_build_cache` from an old `config.yaml` still reads.
4. **The Sandbox page** — the language checkboxes and size fields leave the
   **Build caches** part; what is not a language's stays. AC: the pane has no
   language rows; the Compile Server's own state is still shown where it was.
5. **The override and the take-up** — `languages:` merges over the plugin of
   the same name, an unknown name becomes an installer-provided plugin,
   `enabled` and the size are taken up once. AC: an installer's variable
   override still wins; a language enabled `false` on every device before this
   lands is unticked after it, and one enabled on any device is ticked; two
   devices' sizes take up as the larger; a `languages:` entry naming an eighth
   language draws an eighth row.

## Re-verify at start

- Stage 01 landed: a Category is a WIT world and a `provides:` block kind, and a
  settings field of kind `size` is drawn by the pane.
- `languages.yaml` still has seven entries and only Rust and C++ name
  `sccache`.
- The Sandbox page's **Build caches** part is still where the per-language
  checkbox and size live (`web/src/settings/Languages.tsx`), and
  `rust_build_cache` is still read for Rust's two.
- Installer overrides still merge key by key with `null` taking a variable out
  (`crates/server/src/languages.rs`), and the settings page still writes only
  `enabled` and `size` into an entry.
- No installer in the wild writes a key the plugin grammar renames.
