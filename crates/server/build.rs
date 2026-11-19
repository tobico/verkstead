//! The built-in plugins, embedded: every directory under the repository's
//! `plugins/` with a `plugin.yaml` in it is one, and its manifest, its wasm
//! component and every other file in its directory are compiled into the
//! server — see `src/plugins/built_in.rs`. Every other file but a component
//! crate's own sources: a `Cargo.toml` and its `src/` are what the component is
//! built from, and nothing a plugin reads.
//!
//! **A debug build is also told where the directory is**, and reads it there
//! rather than out of the binary, the way the viewer is read — so a Skill Pack's
//! skills edited in the checkout are what a running debug server installs. Not
//! for a built-in with a component, which is built rather than written, and
//! never for a release, which names no path on the machine that built it.
//!
//! **A wasm component is built once and embedded everywhere.** It is
//! platform-neutral bytes, so the release and CI workflows build every one for
//! `wasm32-wasip2` in a job of their own and put the files in `plugins/dist/`
//! before any leg compiles this crate; every leg then embeds the same bytes.
//! Where `plugins/dist/<name>.wasm` is not there — a checkout that has not
//! built them — this script builds the `plugins/` workspace itself, into its
//! own `OUT_DIR`, and a toolchain without `wasm32-wasip2` is told the command
//! that adds it. `docs/development.md`'s *Built-in plugins* says the same.
//!
//! A built-in's components are named by the directories of their crates: the
//! crate in `plugins/<name>/` names its `[lib]` `<name>` (a `-` written `_`, as
//! cargo would), which is the file cargo writes, and the manifest's
//! `component:` is `<name>.wasm`. And **each `activity` block names a component
//! of its own** (Set 1803, Q3), so a crate in a directory just inside it,
//! `plugins/<name>/<block>/`, is one more: its `[lib]` is `<block>`, it is
//! embedded at the top of the built-in's files as `<block>.wasm`, and that is
//! what the block's `component:` names. None of that crate's files are the
//! built-in's own.
//!
//! **The workspace also holds components that are no built-in's**, under
//! `plugins/tests/`, which the server's suites install as plugins of their
//! own: they are built with the rest and embedded nowhere, and each one's
//! path is handed to the tests in a variable — see `TESTS` below.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const TARGET: &str = "wasm32-wasip2";

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo says"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo says"));
    let plugins = crate_dir.join("../../plugins");

    // The whole tree, `dist/` included: a new built-in, a changed manifest or
    // component source, and a workflow's artefact landing are each a reason to
    // embed again.
    println!("cargo::rerun-if-changed={}", plugins.display());

    let mut names: Vec<String> = fs::read_dir(&plugins)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().join("plugin.yaml").is_file())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    names.sort();

    // What `rust-embed` goes by to read the viewer off disk, so the two agree.
    let debug = env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();

    let mut built: Option<PathBuf> = None;
    let mut list = String::from("&[\n");
    for name in &names {
        let dir = plugins.join(name);
        let file = format!("{name}.wasm");
        let lib = format!("{}.wasm", name.replace('-', "_"));

        let crate_ = dir.join("Cargo.toml").is_file();
        let mut crates: Vec<String> = fs::read_dir(&dir)
            .expect("a built-in's directory reads")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("Cargo.toml").is_file())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        crates.sort();

        let mut files = String::new();
        let mut inside = Vec::new();
        walked(&dir, Path::new(""), crate_, &crates, &mut inside);
        inside.sort();
        for file in &inside {
            let name = file
                .iter()
                .map(|component| component.to_str().expect("a built-in's paths are UTF-8"))
                .collect::<Vec<_>>()
                .join("/");
            write!(
                files,
                "({name:?}, include_bytes!({:?})),",
                path(&dir.join(file))
            )
            .unwrap();
        }

        let components = crate_
            .then(|| (file.clone(), lib.clone()))
            .into_iter()
            .chain(crates.iter().map(|block| {
                (
                    format!("{block}.wasm"),
                    format!("{}.wasm", block.replace('-', "_")),
                )
            }));
        let mut any = false;
        for (file, lib) in components {
            any = true;
            let wasm = match plugins.join("dist").join(&lib) {
                artefact if artefact.is_file() => artefact,
                _ => built
                    .get_or_insert_with(|| build(&plugins, &out))
                    .join(&lib),
            };
            write!(files, "({file:?}, include_bytes!({:?})),", path(&wasm)).unwrap();
        }

        let checkout = if debug && !any {
            format!("Some({:?})", path(&dir))
        } else {
            "None".to_owned()
        };

        writeln!(
            list,
            "    BuiltIn {{ directory: {name:?}, manifest: include_str!({:?}), files: &[{files}], \
             checkout: {checkout} }},",
            path(&dir.join("plugin.yaml")),
        )
        .unwrap();
    }
    list.push(']');

    fs::write(out.join("built_ins.rs"), list).expect("OUT_DIR is writable");

    // And the components the suites install as plugins of their own, each a
    // workspace of its own and embedded nowhere: each test is handed the
    // file's path instead. **None where its source is not there**, which is the
    // packaged build's — see `nix/verkstead-source.nix` — and which compiles no
    // test that would ask.
    for (variable, directory, lib) in TESTS {
        let source = plugins.join("tests").join(directory);
        let wasm = match plugins.join("dist").join(lib) {
            artefact if artefact.is_file() => artefact,
            _ if !source.join("Cargo.toml").is_file() => continue,
            _ => build(&source, &out.join(directory)).join(lib),
        };
        println!("cargo::rustc-env={variable}={}", path(&wasm));
    }
}

