//! A Plugin's `plugin.yaml`, read and refused by field.
//!
//! **The grammar is `docs/plugins.md`**, which is the author's reference and the
//! one place later stages extend: each Category's own grammar is written there
//! by the stage that puts a Category behind it, and read here beside the rest.
//! This module is the reading of it, and [`Manifest::read`] the one way in.
//!
//! **A manifest is refused at the first field that is wrong, naming it** — see
//! [`Refusal`], whose words are screen copy, since the Plugins page draws the
//! same sentence `verkstead plugin check` prints. And **a key nothing here
//! knows is refused** rather than skipped, at the top and in every entry whose
//! shape this stage settles: a misspelt `capabilites:` read as no capabilities
//! is a plugin quietly doing less than its author wrote, which is the gap the
//! interface version exists to stop. A `provides:` entry's body beyond its
//! `category` is read here once a stage has given that Category its grammar —
//! [`agent`], `skills` in [`Walk::packs`], and `language` in the descriptor
//! grammar, see [`crate::languages::provided`] — and kept as written until then.
//!
//! **Per-platform overrides replace, key by key.** A top-level `linux:`,
//! `macos:` or `windows:` holds keys of the common declaration, and reading for
//! that platform takes each key present there in place of the common one,
//! whole: a `windows:` with `capabilities:` is Windows' entire list of
//! capabilities, not an addition to the common list. Whole rather than merged
//! inside, so that what a platform runs with is what one block says, and a
//! capability the common list grants cannot be taken away only by a merge rule
//! the author had to remember. Only what may honestly differ between machines
//! may be overridden — [`OVERRIDABLE`] — and the identity of a plugin is never
//! one of those.
//!
//! **Parsed as YAML into a tree first, then walked**, rather than derived onto
//! structs: the walk is what knows the field's whole name —
//! `dependencies[1].version` — and the override merge is a merge of trees.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value};

use crate::languages::Provided;
use crate::platform::Platform;

pub mod agent;

pub use agent::Agent;

/// The newest interface version this build knows. A manifest naming a higher
/// one was written for a newer Verkstead and is refused at load, rather than
/// read for what parses and left to show its gaps in a session hours later
/// (Set 1762, Q14).
pub const INTERFACE: u64 = 1;

/// The file that makes a directory a Plugin.
pub const FILE: &str = "plugin.yaml";

/// The keys a manifest may hold at its top, the three platform keys aside.
const KEYS: &[&str] = &[
    "uuid",
    "name",
    "author",
    "version",
    "repository",
    "interface",
    "description",
    "mark",
    "dependencies",
    "provides",
    "capabilities",
    "settings",
    "component",
];

/// The keys a platform's block may replace: everything but who the plugin is
/// and the interface it was written against.
pub const OVERRIDABLE: &[&str] = &[
    "description",
    "mark",
    "dependencies",
    "provides",
    "capabilities",
    "settings",
    "component",
];

/// A `plugin.yaml`, read for one platform.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// The plugin's identity: what its record, a dependency on it and anything
    /// stored about it are keyed by. Lowercase, hyphenated.
    pub uuid: String,
    /// What is drawn, and nothing more.
    pub name: String,
    pub author: Author,
    pub version: semver::Version,
    pub repository: Option<String>,
    /// The interface version it was written against, never above [`INTERFACE`].
    pub interface: u64,
    pub description: Option<String>,
    pub mark: Option<Mark>,
    pub dependencies: Vec<Dependency>,
    pub provides: Vec<Provision>,
    /// The Skill Packs its `skills` entries are, in the order written.
    pub packs: Vec<Pack>,
    pub capabilities: Vec<Capability>,
    pub settings: Vec<Setting>,
    /// A `.wasm` inside the plugin's directory, relative to it.
    pub component: Option<PathBuf>,
}

/// Who wrote a plugin, or a dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// The plugin's mark, which is SVG. Whether it is safe to draw is the viewer's
/// to make sure of when it draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark {
    /// Written in the manifest itself.
    Inline(String),
    /// A file inside the plugin's directory, relative to it.
    File(PathBuf),
}

/// Another plugin this one needs, said fully enough to name it and fetch it
/// without it being installed. Resolved against what is installed later, by
/// the loader; here it is only read.
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    pub uuid: String,
    pub name: String,
    pub author: Author,
    pub repository: Option<String>,
    /// Which of its versions will do.
    pub version: semver::VersionReq,
}

/// One thing a plugin provides, of one Category.
#[derive(Debug, Clone, PartialEq)]
pub struct Provision {
    pub category: Category,
    pub body: Body,
    /// The body read, where the Category is `language`.
    pub language: Option<Provided>,
}

/// A provision beyond its Category: read by its Category's grammar where the
/// stage that gives it one has landed, and kept as written where not.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Agent(Box<Agent>),
    Unread(Map<String, Value>),
}

impl Provision {
    /// The harness this provides, where it is an agent.
    pub fn agent(&self) -> Option<&Agent> {
        match &self.body {
            Body::Agent(agent) => Some(agent),
            Body::Unread(_) => None,
        }
    }
}

/// A **Skill Pack**: a `provides:` entry of the `skills` Category, read.
///
/// Pure data — it asks for no capability and carries no component. Each of its
/// skills is a directory holding a `SKILL.md` whose frontmatter names it, the
/// way the agents' own skill loaders read one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// What its skills are named under, `<pack>/<skill>`, and the directory
    /// they are installed in.
    pub name: String,
    /// The directory of `<skill>/SKILL.md`, relative to the plugin's.
    pub directory: PathBuf,
    /// Its skills' names, which are their directories', in order.
    pub skills: Vec<String>,
}

/// The pack only the Skill Pack Verkstead ships may be named: its plugin's
/// uuid is [`super::built_in::VERKSTEAD`].
pub const RESERVED: &str = "verkstead";

/// Which of the six kinds of thing a Plugin provides an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Agent,
    Process,
    Forge,
    Language,
    Activity,
    Skills,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Agent,
        Category::Process,
        Category::Forge,
        Category::Language,
        Category::Activity,
        Category::Skills,
    ];

    /// The word a manifest writes, and the one drawn.
    pub fn word(self) -> &'static str {
        match self {
            Category::Agent => "agent",
            Category::Process => "process",
            Category::Forge => "forge",
            Category::Language => "language",
            Category::Activity => "activity",
            Category::Skills => "skills",
        }
    }
}

/// Something a plugin's wasm component asks to reach, shown at **Install** and
/// granted there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// A path on the host, read-only unless it says otherwise.
    Filesystem { path: String, access: Access },
    /// A host it may open connections to.
    Outbound { host: String },
    /// An environment variable it may read.
    Environment { name: String },
}

impl Capability {
    /// The words a manifest writes for each kind, in the order they are offered.
    const KINDS: [&'static str; 3] = ["filesystem", "outbound", "environment"];
}

/// How much of a granted path a component may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    ReadWrite,
}

