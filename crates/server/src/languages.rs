//! A language is a **descriptor**: data, in one grammar, whether Verkstead
//! shipped it or an installer wrote it (ADR-0021).
//!
//! What a session gets of a language used to be Rust's name in three places —
//! a module, a settings key and a pane — which is the right shape for one
//! language and the wrong one for seven. The people this is for install
//! Verkstead themselves and build in languages its maintainer does not, so
//! adding one has to be a file rather than a build. The descriptors Verkstead
//! ships are therefore a YAML file embedded in the binary, in the grammar an
//! installer writes — `crates/server/languages.yaml`, which is this module's
//! documentation as well as its data.
//!
//! **And what an installer writes is merged over them, key by key.** The map
//! under `languages:` in `config.yaml` is read as a [`Languages`] of its own
//! and written over the built-ins — see [`configured`], which is what every
//! session is actually built from, and [`Languages::merged`] for what merging
//! one entry into another comes to. Key by key rather than an entry replacing
//! a built-in whole, so that changing one variable still gets every later fix
//! to the rest; a variable set to `null` is taken out; and an entry naming a
//! language nothing here has ever heard of is a descriptor in its own right,
//! on by default, with its variables in the next session and its checkbox on
//! the settings page.
//!
//! **`enabled` and `size` are keys of the same entry**, beside the ones only a
//! file ever holds, and they are the only two the settings page writes — which
//! is what lets one request write the whole of `config.yaml` without a save
//! taking an installer's descriptor away. `rust_build_cache`, which is where
//! Rust's switch and size used to be said, is still read as Rust's two, and
//! the map wins where both are written: see
//! [`crate::settings::Config::languages`], where the two are put together.
//!
//! **A descriptor cannot say a command to run.** It says a label, the manifests
//! that detect the language in a Repo, the variables a session is given, and
//! the **capabilities** it names — and a capability is behaviour built into the
//! server, switched on by name. Rust's names [`SCCACHE`], which is the Compile
//! Server; the Rust half of that is [`crate::build_cache`] and stays exactly
//! what it was. A `config.yaml` that could name a program to start would be a
//! settings file that starts programs, in a sandbox somebody would then have to
//! describe in YAML too.
//!
//! **Variables are every session's**, whatever the Repo holds, for the reason
//! Rust's are today: a manifest is often not at the root of a checkout, and a
//! variable nothing reads costs nothing. [`Descriptor::detected`] is for the
//! setup card's warning alone — see
//! [`crate::build_cache::repo_builds_through_sccache`], which is its one caller.
//!
//! **Placeholders are what only the server knows** — see [`Machine`], and the
//! embedded file, which is where each of the four is spelled out. A
//! placeholder's directory is granted to a session only where a loaded
//! descriptor names it: the one beside the Worktrees is where pnpm's, deno's,
//! bun's and uv's stores are, and an installation with every language that
//! names it switched off is opened onto none.
//!
//! **And an entry that will not load falls back to the built-in of that name.**
//! Two ways one fails and one answer to both: an entry naming a variable the
//! Sandbox sets itself is refused — see [`crate::sandbox::sets_itself`], which
//! is the union of all three platforms' names — and an entry nothing can parse
//! is refused the same way. **A key nothing here knows is the second of those**
//! rather than a key quietly skipped: `detct` is the likeliest thing to be
//! wrong with a hand-written descriptor, and a language that silently did
//! nothing about it would be the one mistake this whole arrangement never told
//! anybody about. Either way what is left is the descriptor Verkstead
//! ships, which is the cache the installer already had; a language with no
//! built-in behind it goes **off** rather than on at nothing; every other
//! language loads; and the server comes up. Losing Rust's build cache to one
//! mistyped variable is the worse experience for not having checked, which is
//! the same reason the merge is key by key in the first place.
//!
//! **What was written stays written.** The entry is kept exactly as the file
//! had it — see [`Raw`] — because a save from the settings page writes the
//! whole of `config.yaml`, and an entry nothing could read is still text the
//! installer typed and is about to fix. It goes back the way the descriptor
//! keys the page never drew do. And the page says why: [`Descriptor::unread`]
//! is the reason, in the words it is logged in.

use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

/// The descriptors Verkstead ships, as an installer would have written them.
///
/// Embedded rather than read off the disk, for the reason the Guide is: what
/// runs is exactly what was reviewed, and one grammar means the built-ins are
/// the documentation's worked examples rather than a second shape nobody sees.
const BUILT_IN_YAML: &str = include_str!("../languages.yaml");

/// Rust, whose name `rust_build_cache` is still read as the settings of.
pub const RUST: &str = "rust";

/// And Go, the first of the package stores — two directories and no
/// capability, which is the whole of what a language costs the server now.
pub const GO: &str = "go";

/// And Node, which is one entry for every tool that installs out of the npm
/// registry — six of them — and the first built-in to name the directory beside
/// the Worktrees: pnpm, deno and bun all hardlink out of their store into the
/// project.
pub const NODE: &str = "node";

/// And Python, which is one entry for pip and uv — and for poetry and pipenv
/// after them, an ecosystem being one box on the settings page. uv is the
/// fourth store beside the Worktrees, and the second tool to say out loud that
/// it could not link out of one.
pub const PYTHON: &str = "python";

/// And .NET, which is one entry for one tool — everything on a .NET machine
/// installs through NuGet — and the only built-in whose `detect` is empty: a
/// project is a `*.csproj` or a `*.sln`, and detection matches filenames rather
/// than globs. Three variables, the third of them the directory NuGet locks
/// its store in: two sessions with a `/tmp` each took a lock apiece and wrote
/// over one another.
pub const DOTNET: &str = "dotnet";

/// And C/C++, the second language naming [`SCCACHE`]: CMake's two launcher
/// variables pointed at the one Compile Server Rust's `RUSTC_WRAPPER` is, and
/// no store of its own — one server, one store and one size for the machine.
pub const CPP: &str = "cpp";

/// And the JVM, which is one entry for Maven and Gradle. Maven's local
/// repository has no variable of its own, so its value is a line of JVM
/// options with the path inside it, and the file locks two sessions need to
/// write one repository at once beside it.
pub const JVM: &str = "jvm";

/// The one capability this server has: the **Compile Server**, which is one
/// sccache server for the machine in a sandbox of its own — see
/// [`crate::build_cache::BuildCache::compiling`].
///
/// A descriptor naming it says two things at once: that its variables want the
/// `{sccache}` placeholder filled, and that a session of that language is one
/// the server should be up for.
pub const SCCACHE: &str = "sccache";

/// The Build Cache, which is where a store goes that nothing has to share a
/// filesystem with.
const CACHE: &str = "{cache}";

/// And a directory beside the Worktrees, for a store that does — see
/// [`Machine::of`].
const STORES: &str = "{stores}";

/// How big this language's store may grow, which is the settings page's.
const SIZE: &str = "{size}";

/// The directory of Verkstead's own, at the top of each placeholder's, that a
/// unit is moved aside into before it is deleted — see [`Machine::aside`].
/// Never a store, nor inside one: [`placed`] refuses a directory under it.
const ASIDE: &str = ".verkstead-swept";

/// How big a language's store may grow where nobody has said, for every
/// language but Rust — an installer's own included.
///
/// A release's rather than a machine's, which is why it is a constant here and
/// not a key of the embedded file: a key there would be one more an installer
/// could write, meaning nothing a `size` does not already say. Rust's is
/// [`crate::build_cache::SIZE`], which its sccache has always been started at
/// — see [`default_size`].
pub const DEFAULT_SIZE: &str = "10G";

/// The size `name`'s store may grow to where nobody has said.
///
/// Rust keeps the `30G` its compiled half was always given, because a few Rust
/// workspaces fill the `10G` every other store starts at. Every other language
/// gets [`DEFAULT_SIZE`], an installer's own included.
pub fn default_size(name: &str) -> &'static str {
    match name {
        RUST => crate::build_cache::SIZE,
        _ => DEFAULT_SIZE,
    }
}

/// How many bytes `size` is, read the way sccache reads `SCCACHE_CACHE_SIZE` —
/// or, where it cannot be, why not, as a clause that follows the word.
///
/// **sccache's grammar, so that one word means the same to both.** A whole
/// number, then one of `K`, `M`, `G` or `T` for binary multiples of a byte, or
/// nothing for bytes themselves: `30G` is 30 × 2³⁰. Upper case only and no
/// fractions, because sccache takes neither — a `1.5g` Verkstead read would be
/// a size sccache quietly did not, and the Compile Server would be bounded by
/// something nobody typed.
///
/// What it is for is the sweep, which compares a store's bytes against it.
/// What a self-evicting tool is handed is still the human's own word — see
/// [`Descriptor::size`] — and this is what decides that the word is a size at
/// all.
pub fn bytes(size: &str) -> Result<u64, String> {
    let (number, multiplier) = match size.chars().last() {
        Some('K') => (&size[..size.len() - 1], 1u64 << 10),
        Some('M') => (&size[..size.len() - 1], 1 << 20),
        Some('G') => (&size[..size.len() - 1], 1 << 30),
        Some('T') => (&size[..size.len() - 1], 1 << 40),
        _ => (size, 1),
    };

    number
        .parse::<u64>()
        .ok()
        .filter(|_| number.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|number| number.checked_mul(multiplier))
        .ok_or_else(|| {
            format!(
                "is not a size: a size is a whole number with K, M, G or T after it, such as \
                 {DEFAULT_SIZE}"
            )
        })
}

/// And where a session reaches the sccache this server found, which is the
/// [`SCCACHE`] capability's own.
const SCCACHE_AT: &str = "{sccache}";

/// The descriptors Verkstead ships, parsed once.
///
/// A panic because the file is embedded: it cannot vary between machines, so a
/// file that will not parse is a build that should never have shipped, and the
/// suite reads it — see `the_built_ins_are_in_the_grammar_an_installer_writes`.
///
/// **Both halves of that are checked here**, because only one of them is an
/// error: an entry of an installer's that will not load is a language falling
/// back to its built-in, and an entry *of the built-ins* that will not load has
/// nothing behind it to fall back to. So the leniency [`Languages::read`] gives
/// `config.yaml` is spent here on a clearer panic rather than on shipping a
/// language that quietly does nothing.
pub fn built_in() -> &'static Languages {
    static BUILT_IN: LazyLock<Languages> = LazyLock::new(|| {
        let languages = Languages::read(BUILT_IN_YAML)
            .expect("the descriptors embedded in this binary are in the grammar the loader reads");

        if let Some((name, why)) = languages
            .iter()
            .find_map(|(name, descriptor)| Some((name, descriptor.unread()?)))
        {
            panic!("the descriptor embedded for {name} {why}");
        }

        languages
    });

    &BUILT_IN
}

/// The descriptors this installation has: the ones Verkstead ships, with what
/// `config.yaml` says merged over them.
///
/// Built at the moment it is asked for rather than held from startup, like
/// everything else a session is made out of — see
/// [`crate::settings::Config::languages`]. A language switched off from a phone
/// applies to the next session, and so does a descriptor a hand-edit added an
/// hour ago.
pub fn configured(config: &crate::settings::Config) -> Languages {
    built_in().merged(&config.languages())
}

/// A map of descriptors keyed by language, in the order they were written.
///
/// One type for both halves of the grammar — the file embedded in the binary
/// and the map under `languages:` in `config.yaml` — because they are one
/// grammar: what an installer writes is what Verkstead ships, and a second
/// shape for the overrides would be a second thing to document and a second
/// thing to keep in step.
///
/// Order is kept rather than sorted because it is what a session's environment
/// is built in, and an environment that reordered itself between releases would
/// be one nothing could be asserted about byte for byte.
///
/// Read and written by hand rather than derived, because an entry that will not
/// load is neither an error nor a loss: it is read as a [`Descriptor`] saying
/// nothing with the reason and the text-as-written on it, and it is written
/// back out as that text — see [`Descriptor::read`], and this module's header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Languages(Ordered<Descriptor>);

/// A whole descriptor file: the map under its one `languages:` key.
///
/// The embedded file is written with that key so that it is the fragment of
/// `config.yaml` an installer would paste, rather than a shape of its own —
/// see [`Languages::read`], which is the only thing that goes through this.
#[derive(Debug, Deserialize)]
struct File {
    #[serde(default)]
    languages: Languages,
}

impl Languages {
    /// What `text` says, or what went wrong reading it.
    pub fn read(text: &str) -> Result<Languages, serde_saphyr::Error> {
        serde_saphyr::from_str::<File>(text).map(|file| file.languages)
    }

    /// Rust's entry as `rust_build_cache` says it, which is what an install
    /// that wrote one keeps — see [`crate::settings::Config::languages`].
    ///
    /// Only what is actually written down: two `None`s are an entry saying
    /// nothing, which merges into the built-in without changing it. What that
    /// buys is the whole of the compatibility rule in one place — the old key
    /// is a descriptor merged *under* the new map, so the map wins wherever
    /// both are written, and nothing downstream has to ask which of the two a
    /// value came from.
    pub fn of_rust(enabled: Option<bool>, size: Option<String>) -> Languages {
        Languages(Ordered(vec![(
            RUST.to_owned(),
            Descriptor {
                enabled,
                size,
                ..Descriptor::default()
            },
        )]))
    }

    /// And the entries the settings page has just written: the two keys it
    /// draws, per language it was given.
    ///
    /// Written over the file rather than replacing it — see
    /// [`Languages::under_the_page`], which is what a save goes through — so
    /// what an installer wrote beside those two keys survives a save. A size
    /// that is blank is *nothing configured* rather than a size of nothing,
    /// which is what clearing the field on the page means and what puts the
    /// default back; a language with no size field at all sends that same
    /// blank, there being no size to send.
    pub fn of_page(written: impl IntoIterator<Item = (String, bool, String)>) -> Languages {
        Languages(Ordered(
            written
                .into_iter()
                .map(|(name, enabled, size)| {
                    (
                        name,
                        Descriptor {
                            enabled: Some(enabled),
                            size: crate::settings::blank_is_nothing(size),
                            ..Descriptor::default()
                        },
                    )
                })
                .collect(),
        ))
    }

