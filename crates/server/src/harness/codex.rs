//! Codex.

use std::path::{Path, PathBuf};

use std::time::SystemTime;

use verkstead_render::Broken;

use super::{
    Carrying, Declared, HOME_REFUSED, Harness, Line, MODEL, PROMPT, Resume, SignedOut, said,
};
use crate::platform::Platform;
use crate::sandbox::root::{self, Part, Place, Whose};
use crate::sandbox::{self, Home};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{self, Search};

/// The program a Codex session is run as.
pub(crate) const BINARY: &str = "codex";

/// What codex 0.149.0 opens its usage-limit banner with.
///
/// The stable prefix and no more: what follows it is the plan's — an upgrade to
/// Plus, an upgrade to Pro, credits to buy, an admin to ask — and every one of
/// those sentences carries this one in front of it. Read off the binary rather
/// than guessed at.
///
/// The apostrophe is the ASCII one codex writes, which is what keeps [`says_so`]
/// honest about it: a line of prose quoting the phrase opens with a quotation
/// mark, and ASCII punctuation is not decoration there.
const EXHAUSTED: &str = "you've hit your usage limit";

/// Where codex keeps the credentials a login writes, which inside the sandbox
/// has to be the file beside the configuration rather than a keyring.
///
/// Named because it is somebody else's spelling, the same bargain the
/// usage-limit phrase and the idle signature make: one place to edit when it
/// moves.
const CREDENTIAL_STORE: &str = "cli_auth_credentials_store";

/// And which directory codex resumes a session **in**, which for a conversation
/// carried across a cluster is this device's rather than the one the rollout
/// records.
///
/// A rollout records the directory its session ran in, and a carried one records
/// the *sending* device's Worktree — a path that does not exist here. Codex
/// 0.155.1 puts the two side by side and asks the human to pick, which is a
/// session stopped in front of nobody; this setting is that choice made once, and
/// the choice is the directory codex was started in, which is where the work now
/// is. Its other value would resume into the path the work has left.
///
/// Said on the line rather than written into the account's configuration, for the
/// reason [`CREDENTIAL_STORE`] is: the home belongs to the human's account.
/// And on the resume alone — see [`Resume::Subcommand`] — because an ordinary
/// launch resumes nothing.
///
/// Named for the reason the spelling above it is, and it is the same bargain: the
/// key is codex's and it will move, and moving it costs one edit here.
const RESUME_CWD: &str = "tui.resume_cwd";

/// What codex has on its Screen while it is working, and nothing of what it has
/// there when it is waiting for a human — see [`crate::sessions::Signature::AtWork`].
///
/// Read off codex 0.149.0 rather than guessed at, and the reading is the whole
/// reason this backend's answer is an at-work line rather than a prompt: the
/// frame codex leaves when its turn is over and the frame it draws mid-turn are
/// the same screen but for this one line. The composer, its placeholder — `Ask
/// Codex to do anything` — and the bar under it stand in both, so none of them
/// says anything about whether the session has stopped.
///
/// The fragment rather than the whole line, because the rest of it moves while
/// this does not: the spinner glyph in front changes every frame, and the
/// seconds count up.
///
/// Named for the same reason the spelling above is, and it is the same bargain
/// the usage-limit phrase makes: the wording is codex's and it will move, and
/// moving it costs one edit here.
const AT_WORK: &str = "esc to interrupt";

/// Codex.
pub(crate) struct Codex;

impl Harness for Codex {
    fn binary(&self) -> &str {
        BINARY
    }

    fn named(&self) -> &str {
        "Codex"
    }

    fn mark(&self) -> &str {
        include_str!("marks/codex.svg")
    }

