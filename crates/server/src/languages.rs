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
//! setup card's warning alone — see [`crate::build_cache::builds_rust`], which
//! is its one caller.
//!
//! **Placeholders are what only the server knows** — see [`Machine`], and the
//! embedded file, which is where each of the four is spelled out. A
//! placeholder's directory is granted to a session only where a loaded
//! descriptor names it, so the one beside the Worktrees grants nothing while
//! Rust is the only language built in.

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

/// Rust, which is the only language built in — and the name `rust_build_cache`
/// is still read as the settings of.
pub const RUST: &str = "rust";

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
/// An `expect` because the file is embedded: it cannot vary between machines,
/// so a file that will not parse is a build that should never have shipped, and
/// the suite reads it — see `the_built_ins_are_in_the_grammar_an_installer_writes`.
pub fn built_in() -> &'static Languages {
    static BUILT_IN: LazyLock<Languages> = LazyLock::new(|| {
        Languages::read(BUILT_IN_YAML)
            .expect("the descriptors embedded in this binary are in the grammar the loader reads")
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
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
    pub fn merged(&self, over: &Languages) -> Languages {
        Languages(self.0.merged(&over.0, Descriptor::merged))
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

    /// The store size asked for by the first switched-on descriptor naming
    /// `capability`, and `None` where none of them does — which is what says
    /// whether the behaviour behind it is wanted on this machine at all.
    ///
    /// The size comes back with the answer because the one capability there is
    /// wants one: the Compile Server is started with a size, and that size is a
    /// switched-on language's rather than a number the server holds. Where two
    /// of them name it — C++ beside Rust, when it comes — the first written is
    /// the one that sizes the store, because there is one store and one server
    /// for the machine.
    ///
    /// Asked of the switch rather than of a Repo. The Compile Server comes up
    /// wherever a language naming [`SCCACHE`] is enabled and there is an sccache
    /// to run, whatever the checkout holds: a Repo whose manifest is not at its
    /// root is handed the wrapper variable all the same, and with no server of
    /// Verkstead's up the client inside starts one in its own Sandbox — which
    /// is the hazard the Compile Server exists to remove.
    pub fn wanting(&self, capability: &str) -> Option<&str> {
        self.iter().find_map(|(_, descriptor)| {
            let names = descriptor
                .capabilities
                .iter()
                .any(|(named, _)| named == capability);

            (descriptor.enabled() && names).then(|| descriptor.size())
        })
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
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
}

impl Descriptor {
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
            env: self.env.merged(&over.env, |_, over| over.clone()),
            // And a capability in both is merged in its turn rather than
            // replaced, so that overriding one of the sccache's three variables
            // keeps the other two.
            capabilities: self
                .capabilities
                .merged(&over.capabilities, Capability::merged),
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
    /// [`Languages::naming`].
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
/// `RUSTC_WRAPPER`; C++'s, when it comes, is CMake's two launcher variables
/// pointed at the same binary — one capability, two descriptors, and nothing in
/// the server that knows which language asked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Capability {
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    env: Ordered<Option<String>>,
}

impl Capability {
    /// `over` written over this, which is its variables merged the way a
    /// descriptor's own are — see [`Descriptor::merged`].
    fn merged(&self, over: &Capability) -> Capability {
        Capability {
            env: self.env.merged(&over.env, |_, over| over.clone()),
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
    /// A variable that cannot be filled is left out rather than written with the
    /// placeholder still in it: what a `RUSTC_WRAPPER` naming `{sccache}`
    /// literally would do is fail every build inside.
    fn filled(&self, value: &str, size: &str, used: &mut Used) -> Option<String> {
        let mut filled = value.to_owned();

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

/// The directory beside the Worktrees, under `data`.
///
/// Named here for the reason [`crate::worktrees::directory`] is named there:
/// what makes it the right place is what is *next to* it, so one function says
/// where it is and the descriptors point at it by placeholder.
pub(crate) fn stores(data: &Path) -> PathBuf {
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
    /// makes of the pair, and a key in only one of them is carried as it
    /// stands.
    ///
    /// The order is this one's, with whatever `over` adds on the end — which is
    /// what keeps a session's environment in the built-in file's order however
    /// an installer's own entries are written.
    fn merged(&self, over: &Ordered<V>, merging: impl Fn(&V, &V) -> V) -> Ordered<V> {
        let mut merged = self.0.clone();

        for (key, value) in over.iter() {
            match merged.iter_mut().find(|(name, _)| name == key) {
                Some((_, held)) => *held = merging(held, value),
                None => merged.push((key.to_owned(), value.clone())),
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

    /// The built-ins parse, and what Rust's says is what a session gets today.
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
    }

    /// A session of a machine with an sccache: the four variables it has always
    /// had, in the order it has always had them, and the cache open underneath.
    #[test]
    fn a_session_is_given_what_rust_has_always_been_given() {
        let given = built_in().given(&machine(true));

        assert_eq!(
            given.env(),
            [
                (
                    String::from("CARGO_HOME"),
                    String::from("/var/cache/verkstead/cargo")
                ),
                (
                    String::from("RUSTC_WRAPPER"),
                    String::from("/verkstead/bin/sccache")
                ),
                (
                    String::from("SCCACHE_DIR"),
                    String::from("/var/cache/verkstead/sccache")
                ),
                (String::from("SCCACHE_CACHE_SIZE"), String::from("30G")),
            ],
        );

        assert_eq!(
            given.dirs(),
            [PathBuf::from("/var/cache/verkstead")],
            "the Build Cache, because Rust's descriptor names it"
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
            [(
                String::from("CARGO_HOME"),
                String::from("/var/cache/verkstead/cargo")
            )],
        );
        assert_eq!(given.dirs(), [PathBuf::from("/var/cache/verkstead")]);
        assert!(!given.sccache());
    }

    /// A language switched off is no variables and no directory: the switch
    /// closes the hole rather than leaving it open and unused.
    #[test]
    fn a_language_switched_off_gives_a_session_nothing() {
        let off = built_in().merged(&written("languages:\n  rust:\n    enabled: false\n"));
        let given = off.given(&machine(true));

        assert!(given.is_empty());
        assert!(given.env().is_empty());
        assert!(given.dirs().is_empty());
        assert!(!given.sccache());
        assert!(
            off.wanting(SCCACHE).is_none(),
            "and nothing wants a Compile Server up"
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

    /// The second placeholder is in the grammar and grants nothing while no
    /// loaded descriptor names it — which is the whole of what it costs a
    /// machine that only builds Rust.
    #[test]
    fn the_directory_beside_the_worktrees_is_granted_only_where_it_is_named() {
        assert!(
            !built_in().names_stores(),
            "Rust's store is under the Build Cache"
        );

        let hardlinking = written(
            "languages:\n  node:\n    label: Node\n    env:\n      \
             PNPM_HOME: \"{stores}/pnpm\"\n",
        );

        assert!(hardlinking.names_stores());

        let given = hardlinking.given(&machine(true));

        assert_eq!(
            given.env(),
            [(
                String::from("PNPM_HOME"),
                String::from("/var/lib/verkstead/stores/pnpm")
            )],
        );
        assert_eq!(
            given.dirs(),
            [PathBuf::from("/var/lib/verkstead/stores")],
            "beside the Worktrees, because a hardlink out of a store does not \
             cross a filesystem"
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
            [(
                String::from("GRADLE_USER_HOME"),
                String::from("/var/cache/verkstead/gradle")
            )],
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