/// A settings field the plugin wants drawn in its pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    /// What the value is stored and handed over under.
    pub key: String,
    pub label: String,
    pub kind: SettingKind,
    pub description: Option<String>,
}

/// How a settings field is drawn and checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingKind {
    Text,
    /// A size in the grammar a language's `size` is: `30G`.
    Size,
    List,
    /// Kept in `secrets.yaml` and never returned to the viewer.
    Secret,
}

impl SettingKind {
    const ALL: [(&'static str, SettingKind); 4] = [
        ("text", SettingKind::Text),
        ("size", SettingKind::Size),
        ("list", SettingKind::List),
        ("secret", SettingKind::Secret),
    ];
}

/// Why a directory is not a plugin Verkstead would load: the field at fault
/// where there is one, and the sentence that says what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The field's whole name, `dependencies[0].version`; none where the fault
    /// is the file's as a whole.
    pub field: Option<String>,
    /// Screen copy, drawn as it is on the Plugins page.
    pub why: String,
}

impl Refusal {
    pub(crate) fn of(field: &str, why: impl Into<String>) -> Self {
        Refusal {
            field: Some(field.to_owned()),
            why: why.into(),
        }
    }

    fn missing(field: &str) -> Self {
        Refusal::of(field, format!("`{field}` is missing."))
    }

    fn unknown(field: &str) -> Self {
        Refusal::of(field, format!("`{field}` isn't a key a plugin has."))
    }

    pub(crate) fn unknown_to_a_language(field: &str) -> Self {
        Refusal::of(field, format!("`{field}` isn't a key a language has."))
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.why)
    }
}

impl std::error::Error for Refusal {}

/// The key a platform's block is written under.
pub fn platform_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Linux => "linux",
        Platform::MacOs => "macos",
        Platform::Windows => "windows",
    }
}

const PLATFORMS: [Platform; 3] = [Platform::Linux, Platform::MacOs, Platform::Windows];

impl Manifest {
    /// Read `dir`'s `plugin.yaml` as `platform` would load it.
    pub fn read(dir: &Path, platform: Platform) -> Result<Manifest, Refusal> {
        let path = dir.join(FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Refusal {
                    field: None,
                    why: format!("There's no `{FILE}` here, so this isn't a plugin."),
                });
            }
            Err(error) => {
                return Err(Refusal {
                    field: None,
                    why: format!("`{FILE}` couldn't be read: {error}."),
                });
            }
        };

        Manifest::parse(&text, Files::Dir(dir), platform)
    }

    /// Read a manifest's text, with `files` as where the paths it names are.
    pub fn parse(text: &str, files: Files<'_>, platform: Platform) -> Result<Manifest, Refusal> {
        let tree = serde_saphyr::from_str::<Value>(text).map_err(|error| Refusal {
            field: None,
            why: format!("`{FILE}` isn't YAML Verkstead can read: {error}."),
        })?;
        let Value::Object(tree) = tree else {
            return Err(Refusal {
                field: None,
                why: format!("`{FILE}` should be a map of keys, starting with `uuid`."),
            });
        };

        let fields = merged(tree, platform)?;
        let walk = Walk { files, platform };

        let interface = walk.interface(&fields)?;
        let uuid = uuid(required(&fields, "uuid")?, "uuid")?;
        let provides = languages(walk.provides(&fields)?, text, platform)?;
        let packs = walk.packs(&provides, &uuid)?;
        let settings = settings(&fields)?;
        sized(&provides, &settings)?;

        let manifest = Manifest {
            uuid,
            name: text_of(required(&fields, "name")?, "name")?,
            author: author(required(&fields, "author")?, "author")?,
            version: version(required(&fields, "version")?, "version")?,
            repository: optional_text(&fields, "repository", "repository")?,
            interface,
            description: optional_text(&fields, "description", "description")?,
            mark: walk.mark(&fields, "mark", "mark")?,
            dependencies: list(&fields, "dependencies", dependency)?,
            provides,
            packs,
            capabilities: list(&fields, "capabilities", capability)?,
            settings,
            component: walk.component(&fields)?,
        };
        manifest.whole()?;
        Ok(manifest)
    }

    /// Refused where what it provides needs what the rest of it does not say:
    /// an agent is drawn with a mark, its own or the plugin's, and is three
    /// exports of a component.
    fn whole(&self) -> Result<(), Refusal> {
        for (index, provision) in self.provides.iter().enumerate() {
            let Some(agent) = provision.agent() else {
                continue;
            };
            if agent.mark.is_none() && self.mark.is_none() {
                let field = format!("provides[{index}].mark");
                return Err(Refusal::of(
                    &field,
                    format!(
                        "`{field}` is missing. An agent is drawn with a mark: give it one, or \
                         give the plugin one."
                    ),
                ));
            }
            if self.component.is_none() {
                return Err(Refusal::of(
                    "component",
                    "`component` is missing. A plugin providing an agent needs one, exporting \
                     what the `agent` world asks for.",
                ));
            }
        }
        Ok(())
    }

    /// Read `dir` as each platform would, for `verkstead plugin check`: the
    /// manifest as this machine reads it, or every refusal any platform meets,
    /// each with the platforms it was met on.
    pub fn check(dir: &Path) -> Result<Manifest, Vec<(Refusal, Vec<Platform>)>> {
        let mut refusals: Vec<(Refusal, Vec<Platform>)> = Vec::new();
        let mut here = None;

        for platform in PLATFORMS {
            match Manifest::read(dir, platform) {
                Ok(manifest) if platform == Platform::HERE => here = Some(manifest),
                Ok(_) => {}
                Err(refusal) => match refusals.iter_mut().find(|(seen, _)| *seen == refusal) {
                    Some((_, platforms)) => platforms.push(platform),
                    None => refusals.push((refusal, vec![platform])),
                },
            }
        }

        match here {
            Some(manifest) if refusals.is_empty() => Ok(manifest),
            _ => Err(refusals),
        }
    }
}

/// The common declaration with `platform`'s block written over it, key by key,
/// and the other platforms' blocks checked and set aside.
fn merged(mut tree: Map<String, Value>, platform: Platform) -> Result<Map<String, Value>, Refusal> {
    let mut chosen = None;

    for other in PLATFORMS {
        let key = platform_key(other);
        let Some(block) = tree.remove(key) else {
            continue;
        };
        let Value::Object(block) = block else {
            return Err(Refusal::of(
                key,
                format!("`{key}` should be a map of the keys that differ on that platform."),
            ));
        };
        for name in block.keys() {
            if !OVERRIDABLE.contains(&name.as_str()) {
                let field = format!("{key}.{name}");
                return Err(if KEYS.contains(&name.as_str()) {
                    Refusal::of(&field, format!("`{field}` can't differ by platform."))
                } else {
                    Refusal::unknown(&field)
                });
            }
        }
        if other == platform {
            chosen = Some(block);
        }
    }

    if let Some(name) = tree.keys().find(|name| !KEYS.contains(&name.as_str())) {
        return Err(Refusal::unknown(name));
    }

    if let Some(block) = chosen {
        tree.extend(block);
    }

    Ok(tree)
}

