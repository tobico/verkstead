//! A Plugin's wasm component, instantiated under what its manifest asked for
//! and nothing else (ADR-0030, *Capabilities and trust*).
//!
//! **What a component can reach is its `capabilities:` and no more.** A
//! `filesystem:` path is a preopen, read-only or read-write as written, and is
//! the only directory the component holds a handle on: WASI has no ambient
//! filesystem, so a path nobody granted is not refused so much as unnameable.
//! An `environment:` name is that one variable, with this server's value. An
//! `outbound:` host is reached through WASI HTTP, whose request to any other
//! host is refused before a connection is made — see [`Outbound`]. Raw sockets
//! and name lookups are not granted at all, and nothing of the server's stdio,
//! arguments or environment is inherited.
//!
//! **What fails is [`Trouble`], never a panic and never an error that stops
//! the server** (Set 1762, Q15): bytes that are not a component, a component
//! importing what this host does not provide, one that traps while it starts,
//! and one that is still starting after [`STARTING`] — stopped there, so a
//! component that never finishes cannot hold up every load after it.
//! The same sentence is what `verkstead plugin check` prints and what the
//! Plugins page draws on the row.
//!
//! **wasmtime's errors are its own type**, not `std::error::Error`, so they
//! are worded into a [`Trouble`] here, at the loader's edge, and go no further.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use sha2::{Digest, Sha256};

use wasmtime::component::{Component, Instance, InstancePre, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::{
    WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpView, default_hooks,
};

use super::manifest::{Access, Capability, Category, Files, Manifest};

/// The bindings to `wit/plugin.wit`: the base world every component is checked
/// against, and one world per Category, each filled in by the stage that gives
/// its Category a grammar.
pub mod worlds {
    wasmtime::component::bindgen!({ path: "wit", world: "plugin" });

    /// The Category worlds, which the stages that give each Category its
    /// grammar fill in.
    pub mod category {
        /// An agent's three exports, called on the store the component was
        /// started in, which is an async one.
        pub mod agent {
            wasmtime::component::bindgen!({
                path: "wit",
                world: "agent",
                exports: { default: async },
            });
        }
        pub mod process {
            wasmtime::component::bindgen!({ path: "wit", world: "process" });
        }
        pub mod forge {
            wasmtime::component::bindgen!({ path: "wit", world: "forge" });
        }
        pub mod language {
            wasmtime::component::bindgen!({ path: "wit", world: "language" });
        }
        pub mod activity {
            wasmtime::component::bindgen!({ path: "wit", world: "activity" });
        }
        pub mod skills {
            wasmtime::component::bindgen!({ path: "wit", world: "skills" });
        }
    }
}

/// Why a plugin's component did not load: screen copy, drawn on its row as it
/// is and printed by `verkstead plugin check`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouble {
    pub why: String,
}

impl Trouble {
    fn of(why: impl Into<String>) -> Self {
        Trouble { why: why.into() }
    }
}

impl std::fmt::Display for Trouble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.why)
    }
}

impl std::error::Error for Trouble {}

/// A component instantiated and standing, with the store it lives in.
pub struct Loaded {
    pub store: Store<Host>,
    pub instance: Instance,

    /// What starts the same component again — see [`Again`].
    pub again: Again,
}

/// What starts a loaded component again, in a store of its own, under the
/// same grant with one directory more.
///
/// **For an export handed a path the grant never named**: a harness's
/// `parse-transcript` finds a session's record under the account it ran on,
/// and which account that is is the session's, not the plugin's. WASI's
/// directories are fixed when a store is made, so the one to read is a store
/// made for it — compiled once, at load, and instantiated per reader.
#[derive(Clone)]
pub struct Again {
    pre: InstancePre<Host>,
    capabilities: Vec<Capability>,
    home: Option<PathBuf>,

    /// The SHA-256 of the component's bytes, which is what says whether a
    /// plugin read again is running the same code — see [`Again::same_as`].
    digest: [u8; 32],
}

