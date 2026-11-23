//! Request-local estimates. Prices and preferences are never persisted by hunt views.
use super::{Error, Pricing, Quote, journal::Hunt, key};
use crate::pricing::{GOLD_CEILING, Item, Purpose, npc};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::BTreeMap;
use uuid::Uuid;

type ItemSide = (bool, String);
type Selections = BTreeMap<ItemSide, Selection>;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Selection {
    Custom(i64),
    Npc { npc: String, service: String },
    Bazaar,
}

pub(super) struct Labels {
    pub custom: String,
    pub npc: String,
    pub bazaar: String,
}

pub(super) struct Prices<'a> {
    chosen: BTreeMap<ItemSide, Option<Quote>>,
    fallback: &'a dyn Pricing,
}

pub(super) fn parse(raw: &str) -> Result<Selections, Error> {
    if raw.len() > 4096 || raw.lines().count() > 40 {
        return Err(Error::Invalid);
    }
    let mut prices = BTreeMap::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (entry, value) = line.split_once('=').ok_or(Error::Invalid)?;
        let (purpose, item) = entry.split_once(' ').ok_or(Error::Invalid)?;
        let supplies = match purpose {
            "loot" => false,
            "supply" => true,
            _ => return Err(Error::Invalid),
        };
        let selection = if value == "bazaar" {
            Selection::Bazaar
        } else if let Some((npc, service)) = value.split_once('|') {
            if !key(npc) || !key(service) {
                return Err(Error::Invalid);
            }
            Selection::Npc {
                npc: npc.to_owned(),
                service: service.to_owned(),
            }
        } else {
            let gold = value.parse::<i64>().map_err(|_| Error::Invalid)?;
            if !(0..=GOLD_CEILING).contains(&gold) {
                return Err(Error::Invalid);
            }
            Selection::Custom(gold)
        };
        if !key(item)
            || prices
                .insert((supplies, item.to_owned()), selection)
                .is_some()
        {
            return Err(Error::Invalid);
        }
    }
    Ok(prices)
}

impl<'a> Prices<'a> {
    pub async fn for_hunt(
        pool: &PgPool,
        hunt: &Hunt,
        choices: &Selections,
        fallback: &'a dyn Pricing,
        labels: &Labels,
    ) -> Self {
        let mut chosen = BTreeMap::new();
        for ((supplies, item), selection) in choices {
            let present = if *supplies {
                hunt.session.supplies.contains_key(item)
            } else {
                hunt.session.loot.contains_key(item)
                    || hunt.parties.iter().any(|segment| {
                        segment
                            .ledger
                            .members
                            .values()
                            .any(|loot| loot.contains_key(item))
                    })
            };
            if present {
                let quote = resolve(pool, hunt, item, *supplies, selection, labels).await;
                chosen.insert((*supplies, item.clone()), quote);
            }
        }
        Self { chosen, fallback }
    }
}

impl Pricing for Prices<'_> {
    fn quote(&self, item: &str, character: Uuid) -> Option<Quote> {
        self.quote_for(item, character, false)
    }

    fn quote_for(&self, item: &str, character: Uuid, supplies: bool) -> Option<Quote> {
        self.chosen
            .get(&(supplies, item.to_owned()))
            .cloned()
            .unwrap_or_else(|| self.fallback.quote_for(item, character, supplies))
    }
}