/// Where the files a manifest names are: a plugin's directory, or a built-in's
/// files embedded in this binary beside its manifest — see
/// [`super::built_in`].
#[derive(Debug, Clone, Copy)]
pub enum Files<'a> {
    Dir(&'a Path),
    /// Each file by its path relative to the plugin's directory, written with
    /// `/`.
    Embedded(&'a [(&'a str, &'a [u8])]),
}

impl Files<'_> {
    /// `path`'s contents, or `None` where there is no such file.
    pub fn read(&self, path: &Path) -> Option<Vec<u8>> {
        match self {
            Files::Dir(dir) => {
                let path = dir.join(path);
                path.is_file().then(|| std::fs::read(path).ok()).flatten()
            }
            Files::Embedded(files) => files
                .iter()
                .find(|(name, _)| Path::new(name) == path)
                .map(|(_, bytes)| bytes.to_vec()),
        }
    }

    /// Whether there is a file at `path`, without reading it.
    pub fn has(&self, path: &Path) -> bool {
        match self {
            Files::Dir(dir) => dir.join(path).is_file(),
            Files::Embedded(files) => files.iter().any(|(name, _)| Path::new(name) == path),
        }
    }

    /// Every file under the directory `path`, each relative to it, sorted; or
    /// `None` where there is no such directory.
    pub fn within(&self, path: &Path) -> Option<Vec<PathBuf>> {
        let mut found = match self {
            Files::Dir(dir) => {
                let top = dir.join(path);
                if !top.is_dir() {
                    return None;
                }
                let mut found = Vec::new();
                walked(&top, Path::new(""), &mut found);
                found
            }
            Files::Embedded(files) => {
                let found: Vec<PathBuf> = files
                    .iter()
                    .filter_map(|(name, _)| Path::new(name).strip_prefix(path).ok())
                    .filter(|inside| inside.components().next().is_some())
                    .map(Path::to_path_buf)
                    .collect();
                if found.is_empty() {
                    return None;
                }
                found
            }
        };
        found.sort();
        Some(found)
    }
}

/// Every file under `dir`, written into `found` relative to the directory the
/// walk began at, `at` being `dir`'s place in it.
fn walked(dir: &Path, at: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let inside = at.join(entry.file_name());
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => walked(&entry.path(), &inside, found),
            Ok(kind) if kind.is_file() => found.push(inside),
            _ => {}
        }
    }
}

/// What reading a path needs: where the files it may name are, and which
/// platform a provision's own blocks are read for.
struct Walk<'a> {
    files: Files<'a>,
    platform: Platform,
}

impl Walk<'_> {
    fn interface(&self, fields: &Map<String, Value>) -> Result<u64, Refusal> {
        let field = "interface";
        let value = required(fields, field)?;
        let Some(interface) = value.as_u64().filter(|interface| *interface >= 1) else {
            return Err(Refusal::of(
                field,
                format!("`{field}` should be a whole number from 1, like `{INTERFACE}`."),
            ));
        };
        if interface > INTERFACE {
            return Err(Refusal::of(
                field,
                format!(
                    "`{field}` is {interface}, and this Verkstead reads up to {INTERFACE}. \
                     The plugin is for a newer Verkstead."
                ),
            ));
        }
        Ok(interface)
    }

    /// The mark at `key` of `fields`, named `field` in a refusal.
    fn mark(
        &self,
        fields: &Map<String, Value>,
        key: &str,
        field: &str,
    ) -> Result<Option<Mark>, Refusal> {
        let Some(mark) = optional_text(fields, key, field)? else {
            return Ok(None);
        };

        if mark.trim_start().starts_with('<') {
            return if is_svg(&mark) {
                Ok(Some(Mark::Inline(mark)))
            } else {
                Err(Refusal::of(field, format!("`{field}` isn't SVG.")))
            };
        }

        let path = self.inside(&mark, field)?;
        match self.files.read(&path).map(String::from_utf8) {
            Some(Ok(text)) if is_svg(&text) => Ok(Some(Mark::File(path))),
            Some(_) => Err(Refusal::of(field, format!("`{mark}` isn't SVG."))),
            None => Err(Refusal::of(
                field,
                format!("`{mark}` isn't a file in the plugin's directory."),
            )),
        }
    }

    fn provides(&self, fields: &Map<String, Value>) -> Result<Vec<Provision>, Refusal> {
        list(fields, "provides", provision)?
            .into_iter()
            .enumerate()
            .map(|(index, (category, body))| {
                let body = match category {
                    Category::Agent => {
                        Body::Agent(Box::new(self.agent(&body, &format!("provides[{index}]"))?))
                    }
                    _ => Body::Unread(body),
                };
                Ok(Provision {
                    category,
                    body,
                    language: None,
                })
            })
            .collect()
    }

    fn component(&self, fields: &Map<String, Value>) -> Result<Option<PathBuf>, Refusal> {
        let field = "component";
        let Some(component) = optional_text(fields, field, field)? else {
            return Ok(None);
        };

        let path = self.inside(&component, field)?;
        if path.extension().is_none_or(|extension| extension != "wasm") {
            return Err(Refusal::of(
                field,
                format!("`{field}` should name a `.wasm` file."),
            ));
        }
        if !self.files.has(&path) {
            return Err(Refusal::of(
                field,
                format!("`{component}` isn't a file in the plugin's directory."),
            ));
        }
        Ok(Some(path))
    }

    /// Every `skills` entry of `provides` read as a [`Pack`], each of its skills'
    /// `SKILL.md` read for its frontmatter. Refused at the first that is wrong:
    /// a skill's refusal names the skill.
    fn packs(&self, provides: &[Provision], uuid: &str) -> Result<Vec<Pack>, Refusal> {
        let mut packs: Vec<Pack> = Vec::new();
        for (index, provision) in provides.iter().enumerate() {
            if provision.category != Category::Skills {
                continue;
            }
            let Body::Unread(body) = &provision.body else {
                continue;
            };
            let field = format!("provides[{index}]");
            let pack = self.pack(body, &field, uuid)?;
            if packs.iter().any(|seen| seen.name == pack.name) {
                let field = format!("{field}.name");
                return Err(Refusal::of(
                    &field,
                    format!(
                        "`{field}` is `{}`, which an earlier entry already uses.",
                        pack.name
                    ),
                ));
            }
            packs.push(pack);
        }
        Ok(packs)
    }

    fn pack(&self, body: &Map<String, Value>, field: &str, uuid: &str) -> Result<Pack, Refusal> {
        if let Some(key) = body
            .keys()
            .find(|key| !["name", "directory"].contains(&key.as_str()))
        {
            return Err(Refusal::unknown(&format!("{field}.{key}")));
        }

        let name_field = format!("{field}.name");
        let name = text_of(within(body, field, "name")?, &name_field)?;
        if !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(Refusal::of(
                &name_field,
                format!(
                    "`{name_field}` should be lowercase letters, digits and hyphens, like \
                     `rust-review`."
                ),
            ));
        }
        if name == RESERVED && uuid != super::built_in::VERKSTEAD {
            return Err(Refusal::of(
                &name_field,
                format!(
                    "`{name_field}` is `{RESERVED}`, which is the name of the skills Verkstead \
                     ships. Give the pack a name of its own."
                ),
            ));
        }

        let directory_field = format!("{field}.directory");
        let written = text_of(within(body, field, "directory")?, &directory_field)?;
        let directory = self.inside(&written, &directory_field)?;
        let Some(files) = self.files.within(&directory) else {
            return Err(Refusal::of(
                &directory_field,
                format!("`{written}` isn't a directory in the plugin's directory."),
            ));
        };

        // Each directory at its top is a skill; a file there is the pack's own,
        // a README say, and no skill.
        let mut skills: Vec<String> = Vec::new();
        for file in &files {
            let mut components = file.components();
            let (Some(first), Some(_)) = (components.next(), components.next()) else {
                continue;
            };
            let skill = first.as_os_str().to_string_lossy().into_owned();
            if !skills.contains(&skill) {
                skills.push(skill);
            }
        }
        if skills.is_empty() {
            return Err(Refusal::of(
                &directory_field,
                format!("`{written}` holds no skills. Each is a directory with a `SKILL.md`."),
            ));
        }

        for skill in &skills {
            let path = directory.join(skill).join("SKILL.md");
            let text = match self.files.read(&path) {
                Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                None => {
                    return Err(Refusal::of(
                        &directory_field,
                        format!("The skill `{skill}` has no `SKILL.md`."),
                    ));
                }
            };
            frontmatter(&text, skill).map_err(|why| Refusal::of(&directory_field, why))?;
        }

        Ok(Pack {
            name,
            directory,
            skills,
        })
    }

    /// `written` as a path relative to the plugin's directory, refused where it
    /// would reach outside it.
    fn inside(&self, written: &str, field: &str) -> Result<PathBuf, Refusal> {
        let path = PathBuf::from(written);
        let within = path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
        if within {
            Ok(path)
        } else {
            Err(Refusal::of(
                field,
                format!("`{field}` should be a path inside the plugin's directory."),
            ))
        }
    }
}