    /// Whether there is nothing here at all, which is what a `config.yaml`
    /// nobody has written a descriptor in says.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `over` merged into this, entry by entry and then key by key within an
    /// entry — see [`Descriptor::merged`].
    ///
    /// An entry naming a language that is not here is a descriptor of its own,
    /// taken on as it stands: that is how an installer adds a language
    /// Verkstead has never heard of. An entry naming one that *is* here changes
    /// the keys it gives and leaves the rest as the built-in says, so an
    /// override of one variable still gets every later fix to the others —
    /// which is the whole reason the merge is key by key rather than an entry
    /// replacing a built-in whole (ADR-0021).
    ///
    /// And an entry that would not load is a descriptor saying nothing, so what
    /// comes out of the first case is the built-in exactly as it stands — see
    /// [`Descriptor::read`]. The second case is where the reason is spent: a
    /// language with nothing behind it goes off — see [`Descriptor::alone`].
    pub fn merged(&self, over: &Languages) -> Languages {
        Languages(
            self.0
                .merged(&over.0, Descriptor::merged, Descriptor::alone),
        )
    }

    /// And this with what the settings page has just sent written over it.
    ///
    /// Not [`Languages::merged`], which is one grammar's entries over
    /// another's. That one keeps a key the override does not give, because a
    /// key an installer did not write is a key they said nothing about — and
    /// the page is the other way round: it draws `enabled` and `size` every
    /// time it saves, so a key it did not send is one somebody **cleared**.
    /// Clearing the size field is how the default is asked for back, and a
    /// merge that kept the old value would be a field that could not be
    /// emptied.
    ///
    /// Everything else in an entry is kept from this one, which is what a save
    /// from a page with no descriptor editor on it must not be able to touch —
    /// see [`crate::settings::Config::keeping_what_the_page_never_drew`], the
    /// one caller. An entry the page never sent stays as it stands, and one
    /// naming a language this file has never heard of is taken on as written,
    /// which is what saves an installer's own.
    ///
    /// An entry nothing could read keeps its text here like any other key the
    /// page never drew, and goes back into the file as it was written: it is
    /// still what the installer typed and is about to fix — see [`Languages`]'s
    /// own serialisation, which is where that is done.
    pub fn under_the_page(&self, page: &Languages) -> Languages {
        Languages(
            self.0
                .merged(&page.0, Descriptor::with_the_pages_keys, Descriptor::clone),
        )
    }

    /// The descriptor of `name`, where there is one.
    pub fn get(&self, name: &str) -> Option<&Descriptor> {
        self.0.get(name)
    }

    /// Every one of them, in the order they were written.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Descriptor)> {
        self.0.iter()
    }

    /// What every switched-on descriptor gives one session: the variables, and
    /// the directories that have to be open for them to mean anything.
    ///
    /// The switch and the size are the descriptor's own — `enabled` and `size`,
    /// which are the two keys of an entry the settings page writes and the only
    /// two it ever does. They are read at every session spawn rather than held
    /// from startup, like everything else a session is built from, so a switch
    /// flipped on a phone applies to the next session.
    pub fn given(&self, machine: &Machine) -> Given {
        let mut given = Given::default();

        for (name, descriptor) in self.iter() {
            if !descriptor.enabled() {
                continue;
            }

            let size = descriptor.size(name);

            given.taking(name, machine, size, descriptor.env.iter());

            for (capability, entry) in descriptor.capabilities.iter() {
                // A capability this server does not have is a descriptor asking
                // for behaviour rather than for variables, so its whole entry
                // is left out — which is what makes a machine with no sccache
                // installed a session with the downloads shared and nothing
                // pointed at a compile server that is not there.
                if !machine.offers(capability) {
                    tracing::debug!(
                        language = name,
                        capability,
                        "this server cannot offer the capability a language names, so its \
                         variables were left out of the session",
                    );

                    continue;
                }

                if capability == SCCACHE {
                    given.sccache = true;
                }

                given.taking(name, machine, size, entry.env.iter());
            }
        }

        given
    }

    /// The store size the behaviour behind `capability` is started at, and
    /// `None` where no switched-on descriptor names it — which is what says
    /// whether that behaviour is wanted on this machine at all.
    ///
    /// The size comes back with the answer because the one capability there is
    /// wants one: the Compile Server is started with a size, and that size is a
    /// language's rather than a number the server holds. Where two of them name
    /// it — C/C++ beside Rust — **the first written sizes the store whether or
    /// not it is switched on**, because there is one store and one server for
    /// the machine. Were it the first switched-*on* one instead, switching Rust
    /// off would restart the server at C/C++'s size — the default, where nobody
    /// had set one — and sccache would trim the store down to it as it started.
    ///
    /// Asked of the switch rather than of a Repo. The Compile Server comes up
    /// wherever a language naming [`SCCACHE`] is enabled and there is an sccache
    /// to run, whatever the checkout holds: a Repo whose manifest is not at its
    /// root is handed the wrapper variable all the same, and with no server of
    /// Verkstead's up the client inside starts one in its own Sandbox — which
    /// is the hazard the Compile Server exists to remove.
    pub fn wanting(&self, capability: &str) -> Option<&str> {
        let naming = || {
            self.iter()
                .filter(|(_, descriptor)| descriptor.names(capability))
        };

        let (name, sizer) = naming().next()?;

        naming()
            .any(|(_, descriptor)| descriptor.enabled())
            .then(|| sizer.size(name))
    }

    /// Whether any descriptor loaded here names the directory beside the
    /// Worktrees, which is what says there is one to make.
    ///
    /// Asked of every descriptor rather than of the switched-on ones, because
    /// making a directory is startup's and the switch is a session's: a language
    /// turned off this morning is one turned on again this afternoon without
    /// the server being restarted.
    pub fn names_stores(&self) -> bool {
        self.iter().any(|(_, descriptor)| {
            descriptor
                .env
                .iter()
                .chain(
                    descriptor
                        .capabilities
                        .iter()
                        .flat_map(|(_, entry)| entry.env.iter()),
                )
                .any(|(_, value)| value.as_deref().is_some_and(|value| value.contains(STORES)))
        })
    }
}

/// One language, as the file says it.
///
/// Every field optional, because every one of them is a thing a language may
/// not have: a store with no manifest to detect it by, a language whose whole
/// descriptor is two variables, one that is nothing but a capability.
///
/// **`enabled` and `size` are keys of this same entry**, beside the ones only
/// a file ever holds, and they are the only two the settings page ever writes.
/// One entry per language rather than a switch in one place and a descriptor in
/// another, because a human reading `config.yaml` should find everything it
/// says about a language in the one block under its name.
///
/// Written away when it is absent, every field of it, so that a save from the
/// settings page rewrites `config.yaml` with the keys the human put there and
/// no others — see [`crate::settings::Config::keeping_what_the_page_never_drew`].
///
/// **A key that is not one of these is refused**, which makes a misspelled one
/// an entry that falls back to the built-in with a reason naming the key — see
/// [`Descriptor::read`]. Skipped instead, it would be the one way of writing a
/// descriptor wrong that nothing anywhere mentioned: no variable set, no
/// warning, and a language that looks configured and is not.
///
/// What it costs is a descriptor written for a Verkstead newer than this one,
/// which falls back rather than loading the keys this release does understand.
/// Deliberately the other way round from a **capability** name, which is
/// tolerated so that such a descriptor still gets its variables — see
/// [`Machine::offers`]. The difference is what each one is: a key nobody knows
/// is a descriptor this release cannot honour the shape of, and a capability
/// nobody offers is behaviour this release has not got, which is the ordinary
/// answer on a machine with no sccache either.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    detect: Vec<String>,

    /// The variables, where a `None` is a `null` in the file: a variable taken
    /// **out** rather than set to nothing — see [`Descriptor::merged`], and
    /// [`Given::taking`], which is what leaves it out of a session.
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    env: Ordered<Option<String>>,

    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    capabilities: Ordered<Capability>,

    /// The directories that are this language's **store**, by a name of the
    /// descriptor's own — what is measured for the settings page's disk use,
    /// and what is bounded by the language's size.
    ///
    /// Named here rather than read back out of the variables above, because a
    /// variable is not always a path: Maven's repository is a flag inside the
    /// line of JVM options in `MAVEN_OPTS`. A `None` is a `null` in the file,
    /// which takes a store out the way a `null` takes out a variable — see
    /// [`Descriptor::merged`].
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    stores: Ordered<Option<Store>>,

    /// And why nothing above was taken from the file, where this entry would
    /// not load — with what the file said, so it can go back as written.
    ///
    /// Never read off a file and never written to one: it is what the loader
    /// makes of an entry it refused, and every field above it is left at its
    /// default in that case, so that the entry merges into the built-in without
    /// changing a thing — see [`Descriptor::read`].
    #[serde(skip)]
    unread: Option<Box<Unread>>,
}

/// Why an entry in `config.yaml` was not used, and what it said.
///
/// The text is kept because a save from the settings page writes the whole of
/// the file: an entry nothing could read is still what the installer typed and
/// is about to fix, and a save that quietly dropped it would be the one outcome
/// worse than not loading it.
#[derive(Debug, Clone, PartialEq)]
struct Unread {
    /// In the words the page says and the log carries, as a clause that follows
    /// *its entry in `config.yaml`* — see [`Descriptor::read`].
    why: String,

    /// And the entry as the file had it.
    written: Raw,
}

impl Descriptor {
    /// One entry of `config.yaml`, as written — or, where it will not load, a
    /// descriptor saying **nothing**, carrying the reason and the text.
    ///
    /// Two ways it fails and one answer to both: an entry naming a variable the
    /// Sandbox sets itself, and an entry nothing can parse — a key this grammar
    /// does not have being the commonest of the second sort, because
    /// [`Descriptor`] refuses one. A descriptor saying
    /// nothing is what makes the answer the same in both cases and in every
    /// place downstream — it merges into the built-in of that name without
    /// changing a key of it, which is that language running on exactly what it
    /// ran on before the file was written. What it does *not* blank is
    /// `rust_build_cache`, which is a key of its own, read into Rust's entry
    /// underneath this one and never part of what was refused.
    ///
    /// An entry with nothing under it at all is an entry saying nothing on
    /// purpose, and is read as one rather than refused: `rust:` and no keys is
    /// a line somebody wrote to have somewhere to add a key.
    ///
    /// The reason is a clause, so that whoever draws it can put the language in
    /// front of it — see [`crate::ui`], and the pane it reaches.
    fn read(written: Raw) -> Descriptor {
        let why = match Descriptor::parsed(&written) {
            Ok(descriptor) => match (descriptor.refuses(), descriptor.misplaced()) {
                (Some(name), _) => {
                    format!("sets {name}, which is a variable the Sandbox sets itself")
                }
                (None, Some(why)) => why,
                (None, None) => return descriptor,
            },
            Err(why) => why,
        };

        Descriptor {
            unread: Some(Box::new(Unread { why, written })),
            ..Descriptor::default()
        }
    }

    /// What `written` says, read as a descriptor — or why it could not be.
    ///
    /// Through the grammar's own text rather than through a second reader over
    /// the value: what is wanted is the answer `serde_saphyr` would have given
    /// the whole file, entry by entry, and the way to get exactly that answer is
    /// to ask it. The cost is one small document written and read per entry of
    /// a file that holds a handful, at the moment a session is spawned.
    fn parsed(written: &Raw) -> Result<Descriptor, String> {
        if matches!(written, Raw::Nothing) {
            return Ok(Descriptor::default());
        }

        let text =
            serde_saphyr::to_string(written).map_err(|why| format!("could not be read: {why}"))?;

        serde_saphyr::from_str::<Descriptor>(&text)
            .map_err(|why| format!("could not be read: {why}"))
    }

    /// The first variable this entry names that the Sandbox sets itself, where
    /// it names one — see [`crate::sandbox::sets_itself`].
    ///
    /// A capability's variables are asked the same question as the descriptor's
    /// own, because they reach the same environment. And a name set to `null`
    /// is refused like any other: taking `PATH` out of a session breaks it
    /// exactly as thoroughly as replacing it.
    fn refuses(&self) -> Option<&str> {
        self.env
            .iter()
            .chain(
                self.capabilities
                    .iter()
                    .flat_map(|(_, entry)| entry.env.iter()),
            )
            .map(|(name, _)| name)
            .find(|name| crate::sandbox::sets_itself(name))
    }

    /// Why a store this entry names is not one, where one is not — a clause, in
    /// the words [`Descriptor::read`] gives the page.
    ///
    /// **A store is a directory under `{cache}` or `{stores}`, and nothing
    /// else**, because what is done to a store is delete what is in it: a store
    /// named anywhere else is a sweep reaching into somebody's home. So the
    /// path is checked as it is written, before any machine fills it in — see
    /// [`placed`].
    ///
    /// **And a store says how it is bounded in one way that can be followed**,
    /// for the same reason: a unit that would match anything is a sweep taking
    /// the store apart a file at a time — see [`Store::unbounded`].
    fn misplaced(&self) -> Option<String> {
        self.stores.iter().find_map(|(name, store)| {
            let store = store.as_ref()?;

            if let Some(why) = store.unbounded() {
                return Some(format!("names its store {name} {why}"));
            }

            let dir = store.dir.as_deref()?;

            placed(dir)
                .err()
                .map(|why| format!("names {dir} as its store {name}, {why}"))
        })
    }

    /// Why this language's entry in `config.yaml` was not used, where it was
    /// not — a clause, in the words the page says.
    pub fn unread(&self) -> Option<&str> {
        self.unread.as_ref().map(|unread| unread.why.as_str())
    }

