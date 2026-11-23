//! Read the applied G-9 prefix under the same ownership and restore fences as ingest.
use super::{Damage, Error, Filter, Party, Session, Source, v7};
use crate::authority;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Journal {
    pub pool: PgPool,
    pub source: String,
}

pub(crate) struct Hunt {
    pub session: Session,
    pub catalogue: Option<(Uuid, String)>,
    pub parties: Vec<PartySegment>,
    pub drops: Option<Vec<Drop>>,
}
pub(crate) struct PartySegment {
    pub started_at: i64,
    pub ended_at: i64,
    pub ledger: Party,
}
#[derive(Deserialize)]
pub(crate) struct Drop {
    pub item_key: String,
    pub quantity: String,
    pub occurred_at: String,
    pub monster_key: Option<String>,
    pub place_key: Option<String>,
    pub rarity_rule_key: String,
}

#[derive(Deserialize)]
struct Segment {
    world_id: Uuid,
    catalog_revision: String,
    rare_drop_count: usize,
    character_revision: String,
    started_at: String,
    ended_at: String,
    duration_seconds: String,
    experience_gained: String,
    experience_lost: String,
    places: Vec<Place>,
    loot: Vec<Item>,
    supplies: Vec<Item>,
    damage_dealt: Vec<DamageRow>,
    damage_taken: Vec<DamageRow>,
    healing_done: String,
    healing_received: String,
    monsters_killed: Vec<Kill>,
    party: Option<PartyRow>,
}
#[derive(Deserialize)]
struct Place {
    place_key: Option<String>,
    duration_seconds: String,
}
#[derive(Deserialize)]
struct Item {
    item_key: String,
    quantity: String,
}
#[derive(Deserialize)]
struct Kill {
    monster_key: String,
    count: String,
}
#[derive(Deserialize)]
struct DamageRow {
    damage_type_key: String,
    source_kind: Source,
    source_key: Option<String>,
    amount: String,
}
#[derive(Deserialize)]
struct PartyRow {
    party_id: Uuid,
    members: Vec<Member>,
    loot: Vec<PartyItem>,
}
#[derive(Deserialize)]
struct Member {
    member_slot: u8,
    is_subject: bool,
}
#[derive(Deserialize)]
struct PartyItem {
    member_slot: u8,
    #[serde(flatten)]
    item: Item,
}
type DamageKey = (String, Source, Option<String>);
type Record = (Uuid, String, Value);