impl Again {
    /// Whether `other` starts the same component as this, under the same
    /// grant: a plugin **Update** may change its component and nothing in its
    /// manifest, and what it was loaded as is then not what it is.
    pub fn same_as(&self, other: &Again) -> bool {
        self.digest == other.digest
            && self.capabilities == other.capabilities
            && self.home == other.home
    }

    /// The component started again with `dir` granted read-only beside what
    /// its manifest was granted, named by its own path.
    pub async fn reading(&self, dir: &Path) -> Result<Loaded, Trouble> {
        let mut grant = grant(&self.capabilities, self.home.as_deref())?;
        grant
            .wasi
            .preopened_dir(dir, dir.to_string_lossy(), FsPerms::ReadOnly)
            .map_err(|error| {
                Trouble::of(format!(
                    "`{}` couldn't be opened for the component: {}.",
                    dir.display(),
                    worded(&error)
                ))
            })?;

        started(self.clone(), grant, STARTING).await
    }

    /// The component started again under what its manifest was granted and
    /// nothing more: a store of its own, for an export whose answers build on
    /// what it was asked before — a harness's `login`, one per login.
    pub async fn afresh(&self) -> Result<Loaded, Trouble> {
        let grant = grant(&self.capabilities, self.home.as_deref())?;
        started(self.clone(), grant, STARTING).await
    }
}

/// What a component's store holds: its WASI context, built from the grant, and
/// the outbound hosts its HTTP requests are held to.
pub struct Host {
    wasi: WasiCtx,
    http: WasiHttpCtx,
    outbound: Outbound,
    table: ResourceTable,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Host {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.outbound,
        }
    }
}

/// The hosts a component's manifest named under `outbound:`. A request to one
/// of them is sent; a request to anything else is refused as denied, without a
/// connection being opened.
#[derive(Debug, Clone, Default)]
pub struct Outbound {
    hosts: Vec<String>,
}

impl Outbound {
    fn permits(&self, host: Option<&str>) -> bool {
        host.is_some_and(|host| {
            self.hosts
                .iter()
                .any(|granted| granted.eq_ignore_ascii_case(host))
        })
    }
}

impl WasiHttpHooks for Outbound {
    fn send_request(
        &mut self,
        request: http::Request<wasmtime_wasi_http::WasiBody>,
        options: Option<wasmtime_wasi_http::RequestOptions>,
        fut: Box<dyn Future<Output = wasmtime_wasi_http::Result<()>> + Send>,
    ) -> Box<
        dyn Future<
                Output = wasmtime_wasi_http::Result<(
                    http::Response<wasmtime_wasi_http::WasiBody>,
                    Box<dyn Future<Output = wasmtime_wasi_http::Result<()>> + Send>,
                )>,
            > + Send,
    > {
        if self.permits(request.uri().host()) {
            default_hooks().send_request(request, options, fut)
        } else {
            Box::new(async { Err(wasmtime_wasi_http::Error::HttpRequestDenied) })
        }
    }
}

/// How long a component may take to start before it is stopped and is
/// trouble.
pub const STARTING: Duration = Duration::from_secs(10);

/// How often the engine's epoch moves on, which is how finely a deadline is
/// kept.
pub(crate) const TICK: Duration = Duration::from_millis(100);

/// The one engine every component is compiled by, compiled to stop at an epoch
/// deadline, and the thread that moves its epoch on.
static ENGINE: LazyLock<Engine> = LazyLock::new(|| {
    let engine = Engine::new(Config::new().epoch_interruption(true))
        .expect("the engine's configuration is this build's own");

    let ticking = engine.clone();
    std::thread::Builder::new()
        .name("plugin-epoch".to_owned())
        .spawn(move || {
            loop {
                std::thread::sleep(TICK);
                ticking.increment_epoch();
            }
        })
        .expect("the epoch thread should start");

    engine
});