    /// An override entry with no built-in of that name behind it, which is
    /// ordinarily a descriptor in its own right — an installer's own language,
    /// taken on as it stands.
    ///
    /// Except where nothing could read it, and then it is a language **off**:
    /// what a descriptor saying nothing comes to with nothing underneath it is
    /// a name with no variables, and a box on the settings page that turns on a
    /// language that does not exist would be worse than the language being
    /// plainly off until the entry is fixed.
    fn alone(&self) -> Descriptor {
        match self.unread.is_some() {
            true => Descriptor {
                enabled: Some(false),
                ..self.clone()
            },
            false => self.clone(),
        }
    }

    /// What the settings page calls this language, where the file said.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Whether sessions get this language at all. Nothing written down is
    /// **on**, which is the rule every default in the settings follows: a human
    /// should never have a worse experience for not having been to the settings
    /// page, and a language that came switched off would be one nobody knew to
    /// turn on.
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// And how big this language's store may grow, which is the `{size}`
    /// placeholder's value — [`default_size`] of `name`, which is what this
    /// descriptor is keyed by, where nobody has said.
    ///
    /// The human's own word rather than a number of bytes, because what it
    /// reaches is a tool that reads it in the same grammar — see [`bytes`]. And
    /// the default as well where the word written down is not a size at all,
    /// which only a hand-edit of `config.yaml` can do, the settings page
    /// refusing one: a word sccache could not read would start the Compile
    /// Server at sccache's own default instead, and a store the sweep could not
    /// measure against anything would be one nothing ever bounded. The pane
    /// says why — see [`Descriptor::size_unread`].
    pub fn size(&self, name: &str) -> &str {
        match self.size.as_deref() {
            Some(size) if bytes(size).is_ok() => size,
            _ => default_size(name),
        }
    }

    /// Why the size written down for this language is not the one it runs at,
    /// where it is not — a clause that follows the word, in the words a save
    /// that sent it would have been refused in.
    pub fn size_unread(&self) -> Option<String> {
        bytes(self.size.as_deref()?).err()
    }

    /// The size exactly as it is written down, and `None` where nobody has
    /// written one: what the settings page draws as a placeholder rather than
    /// as a value somebody chose.
    pub fn size_configured(&self) -> Option<&str> {
        self.size.as_deref()
    }

    /// Whether this language has a store of its own: a variable naming the
    /// Build Cache or the directory beside the Worktrees, the descriptor's own
    /// or a capability's.
    ///
    /// What says the settings page has a size to draw under its box. C/C++ has
    /// none — its whole descriptor is the Compile Server's two launcher
    /// variables, and that server's store is sized by Rust's entry — see
    /// [`Languages::wanting`].
    pub fn has_store(&self) -> bool {
        self.names_a_store()
            || self
                .env
                .iter()
                .chain(
                    self.capabilities
                        .iter()
                        .flat_map(|(_, entry)| entry.env.iter()),
                )
                .any(|(_, value)| {
                    value
                        .as_deref()
                        .is_some_and(|value| value.contains(CACHE) || value.contains(STORES))
                })
    }

    /// Whether this descriptor names a store directory at all — which is what
    /// says there is anything to measure on the settings page. An installer's
    /// descriptor naming none is still a descriptor: its variables are given
    /// all the same, and the page says nothing is measured.
    pub fn names_a_store(&self) -> bool {
        self.store_dirs().next().is_some()
    }

    /// Every directory this language names as its store, as written, by the
    /// name it is keyed under — a `null`ed one and one with no `dir` left out.
    fn store_dirs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.stores
            .iter()
            .filter_map(|(name, store)| Some((name, store.as_ref()?.dir.as_deref()?)))
    }

    /// The names this language's store directories are keyed under, in the
    /// order they were written — what the settings page lists them by.
    pub fn store_names(&self) -> impl Iterator<Item = &str> {
        self.store_dirs().map(|(name, _)| name)
    }

    /// And those directories on `machine`, keyed the same way, in the order
    /// they were written. A directory [`Descriptor::misplaced`] would refuse is
    /// left out, though a descriptor holding one never loads.
    pub fn stores(&self, machine: &Machine) -> Vec<(String, PathBuf)> {
        self.store_dirs()
            .filter_map(|(name, dir)| Some((name.to_owned(), machine.store(dir)?)))
            .collect()
    }

    /// How the store this descriptor names `store` is kept under its size —
    /// [`Bounded::NotAtAll`] for one it does not name, which nothing sweeps
    /// either.
    pub fn bounded(&self, store: &str) -> Bounded<'_> {
        let Some(Some(named)) = self
            .stores
            .iter()
            .find(|(name, _)| *name == store)
            .map(|(_, store)| store)
        else {
            return Bounded::NotAtAll;
        };

        match (named.evicted, named.units.as_deref()) {
            (Some(Evicted::ByItsTool), _) => Bounded::ByItsTool,
            (None, Some(units)) if !units.is_empty() => Bounded::ByUnit(units),
            _ => Bounded::NotAtAll,
        }
    }

    /// Whether this descriptor names `capability`, whatever this machine can
    /// offer — which is what [`Languages::wanting`] asks of a switched-on one,
    /// and what says the settings page has a Compile Server to talk about.
    pub fn names(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|(named, _)| named == capability)
    }

    /// `over` written over this, key by key.
    ///
    /// A key `over` does not give is this one's as it stands, which is what
    /// lets an installer change one variable and still get every later fix to
    /// the rest. `detect` is the one list here and a list is replaced whole:
    /// half a set of manifests is not a set of manifests, and there is nothing
    /// a `null` could mean in the middle of one.
    fn merged(&self, over: &Descriptor) -> Descriptor {
        Descriptor {
            enabled: over.enabled.or(self.enabled),
            size: over.size.clone().or_else(|| self.size.clone()),
            label: over.label.clone().or_else(|| self.label.clone()),
            detect: match over.detect.is_empty() {
                true => self.detect.clone(),
                false => over.detect.clone(),
            },
            // A variable in both is the override's, `null` included: what a
            // `null` leaves behind is an entry with no value, which is a
            // variable no session is given.
            env: self
                .env
                .merged(&over.env, |_, over| over.clone(), Clone::clone),
            // And a capability in both is merged in its turn rather than
            // replaced, so that overriding one of the sccache's three variables
            // keeps the other two.
            capabilities: self.capabilities.merged(
                &over.capabilities,
                Capability::merged,
                Capability::clone,
            ),
            // And a store in both merged in its turn, a `null` taking it out —
            // the variables' rule, one level further down.
            stores: self.stores.merged(
                &over.stores,
                |held, over| match (held, over) {
                    (Some(held), Some(over)) => Some(held.merged(over)),
                    (_, over) => over.clone(),
                },
                Clone::clone,
            ),
            // And why the override said nothing, where it said nothing because
            // it would not load. Carried rather than merged into: what it is
            // about is the entry in the file, and the built-in underneath it is
            // exactly what it is falling back *to*.
            unread: over.unread.clone().or_else(|| self.unread.clone()),
        }
    }

    /// This with the two keys the settings page draws taken from `page`,
    /// absence included, and every other key left exactly as it is — see
    /// [`Languages::under_the_page`], which says why that is not a merge.
    fn with_the_pages_keys(&self, page: &Descriptor) -> Descriptor {
        Descriptor {
            enabled: page.enabled,
            size: page.size.clone(),
            ..self.clone()
        }
    }

    /// Whether a Repo at `path` is one a session would build this language in:
    /// one of the manifests at its root.
    ///
    /// Asked of the checkout rather than remembered against the Repo, because it
    /// is a fact about what is on disk now and a repository gains and loses a
    /// manifest like any other file.
    ///
    /// **The setup card's warning and nothing else reads this.** What a session
    /// is given does not turn on it, and neither does the Compile Server — see
    /// [`Languages::wanting`].
    pub fn detected(&self, path: &Path) -> bool {
        self.detect
            .iter()
            .any(|manifest| path.join(manifest).is_file())
    }
}

/// A capability's own half of a descriptor: the variables that are set only
/// where this server can offer it.
///
/// The names are the descriptor's and the answer is the server's, which is what
/// keeps a capability from being one language's. Rust's sccache is a
/// `RUSTC_WRAPPER`; C/C++'s is CMake's two launcher variables
/// pointed at the same binary — one capability, two descriptors, and nothing in
/// the server that knows which language asked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    env: Ordered<Option<String>>,
}

impl Capability {
    /// `over` written over this, which is its variables merged the way a
    /// descriptor's own are — see [`Descriptor::merged`].
    fn merged(&self, over: &Capability) -> Capability {
        Capability {
            env: self
                .env
                .merged(&over.env, |_, over| over.clone(), Clone::clone),
        }
    }
}

/// One directory of a language's store, as the file says it.
///
/// A mapping rather than the bare path, because a store says more about itself
/// than where it is: how it is kept under its language's size, which is one of
/// three answers — see [`Bounded`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    /// Where it is: `{cache}/…` or `{stores}/…`, and nowhere else — see
    /// [`placed`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dir: Option<String>,

    /// `by-its-tool` where the tool filling it evicts for itself, handed the
    /// size in a variable of its own — see [`Evicted`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evicted: Option<Evicted>,

    /// Or what the sweep takes out of it whole — see [`Unit`]. Replaced whole
    /// by an override rather than merged into, the way `detect` is: half a set
    /// of units is a store with packages nobody bounds, and an empty list is
    /// how an installer says a built-in's store is never to be swept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    units: Option<Vec<Unit>>,
}

impl Store {
    /// `over` written over this, key by key.
    fn merged(&self, over: &Store) -> Store {
        Store {
            dir: over.dir.clone().or_else(|| self.dir.clone()),
            evicted: over.evicted.or(self.evicted),
            units: over.units.clone().or_else(|| self.units.clone()),
        }
    }

    /// Why this store's way of being bounded is not one, where it is not — a
    /// clause that follows the store's name, in [`Descriptor::misplaced`]'s
    /// words.
    fn unbounded(&self) -> Option<String> {
        if self.evicted.is_some() && self.units.as_ref().is_some_and(|units| !units.is_empty()) {
            return Some(String::from(
                "both as evicted by its tool and as swept by unit, and a store is one or the \
                 other",
            ));
        }

        self.units.iter().flatten().find_map(Unit::unread)
    }
}

/// How a store is kept under its language's size: the one answer of the three
/// a descriptor can give that is not about the sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evicted {
    /// The tool filling it is handed the size and evicts for itself, so the
    /// sweep leaves it alone. sccache is the one: `SCCACHE_CACHE_SIZE`, least
    /// recently used first. A variable carrying `{size}` is how the tool is
    /// told, which is the descriptor's own business.
    ByItsTool,
}

/// What the sweep takes out of a store whole — **a package, never a file of
/// one**, because a Go module or a Maven artifact with one file missing is a
/// broken store rather than a smaller one.
///
/// Found by walking down from `under` and stopping at the first entry that is
/// one: every entry at `depth`, or the first one down whose name is one of
/// `named`, or the first directory down holding an entry whose name is one of
/// `holding` — and where more than one is said, an entry has to be all of them.
/// The walk follows no symlink and never takes one as a unit.
///
/// **A unit may be a file** where the file is the whole package — a `.crate`,
/// a `.zip` — or where the store is content-addressed and its tool checks every
/// blob it reads, so that a blob gone is a blob fetched again rather than a
/// package half there.
///
/// Patterns are a name's and nothing more: `*` for any run of characters and
/// `?` for one, and no `/`. A descriptor is data, so it says what a unit looks
/// like and never a command that would find one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    /// Where under the store to look, `/` between its segments on every
    /// platform; the store itself where it is not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    under: Option<String>,

    /// How deep under that: one for what is directly in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    depth: Option<usize>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    named: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    holding: Vec<String>,
}

impl Unit {
    /// Why this is not a unit the sweep can find, where it is not.
    fn unread(&self) -> Option<String> {
        if self.depth.is_none() && self.named.is_empty() && self.holding.is_empty() {
            return Some(String::from(
                "with a unit named by none of depth, named or holding, which would be anything",
            ));
        }

        if self.depth == Some(0) {
            return Some(String::from(
                "with a unit at depth 0, which is the whole store rather than anything in it",
            ));
        }

        if let Some(under) = &self.under
            && !under
                .split('/')
                .all(|segment| !matches!(segment, "" | "." | "..") && !segment.contains('{'))
        {
            return Some(format!(
                "with a unit under {under}, and a unit is under a path inside the store, \
                 written with `/` between its segments"
            ));
        }

        self.named
            .iter()
            .chain(&self.holding)
            .find(|pattern| pattern.is_empty() || pattern.contains(['/', '\\']))
            .map(|pattern| {
                format!("with a unit named by `{pattern}`, and a pattern is one name, never a path")
            })
    }

    /// The directory under `store` this unit is looked for in.
    pub fn root(&self, store: &Path) -> PathBuf {
        self.under
            .iter()
            .flat_map(|under| under.split('/'))
            .fold(store.to_owned(), |path, segment| path.join(segment))
    }

    /// Whether an entry the walk reached at `depth` under [`Unit::root`] is one
    /// of these, `holding` asked of `children` — what is in it, read only when
    /// this asks.
    pub fn is(&self, name: &str, depth: usize, children: impl FnOnce() -> Vec<String>) -> bool {
        self.depth.is_none_or(|wanted| wanted == depth)
            && (self.named.is_empty() || self.named.iter().any(|pattern| matches(pattern, name)))
            && (self.holding.is_empty()
                || children()
                    .iter()
                    .any(|child| self.holding.iter().any(|pattern| matches(pattern, child))))
    }

    /// Whether the walk has gone as deep as a unit can be, so that nothing
    /// under an entry at `depth` is worth reading.
    pub fn deepest(&self, depth: usize) -> bool {
        self.depth.is_some_and(|wanted| depth >= wanted)
    }
}

