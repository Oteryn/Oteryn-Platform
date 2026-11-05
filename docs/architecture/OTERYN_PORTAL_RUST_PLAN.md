# Oteryn Portal w Ruście – projekt i plan

> **Status: PROPOZYCJA, rewizja 1** (2026-11-05). Dokument do przeglądu właściciela i koordynatora programu. Nie autoryzuje kodu, wdrożenia, zmian w kontraktach gry ani wydatków. Każda decyzja z sekcji 10 ma wartość domyślną. Obowiązuje dopiero po akceptacji właściciela.
>
> Relacja do szkicu #1643 (`OTERYN_PLATFORM_2_DESIGN_DRAFT.md`, rewizja 4.1): ten dokument zaczyna projekt od potrzeb graczy i zespołu oraz od integracji z klientem i serwerem gry. Ze szkicu #1643 przejmuje stos technologiczny, zasady zgodności wire, zarządzanie serwerami (M5), Atlas (MA) i obserwację GM-a (M10). Zmienia trzy rzeczy: przełączenie odbywa się falami zamiast jednym ruchem, Player API jest osobną, wersjonowaną powierzchnią dla klienta gry, a zakres jest podzielony na rdzeń i rozszerzenia. Szczegółów z #1643 nie powtarzamy, tylko się do nich odwołujemy.

## 0. Streszczenie

- **Co budujemy.** Nowy portal Oteryn w Ruście jest jednym produktem dla czterech odbiorców:
  - gracz w przeglądarce;
  - gracz w kliencie gry i launcherze;
  - serwer gry;
  - zespół (admin, GM, support, redakcja, operacje).
- **Obecny portal** (Laravel) jest źródłem pomysłów i danych, nie specyfikacją. Przenosimy:
  - sprawdzone granice domen;
  - kontrakty z grą;
  - inwarianty bezpieczeństwa.

  Nie przenosimy długu: starego silnika serwera i jego bazy, równoległych ścieżek legacy, pustych modułów.
- **Integracja z grą jest pierwszorzędna.** Każda funkcja, która dotyka gry, ma nazwany kontrakt, właściciela danych, testy zgodności przeciw przypiętemu obrazowi gry i jawną semantykę błędów.
- **Stos:**
  - Rust z axum, askama SSR i htmx;
  - PostgreSQL, sqlx i rustls;
  - jeden workspace, osobne binarki per rola.

  Wersje i uzasadnienie są w #1643, sekcje „Wersje” i D3.
- **Przełączenie w trzech falach**, każda z własnym rollbackiem:
  1. strona publiczna;
  2. konto i logowanie do gry;
  3. handel i sklep.

  Pierwsza wartość dla graczy pojawia się po fali 1, a nie po roku budowy.
- **Repozytorium:** budujemy w `Oteryn/Oteryn-Platform` (decyzja właściciela 2026-11-05: bez osobnego repo). Kod w Ruście trafia do katalogu `portal-rs/` obok Laravela, a Laravel jest usuwany w F7.
- **Plan** to fazy F0–F7 z bramkami wyjścia. Praca jest dzielona na małe zadania: jedna mechanika na jedno zadanie.

## 1. Cele i zasady

### Cele produktu

1. Nowy gracz w kilka minut rozumie, czym jest Oteryn, zakłada konto, pobiera klienta i wchodzi do gry.
2. Powracający gracz od razu widzi swój stan: postacie, premium, saldo, wiadomości i status świata. Działa to tak samo w przeglądarce, w launcherze i w kliencie.
3. Społeczność ma żywe, prawdziwe dane: rankingi, profile, gildie, domy, zgony, online, wiki z katalogu gry i Atlas.
4. Zespół prowadzi grę z jednego panelu: treści, moderacja, support, sklep, serwery, wydania klienta i audyt.
5. Portal nigdy nie zmyśla stanu gry. Gdy brak danych, pokazuje „niedostępne” lub „nieaktualne” z czasem ostatniej aktualizacji, a nie „0 graczy” ani „offline”.

### Zasady inżynierskie

