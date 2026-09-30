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

            let size = descriptor.size();

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
                .map(|(_, descriptor)| descriptor)
                .filter(|descriptor| descriptor.names(capability))
        };

        let sizer = naming().next()?;

        naming().any(Descriptor::enabled).then(|| sizer.size())
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
            Ok(descriptor) => match descriptor.refuses() {
                Some(name) => {
                    format!("sets {name}, which is a variable the Sandbox sets itself")
                }
                None => return descriptor,
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
    /// placeholder's value — [`crate::build_cache::SIZE`] where nobody has
    /// said.
    ///
    /// The human's own word rather than a number of bytes, and nothing here
    /// parses it: what it reaches is the tool that reads it, and a parser here
    /// would be a second opinion about the one thing the value is for.
    pub fn size(&self) -> &str {
        self.size.as_deref().unwrap_or(crate::build_cache::SIZE)
    }

    /// The size exactly as it is written down, and `None` where nobody has
    /// written one: what the settings page draws as a placeholder rather than
    /// as a value somebody chose.
    pub fn size_configured(&self) -> Option<&str> {
        self.size.as_deref()
    }

    /// Whether this descriptor names `capability`, whatever this machine can
    /// offer — which is what says the settings page has a size to draw under
    /// its box, and what [`Languages::wanting`] asks of a switched-on one.
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

    /// `value` with its placeholders filled in, or `None` where one of them
    /// names something this machine has not got.
    ///
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
        assert_eq!(jvm.detect, vec![String::from("pom.xml")]);
        assert!(
            !jvm.names(SCCACHE),
            "and nothing on the JVM compiles through sccache",
        );
    }

    /// A session of a machine with an sccache: Rust's four variables, in the
    /// order it has always had them, then Go's two, Node's seven, Python's
    /// six, .NET's three, C/C++'s two launchers and the JVM's one, and the two
    /// directories they name open underneath.
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
            ],
            "Go's two, Node's seven, Python's six, .NET's three and the JVM's \
             one are in no capability, so a machine with no sccache gets the \
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
            19,
            "which is Go's two, Node's seven, Python's six, .NET's three and the \
             JVM's one, and nothing else"
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
                (
                    String::from("JAVA_LIMIT"),
                    String::from(crate::build_cache::SIZE)
                ),
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
}
