# Oteryn Portal w Ruście – projekt

> **Status: ZAAKCEPTOWANY przez właściciela (D22, 2026-11-05), rewizja 5.** Zastępuje rewizje 1–4. Rewizja 5 dodaje pełny panel gracza (§5.5) po przeglądzie kompletności względem obecnego portalu i stanu gry, uzupełnia panel zespołu, import i przekierowania oraz dzieli duże zadania na pojedyncze mechaniki. Rewizja 4 dodała przegląd od strony gry, decyzje D2, D7, D8 i odczyt danych świata wprost z gry.
>
> Ten dokument opisuje nową platformę Oteryn w Ruście, budowaną od zera według obecnych potrzeb graczy, zespołu i gry. Obecny portal Laravel służy tylko jako źródło wiedzy o potrzebach i kontraktach. Nie jest specyfikacją i nie jest przenoszony.
>
> Kod powstaje w `portal-rs/` w tym repozytorium. Pisze go Codex, zadaniami z sekcji 10. Zadania Platform wysyła ta sesja przez środowisko Oteryn Platform; zadania po stronie gry (G-1, G-2) idą przez koordynatora.
>
> Dokument nie autoryzuje wdrożenia, zmian w kontraktach gry ani nowych jobów CI. Każda z tych rzeczy wymaga osobnej zgody właściciela.

## 1. Co budujemy

Jedną aplikację, `oteryn-portal`, która obsługuje:

- **graczy** w przeglądarce: stronę, konto, sklep, społeczność, wiki i pomoc;
- **klienta gry i launcher**: logowanie, postacie, wejście do gry, newsy, status, saldo i aktualizacje;
- **serwer gry**: admission, projekcje, komendy postaci, premium, sklep i restore;
- **zespół**: panel admina, GM-a, supportu, redakcji i operacji.

Zasady projektu:

- **Jedna baza, jedna binarka.** PostgreSQL platformy i jedna binarka z rolami: `serve`, `worker`, `gateway`, `migrate`, `admin-cli`. Brak mikroserwisów. Rola `gateway` zastępuje obecny gateway logowania w Go (D5): działa jako osobny proces, bez dostępu do bazy.
- **Gra jest osobna.** Brak wspólnej bazy z grą. Dane na żywo idą tylko przez kontrakty HTTP z mTLS, a kontrakty, które gra już implementuje, zostają bajt w bajt (sekcje 5 i 6). Dane świata portal czyta tylko do odczytu z przypiętego wydania gry (§6.1).
- **Mało JS.** SSR w askama i htmx. Bez SPA, bez npm, bez bundlera.
- **Domyślnie wyłączone.** Każda funkcja dotykająca gry lub pieniędzy ma flagę.
- **Uruchomienie jednorazowe.** Gdy nowy portal jest gotowy, importujemy konta (hashe argon2id działają bez resetu haseł) i przełączamy domenę. Bez okresu współistnienia.

## 2. Potrzeby → funkcje

| Kto | Potrzeba | Funkcja |
| --- | --- | --- |
| Nowy gracz | Zrozumieć grę i zacząć | Strona główna, „Jak zacząć”, rejestracja, pobieranie klienta |
| Gracz | Bezpieczne konto | Logowanie, 2FA (TOTP), passkeys, kody odzyskiwania, klucz odzyskiwania konta, sesje i urządzenia z historią logowań i alertem o nowym urządzeniu, zmiana e-maila z linkiem cofnięcia na stary adres, zmiana hasła, prywatność, eksport danych, usunięcie konta z okresem karencji |
| Gracz w kliencie | Logowanie bez hasła w kliencie | OAuth PKCE z przeglądarki systemowej, sesje urządzeń, tickety do gry |
| Gracz | Postacie | Przegląd konta z postaciami (poziom, profesja, świat), tworzenie, usuwanie z anulowaniem, zmiana nazwy, płci i świata, pełne ustawienia profilu publicznego (§5.5) |
| Gracz | Wiedza o świecie | Wiki generowana z treści świata gry (przedmioty, potwory, NPC, czary, questy, łupy, miejsca) i uzupełniana przez redakcję; Atlas, czyli interaktywna mapa świata z warstwami i znacznikami; mapa dostaw zadań i „gdzie wypada”, bestiariusz i bossowie na mapie, miejsca polowań (§5.9); wyszukiwarka |
| Gracz | Społeczność | Rankingi, profile, gildie, domy, zgony, online, status światów |
| Gracz | Statystyki | Historia postaci (doświadczenie dzień po dniu, czas online, nazwy, światy, gildie), rankingi przyrostu, statystyki gildii i świata, porównania, obserwowanie z powiadomieniami (§5.7) |
| Gracz | Wiedzieć, jak mu idzie | Prywatny dziennik gry: automatyczny analizator polowań z wyceną łupu, dropy i rekordy, przeglądarka przedmiotów („gdzie mam”), postęp i cele, recap zgonów, przypomnienia, historia logowań do gry, mapa łowisk, szukanie drużyny (§5.8) |
| Gracz | Rozrywka poza grą | Pomocnik w domu wysyłany na wyprawy, przepustka sezonowa, kolekcja potworów z walkami, codzienne zagadki, rozgrywki gildii i typer; bez hazardu (§5.10, po pierwszym wydaniu) |
| Gracz | Handel z graczami | Bazar: licytacje postaci, ogłoszenia i licytacje cennych przedmiotów, za coiny albo złoto z gry, z depozytem po obu stronach (§5.6) |
| Gracz | Zakupy | Saldo coinów, doładowanie, sklep, status premium, historia, transfer coinów, płatności i rachunki |
| Gracz | Pomoc | Zgłoszenia z załącznikami, zgłaszanie graczy, historia kar z potwierdzeniem i odwołaniem, FAQ, zgłoszenie błędu |
| Gracz | Być na bieżąco | Newsy, wydarzenia, ogłoszenia, strona „Dziś”, powiadomienia (e-mail i w kliencie) z preferencjami, wybór języka |
| Redakcja | Publikować | Newsy, strony, wydarzenia, media i wiki z wersjami i przeglądem |
| GM / moderator | Pilnować porządku | Kary wykonywane w grze, historia gracza, notatki, ogłoszenia w grze |
| Support | Pomagać | Kolejka zgłoszeń, widok konta bez danych wrażliwych |
| Admin | Kontrola | Role i uprawnienia, audyt, flagi, cennik sklepu, zwroty |
| Operacje | Działający system | Status światów, wydania klienta (TUF), zdrowie systemu, kolejki |

Bazar postaci i przedmiotów jest w zakresie (§5.6, D13). Prywatny dziennik gry z automatycznym analizatorem polowań też jest w zakresie (§5.8, D17). Rozrywka z §5.10 przychodzi zaraz po pierwszym wydaniu (D20). Poza pierwszym wydaniem zostają też: zarządzanie węzłami gry, obserwacja GM-a i aplikacja mobilna. Szablony strony głównej z obecnego portalu zastępuje jedna nowa strona główna (T39b).

## 3. Architektura

### 3.1 Stos

| Warstwa | Wybór |
| --- | --- |
| Język | Rust (stable, edycja 2024) |
| HTTP | axum, tower, tower-http (kompresja, limity, nagłówki bezpieczeństwa, trace) |
| TLS i mTLS | rustls; mTLS dla `/internal/v1` terminowany w procesie |
| Szablony | askama (sprawdzane przy kompilacji), htmx, ręczny CSS z tokenami |
| Baza | PostgreSQL 18.6, ten sam obraz co gra (`postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc`), osobna instancja portalu (D24); sqlx (zapytania sprawdzane offline, `sqlx::migrate!`) |
| Hasła | argon2 (argon2id, zgodny z obecnymi hashami) |
| 2FA i passkeys | totp-rs, webauthn-rs |
| Tokeny | ed25519-dalek (granty admission), losowe tokeny nieprzezroczyste przechowywane jako SHA-256 |
| i18n | fluent (pl, en) |
| Markdown | pulldown-cmark i ammonia (sanityzacja) |
| E-mail | lettre (SMTP) |
| Obserwowalność | tracing, metryki Prometheus, OpenTelemetry opcjonalnie |
| Testy | `cargo test`, testy integracyjne na prawdziwym PostgreSQL, testy kontraktów na wektorach golden z `docs/contracts/` i z Game |

Lint: `unsafe_code = forbid`; `unwrap_used`, `expect_used` i `panic` mają poziom `deny` poza testami; `cargo deny` sprawdza licencje i podatności.

### 3.2 Układ kodu

```
portal-rs/
  Cargo.toml                 workspace, wspólne wersje i lints
  bin/oteryn-portal/         main, role: serve | worker | gateway | migrate | admin-cli
  crates/
    core/                    konfiguracja, błędy, identyfikatory (UUIDv7), czas, flagi
    db/                      pula, migracje, transakcje, outbox
    web/                     router, layout, i18n, CSRF, sesje, nagłówki, strony błędów
    identity/                konta, hasła, 2FA, passkeys, odzyskiwanie, e-mail, RODO
    gameauth/                OAuth PKCE, sesje urządzeń, tickety, admission, granty Ed25519
    characters/              projekcja postaci konta, komendy postaci, profile, przegląd konta
    content/                 newsy, strony, wydarzenia, media, ogłoszenia
    world/                   odczyt danych świata z przypiętego wydania gry (encje, relacje, mapa, sprite'y)
    wiki/                    strony encji generowane z `world`, poradniki redakcji, wyszukiwanie
    atlas/                   kafle mapy z `world`, znaczniki, przeglądarka mapy
    community/               projekcje publiczne: rankingi, profile, gildie, domy, zgony, online, światy
    stats/                   historia i statystyki: dzienne migawki postaci, gildii i świata, przyrosty, wykresy, obserwowanie
    wallet/                  ledger coinów, transfer, blokady, idempotencja
    store/                   cennik, zakup, dostawa (wire sklepu), historia
    playerdata/              prywatny dziennik gry: sesje i analizator polowań, wycena, dropy, przedmioty, postęp, zgony, przypomnienia, mapa łowisk
    hirelings/               pomocnicy w domach: zakup, wyprawy, zawody, komendy do gry
    minigames/               przepustka sezonowa, kolekcja potworów, zagadki, rozgrywki i typer
    bazaar/                  bazar: licytacje postaci, ogłoszenia i licytacje przedmiotów, depozyt, rozliczenie, statystyki
    payments/                interfejs dostawcy, webhooki (provider za traitem)
    support/                 zgłoszenia, zgłoszenia graczy, kary, odwołania
    admin/                   RBAC, audyt, panel
    downloads/               kanały TUF, metadane wydań klienta
    notify/                  powiadomienia e-mail i w kliencie
    internal-api/            /internal/v1: guardy mTLS, kodeki wire, handlery
    player-api/              /api/v1 dla klienta gry i launchera
    gateway/                 gateway logowania: publiczne POST /v1/login, woła native-admissions portalu
  templates/                 askama
  static/                    css, htmx, ikony
  migrations/                sqlx
  tests/                     testy integracyjne i kontraktowe
```

Każdy moduł domenowy ma typy i reguły bez I/O, repozytorium (sqlx) i handlery HTTP w swoim crate'cie. Crate `web` składa router z modułów. Moduły nie zależą od siebie nawzajem, poza `core`, `db` i jawnie nazwanymi interfejsami: `identity::AccountId`, `wallet::Ledger`. `payments` i `bazaar` zależą od `wallet`, nigdy odwrotnie.

### 3.3 Hosty i role

| Host | Co obsługuje | Dostęp |
| --- | --- | --- |
| Domena portalu (base URL klienta) | Strona, konto, sklep, `/oauth`, `/v1/client/directory`, `/api/v1` | Publiczny, za Cloudflare |
| `internal` (osobny port) | `/internal/v1` | Tylko mTLS, allowlista DN, sieć prywatna |
| `admin` (osobny host) | Panel zespołu | Cloudflare Access i obowiązkowe MFA |
| `gateway` (osobny proces i host) | `POST /v1/login`, `/health`, `/ready`, `/version` | Publiczny dla klienta gry; bez poświadczeń bazy, woła portal tokenem usługi |

Worker obsługuje:

- dostarczanie outboxa;
- e-maile;
- import danych świata z nowego wydania gry;
- odświeżanie projekcji;
- rekoncyliacje;
- sprzątanie sesji i tokenów.

## 4. Model danych

Wszystkie identyfikatory to UUIDv7. Kwoty to `bigint` w najmniejszej jednostce. Czas to `timestamptz` w UTC. Każda tabela z mutacjami użytkownika ma `created_at` i `updated_at`.

| Moduł | Tabele |
| --- | --- |
| identity | `accounts` (id, email, email_verified_at, password_hash, status, locale, public_account_association, public_status_visible), `account_mfa_totp`, `account_passkeys`, `account_recovery_codes`, `account_recovery_keys` (hash, utworzony, użyty), `account_email_tokens`, `account_email_changes` (nowy adres, token potwierdzenia, token cofnięcia, cooldown), `account_password_resets`, `account_terminations` (zaplanowane na, anulowane, wykonane), `account_consents` (wersja regulaminu i polityki), `account_notification_prefs`, `web_sessions` (hash tokenu, konto, urządzenie, ip, wygasa, sudo_until), `security_events` (też jako historia logowań gracza) |
| gameauth | `oauth_clients`, `oauth_authorization_codes`, `oauth_tokens` (access i refresh jako hash, rodzina rotacji), `device_sessions`, `game_login_tickets` (jednorazowe, krótkie), `native_admissions`, `native_evidence`, `signing_keys` (Ed25519, rotacja), `scope_assignments` |
| characters | `account_characters` (projekcja LCFA z watermarkiem), `character_commands` (intencje do gry: tworzenie, a po G-3 usunięcie, anulowanie, nazwa, płeć, świat), `character_profiles` (postać główna, ukrycie, komentarz i 7 przełączników: konto, status, gildia, dom, umiejętności, zgony, zabójstwa), `reserved_names` |
| content | `news`, `pages`, `editorial_pages` (zasady, regulamin, prywatność, cookies, informacje o serwerze, zgłoś błąd), `events`, `announcements`, `media` (z wariantami), `translations` |
| world | `world_versions` (commit gry, data, aktywna), `world_entities` (rodzina, klucz, dane `jsonb`, wersja), `world_links` (potwór → łup, NPC → quest, przedmiot → miejsce), `world_visibility` (encje dopuszczone przez admina do czasu flagi w grze) |
| wiki | `wiki_guides`, `wiki_revisions`, `wiki_reviews` (tylko teksty redakcji: poradniki i opisy dopisane do encji) |
| atlas | `atlas_markers` (wyliczane z `world`: miasta, NPC, spawny, questy); kafle mapy jako pliki statyczne per wersja świata |
| community | `public_snapshots` (rodzina, wersja, watermark, `jsonb`), `world_status` |
| stats | `stats_character_daily` (postać, dzień, poziom, doświadczenie, umiejętności, punkty, minuty online; partycje miesięczne), `stats_character_events` (zmiana nazwy, świata, gildii, sprzedaż na bazarze, zgon, awans), `stats_guild_daily`, `stats_world_samples` (gracze online co 5 min), `stats_world_daily`, `stats_watches` |
| wallet | `wallet_accounts`, `wallet_entries` (niezmienne, saldo jako suma z blokadą wiersza konta), `wallet_transfers`, `wallet_adjustments` (korekty zespołu z powodem) |
| premium | `premium_ledger` (nadania, odebrania, zakupy; stan jako suma), `loyalty_state` |
| store | `store_price_list_imports`, `store_active_catalog`, `store_offers`, `store_purchase_requests`, `store_orders`, `store_order_lines`, `store_fulfilment_units`, `store_operations` (outbox DELIVER), `store_receipts`, `store_reconcile_state` |
| playerdata | `pd_sessions` (postać, odcinek, start, koniec, miejsca, doświadczenie, bilans, drużyna), `pd_session_items` (łup i zużycie per przedmiot), `pd_session_combat` (obrażenia zadane i otrzymane per typ i źródło, leczenie), `pd_session_kills`, `pd_drops`, `pd_records`, `pd_item_snapshots` (miejsce, `jsonb`, czas), `pd_trade_history`, `pd_progress`, `pd_goals`, `pd_deaths` (recap), `pd_pvp_events`, `pd_game_logins`, `pd_reminders`, `pd_notes`, `pd_price_overrides`, `pd_share_links` (hash tokenu, wygasa, odwołany), `pd_hunting_places_daily` (tylko agregaty), `pd_party_posts` |
| hirelings | `hirelings` (konto, dom, imię, zawody, poziom), `hireling_expeditions` (cel, start, koniec, stan, receipt z gry), `hireling_commands` (outbox do gry) |
| minigames | `pass_seasons`, `pass_challenges`, `pass_progress`, `pass_claims`; `collection_cards`, `collection_decks`, `collection_battles`, `collection_ratings`; `puzzles_daily`, `puzzles_answers`, `puzzles_streaks`; `league_events`, `league_picks` |
| bazaar | `bazaar_listings` (rodzaj: postać lub przedmiot; forma: licytacja, ogłoszenie, kupno; waluta: coiny lub złoto; cena, „przyjmuję oferty”, stan, koniec), `bazaar_snapshots` (dane z gry, hash, czas), `bazaar_escrow` (stan depozytu w grze), `bazaar_bids` (oferta maksymalna, rezerwa portfela albo blokada złota w grze), `bazaar_offers` (oferty do ogłoszeń), `bazaar_settlements` (kwota, prowizja, komenda do gry, receipt), `bazaar_watches`, `bazaar_saved_searches`, `bazaar_reports` |
| payments | `payment_intents`, `payment_events` (idempotencja po id dostawcy), `invoices` |
| support | `tickets`, `ticket_messages` (też wewnętrzne), `ticket_attachments`, `player_reports`, `sanctions` (ostrzeżenie, ograniczenie, zawieszenie; potwierdzenie gracza), `appeals` |
| admin | `roles`, `role_permissions`, `account_roles`, `audit_log` (tylko dopisywanie), `feature_flags`, `four_eyes_requests`, `player_notes` |
| downloads | `client_channels`, `client_releases` |
| notify | `notifications`, `email_outbox` |
| db | `outbox` (wspólny: typ, klucz idempotencji, payload, próby, następna próba) |

