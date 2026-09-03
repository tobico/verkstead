//! `verkstead-desktop.exe` as a Start-menu shortcut starts it: the shim itself,
//! with a stand-in `verkstead.exe` beside it and another one on the `PATH`.
//!
//! What is judged here is the whole of what the shim is — that it opens no
//! console, that the `verkstead` it starts is the file beside its own image
//! rather than whatever the `PATH` finds first, that the arguments it was given
//! reach the app behind the verb, and that what it exits with is what the app
//! exited with. The app itself is `crates/cli`'s own desktop suite, which drives
//! `verkstead desktop` end to end; nothing of it is repeated here.
//!
//! **Windows's alone**, which is what `required-features = ["shim"]` says in
//! `Cargo.toml`: there is no shim to read or run anywhere else, because nothing
//! built one.
//!
//! **Its own `main` rather than the test harness's**, which is the one unusual
//! thing about this file. A shim that finds `verkstead.exe` beside itself needs
//! a `verkstead.exe` to find, and compiling one for the occasion would be a
//! second toolchain in a test — so what stands there is a copy of *this* binary,
//! told to answer as the app by a marker file beside it. That decision has to be
//! made before anything else runs, which is what having the `main` buys; see
//! [`stand_in`].

use std::path::{Path, PathBuf};
use std::process::Command;

/// What says a copy of this binary is standing in for `verkstead.exe`: a file
/// beside its own image, holding the exit code it is to end with.
///
/// Beside the image rather than in the environment, because the environment is
/// inherited: the two stand-ins a run of this file puts on a machine are the
/// same bytes in two directories, and what tells them apart has to be something
/// each of them can read about *itself*.
const MARKER: &str = "stand-in";

/// Where a stand-in writes down what it was started with, one argument a line,
/// beside its own image for [`MARKER`]'s reason.
const RECORDED: &str = "started-with";

fn main() {
    // Before anything else, because a stand-in is not a test run: it is this
    // binary being the app the shim went looking for.
    stand_in();

    the_shim_is_a_windows_subsystem_exe();
    the_shim_starts_the_verkstead_beside_it_and_not_the_one_on_the_path();
    the_exe_carries_the_icon_it_was_built_with();

    println!("the shim holds");
}

/// Answer as `verkstead.exe` and stop, where this copy of the binary is one.
///
/// What it does is the whole of what the shim's own tests need of the app: it
/// writes down the command line it was handed and exits with the code it was
/// asked for. A copy that has no marker beside it is the test binary itself,
/// and this returns.
fn stand_in() {
    let here = std::env::current_exe().expect("a running binary knows where it is");
    let beside = here.parent().expect("and what directory it is in");

    let Ok(code) = std::fs::read_to_string(beside.join(MARKER)) else {
        return;
    };

    let started_with: Vec<String> = std::env::args().skip(1).collect();
    std::fs::write(beside.join(RECORDED), started_with.join("\n"))
        .expect("a stand-in should be able to write beside itself");

    std::process::exit(code.trim().parse().expect("the marker holds an exit code"));
}

