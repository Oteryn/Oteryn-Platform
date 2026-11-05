//! Minimal wire projection; unknown game fields remain forward compatible.
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub(super) struct Appearances {
    #[prost(message, repeated, tag = "1")]
    pub objects: Vec<Appearance>,
    #[prost(message, repeated, tag = "2")]
    pub outfits: Vec<Appearance>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Appearance {
    #[prost(uint32, optional, tag = "1")]
    pub id: Option<u32>,
    #[prost(message, repeated, tag = "2")]
    pub groups: Vec<Group>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Group {
    #[prost(message, optional, tag = "3")]
    pub info: Option<Info>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Info {
    #[prost(uint32, optional, tag = "1")]
    pub width: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    pub height: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub depth: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    pub layers: Option<u32>,
    #[prost(uint32, repeated, packed = "false", tag = "5")]
    pub sprites: Vec<u32>,
    #[prost(message, optional, tag = "6")]
    pub animation: Option<Animation>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Animation {
    #[prost(message, repeated, tag = "6")]
    pub phases: Vec<Phase>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Phase {}

impl Info {
    pub fn dimensions(&self) -> Result<[u32; 4], super::Error> {
        let dimensions = [self.width, self.height, self.depth, self.layers].map(|v| v.unwrap_or(1));
        if dimensions.iter().any(|v| !(1..=64).contains(v)) || dimensions[3] > 16 {
            return Err(super::invalid("invalid appearance dimensions"));
        }
        let phases = self.animation.as_ref().map_or(1, |a| a.phases.len().max(1));
        if phases > 1024 {
            return Err(super::invalid("too many animation phases"));
        }
        let expected = dimensions
            .iter()
            .try_fold(phases, |n, v| n.checked_mul(*v as usize));
        if expected != Some(self.sprites.len()) || self.sprites.len() > 2_000_000 {
            return Err(super::invalid("sprite reference count mismatch"));
        }
        Ok(dimensions)
    }
}