/// Instantiate `manifest`'s component, read out of `files` — an installed
/// plugin's directory or a built-in's embedded files, the one loader for both —
/// under its capabilities, with `home` as what a leading `~` in a granted path
/// means. `Ok(None)` is a plugin with no component, which has nothing to load.
pub async fn load(
    files: Files<'_>,
    manifest: &Manifest,
    home: Option<&Path>,
) -> Result<Option<Loaded>, Trouble> {
    let Some(file) = &manifest.component else {
        return Ok(None);
    };

    let bytes = files.read(file).ok_or_else(|| {
        Trouble::of(format!(
            "The component `{}` couldn't be read.",
            file.display()
        ))
    })?;

    let mut loaded = instantiate(&bytes, &manifest.capabilities, home).await?;
    exports_what_it_provides(&mut loaded, manifest)?;
    Ok(Some(loaded))
}

/// Refused where `manifest` provides a Category whose world has exports and
/// the component does not export them: an agent is its three exports, and
/// one missing would be a harness that fails the first time a session needs
/// it rather than at load.
fn exports_what_it_provides(loaded: &mut Loaded, manifest: &Manifest) -> Result<(), Trouble> {
    let agent = manifest
        .provides
        .iter()
        .any(|provision| provision.category == Category::Agent);
    if agent {
        worlds::category::agent::Agent::new(&mut loaded.store, &loaded.instance).map_err(
            |error| {
                Trouble::of(format!(
                    "The component provides an agent and doesn't export what one needs: {}.",
                    worded(&error)
                ))
            },
        )?;
    }
    Ok(())
}

/// Instantiate a component's bytes under `capabilities`, stopped where it is
/// still starting after [`STARTING`].
pub async fn instantiate(
    bytes: &[u8],
    capabilities: &[Capability],
    home: Option<&Path>,
) -> Result<Loaded, Trouble> {
    started_within(bytes, capabilities, home, STARTING).await
}

async fn started_within(
    bytes: &[u8],
    capabilities: &[Capability],
    home: Option<&Path>,
    within: Duration,
) -> Result<Loaded, Trouble> {
    let component = Component::new(&ENGINE, bytes).map_err(|error| {
        Trouble::of(format!(
            "The component isn't a WebAssembly component Verkstead can load: {}.",
            worded(&error)
        ))
    })?;

    let mut linker = Linker::<Host>::new(&ENGINE);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(internal)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker).map_err(internal)?;

    // What the component imports is checked here, before anything runs, so
    // a refusal at this point is about what it asked for rather than what it
    // did.
    let pre = linker.instantiate_pre(&component).map_err(|error| {
        Trouble::of(format!(
            "The component needs something Verkstead doesn't provide: {}.",
            worded(&error)
        ))
    })?;

    let again = Again {
        pre,
        capabilities: capabilities.to_vec(),
        home: home.map(Path::to_owned),
        digest: Sha256::digest(bytes).into(),
    };
    let grant = grant(capabilities, home)?;
    started(again, grant, within).await
}

/// `again`'s component started in a store holding `grant`, stopped where it
/// is still starting after `within`.
async fn started(again: Again, grant: Grant, within: Duration) -> Result<Loaded, Trouble> {
    let mut store = Store::new(&ENGINE, grant.host());
    store.epoch_deadline_trap();
    store.set_epoch_deadline(within.div_duration_f64(TICK).ceil() as u64);

    let instance = again
        .pre
        .instantiate_async(&mut store)
        .await
        .map_err(|error| {
            // A trap is worded as the trap alone: what wasmtime wraps around it
            // is a backtrace, which is the component author's to read with their
            // own tools rather than a row's to draw.
            if let Some(Trap::Interrupt) = error.downcast_ref::<Trap>() {
                Trouble::of(format!(
                    "The component didn't finish starting within {} seconds, so it was stopped.",
                    within.as_secs()
                ))
            } else if let Some(trap) = error.downcast_ref::<Trap>() {
                Trouble::of(format!(
                    "The component stopped while it was starting: {trap}."
                ))
            } else {
                Trouble::of(format!(
                    "The component couldn't be started: {}.",
                    worded(&error)
                ))
            }
        })?;

    worlds::Plugin::new(&mut store, &instance).map_err(|error| {
        Trouble::of(format!(
            "The component isn't written against Verkstead's plugin world: {}.",
            worded(&error)
        ))
    })?;

    // Started, it has no deadline: what a Category later calls into it for
    // sets its own.
    store.set_epoch_deadline(u64::MAX);

    Ok(Loaded {
        store,
        instance,
        again,
    })
}

