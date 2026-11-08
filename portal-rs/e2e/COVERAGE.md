# Portal coverage inventory

Source scan: `python3 e2e/inventory.py`. `{lang}` expands to pl and en in all four projects. Unprefixed aliases are retained separately. Source locations identify the owning router; route methods are retained in inventory.json. This inventory is provisional until the executed matrix is reconciled.

| Route | Module crate | Enabling flags | Actor | Spec + case | Result |
| --- | --- | --- | --- | --- | --- |
| `/admin/analytics` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/creatures` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/activity` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/levels` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/health` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/economy` | admin | OTERYN_ADMIN_ANALYTICS_INGEST, OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/{lang}/bazaar` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/commerce` | admin | OTERYN_ADMIN_PANEL, OTERYN_PAYMENTS_ENABLED, OTERYN_PREMIUM_ENABLED, OTERYN_REFUNDS_ENABLED, OTERYN_WALLET_ENABLED | staff | pending | NOT_RUN |
| `/admin/commerce/{action}` | admin | OTERYN_ADMIN_PANEL, OTERYN_PAYMENTS_ENABLED, OTERYN_PREMIUM_ENABLED, OTERYN_REFUNDS_ENABLED, OTERYN_WALLET_ENABLED | staff | pending | NOT_RUN |
| `/admin/commerce/approvals/{id}` | admin | OTERYN_ADMIN_PANEL, OTERYN_PAYMENTS_ENABLED, OTERYN_PREMIUM_ENABLED, OTERYN_REFUNDS_ENABLED, OTERYN_WALLET_ENABLED | staff | pending | NOT_RUN |
| `/admin/console/players` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/console/audit` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/console/access` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/console/approvals` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/console/dashboard` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/console/worlds` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/console/nodes` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/console/database` | admin | OTERYN_ADMIN_CONSOLE, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/control/step-up` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/control/approvals/{id}` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/players` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/players/{id}` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/moderation/enforcement/create` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/roles` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/{lang}/atlas/bestiary/{character}` | atlas | OTERYN_ATLAS_BESTIARY_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/atlas/delivery` | atlas | always mounted | anonymous/player | pending | NOT_RUN |
| `/atlas/delivery.css` | atlas | always mounted | anonymous/player | pending | NOT_RUN |
| `/atlas/spawns-text/{lang}` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_ATLAS_SPAWNS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/atlas/spawns` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_ATLAS_SPAWNS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/atlas/private-markers` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_ATLAS_SPAWNS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/atlas/private-markers/{id}` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_ATLAS_SPAWNS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/atlas/map-reports` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_ATLAS_SPAWNS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/atlas` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/atlas/tiles/{version}/{floor}/{zoom}/{x}/{y}` | atlas | OTERYN_ATLAS_DELIVERY_ENABLED, OTERYN_ATLAS_ENABLED, OTERYN_WORLD_ENABLED, PORTAL_ATLAS_DELIVERY_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/bazaar` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/bazaar` | bazaar | always mounted | owner | pending | NOT_RUN |
| `/{lang}/bazaar/{listing}` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/bazaar/sell` | bazaar | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/bazaar/{listing}/bids` | bazaar | always mounted | owner | pending | NOT_RUN |
| `/{lang}/bazaar/items` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/bazaar/items/new` | bazaar | OTERYN_BAZAAR_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/bazaar/items/{listing}` | bazaar | OTERYN_BAZAAR_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/bazaar/{listing}/control` | bazaar | OTERYN_BAZAAR_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/bazaar/items/{id}` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/bazaar/items/{id}/{action}` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/bazaar/sellers/{seller}` | bazaar | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/bazaar/{listing}/report` | bazaar | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/bazaar/{listing}/promote` | bazaar | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/characters/new` | characters | OTERYN_CHARACTER_COMMANDS_ENABLED, OTERYN_CHARACTER_COMMAND_CREATE_ENABLED, OTERYN_CHARACTER_CREATE_WEB_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/characters/requests/{operation}` | characters | OTERYN_CHARACTER_COMMANDS_ENABLED, OTERYN_CHARACTER_COMMAND_CREATE_ENABLED, OTERYN_CHARACTER_CREATE_WEB_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/characters/suggest` | characters | OTERYN_CHARACTER_COMMANDS_ENABLED, OTERYN_CHARACTER_COMMAND_CREATE_ENABLED, OTERYN_CHARACTER_CREATE_WEB_ENABLED | owner | pending | NOT_RUN |
| `/deaths` | community | PORTAL_DEATHS_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/deaths` | community | PORTAL_DEATHS_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/characters/{name}/deaths` | community | PORTAL_DEATHS_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/characters/{name}/deaths` | community | PORTAL_DEATHS_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/guilds` | community | always mounted | anonymous/player | pending | NOT_RUN |
| `/guilds/{name}` | community | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/guilds` | community | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/guilds/{name}` | community | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/houses` | community | PORTAL_HOUSES_INGEST_ENABLED, PORTAL_HOUSES_SERVING_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/houses` | community | PORTAL_HOUSES_INGEST_ENABLED, PORTAL_HOUSES_SERVING_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/houses/{key}` | community | PORTAL_HOUSES_INGEST_ENABLED, PORTAL_HOUSES_SERVING_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/houses/{key}` | community | PORTAL_HOUSES_INGEST_ENABLED, PORTAL_HOUSES_SERVING_ENABLED, PORTAL_PUBLIC_PROJECTIONS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/online` | community | PORTAL_ONLINE_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/online` | community | PORTAL_ONLINE_ENABLED | anonymous/player | pending | NOT_RUN |
| `/admin/{lang}/party` | community | OTERYN_ADMIN_PANEL, PORTAL_PARTY_ENABLED | staff | pending | NOT_RUN |
| `/admin/{lang}/party/{id}/moderate` | community | OTERYN_ADMIN_PANEL, PORTAL_PARTY_ENABLED | staff | pending | NOT_RUN |
| `/{lang}/party` | community | PORTAL_PARTY_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/party/{id}/report` | community | PORTAL_PARTY_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/party/{id}/remove` | community | PORTAL_PARTY_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/characters` | community | PORTAL_PUBLIC_PROFILES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/characters/{name}` | community | PORTAL_PUBLIC_PROFILES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/rankings/{kind}` | community | PORTAL_PUBLIC_RANKINGS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/today/data` | community | PORTAL_PUBLIC_PROJECTIONS_ENABLED, PORTAL_WORLD_DAILY_INGEST_ENABLED, PORTAL_WORLD_DAILY_PUBLIC_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/today/data` | community | PORTAL_PUBLIC_PROJECTIONS_ENABLED, PORTAL_WORLD_DAILY_INGEST_ENABLED, PORTAL_WORLD_DAILY_PUBLIC_ENABLED | anonymous/player | pending | NOT_RUN |
| `/today/widget` | community | PORTAL_PUBLIC_PROJECTIONS_ENABLED, PORTAL_WORLD_DAILY_INGEST_ENABLED, PORTAL_WORLD_DAILY_PUBLIC_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/today/widget` | community | PORTAL_PUBLIC_PROJECTIONS_ENABLED, PORTAL_WORLD_DAILY_INGEST_ENABLED, PORTAL_WORLD_DAILY_PUBLIC_ENABLED | anonymous/player | pending | NOT_RUN |
| `/worlds` | community | PORTAL_WORLDS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/worlds` | community | PORTAL_WORLDS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/api/v1/worlds` | community | PORTAL_WORLDS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/events` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/events/{slug}` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/events` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/events/{slug}` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/admin/events` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/events/create` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/events/{id}/edit` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/events/{id}` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/events/{id}/status` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/media` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/media/{id}/content` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/media/{id}/thumbnail` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/media/{id}` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/media/{id}/delete` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/news` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/news/{slug}` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/news` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/news/{slug}` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/admin/news` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/news/create` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/news/{id}/edit` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/news/{id}` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/news/{id}/delete` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/pages` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/pages/{slug}/{lang}` | content | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/{lang}/pages/{slug}` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/today` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/today` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/announcements/ticker` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/announcements/ticker` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/updates/{channel}/metadata/{filename}` | downloads | always mounted | anonymous/player | pending | NOT_RUN |
| `/updates/{channel}/policy-v1.json` | downloads | always mounted | anonymous/player | pending | NOT_RUN |
| `/updates/{channel}/targets/{*target}` | downloads | always mounted | anonymous/player | pending | NOT_RUN |
| `/oauth/authorize` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/oauth/token` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/panel/hirelings` | hirelings | PORTAL_HIRELINGS_ENABLED | owner | pending | NOT_RUN |
| `/register/abuse/challenge` | identity | always mounted | anonymous/player | pending | NOT_RUN |
| `/register/abuse.js` | identity | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/notifications` | identity | always mounted | owner | pending | NOT_RUN |
| `/{lang}/unsubscribe` | identity | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/privacy/delete` | identity | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/privacy/delete/cancel` | identity | always mounted | owner | pending | NOT_RUN |
| `/{lang}/password/change` | identity | OTERYN_ACCOUNT_CREDENTIALS | anonymous/player | pending | NOT_RUN |
| `/password/change` | identity | OTERYN_ACCOUNT_CREDENTIALS | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/sudo` | identity | OTERYN_ACCOUNT_CREDENTIALS | owner | pending | NOT_RUN |
| `/account/sudo` | identity | OTERYN_ACCOUNT_CREDENTIALS | owner | pending | NOT_RUN |
| `/{lang}/account/security` | identity | OTERYN_ACCOUNT_CREDENTIALS | owner | pending | NOT_RUN |
| `/account/security` | identity | OTERYN_ACCOUNT_CREDENTIALS | owner | pending | NOT_RUN |
| `/{lang}/account/security/email` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | owner | pending | NOT_RUN |
| `/account/security/email` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | owner | pending | NOT_RUN |
| `/{lang}/email-change/confirm/{token}` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | anonymous/player | pending | NOT_RUN |
| `/email-change/confirm/{token}` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | anonymous/player | pending | NOT_RUN |
| `/{lang}/email-change/revert/{token}` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | anonymous/player | pending | NOT_RUN |
| `/email-change/revert/{token}` | identity | OTERYN_ACCOUNT_EMAIL_CHANGE | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/privacy/export` | identity | OTERYN_ACCOUNT_EXPORT_ENABLED | owner | pending | NOT_RUN |
| `/login` | identity | OTERYN_ACCOUNT_LOGIN | anonymous/player | pending | NOT_RUN |
| `/{lang}/login` | identity | OTERYN_ACCOUNT_LOGIN | anonymous/player | pending | NOT_RUN |
| `/logout` | identity | OTERYN_ACCOUNT_LOGIN | anonymous/player | pending | NOT_RUN |
| `/{lang}/logout` | identity | OTERYN_ACCOUNT_LOGIN | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/login` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/login` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/challenge` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/challenge` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/email` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/email` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/enroll` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/enroll` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/confirm` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/confirm` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/mfa/devices` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/mfa/devices` | identity | OTERYN_ACCOUNT_LOGIN, OTERYN_ACCOUNT_MFA | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/passkeys` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/passkeys/register/start` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/passkeys/register/finish` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/passkeys/authenticate/start` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/passkeys/authenticate/finish` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/passkeys/{id}/remove` | identity | OTERYN_ACCOUNT_PASSKEYS | owner | pending | NOT_RUN |
| `/{lang}/account/privacy` | identity | OTERYN_ACCOUNT_PRIVACY_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/recovery-key` | identity | always mounted | anonymous/player | pending | NOT_RUN |
| `/recovery-key` | identity | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/recovery-key` | identity | always mounted | owner | pending | NOT_RUN |
| `/account/recovery-key` | identity | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/recovery-key/confirm` | identity | always mounted | owner | pending | NOT_RUN |
| `/account/recovery-key/confirm` | identity | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/recovery-key/revoke` | identity | always mounted | owner | pending | NOT_RUN |
| `/account/recovery-key/revoke` | identity | always mounted | owner | pending | NOT_RUN |
| `/forgot-password` | identity | OTERYN_ACCOUNT_PASSWORD_RESET | anonymous/player | pending | NOT_RUN |
| `/reset-password` | identity | OTERYN_ACCOUNT_PASSWORD_RESET | anonymous/player | pending | NOT_RUN |
| `/{lang}/forgot-password` | identity | OTERYN_ACCOUNT_PASSWORD_RESET | anonymous/player | pending | NOT_RUN |
| `/{lang}/reset-password` | identity | OTERYN_ACCOUNT_PASSWORD_RESET | anonymous/player | pending | NOT_RUN |
| `/{lang}/reset-password/{token}` | identity | OTERYN_ACCOUNT_PASSWORD_RESET | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/security/wizard` | identity | OTERYN_SECURITY_WIZARD_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/sessions` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/account/sessions` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/{lang}/account/sessions/others` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/account/sessions/others` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/{lang}/account/sessions/all` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/account/sessions/all` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/{lang}/account/sessions/{id}` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/account/sessions/{id}` | identity | OTERYN_ACCOUNT_SESSIONS | owner | pending | NOT_RUN |
| `/{lang}/collection` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/{character}` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/{character}/decks` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/{character}/cards/{id}/image` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/decks/{id}/battle` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/battles/{id}` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/battles/{id}/forfeit` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection/decks/{id}/delete` | minigames | OTERYN_COLLECTION_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/collection-pvp` | minigames | OTERYN_COLLECTION_PVP_ENABLED, OTERYN_COLLECTION_SETS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/league` | minigames | OTERYN_LEAGUE_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/pass` | minigames | OTERYN_SEASON_PASS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/pass/{season}/buy` | minigames | OTERYN_SEASON_PASS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/pass/{season}/claim` | minigames | OTERYN_SEASON_PASS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/puzzles` | minigames | OTERYN_DAILY_PUZZLES_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/puzzles/{id}/answer` | minigames | OTERYN_DAILY_PUZZLES_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/puzzles/{id}/image` | minigames | OTERYN_DAILY_PUZZLES_ENABLED, OTERYN_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/panel/premium` | payments | PANEL_PREMIUM_PAYMENTS | owner | pending | NOT_RUN |
| `/{lang}/panel/payments` | payments | PANEL_PREMIUM_PAYMENTS | owner | pending | NOT_RUN |
| `/{lang}/panel/payments/receipts/{id}` | payments | PANEL_PREMIUM_PAYMENTS | owner | pending | NOT_RUN |
| `/api/v1/account/summary` | player-api | always mounted | owner | pending | NOT_RUN |
| `/api/v1/feed` | player-api | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/tickets` | player-api | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/native-characters` | player-api | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/dashboard` | playerdata | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/journal/{character}/deaths` | playerdata | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/hunts` | playerdata | OTERYN_PLAYERDATA_HUNTS_READ | owner | pending | NOT_RUN |
| `/{lang}/account/hunts/compare` | playerdata | OTERYN_PLAYERDATA_HUNTS_READ | owner | pending | NOT_RUN |
| `/{lang}/account/hunts/{id}` | playerdata | OTERYN_PLAYERDATA_HUNTS_READ | owner | pending | NOT_RUN |
| `/{lang}/account/hunts/{id}/share` | playerdata | OTERYN_PLAYERDATA_HUNTS_READ | owner | pending | NOT_RUN |
| `/static/playerdata/hunts.css` | playerdata | OTERYN_PLAYERDATA_HUNTS_READ | anonymous/player | pending | NOT_RUN |
| `/{lang}/panel/items/{world}/{character}` | playerdata | always mounted | owner | pending | NOT_RUN |
| `/{lang}/panel/history/{world}/{family}` | playerdata | always mounted | owner | pending | NOT_RUN |
| `/{lang}/panel/history/{world}/{family}/{character}` | playerdata | always mounted | owner | pending | NOT_RUN |
| `/{lang}/account/game-logins` | playerdata | OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED, OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED | owner | pending | NOT_RUN |
| `/account/game-logins` | playerdata | OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED, OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/atlas/places` | playerdata | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/atlas/places/participation` | playerdata | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/account/journal/reports` | playerdata | OTERYN_JOURNAL_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/journal/reports/notes` | playerdata | OTERYN_JOURNAL_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/journal/reports/notes/export` | playerdata | OTERYN_JOURNAL_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/journal/reports/email` | playerdata | OTERYN_JOURNAL_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/stats/characters/{id}/{tab}` | stats | PORTAL_CHARACTER_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/stats/compare` | stats | PORTAL_CHARACTER_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/stats/guilds` | stats | PORTAL_GUILD_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/stats/guilds` | stats | PORTAL_GUILD_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/stats/rankings/{kind}` | stats | PORTAL_GROWTH_RANKINGS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/api/v1/stats/watches` | stats | PORTAL_STATS_WATCHES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/stats/world` | stats | PORTAL_WORLD_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/stats/world` | stats | PORTAL_WORLD_STATS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/shop` | store | OTERYN_SHOP_ENABLED, OTERYN_SHOP_TOP_UPS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/shop/coins` | store | OTERYN_SHOP_ENABLED, OTERYN_SHOP_TOP_UPS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/shop/history` | store | OTERYN_SHOP_ENABLED, OTERYN_SHOP_TOP_UPS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/shop/transfer` | store | OTERYN_SHOP_ENABLED, OTERYN_SHOP_TOP_UPS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/__test/payments/webhook` | store | OTERYN_PAYMENTS_ENABLED, OTERYN_SHOP_TEST_PAYMENTS_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/support/reports` | support | OTERYN_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/support/reports/create` | support | OTERYN_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/support/reports/{id}` | support | OTERYN_REPORTS_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/support/tickets` | support | OTERYN_SUPPORT_TICKETS | owner | pending | NOT_RUN |
| `/{lang}/support/tickets/create` | support | OTERYN_SUPPORT_TICKETS | owner | pending | NOT_RUN |
| `/{lang}/support/tickets/{id}` | support | OTERYN_SUPPORT_TICKETS | owner | pending | NOT_RUN |
| `/{lang}/support/tickets/{id}/reply` | support | OTERYN_SUPPORT_TICKETS | owner | pending | NOT_RUN |
| `/{lang}/support/tickets/{id}/status` | support | OTERYN_SUPPORT_TICKETS | owner | pending | NOT_RUN |
| `/{lang}/admin/support/tickets` | support | OTERYN_ADMIN_PANEL, OTERYN_SUPPORT_TICKETS | staff | pending | NOT_RUN |
| `/{lang}/admin/support/tickets/{id}` | support | OTERYN_ADMIN_PANEL, OTERYN_SUPPORT_TICKETS | staff | pending | NOT_RUN |
| `/{lang}/admin/support/tickets/{id}/reply` | support | OTERYN_ADMIN_PANEL, OTERYN_SUPPORT_TICKETS | staff | pending | NOT_RUN |
| `/{lang}/admin/support/tickets/{id}/status` | support | OTERYN_ADMIN_PANEL, OTERYN_SUPPORT_TICKETS | staff | pending | NOT_RUN |
| `/favicon.ico` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/healthz` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/readyz` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/account` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | owner | pending | NOT_RUN |
| `/account/badges` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | owner | pending | NOT_RUN |
| `/account/characters/{name}/profile` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/characters/{name}/profile` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | owner | pending | NOT_RUN |
| `/{lang}/account/characters/{character}/services` | web | OTERYN_ACCOUNT_OVERVIEW_ENABLED, OTERYN_ACCOUNT_RECOVERY_KEYS, OTERYN_CHARACTER_PROFILES_ENABLED | owner | pending | NOT_RUN |
| `/register` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/register` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/register/check-inbox` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/register/check-inbox` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/verify-email` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/verify-email` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/verify-email/resend` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/verify-email/resend` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/search` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/search` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/robots.txt` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/sitemap.xml` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/language/{lang}` | web | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/calculators` | wiki | PORTAL_CALCULATORS_ENABLED, PORTAL_WORLD_ENABLED | anonymous/player | pending | NOT_RUN |
| `/wiki/{kind}` | wiki | PORTAL_WIKI_ENTITIES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/wiki/{kind}/{key}` | wiki | PORTAL_WIKI_ENTITIES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/{kind}` | wiki | PORTAL_WIKI_ENTITIES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/{kind}/{key}` | wiki | PORTAL_WIKI_ENTITIES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/places` | wiki | PORTAL_WIKI_PLACES_ENABLED | anonymous/player | pending | NOT_RUN |
| `/wiki` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/wiki/search` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/wiki/articles/{slug}` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/search` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/wiki/articles/{slug}` | wiki | always mounted | anonymous/player | pending | NOT_RUN |
| `/admin/wiki` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/create` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}/edit` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}/revisions` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}/preview/{lang}` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}/revisions/{source}/restore` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/wiki/articles/{id}/{action}` | wiki | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/world` | world | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/{lang}/rules` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/server-information` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/legal/terms` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/legal/privacy` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/legal/cookies` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/support/report-a-bug` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/help` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/{lang}/support` | content | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/device-sessions` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/device-sessions/native-characters` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/device-sessions/tickets` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/device-sessions/revoke` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/api/v1/game-auth/device-sessions/{*rest}` | gameauth | always mounted | anonymous/player | pending | NOT_RUN |
| `/admin/moderation/reports` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/moderation/reports/{id}` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/moderation/enforcement` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/moderation/enforcement/{id}` | admin | OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/creatures/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/activity/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/levels/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/health/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |
| `/admin/analytics/economy/data` | admin | OTERYN_ADMIN_ANALYTICS_PRESENT, OTERYN_ADMIN_PANEL | staff | pending | NOT_RUN |

