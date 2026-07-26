# Wiki launch-content source map

## Purpose

This record maps Wiki launch-content version `2026-07-26.1` to accepted repository evidence. It is a review aid, not a second runtime content source.

The canonical package is `app/Wiki/Content/WikiLaunchContentCatalog.php`. Installation policy is ADR 0015.

## Evidence classification

### PROVEN

- The Platform implements route-backed Download, registration, account overview, character creation, MFA, announcements, Server Information, Rules, Support and Report a Bug surfaces.
- ADR 0005 approves the five base vocation choices exposed by current character creation.
- Wiki publication requires complete English and Polish content.
- The game-login contracts and project checkpoint do not prove a completed universal Platform-to-game-client login path.

### UNKNOWN

- current numeric Oteryn server rates;
- detailed approved vocation mechanics, spells and progression advice;
- complete current Oteryn PvP and game-rule text;
- approved Discord destination;
- final authoritative game-client login behavior and rollout.

These unknowns remain explicit in the public copy. The catalog points readers to current published operator/runtime sources and never invents a value.

## Article map

| Launch topic | English / Polish slug | Primary repository evidence | Explicit exclusions |
|---|---|---|---|
| Download and Installation | `download-and-installation` / `pobieranie-i-instalacja` | `app/Downloads/PublicDownloadCenterQuery.php`, `app/Downloads/Actions/PublishClientRelease.php`, `resources/views/downloads/index.blade.php` | No invented OS-specific install sequence; no unapproved mirror |
| Creating an Account | `creating-an-account` / `tworzenie-konta` | registration controller, account-overview read model, ADR 0004 | No existing-account claim/import |
| Creating a Character | `creating-a-character` / `tworzenie-postaci` | ADR 0005, create-character action and form | No unapproved vocation or starter/gameplay claims |
| First Login | `first-login` / `pierwsze-logowanie` | game-login contracts and `PROJECT_STATE.md` | No claim that Platform web credentials work in every client |
| Server Information | `server-information` / `informacje-o-serwerze` | homepage query/view and typed Server Information page | No copied runtime value |
| Server Rates | `server-rates` / `tempo-serwera` | public-site plan and typed Server Information page | No numeric rate value |
| Vocations | `vocations` / `profesje` | ADR 0005, create-character action and form | No spell, formula, equipment or progression guide |
| PvP and Game Rules | `pvp-and-game-rules` / `pvp-i-zasady-gry` | homepage runtime summary, typed Rules page and public-site plan | No fixed PvP type, penalty or exception |
| Account Security and MFA | `account-security-and-mfa` / `bezpieczenstwo-konta-i-mfa` | security architecture and MFA implementation | No credential or recovery material |
| FAQ | `frequently-asked-questions` / `najczesciej-zadawane-pytania` | account state, character action and ADR 0013 | No fallback from missing public state |
| Known Issues | `known-issues` / `znane-problemy` | typed support topics, security architecture and release-note UI | No fabricated empty or complete issue list |
| Discord and Support | `discord-and-support` / `discord-i-wsparcie` | typed Support page, support routes and public-site plan | No Discord URL |
| Report a Bug | `report-a-bug` / `zglos-blad` | typed Report a Bug page, support routes and security architecture | No secrets, dumps or unnecessary personal data |

## Review checklist

- Exactly thirteen language-independent articles exist.
- Every article has non-empty English and Polish title, slug, summary and restricted Markdown.
- Each article has at least one existing repository source reference.
- No raw HTML, remote image, arbitrary iframe/script or dangerous URL protocol is present.
- Cross-Wiki links use published localized slugs.
- English is authored first and Polish is created in the same transaction so Polish is not stale at initial publication.
- Later editorial changes are never overwritten by the installer.