/// Refused, in a sentence naming `skill`, where `text` — its `SKILL.md` — does
/// not open with frontmatter carrying its own name and a description.
fn frontmatter(text: &str, skill: &str) -> Result<(), String> {
    let missing = || {
        format!(
            "The skill `{skill}` has no frontmatter. Its `SKILL.md` should open with a `---` \
             line, its `name` and `description`, and another `---` line."
        )
    };

    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Err(missing());
    }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return Err(missing());
    }

    // YAML where it is, and one `key: value` a line where it is not: a
    // description holding `: ` is not YAML, and the agents' skill loaders
    // read it all the same, so this does too.
    let fields = match serde_saphyr::from_str::<Value>(&yaml) {
        Ok(Value::Object(fields)) => fields,
        _ => yaml
            .lines()
            .filter(|line| !line.starts_with([' ', '\t', '#']))
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| {
                (
                    key.trim().to_owned(),
                    Value::String(value.trim().to_owned()),
                )
            })
            .collect(),
    };

    match fields.get("name") {
        Some(Value::String(name)) if name == skill => {}
        Some(Value::String(name)) => {
            return Err(format!(
                "The skill `{skill}` is named `{name}` in its frontmatter. Its `name` should be \
                 its directory's, `{skill}`."
            ));
        }
        _ => return Err(format!("The skill `{skill}`'s frontmatter has no `name`.")),
    }

    match fields.get("description") {
        Some(Value::String(description)) if !description.trim().is_empty() => Ok(()),
        _ => Err(format!(
            "The skill `{skill}`'s frontmatter has no `description`."
        )),
    }
}

/// `written` as a relative path that stays where it is said from: a path a
/// harness keeps inside an account or a home, rather than one in the plugin's
/// directory.
fn relative(written: &str, field: &str) -> Result<PathBuf, Refusal> {
    let path = PathBuf::from(written);
    let within = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if within {
        Ok(path)
    } else {
        Err(Refusal::of(
            field,
            format!("`{field}` should be a relative path that stays inside where it's said from."),
        ))
    }
}

/// `written` as a path that stays inside where it is said from, or `None`:
/// what a file a plugin's component answers is held to — see
/// [`crate::harness`].
pub(crate) fn relative_path(written: &str) -> Option<PathBuf> {
    relative(written, "")
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
}

/// Whether `text` is SVG, which is all the reading asks of a mark.
fn is_svg(text: &str) -> bool {
    text.contains("<svg")
}

fn required<'a>(fields: &'a Map<String, Value>, field: &str) -> Result<&'a Value, Refusal> {
    match fields.get(field) {
        Some(Value::Null) | None => Err(Refusal::missing(field)),
        Some(value) => Ok(value),
    }
}

/// The field `key` of `fields`, named `field` in a refusal.
fn optional_text(
    fields: &Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<String>, Refusal> {
    match fields.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => text_of(value, field).map(Some),
    }
}

fn text_of(value: &Value, field: &str) -> Result<String, Refusal> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Ok(text.clone()),
        Value::String(_) => Err(Refusal::of(field, format!("`{field}` is empty."))),
        _ => Err(Refusal::of(field, format!("`{field}` should be text."))),
    }
}

/// A map whose keys are all `allowed`, at `field`.
fn entry<'a>(
    value: &'a Value,
    field: &str,
    allowed: &[&str],
) -> Result<&'a Map<String, Value>, Refusal> {
    let Value::Object(map) = value else {
        return Err(Refusal::of(field, format!("`{field}` should be a map.")));
    };
    if let Some(name) = map.keys().find(|name| !allowed.contains(&name.as_str())) {
        return Err(Refusal::unknown(&format!("{field}.{name}")));
    }
    Ok(map)
}

/// Something in `map` at `key`, with `field.key` as its whole name.
fn within<'a>(map: &'a Map<String, Value>, field: &str, key: &str) -> Result<&'a Value, Refusal> {
    match map.get(key) {
        Some(Value::Null) | None => Err(Refusal::missing(&format!("{field}.{key}"))),
        Some(value) => Ok(value),
    }
}

fn optional_within(
    map: &Map<String, Value>,
    field: &str,
    key: &str,
) -> Result<Option<String>, Refusal> {
    optional_text(map, key, &format!("{field}.{key}"))
}

/// The list at `key`, each entry read by `read` with its index in its name.
/// Absent is empty.
fn list<T>(
    fields: &Map<String, Value>,
    key: &str,
    read: fn(&Value, &str) -> Result<T, Refusal>,
) -> Result<Vec<T>, Refusal> {
    match fields.get(key) {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(Value::Array(entries)) => entries
            .iter()
            .enumerate()
            .map(|(index, value)| read(value, &format!("{key}[{index}]")))
            .collect(),
        Some(_) => Err(Refusal::of(key, format!("`{key}` should be a list."))),
    }
}