/// What a store is to be made holding: the WASI context a grant builds, and
/// the outbound hosts its HTTP requests are held to.
struct Grant {
    wasi: WasiCtxBuilder,
    outbound: Outbound,
}

impl Grant {
    fn host(mut self) -> Host {
        Host {
            wasi: self.wasi.build(),
            http: WasiHttpCtx::new(),
            outbound: self.outbound,
            table: ResourceTable::new(),
        }
    }
}

/// What the store a component runs in holds: exactly what `capabilities`
/// grant.
fn grant(capabilities: &[Capability], home: Option<&Path>) -> Result<Grant, Trouble> {
    let mut wasi = WasiCtxBuilder::new();
    wasi.allow_tcp(false)
        .allow_udp(false)
        .allow_ip_name_lookup(false);
    let mut outbound = Outbound::default();

    for capability in capabilities {
        match capability {
            Capability::Filesystem { path, access } => {
                let host = expanded(path, home);
                // A granted directory that isn't there yet is nothing to hand
                // over, rather than a plugin that cannot load: a cache it
                // will make on first use is the ordinary case.
                if !host.is_dir() {
                    if host.exists() {
                        return Err(Trouble::of(format!(
                            "`{path}` is granted as a directory, and it isn't one."
                        )));
                    }
                    continue;
                }
                let perms = match access {
                    Access::ReadOnly => FsPerms::ReadOnly,
                    Access::ReadWrite => FsPerms::ReadWrite,
                };
                // The component names a granted directory by the path it
                // was granted as, `~` spelt out, so that what the manifest
                // says and what the code opens are the same words.
                wasi.preopened_dir(&host, host.to_string_lossy(), perms)
                    .map_err(|error| {
                        Trouble::of(format!(
                            "`{path}` couldn't be opened for the component: {}.",
                            worded(&error)
                        ))
                    })?;
            }
            Capability::Environment { name } => {
                if let Ok(value) = std::env::var(name) {
                    wasi.env(name, value);
                }
            }
            Capability::Outbound { host } => outbound.hosts.push(host.clone()),
        }
    }

    Ok(Grant { wasi, outbound })
}

