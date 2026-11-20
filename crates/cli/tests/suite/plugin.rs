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

/// The least that is an Activity, made into a component against the server's
/// `activity` world — see the WAT.
fn activity_component() -> Vec<u8> {
    let server = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server");
    let mut resolve = wit_parser::Resolve::default();
    let (package, _) = resolve
        .push_dir(server.join("wit"))
        .expect("the plugin WIT should parse");
    let world = resolve
        .select_world(&[package], Some("activity"))
        .expect("the activity world should be there");

    let mut module = wat::parse_str(
        std::fs::read_to_string(server.join("tests/fixtures/activity.wat")).unwrap(),
    )
    .expect("the activity should assemble");
    wit_component::embed_component_metadata(
        &mut module,
        &resolve,
        world,
        wit_component::StringEncoding::UTF8,
        false,
    )
    .expect("the activity world should embed");

    wit_component::ComponentEncoder::default()
        .module(&module)
        .expect("the activity should be a module wit-component reads")
        .validate(true)
        .encode()
        .expect("the activity should encode as a component")
}

/// A plugin providing one Activity, *Review rounds*, whose component is
/// `component`.
fn with_activity(component: &[u8]) -> tempfile::TempDir {
    let dir = plugin(&MINIMAL.replace(
        "provides: []",
        "provides:\n  - category: activity\n    id: review-rounds\n    name: Review rounds\n    \
         component: rounds.wasm\n    outcomes: [approved, abandoned]\n",
    ));
    std::fs::write(dir.path().join("rounds.wasm"), component).unwrap();
    dir
}

#[test]
fn a_plugin_providing_an_activity_passes_naming_it() {
    let dir = with_activity(&activity_component());

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("providing activity — its activities Review rounds."),
        "{stdout}"
    );
}

#[test]
fn an_activity_whose_component_lacks_the_worlds_exports_is_refused_saying_so() {
    let dir = with_activity(&wat::parse_str("(component)").unwrap());

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "The component for Review rounds isn't written against Verkstead's activity world"
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

/// The built-in Language plugins are directories an author could have written,
/// and pass the check as they stand in the repository.
#[test]
fn a_built_in_language_plugin_passes_naming_its_languages() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/rust-cpp");

    let output = check(&dir);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("providing language rust, language cpp"),
        "{stdout}"
    );
}

#[test]
fn a_language_entry_setting_a_sandbox_variable_is_refused_naming_it() {
    let dir = plugin(&MINIMAL.replace(
        "provides: []",
        "provides:\n  - category: language\n    language: rust\n    env:\n      PATH: /bin\n",
    ));

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`provides[0].env.PATH`"), "{stderr}");
}

/// A harness declared in full, as `docs/plugins.md` writes one, its component
/// `agent.wasm` and its mark `mark.svg`.
pub(crate) const AGENT: &str = r#"
uuid: 3f0c9a12-7b4e-4c1d-8e2f-5a6b7c8d9e0f
name: Stub harness
author:
  name: Ada Lovelace
  email: ada@example.com
version: 0.1.0
interface: 1
mark: mark.svg
component: agent.wasm
provides:
  - category: agent
    binary: stub
    name: Stub
    launch:
      model: [--model, "{model}"]
      prompt: ["{prompt}"]
      session: [--session-id, "{session}"]
      resume:
        flag: [--resume, "{session}"]
      tail: [--yes]
    account:
      - key: dir
        label: Directory
        kind: directory
        mounted: .stub
      - key: file
        label: Settings file
        kind: file
        mounted: .stub.json
    idle: silence
    ask:
      channel: blocking
      guide: Run `verkstead ask` and wait for it.
    models:
      - id: stub-large
        name: Stub Large
        default: true
    tools: mcp__<server>__<tool>
    instructions: STUB.md
"#;

/// A component exporting the server's `agent` world, made from its WIT by
/// wit-component, each export a trap nothing here calls.
pub(crate) fn exporting_an_agent() -> Vec<u8> {
    let mut resolve = wit_parser::Resolve::default();
    let (package, _) = resolve
        .push_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/wit"))
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

/// A plugin of `manifest` providing an agent, with `component` as its
/// component.
pub(crate) fn agent(manifest: &str, component: &[u8]) -> tempfile::TempDir {
    let dir = plugin(manifest);
    std::fs::write(
        dir.path().join("mark.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    std::fs::write(dir.path().join("agent.wasm"), component).unwrap();
    dir
}

#[test]
fn a_complete_agent_passes() {
    let dir = agent(AGENT, &exporting_an_agent());

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("providing agent"), "{stdout}");
}

#[test]
fn an_agent_missing_a_piece_is_refused_naming_it() {
    let dir = agent(
        &AGENT.replace("        label: Settings file\n", ""),
        &exporting_an_agent(),
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`provides[0].account[1].label` is missing."),
        "{stderr}"
    );
    assert!(!stderr.contains("(read for"), "{stderr}");
}

#[test]
fn an_agent_refused_on_one_platform_says_which() {
    let dir = agent(
        &format!("{AGENT}    windows:\n      idle: never\n"),
        &exporting_an_agent(),
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`provides[0].idle`"), "{stderr}");
    assert!(stderr.contains("(read for windows)"), "{stderr}");
}

#[test]
fn an_agent_whose_component_exports_nothing_is_refused() {
    let dir = agent(
        AGENT,
        &wat::parse_str("(component)").expect("the fixture should assemble"),
    );

    let output = check(dir.path());

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("The component provides an agent and doesn't export what one needs"),
        "{stderr}"
    );
}

/// The Activities Verkstead ships pass as an author's plugin would: the
/// checkout's `plugin.yaml`, beside each block's component as this binary
/// embeds it — the one file a built-in's directory holds the source of rather
/// than the bytes.
#[test]
fn the_activities_verkstead_ships_pass() {
    let shipped = verkstead_server::plugins::built_in::activities();
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/activities");

    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(checkout.join("plugin.yaml"), dir.path().join("plugin.yaml")).unwrap();
    for (name, bytes) in shipped.files {
        if name.ends_with(".wasm") {
            std::fs::write(dir.path().join(name), bytes).unwrap();
        }
    }

    let output = check(dir.path());

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "providing activity — its activities Follow-up rounds, Backlog runner, and Stage \
             runner."
        ),
        "{stdout}"
    );
}
