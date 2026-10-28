//! **Authors as records** (ADR-0028): every Author is a record of the
//! database's replicated table, the same uuid on every device, rather than an
//! entry in `config.yaml`.
//!
//! A record holds what is drawn — the display name, the git name, the email —
//! and whether a token is held for it, with a digest saying which: the fact,
//! never the token, which stays in each device's `secrets.yaml` under the uuid.
//!
//! **Taken up out of `config.yaml` once**, at the serve, after the single
//! `git_author` has been moved into the list — see [`taken_up`]. The uuids the
//! file carried are the records', so nothing a Repo records changes; the key
//! goes out of the file, and after that the file is not read for Authors.
//!
//! **Held in memory for the settings to resolve out of**: an Author is
//! resolved on blocking threads from the files alone, so every write here, and
//! every merge of a member's records, hands the live records on to
//! [`Settings::holding_authors`] once it is written.
//!
//! **Replicated across the cluster** on the facility's route and puller — see
//! [`super`]. Nothing of the token crosses there: the record says whether one
//! is held and which, and the token is copied on a call of its own — see
//! [`copied`].
//!
//! **The token travels from whoever holds one.** A token set or cleared is a
//! write to the record — `holds_token` and the digest, and the stamp — which is
//! what tells the other devices to fetch: after every pull, a device whose
//! `secrets.yaml` holds no copy of a token the record says is held, or a copy
//! whose digest is not the record's, asks its members in turn for it, the
//! writer first, and takes the first answer that is the token the record names.
//! A record whose token was cleared, or that was removed, takes the copy away.
//! A member not answering leaves the copy as it stands, and it is used
//! (ADR-0025).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::{Mutex, MutexGuard};

use verkstead_render::{AuthorEntry, AuthorToken};
use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::peer::authors::TOKEN;
use crate::relaying::{Read, asking, word_from};
use crate::settings::{Author, Recorded, Settings};
use crate::store::replicated::{self, Kind, Written};

/// The Authors, as a kind of record.
pub struct Authors;

impl Kind for Authors {
    const NAME: &'static str = "authors";
    type Fields = Fields;
}

impl Replicating for Authors {
    const MOVED: Nudge = Nudge::Authors;
    const READ: Read = Read::Authors;

    /// The merged records held for the settings — taken up first, where a
    /// server stood up without a serve has not — and the pages told.
    async fn merged(state: &AppState) -> Result<()> {
        taken_up(&state.pool, &state.settings, &state.device).await?;
        held(&state.pool, &state.settings, &state.device).await?;

        state.nudges.announce_here(Nudge::Authors);

        Ok(())
    }

    /// Every token the records say is held and this device has no current
    /// copy of fetched, and every copy of one cleared or removed taken away.
    async fn pulled(state: &AppState) -> Result<()> {
        copied(state).await
    }
}

/// The most a member's answer about one token may be: **sixty-four
/// kilobytes**, the login's bound. A GitHub token is under a hundred bytes.
const MOST_A_TOKEN_IS: usize = 64 * 1024;

/// What is held while an Author's token and its record are written together,
/// so that the copying never reads one written and the other not — a token
/// just set here, read before its record says so, would be taken for a copy
/// whose token was cleared.
///
/// One for the process rather than one per Data Directory: it is held for two
/// file writes and a row, never across a call to a member, and two servers in
/// one process — which is a suite — only take turns.
static WRITING: Mutex<()> = Mutex::const_new(());

/// Hold [`WRITING`], from the runtime.
pub(crate) async fn writing_held() -> MutexGuard<'static, ()> {
    WRITING.lock().await
}

/// And from a blocking thread, which is where the settings save writes its two
/// files.
pub(crate) fn writing_held_blocking() -> MutexGuard<'static, ()> {
    WRITING.blocking_lock()
}