/// A granted path with a leading `~` read as `home`.
fn expanded(path: &str, home: Option<&Path>) -> PathBuf {
    let rest = path
        .strip_prefix("~/")
        .or_else(|| path.strip_prefix("~\\"))
        .or_else(|| (path == "~").then_some(""));
    match (rest, home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

/// wasmtime's account of an error, its causes included, as one line.
pub(crate) fn worded(error: &wasmtime::Error) -> String {
    format!("{error:#}").trim_end_matches('.').to_owned()
}

/// A failure of the host's own setup, which no component can cause.
fn internal(error: wasmtime::Error) -> Trouble {
    Trouble::of(format!(
        "Verkstead couldn't prepare to run the component: {}.",
        worded(&error)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A component with nothing in it, which is the least that loads.
    const EMPTY: &str = "(component)";

    /// A component whose core module traps in its start function, which runs
    /// as the component is instantiated.
    const TRAPS: &str = r#"
(component
  (core module $m
    (func $start unreachable)
    (start $start))
  (core instance (instantiate $m)))
"#;

    /// A component whose core module never finishes its start function.
    const SPINS: &str = r#"
(component
  (core module $m
    (func $start (loop $forever (br $forever)))
    (start $start))
  (core instance (instantiate $m)))
"#;

    /// The slice of `wasi:filesystem` the reader below imports, the types
    /// written exactly as WASI's own, which is what the linker checks them
    /// against.
    const FILESYSTEM: &str = r#"
package wasi:filesystem@0.2.0;

interface types {
  type filesize = u64;

  flags descriptor-flags {
    read, write, file-integrity-sync, data-integrity-sync,
    requested-write-sync, mutate-directory,
  }
  flags path-flags { symlink-follow }
  flags open-flags { create, directory, exclusive, truncate }

  enum error-code {
    access, would-block, already, bad-descriptor, busy, deadlock, quota,
    exist, file-too-large, illegal-byte-sequence, in-progress, interrupted,
    invalid, io, is-directory, loop, too-many-links, message-size,
    name-too-long, no-device, no-entry, no-lock, insufficient-memory,
    insufficient-space, not-directory, not-empty, not-recoverable,
    unsupported, no-tty, no-such-device, overflow, not-permitted, pipe,
    read-only, invalid-seek, text-file-busy, cross-device,
  }

  resource descriptor {
    read: func(length: filesize, offset: filesize)
      -> result<tuple<list<u8>, bool>, error-code>;
    open-at: func(path-flags: path-flags, path: string,
      open-flags: open-flags, %flags: descriptor-flags)
      -> result<descriptor, error-code>;
  }
}

interface preopens {
  use types.{descriptor};
  get-directories: func() -> list<tuple<descriptor, string>>;
}
"#;

    const READER_WORLD: &str = r#"
package fixture:reader;

world reader {
  import wasi:filesystem/types@0.2.0;
  import wasi:filesystem/preopens@0.2.0;
  export read: func(path: string) -> s64;
}
"#;

    /// `read(path)` opens `path` under the first directory the component was
    /// handed and answers how many bytes it read — `-1` where it was handed
    /// none, and `-2 - code` where WASI refused it with `error-code` `code`.
    const READER: &str = r#"
(module
  (import "wasi:filesystem/preopens@0.2.0" "get-directories"
    (func $directories (param i32)))
  (import "wasi:filesystem/types@0.2.0" "[method]descriptor.open-at"
    (func $open (param i32 i32 i32 i32 i32 i32 i32)))
  (import "wasi:filesystem/types@0.2.0" "[method]descriptor.read"
    (func $read (param i32 i64 i64 i32)))
  (memory (export "memory") 1)
  (global $next (mut i32) (i32.const 1024))

  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32)
    (result i32)
    (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $next) (i32.sub (local.get $align) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get $align))))
    (global.set $next (i32.add (local.get $at) (local.get $size)))
    (local.get $at))

  (func (export "read") (param $path i32) (param $length i32) (result i64)
    ;; The list of directories: its pointer at 16, its length at 20.
    (call $directories (i32.const 16))
    (if (i32.eqz (i32.load (i32.const 20)))
      (then (return (i64.const -1))))

    ;; Open the path for reading under the first one's handle.
    (call $open
      (i32.load (i32.load (i32.const 16)))
      (i32.const 0)
      (local.get $path) (local.get $length)
      (i32.const 0)
      (i32.const 1)
      (i32.const 32))
    (if (i32.load8_u (i32.const 32))
      (then (return
        (i64.sub (i64.const -2) (i64.extend_i32_u (i32.load8_u (i32.const 36)))))))

    ;; Read it: the bytes' length lands at 56.
    (call $read (i32.load (i32.const 36)) (i64.const 4096) (i64.const 0) (i32.const 48))
    (if (i32.load8_u (i32.const 48))
      (then (return
        (i64.sub (i64.const -2) (i64.extend_i32_u (i32.load8_u (i32.const 52)))))))
    (i64.extend_i32_u (i32.load (i32.const 56)))))
