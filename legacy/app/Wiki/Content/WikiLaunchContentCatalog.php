<?php

namespace App\Wiki\Content;

use App\Wiki\Domain\WikiCategoryTranslationInput;

final class WikiLaunchContentCatalog
{
    public const VERSION = '2026-07-26.1';

    /**
     * @return non-empty-list<WikiLaunchCategory>
     */
    public function categories(): array
    {
        return [
            $this->category(
                'getting-started',
                10,
                'Getting Started',
                'getting-started',
                'Verified steps for installing the client and preparing an Oteryn account.',
                'Pierwsze kroki',
                'pierwsze-kroki',
                'Sprawdzone kroki instalacji klienta i przygotowania konta Oteryn.',
            ),
            $this->category(
                'server-information',
                20,
                'Server Information',
                'server-information',
                'Authoritative pointers for current world configuration and rules.',
                'Informacje o serwerze',
                'informacje-o-serwerze',
                'Autorytatywne źródła bieżącej konfiguracji świata i zasad.',
            ),
            $this->category(
                'game-systems',
                30,
                'Game Systems',
                'game-systems',
                'Only game-system facts that are established by current Oteryn product policy.',
                'Systemy gry',
                'systemy-gry',
                'Wyłącznie fakty o systemach gry potwierdzone przez bieżącą politykę produktu Oteryn.',
            ),
            $this->category(
                'support',
                40,
                'Support',
                'support',
                'Account safety, known-issue checks and official support paths.',
                'Wsparcie',
                'wsparcie',
                'Bezpieczeństwo konta, sprawdzanie znanych problemów i oficjalne kanały wsparcia.',
            ),
        ];
    }

