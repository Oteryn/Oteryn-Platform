# Oteryn Portal w Ruście – projekt

> **Status: PROJEKT, rewizja 3** (2026-11-05). Zastępuje rewizje 1–2 (plan migracji falami), na polecenie właściciela.
>
> Ten dokument opisuje nową platformę Oteryn w Ruście, budowaną od zera według obecnych potrzeb graczy, zespołu i gry. Obecny portal Laravel służy tylko jako źródło wiedzy o potrzebach i kontraktach. Nie jest specyfikacją i nie jest przenoszony.
>
> Kod powstaje w `portal-rs/` w tym repozytorium. Pisze go Codex, zadaniami z sekcji 10, wysyłanymi przez koordynatora.
>
> Dokument nie autoryzuje wdrożenia, zmian w kontraktach gry ani nowych jobów CI. Każda z tych rzeczy wymaga osobnej zgody właściciela.

## 1. Co budujemy

Jedną aplikację, `oteryn-portal`, która obsługuje:

- **graczy** w przeglądarce: stronę, konto, sklep, społeczność, wiki i pomoc;
- **klienta gry i launcher**: logowanie, postacie, wejście do gry, newsy, status, saldo i aktualizacje;
- **serwer gry**: admission, projekcje, komendy postaci, premium, sklep i restore;
- **zespół**: panel admina, GM-a, supportu, redakcji i operacji.

Zasady projektu:

- **Jedna baza, jedna binarka.** PostgreSQL platformy i jedna binarka z rolami: `serve`, `worker`, `migrate`, `admin-cli`. Brak mikroserwisów.
- **Gra jest osobna.** Brak wspólnej bazy z grą. Wymiana danych idzie tylko przez kontrakty HTTP z mTLS. Kontrakty, które gra już implementuje, zostają bajt w bajt (sekcja 6).
- **Mało JS.** SSR w askama i htmx. Bez SPA, bez npm, bez bundlera.
- **Domyślnie wyłączone.** Każda funkcja dotykająca gry lub pieniędzy ma flagę.
- **Uruchomienie jednorazowe.** Gdy nowy portal jest gotowy, importujemy konta (hashe argon2id działają bez resetu haseł) i przełączamy domenę. Bez okresu współistnienia.

## 2. Potrzeby → funkcje

| Kto | Potrzeba | Funkcja |
| --- | --- | --- |
| Nowy gracz | Zrozumieć grę i zacząć | Strona główna, „Jak zacząć”, rejestracja, pobieranie klienta |
| Gracz | Bezpieczne konto | Logowanie, 2FA (TOTP), passkeys, kody odzyskiwania, lista sesji i urządzeń, zmiana e-maila i hasła, eksport i usunięcie danych |
| Gracz w kliencie | Logowanie bez hasła w kliencie | OAuth PKCE z przeglądarki systemowej, sesje urządzeń, tickety do gry |
| Gracz | Postacie | Lista postaci, tworzenie (intencja → gra), profil publiczny i jego ustawienia |
| Gracz | Wiedza | Wiki z katalogu gry, artykuły, wyszukiwarka, Atlas |
| Gracz | Społeczność | Rankingi, profile, gildie, domy, zgony, online, status światów |
| Gracz | Zakupy | Saldo coinów, doładowanie, sklep, premium, historia, transfer coinów |
| Gracz | Pomoc | Zgłoszenia, odwołania od kar, FAQ |
| Gracz | Być na bieżąco | Newsy, wydarzenia, ogłoszenia, powiadomienia (e-mail i w kliencie) |
| Redakcja | Publikować | Newsy, strony, wydarzenia, media i wiki z wersjami i przeglądem |
| GM / moderator | Pilnować porządku | Kary wykonywane w grze, historia gracza, notatki, ogłoszenia w grze |
| Support | Pomagać | Kolejka zgłoszeń, widok konta bez danych wrażliwych |
| Admin | Kontrola | Role i uprawnienia, audyt, flagi, cennik sklepu, zwroty |
| Operacje | Działający system | Status światów, wydania klienta (TUF), zdrowie systemu, kolejki |

