# Oteryn Platform — kompleksowy audyt repozytorium

**Data:** 2026-09-06  
**Repozytorium:** `Oteryn/Oteryn-Platform`  
**Audytowany baseline:** `3b2ea1c7392187d5d22488673073dc8f8305a374`  
**Governing programme:** Issue #451 (`OTERYN-PLATFORM-PRODUCTION-COMPLETION`)  
**Charakter:** read-only audit; ten plik jest osobną, dokumentacyjną publikacją wyników.

## 1. Werdykt

**RECOMMENDATION.** Zachować obecną architekturę modularnego monolitu Laravel z osobnym Game Gateway. W zbadanym materiale nie ma podstaw do przepisywania Platformy ani dzielenia jej na kolejne mikroserwisy. Największą wartość daje domknięcie konkretnych luk w bramkach CI, bezpieczeństwie operacji, odtwarzalności buildów, wdrożeniu i testach krytycznych granic.

**UNKNOWN.** Ten audyt nie dowodzi gotowości produkcyjnej całej Platformy i nie jest certyfikatem sprawdzenia 100% plików. Nie wykonano pełnego checkoutu z zależnościami, kompletnej suity PHP/Go/Playwright, pełnego Docker build, wszystkich migracji ani rzeczywistego restore/rollback środowiska.

Nie potwierdzono problemu P0. Potwierdzono dwa ustalenia P1 oraz dwanaście P2/P3 lub dodatkowych problemów/ryzyk wymagających planowego zamknięcia. Dwie dalsze kwestie pozostają hipotezami wymagającymi testu wykonawczego.

## 2. Podstawa dowodowa

### Zweryfikowane repo/GitHub

- `main` w czasie audytu: `3b2ea1c7392187d5d22488673073dc8f8305a374`.
- `main` jest chroniony i wymaga kontekstu `platform-gate`; pełne szczegóły ochrony były niedostępne dla integracji.
- CI run `33529328112` z 2026-09-01 na `85eb4c41d977340e2599006d5cfef271d1e334cf` wykonał udany runtime PHP, static analysis, Composer audit i coverage.
- Odczytany artefakt coverage zawierał 598 testów, 4 846 asercji, 0 błędów, 0 failures, 4 skips; statement coverage 14 412 / 17 674 = 81,54%.
- Drzewo `app` oraz główne drzewa Feature/Unit tests były identyczne między tym udanym PHP run i audytowanym baseline; nie oznacza to automatycznie pełnej walidacji nowszego head.
- Niezależnie wykonano wycinek `RequiredTestGateTest`: 8/8 PASS. Nie uruchomiono całego pliku testów kontraktu workflow.

### Główne źródła