    /**
     * @return non-empty-list<WikiLaunchArticle>
     */
    public function articles(): array
    {
        return [
            new WikiLaunchArticle(
                'guide',
                true,
                10,
                ['getting-started'],
                [
                    $this->translation(
                        'en',
                        'Download and installation',
                        'download-and-installation',
                        'Use the approved Download Center and verify the published artifact before installation.',
                        <<<'MARKDOWN'
# Download from the approved source

Open the [Download Center](/download). It lists only currently published client records and shows the version, supported platform, file size and SHA-256 checksum for each approved artifact.

If the page reports that no current download is available, do not substitute a client from an unverified mirror. Check [Support](/support) or wait for an official release notice.

## Verify before installation

1. Choose the artifact for the platform shown by the Download Center.
2. Confirm that the downloaded filename and byte size match the published record.
3. Calculate the file's SHA-256 digest and compare the complete value with the published checksum.
4. Follow the release notes attached to that exact version.

This guide does not invent operating-system steps that have not been published for the selected release.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Pobieranie i instalacja',
                        'pobieranie-i-instalacja',
                        'Użyj zatwierdzonego Centrum pobierania i sprawdź opublikowany plik przed instalacją.',
                        <<<'MARKDOWN'
# Pobierz z zatwierdzonego źródła

Otwórz [Centrum pobierania](/download). Zawiera ono wyłącznie aktualnie opublikowane rekordy klienta oraz pokazuje wersję, obsługiwaną platformę, rozmiar pliku i sumę SHA-256 każdego zatwierdzonego artefaktu.

Jeżeli strona informuje, że aktualny plik nie jest dostępny, nie zastępuj go klientem z niezweryfikowanego źródła. Sprawdź [Wsparcie](/support) albo poczekaj na oficjalną informację o wydaniu.

## Sprawdź plik przed instalacją

1. Wybierz artefakt dla platformy wskazanej w Centrum pobierania.
2. Porównaj nazwę i rozmiar pobranego pliku z opublikowanym rekordem.
3. Oblicz sumę SHA-256 pliku i porównaj pełną wartość z opublikowaną sumą.
4. Postępuj zgodnie z informacjami o dokładnie tej wersji.

Ten poradnik nie tworzy kroków dla systemu operacyjnego, które nie zostały opublikowane dla wybranego wydania.
MARKDOWN,
                    ),
                ],
                [
                    'app/Downloads/PublicDownloadCenterQuery.php',
                    'app/Downloads/Actions/PublishClientRelease.php',
                    'resources/views/downloads/index.blade.php',
                ],
            ),
            new WikiLaunchArticle(
                'guide',
                false,
                20,
                ['getting-started'],
                [
                    $this->translation(
                        'en',
                        'Creating an account',
                        'creating-an-account',
                        'Create one Platform identity and follow its explicit game-account provisioning state.',
                        <<<'MARKDOWN'
# Register a Platform identity

Use the [registration page](/register) with an email address you control and a password you do not reuse elsewhere. Complete any verification step shown by the Platform, then sign in through the Oteryn website.

## Check game-account readiness

Open [your account overview](/account). Oteryn shows whether the supported game account is ready, still pending, temporarily unavailable or requires operator recovery.

Do not create a second identity to work around a pending or unavailable state. The supported ownership rule is one Platform identity bound to one greenfield game account. Use [Support](/support) when the account overview asks for assistance.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Tworzenie konta',
                        'tworzenie-konta',
                        'Utwórz jedną tożsamość Platformy i postępuj zgodnie z jawnym stanem przygotowania konta gry.',
                        <<<'MARKDOWN'
# Zarejestruj tożsamość Platformy

Użyj [strony rejestracji](/register), podając kontrolowany przez siebie adres e-mail i hasło, którego nie używasz w innym serwisie. Wykonaj kroki weryfikacyjne pokazane przez Platformę, a następnie zaloguj się w witrynie Oteryn.

## Sprawdź gotowość konta gry

Otwórz [podsumowanie konta](/account). Oteryn pokazuje, czy obsługiwane konto gry jest gotowe, nadal oczekuje, jest tymczasowo niedostępne albo wymaga pomocy operatora.

Nie twórz drugiej tożsamości, aby omijać stan oczekiwania lub niedostępności. Obsługiwana zasada własności to jedna tożsamość Platformy powiązana z jednym nowym kontem gry. Skorzystaj ze [Wsparcia](/support), gdy podsumowanie konta prosi o pomoc.
MARKDOWN,
                    ),
                ],
                [
                    'app/Http/Controllers/Identity/RegistrationController.php',
                    'app/Accounts/ReadModels/AccountOverviewReadModel.php',
                    'docs/architecture/adr/0004-authoritative-platform-account-ownership.md',
                ],
            ),
            new WikiLaunchArticle(
                'guide',
                false,
                30,
                ['getting-started', 'game-systems'],
                [
                    $this->translation(
                        'en',
                        'Creating a character',
                        'creating-a-character',
                        'Create a character only after the account overview confirms that the bound game account is ready.',
                        <<<'MARKDOWN'
# Before you begin

Sign in and confirm on [your account overview](/account) that the bound game account is ready. Character creation fails closed while ownership is pending, unavailable or ambiguous.

## Choose the supported inputs

The current creation form accepts:

- a character name of one to three ASCII-letter words;
- one base vocation: Sorcerer, Druid, Paladin, Knight or Monk;
- one of the two displayed sex values.

Promoted vocations are progression choices and cannot be selected during web creation. The Platform derives the target account from your authenticated identity; you cannot choose another account identifier.

Open [Create character](/account/characters/create) only when your account is ready. A duplicate name or full active-character quota is reported without creating a partial character.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Tworzenie postaci',
                        'tworzenie-postaci',
                        'Utwórz postać dopiero wtedy, gdy podsumowanie konta potwierdzi gotowość powiązanego konta gry.',
                        <<<'MARKDOWN'
# Zanim zaczniesz

Zaloguj się i sprawdź w [podsumowaniu konta](/account), czy powiązane konto gry jest gotowe. Tworzenie postaci jest bezpiecznie odrzucane, gdy własność oczekuje, jest niedostępna albo niejednoznaczna.

## Wybierz obsługiwane dane

Aktualny formularz tworzenia przyjmuje:

- nazwę postaci złożoną z jednego do trzech słów zapisanych literami ASCII;
- jedną profesję bazową: Sorcerer, Druid, Paladin, Knight albo Monk;
- jedną z dwóch wartości płci pokazanych w formularzu.

Profesje promowane są elementem rozwoju i nie można ich wybrać podczas tworzenia w witrynie. Platforma wyznacza konto docelowe z zalogowanej tożsamości; nie można podać innego identyfikatora konta.

Otwórz [Tworzenie postaci](/account/characters/create) dopiero po przygotowaniu konta. Zajęta nazwa lub pełny limit aktywnych postaci są zgłaszane bez utworzenia częściowego rekordu.
MARKDOWN,
                    ),
                ],
                [
                    'docs/architecture/adr/0005-character-creation-product-policy.md',
                    'app/Characters/Actions/CreateCharacter.php',
                    'resources/views/characters/create.blade.php',
                ],
            ),
            new WikiLaunchArticle(
                'guide',
                false,
                40,
                ['getting-started'],
                [
                    $this->translation(
                        'en',
                        'First login',
                        'first-login',
                        'Use only a game-client login method that Oteryn has explicitly published for the current release.',
                        <<<'MARKDOWN'
# Confirm the current login instructions

Install an approved client from the [Download Center](/download), then read the release notes and current [announcements](/announcements). Use a game-client sign-in method only when those Oteryn sources explicitly describe it.

The Platform website can register an identity, prepare a supported game account and create characters. This launch guide does not claim that Platform web credentials are already accepted by every game client or login service.

If no current client-login method is published, stop and contact [Support](/support). Do not test your password in unofficial clients.

## Keep web MFA private

MFA one-time codes and recovery codes belong to the Platform website. Never enter them into the game client, a chat message or a support report.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Pierwsze logowanie',
                        'pierwsze-logowanie',
                        'Używaj wyłącznie sposobu logowania w kliencie gry jawnie opublikowanego przez Oteryn dla bieżącego wydania.',
                        <<<'MARKDOWN'
# Potwierdź aktualne instrukcje logowania

Zainstaluj zatwierdzonego klienta z [Centrum pobierania](/download), a następnie przeczytaj informacje o wydaniu i bieżące [ogłoszenia](/announcements). Używaj sposobu logowania w kliencie gry tylko wtedy, gdy te źródła Oteryn opisują go wprost.

Witryna Platformy potrafi zarejestrować tożsamość, przygotować obsługiwane konto gry i utworzyć postacie. Ten poradnik startowy nie stwierdza, że internetowe dane logowania Platformy są już przyjmowane przez każdy klient lub serwer logowania.

Jeżeli aktualny sposób logowania klienta nie został opublikowany, zatrzymaj się i skontaktuj ze [Wsparciem](/support). Nie sprawdzaj hasła w nieoficjalnych klientach.

## Chroń internetowe MFA

Jednorazowe kody MFA i kody odzyskiwania należą do witryny Platformy. Nigdy nie wpisuj ich w kliencie gry, wiadomości czatu ani zgłoszeniu do wsparcia.
MARKDOWN,
                    ),
                ],
                [
                    'docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md',
                    'docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md',
                    'docs/agents/PROJECT_STATE.md',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                true,
                50,
                ['server-information'],
                [
                    $this->translation(
                        'en',
                        'Server information',
                        'server-information',
                        'Read current world status and configuration from the live public surfaces instead of a copied snapshot.',
                        <<<'MARKDOWN'
# Current world information

Use the [Oteryn homepage](/) for the current configured world channel. It distinguishes available, empty, stale and unavailable data rather than turning a missing dependency into a made-up zero.

When supplied by the authoritative runtime boundary, the world summary may show status, online count, region, PvP type, capacity, server-save time and data freshness. Treat an unavailable or stale label as part of the result.

Use the published [Server Information](/server-information) page for operator-maintained details. Values are deliberately not copied into this Wiki article because current configuration can change independently of a Wiki revision.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Informacje o serwerze',
                        'informacje-o-serwerze',
                        'Odczytuj bieżący stan i konfigurację świata z aktywnych stron publicznych zamiast z kopii.',
                        <<<'MARKDOWN'
# Bieżące informacje o świecie

Użyj [strony głównej Oteryn](/), aby sprawdzić aktualnie skonfigurowany kanał świata. Strona rozróżnia dane dostępne, puste, nieaktualne i niedostępne, zamiast zamieniać brak zależności w zmyślone zero.

Gdy dostarcza je autorytatywna granica danych, podsumowanie świata może pokazać stan, liczbę graczy online, region, typ PvP, pojemność, czas zapisu serwera i świeżość danych. Etykietę niedostępności lub nieaktualności traktuj jako część wyniku.

Szczegóły utrzymywane przez operatora znajdziesz na opublikowanej stronie [Informacje o serwerze](/server-information). Wartości celowo nie są kopiowane do tego artykułu Wiki, ponieważ bieżąca konfiguracja może zmienić się niezależnie od rewizji Wiki.
MARKDOWN,
                    ),
                ],
                [
                    'app/PublicPortal/HomePageQuery.php',
                    'resources/views/home.blade.php',
                    'app/Cms/Editorial/EditorialPageKey.php',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                false,
                60,
                ['server-information'],
                [
                    $this->translation(
                        'en',
                        'Server rates',
                        'server-rates',
                        'Use only rate values published through Oteryn server information or announcements.',
                        <<<'MARKDOWN'
# Authoritative rate values

This launch-content package contains no approved numeric experience, skill, magic, loot, spawn or other gameplay rate values.

Check the published [Server Information](/server-information) page and current [announcements](/announcements). A numeric value is authoritative only when an Oteryn publisher places it on one of those current public surfaces.

If a rate is absent, do not infer a default from another server, Canary configuration examples or an old client. Ask [Support](/support) before making a gameplay decision that depends on it.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Tempo serwera',
                        'tempo-serwera',
                        'Używaj wyłącznie wartości tempa opublikowanych w informacjach o serwerze lub ogłoszeniach Oteryn.',
                        <<<'MARKDOWN'
# Autorytatywne wartości tempa

Ten pakiet treści startowych nie zawiera zatwierdzonych liczbowych wartości doświadczenia, umiejętności, magii, łupów, odradzania ani innego tempa rozgrywki.

Sprawdź opublikowaną stronę [Informacje o serwerze](/server-information) i bieżące [ogłoszenia](/announcements). Wartość liczbowa jest autorytatywna tylko wtedy, gdy wydawca Oteryn umieści ją na jednej z tych aktualnych stron publicznych.

Jeżeli wartości brakuje, nie wyprowadzaj domyślnej liczby z innego serwera, przykładowej konfiguracji Canary ani starego klienta. Zapytaj [Wsparcie](/support), zanim podejmiesz decyzję w grze zależną od tej wartości.
MARKDOWN,
                    ),
                ],
                [
                    'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
                    'app/Cms/Editorial/EditorialPageKey.php',
                ],
            ),
            new WikiLaunchArticle(
                'system',
                false,
                70,
                ['game-systems'],
                [
                    $this->translation(
                        'en',
                        'Vocations',
                        'vocations',
                        'The current web character-creation choices are Sorcerer, Druid, Paladin, Knight and Monk.',
                        <<<'MARKDOWN'
# Creation-time choices

The current Oteryn web form permits five base vocations:

| Vocation | Creation identifier |
| --- | ---: |
| Sorcerer | 1 |
| Druid | 2 |
| Paladin | 3 |
| Knight | 4 |
| Monk | 9 |

The Platform does not offer `None` or a promoted vocation during character creation. Promotion belongs to game progression.

This source-backed launch article confirms only the available creation choices. It does not assign unapproved spell lists, combat formulas, equipment advice or progression rates to any vocation.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Profesje',
                        'profesje',
                        'Aktualne profesje dostępne przy tworzeniu postaci w witrynie to Sorcerer, Druid, Paladin, Knight i Monk.',
                        <<<'MARKDOWN'
# Wybór podczas tworzenia

Aktualny formularz internetowy Oteryn pozwala wybrać pięć profesji bazowych:

| Profesja | Identyfikator tworzenia |
| --- | ---: |
| Sorcerer | 1 |
| Druid | 2 |
| Paladin | 3 |
| Knight | 4 |
| Monk | 9 |

Platforma nie oferuje wartości `None` ani profesji promowanej podczas tworzenia postaci. Promocja należy do rozwoju w grze.

Ten artykuł startowy potwierdza wyłącznie dostępne wybory podczas tworzenia. Nie przypisuje profesjom niezatwierdzonych list zaklęć, wzorów walki, porad dotyczących wyposażenia ani tempa rozwoju.
MARKDOWN,
                    ),
                ],
                [
                    'docs/architecture/adr/0005-character-creation-product-policy.md',
                    'app/Characters/Actions/CreateCharacter.php',
                    'resources/views/characters/create.blade.php',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                false,
                80,
                ['server-information', 'support'],
                [
                    $this->translation(
                        'en',
                        'PvP and game rules',
                        'pvp-and-game-rules',
                        'Read the current PvP type and complete rules only from Oteryn public sources.',
                        <<<'MARKDOWN'
# Check the current PvP type

The [homepage](/) may show the PvP type supplied for the configured world channel. Because that value is runtime configuration, this Wiki article does not replace it with a fixed label.

# Read the published rules

Use the current [Rules](/rules) page for game, naming, prohibited-software and PvP policy. If a required rule is missing or ambiguous, ask [Support](/support) before acting.

Rules copied from another server, an old announcement or a client file are not Oteryn policy. This launch article intentionally contains no invented penalties, exceptions or combat restrictions.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'PvP i zasady gry',
                        'pvp-i-zasady-gry',
                        'Odczytuj bieżący typ PvP i pełne zasady wyłącznie z publicznych źródeł Oteryn.',
                        <<<'MARKDOWN'
# Sprawdź bieżący typ PvP

[Strona główna](/) może pokazywać typ PvP dostarczony dla skonfigurowanego kanału świata. Ponieważ jest to konfiguracja czasu działania, ten artykuł Wiki nie zastępuje jej stałą etykietą.

# Przeczytaj opublikowane zasady

Bieżąca strona [Zasady](/rules) jest źródłem reguł gry, nazewnictwa, zakazanego oprogramowania i PvP. Jeżeli potrzebnej reguły brakuje lub jest niejednoznaczna, zapytaj [Wsparcie](/support) przed działaniem.

Zasady skopiowane z innego serwera, starego ogłoszenia lub pliku klienta nie są polityką Oteryn. Ten artykuł startowy celowo nie zawiera zmyślonych kar, wyjątków ani ograniczeń walki.
MARKDOWN,
                    ),
                ],
                [
                    'app/PublicPortal/HomePageQuery.php',
                    'app/Cms/Editorial/EditorialPageKey.php',
                    'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
                ],
            ),
            new WikiLaunchArticle(
                'guide',
                true,
                90,
                ['support'],
                [
                    $this->translation(
                        'en',
                        'Account security and MFA',
                        'account-security-and-mfa',
                        'Protect the Platform identity with a unique password, confirmed MFA and private recovery codes.',
                        <<<'MARKDOWN'
# Protect the account

- Use a unique password and keep it out of chat, bug reports and screenshots.
- Sign in only through the Oteryn website and approved client instructions.
- Open [Account security](/mfa) to enroll time-based one-time-password MFA.
- Complete the confirmation step before treating MFA as enabled.
- Store recovery codes offline and never share them.

MFA protects authentication; it does not grant administrator or Wiki permissions.

Password recovery and security-sensitive changes follow the Platform's session-revocation rules. If you suspect compromise, change the password, review MFA and contact [Support](/support). Never send a password, MFA secret, one-time code or recovery code to support.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Bezpieczeństwo konta i MFA',
                        'bezpieczenstwo-konta-i-mfa',
                        'Chroń tożsamość Platformy unikalnym hasłem, potwierdzonym MFA i prywatnymi kodami odzyskiwania.',
                        <<<'MARKDOWN'
# Chroń konto

- Używaj unikalnego hasła i nie umieszczaj go na czacie, w zgłoszeniach ani na zrzutach ekranu.
- Loguj się wyłącznie przez witrynę Oteryn i zgodnie z zatwierdzonymi instrukcjami klienta.
- Otwórz [Bezpieczeństwo konta](/mfa), aby włączyć MFA z czasowymi kodami jednorazowymi.
- Wykonaj krok potwierdzenia, zanim uznasz MFA za aktywne.
- Przechowuj kody odzyskiwania offline i nigdy ich nie udostępniaj.

MFA chroni uwierzytelnianie; nie nadaje uprawnień administratora ani Wiki.

Odzyskiwanie hasła i zmiany dotyczące bezpieczeństwa podlegają zasadom unieważniania sesji Platformy. Jeżeli podejrzewasz przejęcie konta, zmień hasło, sprawdź MFA i skontaktuj się ze [Wsparciem](/support). Nigdy nie wysyłaj do wsparcia hasła, sekretu MFA, kodu jednorazowego ani kodu odzyskiwania.
MARKDOWN,
                    ),
                ],
                [
                    'docs/architecture/SECURITY_ARCHITECTURE.md',
                    'app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php',
                    'app/Identity/Mfa/MfaRecoveryCodes.php',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                false,
                100,
                ['support'],
                [
                    $this->translation(
                        'en',
                        'Frequently asked questions',
                        'frequently-asked-questions',
                        'Answers to common questions using current Oteryn public and account states.',
                        <<<'MARKDOWN'
# Frequently asked questions

## Where should I download the client?

Only from the [Download Center](/download), using its published checksum.

## Why can I not create a character?

Check [your account overview](/account). Character creation is available only when the bound game account is ready. Name conflicts and the active-character limit also fail explicitly.

## Where are current rates and rules?

Use [Server Information](/server-information), [Rules](/rules) and current [announcements](/announcements). The Wiki does not fill missing values with guesses.

## Is Wiki content player-editable?

No. Public Wiki pages are read-only and publication is restricted to trusted Oteryn editors and publishers.

## Where can I get help?

Start at [Support](/support). Never include credentials, MFA codes or recovery codes.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Najczęściej zadawane pytania',
                        'najczesciej-zadawane-pytania',
                        'Odpowiedzi na częste pytania oparte na bieżących publicznych stanach Oteryn i stanie konta.',
                        <<<'MARKDOWN'
# Najczęściej zadawane pytania

## Skąd pobrać klienta?

Wyłącznie z [Centrum pobierania](/download), z użyciem opublikowanej sumy kontrolnej.

## Dlaczego nie mogę utworzyć postaci?

Sprawdź [podsumowanie konta](/account). Tworzenie postaci jest dostępne tylko wtedy, gdy powiązane konto gry jest gotowe. Konflikt nazwy i limit aktywnych postaci również są zgłaszane wprost.

## Gdzie są aktualne wartości tempa i zasady?

Użyj stron [Informacje o serwerze](/server-information), [Zasady](/rules) i bieżących [ogłoszeń](/announcements). Wiki nie uzupełnia brakujących wartości domysłami.

## Czy gracze mogą edytować Wiki?

Nie. Publiczne strony Wiki są tylko do odczytu, a publikacja jest ograniczona do zaufanych redaktorów i wydawców Oteryn.

## Gdzie uzyskać pomoc?

Zacznij od [Wsparcia](/support). Nigdy nie dołączaj danych logowania, kodów MFA ani kodów odzyskiwania.
MARKDOWN,
                    ),
                ],
                [
                    'app/Accounts/ReadModels/AccountOverviewReadModel.php',
                    'app/Characters/Actions/CreateCharacter.php',
                    'docs/architecture/adr/0013-wiki-administration.md',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                false,
                110,
                ['support'],
                [
                    $this->translation(
                        'en',
                        'Known issues',
                        'known-issues',
                        'Check current Oteryn notices and report reproducible problems without assuming an unpublished issue list.',
                        <<<'MARKDOWN'
# Check current notices first

Review current [announcements](/announcements), the [Download Center](/download) release notes and [Support](/support). Those public sources can change faster than a Wiki revision.

This launch package does not declare an empty known-issues list and does not copy unverified reports into public content.

## When a problem is not listed

Record the page or approved client version, the action you took, the exact non-secret error shown and whether retrying changed the result. Then use [Report a bug](/support/report-a-bug).

Do not include passwords, session identifiers, MFA secrets or codes, recovery codes, private keys, personal data or database contents.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Znane problemy',
                        'znane-problemy',
                        'Sprawdź bieżące komunikaty Oteryn i zgłaszaj odtwarzalne problemy bez zakładania nieopublikowanej listy.',
                        <<<'MARKDOWN'
# Najpierw sprawdź bieżące komunikaty

Przejrzyj aktualne [ogłoszenia](/announcements), informacje o wydaniu w [Centrum pobierania](/download) oraz [Wsparcie](/support). Te publiczne źródła mogą zmieniać się szybciej niż rewizja Wiki.

Ten pakiet startowy nie ogłasza pustej listy znanych problemów i nie kopiuje niezweryfikowanych zgłoszeń do treści publicznej.

## Gdy problemu nie ma na liście

Zapisz stronę lub wersję zatwierdzonego klienta, wykonaną czynność, dokładny komunikat bez danych wrażliwych oraz informację, czy ponowienie zmieniło wynik. Następnie użyj strony [Zgłoś błąd](/support/report-a-bug).

Nie dołączaj haseł, identyfikatorów sesji, sekretów ani kodów MFA, kodów odzyskiwania, kluczy prywatnych, danych osobowych ani zawartości bazy danych.
MARKDOWN,
                    ),
                ],
                [
                    'app/Cms/Editorial/EditorialPageKey.php',
                    'docs/architecture/SECURITY_ARCHITECTURE.md',
                    'resources/views/downloads/index.blade.php',
                ],
            ),
            new WikiLaunchArticle(
                'reference',
                false,
                120,
                ['support'],
                [
                    $this->translation(
                        'en',
                        'Discord and support',
                        'discord-and-support',
                        'Use only support and community destinations published by Oteryn.',
                        <<<'MARKDOWN'
# Official support

Start at the current [Support](/support) page. Use only contact or community links published there or in an Oteryn announcement.

This launch-content package does not embed a Discord invitation because no approved destination is established in its repository sources. Do not infer one from another server, a search result or a message from an unknown account.

Before contacting support, check [Known issues](/en/wiki/known-issues) and collect a concise, reproducible description. Never send credentials, MFA material, private keys or personal data that is not required for the report.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Discord i wsparcie',
                        'discord-i-wsparcie',
                        'Używaj wyłącznie kanałów wsparcia i społeczności opublikowanych przez Oteryn.',
                        <<<'MARKDOWN'
# Oficjalne wsparcie

Zacznij od bieżącej strony [Wsparcie](/support). Używaj wyłącznie danych kontaktowych i odnośników społeczności opublikowanych tam albo w ogłoszeniu Oteryn.

Ten pakiet treści startowych nie zawiera zaproszenia Discord, ponieważ w jego źródłach repozytorium nie ustalono zatwierdzonego adresu. Nie wyprowadzaj go z innego serwera, wyniku wyszukiwania ani wiadomości od nieznanego konta.

Przed kontaktem ze wsparciem sprawdź [Znane problemy](/pl/wiki/znane-problemy) i przygotuj zwięzły, odtwarzalny opis. Nigdy nie wysyłaj danych logowania, materiałów MFA, kluczy prywatnych ani danych osobowych, które nie są potrzebne do zgłoszenia.
MARKDOWN,
                    ),
                ],
                [
                    'app/Cms/Editorial/EditorialPageKey.php',
                    'routes/modules/support.php',
                    'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
                ],
            ),
            new WikiLaunchArticle(
                'guide',
                false,
                130,
                ['support'],
                [
                    $this->translation(
                        'en',
                        'Report a bug',
                        'report-a-bug',
                        'Submit a bounded, reproducible report through the official support route without sensitive data.',
                        <<<'MARKDOWN'
# Prepare a useful report

1. Check [Known issues](/en/wiki/known-issues).
2. Record the affected page or approved client version.
3. Write the smallest sequence of steps that reproduces the problem.
4. Include the exact non-secret error and the expected result.
5. Note whether the problem is consistent or intermittent.

Open [Report a bug](/support/report-a-bug) and follow the current instructions shown there.

## Keep sensitive data out

Do not submit passwords, password-reset links, cookies, session identifiers, MFA secrets or codes, recovery codes, access tokens, private keys, database dumps or unnecessary personal data. A screenshot must be reviewed and redacted before it is shared.
MARKDOWN,
                    ),
                    $this->translation(
                        'pl',
                        'Zgłoś błąd',
                        'zglos-blad',
                        'Prześlij ograniczone, odtwarzalne zgłoszenie przez oficjalną stronę wsparcia bez danych wrażliwych.',
                        <<<'MARKDOWN'
# Przygotuj użyteczne zgłoszenie

1. Sprawdź [Znane problemy](/pl/wiki/znane-problemy).
2. Zapisz stronę albo wersję zatwierdzonego klienta, której dotyczy problem.
3. Opisz najkrótszą sekwencję kroków odtwarzających problem.
4. Dołącz dokładny komunikat bez danych wrażliwych oraz oczekiwany wynik.
5. Zaznacz, czy problem występuje zawsze, czy sporadycznie.

Otwórz stronę [Zgłoś błąd](/support/report-a-bug) i wykonaj bieżące instrukcje.

## Nie dołączaj danych wrażliwych

Nie wysyłaj haseł, odnośników resetowania hasła, plików cookie, identyfikatorów sesji, sekretów ani kodów MFA, kodów odzyskiwania, tokenów dostępu, kluczy prywatnych, zrzutów bazy danych ani zbędnych danych osobowych. Przed udostępnieniem zrzutu ekranu sprawdź go i usuń dane wrażliwe.
MARKDOWN,
                    ),
                ],
                [
                    'app/Cms/Editorial/EditorialPageKey.php',
                    'routes/modules/support.php',
                    'docs/architecture/SECURITY_ARCHITECTURE.md',
                ],
            ),
        ];
    }

    private function category(
        string $key,
        int $sortOrder,
        string $englishName,
        string $englishSlug,
        string $englishDescription,
        string $polishName,
        string $polishSlug,
        string $polishDescription,
    ): WikiLaunchCategory {
        return new WikiLaunchCategory($key, $sortOrder, [
            new WikiCategoryTranslationInput(
                'en',
                $englishName,
                $englishSlug,
                $englishDescription,
            ),
            new WikiCategoryTranslationInput(
                'pl',
                $polishName,
                $polishSlug,
                $polishDescription,
            ),
        ]);
    }

    private function translation(
        string $locale,
        string $title,
        string $slug,
        string $summary,
        string $sourceMarkdown,
    ): WikiLaunchTranslation {
        return new WikiLaunchTranslation($locale, $title, $slug, $summary, $sourceMarkdown);
    }
}
