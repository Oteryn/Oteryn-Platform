# Oteryn Portal w Ruście – projekt

> **Status: PROJEKT, rewizja 4** (2026-11-05). Zastępuje rewizje 1–3. Rewizja 4 dodaje przegląd od strony gry i upraszcza wiki oraz Atlas: portal czyta dane świata gry bez nowego formatu eksportu.
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
- **Gra jest osobna.** Brak wspólnej bazy z grą. Dane na żywo idą tylko przez kontrakty HTTP z mTLS, a kontrakty, które gra już implementuje, zostają bajt w bajt (sekcje 5 i 6). Dane świata portal czyta tylko do odczytu z przypiętego wydania gry (§6.1).
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
| Gracz | Wiedza o świecie | Wiki generowana z treści świata gry (przedmioty, potwory, NPC, czary, questy, łupy, miejsca) i uzupełniana przez redakcję; Atlas, czyli interaktywna mapa świata z warstwami i znacznikami; wyszukiwarka |
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
| Testy | `cargo test`, testy integracyjne na prawdziwym PostgreSQL, testy kontraktów na wektorach golden z `docs/contracts/` i z Game |

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
    world/                   odczyt danych świata z przypiętego wydania gry (encje, relacje, mapa, sprite'y)
    wiki/                    strony encji generowane z `world`, poradniki redakcji, wyszukiwanie
    atlas/                   kafle mapy z `world`, znaczniki, przeglądarka mapy
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
| Domena portalu (base URL klienta) | Strona, konto, sklep, `/oauth`, `/v1/client/directory`, `/api/v1` | Publiczny, za Cloudflare |
| `internal` (osobny port) | `/internal/v1` | Tylko mTLS, allowlista DN, sieć prywatna |
| `admin` (osobny host) | Panel zespołu | Cloudflare Access i obowiązkowe MFA |

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
| identity | `accounts` (id, email, email_verified_at, password_hash, status, locale), `account_mfa_totp`, `account_passkeys`, `account_recovery_codes`, `account_email_tokens`, `account_password_resets`, `web_sessions` (hash tokenu, konto, urządzenie, ip, wygasa, sudo_until), `security_events` |
| gameauth | `oauth_clients`, `oauth_authorization_codes`, `oauth_tokens` (access i refresh jako hash, rodzina rotacji), `device_sessions`, `game_login_tickets` (jednorazowe, krótkie), `native_admissions`, `native_evidence`, `signing_keys` (Ed25519, rotacja), `scope_assignments` |
| characters | `account_characters` (projekcja LCFA z watermarkiem), `character_create_intents` (outbox do gry), `character_profiles` (preferencje publiczne) |
| content | `news`, `pages`, `events`, `announcements`, `media` (z wariantami), `translations` |
| world | `world_versions` (commit gry, data, aktywna), `world_entities` (rodzina, klucz, dane `jsonb`, wersja), `world_links` (potwór → łup, NPC → quest, przedmiot → miejsce), `world_visibility` (encje dopuszczone przez admina do czasu flagi w grze) |
| wiki | `wiki_guides`, `wiki_revisions`, `wiki_reviews` (tylko teksty redakcji: poradniki i opisy dopisane do encji) |
| atlas | `atlas_markers` (wyliczane z `world`: miasta, NPC, spawny, questy); kafle mapy jako pliki statyczne per wersja świata |
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
| Publiczne | `/`, `/start`, `/news`, `/news/{slug}`, `/events`, `/pages/{slug}`, `/download`, `/worlds`, `/rankings/{kind}`, `/characters/{name}`, `/guilds/{name}`, `/houses`, `/deaths`, `/online` |
| Świat gry | `/wiki`, `/wiki/{kind}` (np. `items`, `creatures`, `npcs`, `spells`, `quests`), `/wiki/{kind}/{key}`, `/wiki/articles/{slug}`, `/wiki/search`, `/atlas`, `/atlas/{floor}/{x}/{y}`, `/atlas/tiles/{version}/{floor}/{z}/{x}/{y}.webp` |
| Konto | `/register`, `/login`, `/logout`, `/verify-email`, `/forgot-password`, `/reset-password`, `/account`, `/account/security` (hasło, 2FA, passkeys, kody), `/account/sessions`, `/account/devices`, `/account/characters`, `/account/privacy` (eksport, usunięcie) |
| Sklep | `/shop`, `/shop/coins` (doładowanie), `/shop/history`, `/shop/transfer` |
| Pomoc | `/support`, `/support/tickets/{id}`, `/support/appeals` |
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
| `GET /v1/client/directory` | `{epoch:u64, worlds:[{id, name, channels:[{id,name}], characters:[{id,name}]}]}`; ≤1 MiB, ≤256 światów, ≤128 kanałów, ≤256 postaci, `name` ≤96 B | Brak w Platform i w locku |
| `POST /api/v1/game-auth/tickets` | Bearer, `{protocol_version:1}` → `{protocol_version:1, ticket ≤256 B, expires_in>0}`, `deny_unknown_fields`; 401/403 → AUTHENTICATION_REQUIRED | Jest; U1 w grze: NOT_IMPLEMENTED po stronie Platform |
| `GET /api/v1/game-auth/native-characters` | `{protocol_version:2, characters:[{character_id, world_id, name, availability: AVAILABLE\|UNAVAILABLE}]}`, UUIDv7, ≤16384 B | Jest |
| `POST /api/v1/game-auth/device-sessions` oraz `/native-characters`, `/tickets`, `/revoke` | Kontrakt tylko po stronie Platform; gra ich nie woła | Jest (Platform) |
| `GET /api/v1/account/summary`, `GET /api/v1/feed`, `GET /api/v1/worlds` | Nowe; kontrakt do uzgodnienia z grą (G-1) | Nowe |

Uwagi:

- **Directory nie służy do odkrywania tras.** Klient odrzuca na każdej głębokości klucze `host`, `port`, `endpoint`, `protocol`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`. Nowe trasy klient zna z wersji kontraktu, nie z directory.
- **`world.characters` w directory** kłóci się z zasadą LCFA „lista postaci konta nigdy nie jest publiczna”. Do rozstrzygnięcia w G-1, zanim T36 zbuduje directory. Do tego czasu portal zwraca `characters: []`.
- **Refresh token** jest nowym zachowaniem: dzisiejszy klient go nie używa. Portal może go wydawać (z rotacją i unieważnieniem rodziny przy ponownym użyciu), ale odpowiedź dla `authorization_code` musi pozostać zgodna z tym, co czyta klient.
- Kontrakty „jak dziś” portal implementuje na własnych typach wire. Zgodność bajt w bajt sprawdzają testy z wektorami (golden) skopiowanymi z Game. Portal nie zależy od crate'ów gry: `platform-contracts` przypina tokio przez workspace gry, a `build.rs` w `protocol-oteryn` czyta docs repo gry.

### 5.3 Game API `/internal/v1` (mTLS)

Wszystkie trasy: TLS 1.3, HTTP/1.1, osobna tożsamość mTLS per grupa, limit rozmiaru, kanoniczny JSON, porównania w stałym czasie, odpowiedzi 503 i 401 dokładnie jak w kontrakcie. Testy kontraktowe czytają fixtures z `docs/contracts/fixtures/` i wymagają identycznych bajtów.

**5.3.1 Trasy, które portal serwuje** (klientem jest gra albo gateway):

| Grupa | Trasa | Kto woła | Kontrakt i stan |
| --- | --- | --- | --- |
| game-auth | `native-admissions` | Gateway logowania (Go, w tym repo) | Patrz uwaga o kierunku niżej |
| game-auth | `native-evidence`, `native-runtime-status`, `native-scope-assignments`, `native-scope-revocations` | Gra | Bez wpisu w locku |
| game-auth | `native-account-characters`, `native-account-characters/watermark` (LCFA push) | Gra | Accepted rev2; snapshot `{contract_version:1, operation, source_authority, account_id, projection_epoch, projection_revision, source_observed_at, characters[0..64, posortowane]}`, watermark `{complete_through, observed_at}`; producent gotowy, wyłączony; konsumenta w Platform brak |
| game-auth | `character-commands/{pending, read, receipt}` | Gra | GAME-CHAR-CMD-0 Accepted (D1004/D1007); CreateCharacter, TransferCharacterOwnership, kody CHAR_CMD_* |
| game-auth | `character-bootstrap-intents/read` | Gra | Kontrakt v3 (`requested_sex`, binding tag 3); v1 i v2 odrzucane |
| game-auth | `game-restore/notice` | Gra | V2_GAME_RESTORE_RECONCILIATION; domyślnie wyłączony |
| products-entitlements | `premium-snapshots/read` (≤256 B) | Gra | Kandydat (jedyna z tych tras w locku) |
| products-entitlements | `store/balance/read`, `store/history/read`, `store/purchases`, `store/operations/{pending, read, receipt, reconcile}` | Gra | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY; domyślnie wyłączony |
| game-public | `highscores`, `character-profile`, `world-online`, `watermark`; w kontrakcie dodatkowo `character-deaths` i `guild` | Gra | GAME-PUBLIC-PROJ-0 kandydat rev1, nieakceptowany |

**5.3.2 Trasy, które portal woła w grze:** dziś żadnej. Każdy przepływ z grą to pull albo push inicjowany przez grę, a portal tylko serwuje.

**Kierunek `native-admissions`.** W kodzie Platform trasę serwuje portal, a woła ją gateway Go (`services/game-gateway`, `issuerPath`) z poświadczeniem usługi. Dokumenty gry (`OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06`, `tools/qualification/login_local/nginx.conf`) opisują ją jako trasę gry wołaną przez Platform. Do potwierdzenia w G-1. Do tego czasu portal zachowuje kierunek z kodu Platform.

### 5.4 Panel zespołu

Panel ma sekcje: treści, wiki, gracze (wyszukiwanie, konto, postacie, kary, notatki), support, sklep (import i aktywacja cennika, zamówienia, zwroty), role, audyt, flagi, wydania klienta i zdrowie systemu.

Akcje krytyczne wymagają drugiej osoby (zasada czterech oczu): kary stałe, zwroty, aktywacja cennika, nadanie roli admina.

## 6. Integracja z grą

| Przepływ | Kontrakt | Stan po stronie gry (Game main @60de8634) |
| --- | --- | --- |
| Logowanie: OAuth → ticket → gateway | N4-P (OAuth PKCE), U1 (ticket), V2_NATIVE_GATEWAY_LOGIN | N4-P LOCKED, tylko testing/preprod; klient gotowy, tryb Gateway domyślnie wyłączony |
| Gateway logowania | Publiczne `POST /v1/login`, protokół 2 (poniżej) | Klient gotowy (`native_login.rs`, `gateway_login`) |
| Postacie konta | LCFA (`native-account-characters`) | Producent gotowy, wyłączony |
| Tworzenie postaci | GAME-CHAR-CMD-0 | `character_commands.rs` jest na main, a lock mówi ACCEPTED_NOT_IMPLEMENTED; sprzeczność do wyjaśnienia w locku |
| Premium | V2_PREMIUM_TIME_SNAPSHOT | Kandydat; endpoint jest |
| Sklep | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY | PROPOSED, domyślnie wyłączony |
| Dane publiczne | GAME-PUBLIC-PROJ-0 | Kandydat rev1, nieakceptowany |
| Status światów | V2_RUNTIME_STATUS_PROJECTION | PARTIAL |
| Kary | Game: `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` | Kandydat; wykonania w grze brak |
| Restore | V2_GAME_RESTORE_RECONCILIATION | Domyślnie wyłączony |
| Dane świata (wiki, Atlas) | Odczyt `content/**` z przypiętego commita gry (§6.1) | Dane są; flaga widoczności brak (G-2); licencja grafik PENDING (D7) |
| Granty admission | FND-04B-AMEND-1 | Pole status: ACCEPTED (2026-11-05), ale treść ma otwarte decyzje właściciela. Traktujemy jako propozycję i **nie implementujemy na jej podstawie**, dopóki decyzje nie zapadną |

**Gateway `POST /v1/login`** (serwuje gateway Go w tym repo, woła klient gry):

- Żądanie: `{protocol_version:2, game_login_ticket, attempt_ref (UUIDv7), character_id, channel_id|null, offer:{client_build, client_platform:"windows", transports:[{protocol_major:1, transport_profile:1, alpn:"oteryn-game/1"}]}}`.
- Odpowiedź 200: `{protocol_version, attempt_ref, world_id, channel_id, endpoint:{host, port, tls_server_name, alpn, protocol_major, transport_profile}, grant:{profile:"oteryn-pre-admission-v1", token (JWS ≤4096 B), valid_for_seconds}}`.
- Błąd: `{protocol_version, error:{code, public_class}, attempt_ref|null}`, 11 kodów z `GATEWAY_ERROR_ROWS`.
- Ponowienia klienta: ≤6 żądań, backoff 1 s·2^(n-1), `Retry-After` ≤30 s.

Zasady:

- Portal implementuje wszystkie przepływy, ale każdy jest za flagą. Włączamy go, gdy gra ma swoją stronę.
- Funkcja bez danych z gry pokazuje „niedostępne” z czasem ostatniej aktualizacji, nigdy zmyślonych wartości.
- Każda mutacja przez granicę gry jest idempotentna, ma klucz operacji, outbox i rekoncyliację. Timeout nie oznacza sukcesu ani porażki.
- Gateway logowania (Go, `services/game-gateway`) zostaje bez zmian. Portal serwuje mu `native-admissions` bajt w bajt.
- Lock kontraktów w grze jest nieaktualny (klient N4 i producent CharCmd istnieją już w kodzie). Przed każdym zadaniem z §5.2–§5.3 koordynator potwierdza aktualny stan kontraktu.

### 6.1 Treści świata gry (wiki i Atlas)

Zasada: **portal czyta te same dane, które ładuje serwer gry**, z przypiętego wydania gry. Nie ma osobnego formatu eksportu, osobnego kontraktu ani ręcznie przepisywanych danych. Jedno źródło prawdy: repozytorium gry.

| Co | Źródło w repo gry (Game main @6a81df27) | Co robi portal |
| --- | --- | --- |
| Przedmioty, potwory, NPC, łupy, questy, czary, osiągnięcia, obszary | JSON w `content/`: `items/definitions/items-*.json`, `creatures/definitions/`, `npcs/definitions/npcs-*.json`, `loot/loot-*.json`, `quests/definitions/quests-*.json`, `abilities/`, `achievements/`, `world/`. Każdy plik ma postać `{"family":..,"records":[..]}`; schematy per domena w `tools/content-schema/*-authoring/*.schema.json` | Crate `world` czyta te pliki własnymi typami serde i sprawdza je tymi schematami. Wynik trafia do `world_entities` i `world_links`; wiki generuje z nich strony, listy, filtry i wyszukiwanie |
| Pozycje na mapie | `content/world/` (`npc-placements` z komórką `{floor,x,y}`, obszary, obiekty) | Znaczniki Atlasu |
| Mapa | Minimapa `content/assets/files/minimap-32-<x>-<y>-<z>-<sha>.bmp.lzma` | Worker rozpakowuje LZMA, skleja BMP i tnie piramidę kafli WebP per piętro |
| Grafiki | `appearances-*.dat` (protobuf), `sprites-*.bmp.lzma`, manifest `content/assets/catalog/index.json` | Worker wycina miniatury PNG i WebP. **Tylko po decyzji D7** |

Czego portal nie używa:

- Crate'a `oteryn-gameplay-content`, bo nie działa bez reszty workspace gry.
- `crates/renderer` gry, bo robi atlas tekstur klienta, a nie kafle mapy.
- Katalogu CATALOG-V1 (`OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`), bo ma status PROPOSED, producent ma tylko fixture i brakuje w nim pozycji na mapie. Czytanie `content/**` daje to samo bez drugiego formatu.

Jak to działa:

1. Admin wybiera w panelu wersję gry (commit lub tag). Worker pobiera z niej tylko `content/**` i `tools/content-schema/**` kluczem tylko do odczytu (deploy key repo gry, trzymany na hoście jako sekret).
2. `world` parsuje dane do nowej wersji w bazie i, jeśli D7 na to pozwala, renderuje kafle obok. Gdy wszystko się uda, przełącza aktywną wersję jedną transakcją. Błąd parsowania zatrzymuje import, a stara wersja dalej działa.
3. Wiki i Atlas pokazują wersję świata. Odnośniki wiki ↔ Atlas idą po kluczach encji, nie po nazwach.

Co robi redakcja: pisze poradniki i opisy dopisane do encji (`wiki_guides`). Nie przepisuje statystyk ani łupów, bo te zawsze pochodzą z gry. Nowa wersja gry aktualizuje dane bez ruszania tekstów redakcji.

Ukryte treści:

- Osiągnięcia mają flagę `secret`; portal jej nie pokazuje.
- Questy, NPC, obszary i miejsca nie mają flagi widoczności. Gra doda pole w swoich schematach (osobny pakiet po stronie gry, G-2). Do tego czasu wiki pokazuje tylko encje, które admin zatwierdzi w panelu (`world_visibility` działa jako lista dopuszczonych), a domyślnie nic nie jest publiczne.
- Rekordy questów mają `classification` (np. `OTS_HYPOTHESIS_ONLY`, `SCOPED_EVIDENCE_ONLY`, `OTERYN_AUTHORED_APPROXIMATION`). Wiki publikuje tylko `OTERYN_AUTHORED_APPROXIMATION`, chyba że właściciel zdecyduje inaczej.

Licencja: `LICENSE-ASSETS.md` gry zastrzega grafikę, mapy i metadane („all rights reserved”, poza użyciem referencyjnym), a `content/world/manifest.json` ma `licensing_metadata: PENDING`. Publikacja sprite'ów i mapy na stronie to decyzja właściciela (D7). Bez niej wiki działa bez grafik, a Atlas jest wyłączony.

Dane na żywo (online, rankingi, status światów) to projekcje z §5.3, nie część danych świata.

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
| T13 | Wiki: poradniki redakcji, wersje, przegląd, markdown i sanityzacja, wyszukiwanie (PG full text) | T10 | `crates/wiki` |
| T14 | `world`: odczyt `content/**` JSON z przypiętego commita gry, walidacja schematami gry, wersje, atomowe przełączenie, lista dopuszczonych encji | T00 | `crates/world` |
| T14b | Strony wiki generowane z `world`: listy per typ, strona encji, filtry, wyszukiwanie, opisy redakcji | T13, T14 | `crates/wiki/src/entities*`, `templates/wiki/` |
| T15 | Guard mTLS `/internal/v1` (tożsamość per grupa), kanoniczny JSON, harness testów na wektorach golden z Game | T00 | `crates/internal-api` (wspólne) |
| T16 | OAuth PKCE (`/oauth/authorize`, `/oauth/token` w korzeniu base URL, consent); odpowiedź `authorization_code` zgodna z klientem; refresh z rotacją jako nowe zachowanie | T04 | `crates/gameauth/src/oauth*` |
| T17 | `POST /api/v1/game-auth/tickets` i `GET /api/v1/game-auth/native-characters` (kształty z §5.2, wektory golden) | T16, T18 | `crates/gameauth/src/tickets*`, `crates/player-api` |
| T18 | Konsument LCFA: `native-account-characters` i watermark (snapshot ≤64 postaci, epoka i rewizja) | T15 | `crates/characters` |
| T19 | `native-admissions` dla gatewaya Go, evidence, runtime-status, granty Ed25519, rotacja kluczy (bez zmian z FND-04B-AMEND-1) | T15 | `crates/gameauth/src/admission*` |
| T20 | Scope assignments i revocations | T19 | `crates/gameauth/src/scopes*` |
| T21 | Sesje urządzeń (enroll, characters, tickets, revoke); kontrakt po stronie Platform | T17 | `crates/gameauth/src/devices*` |
| T22 | `character-commands/{pending, read, receipt}`, `character-bootstrap-intents/read` (v3), tworzenie postaci z konta | T18 | `crates/characters/src/commands*` |
| T23 | `/internal/v1/game-auth/game-restore/notice` i rekoncyliacja | T22 | `crates/characters/src/restore*` |
| T24 | Projekcje publiczne `/internal/v1/game-public/` (`highscores`, `character-profile`, `world-online`, `watermark`; `character-deaths` i `guild` po akceptacji kontraktu) i strony społeczności | T15, T01 | `crates/community` |
| T25 | Status światów: strona i `GET /api/v1/worlds` | T24 | `crates/community/src/worlds*` |
| T26 | Ledger portfela, saldo, historia, transfer coinów (blokady, idempotencja) | T04 | `crates/wallet` |
| T27 | `premium-snapshots/read` (≤256 B) | T15, T26 | `crates/store/src/snapshots*` |
| T28 | Import i aktywacja cennika sklepu (S-5, `catalog_revision` = SHA-256) | T10 | `crates/store/src/catalog*` |
| T29 | Zakup, saldo i historia przez wire sklepu | T26, T28, T15 | `crates/store/src/purchase*` |
| T30 | Dostawa: operacje pending, read, receipt, reconcile | T29 | `crates/store/src/delivery*` |
| T31 | Płatności: trait dostawcy, provider testowy, webhooki z idempotencją, doładowanie coinów | T26 | `crates/payments` |
| T32 | Sklep web: oferty, doładowanie, historia, transfer | T29, T31 | `crates/store/src/web*`, szablony |
| T33 | Support: zgłoszenia, wiadomości, odwołania | T10 | `crates/support` |
| T34 | Kary: historia, cztery oczy; przekazanie do gry według kandydata enforcement z Game (po akceptacji) | T33, T15 | `crates/support/src/sanctions*` |
| T35 | Pobieranie i kanały TUF | T11 | `crates/downloads` |
| T36 | Powiadomienia e-mail (outbox), `GET /api/v1/account/summary`, `GET /api/v1/feed`, `GET /v1/client/directory` (po G-1) | T11, T26 | `crates/notify`, `crates/player-api` |
| T37 | Worker: outbox, ponowienia, sprzątanie | T00 | `bin/oteryn-portal/src/worker*`, `crates/db/src/outbox*` |
| T38 | `admin-cli import-legacy` z eksportu obecnej bazy | T03–T34 | `bin/oteryn-portal/src/import*` |
| T39 | Projekt graficzny: tokeny CSS, komponenty, strona główna, „Jak zacząć” | T01 | `static/`, `templates/` |
| T40 | Atlas: kafle WebP z minimapy gry (LZMA → BMP → piramida) i znaczniki z `npc-placements` (za flagą D7) | T14 | `crates/atlas/src/tiles*` |
| T41 | Atlas: strona `/atlas` (przeglądarka kafli, warstwy, znaczniki, linki do wiki) | T40, T14 | `crates/atlas/src/web*`, `templates/atlas/`, `static/atlas/` |
| T42 | Miniatury z `appearances-*.dat` i `sprites-*.bmp.lzma` do wiki i znaczników (za flagą D7) | T14 | `crates/world/src/sprites*` |
| G-2 | (gra) Pole widoczności (public/hidden) w schematach questów, NPC i obszarów | – | Oteryn-Game, przez koordynatora |
| G-1 | (gra) Kontrakt nowych tras Player API i `client/directory` (wpis w locku, `world.characters` vs LCFA), kierunek `native-admissions`, aktualizacja locka (N4, CharCmd) | – | Oteryn-Game, przez koordynatora |

Kolejność startu:

1. Najpierw T00, bo daje wspólny szkielet. Ja mogę go przygotować i sprawdzić lokalnie, żeby Codex startował od kompilującej się bazy.
2. Potem jednocześnie: T01, T14, T15, T37 i T39.
3. Potem gałęzie konta (T02–T10), gry (T16–T25) i handlu (T26–T32) równolegle.

### 10.1 Szablon zadania dla Codexa

Koordynator wysyła każde zadanie w tej postaci:

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
Zakazy: brak zmian poza dozwolonymi ścieżkami, brak .github/workflows, brak sekretów, brak unwrap/expect/panic poza testami
Wynik: PR draft z wynikiem komend akceptacji w opisie
```

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
| D7 | Publikacja sprite'ów i mapy gry na stronie (licencja `LICENSE-ASSETS.md`) | Wyłączona: wiki bez grafik, Atlas wyłączony |
| D8 | Klasyfikacje questów publikowane w wiki | Tylko `OTERYN_AUTHORED_APPROXIMATION` |

## 12. Otwarte pytania

| # | Pytanie | Do kogo | Blokuje |
| --- | --- | --- | --- |
| Q1 | Publikacja grafik i mapy (D7) oraz klasyfikacje questów (D8) | Właściciel | T40, T42 |
| Q2 | Kierunek `native-admissions` (§5.3) | Koordynator, G-1 | T19 |
| Q3 | Kontrakt `client/directory` i `world.characters` vs LCFA (§5.2) | Koordynator, G-1 | T36 |
| Q4 | Sprzeczność locka gry dla GAME-CHAR-CMD-0 i N4 (§6) | Koordynator | T22 |
| Q5 | Decyzje właściciela w FND-04B-AMEND-1 (§6) | Właściciel | Zmiany grantów w T19 |
| Q6 | Akceptacja projektu, job CI (D2), sposób wysyłki zadań do Codexa | Właściciel | Start T01+ |

Pozostałe zadania nie czekają na te odpowiedzi.

## 13. Źródła

- Obecny portal: trasy `routes/internal.php`, `routes/api.php`, `routes/modules/*`; moduły `app/*`.
- Kontrakty: `docs/contracts/`, w tym wire sklepu i decyzje z 2026-11-05.
- Gra (main @60de8634): `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`, `crates/platform-client` (`lib.rs`, `native_login.rs`, `native_login/owner_characters.rs`), `crates/gameplay/native-admission-source` (`account_characters.rs`, `character_commands.rs`, `store.rs`, `restore_notice.rs`, `public_projections.rs`), `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md`.
- Przegląd od strony gry: koordynator, 2026-11-05.
- Szkic #1643: źródło pomysłów dla rozszerzeń (węzły, Atlas, obserwacja GM).
