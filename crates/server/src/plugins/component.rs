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
//! importing what this host does not provide, one that traps while it starts.
//! The same sentence is what `verkstead plugin check` prints and what the
//! Plugins page draws on the row.
//!
//! **wasmtime's errors are its own type**, not `std::error::Error`, so they
//! are worded into a [`Trouble`] here, at the loader's edge, and go no further.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use wasmtime::component::{Component, Instance, Linker, ResourceTable};
use wasmtime::{Engine, Store, Trap};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::{
    WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpView, default_hooks,
};

use super::manifest::{Access, Capability, Files, Manifest};

/// The bindings to `wit/plugin.wit`: the base world every component is checked
/// against, and one world per Category with nothing in it yet.
pub mod worlds {
    wasmtime::component::bindgen!({ path: "wit", world: "plugin" });

    /// The Category worlds, which the stages that give each Category its
    /// grammar fill in.
    pub mod category {
        wasmtime::component::bindgen!({ path: "wit", world: "agent" });
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

/// The one engine every component is compiled by.
static ENGINE: LazyLock<Engine> = LazyLock::new(Engine::default);

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

    instantiate(&bytes, &manifest.capabilities, home)
        .await
        .map(Some)
}

/// Instantiate a component's bytes under `capabilities`.
pub async fn instantiate(
    bytes: &[u8],
    capabilities: &[Capability],
    home: Option<&Path>,
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

    let mut store = Store::new(&ENGINE, granted(capabilities, home)?);
    let instance = pre.instantiate_async(&mut store).await.map_err(|error| {
        // A trap is worded as the trap alone: what wasmtime wraps around it
        // is a backtrace, which is the component author's to read with their
        // own tools rather than a row's to draw.
        if let Some(trap) = error.downcast_ref::<Trap>() {
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

    Ok(Loaded { store, instance })
}

/// The store a component runs in, holding exactly what `capabilities` grant.
fn granted(capabilities: &[Capability], home: Option<&Path>) -> Result<Host, Trouble> {
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

    Ok(Host {
        wasi: wasi.build(),
        http: WasiHttpCtx::new(),
        outbound,
        table: ResourceTable::new(),
    })
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
fn worded(error: &wasmtime::Error) -> String {
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