## Registry sections

| Section | Route | Group | Module | Footer | Result |
| --- | --- | --- | --- | --- | --- |
| account-overview | `/account` | Account | identity | False | NOT_RUN |
| account-security | `password/change` | Account | identity | False | NOT_RUN |
| account-wizard | `account/security/wizard` | Account | identity | False | NOT_RUN |
| account-passkeys | `account/passkeys` | Account | identity | False | NOT_RUN |
| account-recovery-key | `account/recovery-key` | Account | identity | False | NOT_RUN |
| account-sessions | `account/sessions` | Account | identity | False | NOT_RUN |
| account-privacy | `account/privacy` | Account | identity | False | NOT_RUN |
| account-notifications | `account/notifications` | Account | identity | False | NOT_RUN |
| create-character | `account/characters/new` | Account | characters | False | NOT_RUN |
| shop | `shop` | Play | store | False | NOT_RUN |
| news | `news` | Community | content | True | NOT_RUN |
| events | `events` | Community | content | False | NOT_RUN |
| support | `support/report-a-bug` | Community | content | True | NOT_RUN |
| privacy | `legal/privacy` | Account | content | True | NOT_RUN |
| terms | `legal/terms` | Account | content | True | NOT_RUN |
| today | `today` | Community | content | False | NOT_RUN |
| rules | `rules` | World | content | False | NOT_RUN |
| server | `server-information` | World | content | False | NOT_RUN |
| wiki | `wiki` | World | wiki | True | NOT_RUN |
| calculators | `wiki/calculators` | World | wiki | False | NOT_RUN |
| worlds | `worlds` | World | community | False | NOT_RUN |
| online | `online` | Community | community | False | NOT_RUN |
| rankings | `rankings/experience` | Community | community | False | NOT_RUN |
| party | `party` | Community | community | False | NOT_RUN |
| guilds | `guilds` | Community | community | False | NOT_RUN |
| houses | `houses` | World | community | False | NOT_RUN |
| deaths | `deaths` | Community | community | False | NOT_RUN |
| world-stats | `stats/world` | World | stats | False | NOT_RUN |
| guild-stats | `stats/guilds` | Community | stats | False | NOT_RUN |
| growth-rankings | `stats/rankings/yesterday` | Community | stats | False | NOT_RUN |
| compare | `stats/compare` | Community | stats | False | NOT_RUN |
| atlas | `atlas` | World | atlas | False | NOT_RUN |
| delivery | `atlas/delivery` | World | atlas | False | NOT_RUN |
| league | `league` | Play | minigames | False | NOT_RUN |
| puzzles | `puzzles` | Play | minigames | False | NOT_RUN |
| collection | `collection` | Play | minigames | False | NOT_RUN |
| pass | `pass` | Play | minigames | False | NOT_RUN |
| hirelings | `panel/hirelings` | Account | hirelings | False | NOT_RUN |
| hunts | `account/hunts` | Account | playerdata | False | NOT_RUN |
| reports | `account/journal/reports` | Account | playerdata | False | NOT_RUN |
| logins | `account/game-logins` | Account | playerdata | False | NOT_RUN |
| places | `atlas/places` | World | playerdata | False | NOT_RUN |
| premium | `panel/premium` | Account | payments | False | NOT_RUN |
| payments | `panel/payments` | Account | payments | False | NOT_RUN |
| tickets | `support/tickets` | Account | support | False | NOT_RUN |
| abuse-reports | `support/reports` | Account | support | False | NOT_RUN |
| bazaar | `bazaar` | World | bazaar | False | NOT_RUN |
| account-bazaar | `account/bazaar` | Account | bazaar | False | NOT_RUN |
