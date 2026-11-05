//! RBAC, audit log, four-eyes approvals and the staff panel shell (design T10).

pub mod audit;
pub mod rbac;

pub mod web;

pub mod bazaar;
pub mod commerce;
#[path = "control/controls.rs"]
pub mod controls;
pub mod players;
pub mod support;

pub mod console;
pub mod control;