- **Jeden właściciel danych.** Platform jest właścicielem konta (AccountId), tożsamości, sesji, OAuth, uprawnień, treści, portfela i sklepu. Gra jest właścicielem postaci (CharacterId), rozgrywki, sesji gry i ostatecznego admission. Zmiany przez granicę idą tylko przez komendy i potwierdzenia (receipts), nigdy przez wspólną bazę ani bezpośredni SQL.
- **Timeout nie jest wynikiem.** Każda mutacja przez granicę jest idempotentna, ma klucz operacji i rekoncyliację.
- **Domyślnie wyłączone.** Każda nowa powierzchnia gry ma flagę i bramkę aktywacji.
- **Rdzeń najpierw.** Moduł trafia do rdzenia tylko wtedy, gdy gracz lub zespół potrzebuje go na start. Reszta jest rozszerzeniem z własną bramką.
- **Bezpieczeństwo bez wyjątków.** Inwarianty z `AGENTS.md` i `docs/architecture/SECURITY_ARCHITECTURE.md` obowiązują w Ruście tak samo. Obejmują CSRF, autoryzację każdej mutacji, rate limity, rotację sesji, transakcje z blokadami przy walucie oraz idempotencję i ochronę przed powtórzeniem dla płatności i webhooków.
- **Kod w Ruście** podlega polityce lintów gry: `unsafe_code = forbid`; `unwrap`, `expect` i `panic` mają poziom `deny`. Domena jest bez I/O, adaptery są osobno.

## 2. Dla kogo: potrzeby

### Gracze

| Persona | Potrzeby | Gdzie to dostaje |
| --- | --- | --- |
| Nowy gracz | Czym jest gra, jak zacząć, rejestracja, pobranie klienta, pierwsza postać, zasady | Web publiczny, onboarding, launcher |
| Powracający gracz | Logowanie jednym kliknięciem, stan konta, co nowego, status świata, wydarzenia | Launcher, panel konta, Player API |
| Aktywny gracz | Rankingi, profile, zgony, gildie, domy, narzędzia (kalkulatory, analizator sesji), wiki i Atlas | Web publiczny, narzędzia, link z klienta |
| Kupujący | Saldo coinów, sklep w kliencie i w przeglądarce, premium, historia zakupów, faktura, transfer coinów | Sklep (fala 3), Player API |
| Handlujący | Bazar postaci i przedmiotów z bezpiecznym escrow | Bazar (fala 3, za kontraktem z grą) |
| Dbający o konto | 2FA, passkeys, klucz odzyskiwania, lista sesji i urządzeń, historia bezpieczeństwa, eksport i usunięcie danych (RODO) | Panel konta |
| Szukający pomocy | Zgłoszenia, odwołania od kar, FAQ, status zgłoszenia | Support |

Wymagania przekrojowe dla graczy:

- język polski i angielski od pierwszego dnia;
- pełna obsługa telefonu;
- dostępność na poziomie WCAG 2.2 AA;
- szybkie strony (budżety w sekcji 4.7);
- powiadomienia e-mail oraz w kliencie;
- jasne komunikaty przy przerwach technicznych.

### Zespół