- [`AGENTS.md`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/AGENTS.md)
- [`docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/docs/agents/PLATFORM_AGENT_BOOTSTRAP.md)
- [`docs/agents/CONTEXT_ROUTING.md`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/docs/agents/CONTEXT_ROUTING.md)
- [`.github/workflows/ci.yml`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/.github/workflows/ci.yml)
- [`.github/workflows/game-auth-ticket-concurrency.yml`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/.github/workflows/game-auth-ticket-concurrency.yml)
- [`.github/workflows/game-gateway-ci.yml`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/.github/workflows/game-gateway-ci.yml)
- [`.github/workflows/build-synology-staging-images.yml`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/.github/workflows/build-synology-staging-images.yml)
- [`deploy/synology/docker/platform.Dockerfile`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/deploy/synology/docker/platform.Dockerfile)
- [`services/game-gateway/cmd/game-gateway/main.go`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/services/game-gateway/cmd/game-gateway/main.go)
- [`services/game-gateway/internal/httpapi/server.go`](https://github.com/Oteryn/Oteryn-Platform/blob/3b2ea1c7392187d5d22488673073dc8f8305a374/services/game-gateway/internal/httpapi/server.go)

## 3. Rejestr ustaleń

### F01 — P1 — wymagany `platform-gate` nie agreguje wszystkich istotnych dowodów

**FACT.** `platform-gate` agreguje klasyfikację i główny job testowy, ale nie jest fan-in dla osobnego workflow współbieżności Game Auth ani testów/builda Go. Odczytane workflow współbieżności i Gateway nie obsługują `merge_group`.

**INFERENCE — wysoka pewność dla konfiguracji.** Zielony wymagany kontekst nie dowodzi wykonania wszystkich kontroli potrzebnych dla zmian auth/concurrency/Gateway. Nie oznacza to, że istnieje potwierdzony bypass ochrony.

**RECOMMENDATION.** Utrzymać fail-closed klasyfikator, ale dodać mały manifest wymaganych profili i prawidłowy fan-in dla dokładnego kandydata PR/Merge Queue.

**Dowód zamknięcia:** canary PR + rzeczywisty Merge Queue; failure/cancelled/brak wyniku/nieuzasadnione skipped blokują właściwy profil.

### F02 — P1 — zmiana e-maila ma ryzyko częściowego dostarczenia powiadomień

**FACT.** `RequestIdentityEmailChange` zatwierdza zmianę stanu bazy, a następnie wysyła potwierdzenie na nowy adres i ostrzeżenie na stary. Powiadomienia mają `Queueable`, ale nie implementują `ShouldQueue`; kontroler nie przekształca awarii transportu w spójny model stanu dostarczenia.

**INFERENCE — wysoka pewność z kodu; brak próby awarii SMTP.** Awaria pierwszej lub drugiej wysyłki może pozostawić częściowo wykonany proces komunikacji bezpieczeństwa.

**RECOMMENDATION.** Zastosować trwały, audytowalny model dostarczania obu wiadomości, np. outbox lub równoważny mechanizm, z idempotencją i osobnym stanem każdej wysyłki.

**Dowód zamknięcia:** test awarii pierwszego maila, drugiego maila, restartu między zapisem i wysyłką, ponowienia, wygaśnięcia i unieważnienia poprzednich tokenów.

### F03 — P2 — Gateway nie ma jawnego limitu czasu odczytu body

**FACT.** `http.Server` ma `ReadHeaderTimeout`, `WriteTimeout` i `IdleTimeout`, ale brak `ReadTimeout`. Handler czyta ograniczone rozmiarem body przez `io.ReadAll` bez własnego deadline.

**INFERENCE — wysoka pewność.** Limit rozmiaru nie jest limitem czasu powolnego przesyłania body. Faktyczna ekspozycja za reverse proxy pozostaje niezbadana.

**RECOMMENDATION.** Dodać jawny deadline dla odczytu request body i test kontrolowanego slow-body.

### F04 — P2 — buildy nie są w pełni odtwarzalne na warstwie systemowej

**FACT.** Odczytane Dockerfile/Compose używają tagów bazowych i instalacji zależności systemowych/PECL bez pełnego pinowania do digestów/wersji, m.in. `php:8.5-cli-alpine`, `composer:2`, APK i `pecl install redis`.

**INFERENCE — wysoka pewność.** Identyczny commit i lock Composer nie gwarantują identycznego pełnego obrazu przy późniejszym buildzie.

**RECOMMENDATION.** Kontrolowane aktualizacje digestów i wersji wejść systemowych oraz manifest release wiążący źródłowy SHA, builder i digest obrazu.

### F05 — P2 — coverage jest raportem, nie ratchetem jakości

**FACT.** Coverage pracuje w `report_only`, bez minimalnego statement threshold. W analizowanym Clover występowały wykonywalne pliki z zerem trafień, m.in. krytyczne ścieżki bezpieczeństwa i katalogu.

**RECOMMENDATION.** Najpierw uzupełnić testy failure/integrity dla krytycznych granic, a dopiero potem wprowadzić mały ratchet dla zmienianego kodu. Nie ustawiać arbitralnego 100%.

### F06 — P2 — jedna zależność taskowa wskazywała zamknięty niescalony PR

**FACT z czasu audytu.** Platform #1267 / PR #1270 wskazywały zależność od META PR #73, który był `closed` i `merged=false`.

**RECOMMENDATION.** Uzgodnić zależność z aktualnym kanonicznym stanem META albo jawnie ją supersedować; nie czekać na zdarzenie, które nie może już nastąpić.

### F07 — P2 — Platform i META miały rozjazd modelu wykonania instrukcji

**FACT.** Platform utrzymywał silniejsze `parallel-first`, podczas gdy aktualny META dopuszczał `single_agent` jako normalny wybór i wymagał równoległości tam, gdzie faktycznie daje korzyść. Platform wskazywał też starszy pin organizacyjnej polityki routingu.

**RECOMMENDATION.** Jeden model-neutralny kontrakt organizacji, cienkie inwarianty repozytorium i task-routed specjalizacje; model/surface/effort powinny być profilem wykonawczym, nie osobnym systemem governance.

### F08 — P2 — branch hygiene wykrył dwie niewyjaśnione gałęzie

**FACT z run `34022612703`.** W artefakcie branch hygiene wykryto m.in. `test/codex-native-publish-canary-20260903-platform` i `test/codex-shared-pat-canary-20260903-platform` jako niewyjaśnione przez otwarty PR/aktywny claim.

**RECOMMENDATION.** Zweryfikować właściciela i unikalną zawartość, następnie przypisać do zadania lub nadać świadomą terminalną dyspozycję. Sama nazwa z `pat` nie jest dowodem wycieku sekretu.

### F09 — P2 — instrukcja świeżego uruchomienia nie tworzy schematu bazy

**FACT.** README tworzy lokalny plik SQLite i uruchamia aplikację, ale nie zawiera kroku migracji; Composer install nie wykonuje migracji automatycznie.

**INFERENCE — wysoka pewność, bez fresh-install run.** Sam plik DB nie zapewnia tabel wymaganych przez realne moduły.

**RECOMMENDATION.** Dodać bezpieczny krok lokalnej inicjalizacji schematu i test „clean checkout → instrukcja → podstawowy flow”.

### F10 — P3 — Wallet zależy od wyjątku Marketplace

**FACT.** `WalletMutator` i `AdjustWalletBalance` używają `App\Marketplace\Exceptions\MarketplaceException`.

**RECOMMENDATION.** Przy następnej zmianie w tej granicy wydzielić neutralny błąd Wallet i mapować go w Marketplace, bez refaktoru poza potrzebą.

### F11 — P2 — Gateway nie synchronizuje zakończenia procesu z graceful shutdown

**FACT.** `server.Shutdown(ctx)` jest uruchamiany w osobnej gorutynie po sygnale. Główna gorutyna po zakończeniu `ListenAndServe()` loguje `gateway_stopped` i wraca z `main`; kod nie ma kanału/WaitGroup synchronizującego zakończenie gorutyny shutdown.

**INFERENCE — wysoka pewność z kodu; brak testu procesu.** Timeout 10 s przekazany do `Shutdown` nie gwarantuje, że proces będzie istnieć do zakończenia drenażu aktywnych żądań.

**RECOMMENDATION.** Zsynchronizować shutdown i zakończenie `main`.

**Dowód zamknięcia:** test procesu z opóźnionym requestem + `SIGTERM`, sprawdzający zarówno poprawny drain, jak i bounded timeout.

### F12 — P2 — build image workflow pomija zmianę ograniczoną do `lang/**`

**FACT.** Filtry `pull_request.paths` i `push.paths` w `build-synology-staging-images.yml` obejmują m.in. `app/**`, `config/**`, `resources/**`, `routes/**`, ale nie `lang/**`.

**INFERENCE — wysoka pewność.** Commit zmieniający wyłącznie tłumaczenia nie uruchamia tego workflow przez PR ani push.

**RECOMMENDATION.** Traktować listę path triggers jako kontrakt wejść runtime i objąć nią `lang/**` oraz testować trigger matrix.

### F13 — P2 — publikacja obrazów nie zależy od `validate-deployment`

**FACT.** `build` i `validate-deployment` są osobnymi jobami; `build` nie ma `needs: validate-deployment`. `docker/build-push-action` publikuje obraz dla `workflow_dispatch` oraz odpowiednich pushy na `main`.

**INFERENCE — wysoka pewność dla tego workflow.** Obraz może zostać opublikowany mimo równoległej porażki walidacji pakietu wdrożeniowego. To nie jest dowód automatycznego wdrożenia wadliwego obrazu.

**RECOMMENDATION.** Rozdzielić równoległy build od promocji kwalifikowanego artefaktu; promocja powinna zależeć od właściwych kontroli.

### F14 — P2 — szeroki Docker context + `COPY . .` bez rootowego `.dockerignore`

**FACT.** Workflow używa `context: .`; Platform Dockerfile wykonuje `COPY . .`; rootowy `.dockerignore` nie istnieje na audytowanym commicie.

**INFERENCE — wysoka pewność dla konstrukcji builda.** Końcowy obraz może zawierać materiały niepotrzebne do runtime; lokalny build może dodatkowo zależeć od nieśledzonej zawartości checkoutu. Nie znaleziono dowodu rzeczywistego opublikowania sekretu.

**RECOMMENDATION.** Ograniczyć kontekst/kopiowane wejścia i dodać test zawartości obrazu. Obecna PR validation Platform sprawdza jedynie `php -v`, co nie dowodzi właściwej zawartości warstwy aplikacyjnej.

## 4. Hipotezy wymagające testu

### H01 — klasyfikator CI a mieszana zmiana typu pliku

**FACT.** Klasyfikator bazuje na liście z `git diff --name-only --diff-filter=ACMRD`; pusta lista ma zachowanie fail-safe.

**UNKNOWN.** Nie wykonano rozstrzygającego przypadku, w którym zmiana typu pliku runtime jest połączona z normalną zmianą dokumentacji.

**Test rozstrzygający:** tymczasowe repo, kontrolowana type-change + Markdown, osobno rename/delete; porównać pełny diff z decyzją klasyfikatora.

### H02 — logowanie a równoległa zmiana hasła/unieważnienie sesji

**FACT.** Hasło jest sprawdzane przed ustanowieniem sesji, a późniejszy kod sesji odświeża `Identity` i zapisuje bieżącą generację sesji.

**UNKNOWN.** Nie potwierdzono zachowania przeplotu, w którym pomiędzy poprawnym sprawdzeniem starego hasła i ustanowieniem sesji następuje zmiana hasła/unieważnienie generacji.

**Test rozstrzygający:** zatrzymać login po weryfikacji starego hasła, wykonać zmianę hasła/revocation, wznowić login i sprawdzić wynik względem jawnego kontraktu bezpieczeństwa.

## 5. Mocne strony potwierdzone w kodzie

- Logowanie rozdziela hasło i MFA; sesje są rejestrowane i mają mechanizm unieważniania.
- Ticket redeem ma transakcyjne sprawdzenie hasha, audience, ważności, jednorazowości, stanu identity, security generation i bindingu.
- Gateway strict JSON odrzuca nieznane/duplikowane/non-canonical fields; request body ma limit rozmiaru; dependency HTTP client ma bounded timeout.
- Wallet używa locking, idempotencji i ledger entry.
- Admin routes w zbadanych modułach łączą auth, MFA i permission middleware.
- Recovery key używa keyed hash, jednorazowego stanu i unieważnienia web/game authorizations po użyciu.
- Dependency base URLs Gateway fail-close dla nie-loopback HTTP i nie dopuszczają credential-bearing/query URL.
- Odczytane GitHub Actions pinują używane actions do pełnych commit SHA.

Te punkty nie zastępują pełnego security review wszystkich endpointów.

## 6. Architektura i jakość projektu

**FACT.** Oteryn Platform jest rozbudowanym modularnym monolitem Laravel/PHP z osobnym procesem Game Gateway w Go. Kod aplikacji jest dzielony domenowo, m.in. Identity, CharacterProfiles, Marketplace, Wallet, Payments, EditorialMedia, Wiki, PublicGameData, Support i GameAuth.

**RECOMMENDATION.** Zachować ten podział. Nową granicę procesową/mikroserwis dodawać dopiero przy konkretnym problemie skalowania, ownership albo wdrożeń. Obecna architektura ogranicza liczbę rozproszonych transakcji i operacyjnych punktów awarii.

Najważniejszą miarą postępu powinien być ukończony pion funkcjonalny: persistence + domain + auth + transport + reachable consumer + failure states + test/evidence + operability, a nie sama liczba modułów czy dokumentów.

## 7. Testy i CI — ocena

**FACT.** Istnieje realna szeroka baza testów PHP i oddzielne workflow krytycznych przypadków, ale dowody nie są jeszcze jednolicie agregowane do jednego required outcome. Coverage jest przydatnym baseline, lecz nie polityką regresji. W części krytycznych ścieżek brakuje bezpośrednich trafień w analizowanym Clover.

**RECOMMENDATION.** Nie wyłączać całego systemu testów. Naprawić routing/fan-in, zachować szybkie docs-only ścieżki, a drogie testy uruchamiać tylko tam, gdzie klasyfikacja lub kontrakt tego wymagają. Merge Queue musi weryfikować dokładnego kandydata kolejki.

## 8. Build, deployment i operacje

Najważniejsze ryzyka operacyjne wynikają z F04, F11–F14. Sam zielony build nie powinien być utożsamiany z release qualification. Docelowy manifest wydania powinien wiązać co najmniej:

- source SHA;
- digest obrazów;
- wejścia bazowe/systemowe;
- wynik wymaganych testów/kwalifikacji;
- migrację/rollback/restore evidence dla konkretnego wydania;
- identity środowiska i decyzję aktywacyjną.

**UNKNOWN.** Ten audyt nie potwierdził świeżej, rzeczywistej próby disaster recovery ani pełnego release pipeline na produkcyjnym/stagingowym środowisku.

## 9. Instrukcje i governance agentów

**RECOMMENDATION.** Utrzymać organizacyjne safety/invariants, ale uprościć warstwę instrukcji: model-neutralny kontrakt centralny, repo-specific boundaries, task-routed specialization i cienki execution profile określający surface/model/effort. Nie mnożyć historycznych promptów i model-specific workaroundów w always-loaded context.

Nie wolno jednak usuwać w celu oszczędności tokenów granic produkcji, credentials, payments, auth/session, cross-repository oraz branch/merge authority.

## 10. Kolejność zamknięcia

1. **P1:** F01 — required evidence/fan-in dla PR i Merge Queue.
2. **P1:** F02 — trwałe bezpieczeństwo procesu zmiany e-maila i failure-path tests.
3. **P2 operacyjne:** F11–F14 — shutdown, trigger coverage, qualification gate, Docker context.
4. **P2 reliability:** F03–F05 — body timeout, build provenance, risk-driven tests/coverage ratchet.
5. **P2 governance/hygiene:** F06–F09.
6. **P3:** F10 przy najbliższej zmianie Wallet/Marketplace.
7. **Rozstrzygnąć testem:** H01 i H02; nie podnosić ich do confirmed findings bez wyniku.

## 11. Braki do pełnego audytu repozytorium

Dokument nie może uczciwie deklarować „100% coverage”. Nadal wymagają pełnej, bezpośredniej kontroli:

- kompletna lista wszystkich plików z rejestrem `reviewed / sampled / metadata-only / not-reviewed`;
- wszystkie migracje, constrainty, indeksy i rollback paths;
- kompletne Marketplace/Payments/Wallet failure + concurrency paths;
- wszystkie public/admin/account routes, controllers, requests, policies i views;
- wszystkie frontend states, EN/PL, accessibility i responsive matrix;
- pełny Go Gateway test run;
- pełny PHP/composer/static-analysis run na exact final audited SHA;
- pełny Playwright/E2E matrix wymagany przez aktualny contract;
- rzeczywisty Docker build i inspekcja zawartości obrazów;
- restore/rollback drill na kontrolowanym środowisku;
- pełna ochrona branch/rulesets/Merge Queue, jeżeli integracja uzyska odpowiednie read permissions;
- historia sekretów/proweniencji/assets i pełny license/distribution audit.

## 12. Status końcowy

**AUDIT_STATUS: OPEN / PARTIAL COVERAGE.**

Raport zawiera konkretne, zweryfikowane ustalenia i jawnie oznaczone hipotezy, ale nie jest certyfikatem pełnego sprawdzenia repozytorium ani produkcyjnej gotowości. Dalszy audyt powinien prowadzić coverage ledger zamiast ponownie używać ogólnego określenia „sprawdzono wszystko”.
