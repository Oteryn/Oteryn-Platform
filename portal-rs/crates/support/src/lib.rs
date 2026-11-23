//! Support tickets, appeals and sanctions executed in game (design T33, T34).

pub mod appeals;
mod attachments;
pub mod sanctions;
pub mod tickets;

pub mod reports;

/// Shared portal navigation registration.
pub mod navigation;

#[cfg(test)]
#[path = "../../admin/src/test_staff.rs"]
pub mod test_staff;
