//! Every Built Root, file for file, held against a record of what each harness's
//! was before its account half moved behind [`crate::harness::Harness`].
//!
//! **One account of each harness in one home**, each holding a login and a
//! configuration with keys a root carries and keys it leaves out, and every
//! answer a root and the account's paths give rendered into one text: what is
//! built, furnished, written, instructed, joined, made in the account, left
//! unshared, synced and mirrored, with the memory switch on and off and with and
//! without an attached MCP server. The text is compared with
//! `built_roots.golden`, which was written by the code before the move — so a
//! change to any harness's root, however small, is a failure here naming it.
//!
//! Run with `VERKSTEAD_BLESS_ROOTS=1` to write the record afresh, which is a
//! change of behaviour and is reviewed as one.

use std::fmt::Write as _;
use std::path::Path;

use super::root::{self, Root};
use crate::platform::Platform;
use crate::settings::AttachedServer;
use crate::store;

/// The record this is held against.
const GOLDEN: &str = include_str!("built_roots.golden");

/// Where the record is, to bless it.
const GOLDEN_AT: &str = "src/sandbox/built_roots.golden";

/// The five, in the order they are rendered.
const TYPES: [store::AgentType; 5] = [
    store::AgentType::CLAUDE,
    store::AgentType::CODEX,
    store::AgentType::GROK,
    store::AgentType::OPENCODE,
    store::AgentType::AGY,
];

/// What every account in the home holds, said from the home.
const FIXTURES: &[(&str, &str)] = &[
    (
        ".claude/.credentials.json",
        "{\"claudeAiOauth\":{\"accessToken\":\"a\"}}",
    ),
    (
        ".claude/settings.json",
        "{\"apiKeyHelper\":\"print-key\",\"env\":{\"A\":\"1\"},\"hooks\":{\"Stop\":[]},\"statusLine\":{}}",
    ),
    (".claude/CLAUDE.md", "the human's own"),
    (
        ".claude.json",
        "{\"mcpServers\":{\"own\":{}},\"projects\":{\"/elsewhere\":{\"hasTrustDialogAccepted\":true}},\
         \"oauthAccount\":{\"emailAddress\":\"h@example.com\",\"displayName\":\"H\",\"accountUuid\":\"u\"},\
         \"numStartups\":3}",
    ),
    (".codex/auth.json", "{\"OPENAI_API_KEY\":\"k\"}"),
    (
        ".codex/config.toml",
        "model_provider = \"mine\"\nnotify = [\"x\"]\n\n[model_providers.mine]\nbase_url = \"http://m\"\n\n[mcp_servers.own]\nurl = \"http://own\"\n",
    ),
    (".grok/auth.json", "{\"grok\":{\"token\":\"t\"}}"),
    (
        ".grok/config.toml",
        "[model.custom]\nbase_url = \"http://g\"\n\n[endpoints]\nmodels_base_url = \"http://b\"\ntrace_url = \"http://t\"\n\n[mcp_servers.own]\nurl = \"http://own\"\n\n[ui]\ntheme = \"dark\"\n",
    ),
    (
        ".config/opencode/opencode.json",
        "{\"provider\":{\"p\":{\"name\":\"first\"}},\"mcp\":{\"own\":{}},\"theme\":\"x\"}",
    ),
    (
        ".config/opencode/opencode.jsonc",
        "{\n  // a comment\n  \"provider\": {\"q\": {\"name\": \"second\"}}\n}",
    ),
    (".local/share/opencode/auth.json", "{\"opencode\":{}}"),
    (
        ".gemini/antigravity-cli/settings.json",
        "{\"modelProvider\":\"m\",\"gcp\":{\"project\":\"p\"},\"theme\":\"dark\"}",
    ),
    (
        ".gemini/antigravity-cli/antigravity-oauth-token",
        "{\"token\":\"t\"}",
    ),
];

/// A root of `account`'s harness, as a session launched in the Worktree at
/// `worktree` of the Repo whose common git directory is `git_dir` is given.
fn root_of(account: &store::Account, git_dir: &Path, worktree: &Path) -> Root {
    Root::of(account, Platform::Linux, git_dir, worktree).expect("a built-in harness")
}

/// `said` with the temporary directory it is under as `<tmp>`, and Claude's
/// name for an entry under it as `<tmp-entry>`.
fn unplaced(tmp: &Path, said: &str) -> String {
    said.replace(&tmp.display().to_string(), "<tmp>")
        .replace(&root::entry_named(tmp), "<tmp-entry>")
}

/// A path said that way.
fn said(tmp: &Path, path: &Path) -> String {
    unplaced(tmp, &path.display().to_string())
}

/// And bytes said as text, the same way.
fn text(tmp: &Path, bytes: &[u8]) -> String {
    unplaced(tmp, &String::from_utf8_lossy(bytes))
}

/// Every file and directory under `dir`, sorted, said from `dir`.
fn listed(dir: &Path) -> Vec<String> {
    let mut all = Vec::new();
    let mut stack = vec![dir.to_owned()];

    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(dir).unwrap().display().to_string();

            if path.is_dir() {
                all.push(format!("{relative}/"));
                stack.push(path);
            } else {
                all.push(relative);
            }
        }
    }

    all.sort();
    all
}

