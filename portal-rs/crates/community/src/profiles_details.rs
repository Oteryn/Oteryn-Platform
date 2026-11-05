//! Narrow read interface for accepted G-4 families. No identity is a public URL.
use super::Unavailable;
use std::{future::Future, pin::Pin};

#[derive(Debug, Clone, Default)]
pub struct Visibility {
    pub guild: bool,
    pub house: bool,
    pub skills: bool,
    pub deaths: bool,
    /// Plain text from the current Platform preference, never HTML.
    pub comment: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Subject {
    pub character_id: String,
    pub world_id: String,
    pub epoch: String,
}
#[derive(Debug, Clone, Default)]
pub struct Details {
    pub guild: Option<(String, String)>,
    pub house: Option<String>,
    /// Magic, fist, club, sword, axe, distance, shielding, fishing, in that order.
    pub skills: Option<[u16; 8]>,
    pub deaths: Option<Vec<Death>>,
}
#[derive(Debug, Clone)]
pub struct Death {
    pub occurred_at: i64,
    pub level: u32,
}
pub type DetailsFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Details, Unavailable>> + Send + 'a>>;
/// The adapter must use accepted G-4 facts at the subject's current epoch, with a
/// complete watermark at most 29 seconds old (at most 1 second in the future).
/// It must recheck the global epoch fence, World and name-handoff barrier.
/// None means unavailable, not an authoritative empty record. Deaths require
/// occurrence IDs internally, descending (time, id), unique and bounded to 20.
/// Historical killer names cannot prove current identity and are never rendered.
/// House owner names cannot establish a persistent ownership join.
/// Equipment, outfit and achievements have no public representation here.
pub trait PublicDetails: Send + Sync {
    fn current<'a>(&'a self, subject: &'a Subject) -> DetailsFuture<'a>;
}
#[derive(Debug)]
pub struct MissingDetails;
impl PublicDetails for MissingDetails {
    fn current<'a>(&'a self, _: &'a Subject) -> DetailsFuture<'a> {
        Box::pin(async { Err(Unavailable) })
    }
}
