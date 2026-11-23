//! Read only the reviewed, public offers in the active world projection.
use super::{Snapshot, content_key, item_key};
use serde::{Deserialize, Serialize};

pub(super) const KIND: &str = "NPC.ShopOffer";

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    NpcSellOffer,
    NpcBuyOffer,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicOffer {
    direction: Direction,
    npc: String,
    item: String,
    currency: String,
    unit_price: u32,
    item_subtype: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShopOffer {
    pub npc: String,
    pub direction: Direction,
    pub currency: String,
    pub unit_price: u32,
    pub item_subtype: i32,
}

impl Snapshot {
    pub(super) fn shop_offers(&self, item: &str) -> Vec<ShopOffer> {
        self.kind(KIND)
            .filter_map(|(_, data)| {
                let offer = PublicOffer::deserialize(data).ok()?;
                if offer.item != item
                    || !content_key(&offer.npc)
                    || !item_key(&offer.currency)
                    || offer.unit_price == 0
                    || self.get("NPC", &offer.npc).is_none()
                    || self.get("Item", &offer.currency).is_none()
                {
                    return None;
                }
                Some(ShopOffer {
                    npc: offer.npc,
                    direction: offer.direction,
                    currency: offer.currency,
                    unit_price: offer.unit_price,
                    item_subtype: offer.item_subtype,
                })
            })
            .collect()
    }
}
