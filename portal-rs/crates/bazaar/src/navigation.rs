//! Public page navigation owned by this module.
use oteryn_ui::{Group, Registry};

/// Only present Bazaar navigation when the runtime serves the browser pages.
pub fn register(registry: &mut Registry, runtime_available: bool) {
    registry.section(
        runtime_available,
        "bazaar",
        "shell-bazaar",
        "bazaar",
        Group::World,
        false,
    );
    registry.section(
        runtime_available,
        "account-bazaar",
        "shell-account-bazaar",
        "account/bazaar",
        Group::Account,
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_ui::Account;

    #[test]
    fn both_bazaar_links_follow_runtime_availability() {
        for runtime_available in [false, true] {
            let mut registry = Registry::default();
            register(&mut registry, runtime_available);
            for lang in ["en", "pl"] {
                let chrome = registry.chrome(lang, "/", Account::Anonymous);
                for (id, path) in [("bazaar", "bazaar"), ("account-bazaar", "account/bazaar")] {
                    assert_eq!(
                        chrome.link(id).map(|link| link.url.clone()),
                        runtime_available.then(|| format!("/{lang}/{path}"))
                    );
                }
            }
        }
    }
}