/// Which token `token` is, as a record says it: a SHA-256 of it in hex, so a
/// device can tell whether its copy is the one the record names without the
/// token itself crossing on the record.
pub fn digest(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The one list as the Git section and the pickers draw it: every live Author,
/// in the order they were first written down here, and nothing of the token
/// but whether this device's `secrets.yaml` holds one.
pub(crate) fn listed(settings: &Settings) -> Vec<AuthorEntry> {
    let secrets = settings.secrets();

    settings
        .config()
        .authors()
        .iter()
        .map(|author| AuthorEntry {
            id: author.id().to_owned(),
            display_name: author.display_name().unwrap_or_default().to_owned(),
            name: author.name().unwrap_or_default().to_owned(),
            email: author.email().unwrap_or_default().to_owned(),
            holds_token: secrets.author_token(author.id()).is_some(),
            device: None,
        })
        .collect()
}

/// What an Author's record holds besides the record's own stamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    #[serde(default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub email: Option<String>,

    /// Whether a token is held for it: set where one was set, on whichever
    /// device, and cleared where one was cleared — and carried as it stands
    /// through an edit that said nothing about the token, which a device
    /// with no copy of it can make.
    #[serde(default)]
    pub holds_token: bool,

    /// Which token that is — see [`digest`] — so a rotation is a write to the
    /// record, and a copy of the token it replaced is known for one. Absent on
    /// a record written before there was one, whose copy is taken as current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_digest: Option<String>,
}

impl Fields {
    fn of(author: &Author, token: Option<&str>) -> Fields {
        Fields {
            display_name: author.display_name().map(str::to_owned),
            name: author.name().map(str::to_owned),
            email: author.email().map(str::to_owned),
            holds_token: token.is_some(),
            token_digest: token.map(digest),
        }
    }

    /// And with the token as `held` says it, rather than as a file does.
    fn keeping_the_token_of(self, held: &Fields) -> Fields {
        Fields {
            holds_token: held.holds_token,
            token_digest: held.token_digest.clone(),
            ..self
        }
    }

    /// Whether `copy`, a token this device holds for it, is the one it names.
    fn names(&self, copy: &str) -> bool {
        self.token_digest
            .as_deref()
            .is_none_or(|named| named == digest(copy))
    }

    fn author(&self, uuid: &str) -> Author {
        Author::of(
            uuid.to_owned(),
            self.display_name.clone(),
            self.name.clone(),
            self.email.clone(),
        )
    }
}

