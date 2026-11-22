# Portal ledger coverage delta

Tested revision `aad9438f9ee7e8c552b1be3da9a4d658d4aea347`.

66 of 131 baseline gaps hit by passing tests, including 34 form rows. 65 baseline gaps remain.

| Row | Method and path | Principal, credential, and status | Passing test and project |
| --- | --- | --- | --- |
| F-admin-4597872030189 | GET `/admin/analytics/health` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `en-390` |
| F-admin-4597872030189 | GET `/admin/analytics/health` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `pl-1440` |
| F-admin-12019668692504 | GET `/admin/analytics` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `en-390` |
| F-admin-12019668692504 | GET `/admin/analytics` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `pl-1440` |
| F-admin-72964517267296 | POST `/admin/roles` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff changes a custom role permission through the roles form` · `en-390` |
| F-admin-72964517267296 | POST `/admin/roles` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff changes a custom role permission through the roles form` · `pl-1440` |
| F-admin-80739904374210 | GET `/admin/analytics/levels` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `en-390` |
| F-admin-80739904374210 | GET `/admin/analytics/levels` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `pl-1440` |
| F-admin-209133946605616 | GET `/admin/analytics/activity` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `en-390` |
| F-admin-209133946605616 | GET `/admin/analytics/activity` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `pl-1440` |
| F-admin-244723736915658 | GET `/admin/analytics/creatures` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `en-390` |
| F-admin-244723736915658 | GET `/admin/analytics/creatures` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff submits filters on each analytics page` · `pl-1440` |
| R-admin-273544798622400 | POST `/admin/control/approvals/{id}` | `platform_admin` [session] 403 | `specs/ledger-unhit-admin.spec.mjs` · `staff approval requires an existing control preview` · `en-390` |
| R-admin-273544798622400 | POST `/admin/control/approvals/{id}` | `platform_admin` [session] 403 | `specs/ledger-unhit-admin.spec.mjs` · `staff approval requires an existing control preview` · `pl-1440` |
| R-atlas-130633923692927 | POST `/atlas/private-markers` | `player` [session] 201 | `specs/ledger-unhit-account.spec.mjs` · `player creates and deletes a private atlas marker with a session CSRF token` · `en-390` |
| R-atlas-130633923692927 | POST `/atlas/private-markers` | `player` [session] 201 | `specs/ledger-unhit-account.spec.mjs` · `player creates and deletes a private atlas marker with a session CSRF token` · `pl-1440` |
| R-atlas-130633923692927 | POST `/atlas/private-markers` | `player` [session] 403 | `specs/ledger-unhit-account.spec.mjs` · `player creates and deletes a private atlas marker with a session CSRF token` · `en-390` |
| R-atlas-130633923692927 | POST `/atlas/private-markers` | `player` [session] 403 | `specs/ledger-unhit-account.spec.mjs` · `player creates and deletes a private atlas marker with a session CSRF token` · `pl-1440` |
| R-bazaar-35352714986507 | POST `/{lang}/account/bazaar/{listing}/promote` | `player` [session] 403 | `specs/ledger-unhit-bazaar.spec.mjs` · `Game-backed buyer can edit the owned profile and seller promotion fails closed` · `en-390` |
| R-bazaar-35352714986507 | POST `/{lang}/account/bazaar/{listing}/promote` | `player` [session] 403 | `specs/ledger-unhit-bazaar.spec.mjs` · `Game-backed buyer can edit the owned profile and seller promotion fails closed` · `pl-1440` |
| F-community-98993181248635 | GET `/{lang}/characters` | `anonymous` [none] 302 | `specs/ledger-unhit-public.spec.mjs` · `character search submits the name to the public profile lookup` · `en-390` |
| F-community-98993181248635 | GET `/{lang}/characters` | `anonymous` [none] 302 | `specs/ledger-unhit-public.spec.mjs` · `character search submits the name to the public profile lookup` · `pl-1440` |
| F-community-188835548187730 | GET `/{lang}/online` | `player` [session] 200 | `specs/privacy.spec.mjs` · `ledger-unhit roster filters exclude other worlds and reorder visible characters` · `en-390` |
| F-community-188835548187730 | GET `/{lang}/online` | `player` [session] 200 | `specs/privacy.spec.mjs` · `ledger-unhit roster filters exclude other worlds and reorder visible characters` · `pl-1440` |
| F-community-215566307321900 | GET `/online` | `player` [session] 200 | `specs/privacy.spec.mjs` · `ledger-unhit roster filters exclude other worlds and reorder visible characters` · `en-390` |
| F-community-215566307321900 | GET `/online` | `player` [session] 200 | `specs/privacy.spec.mjs` · `ledger-unhit roster filters exclude other worlds and reorder visible characters` · `pl-1440` |
| F-content-74217417116926 | POST `/admin/announcements/{id}/edit` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates and edits a bilingual announcement` · `en-390` |
| F-content-74217417116926 | POST `/admin/announcements/{id}/edit` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates and edits a bilingual announcement` · `pl-1440` |
| F-content-74382719931163 | POST `/admin/pages/{slug}/{lang}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `en-390` |
| F-content-74382719931163 | POST `/admin/pages/{slug}/{lang}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `pl-1440` |
| F-content-157588388585825 | GET `/admin/pages` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `en-390` |
| F-content-157588388585825 | GET `/admin/pages` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `pl-1440` |
| F-content-212759397105307 | POST `/admin/pages/{slug}/{lang}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `en-390` |
| F-content-212759397105307 | POST `/admin/pages/{slug}/{lang}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates a page through the finder, publishes it and deletes it` · `pl-1440` |
| F-content-277970024806169 | POST `/admin/announcements` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates and edits a bilingual announcement` · `en-390` |
| F-content-277970024806169 | POST `/admin/announcements` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff creates and edits a bilingual announcement` · `pl-1440` |
| R-content-39960494460558 | PUT `/admin/events/{id}/status` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `en-390` |
| R-content-39960494460558 | PUT `/admin/events/{id}/status` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `pl-1440` |
| R-content-63765776539603 | PUT `/admin/events/{id}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `en-390` |
| R-content-63765776539603 | PUT `/admin/events/{id}` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `pl-1440` |
| R-content-141483568009117 | PUT `/admin/news/{id}` | `platform_admin` [session] 302 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `en-390` |
| R-content-141483568009117 | PUT `/admin/news/{id}` | `platform_admin` [session] 302 | `specs/ledger-unhit-editorial.spec.mjs` · `staff uses the editorial PUT endpoints to edit news and publish an event` · `pl-1440` |
| F-gameauth-38807659866662 | POST `/oauth/authorize` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player denies OAuth consent without granting an authorization code` · `en-390` |
| F-gameauth-38807659866662 | POST `/oauth/authorize` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player denies OAuth consent without granting an authorization code` · `pl-1440` |
| F-identity-42558872787479 | POST `/{lang}/forgot-password` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `mail reset token works through the unprefixed reset endpoint` · `en-390` |
| F-identity-42558872787479 | POST `/{lang}/forgot-password` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `mail reset token works through the unprefixed reset endpoint` · `pl-1440` |
| F-identity-174105284821649 | POST `/{lang}/mfa/devices` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `en-390` |
| F-identity-174105284821649 | POST `/{lang}/mfa/devices` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `pl-1440` |
| F-identity-246951196210217 | POST `/{lang}/mfa/email` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `en-390` |
| F-identity-246951196210217 | POST `/{lang}/mfa/email` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `pl-1440` |
| F-identity-261677171151606 | POST `/{lang}/unsubscribe` | `anonymous` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `signed unsubscribe form disables optional mail for its account` · `en-390` |
| F-identity-261677171151606 | POST `/{lang}/unsubscribe` | `anonymous` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `signed unsubscribe form disables optional mail for its account` · `pl-1440` |
| R-identity-21546738985532 | POST `/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the unprefixed MFA entry point` · `en-390` |
| R-identity-21546738985532 | POST `/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the unprefixed MFA entry point` · `pl-1440` |
| R-identity-22426817360058 | DELETE `/{lang}/account/sessions` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `en-390` |
| R-identity-22426817360058 | DELETE `/{lang}/account/sessions` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `pl-1440` |
| R-identity-29491694713764 | PUT `/{lang}/account/privacy` | `player` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `en-390` |
| R-identity-29491694713764 | PUT `/{lang}/account/privacy` | `player` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `pl-1440` |
| R-identity-72817922306958 | POST `/mfa/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the unprefixed MFA entry point` · `en-390` |
| R-identity-72817922306958 | POST `/mfa/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the unprefixed MFA entry point` · `pl-1440` |
| R-identity-111939233055484 | DELETE `/account/sessions` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `en-390` |
| R-identity-111939233055484 | DELETE `/account/sessions` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `pl-1440` |
| R-identity-112148505166022 | PUT `/{lang}/password/change` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player changes the password through both PUT routes after sudo` · `en-390` |
| R-identity-112148505166022 | PUT `/{lang}/password/change` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player changes the password through both PUT routes after sudo` · `pl-1440` |
| R-identity-125883194273835 | POST `/account/recovery-key/confirm` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `en-390` |
| R-identity-125883194273835 | POST `/account/recovery-key/confirm` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `pl-1440` |
| R-identity-125883194273835 | POST `/account/recovery-key/confirm` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `en-390` |
| R-identity-125883194273835 | POST `/account/recovery-key/confirm` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `pl-1440` |
| R-identity-145836777639757 | POST `/account/recovery-key` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `en-390` |
| R-identity-145836777639757 | POST `/account/recovery-key` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `pl-1440` |
| R-identity-145836777639757 | POST `/account/recovery-key` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `en-390` |
| R-identity-145836777639757 | POST `/account/recovery-key` | `player` [session] 200 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `pl-1440` |
| R-identity-148307403461590 | DELETE `/account/recovery-key` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `en-390` |
| R-identity-148307403461590 | DELETE `/account/recovery-key` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `pl-1440` |
| R-identity-148753966933342 | PUT `/password/change` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player changes the password through both PUT routes after sudo` · `en-390` |
| R-identity-148753966933342 | PUT `/password/change` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player changes the password through both PUT routes after sudo` · `pl-1440` |
| R-identity-163984779009254 | DELETE `/{lang}/account/recovery-key` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `en-390` |
| R-identity-163984779009254 | DELETE `/{lang}/account/recovery-key` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the localized DELETE route aliases` · `pl-1440` |
| R-identity-198061895908889 | POST `/{lang}/mfa/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the localized MFA entry point` · `en-390` |
| R-identity-198061895908889 | POST `/{lang}/mfa/login` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `password sign-in through the localized MFA entry point` · `pl-1440` |
| R-identity-214054331608169 | POST `/reset-password` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `mail reset token works through the unprefixed reset endpoint` · `en-390` |
| R-identity-214054331608169 | POST `/reset-password` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `mail reset token works through the unprefixed reset endpoint` · `pl-1440` |
| R-identity-219751498818379 | POST `/recovery-key` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `en-390` |
| R-identity-219751498818379 | POST `/recovery-key` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `owner generates, confirms, and revokes a recovery key through the unprefixed route aliases` · `pl-1440` |
| R-identity-242956057359226 | POST `/account/sessions/{id}` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `en-390` |
| R-identity-242956057359226 | POST `/account/sessions/{id}` | `player` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `player updates privacy with PUT and revokes other sessions through both DELETE routes` · `pl-1440` |
| R-identity-265648941117731 | POST `/mfa/challenge` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `en-390` |
| R-identity-265648941117731 | POST `/mfa/challenge` | `anonymous` [session] 302 | `specs/ledger-unhit-account.spec.mjs` · `MFA aliases accept a real challenge and revoke its remembered device` · `pl-1440` |
| R-minigames-16116450611608 | POST `/{lang}/collection-pvp` | `player` [session] 503 | `specs/ledger-unhit-playerdata.spec.mjs` · `Collection PvP refuses tournament creation while collection authority is unavailable` · `en-390` |
| R-minigames-16116450611608 | POST `/{lang}/collection-pvp` | `player` [session] 503 | `specs/ledger-unhit-playerdata.spec.mjs` · `Collection PvP refuses tournament creation while collection authority is unavailable` · `pl-1440` |
| F-playerdata-91432524848231 | GET `/{lang}/account/hunts` | `player` [session] 200 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `en-390` |
| F-playerdata-91432524848231 | GET `/{lang}/account/hunts` | `player` [session] 200 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `pl-1440` |
| F-playerdata-166859893201249 | GET `/{lang}/account/hunts/compare` | `player` [session] 400 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `en-390` |
| F-playerdata-166859893201249 | GET `/{lang}/account/hunts/compare` | `player` [session] 400 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `pl-1440` |
| R-playerdata-19451765158851 | POST `/{lang}/account/journal/reports/notes` | `player` [session] 404 | `specs/ledger-unhit-playerdata.spec.mjs` · `journal notes refuse writes to an unknown hunt session` · `en-390` |
| R-playerdata-19451765158851 | POST `/{lang}/account/journal/reports/notes` | `player` [session] 404 | `specs/ledger-unhit-playerdata.spec.mjs` · `journal notes refuse writes to an unknown hunt session` · `pl-1440` |
| R-playerdata-92486286057690 | POST `/{lang}/atlas/places/participation` | `player` [session] 303 | `specs/ledger-unhit-playerdata.spec.mjs` · `player updates Atlas participation while the aggregate page is unavailable` · `en-390` |
| R-playerdata-92486286057690 | POST `/{lang}/atlas/places/participation` | `player` [session] 303 | `specs/ledger-unhit-playerdata.spec.mjs` · `player updates Atlas participation while the aggregate page is unavailable` · `pl-1440` |
| R-playerdata-215913651815162 | POST `/{lang}/account/hunts/{id}/share` | `player` [session] 503 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `en-390` |
| R-playerdata-215913651815162 | POST `/{lang}/account/hunts/{id}/share` | `player` [session] 503 | `specs/ledger-unhit-playerdata.spec.mjs` · `owner submits hunt filters and a comparison, and sharing independently fails closed` · `pl-1440` |
| F-stats-1942566653442 | GET `/stats/guilds` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `guild statistics submit filters while projections are cold` · `en-390` |
| F-stats-1942566653442 | GET `/stats/guilds` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `guild statistics submit filters while projections are cold` · `pl-1440` |
| F-stats-76658134362762 | GET `/{lang}/stats/rankings/{kind}` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `world and ranking statistics submit their filters without disclosing unavailable data` · `en-390` |
| F-stats-76658134362762 | GET `/{lang}/stats/rankings/{kind}` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `world and ranking statistics submit their filters without disclosing unavailable data` · `pl-1440` |
| F-stats-104498940072655 | GET `/{lang}/stats/compare` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `character history and comparison submit date filters and refuse cold projections` · `en-390` |
| F-stats-104498940072655 | GET `/{lang}/stats/compare` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `character history and comparison submit date filters and refuse cold projections` · `pl-1440` |
| F-stats-177464129402272 | GET `/{lang}/stats/characters/{id}/{tab}` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `character history and comparison submit date filters and refuse cold projections` · `en-390` |
| F-stats-177464129402272 | GET `/{lang}/stats/characters/{id}/{tab}` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `character history and comparison submit date filters and refuse cold projections` · `pl-1440` |
| F-stats-197040864587241 | GET `/stats/world` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `world and ranking statistics submit their filters without disclosing unavailable data` · `en-390` |
| F-stats-197040864587241 | GET `/stats/world` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `world and ranking statistics submit their filters without disclosing unavailable data` · `pl-1440` |
| F-stats-229832046893417 | GET `/{lang}/stats/guilds` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `guild statistics submit filters while projections are cold` · `en-390` |
| F-stats-229832046893417 | GET `/{lang}/stats/guilds` | `anonymous` [none] 503 | `specs/ledger-unhit-public.spec.mjs` · `guild statistics submit filters while projections are cold` · `pl-1440` |
| R-stats-823832337050 | POST `/api/v1/stats/watches` | `player` [session] 401 | `specs/ledger-unhit-account.spec.mjs` · `stats watch API rejects browser authentication for write operations` · `en-390` |
| R-stats-823832337050 | POST `/api/v1/stats/watches` | `player` [session] 401 | `specs/ledger-unhit-account.spec.mjs` · `stats watch API rejects browser authentication for write operations` · `pl-1440` |
| R-stats-109362773213568 | DELETE `/api/v1/stats/watches` | `player` [session] 401 | `specs/ledger-unhit-account.spec.mjs` · `stats watch API rejects browser authentication for write operations` · `en-390` |
| R-stats-109362773213568 | DELETE `/api/v1/stats/watches` | `player` [session] 401 | `specs/ledger-unhit-account.spec.mjs` · `stats watch API rejects browser authentication for write operations` · `pl-1440` |
| R-support-120909794989394 | PUT `/{lang}/admin/support/tickets/{id}/status` | `platform_admin` [session] 302 | `specs/ledger-unhit-support.spec.mjs` · `player closes a ticket and staff resolves another through the PUT endpoints` · `en-390` |
| R-support-120909794989394 | PUT `/{lang}/admin/support/tickets/{id}/status` | `platform_admin` [session] 302 | `specs/ledger-unhit-support.spec.mjs` · `player closes a ticket and staff resolves another through the PUT endpoints` · `pl-1440` |
| R-support-268810500855670 | PUT `/{lang}/support/tickets/{id}/status` | `player` [session] 302 | `specs/ledger-unhit-support.spec.mjs` · `player closes a ticket and staff resolves another through the PUT endpoints` · `en-390` |
| R-support-268810500855670 | PUT `/{lang}/support/tickets/{id}/status` | `player` [session] 302 | `specs/ledger-unhit-support.spec.mjs` · `player closes a ticket and staff resolves another through the PUT endpoints` · `pl-1440` |
| F-web-26667257346850 | GET `/search` | `anonymous` [none] 200 | `specs/ledger-unhit-public.spec.mjs` · `unprefixed portal search submits the pagination form for editorial results` · `en-390` |
| F-web-26667257346850 | GET `/search` | `anonymous` [none] 200 | `specs/ledger-unhit-public.spec.mjs` · `unprefixed portal search submits the pagination form for editorial results` · `pl-1440` |
| F-web-55121824626359 | POST `/{lang}/admin/store/codes` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff issues one gift code through the browser form` · `en-390` |
| F-web-55121824626359 | POST `/{lang}/admin/store/codes` | `platform_admin` [session] 200 | `specs/ledger-unhit-admin.spec.mjs` · `staff issues one gift code through the browser form` · `pl-1440` |
| F-web-81506816722856 | GET `/{lang}/search` | `anonymous` [none] 200 | `specs/ledger-unhit-public.spec.mjs` · `localized portal search submits its query` · `en-390` |
| F-web-81506816722856 | GET `/{lang}/search` | `anonymous` [none] 200 | `specs/ledger-unhit-public.spec.mjs` · `localized portal search submits its query` · `pl-1440` |
| F-web-142955662443676 | POST `/{lang}/verify-email/resend` | `anonymous` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `anonymous verification resend has the same generic response through its form and alias` · `en-390` |
| F-web-142955662443676 | POST `/{lang}/verify-email/resend` | `anonymous` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `anonymous verification resend has the same generic response through its form and alias` · `pl-1440` |
| F-web-257415907679051 | POST `/account/characters/{name}/profile` | `player` [session] 303 | `specs/ledger-unhit-bazaar.spec.mjs` · `Game-backed buyer can edit the owned profile and seller promotion fails closed` · `en-390` |
| F-web-257415907679051 | POST `/account/characters/{name}/profile` | `player` [session] 303 | `specs/ledger-unhit-bazaar.spec.mjs` · `Game-backed buyer can edit the owned profile and seller promotion fails closed` · `pl-1440` |
| R-web-19543202782207 | POST `/register` | `anonymous` [session] 422 | `specs/ledger-unhit-account.spec.mjs` · `unprefixed registration enforces proof of work before creating the account` · `en-390` |
| R-web-19543202782207 | POST `/register` | `anonymous` [session] 422 | `specs/ledger-unhit-account.spec.mjs` · `unprefixed registration enforces proof of work before creating the account` · `pl-1440` |
| R-web-173474453673449 | POST `/verify-email/resend` | `anonymous` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `anonymous verification resend has the same generic response through its form and alias` · `en-390` |
| R-web-173474453673449 | POST `/verify-email/resend` | `anonymous` [session] 303 | `specs/ledger-unhit-account.spec.mjs` · `anonymous verification resend has the same generic response through its form and alias` · `pl-1440` |
| F-wiki-104524932852102 | POST `/admin/wiki/articles/{id}/return-draft` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff returns an article from review to draft` · `en-390` |
| F-wiki-104524932852102 | POST `/admin/wiki/articles/{id}/return-draft` | `platform_admin` [session] 303 | `specs/ledger-unhit-editorial.spec.mjs` · `staff returns an article from review to draft` · `pl-1440` |
| F-wiki-183880812014521 | GET `/{lang}/wiki/calculators` | `anonymous` [none] 200 | `specs/ledger-unhit-calculator.spec.mjs` · `calculator passes contrast checks and splits loot on form submission` · `en-390` |
| F-wiki-183880812014521 | GET `/{lang}/wiki/calculators` | `anonymous` [none] 200 | `specs/ledger-unhit-calculator.spec.mjs` · `calculator passes contrast checks and splits loot on form submission` · `pl-1440` |