"#;

    /// `error-code`'s `not-permitted`, which is what a path out of a granted
    /// directory is refused as.
    const NOT_PERMITTED: i64 = 31;

    fn assembled(wat: &str) -> Vec<u8> {
        wat::parse_str(wat).expect("the fixture should assemble")
    }

    /// The reader, made into a component against its WIT.
    fn reader() -> Vec<u8> {
        let mut resolve = wit_parser::Resolve::default();
        resolve
            .push_str("filesystem.wit", FILESYSTEM)
            .expect("the WASI slice should parse");
        let package = resolve
            .push_str("reader.wit", READER_WORLD)
            .expect("the reader's world should parse");
        let world = resolve
            .select_world(&[package], Some("reader"))
            .expect("the reader's world should be there");

        let mut module = assembled(READER);
        wit_component::embed_component_metadata(
            &mut module,
            &resolve,
            world,
            wit_component::StringEncoding::UTF8,
            false,
        )
        .expect("the reader's world should embed");

        wit_component::ComponentEncoder::default()
            .module(&module)
            .expect("the reader should be a module wit-component reads")
            .validate(true)
            .encode()
            .expect("the reader should encode as a component")
    }

    /// A component exporting the `agent` world, made from `wit/` itself by
    /// wit-component: each export traps if it is called, which nothing here
    /// does — what a component providing an agent is checked for at load is
    /// that it has them.
    fn exporting_an_agent() -> Vec<u8> {
        let mut resolve = wit_parser::Resolve::default();
        let (package, _) = resolve
            .push_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("wit"))
            .expect("the plugin WIT should parse");
        let world = resolve
            .select_world(&[package], Some("agent"))
            .expect("the agent world should be there");

        let mut module =
            wit_component::dummy_module(&resolve, world, wit_parser::ManglingAndAbi::Standard32);
        wit_component::embed_component_metadata(
            &mut module,
            &resolve,
            world,
            wit_component::StringEncoding::UTF8,
            false,
        )
        .expect("the agent world should embed");

        wit_component::ComponentEncoder::default()
            .module(&module)
            .expect("the dummy should be a module wit-component reads")
            .validate(true)
            .encode()
            .expect("the dummy should encode as a component")
    }

    /// The agent manifest of the grammar's tests, its component `bytes`.
    async fn load_agent(bytes: &[u8]) -> Result<Option<Loaded>, Trouble> {
        let files: &[(&str, &[u8])] = &[
            ("mark.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            ("agent.wasm", bytes),
        ];
        let manifest = Manifest::parse(
            crate::plugins::manifest::agent::tests::AGENT,
            Files::Embedded(files),
            crate::platform::Platform::HERE,
        )
        .expect("the agent manifest should be read");

        load(Files::Embedded(files), &manifest, None).await
    }

    #[tokio::test]
    async fn a_component_exporting_an_agent_loads_for_an_agent() {
        let loaded = load_agent(&exporting_an_agent()).await;

        assert!(matches!(loaded, Ok(Some(_))), "{:?}", loaded.err());
    }

    /// A plugin read again keeps the harness already standing only where its
    /// component is the same: an **Update** that changed nothing but the
    /// component stands up a new one, running the new code.
    #[tokio::test]
    async fn a_harness_read_again_with_another_component_is_replaced() {
        let agent = Manifest::parse(
            crate::plugins::manifest::agent::tests::AGENT,
            Files::Embedded(&[("mark.svg", b"<svg/>"), ("agent.wasm", b"")]),
            crate::platform::Platform::HERE,
        )
        .expect("the agent manifest should be read")
        .provides
        .iter()
        .find_map(|provision| provision.agent().cloned())
        .expect("it provides an agent");
        let uuid = "3a7d1c52-0e8b-4f6a-9d21-7b5c4e8f0a13";
        let standing = |bytes: Vec<u8>| {
            let agent = agent.clone();
            async move {
                let loaded = load_agent(&bytes).await.unwrap().unwrap();
                let id = crate::harness::plugged(uuid, &agent, "<svg/>".to_owned(), loaded);
                crate::harness::of(id).expect("it is stood up")
                    as *const dyn crate::harness::Harness as *const ()
            }
        };

        let first = standing(exporting_an_agent()).await;
        let same = standing(exporting_an_agent()).await;
        // The same component with a custom section on the end: other bytes,
        // the same exports.
        let mut changed = exporting_an_agent();
        changed.extend_from_slice(&[0, 5, 4, b'n', b'e', b'x', b't']);
        let updated = standing(changed).await;

        assert_eq!(first, same, "the same component keeps the harness standing");
        assert_ne!(first, updated, "another component replaces it");
    }

    /// A `write-configuration` that traps answers nothing, and the call after
    /// it is answered by a component started afresh rather than by the one
    /// that stopped, which would never be entered again.
    #[tokio::test]
    async fn a_harness_whose_configuration_trapped_configures_the_next_session() {
        use crate::harness::Harness as _;

        let stub = std::fs::read(env!("VERKSTEAD_STUB_HARNESS_WASM")).unwrap();
        let loaded = load_agent(&stub).await.unwrap().unwrap();
        let agent = Manifest::parse(
            crate::plugins::manifest::agent::tests::AGENT,
            Files::Embedded(&[("mark.svg", b"<svg/>"), ("agent.wasm", b"")]),
            crate::platform::Platform::HERE,
        )
        .expect("the agent manifest should be read")
        .provides
        .iter()
        .find_map(|provision| provision.agent().cloned())
        .expect("it provides an agent");
        let harness = crate::harness::plugged::Plugged::new(
            crate::store::AgentType::read("6c0e2f4a-8b1d-4e7a-a3c5-9f2d1b7e4c80"),
            agent,
            "<svg/>".to_owned(),
            loaded,
        );
        let settings = |found: &[u8]| {
            harness
                .configured(vec![Some(found.to_vec()), None], &[])
                .into_iter()
                .find(|(path, _)| path == Path::new("settings.json"))
                .map(|(_, contents)| contents)
        };

        assert_eq!(
            settings(br#"{"trap":true}"#),
            None,
            "a trap answers nothing"
        );
        assert_eq!(
            settings(br#"{"model":"large"}"#).as_deref(),
            Some(&serde_json::to_vec_pretty(&serde_json::json!({ "model": "large" })).unwrap()[..]),
            "the next call is answered"
        );
    }

    #[tokio::test]
    async fn a_component_exporting_nothing_is_refused_for_an_agent() {
        let trouble = load_agent(&assembled(EMPTY))
            .await
            .err()
            .expect("an agent with none of its exports should not load");

        assert!(
            trouble
                .why
                .starts_with("The component provides an agent and doesn't export what one needs"),
            "{trouble}"
        );
    }

    async fn read(loaded: &mut Loaded, path: &str) -> i64 {
        let read = loaded
            .instance
            .get_typed_func::<(&str,), (i64,)>(&mut loaded.store, "read")
            .expect("the reader exports `read`");
        let (answer,) = read
            .call_async(&mut loaded.store, (path,))
            .await
            .expect("`read` should answer");
        answer
    }

    /// A directory holding `granted/secret` and `ungranted/secret` beside it.
    fn two_directories() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for side in ["granted", "ungranted"] {
            std::fs::create_dir(root.path().join(side)).unwrap();
            std::fs::write(root.path().join(side).join("secret"), side).unwrap();
        }
        root
    }

    fn granting(dir: &Path) -> Vec<Capability> {
        vec![Capability::Filesystem {
            path: dir.to_string_lossy().into_owned(),
            access: Access::ReadOnly,
        }]
    }

    #[tokio::test]
    async fn a_component_exporting_nothing_loads() {
        let loaded = instantiate(&assembled(EMPTY), &[], None).await;

        assert!(loaded.is_ok(), "{:?}", loaded.err());
    }

    #[tokio::test]
    async fn a_granted_path_can_be_read() {
        let root = two_directories();
        let capabilities = granting(&root.path().join("granted"));

        let mut loaded = instantiate(&reader(), &capabilities, None)
            .await
            .expect("the reader should load");

        assert_eq!(read(&mut loaded, "secret").await, "granted".len() as i64);
    }

    #[tokio::test]
    async fn a_path_outside_what_was_granted_cannot_be_read() {
        let root = two_directories();
        let capabilities = granting(&root.path().join("granted"));

        let mut loaded = instantiate(&reader(), &capabilities, None)
            .await
            .expect("the reader should load");

        assert_eq!(
            read(&mut loaded, "../ungranted/secret").await,
            -2 - NOT_PERMITTED
        );
        let absolute = root.path().join("ungranted").join("secret");
        assert!(read(&mut loaded, &absolute.to_string_lossy()).await < 0);
    }

    #[tokio::test]
    async fn a_component_granted_nothing_has_no_directory_at_all() {
        let mut loaded = instantiate(&reader(), &[], None)
            .await
            .expect("the reader should load");

        assert_eq!(read(&mut loaded, "secret").await, -1);
    }

    #[test]
    fn a_tilde_is_the_home_directory() {
        let home = Path::new("/home/ada");

        assert_eq!(
            expanded("~/.config/glab", Some(home)),
            home.join(".config/glab")
        );
        assert_eq!(expanded("/etc/glab", Some(home)), Path::new("/etc/glab"));
        assert_eq!(expanded("~/.config", None), Path::new("~/.config"));
    }

    #[tokio::test]
    async fn a_component_that_traps_while_starting_is_trouble() {
        let trouble = instantiate(&assembled(TRAPS), &[], None)
            .await
            .err()
            .expect("a trap at instantiation should not load");

        assert_eq!(
            trouble.why,
            "The component stopped while it was starting: \
             wasm trap: wasm `unreachable` instruction executed."
        );
    }

    #[tokio::test]
    async fn a_component_that_never_finishes_starting_is_stopped_and_is_trouble() {
        let trouble = started_within(&assembled(SPINS), &[], None, Duration::from_secs(2))
            .await
            .err()
            .expect("a start that never finishes should not load");

        assert_eq!(
            trouble.why,
            "The component didn't finish starting within 2 seconds, so it was stopped."
        );
    }

    #[tokio::test]
    async fn a_component_that_started_has_no_deadline_left() {
        let root = two_directories();
        let capabilities = granting(&root.path().join("granted"));
        let mut loaded = started_within(&reader(), &capabilities, None, Duration::from_millis(200))
            .await
            .expect("the reader should load");

        std::thread::sleep(Duration::from_millis(500));

        assert_eq!(read(&mut loaded, "secret").await, "granted".len() as i64);
    }

    #[tokio::test]
    async fn a_component_importing_what_isnt_provided_is_trouble() {
        let bytes = assembled(r#"(component (import "nowhere" (func)))"#);

        let trouble = instantiate(&bytes, &[], None)
            .await
            .err()
            .expect("an unsatisfied import should not load");

        assert!(
            trouble
                .why
                .starts_with("The component needs something Verkstead doesn't provide"),
            "{trouble}"
        );
    }

    #[tokio::test]
    async fn bytes_that_are_not_a_component_are_trouble() {
        let trouble = instantiate(b"not wasm", &[], None)
            .await
            .err()
            .expect("garbage should not load");

        assert!(trouble.why.starts_with("The component isn't"), "{trouble}");
    }

    #[tokio::test]
    async fn a_request_to_a_host_not_granted_is_refused() {
        let mut outbound = Outbound {
            hosts: vec!["gitlab.com".to_owned()],
        };
        let request = http::Request::builder()
            .uri("https://example.com/")
            .body(Default::default())
            .unwrap();

        let sent =
            Box::into_pin(outbound.send_request(request, None, Box::new(async { Ok(()) }))).await;

        assert!(
            matches!(sent, Err(wasmtime_wasi_http::Error::HttpRequestDenied)),
            "{:?}",
            sent.err()
        );
    }

    #[test]
    fn only_a_named_host_is_permitted() {
        let outbound = Outbound {
            hosts: vec!["gitlab.com".to_owned()],
        };

        assert!(outbound.permits(Some("gitlab.com")));
        assert!(outbound.permits(Some("GitLab.com")));
        assert!(!outbound.permits(Some("gitlab.com.evil.example")));
        assert!(!outbound.permits(Some("api.gitlab.com")));
        assert!(!outbound.permits(None));
    }
}
