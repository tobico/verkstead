//! **The Instructions as a record** (ADR-0028): the one text every session is
//! given, whatever harness runs it, is one record of the database's replicated
//! table — the same text on every device of the cluster, the newest write
//! winning — rather than a key of each device's `config.yaml`.
//!
//! **One record, under a fixed uuid** that is the same constant on every
//! device, so that two devices' texts are two writes of the same record and
//! the merge rule decides between them. **Cleared is an empty text**, written
//! as an edit like any other and never a deletion stamp: a removal is final,
//! and a text typed after a clearing is to stand.
//!
//! **Taken up out of `config.yaml` once**, at the serve — see [`taken_up`] —
//! and never read there again. Two rules for the move:
//!
//! - **An empty or absent text is never taken up**, so a device that never had
//!   one, upgraded last, does not wipe everybody's.
//! - **A text taken up is stamped at the epoch**, older than any edit — see
//!   [`replicated::take_up_as_the_oldest`] — so any edit made since an upgrade
//!   wins, and between two devices' taken-up texts the Device Id decides:
//!   arbitrary but stable, and the human sees the one that won and edits it.
//!
//! **An Annex takes nothing up.** Its `config.yaml` held only its Keeper's
//! old copy, handed in at boot, so the key is dropped rather than taken up and
//! the Annex is given the text by its Keeper as a member, like any other.
//!
//! **Held in memory for the settings**, as the Authors are and for their
//! reason: a Built Root is composed on a blocking thread from the files alone,
//! so every write here and every merge hands the text on to
//! [`Settings::holding_instructions`].

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use verkstead_schema::Nudge;

use super::Replicating;
use crate::AppState;
use crate::relaying::Read;
use crate::settings::{Settings, prose_written};
use crate::store::replicated::{self, Kind, Written};

/// The Instructions, as a kind of record.
pub struct Instructions;

impl Kind for Instructions {
    const NAME: &'static str = "instructions";
    type Fields = Fields;
}

impl Replicating for Instructions {
    const MOVED: Nudge = Nudge::Instructions;
    const READ: Read = Read::Instructions;

    /// The merged text held for the settings — taken up first, where a server
    /// stood up without a serve has not — and the pages told.
    async fn merged(state: &AppState) -> Result<()> {
        taken_up(&state.pool, &state.settings, &state.device).await?;
        held(&state.pool, &state.settings).await?;

        state.nudges.announce_here(Nudge::Instructions);

        Ok(())
    }
}

/// The uuid the one record is kept under, which is the same on every device.
pub const THE_RECORD: &str = "5e0c7a1d-3b94-4f26-8d1e-9a2c6b4f7e30";

/// What the record holds besides its own stamps: the text, empty where it was
/// cleared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    #[serde(default)]
    pub text: String,
}

/// The text as the record says it: empty where there is no record, which is
/// nobody having typed one anywhere this device has heard of.
pub async fn text(pool: &SqlitePool) -> Result<String> {
    Ok(replicated::live_records::<Instructions>(pool)
        .await?
        .into_iter()
        .find(|record| record.uuid == THE_RECORD)
        .map(|record| record.fields.text)
        .unwrap_or_default())
}

/// Take the Instructions up out of `config.yaml` into the table, where they
/// have not been already, and hold the record's text for the settings either
/// way.
///
/// **Once**, the database first and the file second, as the Authors are — see
/// [`super::authors::taken_up`]. A file that will not parse is left alone and
/// nothing taken up until the human fixes it. After the move, an
/// `instructions:` key written into the file by hand is ignored, and
/// [`Settings::config`] says so in the log.
pub async fn taken_up(pool: &SqlitePool, settings: &Settings, device: &str) -> Result<()> {
    if settings.holds_instructions() {
        return Ok(());
    }

    if !replicated::taken_up::<Instructions>(pool).await? {
        let settings_ = settings.clone();
        let read = tokio::task::spawn_blocking(move || settings_.instructions_as_written()).await?;

        if let Some((written, annex)) = read {
            // Nothing taken up of an empty text, nor of an Annex's, which is
            // its Keeper's old copy.
            let kept = match prose_written(written) {
                Some(text) if !annex => vec![(THE_RECORD.to_owned(), Fields { text })],
                _ => Vec::new(),
            };

            replicated::take_up_as_the_oldest::<Instructions>(pool, device, &kept)
                .await
                .context("taking the Instructions up out of config.yaml")?;

            if !kept.is_empty() {
                tracing::info!(
                    "the Instructions are a record in the database now, out of config.yaml"
                );
            }

            let settings = settings.clone();
            tokio::task::spawn_blocking(move || settings.dropping_the_instructions())
                .await?
                .context("taking the `instructions:` key out of config.yaml")?;
        }
    }

    held(pool, settings).await
}

/// Hand the record's text on to the settings, which is what the Built Root and
/// the settings page read it out of.
pub async fn held(pool: &SqlitePool, settings: &Settings) -> Result<()> {
    settings.holding_instructions(text(pool).await?);

    Ok(())
}

