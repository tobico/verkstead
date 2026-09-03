//! What the Windows shim carries besides its code: the icon Explorer, the Start
//! menu, the taskbar and Alt-Tab draw it with, and the version information its
//! properties sheet shows.
//!
//! **Windows alone**, and the shim alone: it is the file every one of those
//! names — see `src/main.rs` — and it is the only target this crate builds
//! there. Neither the icon nor the version information is a thing an executable
//! can hold on the other two platforms: the AppImage names an icon a desktop
//! installs beside it and the app bundle keeps one in `Contents/Resources`,
//! both of which are the packaging's business rather than the binary's.
//!
//! The `.ico` is generated and committed by `tools/generate-packaging.sh`, out
//! of the same artwork and the same downscales as everything else under
//! `packaging/`. Nothing is generated here.

fn main() {
    // Said whatever the target is, so that a fresh icon is a fresh build rather
    // than a build that kept the last one.
    println!("cargo:rerun-if-changed=../../packaging/net.tobico.Verkstead.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();

    resource.set_icon("../../packaging/net.tobico.Verkstead.ico");

    // What Windows shows where it names the file rather than draws it: the
    // Details tab of its properties, and the Task Manager column that a human
    // looking for what to stop reads. The product is Verkstead in both, said
    // the way `APP_ID` is said once — see `crates/desktop/src/lib.rs`.
    resource.set("ProductName", "Verkstead");
    resource.set("FileDescription", "Verkstead");

    // **Loudly rather than quietly.** A resource that was not compiled is an
    // exe with a default icon and nothing in its properties, which is exactly
    // what this stage was for and exactly the kind of thing nobody notices in
    // a build log. `crates/desktop/tests/shim.rs` reads the built exe for the
    // artwork as well, so a resource compiler that ran and wrote nothing is
    // caught too.
    resource
        .compile()
        .expect("the icon and version information should compile into the exe");
}
