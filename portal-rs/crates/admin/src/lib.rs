//! RBAC, audit log, four-eyes approvals and the staff panel shell (design T10).

pub mod audit;
pub mod rbac;

pub mod web;

pub mod commerce;
pub mod players;
pub mod support;

pub mod console {
    pub mod access;
    pub mod approvals;
    pub mod audit;
    pub mod players;
}