    fn paths(&self) -> Vec<Declared<'_>> {
        vec![Declared {
            key: store::HOME,
            label: "Home directory",
            purpose: "Mounted at `~/.codex`.",
            placeholder: "/home/you/accounts/work/.codex",
            file: false,
            refused: HOME_REFUSED,
            within: &[],
        }]
    }

    fn found_at(&self) -> Vec<&str> {
        vec!["~/.codex"]
    }

    /// **Codex's is a subcommand rather than a flag**, which is the one line in this
    /// table whose shape changes rather than growing an argument: `codex resume
    /// <session-id> <prompt>`, the word straight after the binary and both of the
    /// others its positionals (codex 0.155.1's own help, and driven to prove it). The
    /// model flag and every override the ordinary line carries come across onto it —
    /// a resume without them comes up logged out and sitting on the trust prompt —
    /// and one more goes on that the ordinary line has no business with:
    /// [`RESUME_CWD`].
    ///
    /// **And nothing of codex's store has to be put right, which is why it is asked
    /// this way.** Codex files its rollouts by the date they were written rather than
    /// by the directory they were written in, so what the memory sync carries lands at
    /// the same relative path it left. A resume by an explicit id is resolved against
    /// the whole store and not against the directory codex was started in — driven
    /// against 0.155.1, which resumed a rollout recording a directory that machine
    /// never had — so where Grok needs its log moved, Codex needs nothing moved at
    /// all. Which is the answer worth having: the relocation writes into somebody
    /// else's store, and this does not.
    ///
    /// **What it has instead is a prompt about which directory to resume in.** Where a
    /// rollout records a directory other than the one codex is started in — which a
    /// carried one always does — 0.155.1 asks the human to choose between them, and a
    /// session stopped at a prompt is a run waiting on nobody. [`RESUME_CWD`] is
    /// that choice made: this device's Worktree, which is where the work now is.
    ///
    /// **Its id is the one Verkstead did not pick.** Codex takes no session id at
    /// launch, so what goes after the subcommand is the id the rollout gives itself —
    /// read out of the log as the Transcript search finds it and written onto the
    /// record there, which is what leaves a device that never ran the session able to
    /// resume it. See [`crate::transcript::names_itself`] and
    /// [`crate::transcript::in_the_store`].
    ///
    /// **What its account needs is said on the line rather than written into it.**
    /// Codex takes `-c key=value` overrides, which is how the home is configured
    /// without Verkstead writing into a directory that belongs to the human's
    /// account: the credential store file-backed, since there is no keyring inside
    /// the sandbox and a login that reached for one would find nothing; and the
    /// Worktree pre-seeded as trusted, since some versions still put the trust
    /// prompt up despite the bypass and a session stopped at a prompt is a run
    /// waiting on nobody.
    fn line(&self, worktree: Option<&Path>, _companions: &[PathBuf]) -> Line {
        let mut tail = vec![
            "--dangerously-bypass-approvals-and-sandbox".to_owned(),
            "--no-alt-screen".to_owned(),
            "-c".to_owned(),
            format!("{CREDENTIAL_STORE}=\"file\""),
        ];

        // A session with no Worktree is one that never starts — see
        // [`Sessions::start`] — so what this leaves off is the trust of a
        // directory there is none of, rather than a prompt let through.
        //
        // The whole table rather than the one key under it, because codex
        // splits a `-c` key on every dot it finds and does not stop at the
        // quotes: a Worktree named for a Repo or a branch with a dot in it
        // — and either may have one — is a path
        // `projects."…".trust_level` addresses some other way, and the
        // session then sits on the trust prompt for ever. A table on the
        // right-hand side is read as the TOML it is.
        if let Some(worktree) = worktree {
            tail.push("-c".to_owned());
            tail.push(format!(
                "projects={{\"{}\"={{trust_level=\"trusted\"}}}}",
                worktree.display()
            ));
        }

        Line {
            model: said(&["-m", MODEL]),
            prompt: said(&[PROMPT]),
            session: None,
            resume: Some(Resume::Subcommand {
                words: said(&["resume"]),
                also: vec!["-c".to_owned(), format!("{RESUME_CWD}=\"current\"")],
            }),
            tail,
            worktree: Vec::new(),
        }
    }

    fn at_work(&self) -> Option<&str> {
        Some(AT_WORK)
    }

    fn exhausted(&self) -> Option<&str> {
        Some(EXHAUSTED)
    }

    /// None: Verkstead cannot run a Codex login.
    fn signed_out(&self) -> Option<SignedOut<'_>> {
        None
    }

    fn channel(&self) -> store::Channel {
        store::Channel::StoreAndNudge
    }

    /// `mcp__<server>__<tool>`, as Claude Code's, read off Codex 0.155.1. It
    /// reaches the tool through a namespace of that first half rather than a
    /// flat name, which comes to the same thing from inside the session.
    fn tools_named(&self) -> &str {
        "mcp__<server>__<tool>"
    }

    fn environment(&self, _platform: Platform) -> Vec<(&str, &str)> {
        Vec::new()
    }

    fn kept_in(&self, home: &Path) -> store::Account {
        store::Account::home(
            store::AgentType::CODEX,
            home.join(sandbox::CODEX_INSIDE_HOME),
        )
    }

    fn account_inside(&self, account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)> {
        vec![(
            account.directory().to_owned(),
            home.join(sandbox::CODEX_INSIDE_HOME),
        )]
    }

    fn judged(&self, account: &store::Account) -> Vec<(PathBuf, Broken)> {
        vec![(account.directory().to_owned(), Broken::HomeMissing)]
    }

    fn root(&self) -> &str {
        sandbox::CODEX_INSIDE_HOME
    }

    fn login(&self) -> &str {
        root::AUTH
    }

    fn configuration(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(root::CODEX_CONFIG)]
    }

    /// A `config.toml`, carrying the account's own keys that reach a model
    /// and the Conversation's servers — see [`root::toml_carrying`].
    fn configured(
        &self,
        found: Vec<Option<Vec<u8>>>,
        servers: &[AttachedServer],
    ) -> Vec<(PathBuf, Vec<u8>)> {
        let [config] = <[_; 1]>::try_from(found).expect("one file asked for");

        vec![(
            PathBuf::from(root::CODEX_CONFIG),
            root::toml_carrying(
                config
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .as_deref(),
                &root::CODEX_CARRIED,
                servers,
                root::CODEX_HEADERS,
            ),
        )]
    }

    fn instructions(&self) -> PathBuf {
        PathBuf::from(root::AGENTS)
    }

    fn memory(&self, _place: Option<&Place>) -> Vec<PathBuf> {
        root::CODEX_MEMORY.iter().map(PathBuf::from).collect()
    }

    fn unshared(&self) -> Vec<&str> {
        root::CODEX_MEMORY.to_vec()
    }

    fn synced(
        &self,
        _place: Option<&Place>,
        worktree: Option<&str>,
        _sessions: &[String],
    ) -> Vec<Part> {
        vec![
            Part {
                label: root::MEMORY.into(),
                inside: PathBuf::from(root::CODEX_MEMORY[1]),
                whose: Whose::Everything,
            },
            Part {
                label: root::SESSIONS_PART.into(),
                inside: PathBuf::from(root::SESSIONS),
                whose: Whose::Naming(worktree.unwrap_or_default().to_owned()),
            },
        ]
    }

    fn mirrored(&self) -> Vec<PathBuf> {
        vec![Path::new(sandbox::CODEX_INSIDE_HOME).join(root::CODEX_CONFIG)]
    }

    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>> {
        let config = std::fs::read_to_string(account.directory().join(root::CODEX_CONFIG)).ok();

        vec![Some(root::toml_carrying(
            config.as_deref(),
            &root::CODEX_CARRIED,
            &[],
            root::CODEX_HEADERS,
        ))]
    }

    fn search(
        &self,
        account: &Path,
        memory: bool,
        home: &Home,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Search {
        transcript::codex_search(account, memory, home, session, worktree, launched)
    }

    fn carried<'a>(
        &'a self,
        account: &'a Path,
        memory: bool,
        session: &'a str,
        worktree: &'a Path,
        home: &'a Home,
    ) -> Carrying<'a> {
        Box::pin(transcript::codex_carried(
            account, memory, session, worktree, home,
        ))
    }

    fn model(&self, _role: store::Role) -> &str {
        "gpt-5-codex"
    }

    fn models(&self) -> Vec<(&str, &str)> {
        vec![("gpt-5-codex", "GPT-5 Codex")]
    }

    /// The section the two that store and nudge share, because what they
    /// share is real: the Set is stored, the turn ends, and `verkstead
    /// answers` is what fetches the Answers when the nudge lands.
    fn running(&self) -> &str {
        include_str!("running/store-and-nudge.md")
    }
}

