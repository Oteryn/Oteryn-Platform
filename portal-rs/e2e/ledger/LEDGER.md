# Playwright coverage ledger

Tested revision `c67082cbc59bb44399d210ef7cac6e74141287cf`, run `fdeb764d-f51d-4123-8120-d915505a5d75`.

519 rows, 387 hit, 132 unhit, 91 form rows never submitted.

| Crate | Unhit |
| --- | ---: |
| admin | 23 |
| atlas | 2 |
| bazaar | 13 |
| characters | 0 |
| community | 10 |
| content | 9 |
| downloads | 0 |
| gameauth | 1 |
| hirelings | 0 |
| identity | 19 |
| minigames | 10 |
| payments | 1 |
| player-api | 0 |
| playerdata | 5 |
| stats | 10 |
| store | 0 |
| support | 2 |
| web | 10 |
| wiki | 14 |
| world | 3 |

| ID | Method and path | Submitter | Actors and statuses | HTTP 2xx or 3xx | HTTP 400/401/403/404/422 |
| --- | --- | --- | --- | --- | --- |
| F-admin-2051149960876 | POST `/console/dashboard` | Unnamed | — | No | No |
| F-admin-4597872030189 | GET `/admin/analytics/health` | Unnamed | — | No | No |
| F-admin-12019668692504 | GET `/admin/analytics` | Unnamed | — | No | No |
| F-admin-17456964450888 | POST `/admin/console/players` | Unnamed | — | No | No |
| F-admin-38702896297808 | POST `/console/database` | Unnamed | — | No | No |
| F-admin-45079342387959 | GET `/console/worlds` | Unnamed | — | No | No |
| F-admin-58876857851269 | GET `/admin/console/audit` | Unnamed | — | No | No |
| F-admin-72964517267296 | POST `/admin/roles` | `action=set-permissions` | — | No | No |
| F-admin-80739904374210 | GET `/admin/analytics/levels` | Unnamed | — | No | No |
| F-admin-137657189881775 | GET `/console/nodes` | Unnamed | — | No | No |
| F-admin-138082345336189 | POST `/console/worlds` | Unnamed | — | No | No |
| F-admin-151927750828134 | GET `/console/database` | Unnamed | — | No | No |
| F-admin-163973312113613 | POST `/admin/roles` | `action=revoke` | platform_admin [session] 303, unresolved [session] 303 | Yes | No |
| F-admin-169544806443708 | GET `/admin/analytics/economy` | Unnamed | — | No | No |
| F-admin-209133946605616 | GET `/admin/analytics/activity` | Unnamed | — | No | No |
| F-admin-215839569153342 | GET `/admin/console/players` | Unnamed | — | No | No |
| F-admin-223893554250958 | GET `/console/dashboard` | Unnamed | — | No | No |
| F-admin-244723736915658 | GET `/admin/analytics/creatures` | Unnamed | — | No | No |
| F-admin-253054411857215 | POST `/console/nodes` | Unnamed | — | No | No |
| F-admin-259360998889956 | POST `/admin/roles` | `action=create` | platform_admin [session] 303 | Yes | No |
| F-admin-269333545151525 | POST `/admin/roles` | `action=grant` | platform_admin [session] 303 | Yes | No |
| R-admin-1919642460343 | POST `/console/database` | — | — | No | No |
| R-admin-7087400176389 | GET `/console/worlds` | — | platform_admin [session] 200 | Yes | No |
| R-admin-7155275113463 | GET `/admin/moderation/reports/{id}` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-admin-11640828260702 | GET `/admin/roles` | — | anonymous [session] 303, platform_admin + staff_rbac_91c566b8588468d32597 [session] 200, platform_admin [session] 200, staff_rbac_ea6223f08938fd80258e [session] 403, unresolved [session] 303 | Yes | Yes |
| R-admin-25302599482201 | POST `/admin/players/{id}` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-admin-40625804778367 | GET `/console/dashboard` | — | platform_admin [session] 200, platform_admin [session] 403, player [session] 403 | Yes | Yes |
| R-admin-52803150429123 | GET `/admin/analytics/activity` | — | platform_admin [session] 200 | Yes | No |
| R-admin-54479269810945 | GET `/admin` | — | platform_admin [session] 200, platform_admin [session] 403, player [session] 403 | Yes | Yes |
| R-admin-69625004209542 | GET `/admin/analytics/creatures` | — | platform_admin [session] 200 | Yes | No |
| R-admin-72173742272774 | GET `/admin/control/approvals/{id}` | — | platform_admin [session] 403 | No | Yes |
| R-admin-76565317736447 | GET `/admin/moderation/enforcement/{id}` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-admin-98646810027024 | GET `/admin/control/step-up` | — | platform_admin [session] 200, player [session] 403 | Yes | Yes |
| R-admin-101796607690506 | POST `/admin/commerce/approvals/{id}` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-admin-109512802646132 | GET `/admin/analytics/health` | — | platform_admin [session] 200 | Yes | No |
| R-admin-111999750152239 | POST `/admin/commerce/{action}` | — | platform_admin [session] 303, platform_admin [session] 403, platform_admin [session] 422, player [session] 403 | Yes | Yes |
| R-admin-117789581338039 | GET `/admin/console/audit` | — | platform_admin [session] 200, player [session] 403, staff_rbac_ea6223f08938fd80258e [session] 200, unresolved [session] 303 | Yes | Yes |
| R-admin-122402114022194 | GET `/admin/{lang}/bazaar` | — | platform_admin [session] 200 | Yes | No |
| R-admin-122795670899350 | POST `/console/nodes` | — | — | No | No |
| R-admin-130871195780899 | GET `/admin/console/approvals` | — | platform_admin [session] 200 | Yes | No |
| R-admin-136010976080442 | POST `/console/worlds` | — | — | No | No |
| R-admin-136953732177072 | POST `/admin/{lang}/bazaar` | — | platform_admin [session] 303, platform_admin [session] 403, player [session] 403 | Yes | Yes |
| R-admin-148018487172175 | GET `/admin/analytics/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-152505428925927 | GET `/admin/players` | — | platform_admin [session] 200 | Yes | No |
| R-admin-153146107412647 | GET `/admin/analytics/activity/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-164941640870793 | POST `/admin/control/step-up` | — | platform_admin [session] 403 | No | Yes |
| R-admin-171217184699289 | GET `/admin/analytics/creatures/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-172543822157694 | GET `/admin/console/access` | — | platform_admin [session] 200 | Yes | No |
| R-admin-174763800972966 | GET `/admin/analytics/levels/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-176963345132691 | GET `/console/database` | — | platform_admin [session] 200 | Yes | No |
| R-admin-188915193925788 | GET `/admin/console/players` | — | platform_admin [session] 200, platform_admin [session] 403, player [session] 403 | Yes | Yes |
| R-admin-193127307076843 | GET `/admin/moderation/enforcement` | — | platform_admin [session] 200, player [session] 403 | Yes | Yes |
| R-admin-197351684143002 | GET `/admin/moderation/reports` | — | platform_admin [session] 200, player [session] 403 | Yes | Yes |
| R-admin-203626930969829 | GET `/admin/analytics/economy` | — | platform_admin [session] 200 | Yes | No |
| R-admin-218052089637069 | GET `/console/nodes` | — | platform_admin [session] 200 | Yes | No |
| R-admin-229449368918823 | GET `/admin/players/{id}` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-admin-239054952641410 | GET `/admin/analytics/economy/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-241123197066198 | GET `/admin/moderation/enforcement/create` | — | platform_admin [session] 200 | Yes | No |
| R-admin-242768985880687 | GET `/admin/analytics/levels` | — | platform_admin [session] 200 | Yes | No |
| R-admin-242839363734920 | GET `/admin/analytics/health/data` | — | platform_admin [session] 200 | Yes | No |
| R-admin-258908740605927 | GET `/admin/analytics` | — | platform_admin [session] 200, platform_admin [session] 403 | Yes | Yes |
| R-admin-260508168334342 | POST `/console/dashboard` | — | — | No | No |
| R-admin-261338114289492 | GET `/admin/commerce` | — | platform_admin [session] 200 | Yes | No |
| R-admin-264081041569923 | POST `/admin/roles` | — | platform_admin + staff_rbac_91c566b8588468d32597 [session] 403, platform_admin [session] 303, platform_admin [session] 403, player [session] 403, staff_rbac_ea6223f08938fd80258e [session] 403, unresolved [session] 303 | Yes | Yes |
| R-admin-273544798622400 | POST `/admin/control/approvals/{id}` | — | — | No | No |
| F-atlas-161391314177223 | GET `/{lang}/atlas/bestiary/{character}` | Unnamed | — | No | No |
| R-atlas-17455541038624 | GET `/atlas/delivery.css` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200 | Yes | No |
| R-atlas-23762049084996 | GET `/atlas/private-markers` | — | player [session] 200 | Yes | No |
| R-atlas-27440492509502 | GET `/atlas/tiles/{version}/{floor}/{zoom}/{x}/{y}` | — | player [session] 503 | No | No |
| R-atlas-67464908865522 | POST `/atlas/map-reports` | — | player [session] 403 | No | Yes |
| R-atlas-91175040616691 | DELETE `/atlas/private-markers/{id}` | — | player [session] 403 | No | Yes |
| R-atlas-98258957344240 | GET `/{lang}/atlas` | — | anonymous [none] 503, anonymous [session] 503, player [session] 503 | No | No |
| R-atlas-109424134317489 | GET `/{lang}/atlas/delivery` | — | anonymous [none] 503, anonymous [session] 503, player [session] 503 | No | No |
| R-atlas-130633923692927 | POST `/atlas/private-markers` | — | — | No | No |
| R-atlas-192284406668860 | GET `/{lang}/atlas/bestiary/{character}` | — | player [session] 503 | No | No |
| R-atlas-229232642088498 | GET `/atlas/spawns-text/{lang}` | — | player [session] 200 | Yes | No |
| R-atlas-279798706631441 | GET `/atlas/spawns` | — | player [session] 400 | No | Yes |
| F-bazaar-23275926065244 | POST `/{lang}/bazaar/items/{id}/renew` | Unnamed | — | No | No |
| F-bazaar-46767873991968 | POST `/{lang}/bazaar/items` | Unnamed | — | No | No |
| F-bazaar-56705872138846 | POST `/{lang}/account/bazaar/sell` | Unnamed | player [session] 303 | Yes | No |
| F-bazaar-86148115874390 | POST `/{lang}/bazaar/items/{id}/cancel` | Unnamed | — | No | No |
| F-bazaar-140015055205774 | POST `/{lang}/bazaar/items/{id}/bid` | Unnamed | — | No | No |
| F-bazaar-140608632649640 | POST `/{lang}/account/bazaar/{listing}/bids` | Unnamed | player [session] 303 | Yes | No |
| F-bazaar-160340630540621 | POST `/{lang}/bazaar/items/{id}/accept` | Unnamed | — | No | No |
| F-bazaar-187660078956952 | POST `/{lang}/bazaar/items/{id}/reject` | Unnamed | — | No | No |
| F-bazaar-221617554685096 | POST `/{lang}/bazaar/items/{id}/offer` | Unnamed | — | No | No |
| F-bazaar-231195288605257 | POST `/{lang}/bazaar/items/{id}/publish` | Unnamed | — | No | No |
| F-bazaar-249147474695525 | GET `/{lang}/bazaar/{listing}` | Unnamed | — | No | No |
| F-bazaar-250676990346223 | POST `/{lang}/bazaar/items/{id}/buy-now` | Unnamed | — | No | No |
| R-bazaar-5602778526677 | GET `/{lang}/bazaar/items` | — | player [session] 200 | Yes | No |
| R-bazaar-13638167717685 | POST `/{lang}/bazaar/items` | — | — | No | No |
| R-bazaar-15980521205331 | POST `/{lang}/account/bazaar/sell` | — | player [session] 303, player [session] 403 | Yes | Yes |
| R-bazaar-35352714986507 | POST `/{lang}/account/bazaar/{listing}/promote` | — | — | No | No |
| R-bazaar-37444044833527 | POST `/{lang}/bazaar/items/{id}/{action}` | — | player [session] 404 | No | Yes |
| R-bazaar-82908904463903 | GET `/{lang}/bazaar/items/{listing}` | — | player [session] 200, player [session] 404 | Yes | Yes |
| R-bazaar-100470884131994 | GET `/{lang}/account/bazaar/sell` | — | player [session] 200 | Yes | No |
| R-bazaar-110273497403936 | GET `/{lang}/account/bazaar/{listing}/report` | — | player [session] 200 | Yes | No |
| R-bazaar-128081335135863 | POST `/{lang}/account/bazaar/{listing}/bids` | — | player [session] 303, player [session] 403, player [session] 422 | Yes | Yes |
| R-bazaar-137999873732047 | GET `/{lang}/bazaar/items/{id}` | — | — | No | No |
| R-bazaar-140610708010822 | POST `/{lang}/account/bazaar/{listing}/report` | — | player [session] 403 | No | Yes |
| R-bazaar-168880685974673 | POST `/{lang}/account/bazaar/{listing}/control` | — | player [session] 303, player [session] 403 | Yes | Yes |
| R-bazaar-171649801212846 | GET `/{lang}/bazaar/sellers/{seller}` | — | player [session] 404 | No | Yes |
| R-bazaar-176735998690434 | GET `/{lang}/account/bazaar/{listing}/promote` | — | player [session] 403 | No | Yes |
| R-bazaar-200927950768991 | GET `/{lang}/bazaar` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-bazaar-218608491157449 | GET `/{lang}/bazaar/items/new` | — | player [session] 200 | Yes | No |
| R-bazaar-240423746866466 | GET `/{lang}/bazaar/{listing}` | — | anonymous [session] 200, player [session] 200, player [session] 400, player [session] 404 | Yes | Yes |
| R-bazaar-244002074329814 | GET `/{lang}/account/bazaar` | — | player [session] 200 | Yes | No |
| R-characters-204732008650193 | POST `/{lang}/account/characters/new` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-characters-221647136864964 | GET `/{lang}/account/characters/suggest` | — | player [session] 200 | Yes | No |
| R-characters-256085597357123 | GET `/{lang}/account/characters/requests/{operation}` | — | player [session] 200, player [session] 404 | Yes | Yes |
| R-characters-267840637679871 | GET `/{lang}/account/characters/new` | — | anonymous [session] 303, player [session] 200 | Yes | No |
| F-community-3881337848774 | GET `/characters/{name}/deaths` | Unnamed | — | No | No |
| F-community-4730707544746 | GET `/guilds` | Unnamed | — | No | No |
| F-community-16287300610837 | GET `/{lang}/deaths` | Unnamed | — | No | No |
| F-community-71546943467822 | GET `/{lang}/rankings/{kind}` | Unnamed | — | No | No |
| F-community-98993181248635 | GET `/{lang}/characters` | Unnamed | — | No | No |
| F-community-188835548187730 | GET `/{lang}/online` | Unnamed | — | No | No |
| F-community-193593469157960 | GET `/{lang}/characters/{name}/deaths` | Unnamed | — | No | No |
| F-community-215566307321900 | GET `/online` | Unnamed | — | No | No |
| F-community-219222581560817 | GET `/deaths` | Unnamed | — | No | No |
| F-community-274165327503467 | GET `/{lang}/guilds` | Unnamed | — | No | No |
| R-community-19285942114392 | GET `/{lang}/party` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-community-26892359220553 | GET `/today/data` | — | player [session] 503 | No | No |
| R-community-34825495059419 | POST `/{lang}/party/{id}/remove` | — | player [session] 403 | No | Yes |
| R-community-36175830638820 | GET `/today/widget` | — | player [session] 503 | No | No |
| R-community-39292948985653 | GET `/characters/{name}/deaths` | — | player [session] 503 | No | No |
| R-community-56948402227085 | GET `/{lang}/online` | — | anonymous [none] 503, anonymous [session] 503, player [session] 200, player [session] 503 | Yes | No |
| R-community-80834569254580 | GET `/{lang}/characters/{name}/deaths` | — | player [session] 503 | No | No |
| R-community-81813531777106 | POST `/{lang}/party/{id}/report` | — | player [session] 403 | No | Yes |
| R-community-105495582120385 | GET `/{lang}/characters/{name}` | — | player [session] 200, player [session] 404, player [session] 503 | Yes | Yes |
| R-community-118496164543291 | GET `/api/v1/worlds` | — | player [session] 200 | Yes | No |
| R-community-119531705208118 | GET `/online` | — | player [session] 503 | No | No |
| R-community-121294075739770 | GET `/guilds` | — | player [session] 503 | No | No |
| R-community-128271839500338 | POST `/{lang}/party` | — | player [session] 403 | No | Yes |
| R-community-133524706453102 | POST `/admin/{lang}/party/{id}/moderate` | — | platform_admin [session] 403 | No | Yes |
| R-community-139495989521909 | GET `/guilds/{name}` | — | player [session] 503 | No | No |
| R-community-142834292470773 | GET `/{lang}/deaths` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-community-169965008876822 | GET `/{lang}/guilds/{name}` | — | player [session] 503 | No | No |
| R-community-171114794742126 | GET `/houses` | — | player [session] 503 | No | No |
| R-community-171114941965285 | GET `/{lang}/houses` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-community-173344960238811 | GET `/{lang}/rankings/{kind}` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-community-178446612801635 | GET `/houses/{key}` | — | player [session] 400 | No | Yes |
| R-community-184130815142787 | GET `/{lang}/worlds` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200 | Yes | No |
| R-community-196002416512895 | GET `/admin/{lang}/party` | — | platform_admin [session] 200 | Yes | No |
| R-community-196315784433233 | GET `/worlds` | — | player [session] 200 | Yes | No |
| R-community-208691162068040 | GET `/{lang}/houses/{key}` | — | player [session] 400 | No | Yes |
| R-community-213702337456541 | GET `/{lang}/guilds` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-community-229341934833704 | GET `/deaths` | — | player [session] 503 | No | No |
| R-community-243680013307304 | GET `/{lang}/characters` | — | player [session] 200 | Yes | No |
| R-community-270453740798440 | GET `/{lang}/today/widget` | — | anonymous [none] 503, player [session] 503 | No | No |
| R-community-280417816283812 | GET `/{lang}/today/data` | — | anonymous [none] 503, player [session] 503 | No | No |
| F-content-21985737393666 | POST `/admin/events/{id}` | Unnamed | — | No | No |
| F-content-74217417116926 | POST `/admin/announcements/{id}/edit` | Unnamed | — | No | No |
| F-content-74382719931163 | POST `/admin/pages/{slug}/{lang}` | `action=save` | — | No | No |
| F-content-74759377444544 | POST `/admin/events/{id}/status` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-content-157588388585825 | GET `/admin/pages` | Unnamed | — | No | No |
| F-content-212759397105307 | POST `/admin/pages/{slug}/{lang}` | `action=delete` | — | No | No |
| F-content-232347588452795 | POST `/admin/events` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-content-277970024806169 | POST `/admin/announcements` | Unnamed | — | No | No |
| R-content-1598580398129 | GET `/admin/pages/{slug}/{lang}` | — | platform_admin [session] 200 | Yes | No |
| R-content-6281911721795 | GET `/{lang}/today` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-22660928672679 | GET `/news/{slug}` | — | player [session] 404 | No | Yes |
| R-content-23666124338102 | DELETE `/admin/media/{id}` | — | platform_admin [session] 403 | No | Yes |
| R-content-37408306904157 | GET `/{lang}/events` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-39960494460558 | PUT `/admin/events/{id}/status` | — | — | No | No |
| R-content-41474828233033 | GET `/admin/pages` | — | platform_admin [session] 200 | Yes | No |
| R-content-47992296593037 | GET `/admin/news/{id}/edit` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-content-52790718120554 | POST `/admin/news/{id}` | — | platform_admin [session] 302, platform_admin [session] 403 | Yes | Yes |
| R-content-55902279579622 | GET `/{lang}/support` | — | player [session] 200 | Yes | No |
| R-content-63765776539603 | PUT `/admin/events/{id}` | — | — | No | No |
| R-content-64566092957887 | GET `/{lang}/legal/terms` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-64816078515389 | GET `/{lang}/legal/cookies` | — | player [session] 200 | Yes | No |
| R-content-84398982414491 | GET `/events/{slug}` | — | player [session] 303 | Yes | No |
| R-content-88471330685265 | GET `/{lang}/help` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-91664150747993 | GET `/admin/media/{id}/content` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-content-103335509568403 | GET `/{lang}/legal/privacy` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-111631848483706 | GET `/{lang}/news` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-119090335667007 | POST `/admin/events` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-content-121661766321316 | GET `/announcements/ticker` | — | player [session] 200 | Yes | No |
| R-content-133262685712639 | POST `/admin/pages/{slug}/{lang}` | — | platform_admin [session] 403 | No | Yes |
| R-content-141483568009117 | PUT `/admin/news/{id}` | — | — | No | No |
| R-content-141630859539893 | GET `/{lang}/announcements/ticker` | — | player [session] 200 | Yes | No |
| R-content-145211358111634 | GET `/{lang}/events/{slug}` | — | anonymous [session] 200, platform_admin [session] 200, player [session] 200, player [session] 404 | Yes | Yes |
| R-content-146222808858516 | POST `/admin/media/{id}/delete` | — | platform_admin [session] 302, platform_admin [session] 403 | Yes | Yes |
| R-content-160040037864470 | GET `/news` | — | player [session] 200 | Yes | No |
| R-content-161225677930508 | GET `/today` | — | player [session] 200 | Yes | No |
| R-content-164710134141164 | GET `/admin/events/create` | — | platform_admin [session] 200 | Yes | No |
| R-content-175573663878158 | POST `/admin/news` | — | platform_admin [session] 302, platform_admin [session] 403 | Yes | Yes |
| R-content-177200316933135 | GET `/{lang}/pages/{slug}` | — | player [session] 404 | No | Yes |
| R-content-188112887370869 | GET `/events` | — | player [session] 303 | Yes | No |
| R-content-189851917604499 | POST `/admin/events/{id}` | — | platform_admin [session] 403 | No | Yes |
| R-content-202033650311537 | GET `/{lang}/support/report-a-bug` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-204787100009369 | POST `/admin/media` | — | platform_admin [session] 302, platform_admin [session] 403 | Yes | Yes |
| R-content-209549200621870 | GET `/admin/events` | — | platform_admin [session] 200 | Yes | No |
| R-content-209810101338499 | GET `/{lang}/server-information` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-210744146184507 | POST `/admin/news/{id}/delete` | — | platform_admin [session] 302, platform_admin [session] 403 | Yes | Yes |
| R-content-212641516760627 | GET `/{lang}/rules` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-content-216560574178814 | GET `/admin/media` | — | platform_admin [session] 200 | Yes | No |
| R-content-222769855791519 | GET `/admin/news/create` | — | platform_admin [session] 200 | Yes | No |
| R-content-224470527010009 | GET `/admin/news` | — | platform_admin [session] 200 | Yes | No |
| R-content-225559375868921 | POST `/admin/events/{id}/status` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-content-229516000680490 | GET `/admin/events/{id}/edit` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-content-238223484804763 | GET `/admin/media/{id}/thumbnail` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-content-258791523387798 | GET `/{lang}/news/{slug}` | — | platform_admin [session] 200, platform_admin [session] 404, player [session] 404 | Yes | Yes |
| R-content-266989242151233 | DELETE `/admin/news/{id}` | — | platform_admin [session] 403 | No | Yes |
| R-downloads-183259539286957 | GET `/updates/{channel}/targets/{*target}` | — | player [session] 400 | No | Yes |
| R-downloads-224529468283860 | GET `/updates/{channel}/policy-v1.json` | — | player [session] 400 | No | Yes |
| R-downloads-260709440547894 | GET `/updates/{channel}/metadata/{filename}` | — | player [session] 400, player [session] 404 | No | Yes |
| F-gameauth-2586188806928 | POST `/oauth/authorize` | `decision=approve` | player [session] 302 | Yes | No |
| F-gameauth-38807659866662 | POST `/oauth/authorize` | `decision=deny` | — | No | No |
| R-gameauth-30021764227169 | POST `/api/v1/game-auth/device-sessions` | — | player [session] 503 | No | No |
| R-gameauth-36134473086671 | POST `/api/v1/game-auth/device-sessions/revoke` | — | player [session] 503 | No | No |
| R-gameauth-45217208078333 | POST `/api/v1/game-auth/device-sessions/native-characters` | — | player [session] 503 | No | No |
| R-gameauth-100288821490492 | POST `/api/v1/game-auth/device-sessions/{*rest}` | — | player [session] 503 | No | No |
| R-gameauth-108005780864884 | GET `/oauth/authorize` | — | player [session] 200, player [session] 400 | Yes | Yes |
| R-gameauth-213968412660548 | POST `/oauth/authorize` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-gameauth-224268645680220 | POST `/oauth/token` | — | player [session] 200, player [session] 400, player [session] 401 | Yes | Yes |
| R-gameauth-231455293824167 | POST `/api/v1/game-auth/device-sessions/tickets` | — | player [session] 503 | No | No |
| R-hirelings-176623252573759 | GET `/{lang}/panel/hirelings` | — | player [session] 200 | Yes | No |
| F-identity-24314404736199 | POST `/{lang}/mfa/challenge` | Unnamed | anonymous [session] 302, unresolved [session] 302 | Yes | No |
| F-identity-28114953959570 | POST `/{lang}/account/privacy/delete/cancel` | Unnamed | player [session] 303 | Yes | No |
| F-identity-42558872787479 | POST `/{lang}/forgot-password` | Unnamed | anonymous [session] 302 | Yes | No |
| F-identity-83856483067108 | POST `/{lang}/account/notifications` | Unnamed | player [session] 303 | Yes | No |
| F-identity-131481052480780 | POST `/{lang}/mfa/enroll` | Unnamed | platform_admin [session] 200, player [session] 200 | Yes | No |
| F-identity-174105284821649 | POST `/{lang}/mfa/devices` | Unnamed | — | No | No |
| F-identity-219307662563129 | POST `/{lang}/account/privacy/delete` | Unnamed | unresolved [session] 303 | Yes | No |
| F-identity-241865794976884 | POST `/{lang}/mfa/confirm` | Unnamed | unresolved [session] 200 | Yes | No |
| F-identity-243063234466556 | POST `/{lang}/reset-password` | Unnamed | anonymous [session] 302, unresolved [session] 302 | Yes | No |
| F-identity-246951196210217 | POST `/{lang}/mfa/email` | Unnamed | — | No | No |
| F-identity-261677171151606 | POST `/{lang}/unsubscribe` | Unnamed | — | No | No |
| F-identity-273943731811847 | POST `/{lang}/login` | Unnamed | anonymous [session] 302, unresolved [session] 302 | Yes | No |
| F-identity-277800349969974 | POST `/{lang}/logout` | Unnamed | unresolved [session] 302 | Yes | No |
| R-identity-6046100358 | GET `/{lang}/mfa/login` | — | player [session] 200 | Yes | No |
| R-identity-2956008728909 | POST `/{lang}/account/passkeys/{id}/remove` | — | player [session] 403 | No | Yes |
| R-identity-4332573405923 | GET `/register/abuse.js` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-9867465621545 | GET `/password/change` | — | player [session] 200 | Yes | No |
| R-identity-11524247857523 | GET `/recovery-key` | — | player [session] 200 | Yes | No |
| R-identity-11857972413486 | POST `/email-change/revert/{token}` | — | anonymous [session] 302, anonymous [session] 403, player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-14179741681769 | POST `/{lang}/account/passkeys/authenticate/start` | — | player [session] 403 | No | Yes |
| R-identity-21546738985532 | POST `/login` | — | — | No | No |
| R-identity-22426817360058 | DELETE `/{lang}/account/sessions` | — | — | No | No |
| R-identity-24228855312972 | POST `/{lang}/account/security/email` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-identity-29491694713764 | PUT `/{lang}/account/privacy` | — | — | No | No |
| R-identity-32746754005251 | POST `/{lang}/unsubscribe` | — | player [session] 200, player [session] 422 | Yes | Yes |
| R-identity-32863624232138 | DELETE `/{lang}/account/sessions/{id}` | — | player [session] 403 | No | Yes |
| R-identity-35526082843754 | GET `/{lang}/account/passkeys` | — | player [session] 200 | Yes | No |
| R-identity-37897339814727 | GET `/{lang}/reset-password` | — | player [session] 200 | Yes | No |
| R-identity-42524819159631 | DELETE `/account/sessions/{id}` | — | player [session] 403 | No | Yes |
| R-identity-47075864001367 | GET `/{lang}/password/change` | — | player [session] 200 | Yes | No |
| R-identity-48570195115048 | GET `/reset-password` | — | player [session] 200 | Yes | No |
| R-identity-50059616468218 | POST `/forgot-password` | — | player [session] 403 | No | Yes |
| R-identity-58065727827201 | GET `/{lang}/email-change/confirm/{token}` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-58421947578203 | POST `/{lang}/login` | — | anonymous [session] 302, anonymous [session] 403, anonymous [session] 429, player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-61551749122827 | POST `/{lang}/account/sessions/others` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-identity-63002131513601 | GET `/{lang}/account/sudo` | — | player [session] 200 | Yes | No |
| R-identity-64793814368433 | POST `/{lang}/recovery-key` | — | player [session] 403 | No | Yes |
| R-identity-65545016439871 | GET `/{lang}/account/recovery-key/confirm` | — | player [session] 200 | Yes | No |
| R-identity-65644980932576 | POST `/{lang}/account/recovery-key` | — | player [session] 200, player [session] 403 | Yes | Yes |
| R-identity-68864645871386 | GET `/email-change/revert/{token}` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-72817922306958 | POST `/mfa/login` | — | — | No | No |
| R-identity-73767267345920 | POST `/{lang}/logout` | — | platform_admin [session] 403, player [session] 302, player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-76393382254163 | POST `/account/security/email` | — | player [session] 403 | No | Yes |
| R-identity-77443464805311 | POST `/{lang}/email-change/revert/{token}` | — | player [session] 403 | No | Yes |
| R-identity-78406096895331 | GET `/{lang}/login` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200, unresolved [session] 200 | Yes | No |
| R-identity-78610464499327 | POST `/mfa/enroll` | — | player [session] 403 | No | Yes |
| R-identity-81364293671996 | GET `/account/security` | — | player [session] 303 | Yes | No |
| R-identity-81968511720290 | POST `/{lang}/account/sudo` | — | player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-85161381286520 | POST `/email-change/confirm/{token}` | — | anonymous [session] 302, anonymous [session] 403, player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-85469806944236 | GET `/mfa/challenge` | — | player [session] 302 | Yes | No |
| R-identity-85756749892140 | GET `/{lang}/mfa/challenge` | — | anonymous [session] 200, player [session] 302 | Yes | No |
| R-identity-87661262848889 | GET `/{lang}/account/privacy/delete` | — | player [session] 200 | Yes | No |
| R-identity-88115745844534 | GET `/{lang}/account/security/wizard` | — | player [session] 200 | Yes | No |
| R-identity-92292553805863 | POST `/{lang}/mfa/devices` | — | player [session] 403 | No | Yes |
| R-identity-94661746507694 | POST `/password/change` | — | player [session] 403 | No | Yes |
| R-identity-96864534766882 | GET `/email-change/confirm/{token}` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-100643924609280 | POST `/mfa/devices` | — | player [session] 403 | No | Yes |
| R-identity-100908073161350 | GET `/account/recovery-key/confirm` | — | player [session] 200 | Yes | No |
| R-identity-101222195069006 | POST `/account/sessions/all` | — | player [session] 403 | No | Yes |
| R-identity-102000905050503 | GET `/{lang}/mfa` | — | platform_admin [session] 200, player [session] 200 | Yes | No |
| R-identity-111939233055484 | DELETE `/account/sessions` | — | — | No | No |
| R-identity-112148505166022 | PUT `/{lang}/password/change` | — | — | No | No |
| R-identity-116050176939125 | GET `/{lang}/email-change/revert/{token}` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-119543090203439 | POST `/{lang}/mfa/challenge` | — | anonymous [session] 302, anonymous [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-122412250932724 | POST `/{lang}/account/sessions/all` | — | player [session] 403 | No | Yes |
| R-identity-125883194273835 | POST `/account/recovery-key/confirm` | — | — | No | No |
| R-identity-128789264510342 | POST `/account/sudo` | — | player [session] 403 | No | Yes |
| R-identity-131157961162948 | POST `/{lang}/account/passkeys/register/finish` | — | player [session] 403 | No | Yes |
| R-identity-132711118562262 | POST `/{lang}/forgot-password` | — | anonymous [session] 302, anonymous [session] 403, player [session] 403 | Yes | Yes |
| R-identity-139575925211492 | POST `/{lang}/account/privacy/delete` | — | player [session] 403, player [session] 422, unresolved [session] 303 | Yes | Yes |
| R-identity-139704914722259 | GET `/{lang}/account/privacy` | — | player [session] 200 | Yes | No |
| R-identity-144253154668948 | GET `/account/sessions` | — | player [session] 200 | Yes | No |
| R-identity-145836777639757 | POST `/account/recovery-key` | — | — | No | No |
| R-identity-148307403461590 | DELETE `/account/recovery-key` | — | — | No | No |
| R-identity-148753966933342 | PUT `/password/change` | — | — | No | No |
| R-identity-152572714987110 | POST `/{lang}/account/passkeys/authenticate/finish` | — | player [session] 403 | No | Yes |
| R-identity-160042847639544 | POST `/{lang}/account/recovery-key/confirm` | — | player [session] 200, player [session] 403 | Yes | Yes |
| R-identity-163984779009254 | DELETE `/{lang}/account/recovery-key` | — | — | No | No |
| R-identity-167226149007033 | POST `/{lang}/mfa/enroll` | — | platform_admin [session] 200, platform_admin [session] 403, player [session] 200, player [session] 403 | Yes | Yes |
| R-identity-172328804888524 | POST `/{lang}/account/passkeys/register/start` | — | player [session] 403 | No | Yes |
| R-identity-174628683405105 | GET `/account/sudo` | — | player [session] 200 | Yes | No |
| R-identity-180027789018718 | POST `/{lang}/mfa/confirm` | — | platform_admin [session] 403, player [session] 403, unresolved [session] 200 | Yes | Yes |
| R-identity-186929504325091 | GET `/{lang}/forgot-password` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-190985627216785 | GET `/{lang}/reset-password/{token}` | — | anonymous [none] 200, anonymous [session] 200, player [session] 200 | Yes | No |
| R-identity-191081467777067 | POST `/{lang}/email-change/confirm/{token}` | — | player [session] 403 | No | Yes |
| R-identity-192404097457685 | GET `/mfa` | — | player [session] 200 | Yes | No |
| R-identity-193732814215032 | POST `/{lang}/reset-password` | — | anonymous [session] 302, anonymous [session] 403, player [session] 403, unresolved [session] 302 | Yes | Yes |
| R-identity-194148580354466 | GET `/{lang}/unsubscribe` | — | player [session] 200 | Yes | No |
| R-identity-195071347553869 | POST `/logout` | — | player [session] 403 | No | Yes |
| R-identity-198061895908889 | POST `/{lang}/mfa/login` | — | — | No | No |
| R-identity-204649931415431 | GET `/login` | — | player [session] 200, unresolved [session] 200 | Yes | No |
| R-identity-206247634623172 | GET `/{lang}/account/notifications` | — | player [session] 200 | Yes | No |
| R-identity-212072918176823 | POST `/{lang}/account/privacy` | — | player [session] 303, player [session] 403 | Yes | Yes |
| R-identity-214054331608169 | POST `/reset-password` | — | — | No | No |
| R-identity-216828197546601 | GET `/{lang}/account/sessions` | — | player [session] 200, unresolved [session] 303 | Yes | No |
| R-identity-219751498818379 | POST `/recovery-key` | — | — | No | No |
| R-identity-220336585032911 | GET `/{lang}/account/recovery-key` | — | player [session] 200 | Yes | No |
| R-identity-225775427645786 | POST `/mfa/confirm` | — | player [session] 403 | No | Yes |
| R-identity-227981696455018 | POST `/{lang}/account/sessions/{id}` | — | player [session] 403 | No | Yes |
| R-identity-228329755695497 | POST `/account/sessions/others` | — | player [session] 403 | No | Yes |
| R-identity-231592518542019 | GET `/account/security/email` | — | player [session] 200 | Yes | No |
| R-identity-231631235306538 | POST `/{lang}/account/notifications` | — | player [session] 303, player [session] 403, player [session] 422 | Yes | Yes |
| R-identity-236843841504188 | POST `/mfa/email` | — | player [session] 403 | No | Yes |
| R-identity-237752164197819 | POST `/{lang}/mfa/email` | — | player [session] 403 | No | Yes |
| R-identity-240053760622444 | POST `/register/abuse/challenge` | — | anonymous [session] 200, player [session] 403 | Yes | Yes |
| R-identity-242482970471534 | GET `/{lang}/account/security` | — | player [session] 303 | Yes | No |
| R-identity-242956057359226 | POST `/account/sessions/{id}` | — | — | No | No |
| R-identity-249251437677833 | GET `/{lang}/account/security/email` | — | player [session] 200 | Yes | No |
| R-identity-258843178253286 | GET `/mfa/login` | — | player [session] 200 | Yes | No |
| R-identity-261727741660889 | POST `/{lang}/password/change` | — | player [session] 403 | No | Yes |
| R-identity-261760401483788 | POST `/account/recovery-key/revoke` | — | player [session] 403 | No | Yes |
| R-identity-265648941117731 | POST `/mfa/challenge` | — | — | No | No |
| R-identity-266974736404399 | POST `/{lang}/account/privacy/delete/cancel` | — | player [session] 303, player [session] 403 | Yes | Yes |
| R-identity-273220263814737 | GET `/{lang}/recovery-key` | — | player [session] 200 | Yes | No |
| R-identity-276825567686231 | POST `/{lang}/account/recovery-key/revoke` | — | player [session] 403 | No | Yes |
| R-identity-277787908265092 | GET `/{lang}/account/privacy/export` | — | player [session] 200, player [session] 403 | Yes | Yes |
| R-identity-281410731443649 | GET `/account/recovery-key` | — | player [session] 200 | Yes | No |
| F-minigames-24012547260821 | POST `/{lang}/collection/battles/{id}/forfeit` | Unnamed | — | No | No |
| F-minigames-58824648762686 | POST `/{lang}/pass/{season}/claim` | Unnamed | — | No | No |
| F-minigames-114652082160743 | POST `/{lang}/collection/{character}/decks` | Unnamed | — | No | No |
| F-minigames-177564890723493 | POST `/{lang}/collection/battles/{id}` | `action=strike` | — | No | No |
| F-minigames-187973382217026 | POST `/{lang}/collection/decks/{id}/battle` | Unnamed | — | No | No |
| F-minigames-218934052584363 | POST `/{lang}/collection/battles/{id}` | `action=guard` | — | No | No |
| F-minigames-267968195854043 | POST `/{lang}/pass/{season}/buy` | Unnamed | — | No | No |
| F-minigames-280573004026262 | POST `/{lang}/collection/decks/{id}/delete` | Unnamed | — | No | No |
| R-minigames-3564520758599 | POST `/{lang}/collection/decks/{id}/delete` | — | player [session] 403 | No | Yes |
| R-minigames-9197701970789 | GET `/{lang}/pass` | — | anonymous [session] 303, player [session] 503 | Yes | No |
| R-minigames-16116450611608 | POST `/{lang}/collection-pvp` | — | — | No | No |
| R-minigames-29661397326458 | POST `/{lang}/collection/decks/{id}/battle` | — | player [session] 403 | No | Yes |
| R-minigames-47077761371982 | POST `/{lang}/puzzles/{id}/answer` | — | player [session] 403, player [session] 404 | No | Yes |
| R-minigames-55721280504808 | GET `/{lang}/league` | — | anonymous [none] 200, anonymous [none] 503, anonymous [session] 200, player [session] 200 | Yes | No |
| R-minigames-57162011278575 | POST `/{lang}/collection/{character}/decks` | — | player [session] 403 | No | Yes |
| R-minigames-97438513221716 | POST `/{lang}/pass/{season}/buy` | — | player [session] 403 | No | Yes |
| R-minigames-115252755875764 | POST `/{lang}/collection/battles/{id}` | — | — | No | No |
| R-minigames-160461929923229 | GET `/{lang}/collection` | — | anonymous [session] 303, player [session] 200 | Yes | No |
| R-minigames-175093513191111 | POST `/{lang}/pass/{season}/claim` | — | player [session] 403 | No | Yes |
| R-minigames-175709154809888 | GET `/{lang}/collection/{character}/cards/{id}/image` | — | player [session] 503 | No | No |
| R-minigames-228088975038605 | GET `/{lang}/collection/battles/{id}` | — | player [session] 403 | No | Yes |
| R-minigames-230998125928163 | GET `/{lang}/collection-pvp` | — | player [session] 200 | Yes | No |
| R-minigames-238139689824683 | GET `/{lang}/puzzles/{id}/image` | — | player [session] 404 | No | Yes |
| R-minigames-249508045334089 | POST `/{lang}/collection/battles/{id}/forfeit` | — | player [session] 403 | No | Yes |
| R-minigames-257685905203283 | GET `/{lang}/collection/{character}` | — | player [session] 503 | No | No |
| R-minigames-269090115586552 | GET `/{lang}/puzzles` | — | anonymous [session] 303, player [session] 200 | Yes | No |
| F-payments-249659384691764 | POST `/native-account/coin-checkout/{order_id}` | Unnamed | — | No | No |
| R-payments-17029662574282 | GET `/{lang}/panel/premium` | — | player [session] 200 | Yes | No |
| R-payments-56716917761590 | GET `/{lang}/panel/payments/receipts/{id}` | — | player [session] 404 | No | Yes |
| R-payments-72031898488228 | GET `/{lang}/panel/payments` | — | player [session] 200 | Yes | No |
| R-player-api-910659147063 | POST `/api/v1/game-auth/tickets` | — | player [session] 401 | No | Yes |
| R-player-api-50874756068120 | GET `/api/v1/account/summary` | — | player [session] 503 | No | No |
| R-player-api-108163037194743 | GET `/api/v1/feed` | — | player [session] 503 | No | No |
| R-player-api-186671093432766 | GET `/api/v1/game-auth/native-characters` | — | player [session] 401 | No | Yes |
| F-playerdata-91432524848231 | GET `/{lang}/account/hunts` | Unnamed | — | No | No |
| F-playerdata-166859893201249 | GET `/{lang}/account/hunts/compare` | Unnamed | — | No | No |
| R-playerdata-19451765158851 | POST `/{lang}/account/journal/reports/notes` | — | — | No | No |
| R-playerdata-56868995526575 | GET `/{lang}/account/journal/reports/notes/export` | — | player [session] 200 | Yes | No |
| R-playerdata-87418828233486 | GET `/{lang}/atlas/places/participation` | — | player [session] 200 | Yes | No |
| R-playerdata-92486286057690 | POST `/{lang}/atlas/places/participation` | — | — | No | No |
| R-playerdata-94862570978548 | POST `/{lang}/account/journal/reports/email` | — | player [session] 204, player [session] 403 | Yes | Yes |
| R-playerdata-102781281237011 | GET `/{lang}/panel/items/{world}/{character}` | — | player [session] 503 | No | No |
| R-playerdata-103386110781753 | GET `/{lang}/account/journal/reports/notes` | — | player [session] 400 | No | Yes |
| R-playerdata-107763699258555 | GET `/{lang}/account/journal/{character}/deaths` | — | player [session] 404 | No | Yes |
| R-playerdata-120053784269125 | GET `/{lang}/atlas/places` | — | anonymous [session] 303, player [session] 503 | Yes | No |
| R-playerdata-129847983799444 | GET `/{lang}/panel/history/{world}/{family}` | — | player [session] 404 | No | Yes |
| R-playerdata-163620173657280 | GET `/static/playerdata/hunts.css` | — | player [session] 200 | Yes | No |
| R-playerdata-174536791743752 | GET `/{lang}/account/hunts/{id}/share` | — | player [session] 503 | No | No |
| R-playerdata-181945266541482 | GET `/{lang}/account/dashboard` | — | player [session] 503 | No | No |
| R-playerdata-190357879112810 | GET `/{lang}/account/journal/reports` | — | player [session] 503 | No | No |
| R-playerdata-197568170123833 | GET `/account/game-logins` | — | player [session] 503 | No | No |
| R-playerdata-213013233346215 | GET `/{lang}/panel/history/{world}/{family}/{character}` | — | player [session] 404 | No | Yes |
| R-playerdata-215913651815162 | POST `/{lang}/account/hunts/{id}/share` | — | — | No | No |
| R-playerdata-224754459922466 | GET `/{lang}/account/game-logins` | — | player [session] 503 | No | No |
| R-playerdata-243226036540826 | GET `/{lang}/account/hunts/{id}` | — | player [session] 404 | No | Yes |
| R-playerdata-246527543998998 | GET `/{lang}/account/hunts/compare` | — | player [session] 400 | No | Yes |
| R-playerdata-271632340526234 | GET `/{lang}/account/hunts` | — | player [session] 200 | Yes | No |
| F-stats-1942566653442 | GET `/stats/guilds` | Unnamed | — | No | No |
| F-stats-76658134362762 | GET `/{lang}/stats/rankings/{kind}` | Unnamed | — | No | No |
| F-stats-104498940072655 | GET `/{lang}/stats/compare` | Unnamed | — | No | No |
| F-stats-177464129402272 | GET `/{lang}/stats/characters/{id}/{tab}` | Unnamed | — | No | No |
| F-stats-197040864587241 | GET `/stats/world` | Unnamed | — | No | No |
| F-stats-210255257429913 | GET `/{lang}/stats/world` | Unnamed | anonymous [none] 503 | No | No |
| F-stats-216992143813701 | GET `/{lang}/stats/rankings/{kind}` | `page={{ page + 1 }}` | — | No | No |
| F-stats-222324429744041 | GET `/{lang}/stats/rankings/{kind}` | `page={{ page - 1 }}` | — | No | No |
| F-stats-229832046893417 | GET `/{lang}/stats/guilds` | Unnamed | — | No | No |
| R-stats-823832337050 | POST `/api/v1/stats/watches` | — | — | No | No |
| R-stats-8216417320629 | GET `/{lang}/stats/characters/{id}/{tab}` | — | player [session] 503 | No | No |
| R-stats-26389072922104 | GET `/{lang}/stats/guilds` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-stats-101232934958655 | GET `/{lang}/stats/compare` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-stats-109362773213568 | DELETE `/api/v1/stats/watches` | — | — | No | No |
| R-stats-230622861255655 | GET `/stats/guilds` | — | player [session] 503 | No | No |
| R-stats-241837621972072 | GET `/{lang}/stats/world` | — | anonymous [none] 200, anonymous [none] 503, player [session] 200, player [session] 503 | Yes | No |
| R-stats-250762956540039 | GET `/stats/world` | — | player [session] 200 | Yes | No |
| R-stats-255895304059255 | GET `/{lang}/stats/rankings/{kind}` | — | anonymous [session] 503, player [session] 503 | No | No |
| R-stats-261187344874560 | GET `/api/v1/stats/watches` | — | player [session] 401 | No | Yes |
| R-store-18762022575911 | GET `/{lang}/shop/coins` | — | player [session] 200 | Yes | No |
| R-store-51033146382838 | POST `/{lang}/shop/transfer` | — | player [session] 403 | No | Yes |
| R-store-156552937381132 | POST `/{lang}/shop/coins` | — | player [session] 303, player [session] 403 | Yes | Yes |
| R-store-177814716613157 | GET `/{lang}/shop/history` | — | player [session] 200 | Yes | No |
| R-store-188591329890189 | POST `/__test/payments/webhook` | — | player [session] 204, player [session] 400 | Yes | Yes |
| R-store-212343227939352 | GET `/{lang}/shop` | — | player [session] 200 | Yes | No |
| R-store-216753360486474 | GET `/{lang}/shop/transfer` | — | player [session] 200 | Yes | No |
| R-store-225256665726123 | POST `/{lang}/shop` | — | player [session] 403 | No | Yes |
| R-support-26027271340378 | GET `/{lang}/support/tickets/{id}` | — | player [session] 200, player [session] 404 | Yes | Yes |
| R-support-26185575404256 | POST `/{lang}/support/tickets` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-support-30148363825726 | GET `/{lang}/support/tickets/create` | — | player [session] 200 | Yes | No |
| R-support-44845672236057 | POST `/{lang}/admin/support/tickets/{id}/status` | — | platform_admin [session] 403 | No | Yes |
| R-support-49213692674501 | POST `/{lang}/admin/support/tickets/{id}/reply` | — | platform_admin [session] 403 | No | Yes |
| R-support-117891070916973 | POST `/{lang}/support/tickets/{id}/status` | — | player [session] 403 | No | Yes |
| R-support-120909794989394 | PUT `/{lang}/admin/support/tickets/{id}/status` | — | — | No | No |
| R-support-182126667000584 | GET `/{lang}/admin/support/tickets` | — | platform_admin [session] 200 | Yes | No |
| R-support-194055504388325 | GET `/{lang}/admin/support/tickets/{id}` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-support-212471973415644 | GET `/{lang}/support/reports/create` | — | player [session] 200 | Yes | No |
| R-support-212881022207524 | GET `/{lang}/support/tickets` | — | player [session] 200 | Yes | No |
| R-support-223094873801680 | POST `/{lang}/support/reports` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-support-229098672793467 | POST `/{lang}/support/tickets/{id}/reply` | — | player [session] 302, player [session] 403 | Yes | Yes |
| R-support-241132021282755 | GET `/{lang}/support/reports/{id}` | — | player [session] 200, player [session] 404 | Yes | Yes |
| R-support-268283993558513 | GET `/{lang}/support/reports` | — | player [session] 200 | Yes | No |
| R-support-268810500855670 | PUT `/{lang}/support/tickets/{id}/status` | — | — | No | No |
| F-web-15366296186789 | POST `/account/badges` | Unnamed | — | No | No |
| F-web-16801066688064 | POST `/{lang}/account/characters/{name}/profile` | Unnamed | player [session] 303 | Yes | No |
| F-web-26667257346850 | GET `/search` | Unnamed | — | No | No |
| F-web-33906076394532 | POST `/{lang}/account/characters/{character}/services` | Unnamed | — | No | No |
| F-web-55121824626359 | POST `/{lang}/admin/store/codes` | Unnamed | — | No | No |
| F-web-81506816722856 | GET `/{lang}/search` | Unnamed | — | No | No |
| F-web-142955662443676 | POST `/{lang}/verify-email/resend` | Unnamed | — | No | No |
| F-web-196607846875803 | POST `/{lang}/register` | Unnamed | unresolved [session] 303 | Yes | No |
| F-web-257415907679051 | POST `/account/characters/{name}/profile` | Unnamed | — | No | No |
| R-web-12223205913456 | GET `/account` | — | player [session] 200, unresolved [session] 303 | Yes | No |
| R-web-19543202782207 | POST `/register` | — | — | No | No |
| R-web-36424599356232 | POST `/{lang}/verify-email/resend` | — | player [session] 403 | No | Yes |
| R-web-40453000426730 | GET `/{lang}/search` | — | player [session] 200 | Yes | No |
| R-web-40585967999127 | GET `/readyz` | — | player [session] 200 | Yes | No |
| R-web-43211932877820 | GET `/account/characters/{name}/profile` | — | player [session] 404 | No | Yes |
| R-web-47033674725229 | POST `/{lang}/register` | — | anonymous [session] 403, unresolved [session] 303 | Yes | Yes |
| R-web-70824292890333 | GET `/{lang}/account/characters/{name}/profile` | — | player [session] 200, player [session] 404 | Yes | Yes |
| R-web-74427081425893 | GET `/` | — | player [session] 303 | Yes | No |
| R-web-76114957679878 | GET `/{lang}/verify-email` | — | anonymous [session] 200 | Yes | No |
| R-web-85855948179462 | GET `/robots.txt` | — | player [session] 200 | Yes | No |
| R-web-98846564089840 | POST `/{lang}/account/characters/{character}/services` | — | — | No | No |
| R-web-99836072068686 | GET `/register` | — | player [session] 303 | Yes | No |
| R-web-100024285073621 | GET `/sitemap.xml` | — | player [session] 200 | Yes | No |
| R-web-113799923832122 | GET `/verify-email/resend` | — | player [session] 200 | Yes | No |
| R-web-158989864773554 | GET `/favicon.ico` | — | player [session] 200 | Yes | No |
| R-web-163221802702173 | GET `/search` | — | player [session] 200 | Yes | No |
| R-web-164503364284350 | GET `/verify-email` | — | player [session] 422 | No | Yes |
| R-web-167806610250999 | GET `/register/check-inbox` | — | player [session] 200 | Yes | No |
| R-web-173474453673449 | POST `/verify-email/resend` | — | — | No | No |
| R-web-187490726388487 | GET `/healthz` | — | player [session] 200 | Yes | No |
| R-web-193138689917708 | GET `/language/{lang}` | — | platform_admin [session] 303, player [session] 303 | Yes | No |
| R-web-198848616393959 | GET `/{lang}/verify-email/resend` | — | player [session] 200 | Yes | No |
| R-web-203472439833503 | POST `/account/badges` | — | player [session] 403 | No | Yes |
| R-web-210399968917343 | GET `/{lang}/register/check-inbox` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-web-234139341996349 | GET `/account/badges` | — | player [session] 200 | Yes | No |
| R-web-242750870538372 | GET `/{lang}/` | — | anonymous [none] 200, anonymous [session] 200, platform_admin [session] 200, player [session] 200, staff_rbac_ea6223f08938fd80258e [session] 200, unresolved [session] 200 | Yes | No |
| R-web-254508599152825 | GET `/{lang}/account/characters/{character}/services` | — | player [session] 503 | No | No |
| R-web-268270818038463 | GET `/{lang}/register` | — | anonymous [none] 200, anonymous [session] 200, player [session] 303 | Yes | No |
| R-web-278802062776619 | GET `/{lang}` | — | player [session] 200 | Yes | No |
| F-wiki-5428367139875 | GET `/{lang}/wiki/{kind}` | Unnamed | — | No | No |
| F-wiki-7972091134458 | POST `/admin/wiki/articles/{id}/submit-review` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-wiki-22119884537424 | GET `/{lang}/wiki/search` | Unnamed | — | No | No |
| F-wiki-22599257223435 | POST `/admin/wiki/articles/{id}` | Unnamed | — | No | No |
| F-wiki-58594829099106 | GET `/{lang}/wiki` | Unnamed | — | No | No |
| F-wiki-65456549216585 | POST `/admin/wiki/articles/{id}/archive` | Unnamed | — | No | No |
| F-wiki-71042775092727 | POST `/admin/wiki/articles/{id}/revisions/{source}/restore` | Unnamed | — | No | No |
| F-wiki-73563493703335 | GET `/{lang}/wiki/calculators` (`calculator=training`) | Unnamed | — | No | No |
| F-wiki-79248852392804 | GET `/{lang}/wiki/calculators` (`calculator=experience`) | Unnamed | — | No | No |
| F-wiki-104524932852102 | POST `/admin/wiki/articles/{id}/return-draft` | Unnamed | — | No | No |
| F-wiki-112407452983375 | POST `/admin/wiki/articles/{id}/unpublish` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-wiki-154492065254748 | GET `/wiki/search` | Unnamed | — | No | No |
| F-wiki-155393530910829 | POST `/admin/wiki/articles/{id}/publish` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-wiki-172496912900389 | GET `/wiki/{kind}` | Unnamed | — | No | No |
| F-wiki-210711556138269 | GET `/{lang}/wiki/calculators` (`calculator=loot`) | Unnamed | — | No | No |
| F-wiki-248339543609206 | POST `/admin/wiki/articles` | Unnamed | platform_admin [session] 303 | Yes | No |
| F-wiki-250551961713734 | GET `/wiki` | Unnamed | — | No | No |
| F-wiki-278439863293280 | GET `/{lang}/wiki/calculators` (`calculator=stamina`) | Unnamed | — | No | No |
| R-wiki-4248782030266 | GET `/wiki/{kind}` | — | player [session] 503 | No | No |
| R-wiki-33583177436350 | GET `/wiki/search` | — | player [session] 503 | No | No |
| R-wiki-39752312380693 | GET `/wiki` | — | player [session] 503 | No | No |
| R-wiki-51780182919908 | GET `/{lang}/wiki/calculators` | — | anonymous [session] 200, player [session] 200 | Yes | No |
| R-wiki-93672775642610 | POST `/admin/wiki/articles/{id}/revisions/{source}/restore` | — | platform_admin [session] 403 | No | Yes |
| R-wiki-106194236862500 | GET `/admin/wiki/articles` | — | platform_admin [session] 200 | Yes | No |
| R-wiki-108674912102403 | GET `/{lang}/wiki/articles/{slug}` | — | platform_admin [session] 200, platform_admin [session] 404, player [session] 404 | Yes | Yes |
| R-wiki-111543973842280 | GET `/{lang}/wiki/{kind}` | — | anonymous [none] 503, anonymous [session] 503, player [session] 503 | No | No |
| R-wiki-120400192169802 | POST `/admin/wiki/articles` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-wiki-186041833840501 | GET `/{lang}/wiki/places` | — | player [session] 503 | No | No |
| R-wiki-194496811298106 | GET `/admin/wiki/articles/{id}/revisions` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-wiki-208209495456737 | GET `/wiki/{kind}/{key}` | — | player [session] 503 | No | No |
| R-wiki-215772334129650 | POST `/admin/wiki/articles/{id}/{action}` | — | platform_admin [session] 303, platform_admin [session] 403 | Yes | Yes |
| R-wiki-216400093677132 | GET `/wiki/articles/{slug}` | — | player [session] 404 | No | Yes |
| R-wiki-219021727632959 | GET `/{lang}/wiki/search` | — | anonymous [none] 503, player [session] 503 | No | No |
| R-wiki-224468693872175 | GET `/admin/wiki` | — | platform_admin [session] 200 | Yes | No |
| R-wiki-226002053566292 | POST `/admin/wiki/articles/{id}` | — | platform_admin [session] 403 | No | Yes |
| R-wiki-243565095745956 | GET `/admin/wiki/articles/{id}/preview/{lang}` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-wiki-243845325258228 | GET `/admin/wiki/articles/{id}/edit` | — | platform_admin [session] 200, platform_admin [session] 404 | Yes | Yes |
| R-wiki-256187555116423 | GET `/{lang}/wiki/{kind}/{key}` | — | player [session] 503 | No | No |
| R-wiki-264289199557764 | GET `/admin/wiki/articles/create` | — | platform_admin [session] 200 | Yes | No |
| R-wiki-280322352905156 | GET `/{lang}/wiki` | — | anonymous [none] 503, anonymous [session] 503, player [session] 503 | No | No |
| F-world-86048466149919 | POST `/admin/world` | Unnamed | — | No | No |
| F-world-89263600590152 | GET `/admin/world` | Unnamed | — | No | No |
| R-world-47245520267544 | GET `/admin/world` | — | platform_admin [session] 200 | Yes | No |
| R-world-71870839689412 | POST `/admin/world` | — | — | No | No |