fn unavailable(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn number(input: &str) -> Result<i64, Error> {
    input.parse().map_err(unavailable)
}
fn add(total: &mut i64, amount: i64) -> Result<(), Error> {
    *total = total.checked_add(amount).ok_or(Error::Unavailable)?;
    Ok(())
}
fn combine<K: Ord>(map: &mut BTreeMap<K, i64>, key: K, amount: &str) -> Result<(), Error> {
    add(map.entry(key).or_default(), number(amount)?)
}
fn damage(rows: BTreeMap<DamageKey, i64>) -> Vec<Damage> {
    rows.into_iter()
        .map(
            |((damage_type_key, source_kind, source_key), amount)| Damage {
                damage_type_key,
                source_kind,
                source_key,
                amount,
            },
        )
        .collect()
}

fn merge(id: Uuid, character: Uuid, records: Vec<Record>) -> Result<Hunt, Error> {
    let mut session = Session {
        session_id: id,
        character_id: character,
        started_at: i64::MAX,
        ended_at: 0,
        duration_seconds: 0,
        experience_gained: 0,
        experience_lost: 0,
        places: BTreeMap::new(),
        loot: BTreeMap::new(),
        supplies: BTreeMap::new(),
        damage_dealt: Vec::new(),
        damage_taken: Vec::new(),
        healing_done: 0,
        healing_received: 0,
        monsters_killed: BTreeMap::new(),
        party: None,
    };
    let mut dealt = BTreeMap::new();
    let mut taken = BTreeMap::new();
    let mut parties = Vec::new();
    let mut drops = Vec::<Drop>::new();
    let mut world = None;
    let mut catalogues = BTreeSet::new();
    let mut expected_drops = BTreeMap::new();
    let mut actual_drops = BTreeMap::new();
    for (segment_id, kind, value) in records {
        if kind == "RARE_DROP" {
            *actual_drops.entry(segment_id).or_insert(0) += 1;
            drops.push(serde_json::from_value(value).map_err(unavailable)?);
            continue;
        }
        let row: Segment = serde_json::from_value(value).map_err(unavailable)?;
        world = Some(row.world_id);
        catalogues.insert(row.catalog_revision);
        if row.rare_drop_count > 0 {
            expected_drops.insert(segment_id, row.rare_drop_count);
        }
        let start = number(&row.started_at)?;
        let end = number(&row.ended_at)?;
        let duration = number(&row.duration_seconds)?;
        session.started_at = session.started_at.min(start);
        session.ended_at = session.ended_at.max(end);
        for (total, amount) in [
            (&mut session.duration_seconds, duration),
            (
                &mut session.experience_gained,
                number(&row.experience_gained)?,
            ),
            (&mut session.experience_lost, number(&row.experience_lost)?),
            (&mut session.healing_done, number(&row.healing_done)?),
            (
                &mut session.healing_received,
                number(&row.healing_received)?,
            ),
        ] {
            add(total, amount)?;
        }
        for place in row.places {
            combine(
                &mut session.places,
                place.place_key.unwrap_or_default(),
                &place.duration_seconds,
            )?;
        }
        for item in row.loot {
            combine(&mut session.loot, item.item_key, &item.quantity)?;
        }
        for item in row.supplies {
            combine(&mut session.supplies, item.item_key, &item.quantity)?;
        }
        for kill in row.monsters_killed {
            combine(&mut session.monsters_killed, kill.monster_key, &kill.count)?;
        }
        for (rows, map) in [
            (row.damage_dealt, &mut dealt),
            (row.damage_taken, &mut taken),
        ] {
            for hit in rows {
                combine(
                    map,
                    (hit.damage_type_key, hit.source_kind, hit.source_key),
                    &hit.amount,
                )?;
            }
        }
        if let Some(party) = row.party {
            let subject_slot = party
                .members
                .iter()
                .find(|m| m.is_subject)
                .ok_or(Error::Unavailable)?
                .member_slot;
            let mut members: BTreeMap<_, _> = party
                .members
                .into_iter()
                .map(|member| (member.member_slot, BTreeMap::new()))
                .collect();
            for item in party.loot {
                let items = members
                    .get_mut(&item.member_slot)
                    .ok_or(Error::Unavailable)?;
                combine(items, item.item.item_key, &item.item.quantity)?;
            }
            // Slot and roster changes establish new segments. Keep each ledger local to
            // its own interval and exclude party totals from the personal aggregate.
            parties.push(PartySegment {
                started_at: start,
                ended_at: end,
                ledger: Party {
                    party_id: party.party_id,
                    subject_slot,
                    members,
                },
            });
        }
    }
    if expected_drops != actual_drops {
        return Err(Error::Unavailable);
    }
    session.damage_dealt = damage(dealt);
    session.damage_taken = damage(taken);
    session.validate().map_err(unavailable)?;
    drops.sort_by(|a, b| {
        (b.occurred_at.len(), &b.occurred_at).cmp(&(a.occurred_at.len(), &a.occurred_at))
    });
    Ok(Hunt {
        session,
        catalogue: world.zip(
            (catalogues.len() == 1)
                .then(|| catalogues.into_iter().next())
                .flatten(),
        ),
        parties,
        drops: Some(drops),
    })
}

impl Journal {
    pub async fn read(
        &self,
        account: Uuid,
        now: i64,
        filter: &Filter,
        ids: &[Uuid],
    ) -> Result<Vec<Hunt>, Error> {
        filter.validate()?;
        if ids.len() > 8
            || ids.iter().any(|id| !v7(*id))
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
            || !v7(account)
            || now <= 0
        {
            return Err(Error::Invalid);
        }
        if self.source.is_empty() {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        let (generation, recovery) = self.current_generation(&mut tx, account).await?;
        // Test every record in a session before applying display filters. Removing a
        // retained segment, drop or unresolved revision must never shorten a recap.
        let sessions: Vec<(Uuid, Uuid)> = sqlx::query_as(r#"
            SELECT r.session_id, r.character_id
            FROM pd_g9_records r JOIN pd_g9_streams s USING(source_authority,account_id,stream_generation)
            WHERE r.source_authority=$1 AND r.account_id=$2 AND r.stream_generation=$3
              AND (cardinality($5::uuid[])=0 OR r.session_id=ANY($5))
              AND ($6::uuid IS NULL OR r.character_id=$6)
            GROUP BY r.session_id,r.character_id
            HAVING bool_and(r.resolved AND r.record IS NOT NULL AND r.purged_at IS NULL
                        AND r.expires_at>$4 AND r.source_revision>s.retention_floor
                        AND r.source_revision<=s.applied_through)
               AND count(*) FILTER(WHERE r.record_kind='HUNT_SEGMENT') =
                   max(r.segment_index) FILTER(WHERE (r.record->>'final_segment')::boolean)+1
               AND max((r.record->>'ended_at')::bigint) <= $4
               AND sum(octet_length(r.canonical_bytes)) <= 2097152 AND count(*) <= 1024
               AND ($7::text IS NULL OR bool_or(r.record->'places' @> jsonb_build_array(jsonb_build_object('place_key',$7))))
               AND ($8::bigint IS NULL OR min((r.record->>'started_at')::bigint) >= $8)
               AND ($9::bigint IS NULL OR min((r.record->>'started_at')::bigint) < $9)
               AND ($10::boolean IS NULL OR bool_or(r.record->'party' <> 'null'::jsonb) = $10)
            ORDER BY max((r.record->>'ended_at')::bigint) DESC, r.session_id
            LIMIT 20 OFFSET $11
        "#)
            .bind(&self.source).bind(account).bind(generation).bind(now).bind(ids)
            .bind(filter.character).bind(&filter.place).bind(filter.from).bind(filter.until)
            .bind(filter.party).bind(filter.offset.unwrap_or_default())
            .fetch_all(&mut *tx).await.map_err(unavailable)?;
        let mut hunts = Vec::new();
        for (id, character) in sessions {
            let Ok(authority::Ownership::Owner(cut)) =
                authority::ownership(&mut tx, account, character).await
            else {
                continue;
            };
            if cut.recovery > recovery
                || crate::sessions::erased(&mut tx, account, character, cut.character_revision)
                    .await
                    .map_err(unavailable)?
            {
                continue;
            }
            let records: Vec<Record> = sqlx::query_as(
                "SELECT segment_id, record_kind, record FROM pd_g9_records
                 WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3 AND session_id=$4
                   AND character_id=$5 ORDER BY segment_index NULLS LAST, source_revision LIMIT 1025",
            )
                .bind(&self.source).bind(account).bind(generation).bind(id).bind(character)
                .fetch_all(&mut *tx).await.map_err(unavailable)?;
            if records.len() > 1024
                || !records.iter().all(|(_, kind, record)| {
                    record["character_revision"]
                        .as_str()
                        .and_then(|r| r.parse().ok())
                        == Some(cut.character_revision)
                        && (kind == "RARE_DROP" || segment_matches(record, &cut))
                })
            {
                continue;
            }
            if let Ok(hunt) = merge(id, character, records) {
                hunts.push(hunt);
            }
        }
        tx.commit().await.map_err(unavailable)?;
        if !ids.is_empty() {
            if hunts.len() != ids.len() {
                return Err(Error::NotFound);
            }
            hunts.sort_by_key(|hunt| ids.iter().position(|id| *id == hunt.session.session_id));
        }
        Ok(hunts)
    }

    async fn current_generation(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
    ) -> Result<(Uuid, u64), Error> {
        sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended($1,73))")
            .bind(account.to_string())
            .execute(&mut **tx)
            .await
            .map_err(unavailable)?;
        let stream: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT stream_generation,recovery_generation::text FROM pd_g9_streams
             WHERE source_authority=$1 AND account_id=$2 AND admitted AND NOT blocked
               AND NOT conflicted AND applied_through>=retention_floor FOR SHARE",
        )
        .bind(&self.source)
        .bind(account)
        .fetch_optional(&mut **tx)
        .await
        .map_err(unavailable)?;
        let (generation, admitted) = stream.ok_or(Error::Unavailable)?;
        let recovery = authority::session_fence(tx, account)
            .await
            .map_err(unavailable)?;
        let admitted = admitted.parse::<u64>().map_err(unavailable)?;
        if admitted < recovery {
            return Err(Error::Unavailable);
        }
        Ok((generation, admitted))
    }
}

fn segment_matches(record: &Value, cut: &authority::OwnershipCut) -> bool {
    serde_json::from_value::<Segment>(record.clone()).is_ok_and(|segment| {
        segment.world_id == cut.world
            && segment.character_revision.parse() == Ok(cut.character_revision)
            && segment
                .started_at
                .parse::<u64>()
                .is_ok_and(|at| at >= cut.decided_at)
    })
}