/// Codex's plugin — `plugins/codex/`, embedded and stood up beside this one —
/// held to what this harness says and writes: the plugin is to be this
/// harness, so every difference here is one in the plugin.
#[cfg(test)]
mod tests {
    use super::*;

    use crate::harness::plugged::Plugged;

    async fn plugin() -> Plugged {
        crate::plugins::built_in::codex_plugged().await
    }

    /// A Profile form drawn from the manifest has the one path field this
    /// harness draws, labelled and explained as it is.
    #[tokio::test]
    async fn the_plugins_profile_form_is_one_home_directory() {
        let plugin = plugin().await;
        let (drawn, ours) = (plugin.paths(), Codex.paths());
        assert_eq!(drawn.len(), 1, "one path field");
        assert_eq!(ours.len(), 1);
        let (drawn, ours) = (&drawn[0], &ours[0]);

        assert_eq!(drawn.key, ours.key);
        assert_eq!(drawn.label, "Home directory");
        assert_eq!(drawn.label, ours.label);
        assert_eq!(drawn.purpose, ours.purpose);
        assert_eq!(drawn.placeholder, ours.placeholder);
        assert!(!drawn.file);
    }

    /// Everything about Codex that is data, as the manifest says it.
    #[tokio::test]
    async fn the_plugin_says_what_this_harness_says() {
        let plugin = plugin().await;

        assert_eq!(plugin.binary(), Codex.binary());
        assert_eq!(plugin.named(), Codex.named());
        assert_eq!(plugin.mark(), Codex.mark());
        assert_eq!(plugin.at_work(), Codex.at_work());
        assert_eq!(plugin.exhausted(), Codex.exhausted());
        assert_eq!(plugin.signed_out(), None);
        assert!(plugin.speaking().is_none(), "Verkstead runs no Codex login");
        assert_eq!(plugin.channel(), Codex.channel());
        assert_eq!(plugin.tools_named(), Codex.tools_named());
        assert_eq!(plugin.running(), Codex.running());
        assert_eq!(plugin.models(), Codex.models());
        for role in [
            store::Role::Grilling,
            store::Role::Implementation,
            store::Role::Review,
        ] {
            assert_eq!(plugin.model(role), Codex.model(role), "{role:?}");
        }
        assert_eq!(plugin.instructions(), Codex.instructions());
        assert_eq!(plugin.root(), Codex.root());
        assert_eq!(plugin.login(), Codex.login());
        assert_eq!(plugin.configuration(), Codex.configuration());
        assert_eq!(plugin.mirrored(), Codex.mirrored());
        assert_eq!(plugin.memory(None), Codex.memory(None));
        assert_eq!(plugin.unshared(), Codex.unshared());
        assert_eq!(
            plugin.environment(Platform::HERE),
            Codex.environment(Platform::HERE)
        );
        assert_eq!(plugin.written_in_worktree(), Codex.written_in_worktree());

        // The same directories synced by the same rules, each under the path
        // it is rather than this harness's own names for them.
        let synced = |parts: Vec<Part>| {
            let mut parts: Vec<(PathBuf, Whose)> = parts
                .into_iter()
                .map(|part| (part.inside, part.whose))
                .collect();
            parts.sort_by(|a, b| a.0.cmp(&b.0));
            parts
        };
        let sessions = ["s1".to_owned()];
        assert_eq!(
            synced(plugin.synced(None, Some("/srv/worktrees/tables"), &sessions)),
            synced(Codex.synced(None, Some("/srv/worktrees/tables"), &sessions)),
        );
    }