/// Take the Authors up out of `config.yaml` into the table, where they have not
/// been already, and hold the table's live records for the settings either
/// way.
///
/// **Once.** The table saying the kind has been taken up is what says the file
/// is never read for Authors again: an `authors:` key written into it by hand
/// afterwards is ignored, and [`Settings::config`] says so in the log. The
/// database first and the file second, so that stopping between the two leaves
/// a key nothing reads rather than Authors nobody holds.
///
/// **A file that will not parse is left alone and nothing taken up**, its
/// Authors taken up at the first start after the human fixes it.
///
/// Called at the serve before anything reads an Author, and at the head of
/// every write of one — where it is a lookup in memory, the serve having done
/// it, and the move itself for a server stood up without one.
pub async fn taken_up(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    if settings.holds_authors() {
        return Ok(());
    }

    if !replicated::taken_up::<Authors>(pool).await? {
        let settings_ = settings.clone();
        let read = tokio::task::spawn_blocking(move || {
            let authors = settings_.authors_as_written()?;
            let secrets = settings_.secrets();

            Some(
                authors
                    .into_iter()
                    .map(|author| {
                        let token = secrets.author_token(author.id());
                        (author.id().to_owned(), Fields::of(&author, token))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .await?;

        if let Some(kept) = read {
            replicated::take_up::<Authors>(pool, device, &kept)
                .await
                .context("taking the Authors up out of config.yaml")?;

            if !kept.is_empty() {
                tracing::info!(
                    authors = kept.len(),
                    "the Authors are records in the database now, out of config.yaml",
                );
            }

            let settings = settings.clone();
            tokio::task::spawn_blocking(move || settings.dropping_the_authors())
                .await?
                .context("taking the `authors:` key out of config.yaml")?;
        }
    }

    held(pool, settings, device).await
}

/// Hand the table's live Authors on to the settings, which is what every
/// list, picker and launch resolves an Author out of — each with whether a
/// token is held for it, which is awaited here until this device's
/// `secrets.yaml` holds a copy.
pub async fn held(pool: &SqlitePool, settings: &Settings, _device: &str) -> Result<()> {
    let authors = replicated::live_records::<Authors>(pool)
        .await?
        .into_iter()
        .map(|record| {
            Recorded::of(
                record.fields.author(&record.uuid),
                record.fields.holds_token,
            )
        })
        .collect();

    settings.holding_authors(authors);

    Ok(())
}

/// Write `author` as the record under its uuid — made where there is none.
///
/// What the settings save does, for the one Author its Git section drew and
/// for the wizard's first. Where the save set or cleared the token —
/// `token_moved` — it is written into the file before this, and the record says
/// what the file does now; where it said nothing about the token, the record
/// keeps saying what it said, this device's copy or the want of one being no
/// word about the token. `Removed` where the uuid is a removed record, which is
/// final.
///
/// The caller holds [`WRITING`] across the token and this.
pub async fn written(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
    token_moved: bool,
) -> Result<Written> {
    writing(pool, settings, device, author, true, token_moved).await
}

/// And the same where only an edit is meant: `Absent` where there is no live
/// Author under the uuid, and nothing written.
pub async fn edited(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
    token_moved: bool,
) -> Result<Written> {
    writing(pool, settings, device, author, false, token_moved).await
}

async fn writing(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
    making: bool,
    token_moved: bool,
) -> Result<Written> {
    taken_up(pool, settings, device).await?;

    let token = {
        let settings = settings.clone();
        let id = author.id().to_owned();

        tokio::task::spawn_blocking(move || settings.secrets().author_token(&id).map(str::to_owned))
            .await?
    };

    let fields = Fields::of(author, token.as_deref());

    // A record already held keeps its token's word where this write has none,
    // and a new one says what this device's file holds.
    let fields = match token_moved {
        true => fields,
        false => replicated::live_records::<Authors>(pool)
            .await?
            .into_iter()
            .find(|record| record.uuid == author.id())
            .map_or(fields.clone(), |held| {
                fields.keeping_the_token_of(&held.fields)
            }),
    };

    let outcome = if making {
        replicated::write::<Authors>(pool, device, author.id(), &fields).await?
    } else {
        replicated::rewrite::<Authors>(pool, device, author.id(), &fields).await?
    };

    held(pool, settings, device).await?;

    Ok(outcome)
}

/// Remove the Author whose uuid is `id`: a deletion stamp on its record, and
/// its token out of `secrets.yaml`. `false` where there is no live Author
/// under it and nothing was written.
///
/// The record first, so that stopping between the two leaves a token under a
/// uuid nobody holds, which nothing reads, rather than an Author whose token
/// has gone. Whether anything still points at it is the caller's to ask
/// first — see [`crate::repos::remove_author`].
pub async fn removed(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    id: &str,
) -> Result<bool> {
    taken_up(pool, settings, device).await?;

    let _writing = writing_held().await;

    if !replicated::remove::<Authors>(pool, device, id).await? {
        return Ok(false);
    }

    held(pool, settings, device).await?;

    let settings = settings.clone();
    let id = id.to_owned();
    tokio::task::spawn_blocking(move || settings.setting_author_token(&id, None))
        .await?
        .context("taking a removed Author's token out of secrets.yaml")?;

    Ok(true)
}

/// Bring this device's `secrets.yaml` into line with the records: a copy
/// fetched of every token a live record says is held and this device holds no
/// copy of, or a copy of another one than the record names; and the copy taken
/// away where the record's token was cleared or the record removed. Run after
/// every pull, whatever it merged, so a copy that could not be fetched last
/// time is asked for again.
///
/// **Asked of the members in turn**, the record's writer first, and the first
/// answer that is the token the record names taken — a member holding a copy
/// of the one before is passed over. **A member not answering leaves the copy
/// as it stands**, and it is used (ADR-0025); with no copy at all the Author's
/// token is awaited — see [`crate::settings::Config::token_awaited`].
///
/// **The asking outside [`WRITING`] and the writing inside it**, each copy
/// decided again against the record and the file as they stand then: a token
/// set here between the two is this device's own, and nothing fetched or
/// cleared before it lands over it.
pub(crate) async fn copied(state: &AppState) -> Result<()> {
    let wanted = wanted(state).await?;

    if wanted.is_empty() {
        return Ok(());
    }

    let mut copies = Vec::new();

    for (record, writer) in wanted {
        let copy = match record.fields.holds_token && !record.removed() {
            false => None,
            true => match fetched(state, &record.uuid, &record.fields, &writer).await {
                Some(token) => Some(token),
                None => continue,
            },
        };

        copies.push((record.uuid, copy));
    }

    if copies.is_empty() {
        return Ok(());
    }

    let _writing = writing_held().await;

    let records = replicated::records::<Authors>(&state.pool).await?;
    let settings = state.settings.clone();

    let moved = tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();
        let mut moved = false;

        for (uuid, copy) in copies {
            let Some(record) = records.iter().find(|record| record.uuid == uuid) else {
                continue;
            };
            let held = secrets.author_token(&uuid);

            let token = match (copy, record.fields.holds_token && !record.removed()) {
                (None, false) if held.is_some() => None,
                (Some(token), true) if record.fields.names(&token) && held != Some(&*token) => {
                    Some(token)
                }
                _ => continue,
            };

            settings.setting_author_token(&uuid, token)?;
            moved = true;
        }

        Ok::<_, std::io::Error>(moved)
    })
    .await?
    .context("writing the copies of the Authors' tokens into secrets.yaml")?;

    if moved {
        state.nudges.announce_here(Nudge::Authors);
    }

    Ok(())
}

/// The records whose token this device's copy is out of line with, each with
/// the device that wrote it — removed ones included, whose copy goes.
async fn wanted(state: &AppState) -> Result<Vec<(replicated::Record<Fields>, String)>> {
    let records = replicated::records::<Authors>(&state.pool).await?;
    let settings = state.settings.clone();

    let wanted = tokio::task::spawn_blocking(move || {
        let secrets = settings.secrets();

        records
            .into_iter()
            .filter(|record| {
                let held = secrets.author_token(&record.uuid);

                match record.fields.holds_token && !record.removed() {
                    false => held.is_some(),
                    true => held.is_none_or(|copy| !record.fields.names(copy)),
                }
            })
            .map(|record| {
                let writer = record.written_by.clone();
                (record, writer)
            })
            .collect()
    })
    .await?;

    Ok(wanted)
}

/// The token named by `fields` for the Author `uuid`, from the first member
/// that holds it — `writer` first, where it is a member — or `None` where no
/// member answering does.
async fn fetched(state: &AppState, uuid: &str, fields: &Fields, writer: &str) -> Option<String> {
    let devices = state.devices.as_ref()?;

    let mut members: Vec<String> = match devices.membership().rows().await {
        Ok(members) => members.into_iter().map(|member| member.device).collect(),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no Author's token \
                 was copied",
            );
            return None;
        }
    };

    members.sort_by_key(|member| member != writer);

    for member in members {
        match token_at(Some(devices), &member, uuid).await {
            Ok(Some(token)) if fields.names(&token) => return Some(token),
            Ok(_) => {}
            Err(why) => tracing::debug!(
                device = member,
                author = uuid,
                "a member's copy of an Author's token could not be read: {}",
                why.saying,
            ),
        }
    }

    None
}

/// The token the member at `device` holds for the Author `id`, `None` where it
/// holds none — or the refusal naming the machine, which is a member not
/// answering, a caller it holds no membership for, or a uuid it holds no live
/// record of (ADR-0025).
///
/// Public for the suite that stands devices up and asks it.
pub async fn token_at(
    devices: Option<&crate::device::Devices>,
    device: &str,
    id: &str,
) -> Result<Option<String>, crate::Refusal> {
    let said: AuthorToken = word_from(
        devices,
        device,
        asking(TOKEN.replace("{author}", id)),
        MOST_A_TOKEN_IS,
    )
    .await?;

    Ok(said.token)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database and settings files in one fresh Data Directory.
    async fn fresh() -> (tempfile::TempDir, SqlitePool, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::store::open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();
        let settings = Settings::in_data_dir(dir.path());

        (dir, pool, settings)
    }

    fn author(id: &str, name: &str) -> Author {
        Author::of(id.into(), None, Some(name.into()), None)
    }

    fn ids(settings: &Settings) -> Vec<String> {
        settings
            .config()
            .authors()
            .iter()
            .map(|author| author.id().to_owned())
            .collect()
    }

    /// An installation with `authors:` in `config.yaml` starts with the same
    /// Authors under the same uuids, now out of the database, and the key gone
    /// from the file; a Repo's recorded Author still resolves.
    #[tokio::test]
    async fn the_authors_in_config_yaml_are_taken_up_under_their_uuids() {
        let (_dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            concat!(
                "authors:\n",
                "  - id: work\n",
                "    display_name: Work\n",
                "    name: Tobias Cohen\n",
                "    email: tobi@work.example\n",
                "  - id: home\n",
                "    name: Tobi\n",
                "share_on_done: true\n",
            ),
        )
        .unwrap();
        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: ghp_work\n",
        )
        .unwrap();

        taken_up(&pool, &settings, "this-device").await.unwrap();

        let records = replicated::records::<Authors>(&pool).await.unwrap();
        let taken: Vec<_> = records
            .iter()
            .map(|record| (record.uuid.as_str(), record.fields.holds_token))
            .collect();
        assert_eq!(taken, [("work", true), ("home", false)]);
        assert!(
            records
                .iter()
                .all(|record| record.written_by == "this-device")
        );
        assert_eq!(records[0].fields.display_name.as_deref(), Some("Work"));
        assert_eq!(
            records[0].fields.email.as_deref(),
            Some("tobi@work.example")
        );

        let written = std::fs::read_to_string(settings.config_path()).unwrap();
        assert!(!written.contains("authors"), "{written}");
        assert!(written.contains("share_on_done: true"), "{written}");

        assert_eq!(ids(&settings), ["work", "home"]);
        let config = settings.config();
        let resolved = config.author_of(Some("home")).unwrap();
        assert_eq!(resolved.name(), Some("Tobi"));
        assert_eq!(settings.secrets().author_token("work"), Some("ghp_work"));
    }

    /// A hand-edited `authors:` key added after the move is ignored, at the
    /// next start too: the kind has been taken up, and the file is not read
    /// for Authors again.
    #[tokio::test]
    async fn a_hand_edited_key_after_the_move_is_ignored() {
        let (dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            "authors:\n  - id: work\n    name: Ada\n",
        )
        .unwrap();
        taken_up(&pool, &settings, "this-device").await.unwrap();

        std::fs::write(
            settings.config_path(),
            "authors:\n  - id: by-hand\n    name: By Hand\n",
        )
        .unwrap();
        assert_eq!(ids(&settings), ["work"]);

        // And at the next start, whose settings hold nothing yet.
        let restarted = Settings::in_data_dir(dir.path());
        restarted.forgetting_the_authors();
        assert!(!restarted.holds_authors());
        taken_up(&pool, &restarted, "this-device").await.unwrap();

        assert_eq!(ids(&restarted), ["work"]);
        let uuids: Vec<_> = replicated::records::<Authors>(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|record| record.uuid)
            .collect();
        assert_eq!(uuids, ["work"]);
    }

    /// A file nothing can parse is left for the human, and nothing is taken
    /// up: its Authors are taken up at the first start after it is fixed.
    #[tokio::test]
    async fn a_file_nothing_can_parse_is_taken_up_once_it_is_fixed() {
        let (dir, pool, settings) = fresh().await;

        std::fs::write(settings.config_path(), "authors: [oh\n").unwrap();
        taken_up(&pool, &settings, "this-device").await.unwrap();

        assert_eq!(
            std::fs::read_to_string(settings.config_path()).unwrap(),
            "authors: [oh\n"
        );
        assert!(!replicated::taken_up::<Authors>(&pool).await.unwrap());

        std::fs::write(settings.config_path(), "authors:\n  - id: work\n").unwrap();
        let restarted = Settings::in_data_dir(dir.path());
        restarted.forgetting_the_authors();
        taken_up(&pool, &restarted, "this-device").await.unwrap();

        assert_eq!(ids(&restarted), ["work"]);
    }

    /// A write is a record stamped by this device, and a write that changes
    /// nothing writes nothing.
    #[tokio::test]
    async fn adding_and_editing_an_author_are_writes_to_the_table() {
        let (_dir, pool, settings) = fresh().await;

        let made = written(
            &pool,
            &settings,
            "this-device",
            &author("work", "Ada"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(made, Written::Written);
        assert_eq!(ids(&settings), ["work"]);

        let first = replicated::records::<Authors>(&pool).await.unwrap()[0].clone();
        assert_eq!(first.written_by, "this-device");

        let unchanged = written(
            &pool,
            &settings,
            "this-device",
            &author("work", "Ada"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(unchanged, Written::Unchanged);

        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: ghp_work\n",
        )
        .unwrap();
        let rewritten = edited(
            &pool,
            &settings,
            "this-device",
            &author("work", "Ada L."),
            true,
        )
        .await
        .unwrap();
        assert_eq!(rewritten, Written::Written);

        let record = replicated::records::<Authors>(&pool).await.unwrap()[0].clone();
        assert!(record.updated_at > first.updated_at);
        assert!(record.fields.holds_token, "the file holds its token now");
        assert_eq!(
            settings.config().author("work").unwrap().name(),
            Some("Ada L.")
        );

        let absent = edited(
            &pool,
            &settings,
            "this-device",
            &author("none", "Nobody"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(absent, Written::Absent);
    }

    /// A removal is a deletion stamp on a record that is kept, with the token
    /// out of `secrets.yaml`; the Author is absent from every list after it,
    /// and neither an edit nor a save brings it back.
    #[tokio::test]
    async fn a_removed_author_is_a_deletion_stamp_absent_from_every_list() {
        let (_dir, pool, settings) = fresh().await;

        written(
            &pool,
            &settings,
            "this-device",
            &author("work", "Ada"),
            false,
        )
        .await
        .unwrap();
        written(
            &pool,
            &settings,
            "this-device",
            &author("home", "Tobi"),
            false,
        )
        .await
        .unwrap();
        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: ghp_work\n  home: ghp_home\n",
        )
        .unwrap();

        assert!(
            removed(&pool, &settings, "this-device", "home")
                .await
                .unwrap()
        );
        assert!(
            !removed(&pool, &settings, "this-device", "home")
                .await
                .unwrap()
        );

        assert_eq!(ids(&settings), ["work"]);
        assert_eq!(settings.secrets().author_token("home"), None);
        assert_eq!(settings.secrets().author_token("work"), Some("ghp_work"));

        let records = replicated::records::<Authors>(&pool).await.unwrap();
        let home = records.iter().find(|record| record.uuid == "home").unwrap();
        assert!(home.deleted_at.is_some());

        for again in [
            written(
                &pool,
                &settings,
                "this-device",
                &author("home", "Tobi"),
                false,
            )
            .await,
            edited(
                &pool,
                &settings,
                "this-device",
                &author("home", "Tobi"),
                false,
            )
            .await,
        ] {
            assert_eq!(again.unwrap(), Written::Removed);
        }
        assert_eq!(ids(&settings), ["work"]);
    }
}
