//! Game world data read-only from a pinned game release: entities, links, map and sprites (design T14, T42).

mod import;
mod store;
mod types;

pub use store::WorldStore;
pub use types::{
    Entity, ImportConfig, ImportIssue, ImportMapping, ImportReport, SchemaScope, VersionStatus,
    WorldError, WorldOptions,
};