Poza pierwszym wydaniem zostają: bazar postaci i przedmiotów, analizator sesji, zarządzanie węzłami gry, obserwacja GM-a i aplikacja mobilna.

## 3. Architektura

### 3.1 Stos

| Warstwa | Wybór |
| --- | --- |
| Język | Rust (stable, edycja 2024) |
| HTTP | axum, tower, tower-http (kompresja, limity, nagłówki bezpieczeństwa, trace) |
| TLS i mTLS | rustls; mTLS dla `/internal/v1` terminowany w procesie |
| Szablony | askama (sprawdzane przy kompilacji), htmx, ręczny CSS z tokenami |
| Baza | PostgreSQL, sqlx (zapytania sprawdzane offline, `sqlx::migrate!`) |
| Hasła | argon2 (argon2id, zgodny z obecnymi hashami) |
| 2FA i passkeys | totp-rs, webauthn-rs |
| Tokeny | ed25519-dalek (granty admission), losowe tokeny nieprzezroczyste przechowywane jako SHA-256 |
| i18n | fluent (pl, en) |
| Markdown | pulldown-cmark i ammonia (sanityzacja) |
| E-mail | lettre (SMTP) |
| Obserwowalność | tracing, metryki Prometheus, OpenTelemetry opcjonalnie |
| Testy | `cargo test`, testy integracyjne na prawdziwym PostgreSQL, testy kontraktów na fixtures z `docs/contracts/` |

Lint: `unsafe_code = forbid`; `unwrap_used`, `expect_used` i `panic` mają poziom `deny` poza testami; `cargo deny` sprawdza licencje i podatności.

### 3.2 Układ kodu

```
portal-rs/
  Cargo.toml                 workspace, wspólne wersje i lints
  bin/oteryn-portal/         main, role: serve | worker | migrate | admin-cli
  crates/
    core/                    konfiguracja, błędy, identyfikatory (UUIDv7), czas, flagi
    db/                      pula, migracje, transakcje, outbox
    web/                     router, layout, i18n, CSRF, sesje, nagłówki, strony błędów
    identity/                konta, hasła, 2FA, passkeys, odzyskiwanie, e-mail, RODO
    gameauth/                OAuth PKCE, sesje urządzeń, tickety, admission, granty Ed25519
    characters/              projekcja postaci konta, intencje tworzenia, profile
    content/                 newsy, strony, wydarzenia, media, ogłoszenia
    wiki/                    artykuły, wersje, przegląd, import katalogu gry, wyszukiwanie
    community/               projekcje publiczne: rankingi, profile, gildie, domy, zgony, online, światy
    wallet/                  ledger coinów, transfer, blokady, idempotencja
    store/                   cennik, zakup, dostawa (wire sklepu), historia
    payments/                interfejs dostawcy, webhooki (provider za traitem)
    support/                 zgłoszenia, odwołania, kary (komendy do gry)
    admin/                   RBAC, audyt, panel
    downloads/               kanały TUF, metadane wydań klienta
    notify/                  powiadomienia e-mail i w kliencie
    internal-api/            /internal/v1: guardy mTLS, kodeki wire, handlery
    player-api/              /api/v1 dla klienta gry i launchera
  templates/                 askama
  static/                    css, htmx, ikony
  migrations/                sqlx
  tests/                     testy integracyjne i kontraktowe
```

Każdy moduł domenowy ma typy i reguły bez I/O, repozytorium (sqlx) i handlery HTTP w swoim crate'cie. Crate `web` składa router z modułów. Moduły nie zależą od siebie nawzajem, poza `core`, `db` i jawnie nazwanymi interfejsami: `identity::AccountId`, `wallet::Ledger`. `payments` zależy od `wallet`, nigdy odwrotnie.

### 3.3 Hosty i role

