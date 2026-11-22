use super::{Definition, Error, Kind, Media, Puzzles};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use time::{Date, macros::format_description};
use uuid::Uuid;

/// Editorial submission for one UTC day, with at most one puzzle per kind.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub publication_id: Uuid,
    day: String,
    puzzles: Vec<Puzzle>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Puzzle {
    definition: Definition,
    media: Option<MediaSource>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum MediaSource {
    Monster {
        manifest: PathBuf,
        request: oteryn_world::sprites::Request,
    },
    Atlas {
        manifest: PathBuf,
        tile: String,
    },
}

impl MediaSource {
    fn load(&self, root: &Path) -> Result<Media, Error> {
        let manifest = match self {
            Self::Monster { manifest, .. } | Self::Atlas { manifest, .. } => root.join(manifest),
        };
        let directory = manifest
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        const MANIFEST_LIMIT: u64 = 16 * 1024 * 1024;
        match self {
            Self::Monster { request, .. } => {
                Media::monster(directory, &read_json(&manifest, MANIFEST_LIMIT)?, *request)
            }
            Self::Atlas { tile, .. } => {
                Media::atlas(directory, &read_json(&manifest, MANIFEST_LIMIT)?, tile)
            }
        }
    }
}

fn read_json<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T, Error> {
    let metadata = path.metadata().map_err(|_| Error::Unavailable)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(Error::Invalid);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Invalid);
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)
}

impl std::fmt::Debug for Publication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Publication")
            .field("publication_id", &self.publication_id)
            .finish_non_exhaustive()
    }
}

impl Publication {
    pub fn read(path: &Path) -> Result<Self, Error> {
        read_json(path, 64 * 1024)
    }
}

/// Validated publication with any required media loaded before activation.
#[derive(Debug)]
pub struct PreparedPublication {
    publication_id: Uuid,
    day: Date,
    digest: [u8; 32],
    // An existing claim can be checked without reopening retired assets.
    puzzles: Option<Vec<(Definition, Option<Media>)>>,
}

impl Puzzles {
    pub async fn prepare_publication(
        &self,
        publication: Publication,
        root: &Path,
    ) -> Result<PreparedPublication, Error> {
        self.enabled()?;
        let day = Date::parse(
            &publication.day,
            format_description!("[year]-[month]-[day]"),
        )
        .map_err(|_| Error::Invalid)?;
        if publication.publication_id.is_nil() || !(1..=3).contains(&publication.puzzles.len()) {
            return Err(Error::Invalid);
        }
        for (index, puzzle) in publication.puzzles.iter().enumerate() {
            puzzle.definition.validate_publication()?;
            if publication.puzzles[..index]
                .iter()
                .any(|previous| previous.definition.kind == puzzle.definition.kind)
                || !matches!(
                    (puzzle.definition.kind, &puzzle.media),
                    (Kind::Quiz, None)
                        | (Kind::Monster, Some(MediaSource::Monster { .. }))
                        | (Kind::Atlas, Some(MediaSource::Atlas { .. }))
                )
            {
                return Err(Error::Invalid);
            }
        }

        let document = serde_json::to_vec(&publication).map_err(|_| Error::Invalid)?;
        let digest: [u8; 32] = Sha256::digest(document).into();
        let publication_id = publication.publication_id;
        let claimed: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM puzzles_publications WHERE id=$1)")
                .bind(publication_id)
                .fetch_one(&self.pool)
                .await?;
        let puzzles = if claimed {
            None
        } else {
            let mut puzzles = publication.puzzles;
            puzzles.sort_by_key(|puzzle| puzzle.definition.kind.key());
            Some(
                puzzles
                    .into_iter()
                    .map(|puzzle| {
                        let media = puzzle.media.map(|source| source.load(root)).transpose()?;
                        Ok((puzzle.definition, media))
                    })
                    .collect::<Result<_, Error>>()?,
            )
        };
        Ok(PreparedPublication {
            publication_id,
            day,
            digest,
            puzzles,
        })
    }

    /// The trusted caller must authorize the transaction and audit a new
    /// publication before committing it. Returns the new puzzle IDs, or None for an exact replay.
    pub async fn activate_publication(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        publication: PreparedPublication,
    ) -> Result<Option<Vec<Uuid>>, Error> {
        self.enabled()?;
        let PreparedPublication {
            publication_id,
            day,
            digest,
            puzzles,
        } = publication;
        let inserted = sqlx::query(
            "INSERT INTO puzzles_publications (id, document_sha256) VALUES ($1, $2)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(publication_id)
        .bind(&digest[..])
        .execute(&mut **tx)
        .await?;
        if inserted.rows_affected() == 0 {
            let existing: Vec<u8> =
                sqlx::query_scalar("SELECT document_sha256 FROM puzzles_publications WHERE id=$1")
                    .bind(publication_id)
                    .fetch_one(&mut **tx)
                    .await?;
            return if existing == digest {
                Ok(None)
            } else {
                Err(Error::Conflict)
            };
        }
        let today: Date = sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date")
            .fetch_one(&mut **tx)
            .await?;
        if day < today {
            return Err(Error::Invalid);
        }
        // Never add a new publication from a replay-only preparation.
        let puzzles = puzzles.ok_or(Error::Conflict)?;
        let mut ids = Vec::with_capacity(puzzles.len());
        for (definition, media) in puzzles {
            let puzzle_id = self
                .insert_daily(tx, day, &definition, media, Some(publication_id))
                .await?;
            ids.push(puzzle_id);
        }
        Ok(Some(ids))
    }
}