/// Whether `name` is what `pattern` says: `*` any run of characters, `?` any
/// one, and everything else itself.
fn matches(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();

    // The last `*` seen and where in the name it was tried from, which is the
    // one place to go back to on a mismatch: a greedy match with one step of
    // backtracking, which is enough for patterns with no character classes.
    let (mut p, mut n) = (0, 0);
    let mut star: Option<(usize, usize)> = None;

    while n < name.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p, n));
                p += 1;
            }
            Some(&c) if c == '?' || c == name[n] => {
                p += 1;
                n += 1;
            }
            _ => match star {
                Some((at, from)) => {
                    p = at + 1;
                    n = from + 1;
                    star = Some((at, from + 1));
                }
                None => return false,
            },
        }
    }

    pattern[p..].iter().all(|&c| c == '*')
}

/// How one store is kept under its language's size, as the sweep reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounded<'a> {
    /// By the tool that fills it, which is handed the size — see
    /// [`Evicted::ByItsTool`].
    ByItsTool,

    /// By the sweep, which takes these out whole, oldest first.
    ByUnit(&'a [Unit]),

    /// Not at all: the descriptor names no unit, so the sweep never touches
    /// it, and the page says so.
    NotAtAll,
}

/// Which placeholder a store's directory is under, and the path below it — or
/// why it is not a directory a store can be, as a clause.
///
/// The placeholder first and alone, then at least one segment under it, and no
/// segment that is empty, `.`, `..` or another placeholder: a store **is**
/// somewhere under the Build Cache or the directory beside the Worktrees, and
/// never one of those two whole — each holds every other language's store too.
/// Nor under the directory the sweep moves units aside into, which it empties
/// — see [`Machine::aside`].
fn placed(dir: &str) -> Result<(&'static str, &str), String> {
    let refused = || {
        format!(
            "and a store is a directory under {CACHE} or {STORES}, written as one of the \
             two, a `/` and the path beneath it"
        )
    };

    let (placeholder, rest) = [CACHE, STORES]
        .into_iter()
        .find_map(|placeholder| {
            Some((
                placeholder,
                dir.strip_prefix(placeholder)?.strip_prefix('/')?,
            ))
        })
        .ok_or_else(refused)?;

    match rest
        .split('/')
        .all(|segment| !matches!(segment, "" | "." | "..") && !segment.contains('{'))
        && rest.split('/').next() != Some(ASIDE)
    {
        true => Ok((placeholder, rest)),
        false => Err(refused()),
    }
}

/// What this machine can put behind the placeholders, and which capabilities it
/// can offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    cache: PathBuf,
    stores: PathBuf,
    sccache: Option<PathBuf>,
}

impl Machine {
    /// The machine whose Build Cache is at `cache`, whose Worktrees are under
    /// `data_dir`, and which found `sccache` where it found one — at the path a
    /// session reaches it by rather than the path it is on the host, because
    /// what a variable holds is read inside.
    ///
    /// The stores directory is named here and nowhere else: beside the
    /// Worktrees, because that is the whole of what it is for — a store the
    /// project has to be on one filesystem with, and a Worktree is where the
    /// project is.
    pub fn of(cache: &Path, data_dir: &Path, sccache: Option<PathBuf>) -> Machine {
        Machine {
            cache: cache.to_owned(),
            stores: stores(data_dir),
            sccache,
        }
    }

    /// The directory a store written as `dir` is on this machine, composed a
    /// segment at a time the way this platform composes a path — or `None`
    /// where `dir` is not a store's at all — see [`placed`].
    pub fn store(&self, dir: &str) -> Option<PathBuf> {
        let (placeholder, rest) = placed(dir).ok()?;

        let base = match placeholder {
            CACHE => &self.cache,
            _ => &self.stores,
        };

        Some(
            rest.split('/')
                .fold(base.clone(), |path, segment| path.join(segment)),
        )
    }

    /// Where a unit swept out of the store at `store` is moved aside to before
    /// it is deleted: a directory of Verkstead's own at the top of whichever of
    /// the two placeholders' directories the store is under — see
    /// [`crate::eviction`]. `None` for a directory under neither.
    ///
    /// **At the top of the placeholder's directory** rather than anywhere
    /// else, because a rename is only a rename on one filesystem: the Build
    /// Cache and the directory beside the Worktrees may each be a filesystem of
    /// their own, and everything a descriptor names is under one of them.
    pub fn aside(&self, store: &Path) -> Option<PathBuf> {
        self.asides()
            .into_iter()
            .find(|aside| aside.parent().is_some_and(|base| store.starts_with(base)))
    }

    /// Both of those, whether or not anything is in them — which is where the
    /// sweep looks for what a pass that died left aside.
    pub fn asides(&self) -> [PathBuf; 2] {
        [self.cache.join(ASIDE), self.stores.join(ASIDE)]
    }

    /// Whether this server has the behaviour behind `capability`.
    ///
    /// One arm, and a name it has never heard of is a capability it cannot
    /// offer: a descriptor written for a Verkstead newer than this one is a
    /// language that still gets its variables, rather than a file that will not
    /// load.
    fn offers(&self, capability: &str) -> bool {
        match capability {
            SCCACHE => self.sccache.is_some(),
            _ => false,
        }
    }

    /// The directory `value` would put whitespace into, where `value` is a
    /// **line of words** — a line of flags, which is what the file writes where
    /// a tool has no variable for a path and reads it out of its options.
    ///
    /// A tool reading such a line splits it on whitespace, and not every one
    /// reads a quote: Maven 3's `mvn` splits `MAVEN_OPTS` bare. So a Build
    /// Cache under a name with a space in it, which a Windows user name gives
    /// by default, would hand over a line that breaks the tool outright, and
    /// the variable is left out instead. A value that is one word is a path
    /// read whole, and a space in it is nobody's business.
    fn splitting(&self, value: &str) -> Option<&Path> {
        if !value.contains(char::is_whitespace) {
            return None;
        }

        [
            value.contains(CACHE).then_some(self.cache.as_path()),
            value.contains(STORES).then_some(self.stores.as_path()),
            value
                .contains(SCCACHE_AT)
                .then_some(self.sccache.as_deref())
                .flatten(),
        ]
        .into_iter()
        .flatten()
        .find(|dir| dir.to_string_lossy().contains(char::is_whitespace))
    }

    /// `value` with its placeholders filled in, or `None` where one of them
    /// names something this machine has not got.
    ///
    /// A variable that cannot be filled is left out rather than written with the
    /// placeholder still in it: what a `RUSTC_WRAPPER` naming `{sccache}`
    /// literally would do is fail every build inside.
    ///
    /// **And a value that named a directory is composed the way this platform
    /// composes a path.** The grammar has one spelling of a separator, because a
    /// descriptor is written once and read on three platforms — so `{cache}/go`
    /// is what an installer writes everywhere, and what a Windows session is
    /// handed is the path its own tools would have built. Which is what keeps a
    /// session's `CARGO_HOME` on that platform byte for byte the
    /// `dir.join("cargo")` it was before a descriptor said it.
    ///
    /// Said of what the **file** wrote and before anything is put in its place,
    /// so that every path this machine hands over keeps its own spelling: a
    /// cache directory an installer passed as `C:/builds` is written into a
    /// session exactly as they gave it, and only the `/` the descriptor itself
    /// holds becomes a `\`.
    ///
    /// Safe to say of the whole of what the file wrote, because naming one of
    /// the two directories is what makes a value a path: the grammar has no
    /// other reason to reach for one — see the embedded file, where each
    /// placeholder says what it stands for. A flag with a path in it comes out
    /// right for the same reason, its only separators being that path's.
    fn filled(&self, value: &str, size: &str, used: &mut Used) -> Option<String> {
        let mut filled = match value.contains(CACHE) || value.contains(STORES) {
            true => separated(value),
            false => value.to_owned(),
        };

        if filled.contains(CACHE) {
            used.cache = true;
            filled = filled.replace(CACHE, &self.cache.display().to_string());
        }

        if filled.contains(STORES) {
            used.stores = true;
            filled = filled.replace(STORES, &self.stores.display().to_string());
        }

        if filled.contains(SIZE) {
            filled = filled.replace(SIZE, size);
        }

        if filled.contains(SCCACHE_AT) {
            filled = filled.replace(SCCACHE_AT, &self.sccache.as_ref()?.display().to_string());
        }

        Some(filled)
    }
}

/// `written` with the grammar's separator said the way this platform says one.
///
/// Nothing at all on the two Unixes, where the two are the same character. On
/// Windows it is what keeps a descriptor one file: an installer writes
/// `{cache}/go` once and every platform hands a session the path its own tools
/// compose — which is what a Windows session's `CARGO_HOME` was before a
/// descriptor said it, that having been a `Path::join`.
///
/// A forward slash would have worked there, every Win32 call reading both — and
/// a path spelled two ways in one string is a path two naive readers can
/// disagree about, which is a worse thing to hand a build tool than a separator
/// it has to convert.
fn separated(written: &str) -> String {
    match std::path::MAIN_SEPARATOR {
        '/' => written.to_owned(),
        _ => written.replace('/', std::path::MAIN_SEPARATOR_STR),
    }
}

/// The directory beside the Worktrees, under `data`.
///
/// Named here for the reason [`crate::worktrees::directory`] is named there:
/// what makes it the right place is what is *next to* it, so one function says
/// where it is and the descriptors point at it by placeholder.
///
/// Public for the proofs' sake as much as the server's: `tests/package_stores.rs`
/// really installs out of a store under it, and a suite that spelled the
/// directory out itself would be a suite agreeing with a second copy of this.
pub fn stores(data: &Path) -> PathBuf {
    data.join("stores")
}

/// Which placeholder directories a rendering actually used, which is what says
/// which of them a session is opened onto.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Used {
    cache: bool,
    stores: bool,
}

/// What the loaded descriptors give one session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Given {
    env: Vec<(String, String)>,
    used: Used,
    sccache: bool,
    dirs: Vec<PathBuf>,
}

impl Given {
    /// Every variable, in the order the descriptors name them.
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// And the directories that have to be bound writable at the same path
    /// inside for those variables to mean anything — the Build Cache where
    /// something named it, the stores directory where something named that, and
    /// neither where nothing did.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// Whether anything named the [`SCCACHE`] capability, which is what says the
    /// binary itself has to be joined in beside the directories.
    pub fn sccache(&self) -> bool {
        self.sccache
    }

    /// Whether this is nothing at all — every language off, or a server with no
    /// descriptors loaded.
    pub fn is_empty(&self) -> bool {
        self.env.is_empty() && self.dirs.is_empty()
    }

    /// `env` filled in and taken on, and the directories it named with it.
    fn taking<'a>(
        &mut self,
        language: &str,
        machine: &Machine,
        size: &str,
        env: impl Iterator<Item = (&'a str, &'a Option<String>)>,
    ) {
        for (name, value) in env {
            // A variable a `null` took out, which is an entry with no value —
            // see [`Descriptor::merged`]. Nothing is set and nothing is
            // unset: what the session gets is what the built-in would have
            // given it minus this one name.
            let Some(value) = value else {
                continue;
            };

            // A line of words with a directory among them, where the directory
            // has whitespace in it: whatever reads the line splits it there.
            if let Some(split) = machine.splitting(value) {
                tracing::warn!(
                    language,
                    variable = name,
                    directory = %split.display(),
                    "a variable is a line of words with a directory among them, and the \
                     directory has whitespace in it that would split the line, so the \
                     session was not given it",
                );

                continue;
            }

            let mut used = self.used;

            match machine.filled(value, size, &mut used) {
                Some(filled) => {
                    self.env.push((name.to_owned(), filled));
                    self.used = used;
                }
                None => tracing::debug!(
                    language,
                    variable = name,
                    "a variable names something this machine has not got, so the session \
                     was not given it",
                ),
            }
        }

        // Rebuilt rather than appended to, so that the order is the
        // placeholders' own however the descriptors are written.
        self.dirs = [
            self.used.cache.then(|| machine.cache.clone()),
            self.used.stores.then(|| machine.stores.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();
    }
}

/// A YAML mapping kept in the order it was written.
///
/// A `BTreeMap` would sort it, and what is being read here is the order a
/// session's environment is set in — see [`Languages`]. Serde has no ordered
/// map of its own and the alternative is a crate, for what is a visitor over
/// pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ordered<V>(Vec<(String, V)>);

impl<V> Default for Ordered<V> {
    fn default() -> Ordered<V> {
        Ordered(Vec::new())
    }
}

impl<V> Ordered<V> {
    /// The value under `key`, where there is one.
    fn get(&self, key: &str) -> Option<&V> {
        self.0
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value))
    }

    /// Every pair, in the order they were written.
    fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    /// Whether there is nothing in it, which is what says a key is written
    /// away rather than written empty.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<V: Clone> Ordered<V> {
    /// `over` merged into this key by key: a key in both is what `merging`
    /// makes of the pair, and a key only `over` has is what `alone` makes of it
    /// — ordinarily itself, and for a language descriptor the one place the
    /// answer turns on there being nothing underneath — see
    /// [`Descriptor::alone`].
    ///
    /// The order is this one's, with whatever `over` adds on the end — which is
    /// what keeps a session's environment in the built-in file's order however
    /// an installer's own entries are written.
    fn merged(
        &self,
        over: &Ordered<V>,
        merging: impl Fn(&V, &V) -> V,
        alone: impl Fn(&V) -> V,
    ) -> Ordered<V> {
        let mut merged = self.0.clone();

        for (key, value) in over.iter() {
            match merged.iter_mut().find(|(name, _)| name == key) {
                Some((_, held)) => *held = merging(held, value),
                None => merged.push((key.to_owned(), alone(value))),
            }
        }

        Ordered(merged)
    }
}

impl<V: Serialize> Serialize for Ordered<V> {
    /// Back out as the mapping it was read as, in the order it was read in.
    ///
    /// Written by hand for the reason the reading half is: what goes back into
    /// `config.yaml` is what an installer wrote there, and a map that sorted
    /// itself on the way out would rewrite their file every time somebody
    /// saved the settings page.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Ordered<V> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Ordered<V>, D::Error> {
        deserializer.deserialize_map(Pairs(PhantomData))
    }
}

