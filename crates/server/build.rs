//! The built-in plugins, embedded: every directory under the repository's
//! `plugins/` with a `plugin.yaml` in it is one, and its manifest and its wasm
//! component are compiled into the server — see `src/plugins/built_in.rs`.
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
//! A built-in's component is the one with its directory's name: the crate in
//! `plugins/<name>/` names its `[lib]` `<name>` (a `-` written `_`, as cargo
//! would), which is the file cargo writes, and the manifest's `component:` is
//! `<name>.wasm`.

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

    let mut built: Option<PathBuf> = None;
    let mut list = String::from("&[\n");
    for name in &names {
        let dir = plugins.join(name);
        let file = format!("{name}.wasm");
        let lib = format!("{}.wasm", name.replace('-', "_"));

        let mut files = String::new();
        if dir.join("Cargo.toml").is_file() {
            let wasm = match plugins.join("dist").join(&lib) {
                artefact if artefact.is_file() => artefact,
                _ => built
                    .get_or_insert_with(|| build(&plugins, &out))
                    .join(&lib),
            };
            write!(files, "({file:?}, include_bytes!({:?})),", path(&wasm)).unwrap();
        }

        writeln!(
            list,
            "    BuiltIn {{ directory: {name:?}, manifest: include_str!({:?}), files: &[{files}] }},",
            path(&dir.join("plugin.yaml")),
        )
        .unwrap();
    }
    list.push(']');

    fs::write(out.join("built_ins.rs"), list).expect("OUT_DIR is writable");
}

/// Build every component under `plugins/` for `wasm32-wasip2`, returning the
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
