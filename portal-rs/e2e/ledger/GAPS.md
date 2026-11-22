# Remaining browser coverage gaps

The [coverage delta](DELTA.md) lists the request pattern for each row. These reasons describe the fixtures and browser actions used to produce it.

## Forms without passing submission evidence

| Rows | Reason |
| --- | --- |
| `F-admin-45079342387959`, `F-admin-137657189881775`, `F-admin-151927750828134`, `F-admin-223893554250958` | The console navigation checks open the pages directly, leaving the agent and target selector unsubmitted. |
| `F-admin-58876857851269`, `F-admin-215839569153342` | The console access checks open the audit and player read pages directly, leaving the audit filter and the read-only character lookup unsubmitted. |
| `F-admin-169544806443708` | The Analytics test opens the economy report with a prefilled URL, leaving the economy filter unsubmitted. |
| `F-content-21985737393666` | The content suite submits the event creation and status forms. The branch case edits an existing event with `PUT`. The edit form itself submits with `POST`. |
| `F-web-15366296186789` | The unprefixed badge POST and localized redirect have evidence from a Rust integration test. The badge selection form remains unsubmitted in both browser captures. |
| `F-wiki-22599257223435`, `F-wiki-65456549216585`, `F-wiki-71042775092727` | The Wiki browser flows create a draft, change its review and publication states, and open history. Existing article edit, archive, and revision restore have no browser submission evidence. |

## Authority and fixture gaps

| Rows | Reason |
| --- | --- |
| `F-admin-2051149960876`, `F-admin-38702896297808`, `F-admin-138082345336189`, `F-admin-253054411857215`, `R-admin-1919642460343`, `R-admin-122795670899350`, `R-admin-136010976080442`, `R-admin-260508168334342` | The console only renders these POST controls when a connected, fresh Admin Agent owns the target and advertises the operation. The fixture has no such connected agent. |
| `F-admin-17456964450888` | The player search POST form requires a connected Admin Agent advertising `player.search`. |
| `F-atlas-161391314177223` | The bestiary form requires the owner's progress. The configured Atlas source returns unavailable. |
| `F-bazaar-249147474695525` | The filter requires a signed Game snapshot with an item list. The pinned character fixture omits items. |
| `F-community-3881337848774`, `F-community-16287300610837`, `F-community-193593469157960`, `F-community-219222581560817`, `F-community-4730707544746`, `F-community-274165327503467`, `F-community-71546943467822` | The deaths, guild and ranking pages require fresh, complete Game projections to render their filters. The available Game fixture cannot supply those cuts. |
| `F-minigames-114652082160743`, `F-minigames-187973382217026`, `F-minigames-280573004026262` | Collection deck controls require an admitted collection owned by a character. |
| `F-minigames-24012547260821`, `F-minigames-177564890723493`, `F-minigames-218934052584363`, `R-minigames-115252755875764` | Battle actions require a deck and an owned active battle from an admitted character collection. |
| `F-minigames-58824648762686`, `F-minigames-267968195854043` | Claim and purchase controls require an admitted season and authoritative progress. |
| `F-payments-249659384691764` | Native coin checkout requires an existing Game-authorized order for the authenticated account. |
| `F-stats-216992143813701`, `F-stats-222324429744041` | The statistics filter is covered. Forward and back controls require multiple pages of Game rankings. |
| `F-web-33906076394532`, `R-web-98846564089840` | The character rename and deletion form requires recent lifecycle evidence specific to those operations. The fixture supplies character ownership. |
| `F-wiki-5428367139875`, `F-wiki-172496912900389`, `F-wiki-22119884537424`, `F-wiki-58594829099106`, `F-wiki-154492065254748`, `F-wiki-250551961713734` | Entity and search filters and pagination require an admitted catalogue. The pinned WorldStore candidate remains inactive. |
| `F-world-86048466149919`, `F-world-89263600590152`, `R-world-71870839689412` | The version picker and write controls require a selectable WorldStore version. The inactive fixture candidate has no `world_versions` entry. |

## Legacy and duplicate inventory entries

| Rows | Reason |
| --- | --- |
| `F-bazaar-23275926065244`, `F-bazaar-46767873991968`, `F-bazaar-86148115874390`, `F-bazaar-140015055205774`, `F-bazaar-160340630540621`, `F-bazaar-187660078956952`, `F-bazaar-221617554685096`, `F-bazaar-231195288605257`, `F-bazaar-250676990346223`, `R-bazaar-13638167717685` | The composed portal uses the Game Bazaar sale, bid and control routes. They supersede these legacy create, bid, offer and item-state forms. |
| `R-bazaar-137999873732047` | The inventory has two indistinguishable patterns for this GET route. The recorder assigns the request to `R-bazaar-82908904463903` in inventory order. |
