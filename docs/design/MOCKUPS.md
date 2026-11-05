# Mockups for owner review

Static Polish design fixtures using the existing T39 dark tokens and components. All names and data are invented. No routes, scripts, backend actions or enabled game/money features. Open from a repository-root HTTP server.

| Screen | HTML | 360 px PNG | 1280 px PNG |
| --- | --- | --- | --- |
| Strona główna | [HTML](../../portal-rs/templates/mockups/home.html) | [360](screenshots/home-360.png) | [1280](screenshots/home-1280.png) |
| Artykuł | [HTML](../../portal-rs/templates/mockups/news.html) | [360](screenshots/news-360.png) | [1280](screenshots/news-1280.png) |
| Rankingi | [HTML](../../portal-rs/templates/mockups/rankings.html) | [360](screenshots/rankings-360.png) | [1280](screenshots/rankings-1280.png) |
| Profil postaci | [HTML](../../portal-rs/templates/mockups/character.html) | [360](screenshots/character-360.png) | [1280](screenshots/character-1280.png) |
| Konto | [HTML](../../portal-rs/templates/mockups/account.html) | [360](screenshots/account-360.png) | [1280](screenshots/account-1280.png) |
| Logowanie | [HTML](../../portal-rs/templates/mockups/login.html) | [360](screenshots/login-360.png) | [1280](screenshots/login-1280.png) |
| Rejestracja | [HTML](../../portal-rs/templates/mockups/registration.html) | [360](screenshots/registration-360.png) | [1280](screenshots/registration-1280.png) |
| Sklep | [HTML](../../portal-rs/templates/mockups/shop.html) | [360](screenshots/shop-360.png) | [1280](screenshots/shop-1280.png) |
| Bazar | [HTML](../../portal-rs/templates/mockups/bazaar.html) | [360](screenshots/bazaar-360.png) | [1280](screenshots/bazaar-1280.png) |
| Wiki | [HTML](../../portal-rs/templates/mockups/wiki.html) | [360](screenshots/wiki-360.png) | [1280](screenshots/wiki-1280.png) |
| Atlas | [HTML](../../portal-rs/templates/mockups/atlas.html) | [360](screenshots/atlas-360.png) | [1280](screenshots/atlas-1280.png) |
| Panel zespołu | [HTML](../../portal-rs/templates/mockups/team.html) | [360](screenshots/team-360.png) | [1280](screenshots/team-1280.png) |

## Open owner decisions

- Approve the dark theme, density and two-column desktop layout?
- Should the home page prioritize onboarding or returning players?
- Approve the public character privacy treatment and account summary hierarchy?
- Which Atlas layers should be visible initially? The current schematic is not a real map.
- Approve disabled commerce presentation and the team queue priorities?

## Validation

12 documents parsed with strict html5lib; 24 full-page Playwright PNGs at deviceScaleFactor 1; no horizontal page overflow at 360 or 1280 px. Reused T39 contrast pairs, focus rings, labels and target sizes. Manual accessibility review remains part of owner approval; automated parsing does not establish full WCAG conformance.

Run `node docs/design/check-mockups.cjs` from the repository root with the supplied environment and a root HTTP server on port 8099. The script requires the environment-provided Playwright installation.
