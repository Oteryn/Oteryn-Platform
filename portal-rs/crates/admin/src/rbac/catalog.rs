//! Stable permission vocabulary and built-in role defaults.

use super::Error;

/// Stable identifiers shared with the existing portal. Unknown strings never authorize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    Access,
    ManageRoles,
    ViewAudit,
    ManageNews,
    ManagePages,
    ManageMedia,
    PortalAccess,
    ManagePortalAnnouncements,
    ManagePortalSettings,
    ManageDownloads,
    ManageEvents,
    PublishEvents,
    ManageSupportContent,
    ManageSupportTickets,
    ManageSupportReports,
    ManageSupportEnforcement,
    ManageMarketplace,
    ManagePremiumTime,
    ReconcilePayments,
    GameCatalogAccess,
    ViewGameCatalogSnapshots,
    ImportGameCatalogSnapshots,
    ActivateGameCatalogSnapshots,
    ManageGameCatalogProfiles,
    ManageGameCatalogTranslations,
    ManageGameCatalogOverrides,
    WikiAccess,
    ManageWikiArticles,
    ManageWikiCategories,
    PublishWiki,
}

impl Permission {
    pub const ALL: &'static [Self] = &[
        Self::Access,
        Self::ManageRoles,
        Self::ViewAudit,
        Self::ManageNews,
        Self::ManagePages,
        Self::ManageMedia,
        Self::PortalAccess,
        Self::ManagePortalAnnouncements,
        Self::ManagePortalSettings,
        Self::ManageDownloads,
        Self::ManageEvents,
        Self::PublishEvents,
        Self::ManageSupportContent,
        Self::ManageSupportTickets,
        Self::ManageSupportReports,
        Self::ManageSupportEnforcement,
        Self::ManageMarketplace,
        Self::ManagePremiumTime,
        Self::ReconcilePayments,
        Self::GameCatalogAccess,
        Self::ViewGameCatalogSnapshots,
        Self::ImportGameCatalogSnapshots,
        Self::ActivateGameCatalogSnapshots,
        Self::ManageGameCatalogProfiles,
        Self::ManageGameCatalogTranslations,
        Self::ManageGameCatalogOverrides,
        Self::WikiAccess,
        Self::ManageWikiArticles,
        Self::ManageWikiCategories,
        Self::PublishWiki,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Access => "admin.access",
            Self::ManageRoles => "admin.roles.manage",
            Self::ViewAudit => "audit.view",
            Self::ManageNews => "cms.news.manage",
            Self::ManagePages => "cms.pages.manage",
            Self::ManageMedia => "media.manage",
            Self::PortalAccess => "portal.access",
            Self::ManagePortalAnnouncements => "portal.announcements.manage",
            Self::ManagePortalSettings => "portal.settings.manage",
            Self::ManageDownloads => "downloads.manage",
            Self::ManageEvents => "events.manage",
            Self::PublishEvents => "events.publish",
            Self::ManageSupportContent => "support.content.manage",
            Self::ManageSupportTickets => "support.tickets.manage",
            Self::ManageSupportReports => "support.reports.manage",
            Self::ManageSupportEnforcement => "support.enforcement.manage",
            Self::ManageMarketplace => "marketplace.manage",
            Self::ManagePremiumTime => "products.premium.manage",
            Self::ReconcilePayments => "payments.reconcile",
            Self::GameCatalogAccess => "game_catalog.access",
            Self::ViewGameCatalogSnapshots => "game_catalog.snapshots.view",
            Self::ImportGameCatalogSnapshots => "game_catalog.snapshots.import",
            Self::ActivateGameCatalogSnapshots => "game_catalog.snapshots.activate",
            Self::ManageGameCatalogProfiles => "game_catalog.profiles.manage",
            Self::ManageGameCatalogTranslations => "game_catalog.translations.manage",
            Self::ManageGameCatalogOverrides => "game_catalog.overrides.manage",
            Self::WikiAccess => "wiki.access",
            Self::ManageWikiArticles => "wiki.articles.manage",
            Self::ManageWikiCategories => "wiki.categories.manage",
            Self::PublishWiki => "wiki.publish",
        }
    }
}

impl std::str::FromStr for Permission {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|p| p.as_str() == value)
            .ok_or(Error::InvalidInput)
    }
}

/// Starting policy only; effective permissions are always read from storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInRole {
    PlatformAdmin,
    SecurityAdmin,
    ContentEditor,
    Support,
    Moderator,
    Operations,
}

impl BuiltInRole {
    pub const ALL: &'static [Self] = &[
        Self::PlatformAdmin,
        Self::SecurityAdmin,
        Self::ContentEditor,
        Self::Support,
        Self::Moderator,
        Self::Operations,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::PlatformAdmin => "platform_admin",
            Self::SecurityAdmin => "security_admin",
            Self::ContentEditor => "content_editor",
            Self::Support => "support",
            Self::Moderator => "moderator",
            Self::Operations => "operations",
        }
    }

    pub const fn permissions(self) -> &'static [Permission] {
        match self {
            Self::PlatformAdmin => &[
                Permission::Access,
                Permission::ManageRoles,
                Permission::ViewAudit,
                Permission::ManageNews,
                Permission::ManagePages,
                Permission::ManageMedia,
                Permission::PortalAccess,
                Permission::ManagePortalAnnouncements,
                Permission::ManagePortalSettings,
                Permission::ManageDownloads,
                Permission::ManageEvents,
                Permission::PublishEvents,
                Permission::ManageSupportContent,
                Permission::ManageSupportTickets,
                Permission::ManageSupportReports,
                Permission::ManageSupportEnforcement,
                Permission::ManageMarketplace,
                Permission::ManagePremiumTime,
                Permission::ReconcilePayments,
                Permission::GameCatalogAccess,
                Permission::ViewGameCatalogSnapshots,
                Permission::ImportGameCatalogSnapshots,
                Permission::ActivateGameCatalogSnapshots,
                Permission::ManageGameCatalogProfiles,
                Permission::ManageGameCatalogTranslations,
                Permission::ManageGameCatalogOverrides,
                Permission::WikiAccess,
                Permission::ManageWikiArticles,
                Permission::ManageWikiCategories,
                Permission::PublishWiki,
            ],
            Self::SecurityAdmin => &[
                Permission::Access,
                Permission::ManageRoles,
                Permission::ViewAudit,
            ],
            Self::ContentEditor => &[
                Permission::ManageNews,
                Permission::ManagePages,
                Permission::ManageMedia,
                Permission::ManagePortalAnnouncements,
                Permission::ManageEvents,
                Permission::PublishEvents,
                Permission::ManageSupportContent,
                Permission::WikiAccess,
                Permission::ManageWikiArticles,
                Permission::ManageWikiCategories,
                Permission::PublishWiki,
                Permission::Access,
                Permission::PortalAccess,
            ],
            Self::Support => &[
                Permission::Access,
                Permission::ManageSupportTickets,
                Permission::ManageSupportReports,
            ],
            Self::Moderator => &[
                Permission::Access,
                Permission::ManageSupportEnforcement,
                Permission::ManageMarketplace,
            ],
            Self::Operations => &[
                Permission::Access,
                Permission::ManageDownloads,
                Permission::PortalAccess,
                Permission::ManagePortalSettings,
                Permission::GameCatalogAccess,
                Permission::ViewGameCatalogSnapshots,
                Permission::ImportGameCatalogSnapshots,
                Permission::ActivateGameCatalogSnapshots,
                Permission::ManageGameCatalogProfiles,
                Permission::ManageGameCatalogTranslations,
                Permission::ManageGameCatalogOverrides,
            ],
        }
    }
}
