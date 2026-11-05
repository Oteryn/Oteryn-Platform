use serde::Deserialize;
use serde_json::Value;

use super::{Context, Error, Item, Purpose, gold};
use oteryn_world::WorldStore;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionRef {
    family: String,
    key: String,
    revision: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    direction: String,
    item: DefinitionRef,
    unit_price: i64,
    count: Option<u32>,
    currency: Option<DefinitionRef>,
}

pub(super) fn select(
    npc: &Value,
    service: &Value,
    item: &Item,
    purpose: Purpose,
    npc_key: &str,
    service_key: &str,
) -> Result<i64, Error> {
    let npc = npc.get("declaration").ok_or(Error::Unavailable)?;
    let service = service.get("declaration").ok_or(Error::Unavailable)?;
    if npc.pointer("/identity/key").and_then(Value::as_str) != Some(npc_key)
        || npc.get("kind").and_then(Value::as_str) != Some("NPC")
        || service.pointer("/identity/key").and_then(Value::as_str) != Some(service_key)
        || service.get("kind").and_then(Value::as_str) != Some("Service")
        // Uninterpreted service conditions cannot authorize a price estimate.
        || service.get("fields").and_then(Value::as_array).is_none_or(|v| !v.is_empty())
    {
        return Err(Error::Unavailable);
    }
    let revision = service
        .pointer("/identity/revision")
        .and_then(Value::as_str)
        .ok_or(Error::Unavailable)?;
    let services: Vec<DefinitionRef> =
        serde_json::from_value(npc.get("services").cloned().ok_or(Error::Unavailable)?)
            .map_err(|_| Error::Unavailable)?;
    if services
        .iter()
        .filter(|r| r.family == "Service" && r.key == service_key && r.revision == revision)
        .count()
        != 1
    {
        return Err(Error::Unavailable);
    }
    let offers: Vec<Offer> =
        serde_json::from_value(service.get("offers").cloned().ok_or(Error::Unavailable)?)
            .map_err(|_| Error::Unavailable)?;
    let direction = match purpose {
        Purpose::Loot => "BuyFromPlayer",
        Purpose::Supplies => "SellToPlayer",
    };
    let mut matches = offers.iter().filter(|o| {
        o.direction == direction
            && o.item.family == "Item"
            && o.item.key == item.key
            && o.item.revision == item.revision
    });
    let offer = matches.next().ok_or(Error::Unavailable)?;
    if matches.next().is_some()
        || !gold(offer.unit_price)
        || offer.count.is_some_and(|count| count != 1)
        || offer.currency.is_some()
    {
        return Err(Error::Unavailable);
    }
    Ok(offer.unit_price)
}

pub(super) async fn price(
    world: &WorldStore,
    context: &Context,
    item: &Item,
    purpose: Purpose,
    npc_key: &str,
    service_key: &str,
) -> Result<i64, Error> {
    let npc = world
        .get_entity("NPC", npc_key)
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    let service = world
        .get_entity("Service", service_key)
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    if npc.version_id != context.catalogue_version
        || service.version_id != context.catalogue_version
    {
        return Err(Error::Unavailable);
    }
    select(
        &npc.data,
        &service.data,
        item,
        purpose,
        npc_key,
        service_key,
    )
}
