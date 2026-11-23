//! Shared navigation for the player, admin and GM panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Player,
    Admin,
    Gm,
}

impl Panel {
    pub const fn root(self) -> &'static str {
        match self {
            Self::Player => "/account",
            Self::Admin => "/admin",
            Self::Gm => "/console",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Player => "panel-player",
            Self::Admin => "panel-admin",
            Self::Gm => "panel-gm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    AccountOwner,
    AccountPasskeys,
    Staff(&'static str),
    PasskeyManagement,
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub panel: Panel,
    /// Local route or localized route suffix. Use `{lang}` for a localized route or query parameter.
    pub route: &'static str,
    pub label: &'static str,
    pub permission: Permission,
}

#[derive(Clone, Debug)]
pub struct Link {
    pub url: String,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct Registry(Vec<Entry>);

impl Registry {
    pub fn register(&mut self, enabled: bool, entry: Entry) {
        if enabled
            && !self
                .0
                .iter()
                .any(|e| e.panel == entry.panel && e.route == entry.route)
        {
            self.0.push(entry);
        }
    }

    pub fn visible(
        &self,
        panel: Panel,
        allows: impl Fn(Permission) -> bool,
    ) -> impl Iterator<Item = &Entry> {
        self.0
            .iter()
            .filter(move |entry| entry.panel == panel && allows(entry.permission))
    }

    pub fn links(
        &self,
        panel: Panel,
        lang: &str,
        allows: impl Fn(Permission) -> bool,
    ) -> Vec<Link> {
        let lang = if lang == "en" { "en" } else { "pl" };
        self.visible(panel, allows)
            .map(|entry| Link {
                url: if entry.route.starts_with('/') {
                    entry.route.replace("{lang}", lang)
                } else {
                    format!("/{lang}/{}", entry.route)
                },
                label: crate::text(lang, entry.label),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_is_scoped_to_panel_and_permission() {
        let mut registry = Registry::default();
        for (panel, route, label, permission, enabled) in [
            (
                Panel::Player,
                "/account",
                "panel-player",
                Permission::AccountOwner,
                true,
            ),
            (
                Panel::Admin,
                "/admin",
                "panel-admin",
                Permission::Staff("admin.access"),
                true,
            ),
            (
                Panel::Admin,
                "/admin/disabled",
                "panel-admin",
                Permission::Staff("admin.access"),
                false,
            ),
            (
                Panel::Gm,
                "/console/worlds?lang={lang}",
                "panel-gm",
                Permission::Staff("game.world.read"),
                true,
            ),
        ] {
            registry.register(
                enabled,
                Entry {
                    panel,
                    route,
                    label,
                    permission,
                },
            );
        }
        assert_eq!(registry.visible(Panel::Player, |_| false).count(), 0);
        assert_eq!(
            registry
                .visible(Panel::Player, |p| p == Permission::AccountOwner)
                .count(),
            1
        );
        assert_eq!(registry.visible(Panel::Admin, |_| true).count(), 1);
        assert!(
            registry
                .links(Panel::Gm, "en", |p| p == Permission::Staff("admin.access"))
                .is_empty()
        );
        for lang in ["pl", "en"] {
            let links = registry.links(Panel::Gm, lang, |p| {
                p == Permission::Staff("game.world.read")
            });
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].url, format!("/console/worlds?lang={lang}"));
            assert!(!links[0].label.is_empty());
        }
    }
}