/// Write `typed` as the text, made here by `device` — **only where it differs
/// from the record's**, a text of nothing included: one request saves every
/// section of the settings page, and a save about something else is no write
/// of the Instructions, nor any news to the cluster. `true` where it was
/// written.
///
/// A text that is nothing but whitespace is the empty text, which is a
/// clearing.
pub async fn saved(
    pool: &SqlitePool,
    settings: &Settings,
    device: &str,
    typed: String,
) -> Result<bool> {
    taken_up(pool, settings, device).await?;

    let typed = prose_written(typed).unwrap_or_default();

    if typed == text(pool).await? {
        return Ok(false);
    }

    let written =
        replicated::write::<Instructions>(pool, device, THE_RECORD, &Fields { text: typed })
            .await?;

    held(pool, settings).await?;

    Ok(written == Written::Written)
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

    async fn stamp(pool: &SqlitePool) -> Option<(i64, String)> {
        replicated::records::<Instructions>(pool)
            .await
            .unwrap()
            .into_iter()
            .next()
            .map(|record| (record.updated_at, record.written_by))
    }

    /// An installation with `instructions:` in `config.yaml` starts with that
    /// text out of the database, stamped older than any edit, and the key gone
    /// from the file; a hand-edited key added afterwards is ignored, at the
    /// next start too.
    #[tokio::test]
    async fn the_text_in_config_yaml_is_taken_up_and_a_later_key_ignored() {
        let (dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            "instructions: |\n  Prefer the smallest change.\nshare_on_done: true\n",
        )
        .unwrap();

        taken_up(&pool, &settings, "this-device").await.unwrap();

        assert_eq!(text(&pool).await.unwrap(), "Prefer the smallest change.\n");
        assert_eq!(stamp(&pool).await, Some((0, "this-device".to_owned())));
        assert_eq!(
            settings.config().instructions(),
            "Prefer the smallest change.\n"
        );

        let written = std::fs::read_to_string(settings.config_path()).unwrap();
        assert!(!written.contains("instructions"), "{written}");
        assert!(written.contains("share_on_done: true"), "{written}");

        std::fs::write(settings.config_path(), "instructions: By hand.\n").unwrap();
        assert_eq!(
            settings.config().instructions(),
            "Prefer the smallest change.\n"
        );

        let restarted = Settings::in_data_dir(dir.path());
        restarted.forgetting_the_instructions();
        taken_up(&pool, &restarted, "this-device").await.unwrap();

        assert_eq!(
            restarted.config().instructions(),
            "Prefer the smallest change.\n"
        );
    }

    /// An empty or absent text is never taken up, so the device that never
    /// had one wipes nobody's: there is no record at all.
    #[tokio::test]
    async fn an_empty_text_is_never_taken_up() {
        for file in ["", "instructions: '   '\n", "share_on_done: true\n"] {
            let (_dir, pool, settings) = fresh().await;
            std::fs::write(settings.config_path(), file).unwrap();

            taken_up(&pool, &settings, "this-device").await.unwrap();

            assert_eq!(stamp(&pool).await, None, "{file:?}");
            assert!(replicated::taken_up::<Instructions>(&pool).await.unwrap());
            assert_eq!(settings.config().instructions(), "");
        }
    }

    /// An Annex's `instructions:` key is its Keeper's old copy: dropped from
    /// the file and not taken up.
    #[tokio::test]
    async fn an_annex_takes_nothing_up() {
        let (_dir, pool, settings) = fresh().await;

        std::fs::write(
            settings.config_path(),
            "instructions: The Keeper's words.\nkept_by:\n  device: a\n  fingerprint: f\n",
        )
        .unwrap();

        taken_up(&pool, &settings, "this-device").await.unwrap();

        assert_eq!(stamp(&pool).await, None);
        assert_eq!(settings.config().instructions(), "");

        let written = std::fs::read_to_string(settings.config_path()).unwrap();
        assert!(!written.contains("instructions"), "{written}");
        assert!(written.contains("kept_by"), "{written}");
    }

    /// A save is a write only where the text changed: the same text again, or
    /// an empty one where there is no record, leaves the stamp as it was —
    /// and a clearing is an empty text, never a deletion stamp.
    #[tokio::test]
    async fn a_save_writes_only_a_changed_text_and_a_clearing_is_empty() {
        let (_dir, pool, settings) = fresh().await;

        assert!(
            !saved(&pool, &settings, "this-device", String::new())
                .await
                .unwrap()
        );
        assert_eq!(stamp(&pool).await, None, "no record made of nothing");

        assert!(
            saved(&pool, &settings, "this-device", "Say it plainly.".into())
                .await
                .unwrap()
        );
        let first = stamp(&pool).await.unwrap();

        assert!(
            !saved(&pool, &settings, "this-device", "Say it plainly.".into())
                .await
                .unwrap()
        );
        assert_eq!(stamp(&pool).await.unwrap(), first);

        assert!(
            saved(&pool, &settings, "this-device", "  \n".into())
                .await
                .unwrap()
        );
        let records = replicated::records::<Instructions>(&pool).await.unwrap();
        assert_eq!(records[0].deleted_at, None);
        assert_eq!(records[0].fields.text, "");
        assert_eq!(settings.config().instructions(), "");
    }
}