| Host | Co obsługuje | Dostęp |
| --- | --- | --- |
| `oteryn.com` | Strona, konto, sklep, `/api/v1` | Publiczny, za Cloudflare |
| `internal` (osobny port) | `/internal/v1` | Tylko mTLS, allowlista DN, sieć prywatna |
| `admin` (osobny host) | Panel zespołu | Cloudflare Access i obowiązkowe MFA |

Worker obsługuje:

- dostarczanie outboxa;
- e-maile;
- import katalogu;
- odświeżanie projekcji;
- rekoncyliacje;
- sprzątanie sesji i tokenów.

## 4. Model danych

Wszystkie identyfikatory to UUIDv7. Kwoty to `bigint` w najmniejszej jednostce. Czas to `timestamptz` w UTC. Każda tabela z mutacjami użytkownika ma `created_at` i `updated_at`.

| Moduł | Tabele |
| --- | --- |
| identity | `accounts` (id, email, email_verified_at, password_hash, status, locale), `account_mfa_totp`, `account_passkeys`, `account_recovery_codes`, `account_email_tokens`, `account_password_resets`, `web_sessions` (hash tokenu, konto, urządzenie, ip, wygasa, sudo_until), `security_events` |
| gameauth | `oauth_clients`, `oauth_authorization_codes`, `oauth_tokens` (access i refresh jako hash, rodzina rotacji), `device_sessions`, `game_login_tickets` (jednorazowe, krótkie), `native_admissions`, `native_evidence`, `signing_keys` (Ed25519, rotacja), `scope_assignments` |
| characters | `account_characters` (projekcja LCFA z watermarkiem), `character_create_intents` (outbox do gry), `character_profiles` (preferencje publiczne) |
| content | `news`, `pages`, `events`, `announcements`, `media` (z wariantami), `translations` |
| wiki | `wiki_articles`, `wiki_revisions`, `wiki_reviews`, `catalog_imports` (paczka gry z wersją i hashem), `catalog_entries` |
| community | `public_snapshots` (rodzina, wersja, watermark, `jsonb`), `world_status` |
| wallet | `wallet_accounts`, `wallet_entries` (niezmienne, saldo jako suma z blokadą wiersza konta), `wallet_transfers` |
| store | `store_price_list_imports`, `store_active_catalog`, `store_offers`, `store_purchase_requests`, `store_orders`, `store_order_lines`, `store_fulfilment_units`, `store_operations` (outbox DELIVER), `store_receipts`, `store_reconcile_state` |
| payments | `payment_intents`, `payment_events` (idempotencja po id dostawcy), `invoices` |
| support | `tickets`, `ticket_messages`, `appeals`, `sanctions` (z komendą do gry i potwierdzeniem) |
| admin | `roles`, `role_permissions`, `account_roles`, `audit_log` (tylko dopisywanie), `feature_flags`, `four_eyes_requests` |
| downloads | `client_channels`, `client_releases` |
| notify | `notifications`, `email_outbox` |
| db | `outbox` (wspólny: typ, klucz idempotencji, payload, próby, następna próba) |

Schemat sklepu odpowiada kontraktowi wire sklepu (`docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`) i decyzjom z 2026-11-05: jedna moneta transferowalna, PREMIUM_TIME włączony, COSMETIC wyłączony.

## 5. Powierzchnie HTTP

### 5.1 Web (SSR)

| Obszar | Trasy |
| --- | --- |
| Publiczne | `/`, `/start`, `/news`, `/news/{slug}`, `/events`, `/pages/{slug}`, `/download`, `/worlds`, `/rankings/{kind}`, `/characters/{name}`, `/guilds/{name}`, `/houses`, `/deaths`, `/online`, `/wiki`, `/wiki/{slug}`, `/wiki/search`, `/atlas` |
| Konto | `/register`, `/login`, `/logout`, `/verify-email`, `/forgot-password`, `/reset-password`, `/account`, `/account/security` (hasło, 2FA, passkeys, kody), `/account/sessions`, `/account/devices`, `/account/characters`, `/account/privacy` (eksport, usunięcie) |
| Sklep | `/shop`, `/shop/coins` (doładowanie), `/shop/history`, `/shop/transfer` |
| Pomoc | `/support`, `/support/tickets/{id}`, `/support/appeals` |
| Autoryzacja klienta | `/oauth/authorize`, `/oauth/consent`, `/link` (kod urządzenia) |

