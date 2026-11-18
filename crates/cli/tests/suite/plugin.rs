//! `verkstead plugin check`, run as an author runs it: against a directory, with
//! no server anywhere, saying either what the plugin is or the refusal that
//! keeps it from loading — and exiting the way a script can test.

use std::path::Path;
use std::process::Output;

/// The least a plugin can be.
const MINIMAL: &str = "\
uuid: 0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90
name: Nothing at all
author:
  name: Ada Lovelace
  email: ada@example.com
version: 0.1.0
interface: 1
provides: []
";

fn check(dir: &Path) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_verkstead"))
        .args(["plugin", "check"])
        .arg(dir)
        .output()
        .expect("the verkstead binary should be built for its own tests")
}

fn plugin(manifest: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plugin.yaml"), manifest).unwrap();
    dir
}

#[test]
fn a_minimal_manifest_passes() {
    let dir = plugin(MINIMAL);

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("is a plugin Verkstead would load"),
        "{stdout}"
    );
    assert!(stdout.contains("Nothing at all 0.1.0"), "{stdout}");
    assert!(stdout.contains("providing nothing"), "{stdout}");
}

#[test]
fn a_refusal_is_printed_naming_the_field_and_exits_non_zero() {
    let dir = plugin(&MINIMAL.replace("version: 0.1.0", "version: one"));

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("isn't a plugin Verkstead would load"),
        "{stderr}"
    );
    assert!(
        stderr.contains("`version` isn't a semantic version"),
        "{stderr}"
    );
}

#[test]
fn a_refusal_on_one_platform_says_which() {
    let dir = plugin(&format!(
        "{MINIMAL}windows:\n  capabilities:\n    - registry: HKCU\n"
    ));

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`capabilities[0]`"), "{stderr}");
    assert!(stderr.contains("(read for windows)"), "{stderr}");
}

#[test]
fn a_directory_with_no_manifest_is_not_a_plugin() {
    let dir = tempfile::tempdir().unwrap();

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("There's no `plugin.yaml` here"), "{stderr}");
}

/// A plugin whose component is `wat`, assembled.
fn with_component(wat: &str) -> tempfile::TempDir {
    let dir = plugin(&format!("{MINIMAL}component: plugin.wasm\n"));
    let bytes = wat::parse_str(wat).expect("the fixture should assemble");
    std::fs::write(dir.path().join("plugin.wasm"), bytes).unwrap();
    dir
}

#[test]
fn a_component_exporting_nothing_passes() {
    let dir = with_component("(component)");

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
}

#[test]
fn a_component_that_traps_while_starting_is_refused_saying_so() {
    let dir = with_component(
        r#"
(component
  (core module $m
    (func $start unreachable)
    (start $start))
  (core instance (instantiate $m)))
"#,
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("isn't a plugin Verkstead would load"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "The component stopped while it was starting: wasm trap: wasm `unreachable` instruction executed."
        ),
        "{stderr}"
    );
}

/// A plugin whose one `provides:` entry is the Skill Pack `pack`, its skills
/// each a directory of `skills/` holding the `SKILL.md` given.
fn with_pack(pack: &str, skills: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = plugin(&MINIMAL.replace(
        "provides: []",
        &format!("provides:\n  - category: skills\n    name: {pack}\n    directory: skills\n"),
    ));
    for (skill, text) in skills {
        let at = dir.path().join("skills").join(skill);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("SKILL.md"), text).unwrap();
    }
    dir
}

#[test]
fn a_skill_pack_passes() {
    let dir = with_pack(
        "rust-review",
        &[(
            "reviewing",
            "---\nname: reviewing\ndescription: Review Rust.\n---\n\nRead the diff.\n",
        )],
    );

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("providing skills"), "{stdout}");
}

#[test]
fn a_skill_without_frontmatter_is_refused_naming_it() {
    let dir = with_pack(
        "rust-review",
        &[("reviewing", "# Reviewing\n\nRead the diff.\n")],
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("The skill `reviewing` has no frontmatter"),
        "{stderr}"
    );
}

#[test]
fn a_skill_named_other_than_its_directory_is_refused_naming_it() {
    let dir = with_pack(
        "rust-review",
        &[(
            "reviewing",
            "---\nname: review\ndescription: Review Rust.\n---\n",
        )],
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("The skill `reviewing` is named `review`"),
        "{stderr}"
    );
}

#[test]
fn a_pack_named_verkstead_is_refused() {
    let dir = with_pack(
        "verkstead",
        &[(
            "grilling",
            "---\nname: grilling\ndescription: Grill.\n---\n",
        )],
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`provides[0].name` is `verkstead`"),
        "{stderr}"
    );
}