## Baseline gaps still unhit

- F-admin-2051149960876 POST `/console/dashboard`
- F-admin-17456964450888 POST `/admin/console/players`
- F-admin-38702896297808 POST `/console/database`
- F-admin-45079342387959 GET `/console/worlds`
- F-admin-58876857851269 GET `/admin/console/audit`
- F-admin-137657189881775 GET `/console/nodes`
- F-admin-138082345336189 POST `/console/worlds`
- F-admin-151927750828134 GET `/console/database`
- F-admin-169544806443708 GET `/admin/analytics/economy`
- F-admin-215839569153342 GET `/admin/console/players`
- F-admin-223893554250958 GET `/console/dashboard`
- F-admin-253054411857215 POST `/console/nodes`
- R-admin-1919642460343 POST `/console/database`
- R-admin-122795670899350 POST `/console/nodes`
- R-admin-136010976080442 POST `/console/worlds`
- R-admin-260508168334342 POST `/console/dashboard`
- F-atlas-161391314177223 GET `/{lang}/atlas/bestiary/{character}`
- F-bazaar-23275926065244 POST `/{lang}/bazaar/items/{id}/renew`
- F-bazaar-46767873991968 POST `/{lang}/bazaar/items`
- F-bazaar-86148115874390 POST `/{lang}/bazaar/items/{id}/cancel`
- F-bazaar-140015055205774 POST `/{lang}/bazaar/items/{id}/bid`
- F-bazaar-160340630540621 POST `/{lang}/bazaar/items/{id}/accept`
- F-bazaar-187660078956952 POST `/{lang}/bazaar/items/{id}/reject`
- F-bazaar-221617554685096 POST `/{lang}/bazaar/items/{id}/offer`
- F-bazaar-231195288605257 POST `/{lang}/bazaar/items/{id}/publish`
- F-bazaar-249147474695525 GET `/{lang}/bazaar/{listing}`
- F-bazaar-250676990346223 POST `/{lang}/bazaar/items/{id}/buy-now`
- R-bazaar-13638167717685 POST `/{lang}/bazaar/items`
- R-bazaar-137999873732047 GET `/{lang}/bazaar/items/{id}`
- F-community-3881337848774 GET `/characters/{name}/deaths`
- F-community-4730707544746 GET `/guilds`
- F-community-16287300610837 GET `/{lang}/deaths`
- F-community-71546943467822 GET `/{lang}/rankings/{kind}`
- F-community-193593469157960 GET `/{lang}/characters/{name}/deaths`
- F-community-219222581560817 GET `/deaths`
- F-community-274165327503467 GET `/{lang}/guilds`
- F-content-21985737393666 POST `/admin/events/{id}`
- F-identity-243063234466556 POST `/{lang}/reset-password`
- F-minigames-24012547260821 POST `/{lang}/collection/battles/{id}/forfeit`
- F-minigames-58824648762686 POST `/{lang}/pass/{season}/claim`
- F-minigames-114652082160743 POST `/{lang}/collection/{character}/decks`
- F-minigames-177564890723493 POST `/{lang}/collection/battles/{id}`
- F-minigames-187973382217026 POST `/{lang}/collection/decks/{id}/battle`
- F-minigames-218934052584363 POST `/{lang}/collection/battles/{id}`
- F-minigames-267968195854043 POST `/{lang}/pass/{season}/buy`
- F-minigames-280573004026262 POST `/{lang}/collection/decks/{id}/delete`
- R-minigames-115252755875764 POST `/{lang}/collection/battles/{id}`
- F-payments-249659384691764 POST `/native-account/coin-checkout/{order_id}`
- F-stats-216992143813701 GET `/{lang}/stats/rankings/{kind}`
- F-stats-222324429744041 GET `/{lang}/stats/rankings/{kind}`
- F-web-15366296186789 POST `/account/badges`
- F-web-33906076394532 POST `/{lang}/account/characters/{character}/services`
- R-web-98846564089840 POST `/{lang}/account/characters/{character}/services`
- F-wiki-5428367139875 GET `/{lang}/wiki/{kind}`
- F-wiki-22119884537424 GET `/{lang}/wiki/search`
- F-wiki-22599257223435 POST `/admin/wiki/articles/{id}`
- F-wiki-58594829099106 GET `/{lang}/wiki`
- F-wiki-65456549216585 POST `/admin/wiki/articles/{id}/archive`
- F-wiki-71042775092727 POST `/admin/wiki/articles/{id}/revisions/{source}/restore`
- F-wiki-154492065254748 GET `/wiki/search`
- F-wiki-172496912900389 GET `/wiki/{kind}`
- F-wiki-250551961713734 GET `/wiki`
- F-world-86048466149919 POST `/admin/world`
- F-world-89263600590152 GET `/admin/world`
- R-world-71870839689412 POST `/admin/world`