Prefiks języka: `/pl/...` i `/en/...`, z przekierowaniem z `/` według `Accept-Language` i cookie.

### 5.2 Player API `/api/v1` (klient gry i launcher)

| Endpoint | Cel |
| --- | --- |
| `POST /oauth/token` | Wymiana kodu PKCE i refresh z rotacją |
| `GET /api/v1/client/directory` | Katalog usług dla klienta (jak `fetch_directory` w `platform-client`) |
| `POST /api/v1/game-auth/tickets` | Ticket do gry (jak dziś) |
| `GET /api/v1/game-auth/native-characters` | Postacie konta (jak dziś) |
| `POST /api/v1/game-auth/device-sessions` oraz `.../native-characters`, `.../tickets`, `.../revoke` | Sesje urządzeń (jak dziś) |
| `GET /api/v1/account/summary` | Saldo, premium, powiadomienia (nowe) |
| `GET /api/v1/feed` | Newsy, wydarzenia, ogłoszenia w języku gracza (nowe) |
| `GET /api/v1/worlds` | Status światów (nowe) |

Trasy oznaczone „jak dziś” zachowują obecne kształty żądań i odpowiedzi. Nowe trasy klient odkrywa przez `client/directory`. Kontrakt nowych tras zapisujemy razem z grą (zadanie G-1 w sekcji 10).

### 5.3 Game API `/internal/v1` (serwer gry i gateway; bajt w bajt)

| Grupa | Trasy |
| --- | --- |
| game-auth | `native-admissions`, `native-evidence`, `native-runtime-status`, `native-scope-assignments`, `native-scope-revocations`, `native-account-characters`, `native-account-characters/watermark`, `character-bootstrap-intents/read` |
| character-authority | komendy postaci (lista PENDING, odczyt, receipt), notatka restore |
| products-entitlements | `premium-snapshots/read`, `loyalty-snapshots/read`, `store/balance/read`, `store/history/read`, `store/purchases`, `store/operations/{pending,read,receipt,reconcile}` |
| game-public | snapshoty rodzin projekcji i watermark |

Każda trasa ma:

- guard mTLS z allowlistą tożsamości per grupa;
- TLS 1.3;
- limit rozmiaru;
- kanoniczny JSON;
- porównania w stałym czasie;
- odpowiedzi 503 i 401 dokładnie jak w kontrakcie.

Testy kontraktowe czytają fixtures z `docs/contracts/fixtures/` i wymagają identycznych bajtów odpowiedzi.

### 5.4 Panel zespołu

Panel ma sekcje: treści, wiki, gracze (wyszukiwanie, konto, postacie, kary, notatki), support, sklep (import i aktywacja cennika, zamówienia, zwroty), role, audyt, flagi, wydania klienta i zdrowie systemu.

Akcje krytyczne wymagają drugiej osoby (zasada czterech oczu): kary stałe, zwroty, aktywacja cennika, nadanie roli admina.

## 6. Integracja z grą

| Przepływ | Kontrakt | Stan po stronie gry |
| --- | --- | --- |
| Logowanie: ticket → gateway → admission | V2_NATIVE_GATEWAY_LOGIN, granty Ed25519 | Tylko testing/preprod (D721); FND-04B-AMEND-1 PROPOSED |
| Postacie konta | LCFA (`native-account-characters`) | Producent zaimplementowany, niewłączony |
| Tworzenie postaci | GAME-CHAR-CMD-0 | Zaakceptowany, niezaimplementowany |
| Premium i loyalty | V2_PREMIUM_TIME_SNAPSHOT, V2_LOYALTY_SNAPSHOT | Premium: endpoint jest; loyalty: proposed |
| Sklep | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY | PROPOSED, domyślnie wyłączony |
| Dane publiczne | GAME-PUBLIC-PROJ-0 | Kandydat, niezaakceptowany |
| Status światów | V2_RUNTIME_STATUS_PROJECTION | PARTIAL |
| Kary | V2_GAME_ENFORCEMENT_COMMAND | Wykonanie w grze brak |
| Restore | V2_GAME_RESTORE_RECONCILIATION | Domyślnie wyłączony |
| Katalog i wiki | OTERYN-GAME-PLATFORM-CATALOG-V1 | LOCKED |
| Klient | `platform-client` (PKCE, tickety, `gateway_login`, `owner_characters`, `fetch_directory`) | Działa po stronie klienta |