/// A uuid in its hyphenated form, taken down to lowercase.
fn uuid(value: &Value, field: &str) -> Result<String, Refusal> {
    let text = text_of(value, field)?.to_ascii_lowercase();
    let groups: Vec<&str> = text.split('-').collect();
    let shaped = groups.iter().map(|group| group.len()).eq([8, 4, 4, 4, 12])
        && groups
            .iter()
            .all(|group| group.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if shaped {
        Ok(text)
    } else {
        Err(Refusal::of(
            field,
            format!("`{field}` should be a uuid, like `0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90`."),
        ))
    }
}

fn author(value: &Value, field: &str) -> Result<Author, Refusal> {
    let map = entry(value, field, &["name", "email"])?;
    let name = text_of(within(map, field, "name")?, &format!("{field}.name"))?;
    let email_field = format!("{field}.email");
    let email = text_of(within(map, field, "email")?, &email_field)?;
    if !email.contains('@') {
        return Err(Refusal::of(
            &email_field,
            format!("`{email_field}` should be an email address."),
        ));
    }
    Ok(Author { name, email })
}

fn version(value: &Value, field: &str) -> Result<semver::Version, Refusal> {
    let not_semantic = || {
        Refusal::of(
            field,
            format!("`{field}` isn't a semantic version. Write one like `1.4.0`."),
        )
    };
    let Value::String(text) = value else {
        return Err(not_semantic());
    };
    semver::Version::parse(text).map_err(|_| not_semantic())
}

fn dependency(value: &Value, field: &str) -> Result<Dependency, Refusal> {
    let map = entry(
        value,
        field,
        &["uuid", "name", "author", "repository", "version"],
    )?;
    let name = text_of(within(map, field, "name")?, &format!("{field}.name"))?;

    let version_field = format!("{field}.version");
    let version = match map.get("version") {
        Some(Value::Null) | None => {
            return Err(Refusal::of(
                &version_field,
                format!(
                    "`{version_field}` is missing. Say which versions of {name} will do, \
                     like `^1.2`."
                ),
            ));
        }
        Some(Value::String(text)) => semver::VersionReq::parse(text).map_err(|_| {
            Refusal::of(
                &version_field,
                format!("`{version_field}` isn't a version specifier. Write one like `^1.2`."),
            )
        })?,
        Some(_) => {
            return Err(Refusal::of(
                &version_field,
                format!("`{version_field}` should be a version specifier, like `^1.2`."),
            ));
        }
    };

    Ok(Dependency {
        uuid: uuid(within(map, field, "uuid")?, &format!("{field}.uuid"))?,
        name,
        author: author(within(map, field, "author")?, &format!("{field}.author"))?,
        repository: optional_within(map, field, "repository")?,
        version,
    })
}

/// A provision's Category and the rest of it, which its Category's grammar
/// reads.
fn provision(value: &Value, field: &str) -> Result<(Category, Map<String, Value>), Refusal> {
    let Value::Object(map) = value else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should be a map, starting with its `category`."),
        ));
    };
    let category_field = format!("{field}.category");
    let word = text_of(within(map, field, "category")?, &category_field)?;
    let Some(category) = Category::ALL
        .into_iter()
        .find(|category| category.word() == word)
    else {
        return Err(Refusal::of(
            &category_field,
            format!(
                "`{category_field}` is `{word}`, which isn't a category. It's one of agent, \
                 process, forge, language, activity, or skills."
            ),
        ));
    };

    if category == Category::Language {
        text_of(
            within(map, field, "language")?,
            &format!("{field}.language"),
        )?;
    }

    let mut body = map.clone();
    body.remove("category");
    Ok((category, body))
}

/// `provides` with each `language` entry's body read in the descriptor
/// grammar.
///
/// Out of the manifest's text a second time rather than out of the tree the
/// rest was read from, because that tree is a map that sorts its keys, and a
/// descriptor's variables are a session's environment in the order they were
/// written — see [`crate::languages::Languages`].
fn languages(
    mut provides: Vec<Provision>,
    text: &str,
    platform: Platform,
) -> Result<Vec<Provision>, Refusal> {
    if provides
        .iter()
        .all(|provision| provision.category != Category::Language)
    {
        return Ok(provides);
    }

    let blocks = crate::languages::blocks(text, platform);
    let mut names: Vec<String> = Vec::new();

    for (index, provision) in provides.iter_mut().enumerate() {
        if provision.category != Category::Language {
            continue;
        }

        let field = format!("provides[{index}]");
        let provided = crate::languages::provided(blocks.get(index), &field)?;

        if names.contains(&provided.name) {
            let field = format!("{field}.language");
            return Err(Refusal::of(
                &field,
                format!(
                    "`{field}` is `{}`, which an earlier entry already provides.",
                    provided.name
                ),
            ));
        }

        names.push(provided.name.clone());
        provision.language = Some(provided);
    }

    Ok(provides)
}

fn capability(value: &Value, field: &str) -> Result<Capability, Refusal> {
    let Value::Object(map) = value else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should be a map naming one capability."),
        ));
    };

    let kinds: Vec<&str> = Capability::KINDS
        .into_iter()
        .filter(|kind| map.contains_key(*kind))
        .collect();
    let kind = match kinds.as_slice() {
        [kind] => *kind,
        [] => {
            let named = map
                .keys()
                .find(|key| *key != "access")
                .map_or_else(|| "nothing".to_owned(), |key| format!("`{key}`"));
            return Err(Refusal::of(
                field,
                format!(
                    "`{field}` asks for {named}, which isn't a capability Verkstead knows. \
                     It's one of filesystem, outbound, or environment."
                ),
            ));
        }
        _ => {
            return Err(Refusal::of(
                field,
                format!("`{field}` names more than one capability. Give each its own entry."),
            ));
        }
    };

    let allowed: &[&str] = if kind == "filesystem" {
        &["filesystem", "access"]
    } else {
        &[kind]
    };
    let map = entry(value, field, allowed)?;
    let named = text_of(&map[kind], &format!("{field}.{kind}"))?;

    Ok(match kind {
        "filesystem" => {
            let access_field = format!("{field}.access");
            let access = match optional_text(map, "access", &access_field)?.as_deref() {
                None | Some("read-only") => Access::ReadOnly,
                Some("read-write") => Access::ReadWrite,
                Some(_) => {
                    return Err(Refusal::of(
                        &access_field,
                        format!("`{access_field}` should be `read-only` or `read-write`."),
                    ));
                }
            };
            Capability::Filesystem {
                path: named,
                access,
            }
        }
        "outbound" => Capability::Outbound { host: named },
        _ => Capability::Environment { name: named },
    })
}

