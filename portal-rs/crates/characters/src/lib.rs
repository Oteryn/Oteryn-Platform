//! Account character projection, character create intents and public profiles (design T18, T22, T23).
pub mod commands;

pub mod lcfa;
pub mod restore;

pub mod badges;
pub mod bootstrap;
pub mod create_web;
pub mod overview;

pub mod profile;
pub mod services;

/// Shared portal navigation registration.
pub mod navigation;