| Rola | Potrzeby |
| --- | --- |
| Właściciel / admin | RBAC, audyt każdej akcji, zasada czterech oczu dla akcji krytycznych, flagi funkcji, przegląd zdrowia systemu |
| GM / moderator | Kary (ban, mute, kick, ostrzeżenie) wykonywane w grze przez kontrakt egzekucji, historia gracza, ogłoszenia w grze, obserwacja sesji (M10 z #1643) |
| Support | Kolejka zgłoszeń, odwołania, widok konta bez danych wrażliwych, makra odpowiedzi |
| Redakcja | Newsy, strony, wydarzenia, media, wiki z przeglądem i wersjami, tłumaczenia |
| Operacje | Status światów, zarządzanie węzłami gry (M5 z #1643), wydania klienta (TUF), kopie zapasowe, monitoring, alerty |
| Sklep / finanse | Import cennika z gry (aktywacja jawną komendą), rekoncyliacja dostaw, zwroty i chargebacki, faktury |

## 3. Co bierzemy z obecnego portalu

Skala obecnego portalu (pomiar 2026-11-05, pliki śledzone w git):

| Część | Pliki | Linie |
| --- | --- | --- |
| Kod aplikacji PHP (`app/`) | 682 | ok. 56 tys. (ok. 44 tys. bez komentarzy i pustych linii) |
| Trasy, konfiguracja, migracje | 123 | ok. 6,4 tys. |
| Widoki Blade, CSS, JS | 166 | ok. 13,5 tys. |
| Tłumaczenia (`lang/`) | 33 | ok. 3,2 tys. |
| Testy | 262 | ok. 59 tys. |
| Gateway w Go | 16 | ok. 2,2 tys. |
| Narzędzia, skrypty, wdrożenie | ok. 355 | ok. 74 tys. |
| Dokumentacja | ok. 1000 | ok. 137 tys. |

Do przepisania jest ok. 80 tys. linii kodu produktu (bez testów, narzędzi i dokumentacji). Nie przepisujemy go linia w linię: część modułów odpada (sekcja 3), a testy powstają od nowa w Ruście.

| Obszar | Bierzemy | Zmieniamy | Nie przenosimy |
| --- | --- | --- | --- |
| Tożsamość | AccountId UUIDv7 (ADR 0028), hashe argon2id bez zmian, MFA TOTP, klucz odzyskiwania | Passkeys od startu, jeden ekran bezpieczeństwa konta | – |
| Logowanie do gry | OAuth PKCE, tickety, natywne admission z grantem Ed25519, zaufanie do kluczy podpisu | Gateway logowania w Ruście (D5 z #1643), sesje urządzeń jako pełny kontrakt | Gateway w Go po przełączeniu fali 2 |
| Dane publiczne | Projekcje gry (`/internal/v1/game-public/*`) | Czas aktualności widoczny przy każdej liczbie | Odczyty starego silnika (MySQL) |
| Treści | Model news, stron, wydarzeń, mediów; wiki z przeglądem; katalog gry z wersjami | Wiki generowana z paczki katalogu gry (M6 z #1643) | Szablony strony głównej |
| Sklep i portfel | Ledger coinów, kontrakt sklepu (wire v1), decyzje właściciela z 2026-11-05 | Transfer coinów jako funkcja ledgera Platform | Testowy provider płatności w produkcji |
| Support | Zgłoszenia, odwołania, egzekucja przez komendy | Kary wykonywane w grze, nie tylko jako rekordy webowe | – |
| Pobieranie | Polityka TUF i kanały (ADR 0035) | Launcher korzysta z Player API | – |
| Player Companion | Pomysł analizatora sesji | Część narzędzi (M7 z #1643) | Osobny moduł |

## 4. Architektura

### 4.1 Powierzchnie

| Powierzchnia | Konsument | Uwierzytelnienie | Forma |
| --- | --- | --- | --- |
| Web publiczny | Każdy | Brak lub sesja | SSR + htmx, cache na krawędzi dla anonimowych |
| Panel konta | Gracz | Sesja cookie, tryb sudo dla akcji wrażliwych | SSR + htmx, CSRF |
| **Player API `/api/v1`** | Klient gry, launcher, w przyszłości aplikacja mobilna | OAuth 2.1 PKCE i device flow; tokeny z zakresami | JSON, wersjonowany, OpenAPI |
| Game API `/internal/v1` | Serwer gry, gateway | mTLS (rustls w procesie) i tokeny serwisowe | Bajt w bajt zgodny z obecnym kontraktem (D4 z #1643) |
| Admin / GM / Ops | Zespół | Sesja, obowiązkowe MFA, RBAC, cztery oczy | SSR + htmx, osobny host |
| Worker | – | – | Outbox, importy, rekoncyliacje, e-maile, buildy Atlasa |

### 4.2 Binarki i hosty

- Główna binarka `oteryn-portal` ma role:
  - `serve-public`: web publiczny, panel konta i Player API;
  - `serve-internal`: Game API;
  - `serve-admin`;
  - `worker`;
  - `migrate`;
  - `ops`.

  Jedna binarka upraszcza wydania, a role pozwalają skalować i izolować sieciowo każdą powierzchnię osobno.
- Obok niej:
  - `oteryn-login-gateway`, logowanie do gry (fala 2);
  - `oteryn-node-agent` i `oteryn-node-supervisor` do zarządzania serwerami gry (rozszerzenie, M5 z #1643).
- **Hosty:**
  - Produkcja: `oteryn-vps` (OCI).
  - Preprodukcja i testowe węzły gry: `oteryn-vps-2`.

  `serve-internal` i `serve-admin` nie są wystawione publicznie. Admin działa za Cloudflare Access i OIDC.

### 4.3 Stos

Stos z #1643 (D3, „Wersje”): axum, askama, htmx, sqlx, PostgreSQL, rustls, tokio z łatkami wspólnymi z grą.

Dlaczego SSR + htmx, a nie SPA czy WASM:

- jeden język;
- brak bundlera i łańcucha npm;
- lepsze SEO i pierwsze wyświetlenie;
- mniejsza powierzchnia ataku.

Interaktywne elementy, które naprawdę tego potrzebują (mapa `oteryn-map`), są samodzielnymi komponentami.

### 4.4 Moduły i granice kodu

Workspace Cargo leży w `portal-rs/` w tym repozytorium. Dzielimy go na crate'y domenowe bez I/O i adaptery:

```
portal-rs/crates/
  domain-identity     konto, uwierzytelnienie, MFA, passkeys, sesje, RODO
  domain-gameauth     tickety, admission, urządzenia, zaufanie do kluczy
  domain-characters   intencje komend postaci, projekcje, nazwy
  domain-wallet       ledger coinów, transfer, rezerwacje
  domain-store        cennik, zakup, dostawa, zwroty
  domain-content      news, strony, wydarzenia, media, tłumaczenia
  domain-wiki         artykuły, przegląd, katalog gry, Atlas
  domain-community    rankingi, profile, gildie, domy, zgony, online
  domain-support      zgłoszenia, odwołania, kary
  domain-ops          światy, węzły, wydania klienta, flagi
  adapters-db         sqlx, migracje, outbox/inbox
  adapters-game       klient i serwer kontraktów /internal/v1 (crate'y kontraktowe z gry)
  adapters-payments   dostawca płatności za interfejsem (rdzeń nie zależy od dostawcy)
  web-public, web-account, web-admin, api-player, api-game
```

Granice sprawdza CI (zakaz krawędzi z domeny do adapterów, zakaz zależności konta od płatności). Kontrakty z grą pochodzą ze wspólnych crate'ów bez I/O w Oteryn-Game (`platform-wire`, `catalog`, `game-rules`; #1643, „Repozytorium”).

### 4.4a Współistnienie w jednym repozytorium

- `portal-rs/` ma własny `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` i `deny.toml`. Nie zmienia `composer.json` ani bramek PHP.
- CI uruchamia joby Rusta tylko przy zmianach w `portal-rs/` (filtry ścieżek), a joby PHP tylko przy zmianach w kodzie Laravela. Obie strony są wymagane przed merge, gdy ich ścieżki się zmieniły.
- Wspólne zostają: `docs/contracts/` (kontrakty i fixtures, testowane z obu stron), `docs/agents/` (zadania i governance), `deploy/` (osobne jednostki dla binarek Rusta).
- Bramka pokrycia dla Rusta ma własny próg w `docs/agents/CI_COVERAGE_POLICY.json`; obecny próg PHP się nie zmienia.
- Zadania agentów mają rozłączne zakresy plików: zadanie w `portal-rs/` nie zmienia Laravela i odwrotnie, poza jawnie nazwanymi kontraktami.

### 4.5 Dane

- PostgreSQL platformy na osobnej instancji z rolami właściciela migracji i runtime (jak ADR 0046). Baza gry jest osobna i należy do gry.
- Migracje są addytywne i odwracalne. Nie zakładamy pustej bazy produkcyjnej.
- Każda mutacja przez granicę gry zapisuje intencję w outboxie w tej samej transakcji co zmianę lokalną. Worker dostarcza ją co najmniej raz, a gra stosuje idempotentnie i odsyła potwierdzenie.
- Dane gry na portalu to projekcje tylko do odczytu, ze znacznikiem wersji i czasem aktualności.
- Waluta: ledger z blokadami wierszy, brak ujemnych sald, każda operacja ma klucz idempotencji.

### 4.6 Bezpieczeństwo

- **Hasła i logowanie:**
  - argon2id z rehashem przy zmianie parametrów;
  - sprawdzanie haseł w wyciekach (k-anonymity);
  - rate limity logowania, odzyskiwania i rejestracji;
  - blokada enumeracji kont.
- **Sesje:**
  - cookie `__Host-`, `SameSite=Lax` i CSRF dla mutacji;
  - rotacja sesji przy zmianie hasła, MFA i e-maila;
  - tryb sudo dla akcji wrażliwych.
- **Player API:** tokeny krótkotrwałe z zakresami, refresh z rotacją i wykrywaniem ponownego użycia, powiązanie z urządzeniem.
- **Game API:**
  - mTLS z allowlistą DN i TLS 1.3;
  - limity rozmiaru;
  - porównania w stałym czasie;
  - fail closed.
- **Admin:** obowiązkowe MFA, RBAC, audyt nie do edycji, cztery oczy dla kar, zwrotów i aktywacji cennika.
- **Sekrety:** tylko w menedżerze sekretów lub plikach środowiska na hoście, nigdy w repozytorium.

### 4.7 Jakość dla gracza

- **Budżety wydajności** (strony publiczne, telefon, sieć 4G):
  - LCP poniżej 2,0 s;
  - JS poza mapą poniżej 30 KB;
  - odpowiedź serwera p95 poniżej 150 ms dla stron z cache.
- **i18n:** pl i en z plikami Fluent, adresy z prefiksem języka, `hreflang`, brak mieszanych języków na jednej stronie.
- **SEO:** sitemap, dane strukturalne, kanoniczne adresy, przekierowania 301 ze starych adresów.
- **Dostępność:** WCAG 2.2 AA sprawdzane automatycznie (axe w Playwright) i ręcznie przed każdą falą.

### 4.8 Obserwowalność

- Logi strukturalne z identyfikatorem korelacji przechodzącym do gry i z powrotem.
- Metryki Prometheus, panele Grafana i alerty. Obejmują: opóźnienie outboxa, nieudane potwierdzenia, nieaktualne projekcje, błędy logowania, rozbieżności ledgera.
- Ślady (OpenTelemetry) dla ścieżek logowania do gry i zakupu.

## 5. Integracja z grą

### 5.1 Serwer gry

| Przepływ | Kierunek | Kontrakt | Stan na 2026-11-05 | Fala |
| --- | --- | --- | --- | --- |
| Logowanie do gry: ticket → gateway → admission | Klient → gateway → Platform → gra | V2_NATIVE_GATEWAY_LOGIN, FND-04B-AMEND-1 (Game #2910) | Logowanie natywne zaakceptowane tylko dla testing/preprod. FND-04B-AMEND-1 jest PROPOSED, reconnect zablokowany | 2 |
| Lista postaci konta | Gra → Platform | `native-account-characters` (LCFA) | Działa | 2 |
| Tworzenie i komendy postaci | Platform → gra (pull) | V2_CHARACTER_AUTHORITY_COMMAND, GAME-CHAR-CMD-0 | Zaakceptowane, wyłączone. Transfer: `CHAR_CMD_UNSUPPORTED` | 2 (tworzenie), 3 (transfer) |
| Premium | Gra czyta snapshot | V2_PREMIUM_TIME_SNAPSHOT | Działa | 2 |
| Loyalty | Gra czyta snapshot | V2_LOYALTY_SNAPSHOT | Proposed | 3 |
| Sklep: zakup w kliencie i dostawa | Gra ↔ Platform | V2_STORE_WIRE, V2_STORE_PURCHASE_DELIVERY, poprawka 2026-11-05 | Proposed; STORE-ENT-1 zablokowany | 3 |
| Dane publiczne (rankingi, profile, gildie, domy, zgony, online) | Gra → Platform | V2_PUBLIC_GAME_DATA_PROJECTION, GAME-PUBLIC-PROJ-0 | Zaakceptowany kontrakt; producent w grze PENDING | 1 (gdy producent jest gotowy) |
| Status światów | Gra → Platform | V2_RUNTIME_STATUS_PROJECTION, V2_WORLD_TOPOLOGY | Działa | 1 |
| Kary i ogłoszenia | Platform → gra | V2_GAME_ENFORCEMENT_COMMAND | Zaakceptowany; wykonanie w grze brak | 2 |
| Restore i rekoncyliacja | Gra → Platform | V2_GAME_RESTORE_RECONCILIATION | Zaakceptowany, domyślnie wyłączony | 2 |
| Katalog gry i wiki | Gra publikuje paczkę | OTERYN-GAME-PLATFORM-CATALOG-V1 | LOCKED; konsument Platform nieaktywny | 1 |
| Zarządzanie węzłami, obserwacja GM | Platform ↔ agent węzła, gra | #1643 M5 i M10 | Projekt w #1643 | Rozszerzenie |

Zasady wspólne dla wszystkich przepływów:

- Kontrakty pozostają w Game i Platform w obecnej formie. Przepisanie portalu nie zmienia wire.
- Każda zmiana kontraktu jest osobnym zadaniem z akceptacją obu stron i informacją dla koordynatora.
- Zestaw zgodności (#1643, „Repozytorium”) uruchamia obraz gry przypięty po digeście i przechodzi czarne skrzynki dla każdego wiersza tabeli powyżej. Przed każdą falą działa przeciw digestowi z produkcji.

### 5.2 Klient gry i launcher

Klient i launcher korzystają z **Player API** (`crates/platform-client` w Oteryn-Game już obsługuje OAuth PKCE, tickety, sesje urządzeń i listę postaci).

| Funkcja w kliencie | Endpoint Player API | Zasada |
| --- | --- | --- |
| Logowanie i zapamiętane urządzenie | OAuth PKCE, device sessions | Hasło nigdy nie trafia do klienta. Odświeżenie tokenu z rotacją |
| Lista i tworzenie postaci | `/api/v1/characters` | Tworzenie jako intencja z potwierdzeniem z gry |
| Wejście do gry | `/api/v1/game-auth/tickets` | Ticket krótkotrwały, jednorazowy |
| Newsy, wydarzenia, ogłoszenia | `/api/v1/feed` | Treść tylko do odczytu, cache, język gracza |
| Status światów | `/api/v1/worlds` | Ten sam model co na stronie, z czasem aktualności |
| Saldo, premium, historia | `/api/v1/account/summary` | Tylko do wyświetlenia. Autorytatywny zakup w grze idzie przez wire sklepu |
| Sklep w kliencie | wire sklepu (gra → Platform) i `/api/v1/store/offers` | Natywny UI w kliencie. Doładowanie coinów (płatność) zawsze w przeglądarce |
| Powiadomienia | `/api/v1/notifications` | Kary, odpowiedzi supportu, dostawy |
| Aktualizacje klienta | TUF (ADR 0035) | Launcher weryfikuje podpisy. Portal publikuje politykę kanałów |
| Głębokie linki | `oteryn://` i `https://oteryn.com/...` | Wiki, profil postaci i zgłoszenie otwierane z klienta |

Zasady:

- Brak wbudowanej przeglądarki w kliencie do płatności i logowania. Płatność i wrażliwe akcje konta przechodzą do przeglądarki systemowej, a powrót do klienta działa przez głęboki link.
- Każdy endpoint Player API ma wersję, OpenAPI, limity i testy kontraktowe uruchamiane też w CI klienta.

### 5.3 Wymagane od gry

Dostawy gry potrzebne dla fal (lista i numeracja GC1–GC16 w #1643, sekcja 11.2):

| Fala | Potrzebne od gry |
| --- | --- |
| Fala 1 | Producent projekcji publicznych (GAME-PUBLIC-PROJ-0, GC1), paczka katalogu (GC5), eksport świata Atlasa (GC16) |
| Fala 2 | Akceptacja FND-04B-AMEND-1 przez właściciela, wykonanie kar w grze (GC4), wspólne crate'y kontraktowe (GC6, GC8), konsument N4 |
| Fala 3 | STORE-ENT-1 i konsument DELIVER (sklep), transfer postaci (GC2), sklep w kliencie (GC13) |

## 6. Migracja i przełączenie (trzy fale)

Zamiast jednego przełączenia po ukończeniu wszystkiego (D7 w #1643) proponujemy trzy fale. W każdej chwili każda dana ma dokładnie jednego właściciela-pisarza.

| Fala | Co przechodzi na Rust | Laravel po fali | Dane | Rollback |
| --- | --- | --- | --- | --- |
| 1. Strona publiczna | Strona główna, newsy, wydarzenia, strony, wiki i katalog, Atlas, rankingi, profile, status światów, pobieranie | Konto, logowanie, admin i sklep pod ścieżkami `/account`, `/login`, `/admin` | Treści migrowane jednorazowo (Laravel przechodzi w tryb tylko do odczytu dla treści). Projekcje gry zasilane równolegle | Przełączenie routingu w Caddy z powrotem; treści eksportowane do Laravela |
| 2. Konto i logowanie do gry | Rejestracja, logowanie, MFA, panel konta, Player API, Game API, gateway logowania w Ruście, support, admin | Tylko sklep i płatności (jeśli używane) | Import tożsamości z zachowaniem AccountId i hashy argon2id, okno ≤60 min, sesje web unieważnione, tokeny klienta odświeżane | Powrót routingu i gatewaya Go; rekordy z okna eksportowane |
| 3. Handel | Portfel, sklep, płatności, bazar, transfer coinów | Wyłączony, przekierowania 301 | Ledger przenoszony po zamknięciu i uzgodnieniu sald; zero otwartych operacji przy przełączeniu | Ledger zamrożony; powrót możliwy tylko przed pierwszą operacją po przełączeniu |

Uzasadnienie:

- gracze dostają nowy portal po pierwszej fazie budowy, a nie po roku;
- ryzyko każdego przełączenia jest mniejsze;
- błędy logowania i ledgera nie blokują strony publicznej.

Koszt: przez okres przejściowy działają dwie aplikacje za jednym edge, a fala 1 wymaga wspólnego wyglądu nagłówka i stopki. Alternatywa (jedno przełączenie) zostaje decyzją właściciela w sekcji 10.

Runbook, przekierowania i rollback są dziedziczone z #1643 („Migracja i przełączenie”) i dzielone na fale.

## 7. Plan realizacji

Szacunki są przybliżone. Tydzień oznacza tydzień jednego developera. Faza kończy się bramką: CI zielone, zestaw zgodności zielony, przegląd bezpieczeństwa dla faz z kontem lub pieniędzmi, demo dla właściciela.

| Faza | Zakres | Bramka wyjścia | Szacunek |
| --- | --- | --- | --- |
| F0 Fundament | Katalog `portal-rs/` i workspace, CI (lint, testy, pokrycie, `cargo deny`), szkielet ról, migracje, obserwowalność, preprod na `oteryn-vps-2`, zestaw zgodności z obrazem gry | Pusty portal na preprodzie, zielone CI i zestaw zgodności | 4–6 |
| F1 Treści i strona publiczna | Layout, i18n, newsy, strony, wydarzenia, media, admin treści, RBAC i audyt, migracja treści | Redakcja publikuje na preprodzie | 6–8 |
| F2 Dane gry publicznie | Projekcje (rankingi, profile, gildie, domy, zgony, online), status światów, wiki z katalogu, pobieranie z TUF | **Fala 1 gotowa do przełączenia** | 6–9 |
| F3 Konto | Rejestracja, logowanie, MFA, passkeys, odzyskiwanie, sesje i urządzenia, RODO, e-maile | Przegląd bezpieczeństwa konta | 6–8 |
| F4 Gra i klient | Player API, gateway logowania w Ruście, admission, postacie, premium, kary, restore, support | **Fala 2 gotowa**, demo: logowanie z klienta i gra na węźle testowym | 8–11 |
| F5 Handel | Portfel, transfer coinów, sklep (cennik, zakup, dostawa), płatności za interfejsem, faktury, bazar | **Fala 3 gotowa**, rekoncyliacja ledgera na kopii produkcyjnej | 8–11 |
| F6 Rozszerzenia | Atlas i `oteryn-map` (MA), narzędzia (M7), zarządzanie węzłami (M5), obserwacja GM (M10), OIDC dla staffu | Każde rozszerzenie z własną bramką z #1643 | 14–20 |
| F7 Wygaszenie Laravela | Usunięcie ścieżek przejściowych i kodu Laravela z repozytorium (historia zostaje w git) | Zero ruchu na Laravel przez 30 dni | 1–2 |

Razem rdzeń (F0–F5) to ok. **38–53 tygodni pracy**, a z rozszerzeniami ok. **53–75**. Przy dwóch równoległych osobach fala 1 jest możliwa po ok. 8–11 tygodniach kalendarzowych od startu F0. Daty kalendarzowe ustala właściciel po wyborze składu zespołu.

Sposób pracy:

- Zadania dla agentów: jedna mechanika na zadanie, packet zadania z dozwolonymi plikami i komendami walidacji, wysyłane przez koordynatora.
- Każde zadanie, które dotyka gry, ma osobne zadanie po stronie gry i wpis w rejestrze kontraktów.
- Na każdej fazie właściciel dostaje krótkie demo na preprodzie.

## 8. Ryzyka

| Ryzyko | Skutek | Ograniczenie |
| --- | --- | --- |
| Opóźnienie dostaw gry (projekcje, kary, sklep) | Fala czeka | Fale niezależne; funkcja bez producenta pokazuje „wkrótce”, nie fałszywe dane |
| Rozjazd kontraktu wire | Błędy logowania lub dostaw | Crate'y kontraktowe z gry, złote wektory, zestaw zgodności per digest |
| Utrata lub rozjazd danych przy imporcie | Konta lub saldo nieprawidłowe | Próbne importy na kopii produkcyjnej, sumy kontrolne, rekoncyliacja przed i po |
| Dwie aplikacje w okresie przejściowym | Koszt utrzymania | Krótkie okna między falami; zamrożenie nowych funkcji w Laravelu od F1 |
| Zakres rozszerzeń | Opóźnienie rdzenia | Rozszerzenia dopiero po fali 2 i za osobnymi bramkami |
| Jedna osoba z uprawnieniami krytycznymi | Brak czterech oczu | Samozatwierdzenie z opóźnieniem i e-mailem do właściciela (#1643) |

## 9. Poza zakresem

- Zmiany protokołu gry, logiki rozgrywki i bazy gry.
- Wdrożenie produkcyjne przed akceptacją właściciela („deploy”).
- Prawdziwe płatności przed wyborem dostawcy, danymi sprzedawcy i bramką fali 3.
- Aplikacja mobilna (Player API jest na nią gotowe, ale budowa to osobna decyzja).

## 10. Decyzje właściciela

| # | Decyzja | Wartość domyślna |
| --- | --- | --- |
| P1 | Ten plan zastępuje szkic #1643 jako dokument główny; #1643 zostaje źródłem szczegółów dla M5, MA, M7 i M10 | Tak |
| P2 | Przełączenie | **Trzy fale** (sekcja 6). Alternatywa: jedno przełączenie jak w #1643 |
| P3 | Repozytorium | **Rozstrzygnięte 2026-11-05:** to samo repo `Oteryn/Oteryn-Platform`, katalog `portal-rs/`. Nowe joby CI dla Rusta w `.github/workflows` wymagają osobnej zgody właściciela |
| P4 | Zakres rdzenia | F0–F5. Rozszerzenia (F6) po fali 2 |
| P5 | Zamrożenie nowych funkcji w Laravelu | Od startu F1; tylko poprawki bezpieczeństwa i kontraktów |
| P6 | Skład zespołu | Do ustalenia; plan podaje szacunki dla 1 i 2 osób |
| P7 | Dostawca płatności i faktury | Jak w #1643 (Stripe, KSeF), decyzja najpóźniej na starcie F5 |
| P8 | Sklep w kliencie | Natywny UI w kliencie, doładowanie coinów tylko w przeglądarce |
| P9 | Języki | pl i en od startu |

## 11. Źródła

- Szkic #1643: `docs/architecture/OTERYN_PLATFORM_2_DESIGN_DRAFT.md` (rewizja 4.1, head 1c9c638).
- Kontrakty Platform: `docs/contracts/` (statusy z inwentarza 2026-11-05).
- Rejestr kontraktów gry: Oteryn-Game `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`. Polityka transportu: `PROTOCOL_OTERYN_TRANSPORT_POLICY.json`, rewizja 4 (tylko TCP+TLS 1.3).
- Architektura obecna:
  - `docs/architecture/DATA_OWNERSHIP.md`, `SECURITY_ARCHITECTURE.md`, `PLATFORM_API_ARCHITECTURE.md`, `CLIENT_DISTRIBUTION_ARCHITECTURE.md`;
  - ADR 0028, 0031, 0035, 0046.
- Kierunek produktu: `docs/architecture/PORTAL_PLAYER_EXPERIENCE_COMPLETION_DIRECTION.md`.
- Raporty: `docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md`, `OTERYN-20260810-portal-architecture-product-review.md`.
- Decyzje sklepu: `docs/contracts/OTERYN_V2_STORE_OWNER_DECISIONS_AMENDMENT_2026-11-05.md`.
