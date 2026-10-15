# Oteryn Platform 2 (Rust) – projekt architektury

> **Status: DRAFT** (2026-10-15). Projekt do akceptacji przez właściciela; nic tu nie jest jeszcze zobowiązaniem ani kontraktem.
> Źródłem prawdy jest ten plik w repozytorium. Wersja robocza na żywo (podgląd, może być o krok z tyłu): https://claude.ai/code/artifact/e809ac5a-3f4c-42c2-91b6-b4eaa489b950

Oteryn Platform 2 to nowa, kompletna platforma w Ruście: strona, konto i postacie gracza, wiki z katalogu gry, narzędzia dla gracza, administracja, zarządzanie serwerami gry i podgląd sesji gracza dla GM-a. Powstaje w osobnym repozytorium `Oteryn-Platform-2`. Główna binarka `oteryn-platform` ma role (`serve-public`, `serve-internal`, `worker`, `migrate`, `ops`); do tego dochodzą dwie małe binarki do zarządzania serwerami: `oteryn-node-agent` i `oteryn-node-supervisor`. Stos: axum + askama + htmx 4 + PostgreSQL. Kontrakt `/internal/v1` z serwerem gry zostaje bajt w bajt. Budujemy platformę obok obecnej, bez strangler; gdy będzie kompletna i przetestowana na własnym preprodzie, przełączamy ją jednym przełączeniem zamiast Laravela.

## Najważniejsze

- **Zmienia się repozytorium.** Rekomendacja: osobne repo `Oteryn-Platform-2`. Trzy crate'y kontraktowe bez I/O są w Oteryn-Game i wchodzą do v2 jako zależności git przypięte do tagu.
- **Jedno przełączenie, po M6.** Praca przed przełączeniem to ok. 27–36 tygodni dla jednego developera albo ok. 16–20 tygodni dla dwóch.
- **Repozytorium gry musi najpierw dostarczyć dużo.** Lista GC1–GC10 jest w sekcji „Roadmapa”. Najważniejsze pozycje:
  - producent Public Projections v1;
  - komendy postaci v2 (zmiana nazwy, usunięcie, ukrycie, tworzenie v2);
  - nowa projekcja szczegółów postaci, tylko dla właściciela;
  - Operator Commands v1 (broadcast, kick, zapis i wyłączenie);
  - Catalog Export v2 z paczką assetów;
  - wydzielony crate ze wspólnymi formułami;
  - GM observe (GC10, po przełączeniu).
- **Decyzje właściciela potrzebne teraz:**
  - licencja na publikację sprite'ów gry na wiki (grafika pochodzi z klienta Tibia 15.30);
  - polityka klucza odzyskiwania;
  - okresy karencji przy usuwaniu;
  - dostawca płatności (nie blokuje, bo handel przechodzi na po przełączeniu).
- **Niezweryfikowane:** czy slice'y chisel od Canonical istnieją dla Ubuntu 26.04 (GitHub był w tej sesji zablokowany). Jest sprawdzony plan awaryjny (sekcja „Stack technologiczny”).

## Decyzje

