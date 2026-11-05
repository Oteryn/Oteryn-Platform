# Migration redirects

Targets use the default Polish locale. Unknown paths return 404. Only page, q, world and vocation query parameters survive. Legacy recovery tokens were not imported; their routes lead to recovery or security instead. Authenticated account/admin routes are excluded.

| Legacy GET | Destination |
| --- | --- |
| /support | /pl/support |
| /support/report-a-bug | /pl/support/report-a-bug |
| /legal/terms | /pl/legal/terms |
| /legal/privacy | /pl/legal/privacy |
| /legal/cookies | /pl/legal/cookies |
| /today | /pl/today |
| /server-information | /pl/server-information |
| /rules | /pl/rules |
| /guilds | /pl/guilds |
| /deaths | /pl/deaths |
| /events | /pl/events |
| /events/{slug} | /pl/events/{slug} |
| /download | /pl/download |
| /download/{slug} | /pl/download/{slug} |
| /wiki | /pl/wiki |
| /wiki/search | /pl/wiki/search |
| /wiki/items | /pl/wiki/items |
| /wiki/creatures | /pl/wiki/creatures |
| /register | /pl/register |
| /login | /pl/login |
| /mfa/challenge | /pl/mfa/challenge |
| /forgot-password | /pl/forgot-password |
| /reset-password/{token} | /pl/forgot-password |
| /recovery-key | /pl/recovery-key |
| /email-change/confirm/{token} | /pl/account/security |
| /email-change/recover/{token} | /pl/account/security |
| /news | /pl/news |
| /news/{slug} | /pl/news/{slug} |
| /pages/{slug} | /pl/pages/{slug} |
| /highscores | /pl/rankings/level |
| /characters | /pl/characters |
| /characters/{name} | /pl/characters/{name} |
| /guilds/{name} | /pl/guilds/{name} |
| /online | /pl/online |
| /servers | /pl/worlds |
| /getting-started | /pl/start |
| /wiki/{slug} | /pl/wiki/articles/{slug} |
| /wiki/items/{slug} | /pl/wiki/items/{slug} |
| /wiki/creatures/{slug} | /pl/wiki/creatures/{slug} |

SEO configuration uses OTERYN_PUBLIC_ORIGIN and OTERYN_ENV=production. Nonproduction disallows all crawlers. Modules register published pages through Seo::register before router composition; the binary registers the home page; content modules supply further published URLs.
