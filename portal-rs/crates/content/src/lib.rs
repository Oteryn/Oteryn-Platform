//! News, pages, events, announcements and media (design T11, T12).

pub mod events;
pub mod news;
pub mod pages;

pub mod announcements;
pub mod media;
pub mod today;

/// Shared portal navigation registration.
pub mod navigation;

/// Bounded page offset shared by public and staff content libraries.
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct Pagination {
    #[serde(default)]
    pub page: u32,
}
impl Pagination {
    pub fn offset(&self, size: i64) -> i64 {
        i64::from(self.page) * size
    }
    pub fn next(&self) -> u64 {
        u64::from(self.page) + 1
    }
}

#[cfg(test)]
#[path = "../../admin/src/test_staff.rs"]
pub mod test_staff;

#[cfg(test)]
mod pagination_tests;