fn settings(fields: &Map<String, Value>) -> Result<Vec<Setting>, Refusal> {
    let settings = list(fields, "settings", setting)?;
    for (index, setting) in settings.iter().enumerate() {
        if settings[..index].iter().any(|seen| seen.key == setting.key) {
            let field = format!("settings[{index}].key");
            return Err(Refusal::of(
                &field,
                format!(
                    "`{field}` is `{}`, which an earlier setting already uses.",
                    setting.key
                ),
            ));
        }
    }
    Ok(settings)
}

/// A plugin providing a language may declare a `size` setting only of kind
/// `size`: it is what `{size}` reads and what the language's stores are held
/// to, so a value that is not a size would be a store held to nothing anybody
/// typed.
fn sized(provides: &[Provision], settings: &[Setting]) -> Result<(), Refusal> {
    if !provides
        .iter()
        .any(|provision| provision.language.is_some())
    {
        return Ok(());
    }

    match settings
        .iter()
        .position(|setting| setting.key == crate::languages::SIZE_SETTING)
    {
        Some(index) if settings[index].kind != SettingKind::Size => {
            let field = format!("settings[{index}].kind");
            Err(Refusal::of(
                &field,
                format!(
                    "`{field}` should be `size`: a language's `size` setting is the size its \
                     stores are held to."
                ),
            ))
        }
        _ => Ok(()),
    }
}