/// The whole rendering.
fn rendered() -> String {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path();
    let home = tmp.join("home");
    let built = tmp.join("built");
    let inside = tmp.join("inside");
    let git_dir = tmp.join("repo/.git");
    let worktree = tmp.join("worktrees/repo-branch");

    for (path, contents) in FIXTURES {
        let path = home.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    let servers = [AttachedServer::of(
        "tracker",
        "http://127.0.0.1:9/mcp",
        &[("Authorization", "Bearer s")],
    )];

    let mut out = String::new();
    let o = &mut out;

    for agent_type in TYPES {
        writeln!(o, "# {}", agent_type.word()).unwrap();

        let account = super::kept_in(agent_type, &home).expect("a built-in harness");

        writeln!(
            o,
            "in home: {}",
            super::account_in_home(agent_type, &home).is_some()
        )
        .unwrap();

        for (host, there) in super::account_inside(&account, &inside) {
            writeln!(o, "inside: {} -> {}", said(tmp, &host), said(tmp, &there)).unwrap();
        }

        let login_at = root::login_at(&account).expect("a built-in harness");
        let login_inside = root::login_inside(agent_type).expect("a built-in harness");

        writeln!(o, "login at: {}", said(tmp, &login_at)).unwrap();
        writeln!(o, "login inside: {}", login_inside.display()).unwrap();

        for path in root::mirrored_of(agent_type) {
            writeln!(o, "mirrored of: {}", path.display()).unwrap();
        }

        for (path, contents) in root::mirrored(&account) {
            writeln!(
                o,
                "mirrored: {} = {}",
                path.display(),
                contents.map_or("<none>".to_owned(), |bytes| text(tmp, &bytes))
            )
            .unwrap();
        }

        for memory in [true, false] {
            writeln!(o, "## memory {memory}").unwrap();

            let root = root_of(&account, &git_dir, &worktree).remembering(memory);

            writeln!(o, "named: {}", root.named()).unwrap();
            writeln!(o, "built: {:?}", root.built()).unwrap();

            for path in root.furnished_in(&built) {
                writeln!(o, "furnished: {}", said(tmp, &path)).unwrap();
            }

            writeln!(o, "account: {}", said(tmp, root.account())).unwrap();
            writeln!(o, "credentials: {}", said(tmp, &root.credentials())).unwrap();
            writeln!(
                o,
                "credentials in: {}",
                said(tmp, &root.credentials_in(&inside))
            )
            .unwrap();
            writeln!(o, "login alone: {}", root.login_alone()).unwrap();
            writeln!(
                o,
                "login of: {:?}",
                Root::login_of(&account, memory).map(|path| said(tmp, &path))
            )
            .unwrap();

            for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
                writeln!(
                    o,
                    "login copied on {platform:?}: {}",
                    root.login_copied(platform)
                )
                .unwrap();
            }

            for (attached, servers) in [("none", &[][..]), ("one", &servers[..])] {
                for (path, contents) in root.written(&built, servers) {
                    writeln!(
                        o,
                        "written ({attached}): {}\n{}\n.",
                        said(tmp, &path),
                        text(tmp, &contents)
                    )
                    .unwrap();
                }
            }

            if let Some(config_file) =
                crate::harness::of(agent_type).and_then(|harness| harness.copied(&account))
            {
                for (attached, servers) in [("none", &[][..]), ("one", &servers[..])] {
                    writeln!(
                        o,
                        "config ({attached}):\n{}\n.",
                        text(tmp, &root.config(config_file, servers))
                    )
                    .unwrap();
                }
            }

            for words in ["", "Be kind."] {
                writeln!(
                    o,
                    "instructed {words:?}: {:?}",
                    root.instructed(&built, words)
                        .map(|(path, contents)| (said(tmp, &path), text(tmp, &contents)))
                )
                .unwrap();
            }

            for path in root.unshared_in(&built) {
                writeln!(o, "unshared: {}", said(tmp, &path)).unwrap();
            }

            for part in root.synced(Some("repo-branch"), &["s1".to_owned(), "s2".to_owned()]) {
                writeln!(
                    o,
                    "synced: {} at {} for {:?}",
                    part.label,
                    said(tmp, &part.inside),
                    part.whose
                )
                .unwrap();
            }

            root.made_in_account().unwrap();

            for (from, to) in root.joined(&inside) {
                writeln!(o, "joined: {} -> {}", said(tmp, &from), said(tmp, &to)).unwrap();
            }
        }
    }

    writeln!(o, "# the home, made in").unwrap();

    for path in listed(&home) {
        writeln!(o, "{}", unplaced(tmp, &path)).unwrap();
    }

    out
}

/// Each harness's root is what it was.
///
/// On Linux alone, because what an Antigravity CLI root joins of its summaries
/// database is a platform's answer — see `AGY_SUMMARIES_JOINED` — and the record
/// is Linux's.
#[cfg(target_os = "linux")]
#[test]
fn every_built_root_is_the_one_each_harness_was_given_before_the_trait() {
    let rendered = rendered();

    if std::env::var_os("VERKSTEAD_BLESS_ROOTS").is_some() {
        std::fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_AT),
            &rendered,
        )
        .unwrap();
        return;
    }

    for (number, (now, then)) in rendered.lines().zip(GOLDEN.lines()).enumerate() {
        assert_eq!(now, then, "line {} of the rendering differs", number + 1);
    }

    assert_eq!(rendered.lines().count(), GOLDEN.lines().count());
}