Zasady:

- Portal implementuje wszystkie przepływy, ale każdy jest za flagą. Włączamy go, gdy gra ma swoją stronę.
- Funkcja bez danych z gry pokazuje „niedostępne” z czasem ostatniej aktualizacji, nigdy zmyślonych wartości.
- Każda mutacja przez granicę gry jest idempotentna, ma klucz operacji, outbox i rekoncyliację. Timeout nie oznacza sukcesu ani porażki.
- Gateway logowania (dziś w Go, `services/game-gateway`) zostaje bez zmian. Portal implementuje jego endpointy `/internal/v1` bajt w bajt.

## 7. Bezpieczeństwo

- Hasła: argon2id z rehashem; sprawdzanie haseł w wyciekach (k-anonymity); rate limity logowania, rejestracji i odzyskiwania; ochrona przed enumeracją kont.
- Sesje:
  - cookie `__Host-`, `Secure`, `HttpOnly`, `SameSite=Lax`;
  - rotacja przy logowaniu, zmianie hasła, 2FA i e-maila;
  - tryb sudo dla akcji wrażliwych.
- CSRF: token synchronizowany dla każdej mutacji w web.
- Tokeny API: nieprzezroczyste, przechowywane jako SHA-256, z zakresami. Refresh rotowany; ponowne użycie unieważnia całą rodzinę.
- Waluta: transakcja z `SELECT ... FOR UPDATE` na koncie portfela, brak ujemnych sald, klucz idempotencji dla każdej operacji.
- Webhooki płatności: weryfikacja podpisu, idempotencja po id zdarzenia, ochrona przed powtórzeniem.
- Nagłówki: CSP bez `unsafe-inline`, HSTS, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`.
- Uploady: limit rozmiaru, sprawdzanie typu po zawartości, przetwarzanie obrazów, brak wykonywania.
- Audyt: każda akcja zespołu trafia do `audit_log` (kto, co, kiedy, przed i po).
- Sekrety: tylko zmienne środowiska lub pliki na hoście; nic w repozytorium.

## 8. Wygląd i jakość

- Projekt graficzny od nowa: ciemny motyw z akcentem marki, czytelna typografia, komponenty w jednym pliku tokenów CSS.
- Pełna obsługa telefonu i WCAG 2.2 AA.
- Budżety: LCP poniżej 2 s na 4G, JS poza mapą poniżej 30 KB, p95 odpowiedzi poniżej 150 ms dla stron publicznych.
- SEO: sitemap, `hreflang`, kanoniczne adresy, dane strukturalne.

## 9. Uruchomienie

1. Preprod na `oteryn-vps-2`: PostgreSQL, portal, mTLS z gatewayem i węzłem testowym.
2. Narzędzie `admin-cli import-legacy` jednorazowo czyta eksport z obecnej bazy. Kopiuje: konta (z AccountId i hashami), role, treści, wiki, postacie z projekcji, saldo portfela i historię.
3. Weryfikacja importu: liczby i sumy kontrolne, logowanie próbnych kont, zgodność sald.
4. Przełączenie domeny i gatewaya na nowy portal; Laravel zostaje wyłączony, a jego kod jest usuwany z repozytorium osobnym PR-em.

Każdy krok wymaga zgody właściciela („deploy”).

## 10. Zadania dla Codexa

Każde zadanie to jedna mechanika i effort medium. Ma dozwolone ścieżki, kryterium akceptacji (`cargo test` i `cargo clippy` w `portal-rs/`) oraz zależności. Zadania wysyła koordynator. Zadania bez zależności między sobą idą równolegle.

| ID | Zadanie | Zależy od | Ścieżki |
| --- | --- | --- | --- |
| T00 | Workspace, lints, `core` (konfiguracja, błędy, UUIDv7), `db` (pula, migracje), binarka z rolami, `/healthz` | – | `portal-rs/` (cały szkielet) |
| T01 | `web`: router, layout askama, statyczne pliki, nagłówki bezpieczeństwa, strony błędów, i18n fluent pl/en | T00 | `crates/web`, `templates/`, `static/` |
| T02 | Sesje web (tabela, cookie `__Host-`, rotacja) i CSRF | T01 | `crates/web/src/session*`, migracja |
| T03 | Rejestracja i weryfikacja e-maila | T02 | `crates/identity` (rejestracja), szablony |
| T04 | Logowanie i wylogowanie, argon2id z rehashem, rate limit | T02 | `crates/identity` (login) |
| T05 | Reset hasła | T04 | `crates/identity` (reset) |
| T06 | 2FA TOTP i kody odzyskiwania | T04 | `crates/identity` (mfa) |
| T07 | Passkeys (WebAuthn) | T04 | `crates/identity` (passkeys) |
| T08 | Panel konta: sesje, urządzenia, zmiana e-maila i hasła, tryb sudo | T04 | `crates/identity` (account) |
| T09 | RODO: eksport i usunięcie konta | T08 | `crates/identity` (privacy) |
| T10 | RBAC, audyt, szkielet panelu admina z obowiązkowym MFA | T06 | `crates/admin` |
| T11 | Treści: newsy, strony, wydarzenia, ogłoszenia; CRUD w adminie; strony publiczne | T10 | `crates/content` |
| T12 | Media: upload, warianty obrazów, walidacja | T11 | `crates/content/src/media*` |
| T13 | Wiki: artykuły, wersje, przegląd, markdown i sanityzacja, wyszukiwanie (PG full text) | T10 | `crates/wiki` |
| T14 | Import paczki katalogu gry do wiki (kontrakt CATALOG-V1) | T13 | `crates/wiki/src/catalog*` |
| T15 | Guard mTLS `/internal/v1`, kanoniczny JSON i harness testów kontraktowych na fixtures | T00 | `crates/internal-api` (wspólne) |
| T16 | OAuth PKCE (`/oauth/authorize`, `/oauth/token`, consent, refresh z rotacją) | T04 | `crates/gameauth/src/oauth*` |
| T17 | Tickety do gry i `GET native-characters` (Player API, jak dziś) | T16, T18 | `crates/gameauth/src/tickets*`, `crates/player-api` |
| T18 | `native-account-characters` i watermark (LCFA, `/internal/v1`) | T15 | `crates/characters` |
| T19 | Admission, evidence, runtime-status, granty Ed25519, rotacja kluczy | T15 | `crates/gameauth/src/admission*` |
| T20 | Scope assignments i revocations | T19 | `crates/gameauth/src/scopes*` |
| T21 | Sesje urządzeń (enroll, characters, tickets, revoke) | T17 | `crates/gameauth/src/devices*` |
| T22 | Komendy postaci i character-bootstrap-intents, tworzenie postaci z konta | T18 | `crates/characters/src/commands*` |
| T23 | Restore notice i rekoncyliacja | T22 | `crates/characters/src/restore*` |
| T24 | Projekcje publiczne: snapshoty i watermark (`/internal/v1/game-public`) i strony społeczności | T15, T01 | `crates/community` |
| T25 | Status światów: strona i `GET /api/v1/worlds` | T24 | `crates/community/src/worlds*` |
| T26 | Ledger portfela, saldo, historia, transfer coinów (blokady, idempotencja) | T04 | `crates/wallet` |
| T27 | Premium i loyalty snapshoty (`/internal/v1`) | T15, T26 | `crates/store/src/snapshots*` |
| T28 | Import i aktywacja cennika sklepu (S-5, `catalog_revision` = SHA-256) | T10 | `crates/store/src/catalog*` |
| T29 | Zakup, saldo i historia przez wire sklepu | T26, T28, T15 | `crates/store/src/purchase*` |
| T30 | Dostawa: operacje pending, read, receipt, reconcile | T29 | `crates/store/src/delivery*` |
| T31 | Płatności: trait dostawcy, provider testowy, webhooki z idempotencją, doładowanie coinów | T26 | `crates/payments` |
| T32 | Sklep web: oferty, doładowanie, historia, transfer | T29, T31 | `crates/store/src/web*`, szablony |
| T33 | Support: zgłoszenia, wiadomości, odwołania | T10 | `crates/support` |
| T34 | Kary: komenda do gry (enforcement), historia, cztery oczy | T33, T15 | `crates/support/src/sanctions*` |
| T35 | Pobieranie i kanały TUF | T11 | `crates/downloads` |
| T36 | Powiadomienia e-mail (outbox) i `GET /api/v1/account/summary`, `GET /api/v1/feed`, `client/directory` | T11, T26 | `crates/notify`, `crates/player-api` |
| T37 | Worker: outbox, ponowienia, sprzątanie | T00 | `bin/oteryn-portal/src/worker*`, `crates/db/src/outbox*` |
| T38 | `admin-cli import-legacy` z eksportu obecnej bazy | T03–T34 | `bin/oteryn-portal/src/import*` |
| T39 | Projekt graficzny: tokeny CSS, komponenty, strona główna, „Jak zacząć” | T01 | `static/`, `templates/` |
| G-1 | (gra) Kontrakt nowych tras Player API i `client/directory` | – | Oteryn-Game, przez koordynatora |

Kolejność startu:

1. Najpierw T00, bo daje wspólny szkielet. Ja mogę go przygotować i sprawdzić lokalnie, żeby Codex startował od kompilującej się bazy.
2. Potem jednocześnie: T01, T15, T37 i T39.
3. Potem gałęzie konta (T02–T10), gry (T16–T25) i handlu (T26–T32) równolegle.

Pozostałe warunki:

- **CI.** Testy Rusta w CI wymagają nowego joba w `.github/workflows`, a to potrzebuje zgody właściciela. Do tego czasu każde zadanie dołącza wynik `cargo test` i `cargo clippy` w PR.
- **Zakres plików.** Zadania mają rozłączne ścieżki. Wspólne pliki (`Cargo.toml` workspace, router w `web`, migracje) zmienia się tylko przez dopisanie linii, a konflikty rozwiązuje koordynator przy merge.

## 11. Decyzje właściciela

| # | Decyzja | Domyślnie |
| --- | --- | --- |
| D1 | Zakres pierwszego wydania | Sekcja 2 bez bazaru, analizatora, węzłów i obserwacji GM |
| D2 | Nowy job CI dla `portal-rs/` | Potrzebna zgoda |
| D3 | Dostawca płatności | Provider testowy do czasu wyboru (np. Stripe) |
| D4 | Wygląd | Nowy projekt graficzny (T39) |
| D5 | Gateway logowania | Zostaje w Go; portal zachowuje jego kontrakt |
| D6 | Import danych | Konta, role, treści, wiki, portfel; reszta zaczyna od zera |

## 12. Źródła

- Obecny portal: trasy `routes/internal.php`, `routes/api.php`, `routes/modules/*`; moduły `app/*`.
- Kontrakty: `docs/contracts/`, w tym wire sklepu i decyzje z 2026-11-05.
- Gra: Oteryn-Game `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`, `crates/platform-client`, `crates/platform-contracts`, `crates/protocol-oteryn`.
- Szkic #1643: źródło pomysłów dla rozszerzeń (węzły, Atlas, obserwacja GM).