/// No console window, which is the first thing a human notices about a shortcut
/// that starts a tray app — and the reason there is a shim at all rather than
/// the CLI being started directly.
///
/// Read out of the built exe rather than trusted to the attribute in the source:
/// what makes an exe a windows-subsystem one is a field in its own header, and
/// an attribute that stopped applying — moved into a module, lost to a
/// refactor — would be a green build and a black window in front of the human.
fn the_shim_is_a_windows_subsystem_exe() {
    /// What the field says for an exe Windows gives no console to.
    const WINDOWS_GUI: u16 = 2;

    let exe = std::fs::read(env!("CARGO_BIN_EXE_verkstead-desktop")).expect("the exe is built");

    // The PE header starts where the DOS stub says it does; the optional header
    // follows its four-byte signature and the twenty-byte COFF header, and the
    // subsystem is sixty-eight bytes into that — the same offset whether the
    // optional header is the 32-bit shape or the 64-bit one.
    let pe = u32::from_le_bytes(exe[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&exe[pe..pe + 4], b"PE\0\0", "the exe should be a PE image");

    let subsystem = u16::from_le_bytes(exe[pe + 24 + 68..pe + 24 + 70].try_into().unwrap());
    assert_eq!(
        subsystem, WINDOWS_GUI,
        "the shim should be a windows-subsystem exe, so that a shortcut opens no console"
    );
}

/// The whole of what the shim does: it starts the `verkstead` that was installed
/// beside it, through the verb, with everything it was given itself, and it ends
/// when that ends and with the same code.
///
/// **The one on the `PATH` is the point.** A machine with Verkstead installed
/// has `verkstead` on the `PATH` — the msi puts it there so an agent can ask —
/// and a shim that resolved the name rather than the path would start whichever
/// copy was found first. The `PATH` this gives it holds a stand-in of its own,
/// and what says the shim ignored it is that stand-in having written nothing.
fn the_shim_starts_the_verkstead_beside_it_and_not_the_one_on_the_path() {
    /// What the stand-in beside the shim exits with: nothing a runtime picks, so
    /// a shim that made up an exit code of its own could not have hit it.
    const ITS_OWN_CODE: i32 = 42;

    let tmp = tempfile::tempdir().unwrap();
    let installed = tmp.path().join("installed");
    let elsewhere = tmp.path().join("elsewhere");

    let shim = copy_shim_into(&installed);
    stand_in_in(&installed, ITS_OWN_CODE);
    stand_in_in(&elsewhere, 7);

    let ended = Command::new(&shim)
        .args(["--no-open", "--data-dir", "D:\\somewhere"])
        .env("PATH", &elsewhere)
        .status()
        .expect("the shim should start");

    assert_eq!(
        ended.code(),
        Some(ITS_OWN_CODE),
        "the shim should exit with what the app exited with"
    );
    assert_eq!(
        started_with(&installed),
        Some("desktop\n--no-open\n--data-dir\nD:\\somewhere".to_owned()),
        "the app should have been given the verb and then everything the shim was given"
    );
    assert_eq!(
        started_with(&elsewhere),
        None,
        "the verkstead on the PATH should not have been started"
    );
}

/// The shim copied into `dir`, which is what an installation is: the two files
/// in one directory.
fn copy_shim_into(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();

    let shim = dir.join("verkstead-desktop.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_verkstead-desktop"), &shim).unwrap();

    shim
}

/// A `verkstead.exe` in `dir` that records what it was started with and exits
/// with `code` — see [`stand_in`], which is what the copy does when it runs.
fn stand_in_in(dir: &Path, code: i32) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), dir.join("verkstead.exe")).unwrap();
    std::fs::write(dir.join(MARKER), code.to_string()).unwrap();
}

/// What the stand-in in `dir` was started with, or `None` where it was never
/// started at all.
fn started_with(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join(RECORDED)).ok()
}

/// The shim carries Verkstead's own icon, which is what Explorer, the Start
/// menu, the taskbar and Alt-Tab draw the app with — the shim being the file
/// every one of those names.
///
/// Read out of the built exe rather than out of the build's own log: the icon is
/// compiled in by a resource compiler that `crates/desktop/build.rs` runs, and a
/// run of it that found nothing to do would otherwise be a green build and an
/// exe with a default icon. The resource compiler copies each image out of the
/// `.ico` as it stands, so what says the icon arrived is the image's own bytes
/// being in there.
///
/// Two of the seven sizes rather than all of them: what can go wrong here is the
/// whole icon being absent or the file being a different one, and the smallest
/// and the largest between them say both. Searching a debug binary is not free.
fn the_exe_carries_the_icon_it_was_built_with() {
    let ico = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/net.tobico.Verkstead.ico"),
    )
    .expect("packaging/ should hold the committed icon");
    let exe = std::fs::read(env!("CARGO_BIN_EXE_verkstead-desktop")).expect("the exe is built");

    // An icon file is a directory and then the images: two bytes saying how
    // many, and sixteen per entry, of which the last eight are the image's
    // length and where in the file it starts.
    let images = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    assert!(images >= 2, "the icon should hold every size, got {images}");

    for image in [0, images - 1] {
        let entry = 6 + image * 16;
        let size = u32::from_le_bytes(ico[entry + 8..entry + 12].try_into().unwrap()) as usize;
        let at = u32::from_le_bytes(ico[entry + 12..entry + 16].try_into().unwrap()) as usize;
        let drawn = &ico[at..at + size];

        assert!(
            exe.windows(drawn.len()).any(|carried| carried == drawn),
            "the exe should carry image {image} of packaging/net.tobico.Verkstead.ico"
        );
    }
}