/// Every file under `dir` but its manifest, relative to the built-in's
/// directory (`at` being `dir`'s place in it) — and, where the built-in is a
/// component `crate_`, but its `Cargo.toml` and `src/`, and but the whole of
/// each of its `crates`, an `activity` block's component's.
fn walked(dir: &Path, at: &Path, crate_: bool, crates: &[String], found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a built-in's directory reads") {
        let entry = entry.expect("a built-in's directory reads");
        let inside = at.join(entry.file_name());
        let top = at.as_os_str().is_empty();
        let name = entry.file_name();
        if top
            && (name == "plugin.yaml"
                || (crate_ && (name == "Cargo.toml" || name == "src"))
                || crates.iter().any(|block| name == block.as_str()))
        {
            continue;
        }

        let kind = entry.file_type().expect("a built-in's directory reads");
        if kind.is_dir() {
            walked(&entry.path(), &inside, crate_, &[], found);
        } else if kind.is_file() {
            found.push(inside);
        }
    }
}

/// The components under `plugins/tests/` the suites install, each by the
/// variable a test reads its path from, its directory there, and the file
/// cargo writes.
const TESTS: &[(&str, &str, &str)] = &[(
    "VERKSTEAD_STUB_HARNESS_WASM",
    "stub-harness",
    "stub_harness.wasm",
)];

/// Build every component of the workspace at `plugins` — the built-ins' under
/// `plugins/`, or a test component's own — for `wasm32-wasip2`, returning the
/// directory the `.wasm` files are in.
fn build(plugins: &Path, out: &Path) -> PathBuf {
    let target_dir = out.join("plugins-target");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    // What cargo hands this script about *its* build is no part of this one:
    // the flags are the host target's, and a wrapper such as clippy's is for
    // the workspace above.
    let output = Command::new(cargo)
        .args(["build", "--release", "--locked", "--target", TARGET])
        .arg("--manifest-path")
        .arg(plugins.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("cargo runs");

    if !output.status.success() {
        panic!(
            "\n\nThe built-in plugins' wasm components couldn't be built for {TARGET}, and \
             there are none in plugins/dist/ to embed instead.\n\
             Where the toolchain lacks the target, `rustup target add {TARGET}` adds it \
             (the dev shell's has it).\n\
             Or build them once and leave them where this looks first:\n\n    \
             cargo build --release --target {TARGET} --manifest-path plugins/Cargo.toml\n    \
             cargo build --release --target {TARGET} --target-dir plugins/target \
             --manifest-path plugins/tests/stub-harness/Cargo.toml\n    \
             mkdir -p plugins/dist && cp plugins/target/{TARGET}/release/*.wasm plugins/dist/\n\n\
             What cargo said:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    target_dir.join(TARGET).join("release")
}

/// A path as `include_bytes!` takes it.
fn path(path: &Path) -> String {
    path.to_str()
        .expect("the checkout's path is UTF-8")
        .to_owned()
}