Schemat sklepu odpowiada kontraktowi wire sklepu (`docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`) i decyzjom z 2026-11-05: jedna moneta transferowalna, PREMIUM_TIME włączony, COSMETIC wyłączony.

## 5. Powierzchnie HTTP

### 5.1 Web (SSR)

| Obszar | Trasy |
| --- | --- |
| Publiczne | `/`, `/today`, `/start`, `/news`, `/news/{slug}`, `/events`, `/events/{slug}`, `/pages/{slug}`, `/rules`, `/server-information`, `/legal/{terms,privacy,cookies}`, `/download/{platform?}`, `/worlds`, `/rankings/{kind}`, `/characters` (wyszukiwarka), `/characters/{name}`, `/guilds`, `/guilds/{name}`, `/houses`, `/houses/{id}`, `/deaths`, `/online`, `/search`, `/sitemap.xml`, `/robots.txt` |
| Świat gry | `/wiki`, `/wiki/{kind}` (np. `items`, `creatures`, `npcs`, `spells`, `quests`), `/wiki/{kind}/{key}`, `/wiki/articles/{slug}`, `/wiki/search`, `/atlas`, `/atlas/{floor}/{x}/{y}`, `/atlas/tiles/{version}/{floor}/{z}/{x}/{y}.webp` |
| Konto | `/register`, `/login`, `/logout`, `/mfa/challenge`, `/verify-email`, `/forgot-password`, `/reset-password`, `/recovery-key`, `/email-change/confirm/{token}`, `/email-change/revert/{token}`, `/account` (przegląd), `/account/security` (hasło, e-mail, 2FA, passkeys, kody, klucz odzyskiwania), `/account/sessions` (z historią logowań), `/account/devices`, `/account/privacy` (prywatność, język, eksport, usunięcie), `/account/notifications`, `/account/characters/new`, `/account/characters/requests/{operation}`, `/account/characters/{id}` (profil i zarządzanie), `/account/premium`, `/account/payments` |
| Sklep | `/shop`, `/shop/coins` (doładowanie), `/shop/history`, `/shop/transfer` |
| Pomoc | `/support`, `/support/report-a-bug`, `/support/tickets`, `/support/tickets/{id}`, `/support/reports`, `/support/reports/{id}`, `/support/sanctions`, `/support/sanctions/{id}` (potwierdzenie, odwołanie) |
| Autoryzacja klienta | `/oauth/authorize`, `/oauth/consent`, `/link` (kod urządzenia) |

Prefiks języka: `/pl/...` i `/en/...`, z przekierowaniem z `/` według `Accept-Language` i cookie.

### 5.2 Player API (klient gry i launcher)

Klient ma jeden base URL portalu (domena portalu z §3.3). Od tego zależą bajty żądań:

- `v1/client/directory` i `oauth/token` klient składa **względem base URL**;
- tickety i postacie idą **ścieżką absolutną** `/api/v1/game-auth/...`, która ignoruje ścieżkę bazową.

Dlatego portal serwuje base URL w korzeniu domeny, bez prefiksu.

| Endpoint | Kształt (Game main, `crates/platform-client`) | Stan |
| --- | --- | --- |
| `GET /oauth/authorize` | `response_type=code`, `scope=game:ticket`, PKCE S256, `state` | Jest (Passport) |
| `POST /oauth/token` (form) | Tylko `authorization_code`; odpowiedź `{token_type:"bearer", access_token ≤8192 B}`; klient czyta tylko te dwa pola | Jest (Passport) |
| `GET /v1/client/directory` | `{epoch:u64, worlds:[{id, name, channels:[{id,name}], characters:[{character_ref, display_name}]}]}` (pola postaci według `platform-contracts`; do potwierdzenia w G-1); ≤1 MiB, ≤256 światów, ≤128 kanałów, ≤256 postaci, `name` ≤96 B | Brak w Platform i w locku |
| `POST /api/v1/game-auth/tickets` | Bearer, `{protocol_version:1}` → `{protocol_version:1, ticket ≤256 B, expires_in>0}`, `deny_unknown_fields`; 401/403 → AUTHENTICATION_REQUIRED | Jest; U1 w grze: NOT_IMPLEMENTED po stronie Platform |
| `GET /api/v1/game-auth/native-characters` | `{protocol_version:2, characters:[{character_id, world_id, name, availability: AVAILABLE\|UNAVAILABLE}]}`, UUIDv7, ≤16384 B | Jest |
| `POST /api/v1/game-auth/device-sessions` oraz `/native-characters`, `/tickets`, `/revoke` | Kontrakt tylko po stronie Platform; gra ich nie woła | Jest (Platform) |
| `GET /api/v1/account/summary`, `GET /api/v1/feed`, `GET /api/v1/worlds` | Nowe; kontrakt do uzgodnienia z grą (G-1) | Nowe |

Uwagi:

