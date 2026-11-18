//! Game's D2(b) catalogue identity and its projection onto Platform Store v1.
use super::{Catalog, Error, MAX_OFFERS, Offer, Unit, positive};
use serde::{Deserialize, Deserializer, de};
use serde_json::Value;
use std::{collections::HashSet, fmt};

const SCHEMA: &str = "OTERYN_STORE_RUNTIME_CATALOGUE/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    #[serde(rename = "source")]
    _source: Value,
    offers: Vec<RuntimeOffer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeOffer {
    key: String,
    #[serde(rename = "category")]
    _category: String,
    #[serde(rename = "title")]
    _title: String,
    price: Option<u64>,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    currency: String,
    item: Option<Item>,
    unlock_key: Option<String>,
    hireling: Option<Hireling>,
    preview: Option<Preview>,
    #[serde(rename = "source_line")]
    _source_line: u32,
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    key: String,
    revision: String,
    #[serde(rename = "charges")]
    _charges: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hireling {
    effect: String,
    source_id: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Preview {
    kind: u8,
    key: String,
}

pub(super) fn parse(raw: &[u8]) -> Result<Catalog, Error> {
    let mut json = serde_json::from_slice::<RuntimeJson>(raw)
        .map_err(|_| Error::InvalidDocument)?
        .0;
    json.sort_all_objects();
    // Serialize the complete source, including metadata and disabled offers. Game's
    // catalogue JSON uses sorted UTF-8, while Store wire JSON uses schema member order.
    let bytes = serde_json::to_vec(&json).map_err(|_| Error::InvalidDocument)?;
    let document: Document = serde_json::from_value(json).map_err(|_| Error::InvalidDocument)?;
    if document.schema != SCHEMA || !(1..=MAX_OFFERS).contains(&document.offers.len()) {
        return Err(Error::InvalidDocument);
    }
    let mut seen = HashSet::new();
    let mut offers = Vec::new();
    for offer in document.offers {
        if !crate::purchase::offer_key(&offer.key) || !seen.insert(offer.key.clone()) {
            return Err(Error::InvalidDocument);
        }
        // Game does not quote owner-priced placeholders, but they remain in the digest.
        if offer.enabled {
            offers.push(offer.project()?);
        }
    }
    Catalog::from_offers(bytes, offers)
}

impl RuntimeOffer {
    fn project(self) -> Result<Offer, Error> {
        if self.currency != "TIBIA_COIN"
            || self.preview.is_some_and(|preview| {
                !(1..=3).contains(&preview.kind)
                    || preview.key.is_empty()
                    || preview.key.len() > 128
            })
        {
            return Err(Error::InvalidDocument);
        }
        let unit = match (self.item, self.unlock_key, self.hireling) {
            (Some(item), None, None) => item.unit()?,
            (None, Some(key), None) => unlock_unit(key)?,
            (item, None, Some(hireling)) => hireling.unit(&self.key, item)?,
            _ => return Err(Error::InvalidDocument),
        };
        Ok(Offer {
            offer_key: self.key,
            currency: "COINS".into(),
            unit_price: self.price.ok_or(Error::InvalidDocument)?.to_string(),
            // The Game runtime quotes one delivery per purchase. Item charges are
            // definition data and the Game mints one item, with its authored charges.
            max_quantity: 1,
            purchasable: true,
            units: vec![unit],
        })
    }
}

impl Item {
    fn unit(self) -> Result<Unit, Error> {
        let revision = self
            .revision
            .strip_prefix("definition-r")
            .filter(|revision| positive(revision).is_some())
            .ok_or(Error::InvalidDocument)?;
        Ok(unit("oteryn.store.item_grant", "ITEM", self.key, revision))
    }
}

fn unlock_unit(key: String) -> Result<Unit, Error> {
    let (product, kind) = match key.as_str() {
        "prey-list-expansion" | "charm-expansion" => ("oteryn.store.item_grant", "ITEM"),
        // The Platform contract gives cosmetic entitlements their own rollout
        // gate. The Game dispatch currently requests ITEM for these offers too.
        key if key.starts_with("oteryn:outfit.") => ("oteryn.store.outfit_unlock", "COSMETIC"),
        key if key.starts_with("oteryn:mount.") => ("oteryn.store.mount_unlock", "COSMETIC"),
        _ => return Err(Error::InvalidDocument),
    };
    // The v1 runtime defines unlocks by key; the complete catalogue revision
    // carries their source provenance. Only item bindings have a separate revision.
    Ok(unit(product, kind, key, "1"))
}

impl Hireling {
    fn unit(self, key: &str, item: Option<Item>) -> Result<Unit, Error> {
        if key != format!("store:hireling:{}", self.source_id) {
            return Err(Error::InvalidDocument);
        }
        match (self.effect.as_str(), self.source_id, item) {
            ("hire", 25440, Some(item))
                if item.key == "oteryn:item.tibia.i29432" && item.revision == "definition-r1" =>
            {
                item.unit()
            }
            ("unlock", 1001..=1004 | 2001..=2009, None)
            | ("rename", 25438, None)
            | ("sex_change", 25437, None) => {
                Ok(unit("oteryn.store.item_grant", "ITEM", key.into(), "1"))
            }
            _ => Err(Error::InvalidDocument),
        }
    }
}

fn unit(product: &str, kind: &str, key: String, revision: &str) -> Unit {
    Unit {
        product_id: product.into(),
        product_version: "1".into(),
        unit_kind: kind.into(),
        grant_key: key,
        grant_revision: revision.into(),
        quantity: 1,
    }
}

/// D2(b) accepts the Game's full signed and unsigned JSON integer range, including
/// in provenance metadata. Refuse floats and duplicate keys before canonicalizing.
struct RuntimeJson(Value);

impl<'de> Deserialize<'de> for RuntimeJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = RuntimeJson;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("runtime catalogue JSON with integers and unique object keys")
            }

            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<RuntimeJson, M::Error> {
                let mut members = serde_json::Map::new();
                while let Some((key, RuntimeJson(value))) = map.next_entry()? {
                    if members.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate catalogue member"));
                    }
                }
                Ok(RuntimeJson(Value::Object(members)))
            }

            fn visit_seq<S: de::SeqAccess<'de>>(self, mut seq: S) -> Result<RuntimeJson, S::Error> {
                let mut values = Vec::new();
                while let Some(RuntimeJson(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(RuntimeJson(Value::Array(values)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<RuntimeJson, E> {
                Ok(RuntimeJson(Value::from(value)))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<RuntimeJson, E> {
                Ok(RuntimeJson(Value::from(value)))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<RuntimeJson, E> {
                Ok(RuntimeJson(Value::from(value)))
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<RuntimeJson, E> {
                Ok(RuntimeJson(Value::from(value)))
            }

            fn visit_unit<E: de::Error>(self) -> Result<RuntimeJson, E> {
                Ok(RuntimeJson(Value::Null))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}