/// The visitor behind it: every pair, as the document has them.
struct Pairs<V>(PhantomData<V>);

impl<'de, V: Deserialize<'de>> Visitor<'de> for Pairs<V> {
    type Value = Ordered<V>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a mapping")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Ordered<V>, M::Error> {
        let mut pairs = Vec::with_capacity(map.size_hint().unwrap_or_default());

        while let Some((key, value)) = map.next_entry::<String, V>()? {
            pairs.push((key, value));
        }

        Ok(Ordered(pairs))
    }
}

impl Serialize for Languages {
    /// Every entry as the file had it: a descriptor as its keys, and an entry
    /// nothing could read as the text it was written in.
    ///
    /// Which is why this is not the derive. A save from the settings page writes
    /// the whole of `config.yaml`, and the whole of it includes the entry the
    /// installer is about to go and fix — see [`Languages::under_the_page`],
    /// which is what carries it this far.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.0.len()))?;

        for (name, descriptor) in self.iter() {
            match &descriptor.unread {
                Some(unread) => map.serialize_entry(name, &unread.written)?,
                None => map.serialize_entry(name, descriptor)?,
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for Languages {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Languages, D::Error> {
        deserializer.deserialize_map(Entries)
    }
}

/// The visitor behind it: every entry read on its own, so that one that will
/// not load is one language rather than the file.
///
/// The value is taken as a [`Raw`] first, which cannot fail, and read as a
/// descriptor from there — see [`Descriptor::read`]. Reading the map straight
/// into descriptors would be a single mistyped variable somewhere in
/// `config.yaml` costing every language its entry, and there is no recovering
/// from a failed `next_value` to do it any other way.
struct Entries;

impl<'de> Visitor<'de> for Entries {
    type Value = Languages;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a mapping of language descriptors")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Languages, M::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or_default());

        while let Some(name) = map.next_key::<String>()? {
            let descriptor = Descriptor::read(map.next_value::<Raw>()?);

            if let Some(why) = descriptor.unread() {
                // Said once, where the name is: the file the human wrote and
                // Verkstead cannot read is the one thing in the settings they
                // would want telling about, and the page says the same sentence
                // to whoever is looking at it instead.
                tracing::warn!(
                    language = name,
                    why,
                    "a language's entry in config.yaml was not used, so it falls back to the \
                     descriptor Verkstead ships",
                );
            }

            entries.push((name, descriptor));
        }

        Ok(Languages(Ordered(entries)))
    }
}

/// A YAML value kept exactly as the file had it.
///
/// What an entry nothing could read is held as. Every shape a document can
/// hold, and no opinion about any of them: it is read through `deserialize_any`
/// and written back out as itself, which is the whole of what it is for.
///
/// Numbers are the one place it is not quite the source text — `1.50` comes
/// back `1.5` — which is what keeps it a value rather than a span, and is why
/// nothing here is `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
enum Raw {
    /// A `null`, a `~`, and a key with nothing under it.
    Nothing,
    Yes(bool),
    Whole(i64),
    Counted(u64),
    Fractional(f64),
    Text(String),
    List(Vec<Raw>),
    Mapping(Ordered<Raw>),
}

impl<'de> Deserialize<'de> for Raw {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Raw, D::Error> {
        deserializer.deserialize_any(Anything)
    }
}

/// The visitor behind it: whatever the document holds, in the shape it holds
/// it.
struct Anything;

impl<'de> Visitor<'de> for Anything {
    type Value = Raw;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any value")
    }

    fn visit_unit<E>(self) -> Result<Raw, E> {
        Ok(Raw::Nothing)
    }

    fn visit_none<E>(self) -> Result<Raw, E> {
        Ok(Raw::Nothing)
    }

    fn visit_some<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Raw, D::Error> {
        Raw::deserialize(deserializer)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Raw, E> {
        Ok(Raw::Yes(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Raw, E> {
        Ok(Raw::Whole(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Raw, E> {
        Ok(Raw::Counted(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Raw, E> {
        Ok(Raw::Fractional(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Raw, E> {
        Ok(Raw::Text(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Raw, E> {
        Ok(Raw::Text(value))
    }

    fn visit_seq<S: serde::de::SeqAccess<'de>>(self, mut seq: S) -> Result<Raw, S::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or_default());

        while let Some(item) = seq.next_element::<Raw>()? {
            items.push(item);
        }

        Ok(Raw::List(items))
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Raw, M::Error> {
        Pairs(PhantomData).visit_map(map).map(Raw::Mapping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The machine every test here renders against: a cache, a Data Directory
    /// and an sccache where one is wanted.
    fn machine(sccache: bool) -> Machine {
        Machine::of(
            Path::new("/var/cache/verkstead"),
            Path::new("/var/lib/verkstead"),
            sccache.then(|| PathBuf::from("/verkstead/bin/sccache")),
        )
    }

    /// A descriptor file as an installer would have written it.
    fn written(yaml: &str) -> Languages {
        Languages::read(yaml).expect("a descriptor an installer could have written")
    }

    /// A path under that machine's Build Cache, composed the way this platform
    /// composes one — which is what a descriptor's `{cache}/…` comes to.
    ///
    /// Said rather than spelled, because the grammar has one separator and the
    /// three platforms do not: a descriptor writes `/` everywhere and a Windows
    /// session is handed the path its own tools would have built, so a test that
    /// spelled the answer out would be a test that only held on two of them.
    fn cached(rest: &str) -> String {
        under(Path::new("/var/cache/verkstead"), rest)
    }

    /// And one under the directory beside the Worktrees, for the same reason.
    fn stored(rest: &str) -> String {
        under(&stores(Path::new("/var/lib/verkstead")), rest)
    }

    /// What the JVM's descriptor hands Maven: the local repository under the
    /// Build Cache, and the file locks two sessions writing it need.
    fn maven_opts() -> String {
        format!(
            "-Dmaven.repo.local={} -Daether.syncContext.named.factory=file-lock \
             -Daether.syncContext.named.nameMapper=file-gav",
            cached("maven/repository"),
        )
    }

    /// `rest` under `base`, **a segment at a time**, which is how a descriptor's
    /// own `/` becomes this platform's separator — see [`separated`].
    ///
    /// One `join` of `"go/mod"` would keep that `/` inside the string on
    /// Windows and compare a path no `go` there would read against the one the
    /// code correctly composed. A difference Linux cannot see, so it is written
    /// once here rather than left to the caller.
    fn under(base: &Path, rest: &str) -> String {
        rest.split('/')
            .fold(base.to_owned(), |path, segment| path.join(segment))
            .display()
            .to_string()
    }

    /// The built-ins parse, and what each of them says is what a session gets
    /// today.
    ///
    /// The file is the documentation's worked example as well as the data, so
    /// this is what says the grammar it is written in is the one the loader
    /// reads.
    #[test]
    fn the_built_ins_are_in_the_grammar_an_installer_writes() {
        let rust = built_in()
            .get(RUST)
            .expect("Rust is the language Verkstead ships knowing");

        assert_eq!(rust.label(), Some("Rust"));
        assert_eq!(rust.detect, vec![String::from("Cargo.toml")]);
        assert_eq!(
            built_in().wanting(SCCACHE),
            Some(crate::build_cache::SIZE),
            "Rust's descriptor is what asks for the Compile Server, at the size \
             nobody has configured",
        );

        let go = built_in().get(GO).expect("and Go is the second");

        assert_eq!(go.label(), Some("Go"));
        assert_eq!(go.detect, vec![String::from("go.mod")]);
        assert!(
            !go.names(SCCACHE),
            "Go's compiled half is a directory, so there is no Compile Server \
             in it to name",
        );

        let node = built_in().get(NODE).expect("and Node is the third");

        assert_eq!(node.label(), Some("Node"));
        assert_eq!(node.detect, vec![String::from("package.json")]);
        assert!(
            !node.names(SCCACHE),
            "nothing in the npm ecosystem compiles through a server, so there \
             is nothing here to name either",
        );

        let python = built_in().get(PYTHON).expect("and Python is the fourth");

        assert_eq!(python.label(), Some("Python"));
        assert_eq!(
            python.detect,
            vec![
                String::from("pyproject.toml"),
                String::from("requirements.txt")
            ],
            "two manifests, because Python has two ordinary shapes and a \
             descriptor may name as many as the language has",
        );
        assert!(
            !python.names(SCCACHE),
            "and nothing in this ecosystem compiles through a server either",
        );

        let dotnet = built_in().get(DOTNET).expect("and .NET is the fifth");

        assert_eq!(dotnet.label(), Some(".NET"));
        assert!(
            dotnet.detect.is_empty(),
            "the one built-in with nothing to detect it by: a .NET project is a \
             `*.csproj` or a `*.sln`, and `detect` matches a filename rather \
             than a glob — so the list is empty rather than the server growing \
             a matcher for one language",
        );
        assert!(
            !dotnet.names(SCCACHE),
            "and NuGet has no compiled half at all, so there is nothing here \
             for a Compile Server to be",
        );

        let cpp = built_in().get(CPP).expect("and C/C++ is the sixth");

        assert_eq!(cpp.label(), Some("C/C++"));
        assert_eq!(
            cpp.detect,
            vec![
                String::from("CMakeLists.txt"),
                String::from("native/CMakeLists.txt"),
                String::from("cpp/CMakeLists.txt"),
                String::from("meson.build"),
            ],
            "no one manifest, so the build files a checkout ordinarily holds \
             one of — Meson's among them, for the warning, although only \
             CMake reads the variables",
        );
        assert!(
            cpp.names(SCCACHE),
            "the second language naming the Compile Server, which is the same \
             capability Rust's is rather than a second server",
        );
        assert!(
            cpp.env.is_empty(),
            "and no store of its own: its objects are the one Compile Server's",
        );

        let jvm = built_in().get(JVM).expect("and the JVM is the seventh");

        assert_eq!(jvm.label(), Some("JVM"));
        assert_eq!(
            jvm.detect,
            [
                "pom.xml",
                "build.gradle",
                "build.gradle.kts",
                "settings.gradle",
                "settings.gradle.kts"
            ]
            .map(String::from),
            "Maven's manifest and Gradle's four",
        );
        assert!(
            !jvm.names(SCCACHE),
            "and nothing on the JVM compiles through sccache",
        );
        assert!(
            !jvm.env
                .iter()
                .any(|(_, value)| value.as_deref().is_some_and(|v| v.contains("caching"))),
            "and nothing about Gradle's build cache: whether it is on is the Repo's to say",
        );
    }

    /// A session of a machine with an sccache: Rust's four variables, in the
    /// order it has always had them, then Go's two, Node's seven, Python's
    /// six, .NET's three, C/C++'s two launchers and the JVM's three, and the
    /// two directories they name open underneath.
    ///
    /// Rust's four lead and are unchanged, which is the promise the descriptors
    /// landed on: a language added to the file is variables after the ones a
    /// session already had rather than a different environment.
    #[test]
    fn a_session_is_given_what_the_built_ins_say() {
        let given = built_in().given(&machine(true));

        assert_eq!(
            given.env(),
            [
                (String::from("CARGO_HOME"), cached("cargo")),
                (
                    String::from("RUSTC_WRAPPER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (String::from("SCCACHE_DIR"), cached("sccache")),
                (String::from("SCCACHE_CACHE_SIZE"), String::from("30G")),
                (String::from("GOMODCACHE"), cached("go/mod")),
                (String::from("GOCACHE"), cached("go/build")),
                (String::from("NPM_CONFIG_CACHE"), cached("npm")),
                (String::from("PNPM_CONFIG_STORE_DIR"), stored("pnpm")),
                (
                    String::from("PNPM_CONFIG_CACHE_DIR"),
                    cached("pnpm/metadata")
                ),
                (String::from("YARN_CACHE_FOLDER"), cached("yarn/cache")),
                (String::from("YARN_GLOBAL_FOLDER"), cached("yarn/global")),
                (String::from("DENO_DIR"), stored("deno")),
                (String::from("BUN_INSTALL_CACHE_DIR"), stored("bun")),
                (String::from("PIP_CACHE_DIR"), cached("pip")),
                (String::from("UV_CACHE_DIR"), stored("uv")),
                (String::from("POETRY_CACHE_DIR"), cached("poetry")),
                (
                    String::from("POETRY_VIRTUALENVS_IN_PROJECT"),
                    String::from("true")
                ),
                (String::from("PIPENV_CACHE_DIR"), cached("pipenv")),
                (String::from("PIPENV_VENV_IN_PROJECT"), String::from("1")),
                (String::from("NUGET_PACKAGES"), cached("nuget/packages")),
                (String::from("NUGET_HTTP_CACHE_PATH"), cached("nuget/http")),
                (String::from("NUGET_SCRATCH"), cached("nuget/scratch")),
                (
                    String::from("CMAKE_C_COMPILER_LAUNCHER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (
                    String::from("CMAKE_CXX_COMPILER_LAUNCHER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (String::from("MAVEN_OPTS"), maven_opts()),
                (String::from("GRADLE_USER_HOME"), cached("gradle")),
                (
                    String::from("GRADLE_OPTS"),
                    String::from("-Dorg.gradle.daemon=false")
                ),
            ],
        );

        assert_eq!(
            given.dirs(),
            [
                PathBuf::from("/var/cache/verkstead"),
                PathBuf::from("/var/lib/verkstead/stores"),
            ],
            "the Build Cache once however many descriptors name it, and the \
             directory beside the Worktrees because pnpm's store and uv's are \
             there: a bind per store would be several holes saying the same thing"
        );
        assert!(given.sccache(), "and the sccache to reach it through");
    }

    /// And on a machine with none: the downloads are still shared, and nothing
    /// points at a compile server that is not there.
    #[test]
    fn without_an_sccache_the_capabilitys_variables_are_left_out() {
        let given = built_in().given(&machine(false));

        assert_eq!(
            given.env(),
            [
                (String::from("CARGO_HOME"), cached("cargo")),
                (String::from("GOMODCACHE"), cached("go/mod")),
                (String::from("GOCACHE"), cached("go/build")),
                (String::from("NPM_CONFIG_CACHE"), cached("npm")),
                (String::from("PNPM_CONFIG_STORE_DIR"), stored("pnpm")),
                (
                    String::from("PNPM_CONFIG_CACHE_DIR"),
                    cached("pnpm/metadata")
                ),
                (String::from("YARN_CACHE_FOLDER"), cached("yarn/cache")),
                (String::from("YARN_GLOBAL_FOLDER"), cached("yarn/global")),
                (String::from("DENO_DIR"), stored("deno")),
                (String::from("BUN_INSTALL_CACHE_DIR"), stored("bun")),
                (String::from("PIP_CACHE_DIR"), cached("pip")),
                (String::from("UV_CACHE_DIR"), stored("uv")),
                (String::from("POETRY_CACHE_DIR"), cached("poetry")),
                (
                    String::from("POETRY_VIRTUALENVS_IN_PROJECT"),
                    String::from("true")
                ),
                (String::from("PIPENV_CACHE_DIR"), cached("pipenv")),
                (String::from("PIPENV_VENV_IN_PROJECT"), String::from("1")),
                (String::from("NUGET_PACKAGES"), cached("nuget/packages")),
                (String::from("NUGET_HTTP_CACHE_PATH"), cached("nuget/http")),
                (String::from("NUGET_SCRATCH"), cached("nuget/scratch")),
                (String::from("MAVEN_OPTS"), maven_opts()),
                (String::from("GRADLE_USER_HOME"), cached("gradle")),
                (
                    String::from("GRADLE_OPTS"),
                    String::from("-Dorg.gradle.daemon=false")
                ),
            ],
            "Go's two, Node's seven, Python's six, .NET's three and the JVM's \
             three are in no capability, so a machine with no sccache gets the \
             whole of what those descriptors say — and C/C++'s two launchers are the whole \
             of its capability, so it gets neither",
        );
        assert!(
            !given
                .env()
                .iter()
                .any(|(name, _)| name == "CC" || name == "CXX"),
            "and CC and CXX are set in no case",
        );
        assert_eq!(
            given.dirs(),
            [
                PathBuf::from("/var/cache/verkstead"),
                PathBuf::from("/var/lib/verkstead/stores"),
            ],
        );
        assert!(!given.sccache());
    }

    /// A language switched off is no variables and no directory of *its* own:
    /// the switch closes the hole rather than leaving it open and unused, and
    /// it closes one language's rather than the file's.
    #[test]
    fn a_language_switched_off_gives_a_session_nothing() {
        let off = built_in().merged(&written(
            "languages:\n  rust:\n    enabled: false\n  cpp:\n    enabled: false\n",
        ));
        let given = off.given(&machine(true));

        assert_eq!(
            given.env().first(),
            Some(&(String::from("GOMODCACHE"), cached("go/mod"))),
            "nothing of Rust's is left, and the languages beside it are untouched",
        );
        assert_eq!(
            given.env().len(),
            21,
            "which is Go's two, Node's seven, Python's six, .NET's three and the \
             JVM's three, and nothing else"
        );
        assert!(!given.sccache());
        assert!(
            off.wanting(SCCACHE).is_none(),
            "and with both languages naming it off, nothing wants a Compile \
             Server up"
        );

        // And **both** languages that name the second directory switched off:
        // the Build Cache is still open, because the languages still on point
        // into it, and the directory beside the Worktrees is not. Both of them,
        // because either one on its own leaves it named — Node's three stores
        // and uv's are four reasons for the one bind.
        let without_node = off.merged(&written("languages:\n  node:\n    enabled: false\n"));

        assert_eq!(
            without_node.given(&machine(true)).dirs(),
            [
                PathBuf::from("/var/cache/verkstead"),
                PathBuf::from("/var/lib/verkstead/stores"),
            ],
            "uv's store is still beside the Worktrees with Node switched off",
        );

        let without_either =
            without_node.merged(&written("languages:\n  python:\n    enabled: false\n"));
        let given = without_either.given(&machine(true));

        assert_eq!(
            given.dirs(),
            [PathBuf::from("/var/cache/verkstead")],
            "the one placeholder a language still on names, and not the one \
             only the languages switched off did"
        );

        // And with every one of them off there is nothing to open at all,
        // which is what an installation that wants none of this looks like.
        let none = without_either.merged(&written(
            "languages:\n  go:\n    enabled: false\n  dotnet:\n    enabled: false\n  jvm:\n    \
             enabled: false\n",
        ));
        let given = none.given(&machine(true));

        assert!(given.is_empty());
        assert!(given.env().is_empty());
        assert!(given.dirs().is_empty());
    }

    /// C/C++ on its own wants the Compile Server — nothing starts it by
    /// detection, so the switch is the whole of what brings it up — at the one
    /// size, which is Rust's whether or not Rust is on. And beside Rust it
    /// changes nothing of Rust's: Rust is written first, so its variables lead
    /// and its size is the one the server is started at.
    #[test]
    fn cpp_on_its_own_wants_the_compile_server_and_beside_rust_changes_nothing() {
        let alone = built_in().merged(&written(
            "languages:\n  rust:\n    enabled: false\n    size: 60G\n  cpp:\n    size: 12G\n",
        ));

        assert_eq!(
            alone.wanting(SCCACHE),
            Some("60G"),
            "with Rust off the one server is still sized by Rust's entry: switching \
             Rust off is not the store shrinking to C/C++'s size",
        );

        let given = alone.given(&machine(true));

        assert!(
            given.sccache(),
            "and the sccache is reached from the session"
        );
        assert_eq!(
            given
                .env()
                .iter()
                .filter(|(name, _)| name.starts_with("CMAKE_"))
                .cloned()
                .collect::<Vec<_>>(),
            [
                (
                    String::from("CMAKE_C_COMPILER_LAUNCHER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (
                    String::from("CMAKE_CXX_COMPILER_LAUNCHER"),
                    String::from("/verkstead/bin/sccache")
                ),
            ],
        );
        assert!(
            !given
                .env()
                .iter()
                .any(|(name, _)| name.starts_with("SCCACHE_") || name == "CC" || name == "CXX"),
            "and nothing else: the server reads its own store and size, and CC \
             and CXX are left alone",
        );

        // And both on: Rust's four exactly as they were before this descriptor
        // existed, and Rust's size the server's, whatever C/C++'s says.
        let both = built_in().merged(&written(
            "languages:\n  rust:\n    size: 5G\n  cpp:\n    size: 12G\n",
        ));
        let given = both.given(&machine(true));

        assert_eq!(both.wanting(SCCACHE), Some("5G"));
        assert_eq!(
            given.env()[..4],
            [
                (String::from("CARGO_HOME"), cached("cargo")),
                (
                    String::from("RUSTC_WRAPPER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (String::from("SCCACHE_DIR"), cached("sccache")),
                (String::from("SCCACHE_CACHE_SIZE"), String::from("5G")),
            ],
        );
        assert_eq!(
            given
                .env()
                .iter()
                .filter(|(name, _)| name.starts_with("SCCACHE_"))
                .count(),
            2,
            "each of Rust's two once, and neither pushed again by C/C++",
        );
    }

    /// The size is the human's, and it reaches the variable that reads it.
    #[test]
    fn the_size_a_human_configured_is_what_the_store_is_given() {
        let sized = built_in().merged(&written("languages:\n  rust:\n    size: 5G\n"));
        let given = sized.given(&machine(true));

        assert!(
            given
                .env()
                .contains(&(String::from("SCCACHE_CACHE_SIZE"), String::from("5G"))),
        );
        assert_eq!(
            sized.wanting(SCCACHE),
            Some("5G"),
            "and the Compile Server is started at it"
        );
    }

    /// A size is read in sccache's own grammar, so that one word means the same
    /// to the Compile Server and to the sweep: a whole number, then one binary
    /// multiple or none.
    #[test]
    fn a_size_is_read_the_way_sccache_reads_one() {
        assert_eq!(bytes("30G"), Ok(30 << 30));
        assert_eq!(bytes("10G"), Ok(10 << 30));
        assert_eq!(bytes("500M"), Ok(500 << 20));
        assert_eq!(bytes("64K"), Ok(64 << 10));
        assert_eq!(bytes("2T"), Ok(2 << 40));
        assert_eq!(bytes("4096"), Ok(4096), "nothing after it is bytes");
        assert_eq!(bytes("0"), Ok(0));

        for unread in [
            "",
            "G",
            "lots",
            "1.5G",
            "10g",
            "10GB",
            "10 G",
            " 10G",
            "-1G",
            "+1G",
            "10Gi",
            "99999999999T",
        ] {
            let why = bytes(unread).expect_err(unread);

            assert!(
                why.starts_with("is not a size"),
                "{unread:?} is refused with a reason: {why}",
            );
        }
    }

    /// Rust keeps the size its sccache was always started at, and every other
    /// language — an installer's own among them — starts at the smaller one.
    #[test]
    fn every_language_has_a_default_and_rusts_is_its_own() {
        let installers = built_in().merged(&written(
            "languages:\n  gleam:\n    env:\n      GLEAM_HOME: \"{cache}/gleam\"\n",
        ));

        for (name, descriptor) in installers.iter() {
            let default = match name {
                RUST => "30G",
                _ => "10G",
            };

            assert_eq!(descriptor.size(name), default, "{name}");
            assert_eq!(descriptor.size_configured(), None, "{name}");
            assert_eq!(descriptor.size_unread(), None, "{name}");
        }

        assert_eq!(crate::build_cache::SIZE, "30G");
    }

    /// Every language with a store of its own has a size to draw; C/C++, whose
    /// whole descriptor is the Compile Server's launchers, has none — the one
    /// server is sized by Rust's entry.
    #[test]
    fn every_language_with_a_store_has_one_and_cpp_has_none() {
        for (name, descriptor) in built_in().iter() {
            assert_eq!(descriptor.has_store(), name != CPP, "{name}");
        }

        let installers = written(
            "languages:\n  gleam:\n    env:\n      GLEAM_HOME: \"{stores}/gleam\"\n  \
             zig:\n    env:\n      ZIG_COLOR: \"off\"\n",
        );

        assert!(installers.get("gleam").unwrap().has_store());
        assert!(
            !installers.get("zig").unwrap().has_store(),
            "a language naming neither directory has nowhere to put a store",
        );
    }

    /// A size written into `config.yaml` by hand that is not one is the
    /// default, both to the tool that reads it and to the Compile Server, and
    /// the descriptor says why — the page refuses one, so only a hand-edit gets
    /// here.
    #[test]
    fn a_size_that_is_not_one_is_the_default_and_says_why() {
        let sized = built_in().merged(&written(
            "languages:\n  rust:\n    size: lots\n  go:\n    size: 1.5G\n",
        ));
        let rust = sized.get(RUST).unwrap();
        let go = sized.get(GO).unwrap();

        assert_eq!(rust.size(RUST), "30G");
        assert_eq!(go.size(GO), "10G");
        assert_eq!(
            rust.size_configured(),
            Some("lots"),
            "what was written is kept, so a save puts it back as it was",
        );
        assert!(rust.size_unread().unwrap().starts_with("is not a size"));
        assert!(go.size_unread().is_some());

        assert!(
            sized
                .given(&machine(true))
                .env()
                .contains(&(String::from("SCCACHE_CACHE_SIZE"), String::from("30G"))),
            "sccache is handed the default rather than a word it cannot read",
        );
        assert_eq!(sized.wanting(SCCACHE), Some("30G"));
    }

    /// A size reaches a descriptor's own variable as the human typed it, which
    /// is what a self-evicting tool other than sccache would be handed.
    #[test]
    fn a_typed_size_reaches_any_descriptors_variable_unchanged() {
        let sized = built_in().merged(&written(
            "languages:\n  gleam:\n    size: 512M\n    env:\n      GLEAM_LIMIT: \"{size}\"\n  \
             zig:\n    env:\n      ZIG_LIMIT: \"{size}\"\n",
        ));
        let given = sized.given(&machine(true));

        assert!(
            given
                .env()
                .contains(&(String::from("GLEAM_LIMIT"), String::from("512M")))
        );
        assert!(
            given
                .env()
                .contains(&(String::from("ZIG_LIMIT"), String::from("10G"))),
            "and an installer's own language with nothing said is handed the default",
        );
    }

    /// The second placeholder is granted only where a loaded descriptor names
    /// it, and among the built-ins four do: pnpm's store, deno's cache, bun's
    /// and uv's, because a hardlink out of a store does not cross a filesystem.
    #[test]
    fn the_directory_beside_the_worktrees_is_granted_only_where_it_is_named() {
        assert!(
            built_in().names_stores(),
            "pnpm's, deno's, bun's and uv's stores are beside the Worktrees"
        );

        // Rust, Go and .NET on their own name none of it: Rust's store is under
        // the Build Cache and so are both of Go's and both of NuGet's, so an
        // installation of only those three is opened onto no second directory
        // at all.
        let only_compiled = built_in().merged(&written(
            "languages:\n  node:\n    enabled: false\n  python:\n    enabled: false\n",
        ));

        assert_eq!(
            only_compiled.given(&machine(true)).dirs(),
            [PathBuf::from("/var/cache/verkstead")],
        );

        // An installer's own descriptor naming it is granted it the same way,
        // whatever the built-ins say. A language Verkstead ships nothing for,
        // because what is being shown is the placeholder rather than a store
        // this file already knows where to put.
        let hardlinking = written(
            "languages:\n  zig:\n    label: Zig\n    env:\n      \
             ZIG_GLOBAL_CACHE_DIR: \"{stores}/zig\"\n",
        );

        assert!(hardlinking.names_stores());

        let given = hardlinking.given(&machine(true));

        assert_eq!(
            given.env(),
            [(String::from("ZIG_GLOBAL_CACHE_DIR"), stored("zig"))],
        );
        assert_eq!(
            given.dirs(),
            [PathBuf::from("/var/lib/verkstead/stores")],
            "beside the Worktrees, and the Build Cache not at all: nothing in \
             this descriptor points into it"
        );
    }

    /// A value naming a directory is composed the way this platform composes a
    /// path, which is what the grammar having one separator costs and the whole
    /// of what it costs.
    ///
    /// Said as a `join` rather than as a spelling, because a `join` is what this
    /// has to go on agreeing with: a session's `CARGO_HOME` was
    /// `cache.join("cargo")` before a descriptor said it, and the stage this
    /// module landed in promised the environment byte for byte. On the two
    /// Unixes the two are the same string whatever this does; on Windows they
    /// are the same only because the separators the file wrote are turned into
    /// that platform's before anything is put in their place — which is the one
    /// thing here no Unix run can fail over, so it is said in a form that holds
    /// on all three.
    #[test]
    fn a_value_naming_a_directory_is_the_path_this_platform_would_have_joined() {
        let given = built_in().given(&machine(true));

        let joined = |name: &str, dir: &Path, rest: &str| {
            assert_eq!(
                given
                    .env()
                    .iter()
                    .find_map(|(named, value)| (named == name).then_some(value.as_str())),
                Some(dir.join(rest).display().to_string().as_str()),
                "{name} is the path this platform joins, not the grammar's spelling of it",
            );
        };

        let cache = Path::new("/var/cache/verkstead");

        joined("CARGO_HOME", cache, "cargo");
        joined("SCCACHE_DIR", cache, "sccache");

        // And Go's two, which are the first values with a directory *under* a
        // directory in them: every separator the file wrote is this platform's
        // before anything is put in their place, rather than the first one
        // only.
        joined("GOMODCACHE", &cache.join("go"), "mod");
        joined("GOCACHE", &cache.join("go"), "build");
        joined("PNPM_CONFIG_CACHE_DIR", &cache.join("pnpm"), "metadata");
        joined("YARN_CACHE_FOLDER", &cache.join("yarn"), "cache");

        // And the three built-ins pointed at the other directory, because the
        // two are made different ways — one is the cache as it was handed over
        // and the other is a `join` of this crate's own.
        let beside = stores(Path::new("/var/lib/verkstead"));

        joined("PNPM_CONFIG_STORE_DIR", &beside, "pnpm");
        joined("DENO_DIR", &beside, "deno");
        joined("BUN_INSTALL_CACHE_DIR", &beside, "bun");
        joined("PIP_CACHE_DIR", cache, "pip");
        joined("UV_CACHE_DIR", &beside, "uv");
        joined("POETRY_CACHE_DIR", cache, "poetry");
        joined("PIPENV_CACHE_DIR", cache, "pipenv");
        joined("NUGET_PACKAGES", &cache.join("nuget"), "packages");
        joined("NUGET_HTTP_CACHE_PATH", &cache.join("nuget"), "http");
        joined("NUGET_SCRATCH", &cache.join("nuget"), "scratch");
        joined("GRADLE_USER_HOME", cache, "gradle");

        // And the first value that is a path *inside* a line of flags: Maven's
        // local repository has no variable of its own. The path in it is the
        // one this platform joins, and nothing else in the line has a
        // separator to turn.
        assert_eq!(
            given
                .env()
                .iter()
                .find_map(|(named, value)| (named == "MAVEN_OPTS").then_some(value.as_str())),
            Some(
                format!(
                    "-Dmaven.repo.local={} -Daether.syncContext.named.factory=file-lock \
                     -Daether.syncContext.named.nameMapper=file-gav",
                    cache.join("maven").join("repository").display(),
                )
                .as_str()
            ),
            "MAVEN_OPTS carries the path this platform joins, inside the flag",
        );

        // And Python's other two, which name no directory at all: poetry and
        // pipenv are each told to keep a virtual environment in the project,
        // which is a setting's answer rather than a path, and what a value like
        // that has to come out as is the text the file wrote.
        for (named, said) in [
            ("POETRY_VIRTUALENVS_IN_PROJECT", "true"),
            ("PIPENV_VENV_IN_PROJECT", "1"),
        ] {
            assert_eq!(
                given
                    .env()
                    .iter()
                    .find_map(|(name, value)| (name == named).then_some(value.as_str())),
                Some(said),
                "{named} is the value the descriptor wrote, untouched: nothing \
                 in it names a directory",
            );
        }
    }

    /// A Build Cache with a space in its path, which is what a Windows user
    /// name with one in it gives by default: a value that is a path is handed
    /// over as it stands, and a value that is a **line of words** with the
    /// path among them is left out.
    ///
    /// Because a tool reading a line of words splits it on whitespace, and
    /// Maven 3's `mvn` splits `MAVEN_OPTS` without reading a quote. Handed the
    /// line, every `mvn` in the session would die on a main class named after
    /// the half of the path after the space. Left out, Maven works and keeps a
    /// repository of the session's own.
    #[test]
    fn a_directory_with_a_space_leaves_out_a_line_of_flags_and_keeps_a_path() {
        let spaced = Machine::of(
            Path::new("/home/Jo Doe/.cache/verkstead"),
            Path::new("/var/lib/verkstead"),
            None,
        );
        let given = built_in().given(&spaced);
        let named = |name: &str| {
            given
                .env()
                .iter()
                .find_map(|(named, value)| (named == name).then_some(value.clone()))
        };

        assert_eq!(
            named("CARGO_HOME"),
            Some(under(Path::new("/home/Jo Doe/.cache/verkstead"), "cargo")),
            "a variable that is a path is read as one whole value, space and all",
        );
        assert_eq!(
            named("MAVEN_OPTS"),
            None,
            "and a line of flags with that path inside it is left out, rather \
             than handed to a tool that would split it at the space",
        );
        assert_eq!(
            named("GRADLE_USER_HOME"),
            Some(under(Path::new("/home/Jo Doe/.cache/verkstead"), "gradle")),
            "so Gradle, whose home is a variable of its own, is shared on such a \
             machine where Maven is not",
        );
        assert_eq!(
            named("GRADLE_OPTS").as_deref(),
            Some("-Dorg.gradle.daemon=false"),
            "and its daemon is off there too: a line of words naming no directory \
             has nothing in it to split",
        );

        // And the same line with no space to split on is given as ever.
        assert!(
            built_in()
                .given(&machine(true))
                .env()
                .iter()
                .any(|(name, _)| name == "MAVEN_OPTS")
        );
    }

    /// And a value naming none of them is left exactly as the file wrote it,
    /// separators and all: what makes a value a path is naming a directory, and
    /// a size is not one.
    #[test]
    fn a_value_naming_no_directory_is_left_as_the_file_wrote_it() {
        let flagged = written(
            "languages:\n  java:\n    env:\n      JAVA_TOOL_OPTIONS: \"-Dhttp.proxy=http://x/y\"\n      \
             JAVA_LIMIT: \"{size}\"\n",
        );

        assert_eq!(
            flagged.given(&machine(true)).env(),
            [
                (
                    String::from("JAVA_TOOL_OPTIONS"),
                    String::from("-Dhttp.proxy=http://x/y")
                ),
                (String::from("JAVA_LIMIT"), String::from(DEFAULT_SIZE)),
            ],
            "a value with no directory in it is text, and nothing here reads it \
             as a path",
        );
    }

    /// Both placeholders at once are both directories, in the placeholders' own
    /// order however the descriptors are written.
    #[test]
    fn a_language_naming_both_is_opened_onto_both() {
        let languages = written(
            "languages:\n  python:\n    env:\n      UV_LINK_DIR: \"{stores}/uv\"\n      \
             UV_CACHE_DIR: \"{cache}/uv\"\n",
        );

        assert_eq!(
            languages.given(&machine(true)).dirs(),
            [
                PathBuf::from("/var/cache/verkstead"),
                PathBuf::from("/var/lib/verkstead/stores"),
            ],
        );
    }

    /// A capability this server has never heard of is a language that still
    /// gets its own variables — a descriptor written for a newer Verkstead
    /// rather than a file that will not load.
    #[test]
    fn a_capability_this_server_has_not_got_contributes_nothing() {
        let languages = written(
            "languages:\n  java:\n    env:\n      GRADLE_USER_HOME: \"{cache}/gradle\"\n    \
             capabilities:\n      daemon:\n        env:\n          GRADLE_OPTS: \"-Dx\"\n",
        );

        let given = languages.given(&machine(true));

        assert_eq!(
            given.env(),
            [(String::from("GRADLE_USER_HOME"), cached("gradle"))],
        );
        assert!(
            languages.wanting(SCCACHE).is_none(),
            "and it is not what starts a Compile Server"
        );
    }

    /// Detection is a manifest at the root of a checkout and nothing deeper.
    #[test]
    fn a_repo_is_detected_by_a_manifest_at_its_root() {
        let dir = tempfile::tempdir().unwrap();
        let rust = built_in().get(RUST).unwrap();

        assert!(
            !rust.detected(dir.path()),
            "an empty directory builds nothing"
        );

        std::fs::create_dir(dir.path().join("crates")).unwrap();
        std::fs::write(dir.path().join("crates/Cargo.toml"), "[package]\n").unwrap();

        assert!(
            !rust.detected(dir.path()),
            "a manifest somewhere underneath is not the root's"
        );

        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();

        assert!(rust.detected(dir.path()));
    }

    /// A variable the Sandbox sets itself is refused, and what is left is the
    /// descriptor Verkstead ships — the cache the installer already had.
    #[test]
    fn an_entry_naming_a_variable_the_sandbox_sets_falls_back_to_the_built_in() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    env:\n      CARGO_HOME: \"{cache}/mine\"\n      \
             RUSTUP_HOME: \"{cache}/toolchains\"\n  gleam:\n    label: Gleam\n    env:\n      \
             GLEAM_CACHE: \"{cache}/gleam\"\n",
        ));

        let rust = loaded.get(RUST).unwrap();

        assert_eq!(
            rust.unread(),
            Some("sets RUSTUP_HOME, which is a variable the Sandbox sets itself"),
            "the reason names the variable, because that is what there is to fix",
        );
        let given = loaded.given(&machine(true));
        let (rusts, gleams) = given
            .env()
            .split_at(built_in().given(&machine(true)).env().len());

        assert_eq!(
            rusts,
            built_in().given(&machine(true)).env(),
            "and nothing else in the entry landed either — `CARGO_HOME` included: \
             a session has exactly the variables it had before that file was written",
        );
        assert_eq!(
            gleams,
            [(String::from("GLEAM_CACHE"), cached("gleam"))],
            "while the language beside it is given what its own entry says",
        );
        assert!(
            rust.enabled(),
            "the language is still on, running on the built-in"
        );

        let gleam = loaded
            .get("gleam")
            .expect("every other language still loads");

        assert_eq!(gleam.unread(), None);
        assert_eq!(gleam.label(), Some("Gleam"));
    }

    /// The name is refused whatever case it is written in, because Windows
    /// reads its environment that way.
    #[test]
    fn a_refused_name_is_refused_however_it_is_spelled() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    env:\n      Path: /opt/bin\n",
        ));

        assert_eq!(
            loaded.get(RUST).unwrap().unread(),
            Some("sets Path, which is a variable the Sandbox sets itself"),
        );
    }

    /// And so is one of the numbered pair git's configuration goes into.
    #[test]
    fn the_numbered_names_git_is_configured_through_are_refused_too() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    env:\n      GIT_CONFIG_KEY_0: user.name\n",
        ));

        assert_eq!(
            loaded.get(RUST).unwrap().unread(),
            Some("sets GIT_CONFIG_KEY_0, which is a variable the Sandbox sets itself"),
        );
    }

    /// A capability's variables are asked the same question as a descriptor's
    /// own, because they reach the same environment.
    #[test]
    fn a_capabilitys_variables_are_refused_by_the_same_names() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    capabilities:\n      sccache:\n        env:\n          \
             HOME: /tmp\n",
        ));

        assert_eq!(
            loaded.get(RUST).unwrap().unread(),
            Some("sets HOME, which is a variable the Sandbox sets itself"),
        );
    }

    /// An entry nothing can parse goes the same way, and the file around it
    /// still loads.
    #[test]
    fn an_entry_that_will_not_parse_falls_back_the_same_way() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    detect: 7\n  gleam:\n    label: Gleam\n",
        ));

        let why = loaded
            .get(RUST)
            .unwrap()
            .unread()
            .expect("an entry that will not parse is one that was not used");

        assert!(
            why.starts_with("could not be read:"),
            "the reason is what the reader said: {why}",
        );
        assert_eq!(
            loaded.given(&machine(true)).env(),
            built_in().given(&machine(true)).env(),
        );
        assert!(
            loaded
                .get("gleam")
                .is_some_and(|gleam| gleam.unread().is_none()),
            "and every other language still loads",
        );
    }

    /// And a misspelled key is one of the ways nothing can parse it, rather
    /// than a key skipped in silence.
    ///
    /// The likeliest thing to be wrong with a hand-written descriptor, and the
    /// reason names it — so what the page says is what there is to fix, the way
    /// it is for a refused variable.
    #[test]
    fn a_key_this_grammar_does_not_have_is_refused_and_named() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    detct:\n      - Cargo.toml\n",
        ));

        let why = loaded
            .get(RUST)
            .unwrap()
            .unread()
            .expect("a key nothing here knows is an entry that was not used");

        assert!(
            why.starts_with("could not be read:") && why.contains("detct"),
            "the reason names the key, because that is what there is to fix: {why}",
        );
        assert_eq!(
            loaded.given(&machine(true)).env(),
            built_in().given(&machine(true)).env(),
            "and the language runs on the descriptor Verkstead ships",
        );
    }

    /// And so is one inside a capability, that being the same grammar again.
    #[test]
    fn a_key_a_capability_does_not_have_is_refused_too() {
        let loaded = built_in().merged(&written(
            "languages:\n  rust:\n    capabilities:\n      sccache:\n        \
             envs:\n          SCCACHE_DIR: /tmp\n",
        ));

        assert!(
            loaded
                .get(RUST)
                .unwrap()
                .unread()
                .is_some_and(|why| why.contains("envs")),
            "a variable written under the wrong key is a variable no session gets",
        );
    }

    /// A key with nothing under it is an entry saying nothing on purpose rather
    /// than one that will not load.
    #[test]
    fn an_entry_with_nothing_under_it_is_an_entry_saying_nothing() {
        let loaded = built_in().merged(&written("languages:\n  rust:\n"));

        assert_eq!(loaded.get(RUST).unwrap().unread(), None);
        assert_eq!(
            loaded.given(&machine(true)).env(),
            built_in().given(&machine(true)).env(),
        );
    }

    /// And where there is no built-in to fall back to, the language is off
    /// rather than on at nothing.
    #[test]
    fn an_entry_with_no_built_in_behind_it_that_will_not_load_is_off() {
        let loaded = built_in().merged(&written(
            "languages:\n  gleam:\n    label: Gleam\n    env:\n      PATH: /opt/gleam/bin\n",
        ));

        let gleam = loaded.get("gleam").expect("the entry is still on the list");

        assert!(
            !gleam.enabled(),
            "on at nothing would be a box that turns on a language that is not there",
        );
        assert_eq!(
            gleam.unread(),
            Some("sets PATH, which is a variable the Sandbox sets itself"),
        );
        assert!(
            loaded.given(&machine(true)).env() == built_in().given(&machine(true)).env(),
            "and a session gets nothing of it",
        );
    }

    /// `rust_build_cache` is a key of its own, so a refused `languages` entry
    /// does not take the switch and the size down with it.
    #[test]
    fn a_refused_entry_does_not_take_the_key_it_replaced_with_it() {
        let config = crate::settings::Config::read(
            "rust_build_cache:\n  enabled: false\n  size: 5G\nlanguages:\n  rust:\n    \
             env:\n      PATH: /opt/bin\n",
        )
        .unwrap();

        let rust = configured(&config).get(RUST).unwrap().clone();

        assert!(
            !rust.enabled(),
            "the old key is still read as Rust's switch"
        );
        assert_eq!(rust.size_configured(), Some("5G"));
        assert!(rust.unread().is_some());
    }

    /// The order a file is written in is the order a session's environment is
    /// set in, whatever the keys sort as.
    #[test]
    fn a_mapping_is_kept_in_the_order_it_was_written() {
        let languages = written(
            "languages:\n  zed:\n    env:\n      ZZZ: one\n      AAA: two\n  abel:\n    \
             env:\n      MMM: three\n",
        );

        assert_eq!(
            languages
                .given(&machine(true))
                .env()
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["ZZZ", "AAA", "MMM"],
        );
    }

    /// Each built-in's stores by name, as directories of this machine.
    fn stores_of(languages: &Languages, name: &str) -> Vec<(String, String)> {
        languages
            .get(name)
            .unwrap()
            .stores(&machine(true))
            .into_iter()
            .map(|(store, dir)| (store, dir.display().to_string()))
            .collect()
    }

    /// Every built-in names the directories that are its store, and C/C++,
    /// whose objects are the Compile Server's, names none.
    #[test]
    fn every_built_in_names_its_own_stores() {
        let named = |pairs: &[(&str, String)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(name, dir)| ((*name).to_owned(), dir.clone()))
                .collect()
        };

        assert_eq!(
            stores_of(built_in(), RUST),
            named(&[("cargo", cached("cargo")), ("sccache", cached("sccache"))]),
        );
        assert_eq!(
            stores_of(built_in(), GO),
            named(&[("modules", cached("go/mod")), ("build", cached("go/build"))]),
        );
        assert_eq!(
            stores_of(built_in(), NODE),
            named(&[
                ("npm", cached("npm")),
                ("pnpm", stored("pnpm")),
                ("pnpm-metadata", cached("pnpm/metadata")),
                ("yarn", cached("yarn/cache")),
                ("yarn-berry", cached("yarn/global")),
                ("deno", stored("deno")),
                ("bun", stored("bun")),
            ]),
        );
        assert_eq!(
            stores_of(built_in(), PYTHON),
            named(&[
                ("pip", cached("pip")),
                ("uv", stored("uv")),
                ("poetry", cached("poetry")),
                ("pipenv", cached("pipenv")),
            ]),
        );
        assert_eq!(
            stores_of(built_in(), DOTNET),
            named(&[
                ("packages", cached("nuget/packages")),
                ("http", cached("nuget/http")),
                ("scratch", cached("nuget/scratch")),
            ]),
        );
        assert_eq!(
            stores_of(built_in(), JVM),
            named(&[
                ("maven", cached("maven/repository")),
                ("gradle", cached("gradle")),
            ]),
        );
        assert!(
            stores_of(built_in(), CPP).is_empty(),
            "C/C++'s objects are the one Compile Server's, which is Rust's store",
        );
    }

    /// And every store a built-in names is a directory one of its own variables
    /// already points a tool at, so the list cannot drift from what is given.
    #[test]
    fn every_built_in_store_is_a_directory_its_variables_name() {
        let given = built_in().given(&machine(true));

        for (language, descriptor) in built_in().iter() {
            for (store, dir) in descriptor.stores(&machine(true)) {
                let dir = dir.display().to_string();

                assert!(
                    given.env().iter().any(|(_, value)| value.contains(&dir)),
                    "{language}'s store {store}, {dir}, is named by no variable",
                );
            }
        }
    }

    /// An installer adds a store by naming it and takes one out with a `null`,
    /// key by key, and the rest of the built-in's stay as they were.
    #[test]
    fn an_installer_adds_a_store_or_takes_one_out_key_by_key() {
        let loaded = built_in().merged(&written(
            "languages:\n  go:\n    stores:\n      build: null\n      vendor:\n        \
             dir: \"{cache}/go/vendor\"\n",
        ));

        assert_eq!(
            stores_of(&loaded, GO),
            [
                (String::from("modules"), cached("go/mod")),
                (String::from("vendor"), cached("go/vendor")),
            ],
        );
        assert_eq!(
            loaded.given(&machine(true)).env(),
            built_in().given(&machine(true)).env(),
            "and what a session is given is the variables' business, not the stores'",
        );
    }

    /// A descriptor naming no store is a descriptor all the same: its variables
    /// are given, and there is nothing of it to measure.
    #[test]
    fn a_descriptor_naming_no_store_is_still_one() {
        let loaded =
            written("languages:\n  gleam:\n    env:\n      GLEAM_CACHE: \"{cache}/gleam\"\n");
        let gleam = loaded.get("gleam").unwrap();

        assert_eq!(gleam.unread(), None);
        assert!(!gleam.names_a_store());
        assert!(
            gleam.has_store(),
            "it still has a size: its variable is in the Build Cache"
        );
        assert_eq!(
            loaded.given(&machine(true)).env(),
            [(String::from("GLEAM_CACHE"), cached("gleam"))],
        );
    }

    /// A store anywhere but under `{cache}` or `{stores}` is refused, the entry
    /// falling back to the built-in: what is done to a store is delete what is
    /// in it.
    #[test]
    fn a_store_outside_verksteads_own_directories_is_refused() {
        for dir in [
            "/home/someone",
            "{cache}",
            "{cache}/",
            "{stores}/../worktrees",
            "{cache}/go//mod",
            "{cache}/{stores}/x",
            "~/go/{cache}",
        ] {
            let loaded = built_in().merged(&written(&format!(
                "languages:\n  go:\n    stores:\n      modules:\n        dir: \"{dir}\"\n",
            )));

            let why = loaded
                .get(GO)
                .unwrap()
                .unread()
                .unwrap_or_else(|| panic!("{dir} was taken as a store"));

            assert!(
                why.starts_with(&format!("names {dir} as its store modules")),
                "the reason names the directory: {why}",
            );
            assert_eq!(
                stores_of(&loaded, GO),
                stores_of(built_in(), GO),
                "and the language runs on the built-in's stores",
            );
        }
    }

    /// How each built-in store is bounded, by its name: `tool`, `unit` or
    /// `none`.
    fn bounded_of(languages: &Languages, name: &str) -> Vec<(String, &'static str)> {
        let descriptor = languages.get(name).unwrap();

        descriptor
            .stores(&machine(true))
            .into_iter()
            .map(|(store, _)| {
                let said = match descriptor.bounded(&store) {
                    Bounded::ByItsTool => "tool",
                    Bounded::ByUnit(_) => "unit",
                    Bounded::NotAtAll => "none",
                };

                (store, said)
            })
            .collect()
    }

    /// Every built-in store says how it is bounded: sccache by its tool, the
    /// cargo half beside it by unit, NuGet's scratch directory of locks not at
    /// all, and every other store by unit.
    #[test]
    fn every_built_in_store_is_bounded_one_of_three_ways() {
        let said = |pairs: &[(&str, &'static str)]| -> Vec<(String, &'static str)> {
            pairs
                .iter()
                .map(|(name, how)| ((*name).to_owned(), *how))
                .collect()
        };

        assert_eq!(
            bounded_of(built_in(), RUST),
            said(&[("cargo", "unit"), ("sccache", "tool")]),
        );
        assert_eq!(
            bounded_of(built_in(), DOTNET),
            said(&[("packages", "unit"), ("http", "unit"), ("scratch", "none")]),
        );

        for language in [GO, NODE, PYTHON, JVM] {
            assert!(
                bounded_of(built_in(), language)
                    .iter()
                    .all(|(_, how)| *how == "unit"),
                "every one of {language}'s stores is swept by unit",
            );
        }
    }

    /// The size reaches the one tool that evicts for itself as a variable of
    /// its own, which is what makes `by-its-tool` true of sccache's store.
    #[test]
    fn the_store_its_tool_evicts_is_handed_the_size() {
        let given = built_in().given(&machine(true));

        assert!(
            given
                .env()
                .iter()
                .any(|(name, value)| name == "SCCACHE_CACHE_SIZE" && value == "30G"),
        );
    }

    /// An installer's store naming no unit is never swept, and an override's
    /// `units: []` takes a built-in's away; one naming units replaces the
    /// built-in's whole.
    #[test]
    fn a_store_naming_no_unit_is_not_bounded_at_all() {
        let loaded = built_in().merged(&written(
            "languages:\n  go:\n    stores:\n      build:\n        units: []\n      \
             modules:\n        units:\n          - depth: 1\n  gleam:\n    stores:\n      \
             cache:\n        dir: \"{cache}/gleam\"\n",
        ));

        assert_eq!(
            bounded_of(&loaded, GO),
            [
                (String::from("modules"), "unit"),
                (String::from("build"), "none"),
            ],
        );
        assert_eq!(
            loaded.get(GO).unwrap().bounded("modules"),
            Bounded::ByUnit(&[Unit {
                depth: Some(1),
                ..Unit::default()
            }]),
        );
        assert_eq!(
            bounded_of(&loaded, "gleam"),
            [(String::from("cache"), "none")]
        );
        assert_eq!(
            loaded.get(GO).unwrap().bounded("nothing-of-that-name"),
            Bounded::NotAtAll
        );
    }

    /// A unit that would match anything, one at the store itself, one under a
    /// path that leaves the store, a pattern that is a path, and a store both
    /// evicted by its tool and swept — each refused, the entry falling back.
    #[test]
    fn a_unit_the_sweep_could_not_follow_is_refused() {
        for (store, why) in [
            (
                "units:\n          - under: src",
                "none of depth, named or holding",
            ),
            ("units:\n          - depth: 0", "depth 0"),
            (
                "units:\n          - under: ../x\n            depth: 1",
                "under ../x",
            ),
            (
                "units:\n          - under: \"{cache}\"\n            depth: 1",
                "under {cache}",
            ),
            ("units:\n          - named: [\"a/*\"]", "`a/*`"),
            ("units:\n          - holding: [\"\"]", "``"),
            (
                "evicted: by-its-tool\n        units:\n          - depth: 1",
                "one or the other",
            ),
        ] {
            let loaded = built_in().merged(&written(&format!(
                "languages:\n  go:\n    stores:\n      modules:\n        {store}\n",
            )));

            let read = loaded
                .get(GO)
                .unwrap()
                .unread()
                .unwrap_or_else(|| panic!("{store} was taken"));

            assert!(
                read.starts_with("names its store modules") && read.contains(why),
                "{store}: {read}",
            );
            assert_eq!(bounded_of(&loaded, GO), bounded_of(built_in(), GO));
        }
    }

    /// A pattern's `*` is any run of characters and `?` any one.
    #[test]
    fn a_pattern_is_a_name_with_stars_and_question_marks() {
        for (pattern, name, is) in [
            ("*@*", "greet@v1.0.0", true),
            ("*@*", "@v", true),
            ("*@*", "greet", false),
            ("*@*@@@*", "greet@1.0.0@@@1", true),
            ("*@*@@@*", "1.0.0@@@1", false),
            ("*.pom", "greet-1.0.0.pom", true),
            ("*.pom", "greet-1.0.0.pom.sha1", false),
            ("npm-*", "npm-greet-1.0.0-integrity", true),
            ("?.dat", "a.dat", true),
            ("?.dat", "ab.dat", false),
            ("registry.json", "registry.json", true),
            ("*", "", true),
            ("a*b*c", "aXbYbZc", true),
            ("a*b*c", "aXbYbZ", false),
        ] {
            assert_eq!(matches(pattern, name), is, "{pattern} against {name}");
        }
    }

    /// And a key a store does not have is refused like any other.
    #[test]
    fn a_key_a_store_does_not_have_is_refused_too() {
        let loaded = built_in().merged(&written(
            "languages:\n  go:\n    stores:\n      modules:\n        path: \"{cache}/x\"\n",
        ));

        assert!(
            loaded
                .get(GO)
                .unwrap()
                .unread()
                .is_some_and(|why| why.contains("path")),
        );
    }
}