| # | Decyzja |
| --- | --- |
| D1 | **Osobne repozytorium `Oteryn-Platform-2`**, nowy workspace Rust z tą samą polityką lintów co gra. Trzy wspólne crate'y żyją w Oteryn-Game: `oteryn-platform-wire`, `oteryn-catalog` i `oteryn-game-rules`. Platform 2 bierze je jako zależności `git` przypięte do tagu (`contracts-vX.Y.Z`). Patrz „Repozytorium”. |
| D2 | Trzy binarki z Platform 2:<br>- `oteryn-platform`, z rolami `serve-public`, `serve-internal`, `worker`, `migrate`, `ops`;<br>- `oteryn-node-agent`, bez uprawnień, tylko ruch wychodzący;<br>- `oteryn-node-supervisor`, lokalny pomocnik roota z zamkniętym zestawem komend i bez sieci. |
| D3 | Stack: axum 0.8, askama 0.16, **htmx 4.0**, sqlx 0.9 i PostgreSQL. Ciężkie obliczenia liczy **serwer w natywnym Ruście**. Mały wendorowany plik JS służy tylko do przeglądarki mapy. Bez WASM w MVP. |
| D4 | `/internal/v1/*` zostaje bajt w bajt. **Preprod Platform 2 ma własne wewnętrzne CA, certyfikaty, tokeny bearer i klucze admission.** Przy przełączeniu produkcyjny Platform 2 ładuje **istniejące produkcyjne CA, certyfikat serwera, allowlistę DN, hashe bearerów i klucze Ed25519**, więc konfiguracje produkcyjnej gry i gatewaya się nie zmieniają. mTLS kończy się w procesie (rustls) zamiast nginx. |
| D5 | Gateway w Go zostaje na produkcji. Preprod v2 ma własną instancję gatewaya z tego samego kodu, podpiętą do v2. Port do Rusta dopiero po przełączeniu, z osobnym ADR. |
| D6 | Osobne instancje PostgreSQL, jak dotąd. Platform 2 buduje się na **PostgreSQL 18.6 już teraz**. Preprod przechodzi na **PostgreSQL 19** po jego GA. Przełączenie produkcyjne działa na 19.x, jeśli 19 jest GA od co najmniej 6 tygodni przed bramką; w przeciwnym razie na 18.x. Platforma zna dane gry tylko z projekcji pchanych przez grę. |
| D7 | **Bez strangler.** Platform 2 powstaje jako kompletny produkt, budowany i weryfikowany równolegle na własnym preprodzie: własna baza, certyfikaty, instancja gatewaya Go i testowy węzeł gry. Laravel działa bez zmian. **Jedno przełączenie** importuje dane, przejmuje `172.18.0.12:8543` i przestawia upstream Caddy. |
| D8 | Cała integracja z Canary i Player Companion do usunięcia, bez portowania. |
| D9 | Hasła: argon2id; istniejące hashe PHC weryfikowane bez zmian, rehash przy logowaniu. Laravelowy APP\_KEY zostaje jako klucz legacy do HMAC i odszyfrowania sekretów 2FA. |
| D10 | **Najszybsze odblokowanie właściciela:** demo M2 na preprodzie v2. Właściciel loguje się przez gateway v2, tworzy postać i gra na testowym węźle v2, a jedynym autorytetem jest Platform 2. Cel: ok. tydzień 8. |
| D11 | **Nowe: zarządzanie serwerami gry.** Agent węzła pobiera komendy z platform-internal przez mTLS (platforma nigdy nie łączy się do hosta). Komendy agenta są podpisane przez platformę. Supervisor roota wykonuje tylko zamknięty zestaw czasowników. Wdrożenia obrazów muszą być **podpisane przez wydanie**. Patrz „Zarządzanie serwerami gry”. |
| D12 | **Nowe: wiki i narzędzia z katalogu gry.** Każde wydanie gry publikuje podpisaną paczkę katalogu (catalog JSON v2, sprite'y webp, kafelki minimapy, wektory testowe reguł) jako artefakt OCI. Platform 2 importuje ją jako niezmienny snapshot, a aktywacja i rollback to przestawienie jednego wskaźnika. Patrz „Wiki z katalogu gry”. |
| D13 | **Nowe: formuły.** Logika formuł jest wydzielona do `oteryn-game-rules` (czysta, deterministyczna, na liczbach całkowitych) i współdzielona przez serwer gry i Platform 2. Stałe pochodzą z katalogu. Wartość rules-ABI i złote wektory testowe nie pozwalają rozsynchronizowanej platformie aktywować katalogu. Patrz „Narzędzia dla gracza”. |
| D14 | **Nowe: panel konta.** „Tryb sudo” (ponowne uwierzytelnienie) chroni każdą wrażliwą akcję. Cykl życia postaci zostaje po stronie gry i przechodzi przez komendy postaci v2 oraz projekcję szczegółów postaci. Jak najwięcej wchodzi do MVP. Patrz „Panel konta i postaci”. |
| D15 | **Nowe: najnowszy toolchain.**<br>- Rust 1.99.1 (MSRV 1.99), runnery i obrazy builderów na Ubuntu 26.04, runnery CI przypięte jawnie do `ubuntu-26.04` / `ubuntu-26.04-arm`, nigdy `ubuntu-latest`;<br>- obraz uruchomieniowy: chiselled Ubuntu 26.04 albo distroless debian13 z builderem trixie;<br>- wersje: htmx 4.0.0, axum 0.8.9, sqlx 0.9.0, rustls 0.23.46;<br>- toolchain workspace'u gry podbijamy do 1.99.1, a jego CI do `ubuntu-26.04`. |

## Stan obecny

Z grą działa natywnie tylko kilka ścieżek: tworzenie i lista postaci, logowanie do gry, premium i status światów. Wszystko, co pokazuje dane z gry na stronie, czyta Canary albo pokazuje „niedostępne”.

| Obszar | Laravel dziś | Stan natywny |
| --- | --- | --- |
| Highscores, profil, zgony, online, gildie | Połączenie `canary`; przy `native` pokazuje „niedostępne” | Ingest w Laravelu jest, **producenta w grze nie ma** |
| Bazar / marketplace | `CanaryCharacterTransfer` | Gra wykonuje tylko `CreateCharacter`; Transfer zwraca `CHAR_CMD_UNSUPPORTED` |
| Overview konta | `IdentityCanaryAccount` (Canary) | Brak |
| Tworzenie postaci | Komenda `CreateCharacter`, gra pobiera `/character-commands/pending` | **Działa** (wysyłane są tylko nazwa i świat) |
| Lista postaci | Projekcja LCFA `/native-account-characters` | **Działa** |
| Logowanie do gry | OAuth PKCE → ticket → gateway Go → `/native-admissions` | **Działa** |
| Premium | Ledger czasu → `/premium-snapshots/read` | **Działa** |
| Status światów | LiveOps z natywnego read modelu | **Działa** |
| Gildie | Brak | Gra nie ma tabel gildii (GUILD-0 to kandydat) |
| Moderacja (ban/mute/kick/broadcast) | Tylko rekordy webowe | W grze brak; ban konta tylko przez evidence (`disabled_at` + generation) |
| Płatności | Tylko testowy provider | Brak prawdziwego |

## Kontrakt gra ↔ platforma

Gra jest zawsze klientem mTLS; platforma nigdy nie łączy się z grą. Te ścieżki muszą działać identycznie przed i po przejściu na Rusta.

| Ścieżka `/internal/v1/...` | Wywołujący | Cel |
| --- | --- | --- |
| `game-auth/native-admissions` | gateway Go (bearer, hash SHA-256 current + previous) | Decyzja o wpuszczeniu dla ticketu |
| `game-auth/native-evidence` | gra | ReadAccountSecurityV1, ReadFreshSigningTrustV1, ReadRecovery…V2 |
| `game-auth/native-runtime-status` | gra | Raport stanu runtime |
| `game-auth/native-scope-assignments` | gra | Raport przypisań scope |
| `game-auth/native-scope-revocations` | gra | Jest w grze, brak w trasach Laravela — do sprawdzenia i zaimplementowania |
| `game-auth/native-account-characters` (+ `/watermark`) | gra | Snapshot LCFA (≤64 postacie, ≤16 KB); watermark co 3 s, max przerwa 10 s |
| `game-auth/character-bootstrap-intents/read` | gra | Odczyt intencji bootstrap |
| `game-auth/character-commands/{pending,read,receipt}` | gra | Kolejka komend, polling co 1 s, strona 32 |
| `products-entitlements/premium-snapshots/read` | gra | Snapshot premium |
| `game-auth/game-restore/notice` | gra | Powiadomienie o restore |
| `game-auth/character-commands/*` (nowe pary, GC2) | gra | Zmiana nazwy, usunięcie z karencją i anulowanie, widoczność, tworzenie v2 |
| `game-account/character-details` (+ `/watermark`) (GC3, nowe) | gra | Projekcja szczegółów postaci, tylko dla właściciela |
| `operator-commands/{pending,receipt}` (GC4, nowe) | gra | Broadcast, kick, mute, namelock, SaveAndShutdown (także StartObserve/EndObserve) |
| `node-agent/{commands/pending,heartbeat,receipts,logs,csr}` (nowe) | node-agent | Zarządzanie serwerami gry; patrz „Architektura” |
| `gm-observe/frames` (nowe) | gra (osobny certyfikat) | Zsanityzowane klatki podglądu sesji gracza przez GM |
| `game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}` | gra (jeszcze nie zbudowane) | Public Projections v1 |

Limity klienta gry: connect 1 s, handshake 2 s, wymiana 3 s, 2 sloty pipeline. p99 platformy na tych trasach musi być wyraźnie poniżej 1 s. Każdy cel ma własny certyfikat klienta (np. `CN=oteryn-preprod-character-projection`).

Publiczne API dla launchera/klienta (`crates/platform-client`): `/oauth/authorize`, `/oauth/token` (PKCE S256, scope `game:ticket`), `POST /v1/game-auth/tickets`, `/v1/game-auth/device-sessions/*`, `GET /v1/game-auth/native-characters`, `GET /v1/client/directory`.

## Repozytorium

**Rekomendacja: osobne repo `Oteryn-Platform-2`.**

Bilans w czterech punktach:

1. Za osobnym repo: niezależny produkt i rytm wydań, szybkie CI bez buildów gry, brak reguł granic i polityki zakazanych nazw gry w kodzie webowym, jasna odpowiedzialność.
2. Przeciw osobnemu repo: rozjazd kontraktu nie jest już wyłapywany jednym `cargo test`.
3. Monorepo wychwytywałoby rozjazd w czasie kompilacji, ale wiązałoby każdą zmianę webową z ciężkim CI gry i jej procesem review.
4. Rozwiązanie: crate'y kontraktowe zostają w repo gry i są brane po tagu, a zestaw zgodności uruchamia **prawdziwy, przypięty obraz gry** przeciw Platform 2. Razem odzyskuje to większość bezpieczeństwa monorepo.

Zasady:

- **Wspólne crate'y** w Oteryn-Game, wszystkie bez I/O:

  | Crate | Zawartość |
  | --- | --- |
  | `crates/platform-wire` | Typy wire runtime, ścieżki, limity i kody błędów. Wyciągnięte z `native_admission_source/*`, `premium/client.rs` i `admission_evidence.rs`. |
  | `crates/catalog` | Pakiet `oteryn-catalog`: typy koperty katalogu v1/v2, kanoniczny JSON, digest i walidacja. |
  | `crates/game-rules` | Pakiet `oteryn-game-rules`: czyste formuły. |

  - Każdy dostaje `[package.metadata.oteryn]` i jawne `[edges]` w `workspace-boundaries.toml`. `oteryn-game-server` dostaje krawędzie do wszystkich trzech.
  - Żadna nazwa nie zawiera zakazanego fragmentu.
- **Dozwolone zależności wspólnych crate'ów:** tylko `serde`, `serde_json`, `sha2`, `ed25519-dalek` oraz `foundation` / `error-codes` z repo. **Bez tokio, sqlx i reqwest.**
  - Powód: `[patch.crates-io]` gry (wendorowane tokio 1.53.1, sqlx-core 0.9.0, sqlx-postgres 0.9.0) **nie działa** dla konsumentów git. Zależność przechodnia po cichu podmieniłaby je na niezałatane crate'y upstream.
- **`oteryn-game-rules` musi się kompilować dla `wasm32-unknown-unknown`.** Dochodzi job kontrolny w CI, żeby ewentualna przyszła opcja WASM była tania.
- **CI Platform 2:** token GitHub App albo deploy key z dostępem do odczytu do prywatnego repo Oteryn-Game, dla zależności git.
- **Zgodność** (`tests/conformance` w Platform 2): Docker Compose uruchamia obraz gry przypięty po digeście, jego własny PG gry i Platform 2. Przechodzi czarne skrzynki: admission, evidence, LCFA, komendy, premium, projekcje publiczne, komendy operatora i agent węzła. Sprawdza też złote wektory wire eksportowane przez `oteryn-platform-wire`.
- Przed przełączeniem test zgodności idzie jeszcze raz przeciw dokładnemu digestowi obrazu gry z produkcji.

Zmiany w Oteryn-Game: trzy nowe crate'y i krawędzie `oteryn-game-server` → każdy z nich. Do naprawy przy okazji: `rust-toolchain.toml` ma 1.94.0, a CI używa `+1.95.0` (patrz D15).

## Architektura

Ta sama binarka `oteryn-platform` uruchamia dwa serwery HTTP i workera; każdy w osobnym kontenerze. `platform-internal` ma własny listener, trust store i limity zasobów, więc ruch publiczny nie zje 1-sekundowego budżetu gry. Przejmuje też IP i port nginx mTLS (172.18.0.12:8543), więc konfiguracje gry i gatewaya się nie zmieniają. Na każdym hoście gry działają dodatkowo `oteryn-node-agent` (bez uprawnień) i `oteryn-node-supervisor` (root, bez sieci).

### Kontenery przed i po przełączeniu

```text
PRZED PRZEŁĄCZENIEM (oteryn-vps)
 caddy edge :443 ──> oteryn.com            ──> Laravel platform:8000   (bez zmian)
                 ──> login./gateway.        ──> Go gateway (prod)       (bez zmian)
                 ──> v2.oteryn.com [CF Access] ─> p2-public (preprod)
                 ──> login-v2.oteryn.com    ──> Go gateway (instancja preprod v2)
 nginx internal-mtls 172.18.0.12:8543 <── produkcyjny węzeł gry (systemd + game-supervisor.py)
 sieć preprod v2 172.19.0.0/24: p2-public, p2-internal:8543, p2-worker, p2-postgres (18/19)
 sieć testowej gry v2 172.19.1.0/24: testowe węzły gry :7272.., testowy game-postgres,
                                     node-agent + node-supervisor (zapisane do preprodu v2)

PO PRZEŁĄCZENIU
                       Internet / Cloudflare
                               |
                    caddy edge :443 (ten sam kontener)
        oteryn.com |           | login./gateway.        | TCP 7172.. (per kanał)
                   v           v                        v
          p2-public:8000   Go gateway:8080        węzły gry (docker, logi w journald)
                   |           | bearer                ^ start/stop/deploy przez
                   |           v                       | node-supervisor (root, bez sieci,
                   |   p2-internal 172.18.0.12:8543    |  zamknięty zestaw czasowników,
                   |   (rustls mTLS, w procesie)       |  gniazdo unix)
                   |       ^      ^   ^                | typowane czasowniki podpisane przez platformę
                   |  gra  |  gw  |   | node-agent ----+ (bez uprawnień; tylko wychodzące mTLS:
                   |  mTLS |      |   |  pobiera komendy, wysyła status/logi)
          +--------v-------+------+---+--+     +-------------+
          | platform PG (18/19)          |<----| p2-worker   |--> ghcr.io (paczki katalogu)
          | z partycjami logów ops       |     | joby/outbox |--> SMTP, OCI Object Storage
          +------------------------------+     +-------------+
          game PG (osobna instancja, należy do gry; platforma nigdy się nie łączy)
          cdn.oteryn.com -> Cloudflare -> OCI Object Storage (sprite'y, kafelki, rendery strojów)
```

Worker obsługuje: e-maile, wysyłkę outboxa, czyszczenie retencji, finalizację usunięć kont i postaci, eksporty GDPR, pobieranie paczek katalogu, rotację partycji logów, alarmy świeżości projekcji, retencję nagrań GM observe, później bazar i webhooki płatności.

### Przepływy

Wszystkie wywołania z tabeli idą **z** gry, agenta albo workera. Nic nie łączy się do hosta gry.

| Przepływ | Kierunek | Mechanizm |
| --- | --- | --- |
| Admission | gateway → platform-internal | HTTP + bearer serwisowy |
| Evidence, status, scope | gra → platform-internal | HTTP/1.1 mTLS, certyfikat per cel |
| Komendy postaci | gra **pobiera** pending | Polling co 1 s, idempotentne potwierdzenia |
| Komendy postaci v2 (GC2) | gra pobiera istniejące `/character-commands/*` | Nowe pary komenda/wariant |
| Projekcje (LCFA, publiczne, premium) | gra **pcha** snapshoty + watermarki | Upsert monotoniczny po rewizji |
| Szczegóły postaci, tylko dla właściciela (GC3) | gra → p2-internal `POST .../game-account/character-details` (+ `/watermark`) | Push snapshotu, monotoniczny po rewizji |
| Komendy operatora (GC4): broadcast, kick, mute, namelock, SaveAndShutdown | gra → p2-internal `GET .../operator-commands/pending`, `POST .../receipt` | Ta sama konstrukcja kolejki co komendy postaci |
| Komendy węzła | node-agent → p2-internal `GET /internal/v1/node-agent/commands/pending` | Long poll 25 s. Certyfikat agenta `CN=oteryn-<env>-node-agent-<host>`. |
| Status, zdrowie i potwierdzenia węzła | node-agent → p2-internal `POST .../node-agent/{heartbeat,receipts}` | Heartbeat co 5 s |
| Logi węzła | node-agent → p2-internal `POST .../node-agent/logs` | Paczki ≤256 KB, zstd, z zamaskowanymi sekretami |
| Certyfikaty węzła | node-agent → p2-internal `POST .../node-agent/csr` | Online pośrednie CA z ograniczeniem nazw |
| Paczki katalogu | p2-worker → ghcr.io (pull) | Artefakt OCI + podpis wydania Ed25519 |
| Klatki GM observe | gra → p2-internal `POST /internal/v1/gm-observe/frames` | Osobny certyfikat klienta i pula połączeń; patrz „Podgląd sesji gracza przez GM” |
| Gracz / launcher | klient → platform-public | OAuth PKCE, tickety, directory |

Tożsamość rozmówcy = DN certyfikatu + pin SPKI, mapowane z allowlisty na enum `Peer`. Każda trasa deklaruje, które `Peer` przyjmuje.

## Własność danych i spójność

Postacie i stan gry należą do serwera gry; platforma trzyma tylko ich projekcje i wydaje komendy. Konta, logowanie, płatności i treści należą do platformy.

| Dane | Właściciel | Co ma platforma |
| --- | --- | --- |
| Konta, hasła, 2FA, sesje, OAuth, device sessions | Platforma | Źródło prawdy |
| Generation bezpieczeństwa, disabled/terminated, floors evidence | Platforma | Źródło prawdy, monotoniczne |
| Klucze admission Ed25519 (current + retiring) | Platforma (pliki) | Źródło prawdy |
| Światy, kanały, trasy, topologia, directory epoch | Platforma (rejestr) | Źródło prawdy |
| Komendy postaci | Platforma wydaje | Intencja + kopia potwierdzenia |
| Postacie (nazwy, progres, itemy, domy, bank) | Gra (ADR-0012) | Tylko projekcje |
| Premium | Platforma | Źródło prawdy; gra czyta snapshoty |
| Portfel, zamówienia, płatności | Platforma | Ledger podwójnego zapisu |
| CMS, wiki, eventy, pobieranie, wydania updatera, support, audyt | Platforma | Źródło prawdy |
| Gildie | Gra (GUILD-0, przyszłość) | Projekcja + komendy z weba |
| Highscores, zgony, online | Gra | Projekcja publiczna |
| Hosty, agenci, węzły, stan oczekiwany, wdrożenia, maintenance | **Platforma** (rejestr ops) | Źródło prawdy |
| Stan rzeczywisty węzła, zdrowie procesów | Obserwowany przez agenta | Projekcja statusu |
| Logi gry | Gra emituje, platforma przechowuje | Kopia z retencją (14 dni prod, 7 dni preprod) |
| Snapshoty katalogu, sprite'y, kafelki, wektory reguł | **Gra** (per wydanie) | Niezmienne zaimportowane kopie |
| Opisy wiki, nakładka tłumaczeń, kategorie redakcyjne | **Platforma** | Źródło prawdy |
| Postęp bestiary/charmów/osiągnięć/strojów | **Gra** | Projekcja tylko dla właściciela (GC3) |
| Ręczne wpisy trackera, zapisane sesje narzędzi | **Platforma** (dane użytkownika) | Źródło prawdy, opt-in |
| Ustawienia postaci: główna postać, komentarz, prywatność renderowania | **Platforma** | Źródło prawdy |
| Widoczność postaci (ukryta), cykl życia (zmiana nazwy/usunięcie/transfer) | **Gra** | Intencje komend + potwierdzenia |
| Wnioski, zatwierdzenia i nagrania GM observe | **Platforma** | Źródło prawdy (klatki pochodzą z gry) |

Bez kluczy obcych między bazami; referencje do gry to nieprzezroczyste typowane ID (`CharacterRef`, `WorldRef`).

Reguły spójności:

1. **Monotoniczne watermarki.** Każdy wiersz projekcji ma `(source_authority, epoch, revision)`; zapis tylko jeśli nowszy, cofnięcia odrzucane. UI pokazuje „nieaktualne od” po przerwie > 10 s.
2. **Idempotentne komendy.** `command_id` = UUIDv7 jako klucz główny; potwierdzenia zapisywane raz. UI pokazuje „tworzenie…” do potwierdzenia.
3. **Transakcyjny outbox.** Każda zmiana, która ma dotrzeć do gry, zapisuje się w tej samej transakcji co wiersz biznesowy. Bez podwójnych zapisów.
4. **Liczniki nigdy wstecz** (generation, floors, witness): `CHECK` + `UPDATE … WHERE new > old`. Import i reverse-import przenoszą te wartości.
5. **Ledger** tylko dopisywany, podwójny zapis, suma = 0 na transakcję (deferred constraint trigger). Korekty to storna, nigdy edycja.
6. **Rezerwacje nazw** należą do gry; platforma robi tylko wstępne sprawdzenie, decyduje potwierdzenie gry.

## Bezpieczeństwo i i18n

| Obszar | Rozwiązanie |
| --- | --- |
| Hasła | argon2id m=19456, t=2, p=1 (jak `config/hashing.php` Laravela). Stare hashe PHC weryfikowane bez zmian, `needs_rehash` przy logowaniu. |
| 2FA | `totp-rs`. Sekrety odszyfrowane z laravelowego `encrypted` (AES-256-CBC + HMAC, APP\_KEY) i zaszyfrowane na nowo XChaCha20-Poly1305 z key ringiem. Kody odzyskiwania zostają jako hashe argon2id. |
| Stare keyed hashe | `IdentitySecret::keyedHash` (HMAC-SHA256 z APP\_KEY) jako `LegacyKeyedHash` dla istniejących device sessions; nowe wiersze z nowym kluczem i kolumną wersji. |
| Sesje | Po stronie serwera w PG, podpisane ciasteczko `__Host-` (Secure, HttpOnly, SameSite=Lax), rotacja ID po logowaniu i zmianie uprawnień. |
| CSRF | Token synchronizera na każdym formularzu nie-GET (htmx w nagłówku) + sprawdzenie Origin/Referer. |
| Rate limity | Kubełki per IP (tylko z zaufanego łańcucha Cloudflare/Caddy) i per konto: logowanie, rejestracja, reset hasła, 2FA, tickety, oauth/token; eskalacja do blokady z wpisem audytu. |
| OAuth | Własny minimalny serwer OAuth2: tylko authorization code + PKCE S256, tokeny hashowane, te same ID klientów i scope. Dostawca OIDC w M10. |
| mTLS | rustls 0.23 (aws-lc-rs), tylko TLS 1.3, osobne CA klientów, allowlista DN + SPKI, przeładowanie certyfikatów na SIGHUP. |
| Sekrety | Pliki w `/run/secrets/*`, tryb 0400, nigdy zmienne środowiskowe; loader odrzuca pliki czytelne dla grupy lub wszystkich. |
| Tryb sudo | Ponowne wpisanie hasła lub TOTP (ważne 10 minut, związane z sesją) dla wrażliwych akcji konta; krótszy próg 5 minut dla akcji ops. |
| Podpisy komend | Komendy węzła podpisuje klucz ops Ed25519 (osobny od klucza admission); wdrożenia obrazów wymagają dodatkowo podpisu wydania z CI gry. |
| Audyt | Tylko dopisywana tabela `audit_events` (aktor, peer, akcja, cel, digest przed/po, request ID, IP), łańcuch hashy per dzień. |
| Nagłówki | Restrykcyjne CSP (bez inline JS), HSTS, frame-ancestors none, Referrer-Policy. |
| Łańcuch dostaw | `cargo deny` / `cargo audit` w CI, przypięte zależności, SBOM do obrazów. |

**i18n:** zostają adresy `/{locale}/...` z `en` i `pl`; `/` wybiera język z Accept-Language, potem ciasteczka, domyślnie `en`. Tłumaczenia w Fluent (`locales/{en,pl}/*.ftl`) — ważne dla polskich form mnogich. `lang/en`, `lang/pl` i `pl.json` konwertuje jednorazowy skrypt. CI odrzuca brakujące klucze w którymkolwiek języku.
## Stack technologiczny

Wersje sprawdzone 15.10.2026.

| Komponent | Wybór | Dowód | Konflikt z pinami gry / działanie |
| --- | --- | --- | --- |
| Rust | **1.99.1** (wydany 2026-10-15; 1.99.0 z 2026-10-01), MSRV 1.99 | blog.rust-lang.org/releases; na Docker Hub są tagi `rust:1.99.1-*` | Gra ma `rust-toolchain.toml` 1.94.0, a CI używa `+1.95.0`. **Podbić grę do 1.99.1** jednym PR. Zarezerwować 1–2 dni: `clippy::all = deny` oznaczy nowe linty. |
| Runnery CI | **`ubuntu-26.04`** (x64) i **`ubuntu-26.04-arm`** (natywne arm64), GA od 2026-09-17 | changelog GitHuba z 2026-09-17 | CI gry przypina `ubuntu-24.04` (rust.yml, linie 16, 65, 157). Podbić. `ubuntu-latest` przechodzi na 26.04 między 19.10 a 19.11.2026; **przypiąć jawnie**. |
| Obraz buildera | `ubuntu:26.04` + rustup 1.99.1 | tag `ubuntu:26.04` istnieje na Docker Hub | brak |
| Obraz uruchomieniowy | **Podstawowy:** rootfs chiselled Ubuntu 26.04 budowany w Dockerfile (`chisel cut --release ubuntu-26.04 base-files_base ca-certificates_data libc6_libs libgcc-s1_libs`), użytkownik 65532.<br>**Awaryjny:** `gcr.io/distroless/cc-debian13:nonroot`. | Indeks distroless sprawdzony: amd64, arm64 i inne. Gałąź wydania chisel dla 26.04 jest **niezweryfikowana** (GitHub był zablokowany). | Przy wariancie awaryjnym builder też na Debianie 13 (`rust:1.99.1-slim-trixie`). Binarka zbudowana na 26.04 linkuje nowszy glibc niż trixie i może nie wystartować na debian13. **Nigdy nie mieszać buildera Ubuntu 26.04 z runtime debian13.** |
| PostgreSQL | **18.6** teraz, **19** po GA (RC1 wydane 2026-10-15) | strona główna postgresql.org | Produkcyjny PG gry to 17.11, osobna instancja należąca do gry, bez konfliktu. Zespół gry powinien osobno zaplanować 18 lub 19. |
| axum | **0.8.9** (0.9 nie istnieje) | crates.io | brak (gra nie ma frameworka HTTP) |
| tower-http | 0.7.1 | crates.io | brak |
| tokio | 1.53.3 | crates.io | Gra wendoruje załatane 1.53.1. Platform 2 używa upstream 1.53.3. Wspólne crate'y nie mogą zależeć od tokio. |
| sqlx | 0.9.0 (najnowszy) | crates.io | Gra wendoruje załatane sqlx-core/postgres 0.9.0; powód nie jest udokumentowany w drzewie. Platform 2 używa upstream. **Zapytać zespół gry, czy łatka naprawia błąd, którego potrzebuje też Platform 2.** |
| rustls | 0.23.46 (2026-10-14), aws-lc-rs | crates.io | Jak w grze |
| reqwest | 0.13.5 | crates.io | Gra przypina 0.13.4; nieszkodliwe |
| askama | 0.16.1 (MSRV 1.88) | crates.io | brak |
| htmx | **4.0.0** (2026-08-28) | ogłoszenie four.htmx.org; InfoQ | Zmiany łamiące względem 2.x: API fetch, dziedziczenie atrybutów tylko jawnie (`:inherited`), zmienione nazwy zdarzeń. Greenfield, więc od razu 4.x. Plik wendorowany z hashem SRI (npm `latest` wciąż wskazuje 2.x). |
| argon2 / totp-rs / lettre / fluent-templates | 0.6.0 / 6.0.0 / 0.11.23 / 0.15.1 | crates.io | brak. Duplikaty crate'ów krypto (argon2 0.6 obok ed25519-dalek 2.2) dopuszczone w `cargo deny`. |
| Caddy (edge) | 2.11.7 jest aktualny | Docker Hub | Edge zostaje bez zmian do przełączenia; upgrade osobno. |
| System hosta (oteryn-vps) | Ubuntu 26.04 LTS | Ubuntu 26.04 wydane w kwietniu 2026 | Obecny system hosta **nie był sprawdzany**. Upgrade w **osobnym oknie serwisowym przed deployami M0**: kopia boot volume w OCI, potem `do-release-upgrade`. |

Źródła:

- [changelog GitHuba: Ubuntu 26 GA i migracja latest](https://github.blog/changelog/2026-09-17-ubuntu-26-generally-available-and-latest-migration)
- [changelog GitHuba: nowe obrazy runnerów w publicznym podglądzie](https://github.blog/changelog/2026-06-11-new-runner-images-in-public-preview/)
- [ogłoszenia wydań Rusta](https://blog.rust-lang.org/releases/)
- [strona główna PostgreSQL (18.6 aktualny, 19 RC1)](https://www.postgresql.org/)
- [ogłoszenie htmx 4.0.0](https://four.htmx.org/announcements/2026-08-28-htmx-4.0.0-is-released)
- [InfoQ: wydanie htmx 4](https://www.infoq.com/news/2026/09/htmx-4-released/)
- API crates.io (`https://crates.io/api/v1/crates/<nazwa>`)
- tagi Docker Hub: [ubuntu](https://hub.docker.com/_/ubuntu), [postgres](https://hub.docker.com/_/postgres), [rust](https://hub.docker.com/_/rust)
- [distroless](https://github.com/GoogleContainerTools/distroless) (manifest `gcr.io/distroless/cc-debian13:nonroot`)
- [Chiselled Ubuntu](https://ubuntu.com/containers/chiselled)

## Moduły produktu

Pełna strona do zarządzania serwerem, kontem i postaciami. Kolumna „Etap” pokazuje, kiedy moduł wchodzi (przed przełączeniem to M0–M6), a „Zależność w grze” gdzie potrzebna jest praca po stronie serwera gry (identyfikatory GC1–GC10 są w sekcji „Roadmapa”).

| Moduł | Etap | Zależność w grze |
| --- | --- | --- |
| Konto: bezpieczeństwo, sesje, preferencje, GDPR | M1 rdzeń, M3 komplet | Evidence (jest) |
| Panel postaci (lista/tworzenie/zmiana nazwy/usunięcie/ukrycie/ustawienia/szczegóły) | M2 tworzenie i lista, M3 reszta | GC2, GC3 |
| Publiczne strony gry (highscores, profile, zgony, online, domy, gildie) | M3 | GC1 |
| Support: zgłoszenia, raporty, sankcje, odwołania | M3 | Ban konta przez evidence (jest) |
| Treści: newsy, strony, eventy, ogłoszenia, pobieranie, launcher/updater | M4 | brak |
| Administracja, RBAC, audyt, podstawy GM (broadcast, kick, mute) | M4; akcje GM M5 | GC4 |
| **Zarządzanie serwerami gry** | **M5** | GC4, GC7 |
| **Wiki + katalog** | **M6** | GC5 |
| **Narzędzia dla gracza (MVP)** | **M6** | GC5, GC6, postęp z GC3 |
| Handel: prawdziwe płatności, zakup coinów, prezenty, faktury | M7 (po przełączeniu) | Snapshot premium (jest); inbox (później) |
| **GM observe** (podgląd sesji gracza) | M7b (web) / M9 (klient), po przełączeniu | GC4, GC10 |
| Bazar + transfer świata | M8 | GC2 Transfer/WorldTransfer |
| Narzędzia gracza, faza 2, passkeys, zarządzanie gildiami z weba | M9 | GC5 wypełnione zestawy reguł, GUILD-0 |
| Dostawca OIDC, gateway w Rust (ADR), multi-host | M10 | — |

Do usunięcia bez portowania: źródło danych Canary, `IdentityCanaryAccount`, bazar Canary, parser logów Player Companion, system szablonów strony głównej (zastąpiony stałym układem z blokami CMS).

## Struktura katalogów

Układ Platform 2. W Oteryn-Game dochodzą tylko trzy wspólne crate'y, narzędzie eksportu katalogu i kontrakty. `platform-security` jest osobno, żeby powierzchnię kryptograficzną audytować niezależnie. `platform-internal-api` zależy tylko od `platform-wire`, `-domain`, `-db`, `-security`, a `platform-web` nie może go importować (wymuszone w `[edges]`).

### `Oteryn-Platform-2` (nowe repo)

```text
Oteryn-Platform-2/
├── Cargo.toml                  # workspace; edition 2024; rust-version 1.99; lints = polityka gry
├── rust-toolchain.toml         # 1.99.1
├── deny.toml
├── .github/workflows/{ci.yml, image.yml, deploy-preprod.yml, deploy-prod.yml}
├── apps/
│   ├── platform/src/{main.rs, config.rs, roles/{public,internal,worker,migrate}.rs,
│   │                 ops/{game_auth,catalog,identity,mail,support,wiki,nodes}.rs}
│   ├── node-agent/src/{main.rs, enroll.rs, poll.rs, heartbeat.rs, logs.rs, redact.rs,
│   │                   supervisor_client.rs, metrics.rs}
│   └── node-supervisor/src/{main.rs, socket.rs, verbs.rs, verify.rs (podpisy komend i wydań),
│                            docker.rs (tylko stałe argv), nodespec.rs (typowany render node.toml),
│                            incarnation.rs (port trwałego flow ops z game-supervisor.py),
│                            policy.rs (limity, zakres portów, ścieżki)}
├── crates/
│   ├── domain/        # oteryn-p2-domain: identity, auth, characters, registry, premium,
│   │                  #   ledger, content, wiki, support, rbac, sanctions, ops, tools, account,
│   │                  #   gm/{observe.rs, recordings.rs}
│   ├── db/            # oteryn-p2-db: migrations/*.sql, repozytoria, outbox, joby, partycje logów
│   ├── security/      # argon2, totp, keyring AEAD, odczyt legacy Laravela, tryb sudo, CSRF,
│   │                  #   sesje, Ed25519 (keyring admission, podpisy komend ops),
│   │                  #   observe_grant.rs (JWS oteryn-observe-v1)
│   ├── internal-api/  # routery /internal/v1 wraz z node-agent, operator-commands,
│   │                  #   game-account/character-details; mTLS rustls; mapowanie Peer;
│   │                  #   routes/gm_observe.rs (przyjmowanie klatek, potwierdzenia)
│   ├── node-wire/     # typowane koperty i czasowniki agent<->platforma i agent<->supervisor
│   ├── catalog-import/# pobranie paczki (OCI), weryfikacja, snapshot, read modele, diff
│   ├── assets/        # magazyn obiektów (S3-compatible OCI / lokalny fs), kompozytor strojów, webp
│   ├── tools/         # narzędzia gracza: kalkulatory, trackery, parsery logów (używa oteryn-game-rules)
│   ├── web/
│   │   ├── src/{router.rs, middleware/, handlers/<moduł>.rs, oauth/, api_v1/, sse.rs}
│   │   ├── templates/{layout/, public/, account/, characters/, game/, wiki/, tools/,
│   │   │              support/, admin/ (w tym admin/observe/{request,approve,viewer,replay,recordings}.html),
│   │   │              ops/}
│   │   ├── static/{css/, js/{htmx-4.0.0.min.js, map-viewer.js, leaflet.js, observe-viewer.js}, img/}
│   │   └── locales/{en,pl}/*.ftl
│   └── worker/        # mail, outbox, retencja, terminations, deletions, eksporty, pobieranie katalogu,
│                      #   rotacja partycji logów, alarmy projekcji,
│                      #   jobs/{recording_retention.rs, observe_oversight_report.rs}
├── tools/legacy-import/src/{main.rs, laravel_crypt.rs, map/*.rs, verify.rs, reverse.rs}
├── tests/conformance/  # compose: przypięty obraz gry + game PG + platforma; zestawy czarnej skrzynki
└── deploy/
    ├── compose.base.yml, compose.v2-preprod.yml, compose.v2-testgame.yml, compose.v2-prod.yml
    ├── caddy/{v2-preprod.caddy, cutover.caddy}
    ├── systemd/{oteryn-node-agent.service, oteryn-node-supervisor.service}
    └── scripts/{deploy.sh, backup.sh, restore-drill.sh, cutover.sh, rollback.sh}
```

`observe-viewer.js` rozszerza `map-viewer.js` (SSE + znaczniki). Wspólne crate'y z Oteryn-Game, wszystkie przypięte do jednego tagu:

```toml
oteryn-platform-wire = { git = "...Oteryn-Game", tag = "contracts-v1.0.0" }
oteryn-catalog       = { git = "...Oteryn-Game", tag = "contracts-v1.0.0" }
oteryn-game-rules    = { git = "...Oteryn-Game", tag = "contracts-v1.0.0" }
```

### Dodatki w Oteryn-Game

```text
crates/platform-wire/   crates/catalog/   crates/game-rules/
tools/catalog-export/   # eksporter Rust: content/ + rulesets/ + assety -> paczka (v2);
                        # producer.py zostaje jako niezależny weryfikator w CI (dwie implementacje)
docs/contracts/{OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V2.md, OTERYN_GAME_CATALOG_BUNDLE_V1.md,
                OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V2.md, OTERYN_GAME_OPERATOR_COMMANDS_V1.md,
                OTERYN_GAME_ACCOUNT_CHARACTER_DETAILS_V1.md, OTERYN_GAME_GM_OBSERVE_V1.md}
.github/workflows/release-catalog.yml   # przy tagu wydania: eksport, podpis, push artefaktu OCI
crates/platform-wire/src/observe.rs     # claimy grantu, typy klatek i potwierdzeń
crates/protocol-oteryn/src/observe.rs   # capability OBSERVE_V1 (etap 2)
apps/game-server/src/observe/{grant.rs, feed.rs, attach.rs, pm_filter.rs}
apps/client/...                         # HUD obserwatora i tryb tylko do odczytu (etap 2)
```

## Schemat bazy platformy

Jedna baza PostgreSQL tylko dla platformy, podzielona na schematy. ID kont, klientów OAuth, światów i kanałów zachowane z Laravela. Poniżej pełna lista schematów razem z dodatkami tej rewizji.

| Schemat | Główne tabele |
| --- | --- |
| `identity` | identities, credentials, two\_factor, recovery\_codes, email\_verifications, password\_resets, web\_sessions, security\_state (generation, disabled\_at, terminated\_at), terminations; **nowe:** `email_changes`, `recovery_keys`, `security_events` (partycje miesięczne), `login_history` (partycje miesięczne), `sudo_grants`, `data_exports`, `deletion_requests`, `preferences`, `notification_prefs`, `webauthn_credentials` (M9) |
| `auth` | oauth\_clients, oauth\_codes, oauth\_tokens (hashowane), device\_sessions (legacy keyed hash + wersja), game\_tickets (sha256), admission\_keys\_meta, evidence\_floors, evidence\_witness |
| `registry` | worlds, channels, routes, route\_policies (maintenance), topology\_generations, directory\_epochs |
| `characters` | command\_intents, command\_receipts, bootstrap\_intents, account\_character\_projection, projection\_watermarks; **nowe:** `character_settings` (główna postać, komentarz, comment\_state, flagi prywatności), `lifecycle_intents` (lustro GC2: zmiana nazwy/usunięcie/widoczność/transfer), `details_projection` (GC3, epoka/rewizja), `progress_projection` (bestiary/charmy/osiągnięcia/stroje), `details_watermarks` |
| `game_public` | highscores, character\_profiles, character\_deaths, houses, guilds, world\_online, family\_watermarks |
| `premium` | premium\_ledger, premium\_snapshots |
| `commerce` | products, prices, orders, payments, provider\_events, ledger\_accounts, ledger\_transactions, ledger\_entries |
| `content` | news (+i18n), pages (+i18n), events, announcements, downloads, client\_releases, updater\_generations, media |
| `catalog` (nowy) | `bundles`, `snapshots`, `active` (jeden wiersz), `entities` (snapshot\_id, content\_key, type, data jsonb), `relations`, `xref_item_drop`, `xref_npc_offer`, `xref_spell_teacher`, `xref_area_creature`, `xref_quest_reward`, `entity_changes`, `assets` (sha256, kind, klucz obiektu), `translations`, `slugs` |
| `wiki` | `prose_sections` (content\_key, locale, section, rev, state), `prose_revisions`, `free_articles` (zmigrowane), `categories`, `entity_categories`, `review_queue` |
| `tools` (nowy) | `bestiary_manual` (konto, postać?, klucz rasy, zabójstwa), `goals`, `saved_sessions` (opt-in, domyślnie prywatne), `shared_splits` (token, expires\_at), `tool_settings` |
| `support` | tickets, messages, reports, enforcement\_records, appeals |
| `staff` | roles, permissions, role\_permissions, staff\_assignments |
| `ops` | audit\_events, outbox, jobs, operator\_commands / operator\_receipts (GC4); **zarządzanie serwerami:** `hosts`, `agents` (pin SPKI, enrollment, revoked\_at), `nodes` (host, kanał(y), desired\_state, release\_id, nodespec\_rev), `node_status` (ostatni), `node_status_history` (próbkowana), `agent_commands` (podpisana koperta, maszyna stanów requested → approved → dispatched → acked → succeeded/failed/expired), `command_approvals` (cztery oczy), `command_seq` (per host), `releases` (digest obrazu, git sha, digest paczki, rules\_abi, migracje, rollback\_safe, podpis), `deployments`, `maintenance_windows`, `scheduled_shutdowns`, `node_log_lines` (partycje dzienne) |
| `gm` (nowy) | `observe_requests`, `observe_approvals`, `observe_sessions`, `observe_frames`, `recordings`, `recording_access_log` (szczegóły w tabeli poniżej) |
| `bazaar` (M8) | auctions, bids, escrow\_holds |

Schemat `gm` (GM observe):

| Tabela | Zawartość |
| --- | --- |
| `observe_requests` | id, requester, konto/postać celu, świat, tryb, powód, linked\_ref, pm\_escalation, stan, utworzenie/wygaśnięcie |
| `observe_approvals` | request\_id, approver, decyzja, kiedy, komentarz |
| `observe_sessions` | observe\_id, request\_id, kid grantu, target\_session\_generation, started\_at, ended\_at, end\_reason, recorded |
| `observe_frames` | Partycje dzienne: observe\_id, seq, generation, ts, klatka (zstd jsonb) |
| `recordings` | observe\_id, retention\_until, legal\_hold, linked\_case |
| `recording_access_log` | recording\_id, aktor, akcja (view/export), approval\_id, kiedy |

## Deploy na oteryn-vps

### CI/CD (Platform 2)

| Workflow | Runner | Kroki |
| --- | --- | --- |
| `ci.yml` | `ubuntu-26.04` | fmt, clippy (deny), testy z kontenerem usługi PG (18.6, a po GA także 19), kontrola offline sqlx (`sqlx prepare --check`), cargo-deny, zgodność kluczy i18n, zgodność (obraz gry przypięty po digeście) |
| `image.yml` | `ubuntu-26.04-arm` (natywne arm64, bez QEMU) | Build `oteryn-platform`, `oteryn-node-agent` i `oteryn-node-supervisor`; push do GHCR **po digeście**; SBOM; podpis sigstore keyless przez GitHub OIDC |
| `deploy-preprod.yml` | self-hosted `[self-hosted, linux, ARM64, oteryn-vps]` | Automatycznie z `main`: pull po digeście, migracje, `compose up`, smoke |
| `deploy-prod.yml` | ten sam runner, środowisko `v2-prod` (ręczna akceptacja) | Wyłączony do bramki przełączenia. Potem: zwykłe wydania. |

Zasady runnera:

- Repo Platform 2 musi być **prywatne**. Nigdy nie podpinać self-hosted runnera do repo publicznego.
- Runner jest efemeryczny, działa pod własnym użytkownikiem i uruchamia wyłącznie `deploy/scripts/deploy.sh` z przypiętymi digestami (osobny użytkownik, kontener `ubuntu:26.04`, dostęp tylko do `/opt/oteryn-v2/**` przez ograniczony skrypt sudoers).
- Obrazy są czyszczone do 3 ostatnich tagów, bo dysk jest ciasny; logi `max-size=10m`.
- Runner na Synology jest wycofany dla Platform 2.
- Repo gry przechodzi z CI na `ubuntu-26.04` i dostaje `release-catalog.yml`.

### Projekty compose na oteryn-vps

| Projekt | Zawartość | Sieć | Ekspozycja |
| --- | --- | --- | --- |
| `oteryn-oracle` (istniejący) | Laravel, edge Caddy, nginx internal-mtls, gateway Go, produkcyjna gra | 172.18.0.0/24, 172.30.84.0/24 | Bez zmian, poza **jednym dodanym plikiem site Caddy** dla hostów v2 |
| `oteryn-v2-preprod` | p2-public, p2-internal (:8543), p2-worker, p2-postgres (własny wolumen, archiwum WAL), instancja gatewaya Go (ten sam obraz, własny bearer i zaufanie admission) | 172.19.0.0/24 | `v2.oteryn.com` (Cloudflare Access), `login-v2.oteryn.com` |
| `oteryn-v2-testgame` | Węzły gry z przypiętego obrazu gry, **własna instancja PG gry**, node-agent + node-supervisor (jednostki systemd, zapisane do preprodu v2) | 172.19.1.0/24 | `game-v2.oteryn.com` TCP 7272–7279, tylko DNS |
| `oteryn-v2-prod` | To samo co preprod, na sekretach produkcyjnych | Dołącza do `oteryn-oracle_private` przy przełączeniu | Tworzony przy przełączeniu (obrazy wcześniej pobrane, baza wcześniej zmigrowana) |

Bazowe ustawienia każdego kontenera Platform 2:

- użytkownik 65532, `read_only: true`, `cap_drop: [ALL]`, `no-new-privileges`;
- sekrety w `/run/secrets` ze sprawdzaniem uprawnień;
- healthcheck przez `oteryn-platform healthcheck <rola>`;
- sterownik logowania journald.

Pozostałe elementy:

- **Obraz:** builder `ubuntu:26.04` + Rust 1.99.1 (`cargo build --release --locked`, `SQLX_OFFLINE=true`); rootfs z chisel (patrz „Stack technologiczny”); finalnie `FROM scratch`, `USER 65532`. **Niezweryfikowane:** dostępność slice'ów chisel dla 26.04 sprawdzić w M0; wariant awaryjny to distroless debian13 z builderem trixie.
- **Certyfikaty:**
  - Preprod: własny offline root (zaszyfrowany, w rękach właściciela) plus online pośrednie CA z ograniczeniem nazw w p2-internal dla CSR agentów i węzłów.
  - Prod: przy przełączeniu istniejące produkcyjne CA i certyfikat serwera dla ścieżek gry i gatewaya, plus nowe produkcyjne pośrednie CA dla agentów.
- **Magazyn obiektów:** bucket OCI Object Storage (zgodny z S3) `oteryn-assets`, z prefiksami `preprod/` i `prod/`, za Cloudflare pod `cdn.oteryn.com`. W dev lokalny backend plikowy.
- **Sekrety:** w `/opt/oteryn-v2/secrets/<env>/`, montowane read-only jako Docker secrets, 0400, uid 65532. Sekrety produkcyjne (klucze Ed25519 admission, certyfikat i klucz serwera, CA, hashe bearerów gatewaya, legacy APP\_KEY) trafiają do `/opt/oteryn-v2/secrets/production/` dopiero w oknie przełączenia.
- **Kopie zapasowe:** archiwizacja WAL + base backup jak w obecnym PG platformy, wysyłane poza maszynę do osobnego bucketa OCI z regułami cyklu życia. **Cotygodniowe ćwiczenie odtworzenia** do preprodu (od M1).
- **Pojemność:**
  - W M0 zmierzyć RAM, CPU i dysk hosta, bo dochodzi drugi stos i testowy PG gry. Jeśli RAM < ok. 12 GB, preprod i testgame działają tylko w oknach testów (albo drugi VPS A1 na preprod), a usługi mają twarde `mem_limit`.
  - Preprod i testgame można zatrzymać, gdy są bezczynne; job deployu uruchamia je na żądanie.
- **Host:** upgrade do Ubuntu 26.04 LTS w osobnym oknie przed deployami M0 (najpierw kopia boot volume).
- Caddy dostaje tylko nowe bloki; istniejące bloki się nie zmieniają. `v2.` za Cloudflare Access, dopóki właściciel go nie otworzy.
- v2-preprod nigdy nie czyta danych produkcyjnych, poza próbami importu w zamkniętym środowisku prób.

## Zarządzanie serwerami gry

Z panelu admina: lista hostów, światów, kanałów i węzłów, status i zdrowie na żywo, logi (podgląd na żywo i wyszukiwanie), start, stop i restart, maintenance, zapowiadany restart lub wyłączenie z odliczaniem, wdrożenie nowej wersji gry, dodanie kanału lub świata.

### Model

- **Platforma nigdy nie łączy się z hostami.** Na każdym hoście:
  - `oteryn-node-agent` działa jako użytkownik `oteryn-agent`: bez sudo, **poza** grupą docker, tylko ruch wychodzący (mTLS do `platform-internal`);
  - `oteryn-node-supervisor` działa jako root (musi sterować Dockerem) i **nie ma sieci** (`IPAddressDeny=any`). Jedyny interfejs to `/run/oteryn/supervisor.sock`: tryb 0660, `root:oteryn-agent`, sprawdzany `SO_PEERCRED`.
- **Zastępuje `game-supervisor.py`** i zachowuje jego niezmienniki:
  - trwała autoryzacja ops na inkarnację;
  - nigdy nie używać ponownie zużytego pliku startowego;
  - `reconciliation-required` zatrzymuje automatyzację;
  - kontener zachowuje `restart: 'no'`.
- **Koperta komendy.** Każda komenda to `{command_id (UUIDv7), host_id, node_id, verb, typowane argumenty, seq, not_before, expires_at (≤120 s)}`, podpisana **kluczem komend ops** platformy (Ed25519, plik zamontowany w p2-internal, osobny od klucza admission). Supervisor sprawdza:
  - podpis względem przypiętego klucza publicznego;
  - że `seq` rośnie monotonicznie per host;
  - TTL i idempotencję.

  Skompromitowany agent nie może więc wymyślać komend.
- **Podpis wydania.** Wdrożenia obrazów muszą nieść także **podpis wydania**: klucz Ed25519 w CI Oteryn-Game nad `{digest obrazu, git sha, digest paczki katalogu, rules_abi, zakres migracji, rollback_safe}`. Nawet w pełni skompromitowana platforma może uruchamiać tylko **podpisane wydania gry** dozwolonymi czasownikami.

Nowe endpointy (agent → platforma) są wymienione w tabeli przepływów w sekcji „Architektura” (komendy, heartbeat co 5 s, potwierdzenia, logi, CSR). Jednorazowa rejestracja agenta: token jednorazowy o TTL 15 minut utworzony przez `ops.agents.enroll`, certyfikat przypięty po SPKI, lista unieważnień w platformie.

### Dozwolone akcje (zamknięta lista; wszystko inne jest niemożliwe z konstrukcji)

| Czasownik | Skutek | Zabezpieczenie |
| --- | --- | --- |
| `status` | Stan jednostek, kontenerów i obrazów; CPU/RAM/dysk hosta | odczyt |
| `start` | Nowa inkarnacja przez dostarczony trwały flow `oteryn-game-ops`, jak dziś | typowany node\_id |
| `stop` | Łagodny stop (SIGTERM, 60 s karencji = `stop_grace_period`) | — |
| `restart` | stop + start | — |
| `deploy` | Pobranie `ghcr.io/<org>/oteryn-game@sha256:<digest>`, uruchomienie `oteryn-game-migrate` z **tego** obrazu, przełączenie, start | Digest musi być w manifeście podpisanym przez wydanie |
| `apply-nodespec` | Render `node.toml` z szablonu należącego do supervisora z typowanymi polami | UUID-y; port w 7172–7199; ID kanałów obecne w podpisanej generacji topologii; ścieżki stałe w `/opt/oteryn-game/nodes/<uuid>` |
| `provision-node` | Utworzenie katalogu węzła (0700, uid 1001) i usługi compose ze stałego szablonu | Maks. liczba węzłów na host (konfig) |
| `install-cert` | Instalacja certyfikatu podpisanego przez platformę dla klucza wygenerowanego lokalnie | Wzorzec CN `oteryn-<env>-<cel>-<węzeł>` |
| `log-cursor` | Odczyt journald po stronie agenta (supervisor niepotrzebny) | Dopasowanie tylko `CONTAINER_NAME=oteryn-*` / `_SYSTEMD_UNIT=oteryn-*` |
| `clear-reconciliation` | Zdjęcie flagi `reconciliation-required` po przeglądzie człowieka | Cztery oczy na produkcji |

**Nigdy niedozwolone:**

- shell ani dowolne argv;
- opcje `docker run`, mounty, `--privileged`, sieć hosta;
- pliki poza katalogiem węzła;
- inne jednostki systemd, pakiety, firewall, reboot;
- odczyt sekretów;
- łączenie z PG gry (migracje tylko przez własną binarkę migrate podpisanego obrazu);
- exec w kontenerach, dowolne obrazy, zmienne środowiska, porty spoza zakresu, klucze SSH, kasowanie wolumenów i kopii, aktualizacja samego agenta/supervisora.

Otwarcie portów w security list Oracle dla nowych kanałów zostaje krokiem ręcznym albo Terraform; UI pokazuje to jako punkt checklisty (platforma nie ma poświadczeń chmury).

### Funkcje i przepływy

- **Widok serwerów:**
  - światy → kanały → węzły → host;
  - stan oczekiwany vs rzeczywisty;
  - obraz/wydanie;
  - własne raporty runtime gry i przypisania scope (istniejące `native-runtime-status` i `native-scope-assignments`);
  - heartbeat agenta, liczba online (`world_online`), ostatnia inkarnacja, flaga reconciliation.
  - Stany: `healthy | degraded | down | maintenance | reconciling`; brak danych > 15 s = nieaktualne. Metryki Prometheus.
- **Logi:**
  - Kontener gry przechodzi z json-file na sterownik logowania **journald** z tagiem.
  - Agent czyta `journalctl -o json -f --after-cursor` jako podproces (bez FFI, więc `unsafe` zostaje zabronione). Maskuje tokeny bearer, 43-znakowe seedy base64url i linie `Authorization`, po czym wysyła paczki.
  - Agent trzyma lokalny bufor na dysku do 200 MB.
  - Przechowywanie: tabela PG `ops.node_log_lines`, partycje dzienne, indeks BRIN na `ts` i GIN `pg_trgm` na `message`. Retencja przez usuwanie partycji (14 dni prod, 7 dni preprod).
  - **Podgląd na żywo** to SSE zasilane `LISTEN/NOTIFY`. Wyszukiwanie po czasie, poziomie, węźle i tekście. Eksport wymaga `ops.logs.export` i jest audytowany.
  - Opcja skalowania, za tym samym traitem `LogStore`: VictoriaLogs.
- **Tryb maintenance:** przestawienie polityki trasy w rejestrze; gateway odpowiada `ROUTE_UNAVAILABLE` dla nowych logowań.
- **Zapis i wyłączenie z odliczaniem (GC4):**
  1. Zaplanowanie.
  2. `Broadcast` operatora przy T-15, T-5 i T-1 min.
  3. Przy T-0 maintenance trasy.
  4. `SaveAndShutdown` operatora; gra zapisuje i kończy z kodem 0.
  5. Agent zgłasza planowany stop; supervisor **nie** restartuje.
  6. Przy wdrożeniu lub restarcie wykonanie następnego czasownika.
  - Rozwiązanie awaryjne przed GC4: `stop`. **Zespół gry musi potwierdzić, że SIGTERM wywołuje pełny zapis.**
- **Wdrożenie:**
  1. Wybór wydania z `ops.releases`. Platforma sprawdza `rollback_safe` i zakres migracji.
  2. Przepływ z odliczaniem.
  3. Czasownik `deploy`.
  4. Bramka zdrowia: runtime status + watermark LCFA w ciągu 60 s.
  5. Opcjonalnie aktywacja odpowiadającego snapshotu katalogu.
  - Rollback wdraża poprzedni digest i tylko wtedy, gdy nowsze wydanie zadeklarowało `rollback_safe`.
- **Dodanie kanału lub świata:**
  1. Admin tworzy kanał w rejestrze (świat, nazwa, region, pojemność, host, port). Dla nowego świata `world:ensure` i sprawdzenie, że świat jest w manifeście paczki światów wydania.
  2. Wydanie nowej generacji topologii (semantyka `native-topology:issue`).
  3. Kolejkowanie `provision-node`, `install-cert` (agent generuje klucz → CSR → platforma podpisuje), `apply-nodespec`, `start`.
  4. Gra rejestruje się (`game_node_registrations`) i zgłasza przypisanie scope.
  5. Publikacja trasy (`native-route:publish`) i podbicie epoki directory.
  6. Klient widzi kanał w `GET /v1/client/directory`.
  - **GC7: zespół gry musi potwierdzić obsługę wielu węzłów na jednym hoście.** Testowy węzeł gry v2 udowodni to w M5.

### RBAC, audyt, potwierdzanie

- **Uprawnienia:**
  - `ops.nodes.view`, `ops.logs.view`, `ops.logs.export`
  - `ops.nodes.start`, `ops.nodes.stop`, `ops.nodes.restart`
  - `ops.deploy`, `ops.channels.manage`, `ops.worlds.manage`
  - `ops.maintenance.manage`, `ops.broadcast`, `ops.shutdown.schedule`
  - `ops.agents.enroll`, `ops.reconcile`
- **Każde** żądanie, zatwierdzenie, wysłanie, potwierdzenie i wynik trafia do `audit_events` (łańcuch hashy) i do `ops.agent_commands`.
- **Akcje destrukcyjne** (stop, restart, deploy, wyłączenie, zmiana kanału lub świata, reconcile) wymagają:
  - **trybu sudo**: hasło albo TOTP w ciągu ostatnich 5 minut;
  - podglądu skutków (gracze online, dotknięte kanały);
  - wpisania nazwy celu.
- **Na produkcji także cztery oczy** dla deployu, stopu świata, usunięcia kanału i reconcile: drugi członek staffu z tym samym uprawnieniem zatwierdza w ciągu 30 minut. Na preprodzie wystarcza jeden zatwierdzający.
- **Limity supervisora:** najwyżej 6 restartów węzła na godzinę i jeden deploy w toku na host.
- Agent sprawdza podpis, host, ważność, numer i lokalną politykę (okno zamrożenia, lista akcji dla hosta); supervisor ponownie waliduje argumenty.
- **Role:** GM (podgląd, logi, broadcast), Operator (+ restart, maintenance, zaplanowane wyłączenia), Owner (wszystko).

### Utwardzenie agenta

- Jednostka systemd: `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, pusty `CapabilityBoundingSet`, `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`, `SystemCallFilter=@system-service`.
- Dodatkowa grupa `systemd-journal`, tylko do czytania logów.

## Wiki z katalogu gry

Wiki w stylu TibiaWiki BR / Fandom: każdy potwór, przedmiot, NPC, czar, quest, obszar, profesja, strój, wierzchowiec, dom i osiągnięcie ma stronę generowaną z danych gry. Dane są zablokowane (pochodzą z katalogu), opisy piszą redaktorzy z recenzją.

### Co istnieje w repo gry (origin/main)

- **Rodziny treści:** `content/{creatures (rasy, indeksy bestiary/bosstiary), items (77 plików), npcs (1282), loot (1290 tabel, prawdopodobieństwa w ppm), quests, world/areas (miasta, miejsca polowań, regiony, wyspy, ulice), world/placements (1208 chunków regionów `.b3`), world/pins, houses (995), cosmetics/{outfits, mounts (252)}, abilities (czary), charms (25), services/{trade (324), travel (56)}, proficiencies (443), presentations/bindings (client-ids)}`.
- **Assety:** `content/assets/files/` zawiera `appearances-*.dat` (protobuf 15.30), 6241 arkuszy sprite'ów lzma, `map-*.dat`, `staticmapdata`, `staticdata` i `backdrop_map.jpg`.
- **Zestawy reguł (rulesets):**
  - Wypełnione: tabela doświadczenia do poziomu 2000, progresja charmów, postęp zabójstw bestiary.
  - Szkic: imbuements (`DRAFT_NOT_RUNTIME_READY`).
  - `READY_UNPOPULATED`: exaltation-forge, stamina, skills, doomforging, polityka handlu NPC, statyczne fakty bestiary, katalog licencji assetów.
- **Dzisiejszy eksport:**
  - `tools/game-platform-catalog/producer.py` implementuje **Catalog Export v1** (PROPOSED). Ma zamknięty zestaw 8 capabilities (item, creature, creature\_loot, npc, npc\_shop, spell, quest, achievement), `snapshot_id` adresowany treścią, brak floatów i limit 256 MB.
  - Istnieje tylko **nieobsługiwany, nieaktywowalny fixture**. Prawdziwego adaptera jeszcze nie ma.
  - Laravel ma już pasujące komendy import/validate/diff/activate/rollback/verify (`app/GameCatalog/*`).
- **Dekodowanie sprite'ów:** `crates/client-assets` już dekoduje appearances i arkusze (lzma-rs + prost, przypięte hashem).

### Kontrakt i paczka (GC5)

- **Catalog Export v2** rozszerza v1 bez łamania. Crate `oteryn-catalog` czyta oba.
- **Nowe capabilities:**
  - `creature_spawn` (podsumowania per obszar i reprezentatywne współrzędne, nigdy surowe rozmieszczenia);
  - `npc_trade_offer` (cena kupna/sprzedaży, waluta);
  - `npc_travel`, `house`, `outfit`, `mount`, `vocation`;
  - `area` (granice i wielokąty), `map_marker`, `charm`, `bestiary_entry`, `boss`;
  - `imbuement`, `forge_rule`, `experience_table`, `stamina_rule`, `skill_rule`;
  - `appearance_ref` (content\_key → ID assetów sprite'ów).
- Zostaje semantyka `support` + `completeness`. **Strony i narzędzia renderują tylko capabilities o statusie `supported`.** Brak danych nigdy nie jest pokazywany jako „nie istnieje”, chyba że jest `complete`. Każda kategoria ma poziom kompletności; wiki pokazuje baner „dane niepełne”.
- **Paczka v1** to artefakt OCI `ghcr.io/<org>/oteryn-catalog:<release>`:

  ```text
  manifest.json      # tag wydania, source_revision, digest obrazu, rules_abi, lista sha256 plików
  manifest.sig       # klucz wydania Ed25519
  catalog.json       # koperta v2, kanoniczna
  rules-vectors.json # złote wejścia/wyjścia dla oteryn-game-rules
  assets/<sha256>.webp       # tylko appearances użyte przez encje katalogu;
                             # stwory: animowany idle na południe; stroje: baza + warstwy masek
  tiles/<floor>/<zoom>/<x>/<y>.webp   # minimapa, kafelki XYZ 256 px (piętra 0–15)
  licensing.json     # zgoda na publikację per asset; eksporter przy braku zgody zawodzi
  ```

- **Eksporter:**
  - Rust `tools/catalog-export` w repo gry. Używa ponownie `client-assets` do dekodowania i dodaje zależność enkodera webp (tylko narzędzie).
  - Renderowanie minimapy używa dekodera map stojącego za rendererem klienta. **Zespół gry powinien potwierdzić, że dekoder map.dat da się użyć ponownie** (niezweryfikowane).
  - CI uruchamia producera Python jako drugi, niezależny test digestu.
- Przy każdym tagu gry CI publikuje obraz, paczkę katalogu i assety jako artefakt OCI plus manifest wydania podpisany kluczem wydania.
- **Wspólny crate, a nie `platform-wire`:** `oteryn-catalog` jest osobny, ponieważ katalog ma cykl życia per wydanie i jest duży, ma innych konsumentów (eksporter, platforma, ewentualnie klient), a `platform-wire` musi zostać małym kontraktem gorącej ścieżki. Oba żyją w Oteryn-Game i wychodzą pod tym samym tagiem kontraktów.

### Import, aktywacja, rollback (`catalog-import`)

1. Worker pobiera paczkę albo staff ją wgrywa.
2. Weryfikacja: podpis, hashe plików, digest koperty, spójność referencji, licencje, `rules_abi == ABI zlinkowanego oteryn-game-rules` i **ponowne uruchomienie wektorów reguł**. Każda porażka odrzuca paczkę.
3. Załadowanie niezmiennego snapshotu (`catalog.snapshots`, wiersze kluczowane `snapshot_id`) i wgranie assetów do magazynu obiektów (cdn.oteryn.com za Cloudflare, `immutable`). Assety są adresowane treścią, więc magazyn deduplikuje.
4. Zbudowanie read modeli dla snapshotu: widoki encji i indeksy odsyłaczy.
5. Diff względem aktywnego snapshotu do `catalog.entity_changes`.
6. Podgląd staffu z `?snapshot=<id>`, uprawnienie `wiki.publish`.
7. **Aktywacja = przestawienie wskaźnika w jednym wierszu** (`catalog.active`). Rollback przestawia go z powrotem. Trzymamy 5 ostatnich snapshotów i wszystkie diffy.
8. Rejestr `ops.releases` łączy każdy digest obrazu z jego paczką, więc UI oferuje „aktywuj razem z wdrożeniem”.

### Strony

- **Adresy:** `/{locale}/wiki/{typ}/{slug}`, gdzie `typ` to `creature`, `item`, `npc`, `spell`, `quest`, `area`, `house`, `outfit`, `mount`, `vocation`, `charm` lub `boss`.
  - Slug pochodzi z wyświetlanej nazwy i jest unikalny per typ, z sufiksem klucza przy kolizji nazw.
  - `/{locale}/wiki/k/{content_key}` przekierowuje na adres kanoniczny; przekierowania po zmianie nazwy.
- **Infobox zablokowany na danych**, renderowany tylko z katalogu:
  - statystyki, tabela loota z szansą (ppm → %) i ilościami, „wypada z” z szansą;
  - „sprzedaje / kupuje” NPC z ceną i walutą, „uczy czaru” z ceną;
  - statystyki czaru/runy, nagrody za questy;
  - „występuje w” obszarach z pinezką na mapie;
  - rozmiar domu, czynsz i lokalizacja.
- Powiązania wg encji:

  | Encja | Powiązania |
  | --- | --- |
  | Przedmiot | wypada z (potwór, szansa, ilość), sprzedaje / kupuje (NPC, cena, miasto), nagroda za (quest) |
  | Potwór | tabela loota, występuje w (obszary + pinezka na mapie) |
  | NPC | oferty handlowe, podróże, lokalizacja |
  | Czar / runa | statystyki, profesje |
  | Obszar | mapa (Leaflet + kafelki) i potwory |

- **Interaktywna minimapa:** wendorowana przeglądarka klasy Leaflet na `cdn.oteryn.com/tiles/...`, ze znacznikami z endpointu JSON per encja. To jedyny JS poza htmx.
- **Historia wersji:** listy „Zmienione w wydaniu X” z `entity_changes` pokazują pole, starą i nową wartość. Encja z nagrobkiem pokazuje „Usunięte w wydaniu X” i zachowuje opis.
- **Wyszukiwanie:**
  - PG FTS z konfiguracją `simple` + `unaccent` + `pg_trgm` na literówki. PG nie ma polskiego stemmera, więc polską nieostrą pracę robi dopasowanie trigramowe.
  - Autouzupełnianie w htmx 4.
- **Kategorie:** automatyczne z danych (rasa, klasa przedmiotu, profesja, miasto, obszar) oraz redakcyjne.
- **Opisy redakcyjne:**
  - Sekcje per encja (opis, strategia, uwagi, ciekawostki) w `wiki.prose_sections`, w istniejącym workflow szkic → recenzja → publikacja.
  - Odwołania do danych w treści, np. `{{item:<klucz>|price}}` (także `{{item:oteryn:item.x|price@npc:oteryn:npc.y}}`), rozwiązywane przy renderowaniu. Wiszące referencje są raportowane przy aktywacji.
  - Artykuły wiki z Laravela są migrowane. Te, które mapują się na klucz treści, stają się opisem; reszta zostaje wolnymi artykułami; stare adresy dostają 301.
- **i18n:** nazwy z katalogu są po angielsku. Polskie nazwy i opisy żyją w redakcyjnej nakładce `catalog.translations` (`content_key`, `locale`, `field`, reviewed). Nieprzetłumaczone pozycje wracają do angielskiego ze znacznikiem; jeśli gra kiedyś wyeksportuje nazwy po polsku, mają pierwszeństwo. Teksty UI w Fluent.
- **Cache:** ETag na `(snapshot_id, prose_rev)`; Cloudflare cache'uje publiczne strony.

## Narzędzia dla gracza

### Zestaw narzędzi

| Narzędzie | MVP (M6, przed przełączeniem) | Później (M9+) | Wymagane dane (capability katalogu) |
| --- | --- | --- | --- |
| Tracker bestiary + charmów | Tryb ręczny (wybór stworów, wpisywanie zabójstw; per konto i opcjonalnie postać). **Tryb automatyczny** z postępu GC3, jeśli zostanie dostarczony. | Cele, ETA z sesji polowań | `bestiary_entry`, `charm`, `creature_spawn` |
| Kalkulator doświadczenia/poziomu | tak | — | `experience_table` (wypełniona) |
| Kalkulator stamina | tylko jeśli zestaw reguł jest wypełniony, inaczej ukryty | — | `stamina_rule` (dziś niewypełniona) |
| Podział łupu w party | tak: N członków, zapasy i łup → zbilansowane przelewy, link do udostępnienia (7 dni) | — | brak (czysta matematyka) |
| Analizator sesji polowania / party (wklejka) | tak, **jeśli klient Oteryn eksportuje udokumentowane podsumowanie sesji (GC6/GC9)**; w przeciwnym razie formularz ręczny | Zapisane sesje, wykresy, exp/h per obszar | przedmioty (ceny), stwory |
| Mapa tasków / dostaw | Warstwy mapy: questy, NPC z ofertami handlowymi, miejsca polowań, spawny, domy | Warstwy tasków/dostaw, gdy `content/services/tasks` zostanie wypełnione | `area`, `map_marker`, `creature_spawn`, `npc_trade_offer`, `quest` |
| Kalkulator imbuementów | ukryty (zestaw reguł to SZKIC) | tak, gdy `imbuement` jest `supported` | `imbuement` + ceny rynkowe (później) |
| Symulator forge / exaltation | ukryty (niewypełniony) | Monte Carlo, po stronie serwera | `forge_rule` |
| Kalkulator umiejętności / treningu | ukryty (niewypełniony) | tak | `skill_rule` |
| Kalkulator obrażeń / pancerza | ukryty | tak | zestaw reguł walki (niewypełniony) |

Każde narzędzie deklaruje potrzebne capabilities w rejestrze narzędzi. **Pojawia się tylko wtedy, gdy aktywny snapshot ma je jako `supported`.** Żadna wartość z Tibii nie jest zahardkodowana.

### Zgodność formuł z grą

- **Dane są eksportowane jako dane katalogu:** tabele, stałe i prawdopodobieństwa, np. tabela doświadczenia, koszty charmów i progi bestiary.
- **Logika** (np. krzywa regeneracji stamina, próby umiejętności, sukces forge i przejścia tierów, matematyka imbuementów, redukcja obrażeń) jest wyciągana z `apps/game-server` do **`oteryn-game-rules`**. Istniejący kod do wyciągnięcia to m.in. `durability/character_build.rs`, `spell/mana_training.rs` i `content/item_stats_promotion.rs`.
  - Czysta, na liczbach całkowitych, deterministyczna.
  - Zależności ograniczone do `foundation` / `error-codes`.
  - `game-server` dostaje nową krawędź do niej, więc serwer i strona **uruchamiają ten sam kod**.
- **Dopasowanie do granic crate'ów:**
  - Kategoria `shared-domain`, `release-role = "game-and-platform"`.
  - Brak zakazanych fragmentów nazw. Brak I/O.
  - Kontrola architektury wymusza krawędź.
  - ADR dla wyciągnięcia to deliverable zespołu gry (GC8).
- **Zabezpieczenie przed dryfem:**
  - Crate eksportuje `RULES_ABI: u32`; paczka niesie `rules_abi` i `rules-vectors.json`, generowane przez CI gry z tego samego crate'a.
  - Aktywacja w platformie ponownie uruchamia wektory na zlinkowanym crate'ie i **odmawia** przy każdej niezgodności.
  - Naprawa to podbicie tagu kontraktów Platform 2, redeploy, potem aktywacja.

### Postęp osobisty i prywatność

- **Pchane przez grę (GC3, tylko dla właściciela), per postać:** zabójstwa bestiary per rasa i etap, punkty charmów i odblokowane charmy, bosstiary, osiągnięcia, posiadane stroje/wierzchowce, stany questów (później).
  - Tabele w grze już istnieją: migracje 0019 (postęp bestiary), 0020 (stan charmów), 0021 (osiągnięcia konta), 0056 (stan questów).
- **Wpisywane przez użytkownika:** ręczny tracker (także dla postaci bez GC3), wklejone sesje, cele.
- **Prywatność:**
  - Projekcje są domyślnie **widoczne tylko dla właściciela konta**.
  - Przełączniki opt-in per postać: „pokaż postęp bestiary publicznie”, „pokaż osiągnięcia publicznie”.
  - Wklejone logi **nie są zapisywane**, dopóki użytkownik nie kliknie Zapisz. Zapisane sesje są prywatne, chyba że udostępnione linkiem (z wygaśnięciem). Użytkownik może usunąć wszystko; eksport GDPR to obejmuje.

### Decyzja o renderowaniu

- **Natywny Rust po stronie serwera + częściowe podmiany htmx 4, dla każdego kalkulatora i symulatora.** Formularze używają `hx-post` z wyzwalaczem po debounce'owanym wpisie (składnię sprawdzić względem htmx 4; niezweryfikowane). Fragment wyniku renderuje askama. Symulator robi ograniczone Monte Carlo: najwyżej 100 tys. prób, deterministyczny seed, wynik cache'owany po hashu wejścia, z limitem żądań.
- **Mapa:** jeden wendorowany plik JS przeglądarki (przypięty, hash SRI, bez builda npm), bo kafelki i znaczniki są tylko do wyświetlania.
- **Dlaczego nie WASM:**
  - jedna ścieżka kodu;
  - działa bez JS;
  - brak toolchaina wasm i rozmiaru paczki;
  - koszt CPU jest znikomy w skali Oteryn.
- **WASM zostaje tanią opcją na później.** `oteryn-game-rules` jest sprawdzany w CI pod `wasm32-unknown-unknown`, więc mógłby napędzać symulator offline, gdyby opóźnienie serwera kiedyś miało znaczenie.

## Panel konta i postaci

Panel „Moje konto”. Klucz kontraktów:

- **P** = tylko platforma
- **GC2** = komendy postaci v2 (nowe pary na istniejących `/internal/v1/game-auth/character-commands/*`)
- **GC3** = `/internal/v1/game-account/character-details` (+ `/watermark`), projekcja tylko dla właściciela
- **LCFA** = istniejące `/native-account-characters`
- **PUB** = projekcje publiczne (GC1)
- **EV** = podbicie generacji evidence (istniejące `native-evidence`)

### Funkcje

| Funkcja | Kontrakt | Etap | Uwagi |
| --- | --- | --- | --- |
| Przegląd: status, czas premium, saldo coinów, wynik bezpieczeństwa | P (ledger premium, ledger portfela) | M1 | Wynik liczy 2FA, klucz odzyskiwania, zweryfikowany e-mail, wiek hasła i liczbę sesji. |
| Zmiana e-maila, oba adresy potwierdzone | P + EV | M1 | Token na stary adres (potwierdzenie plus „to nie ja”, co blokuje zmianę); token na nowy adres. Oba w ciągu 24 h. Stary adres dostaje 7-dniowy link cofnięcia. |
| Zmiana hasła | P + EV | M1 | Podbicie generacji unieważnia sesje urządzeń i tickety. Opcjonalnie sprawdzenie HIBP (k-anonymity), decyduje właściciel. |
| 2FA TOTP + kody odzyskiwania | P + EV | M1 | Zmigrowane sekrety (zaszyfrowane na nowo). Wyłączenie wymaga trybu sudo plus TOTP. |
| Passkeys / WebAuthn | P | M9 | Później, zgodnie z życzeniem. |
| Aktywne sesje i urządzenia, zdalne wylogowanie | P + EV | M1 | Sesje webowe, tokeny odświeżania OAuth, sesje urządzeń launchera. „Wyloguj wszędzie” podbija generację. |
| Historia logowań i zdarzenia bezpieczeństwa | P | M1 | Logowania webowe, granty OAuth, wydawanie ticketów, admission allow/deny (platforma decyduje o admission; IP zapisywane tylko jeśli gateway je przekaże). Retencja 180 dni. |
| Klucz odzyskiwania | P | M1 | 25 znaków, pokazywany raz, hash argon2id. Pozwala zresetować hasło i 2FA po utracie e-maila. Rotacja wymaga trybu sudo. |
| Eksport danych GDPR | P | M3 | Worker buduje ZIP (JSON). Link e-mailem ważny 24 h. Limit 1 na 7 dni. |
| Usunięcie konta z karencją | P + EV + GC2 | M3 | 30 dni. Logowanie do gry zablokowane w karencji (evidence odmawia). Anulowanie w dowolnym momencie. Przy finalizacji `RequestCharacterDeletion` dla wszystkich postaci, potem anonimizacja tożsamości; ledgery są pseudonimizowane i zachowane. |
| Powiadomienia, język, strefa czasowa | P | M1 | Maili bezpieczeństwa nie można wyłączyć. |
| Lista postaci: online, świat, poziom, profesja, ostatnie logowanie | LCFA + PUB `world_online` + GC3 | M2 lista, M3 pola na żywo | Profil publiczny celowo nie ma ostatniego logowania ani ukrytych postaci, stąd GC3. |
| Tworzenie postaci | istniejące `CreateCharacter` | M2 | **v1 ignoruje płeć, profesję i strój.** GC2 dodaje `CreateCharacter`/`PLATFORM_USER_CREATE_V2` z nimi. Do tego czasu UI pokazuje tylko nazwę + świat. |
| Zmiana nazwy | GC2 `RenameCharacter`/`PLATFORM_USER_RENAME` | M3 | Platforma wstępnie sprawdza nazwę; decyduje gra przez rezerwacje nazw. Cooldown 30 dni. Wariant płatny w M7. |
| Usunięcie z karencją i anulowanie | GC2 `RequestCharacterDeletion` / `CancelCharacterDeletion` z `effective_at` | M3 | Cykl życia należy do gry i to ona go finalizuje. Postać znika ze stron publicznych (`profile: []`). |
| Ukrycie/pokazanie w profilu publicznym | GC2 `SetCharacterVisibility` | M3 | Projekcje publiczne filtruje gra, więc to musi być komenda gry. |
| Komentarz / podpis profilu | P | M3 | 500 znaków, moderowany, `support.content.manage` może usunąć. |
| Główna postać | P | M3 | Używana na publicznej karcie konta i podpisach na forum. |
| Transfer świata | GC2 `TransferCharacterWorld` | M8 | Wymaga reguł transferu świata w grze. |
| Podgląd stroju | GC3 (look type, kolory, addony, wierzchowiec) + assety katalogu | M6 | Kompozytor po stronie serwera; webp w cache w magazynie obiektów. |
| Osiągnięcia, zgony, statystyki | GC3 (osiągnięcia, umiejętności, statystyki bazowe), PUB `character_deaths` | M3; osiągnięcia M6 | Właściciele widzą też dane swoich ukrytych postaci. |
| Prywatność per postać | P (flagi renderowania), GC2 dla ukrycia | M3 | Flagi: pokaż zgony, osiągnięcia, bestiary, strój. |
| Historia coinów, historia czasu premium | P (ledgery) | M3, zmigrowane dane tylko do odczytu | |
| Zakupy, prezenty, faktury | P | M7 | Prawdziwy dostawca płatności |
| Wpisy sklepu / bazaru | P (+ GC2 Transfer) | M7 / M8 | Ukryte za flagami funkcji do tego czasu. |
| Zgłoszenia do supportu | P | M3 | |
| Status banów i odwołań | P (rekordy egzekwowania, evidence `disabled_at`) | M3 | |
| Haki narzędzi gracza | P + GC3 | M6 | „Narzędzia dla tej postaci”: tracker w trybie automatycznym, kalkulator exp wstępnie wypełniony. |

### Trasy

Wszystkie pod `/{locale}/account/...`, SSR + htmx 4:

```text
/                         przegląd            /security            hasło, 2FA, klucz odzyskiwania, wynik
/email                    zmiana              /email/confirm/{token}, /email/revert/{token}
/sessions                 lista + unieważnienie   /history         logowania + zdarzenia bezpieczeństwa
/privacy                  eksport, usunięcie  /preferences         powiadomienia, język, strefa czasowa
/characters               lista (status na żywo przez fragment hx-trigger="every 15s")
/characters/new           tworzenie
/characters/{id}          szczegóły (statystyki, strój, osiągnięcia, zgony, oczekujące komendy)
/characters/{id}/rename   /delete  /visibility  /settings (komentarz, główna, prywatność)  /transfer (M8)
/premium                  historia czasu premium i coinów     /shop/history, /invoices (M7)
/bazaar (M8)              /support/tickets, /support/tickets/{id}   /sanctions (bany, odwołania)
/tools                    wpisy narzędzi powiązane z postacią
/sudo                     ponowne uwierzytelnienie (hasło lub TOTP), powrót na poprzednią stronę
```

Szablony: `templates/account/{overview,security,email,sessions,history,privacy,preferences,premium,sanctions,sudo}.html` oraz `templates/characters/{list,_row,new,detail,rename,delete,visibility,settings}.html`.

### Zasady UX

- **Tryb sudo** (ponowne wpisanie hasła lub TOTP; ważny 10 minut, związany z sesją) jest wymagany dla:
  - zmian e-maila, hasła, 2FA i klucza odzyskiwania;
  - unieważnienia wszystkich sesji;
  - eksportu danych i usunięcia konta;
  - zmiany nazwy, usunięcia, transferu lub ukrycia postaci.
- **Akcje destrukcyjne** używają strony potwierdzenia (nie JS confirm), która pokazuje skutki i prosi o wpisanie nazwy postaci. Każda z nich wysyła e-mail z powiadomieniem.
- **Oczekujące komendy gry** pokazują swój stan w wierszu postaci („zmiana nazwy w toku…”). Stan odświeża polling htmx do czasu potwierdzenia albo gdy LCFA/GC3 to odzwierciedli. Błędy pokazują zmapowany komunikat `CHAR_CMD_*` po polsku lub angielsku.

### Deliverable gry dla MVP panelu

- **GC2:** `RenameCharacter`, `RequestCharacterDeletion`, `CancelCharacterDeletion`, `SetCharacterVisibility`, `CreateCharacter` v2.
- **GC3:** projekcja szczegółów postaci.

Oba są potrzebne do **M3**. W przeciwnym razie te wiersze wychodzą wyłączone, co blokuje element „profesjonalny panel” bramki przełączenia.

## Podgląd sesji gracza przez GM

Gra dziś nie ma funkcji obserwatora. W `protocol-oteryn`, `apps/game-server`, crate'ach sesji i dokumentach „spectator” i „observer” występują tylko w kontekście zasięgu okrzyku Canary, „widoku referencyjnego” ruchu (-8..9 x -6..7 kafelków) i kolejności czaru dla widzów. Są jednak dwa elementy, na których można to oprzeć:

- **System capability i domen stanu.** Capabilities są leasingowane przez control plane #1622.
- **Capability protokołu `ANALYSER_V1`.** To capability 10, domena stanu 15 `ACTOR_ANALYSER`; niesie fakty o obrażeniach, leczeniu, doświadczeniu, łupie i zabójstwach. Schemat: `docs/contracts/protocol-oteryn/v1/analyser_v1.proto`. Jeszcze nie jest oferowane; czeka na ANALYSER-EMIT-1.

### Rekomendacja: (c) oba, etapami

| Opcja | Co daje | Wykonalność na tym, co istnieje | Decyzja |
| --- | --- | --- | --- |
| (b) „Observe-lite” w przeglądarce, w adminie | Pozycja na żywo na kafelkach minimapy (te same kafelki i przeglądarka co mapa wiki), pobliskie stwory i gracze jako znaczniki ze sprite'ami z assetów katalogu, ślad ruchu, czat publiczny, log akcji (ataki, czary, łup, handel, użycie NPC, zgony) oraz paski HP/MP/poziomu | **Wykonalne.** Wymaga tylko zsanityzowanego feedu zdarzeń z gry. To **nie** jest widok gry co do piksela: minimapa pokazuje jeden kolor na kafelek, a prawdziwy widok oznaczałby reimplementację renderera klienta w przeglądarce. Ta wierność wystarcza do anti-bota i anti-cheatu (ścieżki, czasy reakcji, wzorce łupu) oraz do supportu. | **Etap 1** |
| (a) Podgląd w kliencie | Klient Oteryn GM-a podpina się tylko do odczytu do widoku celu i widzi dokładnie to, co gracz | Wymaga nowej capability protokołu, nowego profilu grantu admission i „sesji obserwatora bez aktora”. Renderer klienta już jest gotowy. | **Etap 2.** To narzędzie o pełnej wierności. |

Dlaczego ta kolejność: (b) wymaga mniej pracy w grze, działa z dowolnej przeglądarki z pełnym audytem, a jego feed jest zarazem formatem nagrań. (a) to złoty standard, ale większa i ryzykowniejsza zmiana w warstwie sesji gry.

### Co musi zbudować gra: GC10 „GM Observe v1” (`docs/contracts/OTERYN_GAME_GM_OBSERVE_V1.md`)

**Autorytet: platforma wydaje, gra egzekwuje.**

1. Platforma tworzy **grant obserwacji**, JWS Ed25519 podpisany istniejącym natywnym keyringiem admission pod nowym profilem `oteryn-observe-v1`. Pola:
   - `observe_id` (UUIDv7) i `approval_id`
   - `gm_account_id`
   - `target_character_id`, `world_id`
   - `mode` (`feed` | `client`)
   - `pm_escalation` (domyślnie false)
   - `max_duration` (≤30 min)
   - `issued_at`, `expires_at`
   - `audience = OTERYN_GAME`
2. Gra sprawdza cztery rzeczy:
   - podpis i kid względem bieżącego lub wycofywanego zaufania;
   - audience i wygaśnięcie;
   - że cel jest online na tym świecie;
   - że konto GM-a nie jest kontem własnym celu.
3. Gra wiąże następnie grant z **bieżącą generacją sesji** celu.

**Ścieżka dostarczania w trybie feed, (b):**

- Platforma kolejkuje `StartObserve` / `EndObserve` przez **Operator Commands v1 (GC4)**. Gra pobiera je jak wszystkie komendy.
- Gra pcha klatki na `POST /internal/v1/gm-observe/frames`, ok. 1 paczka na sekundę, do 64 KB każda. Klatka niesie `{observe_id, target_session_generation, seq, ts, position, view_entities[], public_chat[], actions[]}`.
- `actions` budowane są z tych samych faktów co `ANALYSER_V1`, plus zdarzenia ruchu i interakcji.
- Klatki idą na **dedykowanym certyfikacie klienta** (`CN=oteryn-<env>-gm-observe`) i osobnej puli połączeń. Limity HTTP klienta gry (2 sloty pipeline, 8 w kolejce) nigdy więc nie opóźniają komend ani projekcji.
- Najwyżej 3 równoczesne obserwacje na świat.

**Ścieżka dostarczania w trybie klienta, (a):**

- Nowa capability protokołu `OBSERVE_V1` i domena stanu tylko do odczytu.
- Klient GM-a loguje się zwykłym przepływem OAuth, ticket i gateway, przedstawiając grant obserwacji. Admission zwraca **sesję obserwatora bez aktora w świecie**. Niewidzialność wynika z konstrukcji: nie ma stworzenia, wpisu na liście widzów, kolizji ani niczego wysyłanego do innych klientów.
- Gra odzwierciedla obserwatorowi domeny stanu widoku celu (mapa, stworzenia, efekty, własne statystyki, czat).
- Wszystkie komendy obserwatora są odrzucane poza `detach`.

**Odcięcie (oba tryby).** Strumień kończy się potwierdzeniem do platformy, gdy:
- cel się wyloguje albo zmieni się generacja sesji celu (relog wymaga nowego startu pod tą samą akceptacją, jeśli jest jeszcze ważna);
- grant wygaśnie;
- przyjdzie `EndObserve`;
- GM się rozłączy;
- gra się zrestartuje.

Każda klatka niesie generację i monotoniczny `seq`. Platforma odrzuca każdą klatkę, której generacja lub `seq` się nie zgadza.

**Wiadomości prywatne.** Treść PM i kanałów prywatnych jest **wykluczona u źródła**, w grze, chyba że grant ma `pm_escalation = true`. Nawet wtedy dołączane są tylko rozmowy z udziałem celu.

**Szacunek pracy po stronie gry:**

| Tryb | Praca | Nakład |
| --- | --- | --- |
| Feed (b) | Kontrakt, budowniczy feedu, weryfikacja grantu, czasowniki operatora | 2–3 tygodnie |
| Klient (a) | Capability protokołu, sesja obserwatora bez aktora, odzwierciedlanie widoku, filtr PM, HUD obserwatora w kliencie | 4–6 tygodni |

### Bezpieczeństwo i prywatność

**Uprawnienia:**

| Uprawnienie | Co pozwala |
| --- | --- |
| `gm.observe.request` | Poprosić o sesję obserwacji |
| `gm.observe.approve` | Zatwierdzić cudzą prośbę |
| `gm.observe.pm_escalate` | Zezwolić na treść PM (tylko rola admin) |
| `gm.recordings.view` | Oglądać nagrania |
| `gm.recordings.export` | Eksportować nagrania |

**Zasady:**

- **Powód i link są zawsze obowiązkowe.** Link wskazuje zgłoszenie, raport lub sankcję. Sam powód tekstem jest dozwolony tylko dla „przeglądu anti-cheat” (sweep), który zawsze wymaga zatwierdzenia czterech oczu.
- **Zatwierdzenie czterech oczu** (drugi członek staffu z `gm.observe.approve`) jest wymagane dla:
  - przeglądów bez otwartego raportu;
  - każdej eskalacji PM;
  - każdego eksportu nagrania;
  - każdej obserwacji na produkcji, w której wnioskujący nie ma powiązanej otwartej sprawy.

  Przy powiązanej otwartej sprawie pojedynczy GM może zacząć od razu, a druga osoba musi to przejrzeć w ciągu 24 godzin.
- **Zawsze tylko do odczytu.** Nie ma w ogóle trybu „działaj jako gracz”.
- **Twarde limity:** 30 minut na grant, a odnowienie wymaga nowego zatwierdzenia. Do startu potrzebne jest ponowne uwierzytelnienie (tryb sudo).
- **Audyt** (`audit_events`, łańcuch hashy) zapisuje: kto, kogo (konto i postać), kiedy, czas trwania, tryb, powód, powiązaną sprawę, zatwierdzającego, eskalację PM tak/nie, powód zakończenia i czy powstało nagranie.
- **Miesięczny nadzór:** automatyczny raport wszystkich sesji obserwacji trafia do właściciela.

**Powiadamianie gracza:**

- **Bez powiadomienia w czasie rzeczywistym.** Przekreślałoby anti-cheat.
- **Ujawnienie** w regulaminie i polityce prywatności, art. 13 RODO: co jest obserwowane, po co, retencja i jak się sprzeciwić.
- Gdy obserwacja została poproszona **przez samego gracza** (sprawa supportu), gracz jest informowany o niej w momencie jej wykonania, na tym zgłoszeniu.

**RODO (UE/Polska; wskazówki architektoniczne, nie porada prawna; właściciel powinien zlecić przegląd prawnikowi lub IOD):**

- **Podstawa prawna:** art. 6 ust. 1 lit. f, prawnie uzasadniony interes: zapobieganie oszustwom i cheatingowi, zabezpieczenie usługi i support. Napisać i przechowywać ocenę prawnie uzasadnionego interesu.
- **DPIA:** wykonać (art. 35). To systematyczne monitorowanie i może podlegać liście UODO operacji wymagających DPIA.
- **Minimalizacja wbudowana:**
  - domyślny jest tryb feed;
  - domyślnie bez PM;
  - inni gracze w pobliżu występują jako samo imię i pozycja, tylko w obrębie widoku;
  - w feedzie nie ma adresów IP ani danych konta.
- **Prawa:**
  - **Sprzeciw, art. 21:** rozpatrywany testem równowagi, udokumentowanym per wniosek.
  - **Dostęp, art. 15:** nagrania dotyczące gracza wchodzą do jego eksportu danych, z zaczernionymi danymi osób trzecich.
- **Treść PM:** tajemnica komunikacji czyni to najwyższym ryzykiem, dlatego eskalacja wymaga dwóch osób i powiązanej sprawy oraz jest logowana osobno.

### Nagrywanie i odtwarzanie (opcjonalne per sesja; domyślnie wyłączone)

- **Format:** nagrywane są tylko zsanityzowane klatki feedu, skompresowane, odtwarzane w tej samej przeglądarce webowej z suwakiem osi czasu. Bez wideo. W trybie klienta serwer nagrywa równolegle ten sam feed, nigdy surowy strumień klienta.
- **Retencja:**

  | Nagranie | Przechowywane |
  | --- | --- |
  | Niepowiązane ze sprawą | 30 dni, potem automatyczne usunięcie |
  | Powiązane z raportem, sankcją lub odwołaniem | Do zamknięcia sprawy i końca okna odwołań, plus 90 dni |
  | Dowolna sprawa | Nigdy dłużej niż 1 rok, poza udokumentowanym legal hold ustawionym przez admina |

- **Dostęp:** każde obejrzenie lub eksport jest logowane w `gm.recording_access_log`. Eksport zawsze wymaga zatwierdzenia czterech oczu i niesie znak wodny z tożsamością eksportującego.

### Umiejscowienie

GM observe jest **po przełączeniu**. Laravel nie ma takiej funkcji, więc nie jest to pozycja parytetu i nie przesuwa bramki przełączenia. Tryb feed można przesunąć do M5, jeśli gra dostarczy GC10a razem z GC4.

| Etap | Zakres | Nakład platformy | Nakład gry |
| --- | --- | --- | --- |
| **M7b** (równolegle do M7) | Lekki podgląd webowy, zatwierdzenia, audyt, nagrywanie, job retencji | 3–4 tygodnie | GC10a, 2–3 tygodnie; wymaga GC4 |
| **M9** | Podgląd w kliencie: platforma wydaje granty, tryb klienta, audyt | 1 tydzień | GC10b, 4–6 tygodni |

Struktura katalogów dla GM observe jest w sekcji „Struktura katalogów”, a tabele w schemacie `gm` w sekcji „Schemat bazy platformy”. Ryzyka tej funkcji są w sekcji „Ryzyka i otwarte decyzje”.

## Migracja i przełączenie

### Zasady

- Bez podziału ruchu. Laravel działa bez zmian do T0.
- Wszystko jest ćwiczone na preprodzie v2 ze świeżych snapshotów produkcji: **co najmniej trzy próby**, w tym jedna z **importem odwrotnym**.
- Te same nazwy hostów, ścieżki, bajty `/internal/v1`, CA, subjecty certyfikatów, hashe bearerów i klucze Ed25519. Konfiguracje produkcyjnej gry i gatewaya się nie zmieniają. Gateway w Go działa dalej bez zmian konfiguracji, bo upstream ma ten sam adres i port.

### Import danych (`tools/legacy-import`)

- Czyta odtworzony `pg_dump` Laravela (PG 17.11), nigdy żywą bazę; pisze do v2. Tryby: `import`, `verify`, `reverse`.
- Zachowuje: ID tożsamości, hashe argon2id i kody odzyskiwania; sekrety 2FA (odszyfrowane z Laravelowego `encrypted` kluczem APP\_KEY i zaszyfrowane key ringiem v2); klientów OAuth i redirecty; hashe sesji urządzeń jako wersja legacy; ID światów, kanałów i tras z epokami; `native_security_generation`, floory evidence i pliki witness; klucze admission z kid; intencje i potwierdzenia komend postaci; epoki i rewizje LCFA; ledger premium; całą treść CMS, wiki, media, eventy, pobieranie i updater; support, audyt i role (1:1 na 23 uprawnienia).
- Pomija: dane Canary i Player Companion, sesje, cache, joby i tickety Laravela.
- Publiczne dane gry nie są importowane: po przełączeniu produkcyjny węzeł gry zaczyna pchać do v2 Public Projections v1.
- `verify` porównuje liczby wierszy i digesty kolumn z sekretami, równość generacji, floorów i witness, sprawdza hasło konta testowego i rewizje LCFA z bazą gry (tylko w próbach). Każda niezgodność przerywa import.
- Próby: na preprodzie v2 ze świeżych snapshotów prod: import, testowy węzeł gry, pełny test zgodności, logowanie, lista i tworzenie postaci, snapshot premium.

### Runbook

| Kiedy | Krok |
| --- | --- |
| T-14 d | Zamrożenie funkcji i schematu Laravela. Ogłoszenie prac (baner na stronie, Discord). |
| T-7 d | Próba #3 ze świeżym snapshotem prod. Zapis czasów. |
| T-1 d | Wstępne pobranie obrazów v2 prod. Przygotowanie sekretów prod (CA, certyfikat/klucz serwera, allowlista DN, hashe bearerów gatewaya, klucze Ed25519 admission i kid, legacy APP\_KEY, pliki witness evidence z uprawnieniami, klucz komend ops, pośrednie CA agentów). Utworzenie bazy v2 prod i migracje. Import i aktywacja paczki katalogu bieżącego wydania gry. Instalacja pakietów node-agent i supervisor, **wyłączonych**. |
| T0 | Maintenance trasy przez Laravela; gateway odpowiada ROUTE\_UNAVAILABLE. Oczekiwanie na odpłynięcie graczy. Zatrzymanie gry przez `game-supervisor.py` (łagodnie). |
| T0+5 | `artisan down`; zatrzymanie schedulera Laravela, kolejki i schedulera marketplace. Ostatnia kopia PG Laravela. |
| T0+10 | `legacy-import` + `verify`: liczby, hashe poświadczeń, generacje, floory, witness, `source_revision` komend, epoki LCFA, ledger premium. **Każda niezgodność: przerwanie i rollback (a).** |
| T0+25 | Zatrzymanie i wyłączenie nginx internal-mtls oraz `oteryn-game.service` (supervisor python). Start v2 prod: p2-internal **na 172.18.0.12:8543**, p2-public, p2-worker. Włączenie node-supervisor + node-agent, które przejmują istniejący węzeł: import stanu kontrolera z `/opt/oteryn-oracle/state/game-controller`; ustawiona flaga `reconciliation-required` blokuje przełączenie. |
| T0+30 | `caddy reload` z `cutover.caddy`: `oteryn.com` → p2-public. Trasy gatewaya bez zmian. |
| T0+35 | Start gry z konsoli v2. Sprawdzenie rejestracji, runtime status, przypisania scope, watermarku LCFA ≤10 s i płynących projekcji publicznych. |
| T0+40 | Testy smoke: logowanie prawdziwym klientem (OAuth → ticket → gateway → admission); jedno `CreateCharacter` na koncie testowym; self-check evidence; sesja urządzenia; directory; strony wiki i narzędzi; wysyłka e-maila; podgląd logów. |
| T0+50 | Zdjęcie maintenance. Docelowe okno to **≤60 minut**; decyzja go/no-go należy do właściciela. |
| T+14 d | Okno pracy równoległej. Laravel zostaje zainstalowany, ale zatrzymany, z bazą tylko do odczytu. Dla porównania może działać staffowy Laravel tylko do odczytu na wewnętrznym hoście. Codzienny raport rekoncyliacji. |

Przekierowania: każdy istniejący publiczny adres zostaje. Wycofane funkcje zwracają 301 na najbliższą stronę albo 410. Mapa przekierowań jest sprawdzana w CI względem crawla starego sitemapu.

### Rollback

- **(a) Przed T0+50 (brak zapisów graczy w v2):**
  1. Przywrócenie Caddy.
  2. Zatrzymanie v2.
  3. Start nginx i Laravela.
  4. Ponowne włączenie `oteryn-game.service`.

  Bez utraty danych: baza Laravela jest nietknięta.
- **(b) Po zapisach graczy w v2:** **import odwrotny do przodu** (`legacy-import reverse`, przećwiczony) do schematu Laravela. Przenosi tożsamości, hashe, podniesione generacje, floory, witness, intencje i potwierdzenia komend, epoki LCFA i wiersze ledgera premium. Potem powrót do poprzednich kontenerów.
  - Zwykłe przełączenie z powrotem jest zabronione, bo gra odrzuciłaby cofnięte floory i rewizje.
  - Dane istniejące tylko w v2 (katalog, opisy wiki, ops, narzędzia, nowe funkcje konta) zostają w bazie v2 na kolejną próbę.
- **Wyzwalacze rollbacku:**
  - odsetek udanych logowań poniżej 95% przez 15 minut;
  - błędy admission lub evidence powyżej bazowych;
  - jakikolwiek alarm regresji licznika monotonicznego;
  - nieudany verify.
- **Okno rollbacku zamyka się w T+14 d.** Potem Laravel i runner na Synology są wycofywane, a obrazy archiwizowane (obraz Laravela, ostatni dump, APP\_KEY w zimnym magazynie).

## Roadmapa

### Kamienie milowe budowy na v2-preprod

Nakład zakłada jednego seniora Rust na platformie; dwóch developerów to w przybliżeniu ×0,6 czasu kalendarzowego.

| M | Zakres | Nakład | Potrzebny deliverable gry | Demo dla właściciela |
| --- | --- | --- | --- | --- |
| M0 | Repo, CI na 26.04, obrazy, stosy v2 preprod i testgame, własne CA/certyfikaty, PG, layout + CSS + i18n, obserwowalność. Wspólne crate'y wyciągnięte w grze (GC6). | 2–3 tyg. | crate'y GC6 + podbicie toolchaina | v2.oteryn.com działa |
| M1 | Tożsamość, OAuth PKCE, tickety, sesje urządzeń, directory, rdzeń bezpieczeństwa konta (wiersze oznaczone M1), tryb sudo, legacy-import v0 | 4–5 tyg. | — | Rejestracja, 2FA, sesje |
| M2 | Wszystkie rodziny `/internal/v1` w parytecie, zgodność z węzłem testowym, tworzenie/lista postaci, premium, ops rejestru/topologii | 3–4 tyg. | — | **Gra na testowym węźle v2 (najszybsze odblokowanie, ok. tydzień 8)** |
| M3 | Pełny panel konta i postaci, publiczne strony gry, zgłoszenia, sankcje, odwołania, eksport i usunięcie GDPR | 4–5 tyg. | GC1, GC2, GC3 | Profesjonalny panel |
| M4 | CMS, newsy, strony, eventy, ogłoszenia, pobieranie, launcher/updater, admin RBAC + audyt | 3–4 tyg. | — | Parytet treści |
| M5 | Zarządzanie serwerami: agent, supervisor, logi, maintenance, odliczanie, deploy, tworzenie kanałów/światów | 4–6 tyg. | GC4, GC7 | Restart i dodanie kanału z przeglądarki |
| M6 | Import katalogu, wygenerowane strony wiki + opisy + mapa + wyszukiwanie, MVP narzędzi gracza, podgląd stroju, tracker w trybie automatycznym | 5–7 tyg. | GC5, GC8, GC9 | Pełna wiki i narzędzia |
| Bramka | Hardening, przegląd bezpieczeństwa, test obciążenia, 3 próby (w tym odwrotna), UAT | 2 tyg. | — | Akceptacja właściciela |
| **Przełączenie** | Sekcja „Migracja i przełączenie” | 1 noc | | |
| M7 | Handel: prawdziwy dostawca, coiny, prezenty, faktury, sklep | 4–6 tyg. | Inbox (opcjonalnie) | |
| M7b | **GM observe, etap 1:** lekki podgląd webowy, zatwierdzenia, audyt, nagrywanie, retencja (równolegle do M7; tryb feed można przesunąć do M5, jeśli gra dostarczy GC10a razem z GC4) | 3–4 tyg. | GC10a (2–3 tyg.); wymaga GC4 | |
| M8 | Bazar + transfer świata | 4–6 tyg. | Konsument GC2 Transfer/WorldTransfer | |
| M9 | Narzędzia gracza, faza 2 (forge, imbuementy, umiejętności, obrażenia), passkeys, zarządzanie gildiami z weba; **GM observe, etap 2:** podgląd w kliencie (platforma wydaje granty, tryb klienta, audyt; ok. 1 tydzień) | 5–7 tyg. | Wypełnione zestawy reguł, GUILD-0; GC10b (4–6 tyg.) | |
| M10 | Dostawca OIDC, gateway w Rust (ADR), multi-host | — | — | |

**MVP = M0–M6 + bramka:** ok. 27–36 tygodni dla jednego developera, ok. 16–20 tygodni dla dwóch. GM observe jest po przełączeniu i nie przesuwa bramki.

### Deliverable'y w repo gry

| ID | Deliverable | Potrzebny do |
| --- | --- | --- |
| GC1 | Producent Public Projections v1 | M3 |
| GC2 | Komendy postaci v2: Rename, Request/CancelDeletion, SetVisibility, Create v2 (później konsument Transfer, WorldTransfer) | M3 (M8) |
| GC3 | Account Character Details v1, tylko dla właściciela: online, ostatnie logowanie/wylogowanie, poziom, profesja, płeć, strój, statystyki, osiągnięcia, plus postęp bestiary/charmów/strojów | M3 (część postępu M6) |
| GC4 | Operator Commands v1: broadcast, kick, mute, namelock, SaveAndShutdown. Plus potwierdzenie, że SIGTERM zapisuje. | M5 |
| GC5 | Catalog Export v2 + Bundle v1 + eksporter Rust + podpisana publikacja OCI + dane licencyjne | M6 |
| GC6 | Wyciągnięcie `platform-wire`, `catalog`, `game-rules`; toolchain 1.99.1; CI na ubuntu-26.04 | M0–M1 |
| GC7 | Obsługa wielu węzłów/kanałów na jednym hoście, udowodniona na węźle testowym | M5 |
| GC8 | ADR + wyciągnięcie `oteryn-game-rules` z `RULES_ABI` i wektorami | M6 |
| GC9 | Format eksportu sesji polowania/party z klienta Oteryn | M6 (lub M9) |
| GC10 | GM Observe v1: **GC10a** feed GM observe (M7b); **GC10b** sesje obserwatora w kliencie (M9) | M7b / M9 (po przełączeniu) |

### Bramka gotowości do przełączenia (wszystko musi być zielone)

1. **Parytet:** każda funkcja Laravela, która dziś działa natywnie, istnieje w v2 albo jest jawnie wycofana za zgodą właściciela.
2. **Kontrakt:** zestaw zgodności przechodzi w 100% przeciw digestowi **produkcyjnego** obrazu gry. Złote wektory wire się zgadzają.
3. **Dane:** 3 próby importu z zerem różnic w verify, 1 próba importu odwrotnego, zmierzone okno ≤60 minut.
4. **Bezpieczeństwo:**
   - weryfikacja legacy argon2, odszyfrowanie 2FA i wyszukiwanie po keyed-hashach przetestowane na snapshocie prod;
   - CSRF i limity;
   - tryb sudo;
   - fuzzing czasowników supervisora;
   - zewnętrzny lub koleżeński przegląd bezpieczeństwa;
   - czysty `cargo deny`.
5. **Wydajność:** `/internal/v1` p99 < 250 ms przy 2× oczekiwanego obciążenia pollingiem; strony publiczne p95 < 300 ms.
6. **Operacje:**
   - kopia + ćwiczenie odtworzenia zaliczone;
   - alerty dla opóźnienia projekcji, wieku komend, błędów admission, heartbeatu agenta i dysku;
   - napisane runbooki;
   - agent węzła przetestowany na awarię, utratę sieci i zużyty plik startowy.
7. **Produkt:** pokrycie i18n 100% dla ciągów UI w EN i PL; strony prawne; e-mail SPF, DKIM i DMARC; UAT zaakceptowany przez właściciela.
8. **Katalog:** paczka bieżącego wydania gry zaimportowana, zweryfikowana i aktywowana, z zatwierdzonymi licencjami.

## Ryzyka i otwarte decyzje

| Ryzyko / decyzja | Jak ograniczamy / rekomendacja |
| --- | --- |
| Wielkie przełączenie koncentruje ryzyko | Trzy próby, przećwiczony import odwrotny, okno ≤60 minut, obiektywne wyzwalacze rollbacku, 14-dniowe okno |
| Deliverable'y gry GC1–GC10 się opóźnią, co blokuje bramkę | Zablokować kontrakty w `CROSS_REPOSITORY_CONTRACT_LOCK.json` do M1. Funkcje wychodzą za flagami. Uzgodnić z właścicielem, które wiersze mogą wejść wyłączone (np. transfer świata). |
| **IP sprite'ów i assetów:** grafika pochodzi z klienta Tibia 15.30 (appearances; klucze `oteryn:item.tibia.*`) | **Decyzja właściciela.** Eksporter publikuje tylko assety, na które zezwala `licensing.json`, i zawodzi (fail closed). Wiki działa bez obrazków. |
| Zestawy reguł niewypełnione (forge, stamina, skills, damage) lub szkic (imbuements) | Narzędzia zostają ukryte przez kontrolę capabilities. Nigdy nie hardcodować wartości. |
| Adapter Python dla katalogu v1 nie jest zbudowany; zakres v2 jest duży | Eksporter Rust w repo gry, z Pythonem jako kontrolą krzyżową. Zacząć od item, creature, loot, npc, trade i area, potem rozszerzać. |
| Dryf formuł między grą a stroną | Wspólny crate `oteryn-game-rules`, `RULES_ABI`, wektory uruchamiane ponownie przy aktywacji |
| Agent węzła staje się drogą do roota na hoście | Zamknięty zestaw czasowników, podpisane komendy, obrazy podpisane przez wydanie, supervisor bez sieci, agent bez uprawnień, cztery oczy na produkcji, limity |
| Przejęcie żywego węzła przez supervisora przy przełączeniu | Port semantyki `game-supervisor.py` jeden do jednego, test na stosie testgame, blokada przełączenia flagą reconciliation |
| Konsumenci downstream nie dostają wendorowanych łatek tokio/sqlx gry | Wspólne crate'y nie mają zależności async/DB. Zapytać zespół gry, dlaczego sqlx i tokio są załatane. |
| Niezgodność glibc między builderem a obrazem uruchomieniowym | Builder Ubuntu 26.04 tylko z chiselled runtime 26.04; builder trixie z distroless debian13 |
| Slice'y chisel `ubuntu-26.04` niezweryfikowane | Zweryfikować w M0; awaryjnie distroless debian13 z builderem trixie |
| PG 19 to wciąż RC | Budować na 18.6. Przyjąć 19 na preprodzie po GA; na prod tylko jeśli GA było co najmniej 6 tygodni przed bramką. |
| Pojemność i dysk jednego VPS z 3–4 stosami | Pomiar w M0, wyłączanie preprodu/testgame w spoczynku, kopie poza maszynę, czyszczenie obrazów, OCI Object Storage na assety |
| Objętość logów w PG | Partycje dzienne, retencja 14 dni, alarmy rozmiaru; VictoriaLogs za traitem `LogStore` jako wyjście awaryjne |
| Self-hosted runner na hoście prod | Prywatne repo, efemeryczny runner, chronione środowiska, tylko przypięte digesty |
| Upgrade Ubuntu na oteryn-vps | Osobne okno przed M0, kopia boot volume, testy smoke gry i Laravela po upgradzie |
| htmx 4 jest nowy (sierpień 2026) | Greenfield, więc od razu 4.x, wendorowany po hashu; minimum JS, żeby powrót do 2.x był tani |
| Migracja etykiety `ubuntu-latest` (19.10–19.11.2026) | Jawne `ubuntu-26.04(-arm)` w obu repo; CI gry schodzi z `ubuntu-24.04` w PR toolchaina |
| Wierność odszyfrowania `encrypted`/APP\_KEY | Testy na fixture'ach z obecnego Laravela i próby; APP\_KEY w zimnym magazynie po wyłączeniu |
| Dane produkcyjne w środowisku prób (PII) | Środowisko prób niewystawione do internetu, snapshoty kasowane po każdej próbie, dostęp logowany |
| GM observe: nadużycie przez staff (stalking od środka) | Obowiązkowy link do sprawy, zatwierdzenie czterech oczu, limity czasu, audyt z łańcuchem hashy, miesięczny raport dla właściciela, polityka sankcji dla staffu |
| GM observe: zakwestionowanie prawne lub RODO | Ocena prawnie uzasadnionego interesu, DPIA, ujawnienie w regulaminie i polityce prywatności, minimalizacja, krótka retencja, przegląd prawnika przed uruchomieniem |
| GM observe: obecność obserwatora wycieka do celu (kanały boczne, czasy) | Tryb klienta to sesja bez aktora. Tryb feed działa w procesie gry i nic nie wysyła do klientów. Test, że strumień pakietów celu jest bajt w bajt identyczny z obserwatorem i bez niego. |
| GM observe: ruch feedu zakłóca krytyczne wywołania wewnętrzne | Dedykowany certyfikat i pula połączeń, paczkowanie 1 Hz, limit współbieżności |
| GM observe: rozjazd oczekiwań co do wierności widoku webowego | Powiedzieć wprost, że widok webowy to widok analityczny o wierności minimapy; pełna wierność przychodzi w M9 przez klienta |
| GM observe: nagrania zawierają dane osób trzecich | Minimalizacja, zaczernianie w eksportach danych, logowanie dostępu, retencja |
| **Otwarte:** okresy karencji (usunięcie postaci 30 dni? konta 30 dni?), cooldown i cena zmiany nazwy | Właściciel |
| **Otwarte:** wychodzące sprawdzenie haseł w HIBP | Rekomendacja: tak (k-anonymity) |
| **Otwarte:** dostawca płatności | Przed M7 (Przelewy24/BLIK albo Stripe) |
| **Otwarte:** dostępność staffu dla zasady czterech oczu | Jeśli jest tylko jedna osoba ops, dopuścić opóźnione samozatwierdzenie (15 minut chłodzenia) |
| **Otwarte:** licencja na sprite'y | Właściciel (patrz „Najważniejsze”) |
| **Otwarte:** polityka klucza odzyskiwania | Właściciel |

## Dowody dodane w tej rewizji (tylko do odczytu)

**Oteryn-Game:**

- `Cargo.toml` (`[patch.crates-io]`: wendorowane sqlx-core/postgres 0.9.0 i tokio 1.53.1), `rust-toolchain.toml` (1.94.0), `.github/workflows/rust.yml` (`ubuntu-24.04`, `+1.95.0`)
- `tools/game-platform-catalog/README.md`, `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`
- `content/{creatures,items,npcs,loot,quests,world,assets,cosmetics,houses,abilities,charms,services,presentations}/*`
- `rulesets/{character,progression,items,economy,combat}/*`
- `crates/client-assets/src/lib.rs`
- `apps/game-server/migrations/{0019,0020,0021,0056}*`
- `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §4.1–4.2 (wykluczenie ukrytych postaci; brak ostatniego logowania i online w profilu)
- `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (tylko Create/Transfer; płeć, profesja i strój nie są wejściami v1; zmiana nazwy, usunięcie i transfer świata poza zakresem)
- `docs/contracts/protocol-oteryn/v1/analyser_v1.proto` oraz kod capability analysera (dla GM observe)

**Oteryn-Platform:**

- `deploy/oracle/{game-supervisor.py, oteryn-game.service, game-runtime.compose.yml, public-platform.compose.yml}`
- `app/GameCatalog/*` (import, validate, diff, activate, rollback, verify)
- `app/Admin/AdminPermission.php` (obecne uprawnienia)
