//! **Authors as records** (ADR-0028): every Author is a record of the
//! database's replicated table, the same uuid on every device, rather than an
//! entry in `config.yaml`.
//!
//! A record holds what is drawn — the display name, the git name, the email —
//! and whether this device's `secrets.yaml` holds a token for it: the fact,
//! never the token, which stays in that file under the uuid.
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
//! [`super`]. Nothing of the token crosses there: the record says whether the
//! device that wrote it holds one, and the token is copied on a call of its
//! own.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use verkstead_render::AuthorEntry;
use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::relaying::Read;
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

    /// Whether the `secrets.yaml` of the device that wrote it holds a token
    /// for it.
    #[serde(default)]
    pub holds_token: bool,
}

impl Fields {
    fn of(author: &Author, holds_token: bool) -> Fields {
        Fields {
            display_name: author.display_name().map(str::to_owned),
            name: author.name().map(str::to_owned),
            email: author.email().map(str::to_owned),
            holds_token,
        }
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
                        let holds_token = secrets.author_token(author.id()).is_some();
                        (author.id().to_owned(), Fields::of(&author, holds_token))
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
/// list, picker and launch resolves an Author out of — each last written on
/// another device that holds its token with that device's name, which is
/// where the token's copy is awaited from. `device` is this one's id.
pub async fn held(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    let records = replicated::live_records::<Authors>(pool).await?;

    let elsewhere = records
        .iter()
        .any(|record| record.fields.holds_token && record.written_by != device);
    let names = match elsewhere {
        true => crate::profiles::members_by_device(pool).await?,
        false => Default::default(),
    };

    let authors = records
        .into_iter()
        .map(|record| {
            let author = record.fields.author(&record.uuid);

            match record.fields.holds_token && record.written_by != device {
                true => Recorded::with_token_at(
                    author,
                    crate::profiles::whose(&names, &record.written_by).name,
                ),
                false => Recorded::of(author),
            }
        })
        .collect();

    settings.holding_authors(authors);

    Ok(())
}

/// Write `author` as the record under its uuid — made where there is none —
/// with whether `secrets.yaml` holds its token as that file stands now.
///
/// What the settings save does, for the one Author its Git section drew and
/// for the wizard's first: the token, where the save set or cleared one, is
/// written into the file before this, so that the record says what the file
/// does. `Removed` where the uuid is a removed record, which is final.
pub async fn written(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
) -> Result<Written> {
    writing(pool, settings, device, author, true).await
}

/// And the same where only an edit is meant: `Absent` where there is no live
/// Author under the uuid, and nothing written.
pub async fn edited(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
) -> Result<Written> {
    writing(pool, settings, device, author, false).await
}

async fn writing(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    author: &Author,
    making: bool,
) -> Result<Written> {
    taken_up(pool, settings, device).await?;

    let holds_token = {
        let settings = settings.clone();
        let id = author.id().to_owned();

        tokio::task::spawn_blocking(move || settings.secrets().author_token(&id).is_some()).await?
    };

    let fields = Fields::of(author, holds_token);

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

        let made = written(&pool, &settings, "this-device", &author("work", "Ada"))
            .await
            .unwrap();
        assert_eq!(made, Written::Written);
        assert_eq!(ids(&settings), ["work"]);

        let first = replicated::records::<Authors>(&pool).await.unwrap()[0].clone();
        assert_eq!(first.written_by, "this-device");

        let unchanged = written(&pool, &settings, "this-device", &author("work", "Ada"))
            .await
            .unwrap();
        assert_eq!(unchanged, Written::Unchanged);

        std::fs::write(
            settings.secrets_path(),
            "author_tokens:\n  work: ghp_work\n",
        )
        .unwrap();
        let rewritten = edited(&pool, &settings, "this-device", &author("work", "Ada L."))
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

        let absent = edited(&pool, &settings, "this-device", &author("none", "Nobody"))
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

        written(&pool, &settings, "this-device", &author("work", "Ada"))
            .await
            .unwrap();
        written(&pool, &settings, "this-device", &author("home", "Tobi"))
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
            written(&pool, &settings, "this-device", &author("home", "Tobi")).await,
            edited(&pool, &settings, "this-device", &author("home", "Tobi")).await,
        ] {
            assert_eq!(again.unwrap(), Written::Removed);
        }
        assert_eq!(ids(&settings), ["work"]);
    }
}