fn setting(value: &Value, field: &str) -> Result<Setting, Refusal> {
    let map = entry(value, field, &["key", "label", "kind", "description"])?;
    let kind_field = format!("{field}.kind");
    let word = text_of(within(map, field, "kind")?, &kind_field)?;
    let Some((_, kind)) = SettingKind::ALL.into_iter().find(|(name, _)| *name == word) else {
        return Err(Refusal::of(
            &kind_field,
            format!(
                "`{kind_field}` is `{word}`, which isn't a kind of setting. It's one of text, \
                 size, list, or secret."
            ),
        ));
    };

    Ok(Setting {
        key: text_of(within(map, field, "key")?, &format!("{field}.key"))?,
        label: text_of(within(map, field, "label")?, &format!("{field}.label"))?,
        kind,
        description: optional_within(map, field, "description")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The least a plugin can be: who it is, and nothing it provides.
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

    fn read(yaml: &str) -> Result<Manifest, Refusal> {
        read_for(yaml, Platform::Linux)
    }

    fn read_for(yaml: &str, platform: Platform) -> Result<Manifest, Refusal> {
        Manifest::parse(yaml, Files::Dir(Path::new("/nonexistent")), platform)
    }

    /// The field a manifest was refused at, insisting it was refused.
    fn refused_at(yaml: &str) -> String {
        let refusal = read(yaml).expect_err("the manifest should be refused");
        refusal.field.expect("the refusal should name a field")
    }

    #[test]
    fn a_minimal_manifest_is_read() {
        let manifest = read(MINIMAL).unwrap();

        assert_eq!(manifest.uuid, "0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90");
        assert_eq!(manifest.name, "Nothing at all");
        assert_eq!(manifest.author.email, "ada@example.com");
        assert_eq!(manifest.version, semver::Version::new(0, 1, 0));
        assert_eq!(manifest.interface, 1);
        assert!(manifest.provides.is_empty());
        assert!(manifest.capabilities.is_empty());
    }

    #[test]
    fn a_missing_uuid_is_refused_naming_it() {
        let yaml = MINIMAL.replace("uuid: 0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90\n", "");

        assert_eq!(refused_at(&yaml), "uuid");
        assert_eq!(read(&yaml).unwrap_err().why, "`uuid` is missing.");
    }

    #[test]
    fn a_version_that_is_not_semantic_is_refused() {
        let yaml = MINIMAL.replace("version: 0.1.0", "version: \"1.0\"");

        assert_eq!(refused_at(&yaml), "version");
    }

    #[test]
    fn a_dependency_with_no_version_specifier_is_refused() {
        let yaml = format!(
            "{MINIMAL}dependencies:
  - uuid: 7c1d2a43-0f5e-4b9a-8c6d-3e2f1a0b9c8d
    name: Shared skills
    author:
      name: Grace Hopper
      email: grace@example.com
"
        );

        assert_eq!(refused_at(&yaml), "dependencies[0].version");
        assert!(read(&yaml).unwrap_err().why.contains("Shared skills"));
    }

    #[test]
    fn a_dependency_is_read_with_its_specifier() {
        let yaml = format!(
            "{MINIMAL}dependencies:
  - uuid: 7c1d2a43-0f5e-4b9a-8c6d-3e2f1a0b9c8d
    name: Shared skills
    author:
      name: Grace Hopper
      email: grace@example.com
    repository: https://example.com/skills.git
    version: ^2
"
        );

        let dependency = &read(&yaml).unwrap().dependencies[0];
        assert!(dependency.version.matches(&semver::Version::new(2, 3, 0)));
        assert!(!dependency.version.matches(&semver::Version::new(1, 4, 0)));
    }

    #[test]
    fn an_unknown_category_is_refused() {
        let yaml = MINIMAL.replace("provides: []", "provides:\n  - category: theme\n");

        assert_eq!(refused_at(&yaml), "provides[0].category");
    }

    #[test]
    fn a_provision_keeps_its_body() {
        let yaml = MINIMAL.replace(
            "provides: []",
            "provides:\n  - category: forge\n    noun: merge request\n",
        );

        let provision = &read(&yaml).unwrap().provides[0];
        assert_eq!(provision.category, Category::Forge);
        let Body::Unread(body) = &provision.body else {
            panic!("a forge's body has no grammar yet");
        };
        assert_eq!(body["noun"], "merge request");
    }

    /// A manifest providing one language, `rust`, whose entry goes on with
    /// `rest` — written at the entry's own indentation.
    fn providing(rest: &str) -> String {
        MINIMAL.replace(
            "provides: []",
            &format!("provides:\n  - category: language\n    language: rust\n{rest}"),
        )
    }

    #[test]
    fn a_languages_size_setting_is_a_size() {
        let settings = |kind: &str| {
            providing("    label: Rust\n").replace(
                "provides:",
                &format!("settings:\n  - key: size\n    label: Size\n    kind: {kind}\nprovides:"),
            )
        };

        assert!(read(&settings("size")).is_ok());
        assert_eq!(refused_at(&settings("text")), "settings[0].kind");

        // A plugin providing no language may call a setting what it likes.
        let elsewhere =
            format!("{MINIMAL}settings:\n  - key: size\n    label: Size\n    kind: text\n");
        assert!(read(&elsewhere).is_ok());
    }

    #[test]
    fn a_language_entry_is_read_in_the_descriptor_grammar() {
        let yaml = providing(
            "    label: Rust
    detect: [Cargo.toml]
    env:
      CARGO_HOME: \"{cache}/cargo\"
    capabilities:
      sccache:
        env:
          RUSTC_WRAPPER: \"{sccache}\"
    stores:
      cargo:
        dir: \"{cache}/cargo\"
        units:
          - under: registry/cache
            depth: 2
",
        );

        let provision = &read(&yaml).unwrap().provides[0];
        let provided = provision
            .language
            .as_ref()
            .expect("a language entry is read");

        assert_eq!(provided.name, "rust");
        assert_eq!(provided.descriptor.label(), Some("Rust"));
        assert!(provided.descriptor.names(crate::languages::SCCACHE));
        assert_eq!(
            provided.descriptor.store_names().collect::<Vec<_>>(),
            ["cargo"]
        );
    }

    #[test]
    fn a_language_entry_with_a_key_a_language_has_not_got_is_refused_naming_it() {
        for (rest, field) in [
            ("    enabled: false\n", "provides[0].enabled"),
            ("    detct: [Cargo.toml]\n", "provides[0].detct"),
            (
                "    capabilities:\n      sccache:\n        environment: {}\n",
                "provides[0].capabilities.sccache.environment",
            ),
            (
                "    stores:\n      cargo:\n        dir: \"{cache}/cargo\"\n        \
                 units:\n          - depht: 2\n",
                "provides[0].stores.cargo.units[0].depht",
            ),
        ] {
            let yaml = providing(rest);

            assert_eq!(refused_at(&yaml), field, "{rest}");
            assert_eq!(
                read(&yaml).unwrap_err().why,
                format!("`{field}` isn't a key a language has.")
            );
        }
    }

    #[test]
    fn a_language_entry_setting_a_variable_the_sandbox_sets_is_refused_naming_it() {
        let yaml = providing("    env:\n      PATH: /usr/bin\n");

        assert_eq!(refused_at(&yaml), "provides[0].env.PATH");

        let yaml = providing(
            "    capabilities:\n      sccache:\n        env:\n          HOME: \"{cache}\"\n",
        );

        assert_eq!(
            refused_at(&yaml),
            "provides[0].capabilities.sccache.env.HOME"
        );
        assert!(
            read(&yaml)
                .unwrap_err()
                .why
                .contains("a variable the sandbox sets itself")
        );
    }

    #[test]
    fn a_language_entry_whose_store_is_not_one_is_refused_naming_the_store() {
        let yaml = providing("    stores:\n      cargo:\n        dir: ~/.cargo\n");

        assert_eq!(refused_at(&yaml), "provides[0].stores.cargo");
    }

    #[test]
    fn a_language_entry_of_the_wrong_shape_is_refused() {
        let yaml = providing("    detect: Cargo.toml\n");

        assert_eq!(refused_at(&yaml), "provides[0]");
    }

    #[test]
    fn a_language_entry_needs_its_name() {
        let yaml = MINIMAL.replace(
            "provides: []",
            "provides:\n  - category: language\n    label: Rust\n",
        );

        assert_eq!(refused_at(&yaml), "provides[0].language");
    }

    #[test]
    fn a_language_provided_twice_is_refused() {
        let yaml = format!(
            "{}  - category: language\n    language: rust\n",
            providing("    label: Rust\n")
        );

        assert_eq!(refused_at(&yaml), "provides[1].language");
    }

    #[test]
    fn a_platforms_own_list_is_read_for_its_languages() {
        let yaml = format!(
            "{}windows:\n  provides:\n    - category: language\n      language: rust\n      \
             label: Rust on Windows\n",
            providing("    label: Rust\n")
        );

        let label = |platform| {
            read_for(&yaml, platform).unwrap().provides[0]
                .language
                .as_ref()
                .unwrap()
                .descriptor
                .label()
                .map(str::to_owned)
        };
        assert_eq!(label(Platform::Windows).as_deref(), Some("Rust on Windows"));
        assert_eq!(label(Platform::Linux).as_deref(), Some("Rust"));
    }

    #[test]
    fn a_capability_of_an_unknown_kind_is_refused() {
        let yaml = format!("{MINIMAL}capabilities:\n  - outbound: gitlab.com\n  - camera: front\n");

        assert_eq!(refused_at(&yaml), "capabilities[1]");
        assert!(read(&yaml).unwrap_err().why.contains("`camera`"));
    }

    #[test]
    fn capabilities_are_read_by_kind() {
        let yaml = format!(
            "{MINIMAL}capabilities:
  - filesystem: ~/.config/glab
  - filesystem: /tmp/glab
    access: read-write
  - outbound: gitlab.com
  - environment: GITLAB_HOST
"
        );

        assert_eq!(
            read(&yaml).unwrap().capabilities,
            [
                Capability::Filesystem {
                    path: "~/.config/glab".to_owned(),
                    access: Access::ReadOnly,
                },
                Capability::Filesystem {
                    path: "/tmp/glab".to_owned(),
                    access: Access::ReadWrite,
                },
                Capability::Outbound {
                    host: "gitlab.com".to_owned(),
                },
                Capability::Environment {
                    name: "GITLAB_HOST".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_settings_field_of_an_unknown_kind_is_refused() {
        let yaml =
            format!("{MINIMAL}settings:\n  - key: host\n    label: Host\n    kind: colour\n");

        assert_eq!(refused_at(&yaml), "settings[0].kind");
    }

    #[test]
    fn an_interface_ahead_of_this_build_is_refused() {
        let yaml = MINIMAL.replace("interface: 1", &format!("interface: {}", INTERFACE + 1));

        assert_eq!(refused_at(&yaml), "interface");
        assert!(read(&yaml).unwrap_err().why.contains("newer Verkstead"));
    }

    #[test]
    fn an_unknown_top_level_key_is_refused() {
        let yaml = format!("{MINIMAL}capabilites: []\n");

        assert_eq!(refused_at(&yaml), "capabilites");
    }

    #[test]
    fn an_unknown_key_inside_an_entry_is_refused() {
        let yaml = format!(
            "{MINIMAL}settings:\n  - key: host\n    label: Host\n    kind: text\n    defualt: x\n"
        );

        assert_eq!(refused_at(&yaml), "settings[0].defualt");
    }

    #[test]
    fn a_windows_override_replaces_the_common_value_only_on_windows() {
        let yaml = format!(
            "{MINIMAL}description: Everywhere.
capabilities:
  - environment: HOME
windows:
  capabilities:
    - environment: USERPROFILE
"
        );

        let windows = read_for(&yaml, Platform::Windows).unwrap();
        let linux = read_for(&yaml, Platform::Linux).unwrap();

        assert_eq!(
            windows.capabilities,
            [Capability::Environment {
                name: "USERPROFILE".to_owned()
            }]
        );
        assert_eq!(
            linux.capabilities,
            [Capability::Environment {
                name: "HOME".to_owned()
            }]
        );
        assert_eq!(windows.description.as_deref(), Some("Everywhere."));
    }

    #[test]
    fn identity_cannot_be_overridden() {
        let yaml = format!("{MINIMAL}macos:\n  uuid: 7c1d2a43-0f5e-4b9a-8c6d-3e2f1a0b9c8d\n");

        assert_eq!(refused_at(&yaml), "macos.uuid");
    }

    #[test]
    fn another_platforms_block_is_still_checked() {
        let yaml = format!("{MINIMAL}windows:\n  capabilites: []\n");

        assert_eq!(
            read_for(&yaml, Platform::Linux)
                .unwrap_err()
                .field
                .as_deref(),
            Some("windows.capabilites")
        );
    }

    #[test]
    fn paths_stay_inside_the_directory() {
        let yaml = format!("{MINIMAL}component: ../elsewhere.wasm\n");

        assert_eq!(refused_at(&yaml), "component");
    }

    #[test]
    fn a_mark_and_a_component_are_read_from_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mark.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        )
        .unwrap();
        std::fs::write(dir.path().join("plugin.wasm"), b"\0asm").unwrap();
        std::fs::write(
            dir.path().join(FILE),
            format!("{MINIMAL}mark: mark.svg\ncomponent: plugin.wasm\n"),
        )
        .unwrap();

        let manifest = Manifest::read(dir.path(), Platform::Linux).unwrap();

        assert_eq!(manifest.mark, Some(Mark::File(PathBuf::from("mark.svg"))));
        assert_eq!(manifest.component, Some(PathBuf::from("plugin.wasm")));
    }

    #[test]
    fn a_mark_that_is_not_svg_is_refused() {
        let yaml = format!("{MINIMAL}mark: <png/>\n");

        assert_eq!(refused_at(&yaml), "mark");
    }

    /// A plugin directory holding `MINIMAL` with one Skill Pack entry, `entry`
    /// written under `provides:`, and each of `skills` as `skills/<name>/SKILL.md`.
    fn packed(entry: &str, skills: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (skill, text) in skills {
            let at = dir.path().join("skills").join(skill);
            std::fs::create_dir_all(&at).unwrap();
            std::fs::write(at.join("SKILL.md"), text).unwrap();
        }
        std::fs::write(
            dir.path().join(FILE),
            MINIMAL.replace(
                "provides: []",
                &format!("provides:\n  - category: skills\n{entry}"),
            ),
        )
        .unwrap();
        dir
    }

    const PACK: &str = "    name: rust-review\n    directory: skills\n";

    const REVIEWING: (&str, &str) = (
        "reviewing",
        "---\nname: reviewing\ndescription: Review Rust.\n---\n\nRead the diff.\n",
    );

    #[test]
    fn a_skill_pack_is_read_with_its_skills() {
        let dir = packed(
            PACK,
            &[
                REVIEWING,
                ("linting", "---\nname: linting\ndescription: Lint.\n---\n"),
            ],
        );
        std::fs::write(dir.path().join("skills/README.md"), "# A pack\n").unwrap();

        let manifest = Manifest::read(dir.path(), Platform::Linux).unwrap();

        assert_eq!(
            manifest.packs,
            [Pack {
                name: "rust-review".to_owned(),
                directory: PathBuf::from("skills"),
                skills: vec!["linting".to_owned(), "reviewing".to_owned()],
            }],
            "a file at the directory's top is no skill",
        );
    }

    #[test]
    fn a_skill_without_frontmatter_is_refused_naming_the_skill() {
        let dir = packed(PACK, &[("reviewing", "# Reviewing\n")]);

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert_eq!(refusal.field.as_deref(), Some("provides[0].directory"));
        assert!(
            refusal
                .why
                .starts_with("The skill `reviewing` has no frontmatter"),
            "{refusal}"
        );
    }

    #[test]
    fn a_skill_whose_name_is_not_its_directorys_is_refused() {
        let dir = packed(
            PACK,
            &[(
                "reviewing",
                "---\nname: review\ndescription: Review.\n---\n",
            )],
        );

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert!(
            refusal.why.contains("`reviewing` is named `review`"),
            "{refusal}"
        );
    }

    #[test]
    fn a_description_holding_a_colon_is_read_as_the_skill_loaders_read_it() {
        let dir = packed(
            PACK,
            &[(
                "reviewing",
                "---\nname: reviewing\ndescription: Review Rust: the diff, then the tests.\n---\n",
            )],
        );

        assert!(Manifest::read(dir.path(), Platform::Linux).is_ok());
    }

    #[test]
    fn a_skill_with_no_description_is_refused() {
        let dir = packed(PACK, &[("reviewing", "---\nname: reviewing\n---\n")]);

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert!(refusal.why.contains("no `description`"), "{refusal}");
    }

    #[test]
    fn a_skill_directory_without_a_skill_md_is_refused() {
        let dir = packed(PACK, &[REVIEWING]);
        std::fs::create_dir_all(dir.path().join("skills/linting")).unwrap();
        // Not `skill.md`, which a case-insensitive filesystem — Windows', a
        // Mac's by default — opens as `SKILL.md`.
        std::fs::write(dir.path().join("skills/linting/README.md"), "x").unwrap();

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert_eq!(refusal.why, "The skill `linting` has no `SKILL.md`.");
    }

    #[test]
    fn a_pack_named_verkstead_is_refused_but_for_the_built_in() {
        let entry = "    name: verkstead\n    directory: skills\n";
        let dir = packed(entry, &[REVIEWING]);

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();
        assert_eq!(refusal.field.as_deref(), Some("provides[0].name"));

        let text = std::fs::read_to_string(dir.path().join(FILE))
            .unwrap()
            .replace(
                "0b6f6b7e-5c3a-4d0e-9a51-2f7d8c1e4a90",
                super::super::built_in::VERKSTEAD,
            );
        Manifest::parse(&text, Files::Dir(dir.path()), Platform::Linux)
            .expect("the name is the built-in's own");
    }

    #[test]
    fn a_pack_name_is_lowercase_and_hyphens() {
        let dir = packed(
            "    name: Rust Review\n    directory: skills\n",
            &[REVIEWING],
        );

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert_eq!(refusal.field.as_deref(), Some("provides[0].name"));
    }

    #[test]
    fn a_pack_entry_with_an_unknown_key_is_refused() {
        let dir = packed(&format!("{PACK}    skils: x\n"), &[REVIEWING]);

        assert_eq!(
            Manifest::read(dir.path(), Platform::Linux)
                .unwrap_err()
                .field
                .as_deref(),
            Some("provides[0].skils")
        );
    }

    #[test]
    fn a_pack_whose_directory_is_not_there_is_refused() {
        let dir = packed(
            "    name: rust-review\n    directory: elsewhere\n",
            &[REVIEWING],
        );

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert_eq!(refusal.field.as_deref(), Some("provides[0].directory"));
    }

    #[test]
    fn a_directory_with_no_manifest_is_not_a_plugin() {
        let dir = tempfile::tempdir().unwrap();

        let refusal = Manifest::read(dir.path(), Platform::Linux).unwrap_err();

        assert_eq!(refusal.field, None);
        assert!(refusal.why.contains("isn't a plugin"));
    }
}