- **Directory nie służy do odkrywania tras.** Klient odrzuca na każdej głębokości klucze `host`, `port`, `endpoint`, `protocol`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`. Nowe trasy klient zna z wersji kontraktu, nie z directory.
- **`world.characters` w directory.** Kontraktu directory w grze nie ma, jest tylko konsument (`platform-contracts`: `DirectoryCharacter{character_ref, display_name}`, ≤256 na świat). LCFA (Accepted rev 2) zabrania publicznej listy postaci konta. Reguła: `characters` tylko dla uwierzytelnionego właściciela konta, a anonimowo `characters: []`. G-1 zapisze kontrakt directory z tą regułą w locku; do tego czasu portal zawsze zwraca `[]`.
- **Refresh token** jest nowym zachowaniem: dzisiejszy klient go nie używa. Portal może go wydawać (z rotacją i unieważnieniem rodziny przy ponownym użyciu), ale odpowiedź dla `authorization_code` musi pozostać zgodna z tym, co czyta klient.
- Kontrakty „jak dziś” portal implementuje na własnych typach wire. Zgodność bajt w bajt sprawdzają testy z wektorami (golden) skopiowanymi z Game. Portal nie zależy od crate'ów gry: `platform-contracts` przypina tokio przez workspace gry, a `build.rs` w `protocol-oteryn` czyta docs repo gry.

### 5.3 Game API `/internal/v1` (mTLS)

Wszystkie trasy poza `native-admissions` (token usługi gatewaya): TLS 1.3, HTTP/1.1, osobna tożsamość mTLS per grupa, limit rozmiaru, kanoniczny JSON, porównania w stałym czasie, odpowiedzi 503 i 401 dokładnie jak w kontrakcie. Testy kontraktowe czytają fixtures z `docs/contracts/fixtures/` i wymagają identycznych bajtów.

**5.3.1 Trasy, które portal serwuje** (klientem jest gra albo gateway):

| Grupa | Trasa | Kto woła | Kontrakt i stan |
| --- | --- | --- | --- |
| game-auth | `native-admissions` | Gateway logowania (dziś Go, docelowo rola `gateway`), token usługi | Kierunek potwierdzony; wpis w locku przez G-1 |
| game-auth | `native-evidence`, `native-runtime-status`, `native-scope-assignments`, `native-scope-revocations` | Gra | Bez wpisu w locku |
| game-auth | `native-account-characters`, `native-account-characters/watermark` (LCFA push) | Gra | Accepted rev2; snapshot `{contract_version:1, operation, source_authority, account_id, projection_epoch, projection_revision, source_observed_at, characters[0..64, posortowane]}`, watermark `{complete_through, observed_at}`; producent gotowy, wyłączony; konsumenta w Platform brak |
| game-auth | `character-commands/{pending, read, receipt}` | Gra | GAME-CHAR-CMD-0 Accepted (D1004/D1007); CreateCharacter, TransferCharacterOwnership, kody CHAR_CMD_* |
| game-auth | `character-bootstrap-intents/read` | Gra | Kontrakt v3 (`requested_sex`, binding tag 3); v1 i v2 odrzucane |
| game-auth | `game-restore/notice` | Gra | V2_GAME_RESTORE_RECONCILIATION; domyślnie wyłączony |
| products-entitlements | `premium-snapshots/read` (≤256 B) | Gra | Kandydat (jedyna z tych tras w locku) |
| products-entitlements | `loyalty-snapshots/read` | Gra | Gra używa (read model lojalności po stronie Platform, Game `0406`); domyślnie wyłączony |
| products-entitlements | `store/balance/read`, `store/history/read`, `store/purchases`, `store/operations/{pending, read, receipt, reconcile}` | Gra | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY; domyślnie wyłączony |
| game-public | `highscores`, `character-profile`, `world-online`, `watermark`; w kontrakcie dodatkowo `character-deaths`, `guild` i `house` | Gra | Highscores i profil Accepted (producent highscores jest w kodzie); zgony, gildie i domy CANDIDATE |

**5.3.2 Trasy, które portal woła w grze:** dziś żadnej. Każdy przepływ z grą to pull albo push inicjowany przez grę, a portal tylko serwuje.

**Kierunek `native-admissions`** (potwierdzony przez grę, Game main @6a81df27). Trasę serwuje portal, a woła ją tylko gateway logowania (dziś `services/game-gateway` w Go, `issuerPath`; docelowo rola `gateway`, T52) tokenem usługi, nie mTLS, przez loopback lub upstream; trasa nie jest publiczna. Serwer gry jej nie serwuje i nie woła. Lock gry (wpis N4P) jeszcze jej nie wymienia; dopisze ją G-1.

### 5.4 Panel zespołu

Panel działa na osobnym hoście, z obowiązkowym MFA. Uprawnienia odpowiadają obecnym (30 kluczy w 7 grupach, `app/Admin/AdminPermission.php`). Każde uprawnienie należy do co najmniej jednej wbudowanej roli, co sprawdza test; w obecnym portalu 6 uprawnień nie ma żadnej roli. Role i ich uprawnienia edytuje się w panelu.

| Sekcja | Zakres | Zadanie |
| --- | --- | --- |
| Treści | Newsy, strony, strony redakcyjne, wydarzenia, ogłoszenia, media, tłumaczenia pl/en, usuwanie | T11–T12 |
| Wiki | Poradniki, wersje, przegląd, publikacja | T13 |
| Świat gry | Import wersji, diff, wyniki walidacji, lista dopuszczonych encji, nazwy pl/en | T14c |
| Gracze | Wyszukiwanie, widok konta bez danych wrażliwych, sesje, postacie, wylogowanie wszędzie, notatki, reset MFA po weryfikacji przez support | T49 |
| Support i moderacja | Zgłoszenia z notatkami wewnętrznymi, zgłoszenia graczy, kary, odwołania | T51 |
| Handel | Cennik sklepu, nadanie i odebranie premium, korekta salda z powodem, rekoncyliacja płatności, zwroty | T28, T50 |
| Bazar | Moderacja ofert i zgłoszeń, wstrzymanie oferty, unieważnienie transakcji, opłaty i lista zbywalnych przedmiotów | T64 |
| Wydania klienta | Kanały i polityka aktualizatora | T35 |
| System | Role, audyt, flagi, zdrowie | T10, T10b |

Akcje krytyczne wymagają drugiej osoby (zasada czterech oczu): kary stałe, zwroty, aktywacja cennika, unieważnienie transakcji bazaru, nadanie roli admina, korekta salda powyżej progu i reset MFA.

### 5.5 Panel gracza

Panel obejmuje wszystko, co dziś ma Laravel, oraz to, czego tam brakuje lub co jest wyłączone. Porównanie powstało z inwentarza obecnego kodu i z tabeli stanu gry od koordynatora (Game main @00c01066). Uzupełniono je o przegląd panelu konta Tibii (2026-11-05). Przeniesiono funkcje, nie wygląd ani nazwy.

| Funkcja | Laravel dziś | Nowy portal | Zadanie | Dane z gry |
| --- | --- | --- | --- | --- |
| Przegląd konta | Postać pokazuje tylko nazwę i znacznik „main” | Postacie z poziomem, profesją, światem i linkiem do profilu; stan powiązania z grą; limit postaci; skróty do bezpieczeństwa, premium i salda | T44 | LCFA i profil publiczny (są) |
| Tworzenie postaci | Jest, wyłączone | Formularz, reguły nazw i lista zastrzeżonych, status operacji | T22c | GAME-CHAR-CMD-0 (jest, wyłączony) |
| Usunięcie postaci z anulowaniem | Brak | Jest; przywrócenie przez 60 dni | T46 | G-3 |
| Zmiana płci i transfer świata | Brak | Płatne coinami (D10); transfer włączony dopiero przy drugim świecie | T46 | G-3 |
| Zmiana nazwy | Brak | Płatna coinami (D10) | T46 | G-3 |
| Ustawienia profilu postaci | 2 z 9 opcji | Postać główna, ukrycie, komentarz i 7 przełączników widoczności | T45 | Niepotrzebne: portal sam renderuje profile |
| Profil publiczny | Świat, profesja, poziom, gildia | Dodatkowo zgony, osiągnięcia, umiejętności, ekwipunek i dom, według ustawień gracza | T24b | Podstawowy jest; reszta G-4 |
| Hasło, 2FA, kody odzyskiwania | Jest (TOTP) | Dodatkowo passkeys, zaufane urządzenia i 2FA z e-maila | T06, T07, T08c | – |
| Klucz odzyskiwania konta | Jest | Jest | T05b | – |
| Zmiana e-maila z cofnięciem | Jest | Jest | T08b | – |
| Sesje i urządzenia | Sesje web | Dodatkowo urządzenia klienta, historia logowań, alert o nowym urządzeniu | T08, T21 | – |
| Prywatność i język | 2 flagi | 2 flagi i język | T09c | – |
| Eksport danych | Brak | Jest | T09 | – |
| Usunięcie konta | Jest (14 dni) | Karencja 1–90 dni, anulowanie, przekazanie do gry | T09b | G-6 |
| Powiadomienia | Brak ustawień | Preferencje per kategoria, wypisanie, w kliencie | T36 | – |
| Saldo i historia coinów, transfer | Tylko w bazarze | Osobne strony | T26, T32 | – |
| Sklep i doładowanie | Brak | Jest | T29–T32 | Wire sklepu (wyłączony) |
| Premium | Brak strony | Status, data końca, historia | T47 | Snapshot premium (kandydat) |
| Płatności i rachunki | Lista zamówień | Dodatkowo rachunki | T47 | – |
| Zgłoszenia do supportu | Jest, bez załączników | Z załącznikami | T33, T33b | – |
| Zgłaszanie graczy | Jest | Jest | T33c | – |
| Historia kar i odwołania | Jest (tylko zapis w Platform) | Jest; zawieszenie konta blokuje logowanie do gry | T34, T34b | Kary w grze: G-5 |
| Postacie w kliencie (urządzenia) | Wyłączone | Jest, za flagą | T21 | – |
| Bazar | Tylko postacie, wyłączony | Postacie i cenne przedmioty: licytacje, ogłoszenia, depozyt, statystyki (§5.6) | T57–T64 | G-7 |
| Analizator sesji | Jest (wklejanie logu) | Automatyczny, w prywatnym dzienniku gry, bez wklejania (§5.8) | T72–T83 | G-9…G-11 |
| Ostatnie logowanie, czas online | Brak | Na profilu, z przełącznikiem prywatności (D11) | T24b | G-4 (dziś kontrakt je wyklucza, LCFA:48) |
| Dane konta w przeglądzie | Brak | Data założenia, ostatnie logowanie w grze i na stronie, status konta, saldo, punkty i tytuł lojalności | T44 | Logowanie w grze: G-4 |
| Kreator bezpieczeństwa | Brak | Ocena bezpieczeństwa konta i lista kroków: potwierdzony e-mail, 2FA, klucz odzyskiwania, zapasowy e-mail, data ostatniej zmiany hasła | T53 | – |
| Zapasowy e-mail | Brak | Drugi potwierdzony adres do odzyskiwania konta | T08b | – |
| Zaufane urządzenia | Brak | „Zapamiętaj urządzenie” przy 2FA, lista i usuwanie | T06 | – |
| 2FA kodem z e-maila | Brak | Rezerwowa metoda, gdy aplikacja nie jest możliwa | T06 | – |
| Odznaki konta | Brak | Odznaki za wiek konta, lojalność, sumę poziomów i poziomy wszystkich profesji; gracz wybiera, które pokazać na profilu | T54 | Poziomy: profil publiczny (jest) |
| Osiągnięcia na profilu | Brak | Gracz wybiera do 5 zdobytych osiągnięć do pokazania | T54 | G-4 |
| Postać do rankingu lojalności | Brak | Wybór postaci (nieukrytej) do rankingu lojalności | T54 | – |
| Kody prezentowe i kody z gry | Brak | Realizacja kodu: coiny, premium albo usługa, przypisanie do postaci; kod jednorazowy, idempotentnie | T55 | – |
| Usługi gotowe do użycia | Brak | Lista kupionych, jeszcze nieużytych usług (np. zmiana nazwy) | T46 | – |
| Saldo coinów | Jedno saldo | Rozbicie: dostępne, przenoszalne, zarezerwowane | T26 | – |
| Polecenia znajomych | Brak | Zaproszenie i powrót gracza, nagrody i statystyki | T56 | Nagrody w grze przez sklep |
| Tworzenie postaci, filtry świata | – | Sugestia nazwy; filtry świata (typ PvP, lokalizacja), gdy będzie więcej światów | T22c | – |

Reguły z przeglądu panelu Tibii. Przenosimy mechanikę, nie wygląd. Zadanie w nawiasie obejmuje regułę:

- Przegląd konta pokazuje przy postaci dom, gildię, ukrycie i trwające usunięcie. Limity postaci aktywnych i łącznie są w konfiguracji. Nazwa usuniętej postaci jest zablokowana przez 6 miesięcy w `reserved_names` (T44, T46).
- Pasek statusu konta informuje o:
  - niepotwierdzonym e-mailu;
  - odpowiedzi w zgłoszeniu;
  - nowej karze;
  - oczekującej zmianie e-maila;
  - zaplanowanym usunięciu konta (T44).
- Ustawienia postaci pokazują zgony z 30 dni i zabójstwa PvP z 10 tygodni. Jest też ukrycie wszystkich postaci konta naraz (T45, dane z G-4).
- Niepotwierdzone konto:
  - może ponownie wysłać link;
  - może poprawić adres przed potwierdzeniem;
  - do potwierdzenia nie ma dostępu do sklepu, bazaru ani płatnych usług (T03).
- Klucz odzyskiwania:
  - przy generowaniu gracz wpisuje go ponownie dla sprawdzenia;
  - jest wymagany do wyłączenia 2FA;
  - nowy klucz można dostać przez zapasowy e-mail (T05b).
- E-mail: zmiana od razu z kluczem odzyskiwania albo po 7 dniach oczekiwania z możliwością anulowania (D14). Gracz może też zamienić adres główny z zapasowym (T08b).
- Zaufane urządzenia są ważne 30 dni i znikają przy zmianie hasła lub 2FA (T06). Zmiana hasła wymaga obecnego hasła i wysyła e-mail (T08c).
- Strona wylogowuje po 45 minutach bezczynności; czas jest w konfiguracji (T02).
- Zgłoszenia graczy:
  - lista z 30 dni ze statusem;
  - najwyżej 20 oczekujących;
  - blokada zgłaszania po wielu nieuzasadnionych (T33c).
- Historia kar pokazuje poziom zachowania konta (T34b). Zgłoszenia supportu mają znacznik nowej odpowiedzi i są przechowywane 180 dni (T33).
- Historia w panelu (T47) obejmuje:
  - premium;
  - płatności, z wycofaniem nieopłaconego przelewu;
  - coiny (+/−, prezenty, bazar);
  - usługi;
  - kody.
- Zgody na e-maile informacyjne i powrotne są oddzielone od zgód obowiązkowych (T09c, T36).
- Lojalność:
  - punkt za każdy dzień premium;
  - tytuły na progach;
  - ranking lojalności per świat;
  - bonus w grze tylko wtedy, gdy gra go wdroży (T27b, T24a).
- Odzyskiwanie konta bez logowania obejmuje:
  - nowe hasło;
  - przejęte konto;
  - natychmiastową zmianę e-maila kluczem;
  - nowy klucz;
  - wyłączenie 2FA kluczem (T05, T05b).
- Rejestracja tworzy od razu pierwszą postać, z sugestią nazwy, płcią i wyborem świata (T03, T22c).
- Pomijamy:
  - dane adresowe i telefon (odzyskiwanie pocztą, SMS);
  - aplikację Observer;
  - wymianę tokenów.

### 5.6 Bazar

Bazar to miejsce, gdzie gracze sprzedają innym graczom postacie i cenne przedmioty za coiny albo za złoto z gry. Walutę wybiera sprzedawca przy wystawieniu. W portalu nazywa się po prostu „Bazar” (D13). Wzorce, przejrzane 2026-11-05:

- Char Bazaar Tibii: licytacje postaci;
- Exevopan: wyszukiwarka, filtry i statystyki aukcji;
- TibiaTrade: ogłoszenia przedmiotów i historia cen.

Najważniejsza różnica: portal pilnuje obu stron transakcji. Postać albo przedmiot czeka w depozycie gry. Coiny kupującego czekają w rezerwie portfela, a złoto w blokadzie po stronie gry. Wymiana dzieje się automatycznie, więc nie da się oszukać przy przekazaniu. TibiaTrade tylko łączy strony i nie odpowiada za wymianę.

| Co | Forma | Zasady |
| --- | --- | --- |
| Postać | Licytacja | Cena minimalna, koniec wybrany przez sprzedawcę od 1 do 14 dni, opcjonalnie „kup teraz” do pierwszej oferty (D15). Od wystawienia postać jest zablokowana: nie można się na nią zalogować ani jej zmienić. Po sprzedaży przechodzi na konto kupującego, a bez ofert wraca do sprzedawcy |
| Przedmiot | Ogłoszenie | Cena stała albo „przyjmuję oferty”. Oferta kupującego rezerwuje coiny; sprzedawca ją przyjmuje albo odrzuca. Ogłoszenie trwa 14 dni i można je odnowić |
| Przedmiot | Licytacja | Jak przy postaci: cena minimalna, czas, opcjonalnie „kup teraz” |
| Przedmiot | Ogłoszenie kupna | Gracz szuka przedmiotu, podaje cenę albo czeka na oferty. Sprzedawca odpowiada przedmiotem z depozytu |

Przedmioty:

- Wystawić można każdy przedmiot, który gra oznaczy jako zbywalny.
- Admin może zawęzić listę albo ustawić minimalną wartość.
- Stos ma cenę za sztukę.
- Atrybuty przedmiotu (np. ulepszenie) pochodzą z depozytu gry, nie z opisu sprzedawcy.

Przepływ:

1. Wystawienie w portalu wymaga potwierdzonego e-maila, 2FA i trybu sudo. Wystawienie jest darmowe (D13). Gra sprawdza warunki (np. dom, gildia, aktywna kara) i podaje powód odmowy.
2. Gra blokuje postać albo przenosi przedmioty do depozytu bazaru i zwraca snapshot z hashem. Portal publikuje ofertę dopiero po tym potwierdzeniu.
3. Licytacja:
   - gracz podaje swoją ofertę maksymalną, a system podbija cenę automatycznie o minimalny krok;
   - licytować może tylko gracz, który ma pełną kwotę oferty maksymalnej; portfel rezerwuje ją w coinach, a gra blokuje ją w złocie;
   - przebita oferta zwalnia rezerwę;
   - minimalne przebicie to 1% aktualnej ceny, co najmniej 1 coin albo 1 000 złota (D15);
   - oferta w ostatnich 5 minutach przedłuża aukcję o 5 minut (ochrona przed snipingiem, D15);
   - coinami licytuje się tylko przenoszalnymi.
4. Rozliczenie:
   - przy coinach jedna transakcja portfela obciąża kupującego i uznaje sprzedawcę, pomniejszając kwotę o prowizję 10% (D13);
   - przy złocie gra przenosi zablokowane złoto kupującego do sprzedawcy, pomniejszone o prowizję 2,5% bez górnego limitu, a portal zleca to komendą i czeka na potwierdzenie;
   - komenda do gry przenosi postać (TransferCharacterOwnership) albo dostarcza przedmiot na wybraną postać kupującego;
   - komenda ma outbox, klucz idempotencji i rekoncyliację;
   - coiny sprzedawcy stają się dostępne po potwierdzeniu przekazania z gry.
5. Sprzedawca może anulować ofertę przed pierwszą ofertą kupna. Później anuluje tylko zespół, na zasadzie czterech oczu. Oferta wygasła bez sprzedaży wraca do sprzedawcy.

Wyszukiwanie i filtry. Wszystkie filtry są dostępne za darmo; w Exevopan część jest płatna.

- Postacie:
  - świat, typ PvP i lokalizacja;
  - profesja, poziom od–do, umiejętność z zakresem;
  - nazwa, przedmiot posiadany przez postać;
  - usługi i rozszerzenia ze sklepu, outfity i mounty;
  - questy, osiągnięcia i punkty, które gra poda w snapshocie;
  - tylko z ofertami, rzadkie nazwy.
- Przedmioty: kategoria, typ, nazwa, atrybuty, świat, sprzedam lub kupię, waluta, cena od–do, tylko z ceną.
- Postacie i przedmioty: waluta (coiny albo złoto).
- Sortowanie: koniec, początek, cena, poziom albo umiejętność, rosnąco lub malejąco.
- Zakładki: bieżące, zakończone (z ceną sprzedaży), moje oferty kupna, moje wystawienia, obserwowane.

Strony:

- Karta postaci zawiera:
  - nazwę, poziom, profesję, świat, outfit i 4 wyróżnione przedmioty;
  - czas do końca aukcji;
  - obecną albo minimalną ofertę;
  - umiejętności z paskami;
  - skrót punktów i questów;
  - automatyczne etykiety, np. „dużo questów” albo „rzadkie mounty”.
- Szczegóły postaci mają zakładki ze snapshotu:
  - ogólne: HP, mana, udźwig, szybkość, błogosławieństwa, umiejętności z procentem, data utworzenia, doświadczenie, złoto, możliwość transferu świata;
  - przedmioty z wyszukiwaniem i kategoriami;
  - przedmioty ze sklepu;
  - outfity, mounty, osiągnięcia, questy.

  Zakładki zależą od systemów gry; systemy, których gra nie ma, nie mają zakładek. Strona pokazuje datę i hash snapshotu.
- Strona ogłoszenia przedmiotu pokazuje:
  - opis z gry i link do wiki;
  - cenę;
  - sprzedawcę;
  - cenę rynkową z ostatnich transakcji z linkiem do historii;
  - przycisk zgłoszenia;
  - link do udostępnienia z podglądem (Open Graph).
- Profil sprzedawcy pokazuje postać wybraną przez sprzedawcę, liczbę zakończonych transakcji, datę dołączenia i aktywne oferty. Kontakt odbywa się przez oferty z krótką wiadomością; czatu nie ma.

Ulepszenia ponad wzorce:

- depozyt i automatyczne rozliczenie zamiast zaufanego pośrednika;
- jawna licytacja z ofertą maksymalną i ochrona przed snipingiem;
- obserwowane oferty i zapisane wyszukiwania z powiadomieniem (e-mail i w kliencie): nowa pasująca oferta, przebicie, koniec za godzinę, wygrana;
- historia cen i cena rynkowa (mediana ostatnich transakcji), osobno dla coinów i złota;
- statystyki: obrót, odsetek udanych aukcji, podział profesji, rekordy;
- szacowanie wartości postaci na podstawie podobnych sprzedanych;
- porównanie dwóch postaci;
- wyróżnienie oferty za coiny (później; cena w konfiguracji).

Bezpieczeństwo:

- Nie można licytować własnej oferty.
- Zawieszone konto nie wystawia i nie licytuje.
- Oferty kupna mają limity.
- Zespół może wstrzymać ofertę.
- Wszystkie operacje trafiają do audytu, a kwoty do niezmiennego ledgera.
- Postać kupioną na bazarze można wystawić ponownie dopiero po 30 dniach (D15).
- Oferta o 25% droższa lub tańsza od ceny rynkowej jest oznaczona ostrzeżeniem (D15).

### 5.7 Statystyki

Wzór: GuildStats (przegląd 2026-11-05). GuildStats zbiera dane, odpytując stronę Tibii. Portal ma je wprost z projekcji gry, więc historia jest dokładna i bez opóźnień.

Portal po każdym ingeście projekcji (T24) zapisuje dzienną migawkę każdej postaci: poziom, doświadczenie, umiejętności i punkty. Do tego dochodzą zdarzenia:

- zmiana nazwy;
- zmiana świata;
- wejście do gildii i wyjście z niej;
- sprzedaż na bazarze;
- zgon;
- awans.

Czas online pochodzi z sesji w grze (G-8).

| Obszar | Funkcje | Zadanie |
| --- | --- | --- |
| Strona postaci | Zakładki: historia (nazwy, światy, gildie, sprzedaże na bazarze), doświadczenie (wykres dzienny, przyrost dzień, tydzień, miesiąc, rekordy), czas online (kalendarz dni, suma tygodnia i miesiąca), pozycje w rankingach w czasie, zgony; postęp gry (procent zdobytych osiągnięć i questów); przewidywanie daty poziomu | T66 |
| Porównanie | Dwie lub więcej postaci na jednym wykresie: poziom, doświadczenie, umiejętności | T66 |
| Rankingi przyrostu | Top doświadczenia (wczoraj, 7 dni, 30 dni, rekordy), czas online, najstarsze postacie i rocznice, zmiany nazw, transfery światów, postacie sprzedane na bazarze. Filtry: świat, typ PvP, profesja | T67 |
| Gildie | Lista: członkowie, aktywni, średni i łączny poziom, doświadczenie, punkty osiągnięć, wojny, data założenia. Historia członków, przyrost gildii, ranking gildii, porównanie gildii, rocznice, rozwiązane gildie | T68 |
| Świat | Wykres graczy online, spis (profesje, poziomy, płeć), zgony dziennie, statystyki zabitych potworów i bossów (G-8) | T69 |
| Obserwowanie | Postać lub gildia z powiadomieniem: online, awans, zgon, zmiana gildii | T70 |
| Kalkulatory | Doświadczenie do poziomu, stamina, trening, podział łupu; liczone z danych świata (T14) | T71 |

Prywatność:

- Statystyki postaci podlegają przełącznikom profilu (T45) i D11.
- Postać ukryta nie pojawia się w statystykach ani rankingach przyrostu.
- Czas online jest widoczny tylko przy włączonym przełączniku gracza.
- Obserwowanie nie omija tych ustawień.

Retencja: migawki dzienne przechowujemy bez limitu, a próbki co 5 minut przez 90 dni; potem zostają agregaty dzienne.

### 5.8 Dziennik gry (prywatny panel gracza)

Prywatna część panelu, widoczna tylko dla właściciela konta. Gra wysyła dane prywatnym kanałem per konto (G-9…G-11), nie przez projekcje publiczne. Analizator działa sam: gracz niczego nie wkleja. Wzór to analizatory klienta Tibii (sesja, łup, zużycie, obrażenia, doświadczenie), ale portal dodaje historię, porównania, wycenę i rekordy, których klient nie przechowuje.

| Obszar | Funkcje | Zadanie | Dane z gry |
| --- | --- | --- | --- |
| Pulpit konta | Wszystkie postacie naraz: online, poziom, stamina, premium, dostępni bossowie, czynsz domu, wynik ostatniej sesji | T79 | G-9, G-11 |
| Analizator polowań | Lista sesji z filtrami (postać, miejsce, data, drużyna). Szczegóły sesji: czas, doświadczenie i exp/h, łup, zużycie (mikstury, amunicja, runy), bilans i profit/h, obrażenia zadane i otrzymane według typu i źródła, leczenie, zabite potwory, miejsce. Porównanie sesji i miejsc; podział łupu w drużynie z listą przelewów; link udostępnienia | T73 | G-9 |
| Wycena | Ceny łupu i zapasów: u NPC (dane świata), rynkowa z bazaru (T63) albo własna gracza; źródło wybierane per przedmiot | T74 | – |
| Dropy i rekordy | Dziennik rzadkich dropów (próg rzadkości z danych świata albo wartości); rekordy osobiste: najlepszy exp/h i profit/h, największy łup, najdłuższa sesja, pierwsze zabicie bossa; prywatna kronika postaci | T75 | G-9 |
| Przedmioty | Ekwipunek, depot w każdym mieście, skrzynka pocztowa i dom, tylko do odczytu, ze stanem z chwili wylogowania; wyszukiwarka „gdzie mam przedmiot”; wartość majątku; skrót „wystaw na bazar” przy zbywalnych. Historia banku, marketu, wymian z graczami i bazaru | T76 | G-10 |
| Postęp i cele | Bestiariusz, bossowie z cooldownem, questy, osiągnięcia, historia umiejętności; cele gracza (poziom, umiejętność, bestiariusz) z prognozą daty z jego tempa | T77 | G-11 (bestiariusz i receipty bossów już są) |
| Zgony i PvP | Recap zgonu: kto i czym zadał obrażenia w ostatnich sekundach, leczenie, utracone poziomy i przedmioty, działanie błogosławieństw. Zabójstwa, asysty, skull i czas do jego wygaśnięcia | T78 | G-11 |
| Przypomnienia | Czynsz domu, koniec premium, cooldown bossa, pełna stamina, koniec licytacji na bazarze; e-mail i web push według preferencji (T36) | T79 | G-11 |
| Logowania do gry | Historia wejść do gry (czas, postać, kraj z IP, urządzenie) z przyciskiem „to nie ja”: wylogowanie wszędzie i zmiana hasła | T80 | G-11 |
| Mapa łowisk | Warstwa Atlasu: średni exp/h i profit/h według miejsca, przedziału poziomu i profesji, popularność; tylko agregaty | T81 | G-9 |
| Raport i eksport | Raport tygodniowy (opcjonalny e-mail), notatki do sesji i postaci, eksport sesji i dropów do CSV i JSON | T82 | – |
| Szukam drużyny | Tablica ogłoszeń: świat, cel (polowanie, boss, quest), przedział poziomu dla dzielonego doświadczenia, profesje, termin; kontakt przez nazwę postaci w grze; zgłoszenia i moderacja | T83 | – (poziom i profesja z LCFA) |

Najpierw powstają analizator polowań z wyceną (T72–T74) i przeglądarka przedmiotów (T76).

Zasady:

- Dziennik widzi tylko właściciel konta. Zespół widzi go wyłącznie z uprawnieniem `playerdata.read`, w ramach zgłoszenia, z wpisem do audytu.
- Gra wysyła podsumowanie po zakończeniu sesji, a długą sesję dzieli na godzinne odcinki. Portal nie pokazuje niczego na żywo ani pozycji innych graczy, więc dziennik nie pomaga botom.
- Link udostępnienia sesji jest opcjonalny, ma datę ważności i można go odwołać. Nie zawiera nazwy konta ani innych postaci gracza.
- Podział łupu pokazuje członków drużyny po nazwie postaci, tak jak gra w oknie drużyny.
- Po sprzedaży postaci na bazarze jej historia zostaje u sprzedawcy. Kupujący widzi dane od dnia przejęcia.
- Mapa łowisk liczy tylko miejsca z co najmniej 10 sesjami z 5 różnych kont, z opóźnieniem 24 h. Gracz może wyłączyć udział swoich sesji.
- Przeglądarka przedmiotów niczego nie przenosi. Jedyną drogą przedmiotu z gry przez portal pozostaje depozyt bazaru.
- Usunięcie konta usuwa dziennik, a eksport danych (T09) go obejmuje.

Retencja: szczegóły sesji 365 dni, potem agregaty miesięczne bez limitu; dropy, rekordy, zgony i postęp bez limitu; historia logowań do gry 180 dni.

### 5.9 Atlas i narzędzia gracza

Wzór: TibiaRoute (przegląd 2026-11-05): mapa dostaw zadań, trackery bestiariusza i bossów, miejsca polowań, charmów i bossów, kalkulatory. TibiaRoute wpisuje dane ręcznie, a gracz sam zaznacza postęp po zalogowaniu przez Discord. Portal ma dane świata wprost z gry (§6.1) i postęp gracza z dziennika (§5.8), więc wszystko liczy się samo.

| Narzędzie | Funkcje | Zadanie | Dane |
| --- | --- | --- | --- |
| Spawny na Atlasie | Każdy spawn potwora: ikona, liczba potworów, piętro, czas odrodzenia; skupienia przy oddaleniu; licznik potworów i spawnów w widoku. Link do miejsca (x, y, piętro, przybliżenie), pełny ekran, prywatne znaczniki gracza, „zgłoś błąd na mapie” | T84 | `content/world/spawns/spawns-*.json`, rodzina `Spawn.Source`: centrum i punkty `{cell, creature, respawn_ms}`, jeden punkt to jeden potwór |
| Mapa dostaw zadań | Zakładki: wszystko, potwory, przedmioty zadań. Wybór przedmiotu wymaganego przez krok questa (`collect`, `item`) podświetla potwory, które go upuszczają (z szansą i liczbą sztuk z tabel łupu), i ich spawny. Ranking potworów jako źródła przedmiotu (tier z szansy, gęstości spawnu i poziomu). Wyszukiwarka i filtry: poziom, piętro, miasto, rzadkość | T85 | Kroki questów, łupy (`probability_ppm`, `min_count`, `max_count`), spawny. Po G-13 krok ma klucz `item` przy jednoznacznym dopasowaniu, a pozostałe mają `item_resolution` (`ambiguous`, `unresolved`, `not_an_item`); portal pokazuje tylko kroki z kluczem, resztę oznacza jako nierozpoznaną i nie zgaduje po nazwie |
| Gdzie wypada | To samo dla dowolnego przedmiotu; przycisk „pokaż na mapie” na stronie wiki przedmiotu i potwora | T85 | Łupy, spawny |
| Bestiariusz na mapie | Postęp każdej postaci gracza bez wpisywania; kolor spawnu według stanu (ukończony, w trakcie, nie rozpoczęty); punkty charmów i progi zabójstw; filtry klasy, trudności i gwiazdek; spawny pojedyncze i grupowe; rekomendacje: nieukończone potwory z najlepszym tempem dla poziomu postaci | T86 | Pole `bestiary` w definicjach potworów (punkty, trudność, progi, klasa, gwiazdki); postęp postaci (G-11); spawny; mapa łowisk (T81) |
| Bossowie na mapie | Postęp i punkty (prowess, expertise, mastery), cooldowny postaci, stałe miejsca bossów i szansa pojawienia się po każdym restarcie serwera | T86 | Pole `bosstiary` w definicjach potworów; spawny bossów z `boss_policy.at_reset.chance` (polityka zatwierdzona przez właściciela, w realizacji w grze); G-11. Bossowie z pokoi i encounterów zostają wstrzymani i mapa ich nie pokazuje. To, czy boss pojawił się dziś, nie jest publiczne |
| Miejsca polowań | Karty: solo, duo, pełna drużyna; poziom, exp/h, profit/h; sortowanie po exp i po proficie; opis redakcji, dane z mapy łowisk i raporty graczy (sesje udostępnione z analizatora) | T87 | T81, T13 |
| Miejsca na bestiariusz i bossów | Karty: potwór lub boss, poziom, punkty, szacowany czas ukończenia z tempa zabójstw graczy | T87 | T81 |
| Dziś na świecie | Na stronie „Dziś” i w nagłówku: wzmocniony dziś potwór i boss, historia wzmocnień, rajdy z harmonogramu | T88 | G-12 |

Pierwowzór: repozytorium `Oteryn/Oteryn-Atlas` (przegląd 2026-11-05, main @0bbd188). Ma przeglądarkę WebGL całego świata, kafelek po kafelku. Publikacja ma 18,75 GB i powstała ze starego OTBM oraz klienta Tibii 15.32 z atestacją praw. Działa tylko jako podgląd w sieci domowej, a repozytorium jest w zamrożeniu. Atlas portalu je zastępuje (D19). Portal nie przenosi tego renderera: kafle z minimapy gry (T40) są lżejsze i nie zależą od danych Tibii. Przenosi natomiast:

- estymator farmy: oczekiwana liczba zabójstw oraz P50, P80 i P95 do zdobycia N sztuk z dokładnych szans ppm, wraz z kontraktem liczbowym i testami (`src/browser/farm-intelligence.mjs`, `docs/testing/ATLAS-FARM-ESTIMATOR-NUMERIC-CONTRACT.md`); w mapie dostaw zadań pokazuje na przykład „ok. 120 zabójstw, 80% szans przy 190”;
- ranking wyszukiwania (NPC, potwory, miasta, miejsca, teleporty, domy, obszary questów);
- odznaki ról NPC (bank, podróż, sklep, quest, błogosławieństwo, trener);
- obszary i podobszary;
- stan widoku w adresie URL;
- szuflady na telefonie.

Reguły z Atlasu obowiązują też portal:

- portal nie wymyśla współrzędnych, pięter ani identyfikatorów należących do gry;
- w czasie działania nie sięga po dane OTBM ani Tibii;
- każdy artefakt niesie wersję treści gry;
- brak danych pokazuje jako „niedostępne”;
- wyliczenia z szans łupu oznacza jako szacunek, nigdy jako fakt.

Mechaniki właściwe tylko dla Tibii (np. fiendish przez exivę, koło przeznaczenia, imbuementy) przenosimy tylko wtedy, gdy gra je ma. Kalkulatory (T71) dostają w takim przypadku odpowiedniki: obrażenia na bossach, koszt ulepszeń, miejsce na punkty bestiariusza.

### 5.10 Rozrywka i dodatkowe przychody (bez hazardu)

Zasada: gracz płaci za konkretną rzecz, którą widzi przed zakupem, a nie za szansę na cenną wygraną. Gry losowe o coiny, złoto lub zbywalne przedmioty to w Polsce hazard (ustawa o grach hazardowych; kasyno online to monopol państwa), więc portal ich nie ma. Konsultacja prawna (ochrona konsumentów, nieletni) nie blokuje prac (D23).

| Funkcja | Opis | Przychód | Zadanie | Dane z gry |
| --- | --- | --- | --- | --- |
| Pomocnik w domu | Gracz kupuje pomocnika do swojego domu za stałą cenę w sklepie (jak hireling). Pomocnik ma zawody (np. górnik, zielarz, rybak, kupiec) i poziom. Gracz wysyła go na wyprawy (1–12 h) z portalu, telefonu albo w grze, rozmawiając z nim w domu. Po powrocie łup trafia do skrzyni pomocnika w domu. Powiadomienie o powrocie (T36) | Pomocnik, kolejne zawody, dodatkowy pomocnik (limit 3 na konto), wygląd pomocnika | T92 | G-14 |
| Przepustka sezonowa | Sezon 6–8 tygodni, wyzwania w grze (z dziennika, §5.8) i na portalu, ścieżka darmowa i płatna, nagrody kosmetyczne | Płatna ścieżka za coiny | T89 | G-9, G-11 |
| Kolekcja potworów | Karty potworów ze sprite'ami gry, odblokowywane postępem bestiariusza; talia, turowe walki PvE i PvP, rankingi, turnieje. Własna mechanika, bez nazwy i wyglądu Pokémona | Skórki kart, dodatkowe talie, zestawy o znanej zawartości | T90, T91 | G-11 |
| Codzienne zagadki | „Zgadnij potwora” po sylwetce, „Gdzie to jest?” na kaflach Atlasu, quiz wiedzy o świecie; serie dni i odznaki | Brak; ruch na portalu | T93 | – |
| Rozgrywki | Rankingi gildii w wyzwaniach sezonu, typer wydarzeń świata bez stawek | Brak | T94 | G-12 |

Reguły:

- Każdy zakup ma stałą cenę i znaną zawartość. Nie ma płatnych losowych skrzynek.
- Losowość jest tylko w nagrodach, za które gracz nie płaci. Płatne rzeczy nie zwiększają liczby losowań zbywalnych nagród, więc pomocnik nie ma płatnego przyspieszania wypraw.
- Cele wyprawy mają jawną tabelę nagród. Łup z wypraw ma tygodniowy limit na konto, a zakup pomocnika nie daje przewagi w walce, żeby nie było pay-to-win ani inflacji.
- Waluty, karty, odznaki i nagrody minigier są niezbywalne: nie da się ich przekazać, wymienić na coiny lub złoto ani wystawić na Bazarze. Wyjątkiem jest łup pomocnika: to zwykłe przedmioty gry, objęte limitem.
- Nic z minigier nie daje przewagi w walce w grze.
- Nagrody kosmetyczne wymagają rodzaju COSMETIC w kontrakcie sklepu. Właściciel zgodził się go włączyć (D21); zmianę kontraktu z grą robimy przed T89.

## 6. Integracja z grą

| Przepływ | Kontrakt | Stan po stronie gry (Game main @60de8634) |
| --- | --- | --- |
| Logowanie: OAuth → ticket → gateway | N4-P (OAuth PKCE), U1 (ticket), V2_NATIVE_GATEWAY_LOGIN | N4-P LOCKED, tylko testing/preprod; klient N4 częściowy (`native_login.rs`), pełne pokrycie do potwierdzenia w G-1; tryb Gateway domyślnie wyłączony |
| Gateway logowania | Publiczne `POST /v1/login`, protokół 2 (poniżej) | Klient gotowy (`native_login.rs`, `gateway_login`) |
| Postacie konta | LCFA (`native-account-characters`) | Producent gotowy, wyłączony |
| Tworzenie postaci | GAME-CHAR-CMD-0 | Producent zaimplementowany, domyślnie wyłączony (`[platform.character_commands]`, `production_enabled=false`); lock mówi jeszcze ACCEPTED_NOT_IMPLEMENTED, poprawi G-1 |
| Premium | V2_PREMIUM_TIME_SNAPSHOT | Kandydat; endpoint jest |
| Sklep | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY | PROPOSED, domyślnie wyłączony |
| Dane publiczne | GAME-PUBLIC-PROJ-0 | Kandydat rev1, nieakceptowany |
| Status światów | V2_RUNTIME_STATUS_PROJECTION | PARTIAL |
| Kary | Platform jest autorytetem (D9). Zawieszenie konta działa bez gry: portal odmawia ticketu i admission. Kary w grze (np. wyciszenie) według `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` po G-5 | Kandydat; wykonania w grze brak |
| Komendy postaci poza tworzeniem | Usunięcie i anulowanie, nazwa, płeć, świat (G-3) | Brak kontraktu; dane cyklu życia postaci są tylko szkicem |
| Rozszerzone dane publiczne | Zgony z zabójcą, gildie, domy, osiągnięcia, umiejętności, ekwipunek (G-4) | Zgony, gildie, domy CANDIDATE; reszta: dane są, projekcji brak |
| Usunięcie konta | Co dzieje się z postaciami i projekcjami (G-6) | Brak |
| Statystyki | Sesje online per postać (start, koniec) i dzienne liczniki zabitych potworów i bossów per świat (G-8); reszta z projekcji publicznych (G-4 dostaje doświadczenie i umiejętności per postać) | G-8 w kolejce gry po G-7 (koordynator, 2026-11-05). Jest: aktywna sesja, postęp bestiariusza per postać, receipty zabójstw bossów. Brak: historii sesji i dziennych liczników per świat; doda je G-8 |
| Bazar | Blokada i snapshot postaci, warunki wystawienia, depozyt i dostawa przedmiotów, blokada i przekazanie złota (G-7); przeniesienie postaci przez TransferCharacterOwnership z GAME-CHAR-CMD | G-7 w kolejce gry po G-5 (koordynator, 2026-11-05). Jest: bank konta i gildii, depot z transferami, market, TransferCharacterOwnership, pole `tradeable` w danych przedmiotów (bez egzekwowania). Brak: blokady złota, blokady postaci, podpisanego snapshotu. Gra przyśle kontrakt wire do akceptacji Platform przed implementacją |
| Dziennik gry | Prywatny kanał per konto, oddzielony od projekcji publicznych: sesje, łup, zużycie, obrażenia i dropy (G-9); przedmioty i historia handlu (G-10); postęp, zgony, PvP, liczniki czasu i logowania do gry (G-11) | G-9…G-11 w kolejce gry po G-8 (koordynator, 2026-11-05); kolejność między nimi ustala właściciel. Jest: receipty zabójstw bossów i łup z pojemników (sesje); stan ekwipunku, depotu, skrzynki i domu, bank, market (przedmioty); bestiariusz, bosstiary z cooldownami, questy, osiągnięcia, umiejętności (postęp). Brak: dziennika odcinków sesji z obrażeniami, leczeniem i zużyciem; snapshotu przy wylogowaniu; recapu zgonu, historii logowań i zdarzeń premium, staminy i czynszu. Dziennik wymian do sprawdzenia. Gra przyśle kontrakt wire do akceptacji Platform |
| Atlas i narzędzia | Spawny, kroki questów, łupy, bestiariusz i bosstiary z `content/**` (§6.1); wzmocnienia dnia i rajdy (G-12); klucze przedmiotów w questach (G-13) | Koordynator (Game main @9418ffb8): spawny są (31 plików, `Spawn.Source`); łupy z szansą ppm; `bestiary` i `bosstiary` w definicjach potworów. Polityka spawnów bossów (losowanie przy restarcie, stałe miejsca, szansa per boss) jest w realizacji. Osobnego typu „dostawa” nie ma; G-13 (w realizacji) dodaje klucze przedmiotów do kroków questów. Stare eksporty dla Oteryn-Atlas (`OTERYN_GAME_ATLAS_*_V1`) nie liczą z `content/**` i zostają historyczne (koordynator, Game main @dba9befb). Wzmocnienie dnia jest (mechanika 0351), rajdy z harmonogramu częściowo (0434), szans bossów brak |
| Pomocnik w domu | Zakup jako uprawnienie przez wire sklepu; komendy wyprawy (start, odbiór) idempotentne przez outbox jak komendy postaci; wynik wyprawy i łup liczy gra, portal dostaje receipt (G-14) | Gra ma domy i hirelinga w domu (mechaniki 0425, 0610: usługi, strój, jedzenie). Brak zawodów z poziomami, wypraw z tabelą nagród, limitu łupu, skrzyni pomocnika i komend portalu. G-14 rozszerza istniejącego hirelinga; w kolejce gry po pierwszym wydaniu (Game main @dba9befb) |
| Restore | V2_GAME_RESTORE_RECONCILIATION | Domyślnie wyłączony |
| Dane świata (wiki, Atlas) | Odczyt `content/**` z przypiętego commita gry (§6.1) | Dane są; flaga widoczności brak (G-2); publikacja grafik: zgoda (D7) |
| Granty admission | FND-04B-AMEND-1 | Pole status: ACCEPTED (2026-11-05), ale treść ma otwarte decyzje właściciela. Traktujemy jako propozycję i **nie implementujemy na jej podstawie**, dopóki decyzje nie zapadną |

**Gateway `POST /v1/login`** (serwuje gateway logowania, woła klient gry; kontrakt `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`):

- Żądanie: `{protocol_version:2, game_login_ticket, attempt_ref (UUIDv7), character_id, channel_id|null, offer:{client_build, client_platform:"windows", transports:[{protocol_major:1, transport_profile:1, alpn:"oteryn-game/1"}]}}`.
- Odpowiedź 200: `{protocol_version, attempt_ref, world_id, channel_id, endpoint:{host, port, tls_server_name, alpn, protocol_major, transport_profile}, grant:{profile:"oteryn-pre-admission-v1", token (JWS ≤4096 B), valid_for_seconds}}`.
- Błąd: `{protocol_version, error:{code, public_class}, attempt_ref|null}`, 11 kodów z `GATEWAY_ERROR_ROWS`.
- Ponowienia klienta: ≤6 żądań, backoff 1 s·2^(n-1), `Retry-After` ≤30 s.

Zasady:

- Portal implementuje wszystkie przepływy, ale każdy jest za flagą. Włączamy go, gdy gra ma swoją stronę.
- Funkcja bez danych z gry pokazuje „niedostępne” z czasem ostatniej aktualizacji, nigdy zmyślonych wartości.
- Ustawienia profilu (ukrycie, postać główna, komentarz, przełączniki) należą do portalu, bo to portal renderuje profile z projekcji. Gra nie musi nic zmieniać.
- Każda mutacja przez granicę gry jest idempotentna, ma klucz operacji, outbox i rekoncyliację. Timeout nie oznacza sukcesu ani porażki.
- Gateway logowania przechodzi z Go do Rusta (rola `gateway`, T52) z tym samym kontraktem bajt w bajt, więc klient gry nic nie zmienia. Obecny gateway w Go (`services/game-gateway`, ok. 900 linii, bezstanowy) działa do przełączenia. Portal serwuje `native-admissions` bajt w bajt obu wersjom.
- Wymagania klienta wobec gatewaya (koordynator, Game main @7cdd69a7). Klient dekoduje odpowiedź ściśle: odrzuca nieznane i zduplikowane pola. Wymaga:
  - zgodności trójki status HTTP + kod + `public_class` z tabelą błędów;
  - `Content-Type: application/json`;
  - echa `attempt_ref`;
  - ALPN `oteryn-game/1`;
  - żądania do 2048 B i odpowiedzi w limicie.
  
  `Retry-After` przy 429 to wyłącznie liczba całkowita sekund, najwyżej 30; format HTTP-date nie jest obsługiwany. Wybór świata i kanału zostaje po stronie portalu, w grancie. Przepisanie jest 1:1. Kolejka logowania i komunikat przerwy technicznej (np. `NATIVE_LOGIN_MAINTENANCE`) to późniejsze zmiany kontraktu z zadaniem dla gry. Do tego czasu nie wysyłamy nowych kodów.
- Lock kontraktów w grze jest nieaktualny dla N4 i CharCmd; poprawi go G-1. Przed każdym zadaniem z §5.2–§5.3 koordynator potwierdza aktualny stan kontraktu.

### 6.1 Treści świata gry (wiki i Atlas)

Zasada: **portal czyta te same dane, które ładuje serwer gry**, z przypiętego wydania gry. Nie ma osobnego formatu eksportu, osobnego kontraktu ani ręcznie przepisywanych danych. Jedno źródło prawdy: repozytorium gry.

| Co | Źródło w repo gry (Game main @6a81df27) | Co robi portal |
| --- | --- | --- |
| Przedmioty, potwory, NPC, łupy, questy, czary, osiągnięcia, obszary | JSON w `content/`: `items/definitions/items-*.json`, `creatures/definitions/`, `npcs/definitions/npcs-*.json`, `loot/loot-*.json`, `quests/definitions/quests-*.json`, `abilities/`, `achievements/`, `world/`. Każdy plik ma postać `{"family":..,"records":[..]}`; schematy per domena w `tools/content-schema/*-authoring/*.schema.json` | Crate `world` czyta te pliki własnymi typami serde i sprawdza je tymi schematami. Wynik trafia do `world_entities` i `world_links`; wiki generuje z nich strony, listy, filtry i wyszukiwanie |
| Pozycje na mapie | `content/world/` (`npc-placements` z komórką `{floor,x,y}`, obszary, obiekty) | Znaczniki Atlasu |
| Mapa | Minimapa `content/assets/files/minimap-32-<x>-<y>-<z>-<sha>.bmp.lzma` | Worker rozpakowuje LZMA, skleja BMP i tnie piramidę kafli WebP per piętro |
| Grafiki | `appearances-*.dat` (protobuf), `sprites-*.bmp.lzma`, manifest `content/assets/catalog/index.json` | Worker wycina miniatury PNG i WebP (D7: zgoda) |

Czego portal nie używa:

- Crate'a `oteryn-gameplay-content`, bo nie działa bez reszty workspace gry.
- `crates/renderer` gry, bo robi atlas tekstur klienta, a nie kafle mapy.
- Katalogu CATALOG-V1 (`OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`), bo ma status PROPOSED, producent ma tylko fixture i brakuje w nim pozycji na mapie. Czytanie `content/**` daje to samo bez drugiego formatu.

Jak to działa:

1. Admin wybiera w panelu wersję gry (commit lub tag). Worker pobiera z niej tylko `content/**` i `tools/content-schema/**` kluczem tylko do odczytu (deploy key repo gry, trzymany na hoście jako sekret).
2. `world` parsuje dane do nowej wersji w bazie i renderuje kafle obok. Gdy wszystko się uda, przełącza aktywną wersję jedną transakcją. Błąd parsowania zatrzymuje import, a stara wersja dalej działa.
3. Wiki i Atlas pokazują wersję świata. Odnośniki wiki ↔ Atlas idą po kluczach encji, nie po nazwach.

Co robi redakcja: pisze poradniki i opisy dopisane do encji (`wiki_guides`). Nie przepisuje statystyk ani łupów, bo te zawsze pochodzą z gry. Nowa wersja gry aktualizuje dane bez ruszania tekstów redakcji.

Ukryte treści:

- Osiągnięcia mają flagę `secret`; portal jej nie pokazuje.
- Questy, NPC, obszary i miejsca nie mają flagi widoczności. Gra doda pole w swoich schematach (osobny pakiet po stronie gry, G-2). Do tego czasu wiki pokazuje tylko encje, które admin zatwierdzi w panelu (`world_visibility` działa jako lista dopuszczonych), a domyślnie nic nie jest publiczne.
- Rekordy questów mają `classification` (np. `OTS_HYPOTHESIS_ONLY`, `SCOPED_EVIDENCE_ONLY`, `OTERYN_AUTHORED_APPROXIMATION`). Wiki publikuje wszystkie (D8) i pokazuje klasyfikację przy queście.

Licencja: `LICENSE-ASSETS.md` gry zastrzega grafikę, mapy i metadane („all rights reserved”, poza użyciem referencyjnym), a `content/world/manifest.json` ma `licensing_metadata: PENDING`. Właściciel zgodził się na publikację sprite'ów i mapy na stronie (D7, 2026-11-05).

Dane na żywo (online, rankingi, status światów) to projekcje z §5.3, nie część danych świata.

**OPEN QUESTION — signed content release manifest.** Should a signed manifest bind the pinned game release, wiki/catalog data, Atlas assets and rule test vectors, with verification and atomic activation/rollback? This remains non-binding and Game-contract dependent; it does not replace the direct `content/**` import above. Source: #1643 draft:55 @ 1c9c638f (draft D12).

## 7. Bezpieczeństwo

- Hasła: argon2id z rehashem; sprawdzanie haseł w wyciekach (k-anonymity); rate limity logowania, rejestracji i odzyskiwania; ochrona przed enumeracją kont.
- Sesje:
  - cookie `__Host-`, `Secure`, `HttpOnly`, `SameSite=Lax`;
  - rotacja przy logowaniu, zmianie hasła, 2FA i e-maila;
  - tryb sudo dla akcji wrażliwych;
  - po zmianie hasła lub e-maila, użyciu klucza odzyskiwania i wyłączeniu 2FA portal unieważnia sesje web, rodziny tokenów OAuth i sesje urządzeń klienta;
  - e-mail o logowaniu z nowego urządzenia.
- Zmiana e-maila (D14): bez klucza odzyskiwania zmiana po 7 dniach z anulowaniem ze starego adresu; po zmianie 72 h blokady transferu coinów, bazaru i kolejnej zmiany e-maila.
- Rejestracja: rate limit i proof-of-work bez zewnętrznych usług.
- Rate limity: zestaw z obecnego portalu (`app/Providers/AppServiceProvider.php`) jako punkt wyjścia.
- CSRF: token synchronizowany dla każdej mutacji w web.
- Tokeny API: nieprzezroczyste, przechowywane jako SHA-256, z zakresami. Refresh rotowany; ponowne użycie unieważnia całą rodzinę.
- Waluta: transakcja z `SELECT ... FOR UPDATE` na koncie portfela, brak ujemnych sald, klucz idempotencji dla każdej operacji.
- Bazar: postać i przedmiot w depozycie gry, coiny w rezerwie portfela, złoto w blokadzie gry, rozliczenie w jednej transakcji z kluczem idempotencji; unieważnienie tylko na zasadzie czterech oczu.
- Dziennik gry: dostęp tylko właściciela konta, zespół z uprawnieniem i audytem; dane po zakończeniu sesji, nigdy na żywo; mapa łowisk tylko z agregatów z progiem i opóźnieniem (§5.8).
- Webhooki płatności: weryfikacja podpisu, idempotencja po id zdarzenia, ochrona przed powtórzeniem.
- Nagłówki: CSP bez `unsafe-inline`, HSTS, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`.
- Uploady: limit rozmiaru, sprawdzanie typu po zawartości, przetwarzanie obrazów, brak wykonywania.
- Audyt: każda akcja zespołu trafia do `audit_log` (kto, co, kiedy, przed i po).
- Sekrety: tylko zmienne środowiska lub pliki na hoście; nic w repozytorium.

### 7.1 Security proposals salvaged from #1643

These are non-binding design notes for later implementation tasks, not evidence of delivered controls or authorization to change existing contracts. In the citations below, `draft` means `docs/architecture/OTERYN_PLATFORM_2_DESIGN_DRAFT.md` on the source branch; the draft is not copied here.

- **Audit integrity:** append-only daily hash chains, with each chain head exported to logs. Retain actor, peer, action, target, before/after digests, request ID and IP alongside the existing audit fields. Source: #1643 draft:249-266 @ 1c9c638f.
- **Supply chain:** produce an SBOM and apply keyless sigstore signing to each image. Source: #1643 draft:249-266 @ 1c9c638f.
- **Single-approver fallback:** where only one eligible approver exists, allow self-approval only after a 15-minute cooldown and email the owner. This proposed exception needs explicit acceptance before it can alter the four-eyes controls in §5.4 or other accepted contracts. Source: #1643 draft:37 @ 1c9c638f.
- **OPEN OWNER DECISION — secret-file permissions:** should the loader require secret files in `/run/secrets` with mode `0400` and reject group- or world-readable files? The draft's file-only policy conflicts with §7's “environment variables or files”; §7 remains unchanged pending the owner's decision. Source: #1643 draft:249-266 @ 1c9c638f.

## 8. Wygląd i jakość

- Projekt graficzny od nowa (D4). Obecny wygląd Oteryn Platform jest tylko przykładem: nie kopiujemy go, bo portal ma dużo więcej powierzchni (bazar, Atlas, dziennik gry, statystyki, rozrywka).
- Najpierw system projektowy (T39): ciemny motyw z akcentem marki, czytelna typografia, ikony i komponenty (tabele, karty, formularze, wykresy, mapa, panele boczne) w jednym pliku tokenów CSS, z galerią komponentów.
- Potem makiety kluczowych ekranów na tych tokenach (T39b): strona główna, panel gracza, bazar, Atlas, dziennik gry, statystyki. Właściciel akceptuje makiety, zanim powstaną strony; do tego czasu strony korzystają tylko z komponentów T39.
- Pełna obsługa telefonu i WCAG 2.2 AA.
- Budżety: LCP poniżej 2 s na 4G, JS poza mapą poniżej 30 KB, p95 odpowiedzi poniżej 150 ms dla stron publicznych.
- SEO: sitemap, `hreflang`, kanoniczne adresy, dane strukturalne.

## 9. Uruchomienie

1. Preprod na `oteryn-vps-2`: PostgreSQL, portal, mTLS z gatewayem i węzłem testowym.
2. Narzędzie `admin-cli import-legacy` (T38–T38e) jednorazowo czyta eksport z obecnej bazy. Kopiuje:
   - konta z AccountId, hashami haseł, sekretami TOTP (ponownie zaszyfrowanymi), kodami i kluczami odzyskiwania, flagami prywatności i zaplanowanymi usunięciami;
   - role;
   - treści, wiki i media;
   - postacie z projekcji i ustawienia ich profili;
   - portfel, premium i płatności;
   - zgłoszenia, zgłoszenia graczy, kary i odwołania, więc aktywne zawieszenia działają od pierwszej minuty.

   Nie kopiuje sesji ani tokenów w locie (reset hasła, zmiana e-maila). Gracz loguje się ponownie i w razie potrzeby powtarza prośbę.
3. Weryfikacja importu: liczby i sumy kontrolne, logowanie próbnych kont, zgodność sald.
4. Bazar Laravela jest wyłączony poza środowiskiem testowym, więc w produkcji nie ma otwartych aukcji ani zablokowanych coinów. Import to sprawdza i przerywa się, jeśli jakieś znajdzie. Nowy bazar startuje pusty.
5. Przekierowania 301 ze starych adresów (T43): `/highscores` → `/rankings/level`, `/servers` → `/worlds`, `/getting-started` → `/start`, `/wiki/{slug}` → `/wiki/articles/{slug}`, `/wiki/items/*` i `/wiki/creatures/*` → strony encji.
6. Przełączenie domeny i gatewaya na nowy portal (rola `gateway` zamiast `services/game-gateway`). Laravel i gateway w Go zostają wyłączone, a ich kod i workflowy CI są usuwane z repozytorium osobnym PR-em.

Stan 2026-11-11 (decyzja właściciela): `portal-rs/` jest główną platformą na `main`; kod Laravela i gateway w Go przeniesiono do `legacy/` (gateway: `legacy/services/game-gateway/`), ich workflowy CI wyłączono, a sam katalog `legacy/` jest usuwany przy przełączeniu w kroku 6.

Każdy krok wymaga zgody właściciela („deploy”).

### 9.1 Rollback and cutover gate

Non-binding rehearsal and gate proposals, to be qualified in the deployment task. Every deployment still requires the owner's authorization above. Apply them only to the accepted first-release scope; node management and GM observe remain deferred (D1).

- **Rollback triggers:** successful logins below 95% for 15 minutes; admission or evidence errors above the measured baseline; any monotonic-counter regression; or failed import verification. Source: #1643 draft:1002-1013 @ 1c9c638f.
- **Rollback boundary:** before the new portal accepts writes, restore the previous routing and processes. After writes begin, use a rehearsed reverse import before returning to Laravel and the Go gateway. Preserve identities, password hashes, generations, floors, witnesses, intents and acknowledgements, epochs and ledgers; reconcile accepted payments into the legacy ledger. A route-only rollback would regress Game floors/revisions and is unsafe. Retain new-portal-only data for a later attempt. Source: #1643 draft:1002-1013 @ 1c9c638f.
- **Data gate:** three cutover rehearsals and one reverse-import rehearsal; fourteen nightly imports against an isolated production-like copy with zero differences; measured cutover window at most 60 minutes. Source: #1643 draft:1068-1095 @ 1c9c638f.
- **Scope gate:** accepted first-release features pass preprod and dark-production qualification, including the Atlas parity checklist and legacy-URL redirects checked against a crawl. Source: #1643 draft:1068-1095 @ 1c9c638f.
- **Compatibility and security gate:** all applicable wire/rule vectors pass; verify legacy password hashes, TOTP decryption and keyed-hash lookups on a controlled snapshot; review authentication and payments, and verify CSRF, rate limits, sudo and CSP. Source: #1643 draft:1068-1095 @ 1c9c638f.
- **Performance gate:** `/internal/v1` p99 below 250 ms at twice expected load; tools p95 below 200 ms; gateway p99 no worse than the Go gateway; cold Atlas overview first frame below 2.5 s at 50 Mbit/s, CPU frame time p95 below 16 ms on an agreed reference machine, and incremental content build below 45 minutes. The draft's public-page p95 below 300 ms does not relax §8's stricter below-150-ms budget. Source: #1643 draft:1068-1095 @ 1c9c638f.
- **Operational and product gate:** successful restore drill, working alerts for projections, command age, admission, disk, payment webhooks, content builds/validation and CDN failures; reviewed runbooks, staff training, applicable legal/privacy review, EN/PL coverage, SPF/DKIM/DMARC, reviewed map visuals and signed owner UAT. Source: #1643 draft:1068-1095 @ 1c9c638f.

## 10. Zadania dla Codexa

Każde zadanie to jedna mechanika i effort medium. Ma dozwolone ścieżki, kryterium akceptacji (`cargo test` i `cargo clippy` w `portal-rs/`) oraz zależności. Zadania Platform wysyła sesja Platform; zadania gry wysyła koordynator. Zadania bez zależności między sobą idą równolegle.

| ID | Zadanie | Zależy od | Ścieżki |
| --- | --- | --- | --- |
| T00 | Workspace, lints, `core` (konfiguracja, błędy, UUIDv7), `db` (pula, migracje), binarka z rolami, `/healthz` | – | `portal-rs/` (cały szkielet) |
| T01 | `web`: router, layout askama, statyczne pliki, nagłówki bezpieczeństwa, strony błędów, i18n fluent pl/en | T00 | `crates/web`, `templates/`, `static/` |
| T02 | Sesje web (tabela, cookie `__Host-`, rotacja) i CSRF | T01 | `crates/web/src/session*`, migracja |
| T03 | Rejestracja, weryfikacja e-maila, zgody (wersja regulaminu i polityki) | T02 | `crates/identity/src/register*` |
| T03b | Ochrona rejestracji: proof-of-work bez zewnętrznych usług, sprawdzanie haseł w wyciekach | T03 | `crates/identity/src/abuse*` |
| T04 | Logowanie i wylogowanie, argon2id z rehashem, rate limit | T02 | `crates/identity/src/login*` |
| T05 | Reset hasła | T04 | `crates/identity/src/reset*` |
| T05b | Klucz odzyskiwania konta: generowanie, unieważnienie, odzyskanie (e-mail, klucz i nowe hasło) | T05 | `crates/identity/src/recovery_key*` |
| T06 | 2FA TOTP, kody odzyskiwania, wyzwanie przy logowaniu; zaufane urządzenia (lista, usuwanie); rezerwowe 2FA kodem z e-maila | T04 | `crates/identity/src/mfa*` |
| T07 | Passkeys (WebAuthn) | T04 | `crates/identity/src/passkeys*` |
| T08 | Sesje web: lista, unieważnienie jednej i wszystkich, historia logowań, e-mail o nowym urządzeniu | T04, T36 | `crates/identity/src/sessions*` |
| T08b | Zmiana e-maila: potwierdzenie na nowy adres, link cofnięcia na stary, zmiana od razu z kluczem odzyskiwania albo po 7 dniach, przycisk „to nie ja” na starym adresie, link cofnięcia przez 30 dni, po zmianie 72 h blokady transferu coinów, bazaru i kolejnej zmiany e-maila (D14); zapasowy e-mail z potwierdzeniem | T04, T36 | `crates/identity/src/email_change*` |
| T08c | Zmiana hasła i tryb sudo; po zmianie poświadczeń unieważnienie sesji web, rodzin OAuth i sesji urządzeń | T04, T16, T21 | `crates/identity/src/credentials*` |
| T09 | Eksport danych (RODO) | T08 | `crates/identity/src/export*` |
| T09b | Usunięcie konta: karencja 1–90 dni (domyślnie 14), anulowanie, finalizacja w workerze; przekazanie do gry po G-6 | T08c, T37 | `crates/identity/src/termination*` |
| T09c | Prywatność konta (powiązanie postaci z kontem, widoczność statusu) i język | T08 | `crates/identity/src/privacy*` |
| T10 | RBAC i audyt; test, że każde uprawnienie ma wbudowaną rolę | T06 | `crates/admin/src/rbac*`, `crates/admin/src/audit*` |
| T10b | Szkielet panelu zespołu: host admin, obowiązkowe MFA, nawigacja według uprawnień, edycja ról | T10 | `crates/admin/src/web*`, `templates/admin/` |
| T11 | Newsy: CRUD z usuwaniem, tłumaczenia pl/en, strony publiczne | T10b | `crates/content/src/news*` |
| T11b | Strony i strony redakcyjne (zasady, regulamin, prywatność, cookies, informacje o serwerze, zgłoś błąd) | T10b | `crates/content/src/pages*` |
| T11c | Wydarzenia ze statusem publikacji | T10b | `crates/content/src/events*` |
| T11d | Ogłoszenia (pasek na stronie) i strona `/today` | T11, T11c | `crates/content/src/announcements*`, `crates/content/src/today*` |
| T12 | Media: upload, warianty obrazów, walidacja | T11 | `crates/content/src/media*` |
| T13 | Wiki: poradniki redakcji, wersje, przegląd, markdown i sanityzacja, wyszukiwanie (PG full text) | T10b | `crates/wiki` |
| T14 | `world`: odczyt `content/**` JSON z przypiętego commita gry, walidacja schematami gry, wersje, atomowe przełączenie, lista dopuszczonych encji | T00 | `crates/world` |
| T14b | Strony wiki generowane z `world`: listy per typ, strona encji, filtry, wyszukiwanie, opisy redakcji | T13, T14 | `crates/wiki/src/entities*`, `templates/wiki/` |
| T14c | Panel świata gry: wybór wersji, diff, wyniki walidacji, lista dopuszczonych encji, nazwy pl/en | T14, T10b | `crates/world/src/admin*`, `templates/admin/world/` |
| T15 | Guard mTLS `/internal/v1` (tożsamość per grupa), kanoniczny JSON, harness testów na wektorach golden z Game | T00 | `crates/internal-api` (wspólne) |
| T16 | OAuth PKCE (`/oauth/authorize`, `/oauth/token` w korzeniu base URL, consent); odpowiedź `authorization_code` zgodna z klientem; refresh z rotacją jako nowe zachowanie | T04 | `crates/gameauth/src/oauth*` |
| T17 | `POST /api/v1/game-auth/tickets` i `GET /api/v1/game-auth/native-characters` (kształty z §5.2, wektory golden); zawieszone konto nie dostaje ticketu | T16, T18 | `crates/gameauth/src/tickets*`, `crates/player-api` |
| T18 | Konsument LCFA: `native-account-characters` i watermark (snapshot ≤64 postaci, epoka i rewizja) | T15 | `crates/characters/src/lcfa*` |
| T19 | `native-admissions` dla gatewaya logowania (token usługi, bieżący i poprzedni hash); odmowa dla zawieszonych kont | T15 | `crates/gameauth/src/admission*` |
| T19b | `native-evidence` i `native-runtime-status` | T15 | `crates/gameauth/src/evidence*`, `crates/gameauth/src/runtime*` |
| T19c | Klucze Ed25519 grantów i ich rotacja (bez zmian z FND-04B-AMEND-1) | T19 | `crates/gameauth/src/keys*` |
| T20 | Scope assignments i revocations | T19 | `crates/gameauth/src/scopes*` |
| T21 | Sesje urządzeń (enroll, characters, tickets, revoke); kontrakt po stronie Platform | T17 | `crates/gameauth/src/devices*` |
| T22 | `character-commands/{pending, read, receipt}` (CreateCharacter, TransferCharacterOwnership) | T18 | `crates/characters/src/commands*` |
| T22b | `character-bootstrap-intents/read` (v3) | T18 | `crates/characters/src/bootstrap*` |
| T22c | Tworzenie postaci z konta: formularz, reguły nazw i lista zastrzeżonych, sugestia nazwy, status operacji | T22, T44 | `crates/characters/src/create_web*` |
| T23 | `/internal/v1/game-auth/game-restore/notice` i rekoncyliacja | T22 | `crates/characters/src/restore*` |
| T24 | Ingest projekcji publicznych: wspólny handler rodzin, watermark, migawki | T15 | `crates/community/src/ingest*` |
| T24a | Rankingi: kategorie i filtr profesji | T24, T01 | `crates/community/src/rankings*` |
| T24b | Profil postaci i wyszukiwarka, z ustawieniami z T45 i T09c | T24, T45 | `crates/community/src/profiles*` |
| T24c | Lista online | T24 | `crates/community/src/online*` |
| T24d | Zgony (po akceptacji rodziny w G-4) | T24 | `crates/community/src/deaths*` |
| T24e | Gildie: lista i strona (po akceptacji rodziny w G-4) | T24 | `crates/community/src/guilds*` |
| T24f | Domy: lista i strona (po akceptacji rodziny w G-4) | T24 | `crates/community/src/houses*` |
| T25 | Status światów: strona i `GET /api/v1/worlds` | T24 | `crates/community/src/worlds*` |
| T26 | Ledger portfela, saldo (dostępne, przenoszalne, zarezerwowane), historia, transfer coinów (blokady, idempotencja) | T04 | `crates/wallet` |
| T27 | Ledger premium i `premium-snapshots/read` (≤256 B) | T15, T26 | `crates/store/src/premium*` |
| T27b | `loyalty-snapshots/read` | T27 | `crates/store/src/loyalty*` |
| T28 | Import i aktywacja cennika sklepu (S-5, `catalog_revision` = SHA-256) | T10 | `crates/store/src/catalog*` |
| T29 | Zakup, saldo i historia przez wire sklepu | T26, T28, T15 | `crates/store/src/purchase*` |
| T30 | Dostawa: operacje pending, read, receipt, reconcile | T29 | `crates/store/src/delivery*` |
| T31 | Płatności: trait dostawcy, provider testowy, webhooki z idempotencją, doładowanie coinów | T26 | `crates/payments` |
| T32 | Sklep web: oferty, doładowanie, saldo i historia, transfer | T29, T31 | `crates/store/src/web*`, szablony |
| T33 | Zgłoszenia supportu: kategorie, limit otwartych, odpowiedzi, status, powiadomienia | T10b, T36 | `crates/support/src/tickets*` |
| T33b | Załączniki do zgłoszeń (zrzuty ekranu) przez potok uploadu z T12 | T33, T12 | `crates/support/src/attachments*` |
| T33c | Zgłaszanie graczy, treści i gildii | T33 | `crates/support/src/reports*` |
| T34 | Kary: ostrzeżenia, ograniczenia, zawieszenia, cztery oczy; zawieszenie blokuje ticket i admission; kary w grze po G-5 | T33, T19 | `crates/support/src/sanctions*` |
| T34b | Historia kar gracza: potwierdzenie, odwołanie, decyzja | T34 | `crates/support/src/appeals*` |
| T35 | Pobieranie, kanały TUF i polityka aktualizatora | T11 | `crates/downloads` |
| T36 | Powiadomienia: e-mail przez outbox (szablony pl/en), w kliencie, preferencje per kategoria, wypisanie | T02, T37 | `crates/notify` |
| T36b | Player API: `GET /api/v1/account/summary`, `GET /api/v1/feed`, `GET /v1/client/directory` (po G-1) | T11, T26, T36 | `crates/player-api` |
| T37 | Worker: outbox, ponowienia, sprzątanie (sesje, tokeny, retencja supportu) | T00 | `bin/oteryn-portal/src/worker*`, `crates/db/src/outbox*` |
| T38 | Import: konta i bezpieczeństwo (hashe, TOTP, kody i klucze odzyskiwania, prywatność, zaplanowane usunięcia), role | T03–T10 | `bin/oteryn-portal/src/import/identity*` |
| T38b | Import: treści, wiki, media | T11–T13 | `bin/oteryn-portal/src/import/content*` |
| T38c | Import: portfel, premium, płatności; kontrola braku otwartych aukcji | T26–T31 | `bin/oteryn-portal/src/import/commerce*` |
| T38d | Import: zgłoszenia, zgłoszenia graczy, kary, odwołania | T33–T34b | `bin/oteryn-portal/src/import/support*` |
| T38e | Import: ustawienia profili postaci | T45 | `bin/oteryn-portal/src/import/characters*` |
| T39 | Projekt graficzny od nowa: tokeny CSS, typografia, ikony, komponenty i galeria komponentów | T01 | `static/`, `templates/components/` |
| T39b | Makiety kluczowych ekranów na tokenach T39 do akceptacji właściciela (§8) | T39 | `docs/design/`, `templates/mockups/` |
| T39b | Strona główna i „Jak zacząć” | T39, T11 | `templates/pages/` |
| T40 | Atlas: kafle WebP z minimapy gry (LZMA → BMP → piramida) i znaczniki z `npc-placements` | T14 | `crates/atlas/src/tiles*` |
| T41 | Atlas: strona `/atlas` (przeglądarka kafli, warstwy, znaczniki, linki do wiki) | T40, T14 | `crates/atlas/src/web*`, `templates/atlas/`, `static/atlas/` |
| T42 | Miniatury z `appearances-*.dat` i `sprites-*.bmp.lzma` do wiki i znaczników | T14 | `crates/world/src/sprites*` |
| T43 | SEO i przejście: sitemap, robots, `hreflang`, przekierowania 301 ze starych adresów | T01 | `crates/web/src/seo*`, `crates/web/src/redirects*` |
| T44 | Przegląd konta `/account`: postacie (poziom, profesja, świat, link do profilu), stan powiązania z grą, limit postaci; data założenia, ostatnie logowanie w grze i na stronie, status konta, saldo, lojalność; skróty do bezpieczeństwa, premium i salda | T18, T08 | `crates/characters/src/overview*`, `templates/account/` |
| T45 | Ustawienia profilu postaci: postać główna, ukrycie, komentarz, 7 przełączników widoczności | T44 | `crates/characters/src/profile*` |
| T46 | Zarządzanie postacią: usunięcie z przywróceniem przez 60 dni, lista usług gotowych do użycia, zmiana nazwy, płci i świata; usługi płatne coinami (D10): obciążenie portfela razem z komendą do gry, zwrot przy odrzuconym potwierdzeniu; każda akcja za flagą do czasu G-3 | T22, T26, G-3 | `crates/characters/src/manage*` |
| T47 | Premium i płatności w panelu: status i data końca premium, historia płatności, rachunki | T27, T31 | `crates/store/src/account*`, `crates/payments/src/account*` |
| T48 | Wyszukiwarka portalu: newsy, strony, wydarzenia, wiki, encje świata, postacie | T11, T14b, T24b | `crates/web/src/search*` |
| T49 | Panel zespołu, gracze: wyszukiwanie, konto, sesje, postacie, wylogowanie wszędzie, notatki, reset MFA | T10b, T08 | `crates/admin/src/players*` |
| T50 | Panel zespołu, handel: premium, korekta salda, rekoncyliacja płatności, zwroty | T10b, T26, T27, T31 | `crates/admin/src/commerce*` |
| T51 | Panel zespołu, support i moderacja: zgłoszenia z notatkami wewnętrznymi, zgłoszenia graczy, kary, odwołania | T10b, T33c, T34b | `crates/admin/src/support*` |
| T52 | Rola `gateway`: `POST /v1/login` (protokół 2, 11 kodów błędów, limiter), `/health`, `/ready`, `/version`; wektory golden z testów gatewaya Go i fixtures klienta gry (`platform-client/src/native_login/tests.rs`, przez koordynatora), zgodność bajt w bajt; bez konfiguracji bazy | T00 | `crates/gateway`, `bin/oteryn-portal/src/gateway*` |
| T53 | Kreator bezpieczeństwa: ocena konta i lista brakujących kroków | T05b, T06, T08b | `crates/identity/src/security_wizard*` |
| T54 | Odznaki konta, wybór do 5 osiągnięć na profilu, postać do rankingu lojalności | T45, T47 | `crates/characters/src/badges*` |
| T55 | Kody prezentowe i kody z gry: generowanie (admin), realizacja jednorazowa i idempotentna, przypisanie do postaci | T26, T27 | `crates/store/src/codes*` |
| T56 | Polecenia znajomych: zaproszenia, powrót gracza, nagrody i statystyki | T36, T55 | `crates/community/src/referrals*` |
| T57 | Bazar, rdzeń: wystawienia w coinach lub złocie, oferty, licytacja z ofertą maksymalną, ochrona przed snipingiem, stany; rezerwy w portfelu; rozliczenie z prowizją (bez I/O gry) | T26 | `crates/bazaar/src/domain*`, migracja |
| T58 | Bazar, gra (G-7): blokada i snapshot postaci, depozyt przedmiotów, blokada i przekazanie złota, przeniesienie i dostawa przez outbox, rekoncyliacja | T57, T22 | `crates/bazaar/src/game*`, trasy bazaru w `crates/internal-api` |
| T59 | Bazar, postacie: wystawienie, karta, szczegóły z zakładkami ze snapshotu, licytacja | T57, T58, T39 | `crates/bazaar/src/characters*`, `templates/bazaar/` |
| T60 | Bazar, przedmioty: ogłoszenia (cena stała, przyjmuję oferty, kupię), oferty i ich przyjęcie, licytacje przedmiotów | T57, T58, T14b | `crates/bazaar/src/items*`, `templates/bazaar/` |
| T61 | Bazar: wyszukiwarka, filtry, sortowanie, zakładki, obserwowane, zapisane wyszukiwania | T59, T60 | `crates/bazaar/src/search*` |
| T62 | Bazar: powiadomienia (przebicie, nowa oferta, koniec, wygrana, pasujące wyszukiwanie) | T61, T36 | `crates/bazaar/src/notify*` |
| T63 | Bazar: historia cen, cena rynkowa, statystyki, szacowanie wartości postaci, porównanie postaci | T59, T60 | `crates/bazaar/src/stats*` |
| T64 | Bazar: profil sprzedawcy, zgłoszenia ofert, wyróżnienie; panel zespołu (moderacja, wstrzymanie, unieważnienie z czterema oczami, opłaty, lista zbywalnych przedmiotów) | T59, T60, T10b, T33c | `crates/bazaar/src/moderation*`, `crates/admin/src/bazaar*` |
| T65 | Statystyki, rdzeń: dzienne migawki postaci z ingestu projekcji, zdarzenia (nazwa, świat, gildia, bazar, zgon, awans), próbki online, agregaty, partycje i retencja | T24 | `crates/stats/src/store*`, migracja |
| T66 | Statystyki postaci: zakładki historii, doświadczenia, czasu online, rankingów i zgonów; postęp gry; przewidywanie poziomu; porównanie postaci | T65, T24b | `crates/stats/src/character*`, `templates/stats/` |
| T67 | Rankingi przyrostu: doświadczenie, czas online, najstarsze postacie, zmiany nazw, transfery, sprzedane postacie; filtry | T65, T24a | `crates/stats/src/rankings*` |
| T68 | Statystyki gildii: lista z miarami, historia członków, przyrost, ranking, porównanie | T65, T24e | `crates/stats/src/guilds*` |
| T69 | Statystyki świata: wykres online, spis, zgony dziennie, potwory i bossowie (po G-8) | T65, T25 | `crates/stats/src/world*` |
| T70 | Obserwowanie postaci i gildii z powiadomieniami, z poszanowaniem prywatności | T66, T36 | `crates/stats/src/watch*` |
| T71 | Kalkulatory z danych świata: doświadczenie do poziomu, stamina, trening, podział łupu | T14 | `crates/wiki/src/calculators*` |
| T72 | Dziennik gry, rdzeń: crate `playerdata`, ingest rodzin prywatnych (G-9…G-11) z idempotencją i watermarkiem, dostęp tylko właściciela (testy odmowy dla innych kont i zespołu bez uprawnienia), granica sprzedaży postaci, retencja, flagi | T15, T24, T44 | `crates/playerdata/src/store*`, `crates/internal-api/src/playerdata*`, migracja |
| T73 | Analizator polowań: lista sesji z filtrami, szczegóły (exp/h, łup, zużycie, bilans, obrażenia, leczenie, zabici), porównania, podział łupu w drużynie, link udostępnienia | T72, T74 | `crates/playerdata/src/hunts*`, `templates/playerdata/` |
| T74 | Wycena łupu i zapasów: ceny NPC z danych świata, rynkowa z bazaru, własne ceny gracza | T72, T14 | `crates/playerdata/src/pricing*` |
| T75 | Dziennik dropów, rekordy osobiste, kronika postaci | T73 | `crates/playerdata/src/drops*` |
| T76 | Przeglądarka przedmiotów: ekwipunek, depoty, skrzynka, dom; „gdzie mam”; wartość majątku; skrót do bazaru; historia banku, marketu i wymian | T72, T74, T14b | `crates/playerdata/src/items*` |
| T77 | Postęp i cele: bestiariusz, bossowie z cooldownem, questy, osiągnięcia, historia umiejętności, cele z prognozą | T72, T65 | `crates/playerdata/src/progress*` |
| T78 | Recap zgonu i PvP: źródła obrażeń, straty, błogosławieństwa, zabójstwa, asysty, skull | T72 | `crates/playerdata/src/deaths*` |
| T79 | Pulpit konta i przypomnienia (czynsz, premium, bossowie, stamina, bazar) przez e-mail i web push | T72, T36 | `crates/playerdata/src/dashboard*`, `crates/playerdata/src/reminders*` |
| T80 | Historia logowań do gry z „to nie ja” (wylogowanie wszędzie, zmiana hasła) | T72, T08 | `crates/playerdata/src/logins*` |
| T81 | Mapa łowisk: dzienne agregaty z progiem 10 sesji i 5 kont, opóźnienie 24 h, wyłączenie udziału, warstwa Atlasu | T73, T41 | `crates/playerdata/src/places*`, `static/atlas/` |
| T82 | Raport tygodniowy, notatki do sesji i postaci, eksport CSV i JSON | T73, T36, T09 | `crates/playerdata/src/reports*` |
| T83 | Szukam drużyny: ogłoszenia, filtry, wygasanie, zgłoszenia, moderacja | T44, T33c, T10b | `crates/community/src/party*` |
| T84 | Atlas: warstwa spawnów ze skupieniami, link do miejsca, prywatne znaczniki gracza, zgłoszenie błędu na mapie | T41, T33c | `crates/atlas/src/spawns*`, `static/atlas/` |
| T85 | Mapa dostaw zadań i „gdzie wypada”: przedmiot → potwory → spawny, tiery źródeł, estymator zabójstw (P50, P80, P95) przeniesiony z Oteryn-Atlas z jego testami, filtry, linki z wiki | T84, T14b | `crates/atlas/src/delivery*` |
| T86 | Bestiariusz i bossowie na mapie: postęp postaci z dziennika, stany spawnów, punkty, rekomendacje | T84, T77 | `crates/atlas/src/bestiary*` |
| T87 | Miejsca polowań, na bestiariusz i bossów: karty z danymi z mapy łowisk, opisy redakcji, raporty graczy | T81, T13 | `crates/wiki/src/places*` |
| T88 | „Dziś na świecie”: wzmocnienia i ich historia, rajdy (po G-12) | T24 | `crates/community/src/today*` |
| T89 | Przepustka sezonowa: sezony, wyzwania z dziennika gry i portalu, ścieżki darmowa i płatna, odbiór nagród przez wire sklepu | T72, T26, T29 | `crates/minigames/src/pass*` |
| T90 | Kolekcja potworów: karty z danych świata i postępu bestiariusza, talie, walki turowe PvE | T14, T42, T77 | `crates/minigames/src/collection*` |
| T91 | Kolekcja potworów: PvP, rankingi, turnieje, zestawy o znanej zawartości | T90 | `crates/minigames/src/collection_pvp*` |
| T92 | Pomocnik w domu: zakup, zawody, wybór wyprawy z jawną tabelą nagród, stan i historia, limity, powiadomienia, komendy do gry przez outbox (po G-14) | T26, T29, T36 | `crates/hirelings` |
| T93 | Codzienne zagadki: zgadnij potwora, „Gdzie to jest?” na kaflach Atlasu, quiz; serie i odznaki | T14, T40, T42 | `crates/minigames/src/puzzles*` |
| T94 | Rozgrywki gildii w wyzwaniach sezonu i typer wydarzeń bez stawek | T89, T88 | `crates/minigames/src/league*` |
| G-1 | (gra) Kontrakt nowych tras Player API i `client/directory` (wpis w locku, `world.characters` vs LCFA), kierunek `native-admissions`, aktualizacja locka (N4, CharCmd) | – | Oteryn-Game, przez koordynatora |
| G-2 | (gra) Pole widoczności (public/hidden) w schematach questów, NPC i obszarów | – | Oteryn-Game, przez koordynatora |
| G-3 | (gra) Komendy postaci: usunięcie i anulowanie, zmiana nazwy, płci i świata (rozszerzenie GAME-CHAR-CMD; płatne usługi rozlicza Platform) | – | Oteryn-Game, przez koordynatora |
| G-4 | (gra) Rodziny publiczne: zgony z zabójcą, gildie, domy, osiągnięcia, umiejętności, ekwipunek i outfit, ostatnie logowanie i czas online (D11) | – | Oteryn-Game, przez koordynatora |
| G-5 | (gra) Wykonywanie kar w grze (po D9) | – | Oteryn-Game, przez koordynatora |
| G-6 | (gra) Usunięcie konta: los postaci i projekcji | – | Oteryn-Game, przez koordynatora |
| G-7 | (gra) Bazar: blokada i podpisany snapshot postaci z hashem, warunki wystawienia, zbywalność przedmiotów, depozyt przedmiotów z atrybutami, dostawa do kupującego i zwrot do sprzedawcy; blokada złota kupującego (oferta, rezerwa), przekazanie złota sprzedawcy i zwolnienie blokady; przeniesienie postaci przez istniejące TransferCharacterOwnership | – | Oteryn-Game, przez koordynatora |
| G-8 | (gra) Statystyki: sesje online per postać (start, koniec) oraz dzienne liczniki zabitych potworów i bossów per świat, jako rodziny publiczne | – | Oteryn-Game, przez koordynatora |
| G-9 | (gra) Dziennik gry, sesje: prywatny kanał per konto; podsumowanie po zakończeniu sesji i co godzinę (czas, miejsca, doświadczenie, łup i zużycie per przedmiot, obrażenia zadane i otrzymane per typ i źródło, leczenie, zabite potwory, drużyna i jej łup); rzadkie dropy; idempotencja po id sesji i odcinka | – | Oteryn-Game, przez koordynatora |
| G-10 | (gra) Dziennik gry, przedmioty: snapshot ekwipunku, depotów, skrzynki i domu przy wylogowaniu, tylko do odczytu; historia banku, marketu i wymian | – | Oteryn-Game, przez koordynatora |
| G-11 | (gra) Dziennik gry, postęp i zdarzenia: bestiariusz, cooldowny bossów, questy, osiągnięcia, umiejętności; recap zgonu; zabójstwa PvP i skull; stamina, koniec premium, czynsz domu; historia logowań do gry | – | Oteryn-Game, przez koordynatora |
| G-12 | (gra) Dzienne dane świata jako rodzina publiczna: wzmocniony potwór i boss dnia, rajdy z harmonogramu | – | Oteryn-Game, przez koordynatora |
| G-13 | (gra) Klucze przedmiotów (`oteryn:item.*`) w celach i nagrodach kroków questów zamiast nazw | – | Oteryn-Game, przez koordynatora |
| G-14 | (gra) Pomocnik w domu jako rozszerzenie istniejącego hirelinga (0425, 0610): zawody i poziom, wyprawy liczone przez grę z jawnej tabeli nagród i tygodniowym limitem, skrzynia pomocnika, rozmowa w grze; komendy z portalu (start, odbiór) z kluczem idempotencji i receipt; zakup jako uprawnienie ze sklepu | – | Oteryn-Game, przez koordynatora |

Kolejność startu:

1. T00 jest gotowy w PR #1685 (kompilujący się szkielet i job CI).
2. Potem jednocześnie: T01, T14, T15, T37, T39 i T52.
3. Potem równolegle gałęzie: konta (T02–T09c, T44–T45, T53–T54), gry (T16–T25), handlu (T26–T32, T47, T55–T56), bazaru (T57–T64), statystyk (T65–T71), dziennika gry (T72–T83), Atlasu i narzędzi (T84–T88) i panelu zespołu (T10–T10b, T49–T51).
4. Zadania zależne od G-3…G-14 powstają od razu, ale ich funkcje zostają wyłączone flagą, dopóki gra nie dostarczy kontraktu.
5. Po pierwszym wydaniu: rozrywka (T89–T94, D20).

### 10.1 Szablon zadania dla Codexa

Sesja Platform wysyła każde zadanie w tej postaci:

```
Repo: Oteryn/Oteryn-Platform, gałąź z main: codex/portal-rs-<ID>
Zadanie <ID>: <nazwa z tabeli>
Projekt: docs/architecture/OTERYN_PORTAL_RUST_PLAN.md, sekcje <§>
Dozwolone ścieżki: <ścieżki z tabeli>; wspólne pliki tylko przez dopisanie linii
Kontrakt: <plik kontraktu i wektory golden, jeśli dotyczy>
Akceptacja:
  - cd portal-rs && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
  - testy pokrywają ścieżkę sukcesu, odmowę i błąd wejścia
  - każda funkcja gry lub pieniędzy za flagą, domyślnie wyłączona
  - strony: działają na telefonie (szerokość 360 px) i spełniają WCAG 2.2 AA (kontrast, klawiatura, etykiety)
Zakazy: brak zmian poza dozwolonymi ścieżkami, brak .github/workflows, brak sekretów, brak unwrap/expect/panic poza testami
Wynik: PR draft z wynikiem komend akceptacji w opisie
```

Pozostałe warunki:

- **CI.** Job `Portal RS CI` (`.github/workflows/portal-rs-ci.yml`, zgoda właściciela 2026-11-05) uruchamia `cargo fmt --check`, `cargo clippy -D warnings` i `cargo test --locked` dla zmian w `portal-rs/**`. Dodaje go PR #1685 razem ze szkieletem.
- **Zakres plików.** Zadania mają rozłączne ścieżki. Wspólne pliki (`Cargo.toml` workspace, router w `web`, migracje) zmienia się tylko przez dopisanie linii, a konflikty rozwiązuje sesja Platform przy merge.

## 11. Decyzje właściciela

| # | Decyzja | Domyślnie |
| --- | --- | --- |
| D1 | Zakres pierwszego wydania | Sekcja 2 bez węzłów i obserwacji GM; bazar (D13) i dziennik gry z analizatorem (D17) w zakresie |
| D2 | Nowy job CI dla `portal-rs/` | **Zgoda** (2026-11-05), PR #1685 |
| D3 | Dostawca płatności | **Zgoda** (2026-11-05): dostawca testowy do czasu wyboru (np. Stripe); prawdziwy wybrany przed pierwszymi wpłatami |
| D4 | Wygląd | **Od nowa** (2026-11-05): obecny portal jest tylko przykładem; system projektowy (T39) i makiety do akceptacji właściciela (T39b), §8 |
| D5 | Gateway logowania | **Zgoda** (2026-11-05): przepisany do Rusta jako rola `gateway` w tej samej binarce, osobny proces bez dostępu do bazy, kontrakt `POST /v1/login` bez zmian. Koordynator potwierdził przepisanie 1:1; kolejka i kod przerwy technicznej dopiero później, jako zmiana kontraktu (§6) |
| D6 | Import danych | Konta z bezpieczeństwem, role, treści, wiki, portfel, premium, support i kary, ustawienia profili (§9); sesje i tokeny w locie nie |
| D7 | Publikacja sprite'ów i mapy gry na stronie (licencja `LICENSE-ASSETS.md`) | **Wolno** (2026-11-05): wiki z grafikami, Atlas włączony |
| D8 | Klasyfikacje questów publikowane w wiki | **Wszystkie** (2026-11-05); wiki pokazuje klasyfikację przy queście |
| D9 | Autorytet kar | **Zgoda** (2026-11-05): Platform; zawieszenie konta przez odmowę logowania do gry; kary w grze po G-5 |
| D10 | Zmiana płci, świata i nazwy postaci | **Płatne** (2026-11-05), jak w Tibii: zmiana płci to kosmetyka, transfer świata płatny. Dziś jest jeden świat, więc transfer czeka na drugi. Zmiana nazwy też płatna |
| D11 | Ostatnie logowanie i czas online na profilu | **Tak** (2026-11-05), z przełącznikiem prywatności gracza; wymaga danych z gry (G-4) |
| D12 | Kolejność zadań gry G-3…G-6 | **G-4, G-3, G-6, G-5** (2026-11-05) |
| D13 | Bazar | **Tak** (2026-11-05): nazwa „Bazar”; postacie na licytacji, cenne przedmioty w ogłoszeniu albo na licytacji; za coiny albo złoto z gry; wystawienie darmowe; prowizja tylko od sprzedaży: 10% przy coinach, 2,5% przy złocie, bez górnego limitu (inaczej niż Market Tibii); licytuje tylko gracz z pełną kwotą oferty (§5.6). G-7 trafia do kolejki gry po G-5, a G-8 po G-7; prowizja w złocie jest spalana w grze |
| D14 | Zmiana e-maila (było Q8) | **Zgoda** (2026-11-05): z kluczem odzyskiwania od razu; bez klucza hasło, 2FA i potwierdzenie nowego adresu, zmiana po 7 dniach; stary adres dostaje od razu „to nie ja” (anuluje) i link cofnięcia ważny 30 dni; po zmianie wylogowanie wszędzie i 72 h blokady transferu coinów, bazaru i kolejnej zmiany e-maila |
| D15 | Parametry bazaru (było Q10) | **Zgoda** (2026-11-05): licytacja 1–14 dni; ogłoszenie 14 dni z odnowieniem; „kup teraz” do pierwszej oferty; przebicie min. 1% (≥1 coin lub 1 000 złota); oferta w ostatnich 5 min przedłuża o 5 min; ponowna sprzedaż kupionej postaci po 30 dniach; ostrzeżenie przy cenie ±25% od rynkowej |
| D16 | Statystyki graczy, gildii i świata (wzór GuildStats) | **Tak** (2026-11-05), §5.7 |
| D17 | Prywatny dziennik gry z automatycznym analizatorem polowań | **Tak** (2026-11-05), §5.8; najpierw analizator i przeglądarka przedmiotów |
| D18 | Atlas z mapą dostaw zadań i trackerami (wzór TibiaRoute) | **Tak** (2026-11-05), §5.9 |
| D19 | Repozytorium Oteryn-Atlas (było Q11) | **Zgoda** (2026-11-05): Atlas w portalu je zastępuje. Repo zostaje zamrożone i trafia do archiwum po uruchomieniu Atlasu portalu (T41, T84–T86); archiwizuje je właściciel |
| D20 | Rozrywka bez hazardu | **Tak** (2026-11-05), §5.10: pomocnik w domu zamiast wysyłania postaci, przepustka sezonowa, kolekcja potworów, zagadki; bez gier losowych o wartość; po pierwszym wydaniu |
| D21 | Rodzaj COSMETIC w sklepie (wyglądy, ramki, tytuły) | **Zgoda** (2026-11-05) na włączenie, wcześniej wyłączony; zmiana kontraktu wire sklepu z grą przed T89 |
| D22 | Akceptacja projektu (było Q5) | **Akceptuję** (2026-11-05); start od T00 (PR #1685), potem kolejność z §10 |
| D23 | Konsultacja prawna (bazar, sklep i coiny, regulamin i RODO, §5.10) | **Nie blokuje prac** (2026-11-05); właściciel zrobi ją później |
| D24 | Wersja PostgreSQL | **18.6** (2026-11-05), jak w grze i w środowisku Codexa. Stara baza Laravela zostaje na 17.11 do wyłączenia, bez aktualizacji; dane przenosi import (T38) |

## 12. Otwarte pytania

| # | Pytanie | Do kogo | Blokuje |
| --- | --- | --- | --- |
| Q2 | Kontrakt `client/directory` w locku gry, z regułą postaci tylko dla właściciela (§5.2) | Koordynator, G-1 | Tylko pole `characters` w T36 |
| Q3 | Aktualizacja locka gry: N4P (z `native-admissions`) i GAME-CHAR-CMD-0 (§6) | Koordynator, G-1 | Nic; portal implementuje według kodu |
| Q4 | Decyzje właściciela w FND-04B-AMEND-1 (§6) | Właściciel | Zmiany grantów w T19 |

Pozostałe zadania nie czekają na te odpowiedzi.

## 13. Źródła

- Obecny portal: trasy `routes/internal.php`, `routes/api.php`, `routes/modules/*`; moduły `app/*`.
- Kontrakty: `docs/contracts/`, w tym wire sklepu i decyzje z 2026-11-05.
- Gra (main @60de8634): `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`, `crates/platform-client` (`lib.rs`, `native_login.rs`, `native_login/owner_characters.rs`), `crates/gameplay/native-admission-source` (`account_characters.rs`, `character_commands.rs`, `store.rs`, `restore_notice.rs`, `public_projections.rs`), `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md`.
- Przegląd od strony gry: koordynator, 2026-11-05; stan panelu gracza po stronie gry: Game main @00c01066.
- Inwentarz obecnego portalu i niezależny przegląd kompletności panelu gracza, 2026-11-05.
- Przegląd panelu konta Tibii, Char Bazaar Tibii, Exevopan, TibiaTrade i GuildStats, 2026-11-05.
- `Oteryn/Oteryn-Atlas` (main @0bbd188), pierwowzór Atlasu, 2026-11-05.
- TibiaRoute (mapa dostaw zadań, trackery bestiariusza i bossów, miejsca, kalkulatory), 2026-11-05.
- Analizatory w kliencie Tibii (sesja, łup, zużycie, obrażenia, doświadczenie) jako wzór dla §5.8.
- Szkic #1643: źródło pomysłów dla rozszerzeń (węzły, Atlas, obserwacja GM).
