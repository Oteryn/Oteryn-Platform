//! Qualification metadata for the proposed character correction profiles.
//! Executable and advertised operations are defined separately in the ADM catalog.
use super::access::tr;
use crate::audit::RiskClass;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Selection {
    Overview,
    Rescue,
    Disconnect,
    Outfit,
    Compensate,
    Confiscate,
}

#[derive(Clone, Copy)]
struct Profile {
    selection: Selection,
    slug: &'static str,
    operation: &'static str,
    permission: &'static str,
    risk: RiskClass,
}

const PROFILES: [Profile; 5] = [
    Profile {
        selection: Selection::Rescue,
        slug: "rescue",
        operation: "character.rescue.teleport",
        permission: "game.player.rescue",
        risk: RiskClass::R3,
    },
    Profile {
        selection: Selection::Disconnect,
        slug: "disconnect",
        operation: "character.session.disconnect",
        permission: "game.player.session.disconnect",
        risk: RiskClass::R3,
    },
    Profile {
        selection: Selection::Outfit,
        slug: "outfit",
        operation: "character.outfit.correct",
        permission: "game.player.outfit.correct",
        risk: RiskClass::R2,
    },
    Profile {
        selection: Selection::Compensate,
        slug: "compensate",
        operation: "character.item.compensate",
        permission: "game.player.item.compensate",
        risk: RiskClass::R3,
    },
    Profile {
        selection: Selection::Confiscate,
        slug: "confiscate",
        operation: "character.item.confiscate",
        permission: "game.player.item.confiscate",
        risk: RiskClass::R4,
    },
];

pub(super) struct View {
    pub intro: String,
    pub unavailable: String,
    pub profiles: Vec<ProfileView>,
}

pub(super) struct ProfileView {
    pub title: String,
    pub slug: &'static str,
    pub operation: &'static str,
    pub risk: &'static str,
    pub detail: Option<Detail>,
}

pub(super) struct Detail {
    pub permission: &'static str,
    pub controls: String,
    pub evidence: String,
    pub receipt: String,
    pub states: Vec<String>,
}

impl View {
    pub fn new(lang: &str, selection: Selection) -> Self {
        let profiles = PROFILES
            .into_iter()
            .filter(|profile| selection == Selection::Overview || selection == profile.selection)
            .map(|profile| ProfileView {
                title: tr(lang, &format!("remediation-{}", profile.slug)),
                slug: profile.slug,
                operation: profile.operation,
                risk: profile.risk.as_str(),
                detail: (selection != Selection::Overview).then(|| profile.detail(lang)),
            })
            .collect();
        Self {
            intro: tr(lang, "remediation-blocked"),
            unavailable: tr(lang, "unavailable"),
            profiles,
        }
    }
}

impl Profile {
    fn detail(self, lang: &str) -> Detail {
        let mut states = vec![tr(lang, "remediation-target")];
        if matches!(self.risk, RiskClass::R3 | RiskClass::R4) {
            states.push(tr(lang, "remediation-preview"));
        }
        if self.risk == RiskClass::R4 {
            states.push(tr(lang, "remediation-approval"));
        }
        states.push(tr(lang, "remediation-receipt"));
        Detail {
            permission: self.permission,
            controls: tr(
                lang,
                &format!("remediation-controls-{}", self.risk.as_str()),
            ),
            evidence: tr(lang, &format!("remediation-{}-evidence", self.slug)),
            receipt: tr(lang, &format!("remediation-{}-receipt", self.slug)),
            states,
        }
    }
}
