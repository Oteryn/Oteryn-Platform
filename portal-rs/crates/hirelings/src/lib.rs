//! Platform hireling intents. Game outcomes remain authoritative behind G-14.
mod service;
pub use service::*;

fn text(lang: &str, key: &str) -> Result<String> {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = match lang {
        "pl" => include_str!("../locales/pl.ftl"),
        "en" => include_str!("../locales/en.ftl"),
        _ => return Err("unsupported locale".into()),
    };
    let resource =
        FluentResource::try_new(source.to_owned()).map_err(|_| "invalid translations")?;
    let mut bundle = FluentBundle::new(vec![lang.parse()?]);
    bundle
        .add_resource(resource)
        .map_err(|_| "duplicate translations")?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or("missing translation")?;
    let mut errors = Vec::new();
    let result = bundle
        .format_pattern(pattern, None, &mut errors)
        .into_owned();
    if !errors.is_empty() {
        return Err("invalid translation pattern".into());
    }
    Ok(result)
}
pub mod web;

pub mod commands;

/// Shared portal navigation registration.
pub mod navigation;
