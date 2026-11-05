use fluent_bundle::{FluentBundle, FluentResource};

pub(super) fn text(lang: &str, key: &str) -> String {
    let source = match lang {
        "pl" => include_str!("pl.ftl"),
        "en" => include_str!("en.ftl"),
        _ => return String::new(),
    };
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let Ok(language) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![language]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