async fn resolve(
    pool: &PgPool,
    hunt: &Hunt,
    item: &str,
    supplies: bool,
    selection: &Selection,
    labels: &Labels,
) -> Option<Quote> {
    if let Selection::Custom(unit_gold) = selection {
        return Some(Quote {
            unit_gold: *unit_gold,
            provenance: labels.custom.clone(),
        });
    }
    if !oteryn_world::configured_world_enabled() {
        return None;
    }
    // Match the journal's accepted capture revision to the active published content
    // commit exactly. A name or an inferred item revision cannot qualify a quote.
    let (world, catalogue) = hunt.catalogue.as_ref()?;
    let (version, revision): (Uuid, String) = sqlx::query_as(
        "SELECT e.version_id, e.data #>> '{declaration,identity,revision}'
         FROM world_entities e
         JOIN world_active_version a ON a.version_id=e.version_id
         JOIN world_versions v ON v.id=e.version_id AND v.status='ready'
         JOIN world_visibility p USING(kind,key)
         WHERE e.kind='Item' AND e.key=$1 AND e.publishable AND p.allowed
           AND v.source_commit=$2 AND e.data #>> '{declaration,identity,key}'=$1",
    )
    .bind(item)
    .bind(catalogue)
    .fetch_optional(pool)
    .await
    .ok()??;
    match selection {
        Selection::Npc { npc, service } => {
            let (npc_data, service_data): (Value, Value) = sqlx::query_as(
                "SELECT n.data, s.data FROM world_active_version a
                 JOIN world_entities n ON n.version_id=a.version_id AND n.kind='NPC' AND n.key=$2
                 JOIN world_entities s ON s.version_id=a.version_id AND s.kind='Service' AND s.key=$3
                 JOIN world_visibility pn ON pn.kind=n.kind AND pn.key=n.key
                 JOIN world_visibility ps ON ps.kind=s.kind AND ps.key=s.key
                 WHERE a.version_id=$1 AND pn.allowed AND ps.allowed AND n.publishable AND s.publishable",
            ).bind(version).bind(npc).bind(service).fetch_optional(pool).await.ok()??;
            let purpose = if supplies {
                Purpose::Supplies
            } else {
                Purpose::Loot
            };
            let unit_gold = npc::select(
                &npc_data,
                &service_data,
                &Item {
                    key: item.to_owned(),
                    revision,
                },
                purpose,
                npc,
                service,
            )
            .ok()?;
            Some(Quote {
                unit_gold,
                provenance: format!("{} · {npc} · {service} · {catalogue}", labels.npc),
            })
        }
        Selection::Bazaar => {
            if std::env::var("OTERYN_BAZAAR_ENABLED").as_deref() != Ok("true") {
                return None;
            }
            let (unit_gold, sample_count): (i64, i64) = sqlx::query_as(
                "WITH latest AS (
                     SELECT DISTINCT ON (s.seller) s.seller,s.buyer,s.gross_amount/a.quantity AS unit_gold
                     FROM bazaar_authority_gold_sales s
                     JOIN bazaar_authority_assets a ON a.listing_id=s.listing_id
                     JOIN bazaar_listings l ON l.id=s.listing_id
                     WHERE a.world_id=$1 AND a.definition_key=$2 AND a.definition_revision=$3
                       AND a.publication_consent AND l.kind='item' AND l.currency='gold'
                       AND l.state='completed' AND s.completed_at >= date_trunc('day', now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' - interval '7 days'
                       AND s.completed_at < date_trunc('day', now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'
                       AND s.gross_amount>=a.quantity
                     ORDER BY s.seller,s.completed_at DESC,s.id DESC
                 )
                 SELECT percentile_disc(0.5) WITHIN GROUP (ORDER BY unit_gold), count(*)
                 FROM latest HAVING count(*)>=5 AND count(DISTINCT buyer)>=5",
            ).bind(world).bind(item).bind(&revision).fetch_optional(pool).await.ok()??;
            Some(Quote {
                unit_gold,
                provenance: format!("{} · {catalogue} · {sample_count}", labels.bazaar),
            })
        }
        Selection::Custom(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_distinguish_sources_and_check_bounds() -> Result<(), Error> {
        let prices = parse(
            "loot item:ruby=12\nsupply item:ruby=15\nloot item:ore=bazaar\nsupply item:ore=npc:merchant|service:trade",
        )?;
        assert_eq!(
            prices.get(&(false, "item:ruby".to_owned())),
            Some(&Selection::Custom(12))
        );
        assert_eq!(
            prices.get(&(true, "item:ruby".to_owned())),
            Some(&Selection::Custom(15))
        );
        assert_eq!(
            prices.get(&(false, "item:ore".to_owned())),
            Some(&Selection::Bazaar)
        );
        assert!(matches!(
            prices.get(&(true, "item:ore".to_owned())),
            Some(Selection::Npc { .. })
        ));
        for malformed in [
            "loot item:ruby=-1",
            "loot item:ruby=1000000000000",
            "loot item:ruby=1\nloot item:ruby=2",
            "unknown item:ruby=1",
            "loot item:ruby=npc:merchant|broken",
        ] {
            assert_eq!(parse(malformed), Err(Error::Invalid));
        }
        Ok(())
    }
}