    /// What the sandbox suite's Codex accounts keep in their `config.toml`.
    const ACCOUNTS: &[Option<&str>] = &[
        None,
        Some(
            "model_provider = \"proxy\"\nnotify = [\"notify-send\"]\n\n\
             [model_providers.proxy]\nname = \"The proxy\"\n\
             base_url = \"https://proxy.example/v1\"\n\n\
             [mcp_servers.the-humans]\ncommand = \"npx\"\n\n\
             [features]\nmemories = true\n",
        ),
        Some(
            "model_provider = \"proxy\"\n\n[model_providers.proxy]\n\
             base_url = \"https://proxy.example/v1\"\n\n\
             [mcp_servers.the-humans]\ncommand = \"npx\"\n",
        ),
        Some("model = \"gpt-5-codex\"\n"),
        Some("model_provider = "),
    ];

    /// For each of them, with and without attached servers, `write-configuration`
    /// answers the `config.toml` this harness writes, byte for byte.
    #[tokio::test]
    async fn the_plugin_writes_the_config_toml_this_harness_writes() {
        let plugin = plugin().await;
        let attached = [
            AttachedServer::of(
                "docs",
                "https://mcp.example.com/docs",
                &[
                    ("Authorization", "Bearer sk-averysecretkey"),
                    ("X-Tenant", "verkstead"),
                ],
            ),
            AttachedServer::of("tickets", "https://mcp.example.com/tickets", &[]),
            AttachedServer::of("dotted.name", "https://mcp.example.com/\"quoted\"", &[]),
        ];

        for account in ACCOUNTS {
            for servers in [&[][..], &attached[..]] {
                let found = vec![account.map(|text| text.as_bytes().to_vec())];

                assert_eq!(
                    plugin.configured(found.clone(), servers),
                    Codex.configured(found, servers),
                    "{account:?} with {} servers",
                    servers.len()
                );
            }
        }
    }

    /// And an account's mirror holds what this harness's does.
    #[tokio::test]
    async fn the_plugins_mirror_is_this_harness_s() {
        let plugin = plugin().await;

        for config in ACCOUNTS {
            let home = tempfile::tempdir().unwrap();
            if let Some(config) = config {
                std::fs::write(home.path().join(root::CODEX_CONFIG), config).unwrap();
            }
            let account = store::Account::home(store::AgentType::CODEX, home.path().to_owned());

            assert_eq!(
                plugin.mirroring(&account),
                Codex.mirroring(&account),
                "{config:?}"
            );
        }
    }
}
