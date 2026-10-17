# Oteryn Platform 2 (Rust) – projekt architektury

> **Status: DRAFT, rewizja 4.1** (2026-10-15). Projekt do akceptacji przez właściciela. Domyślne decyzje z sekcji 0 obowiązują i są budowane, o ile właściciel nie zmieni ich przed podanym tam terminem.
> Źródłem prawdy jest ten plik w repozytorium. Wersja robocza na żywo (podgląd, może być o krok z tyłu): https://claude.ai/code/artifact/e809ac5a-3f4c-42c2-91b6-b4eaa489b950

Oteryn Platform 2 to nowa, kompletna platforma w Ruście: strona, konto i postacie gracza, wiki z katalogu gry, narzędzia dla gracza, Oteryn-Atlas ze wspólnym silnikiem mapy, administracja, zarządzanie serwerami gry i podgląd sesji gracza dla GM-a. Powstaje w osobnym repozytorium `Oteryn-Platform-2`. Główna binarka `oteryn-platform` ma role (`serve-public`, `serve-internal`, `worker`, `migrate`, `ops`); obok niej są `oteryn-login-gateway` (Rust, zastępuje gateway w Go), `oteryn-node-agent` i `oteryn-node-supervisor`. Stos: axum + askama + htmx 4 + PostgreSQL. Kontrakt `/internal/v1` z serwerem gry zostaje bajt w bajt.

Ta rewizja zastępuje rewizje 1 i 2 oraz dodatek O. Niczego tu nie odkładamy i nie uzależniamy od warunków: wszystko poniżej jest zbudowane, wdrożone na preprodzie v2, przechodzi swoją bramkę i trafia na produkcję w jednym przełączeniu. Gdy właściciel ma coś rozstrzygnąć, plan buduje z podaną wartością domyślną i podaje termin zmiany, więc żadna decyzja nie blokuje budowy. Platformę budujemy obok obecnej, bez strangler, a po bramce przełączamy ją jednym przełączeniem zamiast Laravela.

## 0. Podsumowanie i domyślne decyzje właściciela

**Kalendarz.** Platform 2 powstaje w 13 kamieniach milowych: M0–M5, **MA (Atlas i ujednolicony silnik mapy)**, potem M6–M11. Po M11 następuje jedno przełączenie.

| Zespół | Nakład | Przełączenie |
| --- | --- | --- |
| 2 developerów platformy | ok. **36–45 tygodni** | tydzień **2027-09-13**, przy starcie M0 w dniu 2026-11-02 |
| 1 developer platformy | ok. **60–75 tygodni** | tydzień **2028-04-10** |

Repozytorium gry ma 16 zobowiązanych deliverable'i, GC1–GC16, łącznie ok. **44–66 tygodni pracy developera**. Potrzebują **2 developerów gry pracujących równolegle** od 2026-11-02 (sekcja „Roadmapa”, 11.2).

**Decyzje właściciela.** Plan buduje z wartością domyślną. Właściciel może ją zmienić do podanego terminu. Daty wynikają z kalendarza dla 2 developerów.

| Decyzja | Domyślna wartość, która jest budowana | Termin zmiany |
| --- | --- | --- |
| Publikacja sprite'ów na wiki, w narzędziach, w podglądzie stroju i **publikacja pikseli Atlasa** | **Publikujemy każdy sprite, który klient Oteryn już dostarcza graczom.** Każdy asset ma wpis w `licensing.json`. Istniejące atesty praw Atlasa (`docs/legal/DYN-ATLAS-001-*`) przechodzą do `docs/atlas/provenance/` w Platform-2. Assety bez rekordu atestu są wykluczane w czasie budowy. Pipeline potrafi wstrzymać pojedynczy asset bez dodatkowego kosztu. | Przed startem MA (**2027-03-15**) |
| Umiejscowienie Atlasa | **Kod Atlasa wchodzi do Platform-2** (sekcja „Oteryn-Atlas i wspólny silnik mapy”, X.1). Osobne wdrożenie jest wycofane i przekierowane przy przełączeniu. | Przed końcem M0 (2026-11-27) |
| ADR topologii ekosystemu | **Zaakceptowany.** Topologia repozytoriów należy do ADR 0001 w `Oteryn/Oteryn`, więc w M0 powstaje poprawka ADR w META: „środowisko uruchomieniowe i publikacja Atlasa przechodzą do Oteryn-Platform-2”. | Przed końcem M0 |
| Próg prywatności dla analityki farm/polowań | Metryka publiczna wymaga co najmniej 10 sesji i co najmniej 5 różnych kont w kohorcie. W przeciwnym razie metryka ma stan `SUPPRESSED`. | Przed startem MA |
| Maszyna referencyjna GPU | Molehill-PC, zarejestrowany jako efemeryczny runner self-hosted z etykietą `molehill-gpu`. Uruchamia jedno zadanie wydajnościowe przy każdej bramce kamienia milowego i w bramce przełączenia. | Przed startem MA |
| Dostawca płatności | **Stripe.** Payment Element z kartami, BLIK, Przelewy24, Apple Pay i Google Pay. Webhooki. Jedna integracja. | Przed startem M8 (**2027-06-14**) |
| Faktury | Platforma wystawia faktury PDF z polską numeracją ciągłą. **Faktury B2B są wysyłane do KSeF przez jego API.** Dane sprzedawcy pochodzą z konfiguracji. | Dane sprzedawcy najpóźniej na starcie M8 (2027-06-14) |
| Klucz odzyskiwania | Generowany przy włączeniu 2FA i pokazywany raz. Jeden aktywny klucz. Na przeglądzie konta przypomnienie, dopóki nie zostanie zapisany. | Przed końcem M1 (2026-12-28) |
| Okresy karencji | Usunięcie postaci **30 dni**. Usunięcie konta **30 dni**. Cooldown zmiany nazwy **30 dni**. Pierwsza zmiana nazwy za darmo, kolejne **250 coinów** (cena edytowalna w panelu sklepu). | Przed startem M3 (2027-01-11) |
| Sprawdzanie haseł w wyciekach | **Sprawdzenie HIBP (k-anonymity) jest włączone** przy rejestracji i zmianie hasła. | Przed końcem M1 |
| Drugi VPS | **Tak: `oteryn-vps-2`.** OCI Ampere A1 Flex, 4 OCPU / 24 GB, Ubuntu 26.04, ta sama VCN. Hostuje preprod v2, testowe węzły gry i monitoring. Plan OpenTofu dodaje **wolumen blokowy 60 GB na scratch buildów** (buildy Atlasa). Płatny zgodnie z cennikiem OCI. | Przed tygodniem 2 w M0 (2026-11-13) |
| Nazwy hostów | `v2.oteryn.com` (web preprod, Cloudflare Access), `login-v2.oteryn.com`, `game-v2.oteryn.com` (TCP 7272–7279), `cdn.oteryn.com`, `grafana.oteryn.com` (Cloudflare Access + OIDC). Atlas działa pod `oteryn.com/atlas` i `oteryn.com/pl/atlas` (preprod: `v2.oteryn.com/atlas`). Każda nazwa hosta obsługująca dziś osobny Atlas zwraca od przełączenia przekierowanie 301 na ten adres. | Przed tygodniem 2 w M0 |
| Jedyny zatwierdzający | Zasada czterech oczu jest wymuszona. Gdy istnieje tylko jeden posiadacz uprawnienia zatwierdzającego, system wymaga **samozatwierdzenia po 15 minutach chłodzenia** i wysyła e-mail do właściciela. | W dowolnym momencie |
| ADR gatewaya w Rust | **Zaakceptowany.** Pisany w M0 z pomiarami (patrz D5). | Właściciel architektury gry, do końca M0 (2026-11-27) |

## Decyzje

| # | Decyzja |
| --- | --- |
| D1 | **Osobne repozytorium `Oteryn-Platform-2`**, workspace Rust z polityką lintów gry. Trzy wspólne crate'y bez I/O żyją w Oteryn-Game: `oteryn-platform-wire`, `oteryn-catalog` i `oteryn-game-rules`. Platform 2 bierze je jako zależności `git` przypięte do tagu `contracts-vX.Y.Z`. **Kod środowiska uruchomieniowego i publikacji Oteryn-Atlas wchodzi do Platform-2 razem z historią git** (sekcja „Oteryn-Atlas i wspólny silnik mapy”). |
| D2 | Binarki: `oteryn-platform` (role `serve-public`, `serve-internal`, `worker`, `migrate`, `ops`), `oteryn-login-gateway`, `oteryn-node-agent` i `oteryn-node-supervisor`. Buildy publikacji Atlasa działają w roli `worker`. |
| D3 | Renderowanie: axum 0.8.9 + askama 0.16.1 SSR + htmx 4.0.0, obliczenia po stronie serwera w natywnym Ruście. **Jedynym JavaScriptem poza htmx jest silnik `oteryn-map`.** To moduł ES z WebGL2 z Atlasa, spakowany jako element własny `<oteryn-map>`. Renderuje stronę Atlasa, każdą mapę na wiki, każdą mapę w narzędziach i webowy widok GM observe. Bez bundlera, bez zależności npm w runtime i bez WASM. |
| D4 | `/internal/v1/*` zostaje bajt w bajt. Preprod v2 ma własne CA, certyfikaty, tokeny bearer i klucze admission. Przy przełączeniu produkcja ładuje **istniejące produkcyjne CA, certyfikat serwera, allowlistę DN, hashe bearerów i klucze Ed25519**. mTLS kończy się w procesie (rustls) zamiast w nginx. |
| D5 | **Rustowy `oteryn-login-gateway` zastępuje gateway w Go przy przełączeniu.** ADR-P2-0001 powstaje w M0 i spełnia wymóg „zmierzonego uzasadnienia” z ADR-0003: jeden język i toolchain, typy wire współdzielone przez `oteryn-platform-wire`, przekazywanie IP klienta do historii bezpieczeństwa oraz test obciążenia, w którym p99 jest nie gorsze niż p99 gatewaya w Go. Jest zgodny bajt w bajt z `/v1/login` gatewaya w Go (protocol_version 2, ALPN `oteryn-game/1`, profil grantu `oteryn-pre-admission-v1`, 11 kodów NATIVE_LOGIN_*). |
| D6 | **PostgreSQL 19** w każdym środowisku v2. M0 startuje na 19 RC1 (wydane 2026-10-15) i przechodzi na 19 GA, czyli aktualizacją w obrębie tej samej wersji głównej, gdy tylko GA wyjdzie. Platforma i gra mają osobne instancje. PG gry (17.11) należy do gry. |
| D7 | **Bez strangler.** Platform 2 powstaje kompletna, równolegle. Laravel działa bez zmian. **Jedno przełączenie** po bramce M11. |
| D8 | **Wycofane przy przełączeniu:** źródło danych Canary, IdentityCanaryAccount, bazar Canary, Player Companion (zastąpiony analizatorem sesji w narzędziach), szablony strony głównej, **osobne wdrożenie Atlasa na Synology** i **Leaflet**. |
| D9 | argon2id: istniejące hashe PHC weryfikowane bez zmian, rehash przy zmianie parametrów. Laravelowy APP_KEY jest kluczem legacy tylko do odczytu. |
| D10 | **Pierwsze demo dla właściciela, M2 (ok. tydzień 13 przy 2 developerach):** właściciel loguje się przez rustowy gateway v2, tworzy postać i gra na testowym węźle v2. |
| D11 | **Zarządzanie serwerami gry** przez agenta węzła w modelu pull, supervisor roota z zamkniętym zestawem czasowników, komendy podpisane przez platformę i obrazy podpisane przez wydanie, na wielu hostach (sekcja „Zarządzanie serwerami gry”). |
| D12 | **Jedno wydanie treści na wydanie gry.** To podpisany manifest wiążący paczkę katalogu (GC5), eksport świata Atlasa (GC16) i wektory reguł. Są importowane, weryfikowane, budowane i **aktywowane razem jedną transakcją**, a rollback też działa razem (sekcje „Wiki z katalogu gry” i „Oteryn-Atlas i wspólny silnik mapy”). |
| D13 | Formuły pochodzą ze wspólnego crate'a `oteryn-game-rules` i są sprawdzane przez `RULES_ABI` oraz złote wektory testowe. Wszystkie narzędzia dla gracza są dostarczone. |
| D14 | **Pełny panel konta jest dostarczony**, w tym passkeys, handel, bazar, domy i gildie. Tryb sudo (ponowne uwierzytelnienie) chroni wrażliwe akcje. |
| D15 | Toolchain: Rust 1.99.1. Runnery i builder na Ubuntu 26.04 (`ubuntu-26.04`, `ubuntu-26.04-arm`), nigdy `ubuntu-latest`. Obraz uruchomieniowy to **chiselled Ubuntu 26.04** z własnymi definicjami slice'ów w `deploy/chisel/` tam, gdzie brakuje ich w upstreamie. Workspace gry przechodzi na 1.99.1 i Ubuntu 26.04 w GC6. |
| D16 | **GM observe w dwóch trybach:** feed webowy i sesje obserwatora w kliencie. Każda sesja obserwacji jest nagrywana. Feed webowy renderuje się na `oteryn-map` z pełnym szczegółem Atlasa. |
| D17 | **Oteryn jest dostawcą OIDC** (discovery, JWKS, id_token, userinfo) i przechodzi test zgodności OpenID Basic OP. Pierwszym konsumentem jest logowanie staffu do Grafany. |
| D18 | **Infrastruktura jako kod:** OpenTofu zarządza instancjami OCI, grupami bezpieczeństwa sieci (zakres portów gry 7172–7199 otwierany raz na host gry), bucketami object storage, rekordami DNS i wolumenem scratch na buildy. |
| D19 | **Publikacje Atlasa są pochodnymi modelami do odczytu.** Ich jedynym wejściem są podpisane, bezpieczne publicznie eksporty gry (GC16 i GC5). Baza platformy nigdy nie jest wejściem publikacji ani jej indeksu wyszukiwania. Każdy transport i każde umiejscowienie GPU zachowuje `identityAuthority=false`. Nieznane pochodzenie, prawa, współrzędne lub tożsamość przerywają build i nigdy nie są zgadywane. |
| D20 | **Buduj raz, promuj po digeście.** Publikacje są budowane i weryfikowane na preprodzie (oteryn-vps-2). Produkcja dostaje te same obiekty adresowane treścią przez kopię po stronie serwera w object storage. Produkcja nigdy nie buduje. |
| D21 | **Dyscyplina weryfikacji Atlasa obowiązuje na każdej ścieżce mapy** (`map-engine`, `atlas-publish`, `atlas-oracle`, `e2e`): <ul><li>przeglądarkowe scenariusze Playwright;</li><li>zrecenzowane dowody wizualne z digestami zrzutów, recenzentem i dokładną rewizją;</li><li>zero zaakceptowanych ponowień;</li><li>niezależny oracle w Pythonie;</li><li>obowiązkowa Merge Queue.</li></ul> |

## Stan obecny

Z grą działa natywnie tylko kilka ścieżek: tworzenie i lista postaci, logowanie do gry, premium i status światów. Wszystko, co pokazuje dane z gry na stronie, czyta Canary albo pokazuje „niedostępne”.

| Obszar | Laravel dziś | Stan natywny |
| --- | --- | --- |
| Highscores, profil, zgony, online, gildie | Połączenie `canary`; przy `native` pokazuje „niedostępne” | Ingest w Laravelu jest, **producenta w grze nie ma** (dostarcza go GC1) |
| Bazar / marketplace | `CanaryCharacterTransfer` | Gra wykonuje tylko `CreateCharacter`; Transfer zwraca `CHAR_CMD_UNSUPPORTED` (GC2) |
| Overview konta | `IdentityCanaryAccount` (Canary) | Brak |
| Tworzenie postaci | Komenda `CreateCharacter`, gra pobiera `/character-commands/pending` | **Działa** (wysyłane są tylko nazwa i świat) |
| Lista postaci | Projekcja LCFA `/native-account-characters` | **Działa** |
| Logowanie do gry | OAuth PKCE → ticket → gateway Go → `/native-admissions` | **Działa** |
| Premium | Ledger czasu → `/premium-snapshots/read` | **Działa** |
| Status światów | LiveOps z natywnego read modelu | **Działa** |
| Gildie | Brak | Gra nie ma tabel gildii (dostarcza je GUILD-0 w GC12) |
| Moderacja (ban/mute/kick/broadcast) | Tylko rekordy webowe | W grze brak (dostarcza GC4); ban konta tylko przez evidence (`disabled_at` + generation) |
| Płatności | Tylko testowy provider | Brak prawdziwego (Stripe w M8) |
| Atlas | Osobne wdrożenie na Synology (runner `oteryn-synology-atlas`, etykieta `oteryn-atlas`) | Wchodzi do Platform-2 w MA; wdrożenie Synology jest wycofane w T+14 d |

## Repozytorium

**Osobne repo `Oteryn-Platform-2`.** Rust workspace z edycją 2024 i rust-version 1.99. Lint jak w grze: `unsafe_code = forbid`; clippy all, unwrap, expect i panic na poziomie `deny`.

Dlaczego osobne repo:

- Zyski: niezależny rytm wydań, CI bez buildów gry i kod webowy niezwiązany regułami granic oraz polityką nazw gry.
- Koszt: rozjazd kontraktu nie jest już wyłapywany w czasie kompilacji wewnątrz jednego workspace'u. Lukę zamykają otagowane crate'y kontraktowe oraz zestaw zgodności przeciw przypiętemu obrazowi gry.

**Wspólne crate'y w Oteryn-Game** (powstają w GC6 i GC8), wszystkie bez I/O:

| Crate | Zawartość |
| --- | --- |
| `crates/platform-wire` | Ścieżki, limity, kody błędów i typy wire. Wyciągnięte z `native_admission_source/*`, `premium/client.rs` i `admission_evidence.rs`. |
| `crates/catalog` | Pakiet `oteryn-catalog`: koperta katalogu v1/v2, kanoniczny JSON, digest i walidacja. |
| `crates/game-rules` | Pakiet `oteryn-game-rules`: czyste formuły na liczbach całkowitych plus `RULES_ABI`. |

- Każdy crate dostaje `[package.metadata.oteryn]` i jawne `[edges]` w `workspace-boundaries.toml`. `oteryn-game-server` dostaje krawędzie do wszystkich trzech. Żadna nazwa nie zawiera zakazanego fragmentu.
- **Reguła zależności wspólnych crate'ów.** Zależą tylko od `serde`, `serde_json`, `sha2`, `ed25519-dalek` oraz `foundation` / `error-codes` z repo. Nie zależą od tokio, sqlx ani reqwest. Powód: `[patch.crates-io]` gry nie działa dla konsumentów git, a przechodnia zależność po cichu podmieniłaby załatane crate'y na upstream.
- **Wendorowane łatki.** W GC6 gra przenosi swoją łatkę tokio na 1.53.3. Platform-2 wendoruje **te same łatki tokio i sqlx-core/sqlx-postgres** we własnym `[patch.crates-io]`, więc oba produkty uruchamiają identyczny kod runtime i sterownika bazy.
- **Dostęp CI.** Token GitHub App z odczytem Oteryn-Game rozwiązuje zależności git.
- **Zestaw zgodności** (`tests/conformance`). Stos Compose uruchamia obraz gry przypięty po digeście, jego własny PG gry, Platform-2 i rustowy gateway logowania. Przechodzi czarne skrzynki dla:
  - admission i evidence;
  - LCFA (projekcja postaci konta);
  - komend postaci v2;
  - premium;
  - projekcji publicznych i szczegółów postaci;
  - komend operatora;
  - dostarczania sklepu oraz komend domów i gildii;
  - observe;
  - agenta węzła;
  - złotych wektorów wire i wektorów reguł.
- Przed przełączeniem zestaw zgodności przechodzi jeszcze raz przeciw dokładnemu digestowi obrazu gry z produkcji (GC15).

Zmiany w Oteryn-Game: trzy nowe crate'y i krawędzie `oteryn-game-server` → każdy z nich. W GC6 `rust-toolchain.toml` (dziś 1.94.0) i CI (dziś `+1.95.0`, `ubuntu-24.04`) przechodzą na 1.99.1 i `ubuntu-26.04`.

## Wersje

| Komponent | Wersja, z którą budujemy | Weryfikacja |
| --- | --- | --- |
| Rust | 1.99.1 (2026-10-15), MSRV 1.99 | blog.rust-lang.org/releases; Docker Hub `rust:1.99.1-*` |
| Runnery | `ubuntu-26.04`, `ubuntu-26.04-arm` (GA 2026-09-17). `ubuntu-latest` przechodzi na 26.04 między 19.10 a 19.11.2026, więc etykiety są przypięte jawnie w obu repo. | changelog GitHuba z 2026-09-17 |
| Builder / system hosta | Ubuntu 26.04 LTS; Docker `ubuntu:26.04` + rustup 1.99.1 | Tag `ubuntu:26.04` jest na Docker Hub |
| Obraz uruchomieniowy | Chiselled Ubuntu 26.04: slice'y `base-files`, `ca-certificates`, `libc6`, `libgcc-s1`; uid 65532. Własne definicje slice'ów w `deploy/chisel/` tam, gdzie brakuje ich w upstreamie. | Zbudowany i przeskanowany w M0 |
| PostgreSQL | 19 (RC1 2026-10-15, potem GA). PG gry to 17.11 i należy do gry. | postgresql.org |
| axum / tower-http | 0.8.9 / 0.7.1 | crates.io |
| tokio | 1.53.3, z łatką wspólną z grą | crates.io; GC6 |
| sqlx | 0.9.0, z łatką wspólną z grą | crates.io |
| rustls / reqwest | 0.23.46 (aws-lc-rs) / 0.13.5 | crates.io |
| askama / htmx | 0.16.1 / 4.0.0 (wendorowany, hash SRI) | crates.io; four.htmx.org |
| argon2 / totp-rs / lettre / fluent-templates | 0.6.0 / 6.0.0 / 0.11.23 / 0.15.1 | crates.io. Duplikaty crate'ów krypto (argon2 0.6 obok ed25519-dalek 2.2) są dopuszczone w `cargo deny`. |
| WebAuthn | `webauthn-rs` | Przypięty w Cargo.lock w M1 |
| Edge Caddy | 2.11.7, przyjęty przy przełączeniu | Docker Hub |
| Prometheus, Alertmanager, Grafana OSS, OpenTofu | Najnowsza stabilna w M0 | Przypięte digestem obrazu lub lock file w M0 |
| Playwright | 1.62.0, ta sama wersja, którą Atlas przypina w `e2e/package.json` | Lock file |

htmx 4 różni się od 2.x: API fetch, dziedziczenie atrybutów tylko jawne, zmienione nazwy zdarzeń. Platforma jest budowana od zera, więc używa od razu 4.x. Plik jest wendorowany z hashem SRI. Leaflet nie należy do stosu.

Źródła:

- [changelog GitHuba: Ubuntu 26 GA i migracja latest](https://github.blog/changelog/2026-09-17-ubuntu-26-generally-available-and-latest-migration)
- [ogłoszenia wydań Rusta](https://blog.rust-lang.org/releases/)
- [strona główna PostgreSQL](https://www.postgresql.org/)
- [ogłoszenie htmx 4.0.0](https://four.htmx.org/announcements/2026-08-28-htmx-4.0.0-is-released)
- API crates.io (`https://crates.io/api/v1/crates/<nazwa>`)
- tagi Docker Hub: [ubuntu](https://hub.docker.com/_/ubuntu), [postgres](https://hub.docker.com/_/postgres), [rust](https://hub.docker.com/_/rust), [caddy](https://hub.docker.com/_/caddy)

## Architektura

### Hosty i kontenery

```
oteryn-vps (host prod, Ubuntu 26.04)                  oteryn-vps-2 (preprod + monitoring + buildy)
 caddy edge :443 (2.11.7 od przełączenia)              caddy :443 (v2.oteryn.com, login-v2, grafana)
  oteryn.com (z /atlas) -> p2-public:8000               p2-public / p2-internal / p2-worker / p2-pg19
  login./gateway.   -> login-gateway (Rust):8080        p2-worker: buildy wydań treści (scratch 60 GB)
  dawne hosty Atlasa -> 301 oteryn.com/atlas            login-gateway (preprod)
 p2-internal 172.18.0.12:8543 (rustls mTLS)             testowe węzły gry ch-A, ch-B + testowy PG gry
 p2-worker (promocja, bez buildów), p2-pg19             node-agent + node-supervisor
 węzły gry + PG gry (osobny)                            prometheus + alertmanager + grafana (OIDC)
 node-agent + node-supervisor                           runnery self-hosted: deploy, e2e real_fullworld
 testowy kanał v2 ch-C
 cdn.oteryn.com -> Cloudflare -> OCI Object Storage: assety katalogu, publikacje Atlasa, crops, rendery strojów
                                                        (nagrania observe nigdy nie trafiają do CDN)
Synology (do T+14 d): osobny Atlas, bez trasy od T0+30
Molehill-PC: efemeryczny runner `molehill-gpu` (tylko bramka wydajności GPU)
```

- **Multi-host działa naprawdę już na preprodzie.** Świat testowy ma kanały na obu hostach: ch-A i ch-B na vps-2, ch-C na oteryn-vps.
- **Dark prod.** Od M5 stos `oteryn-v2-prod` działa **dark** na oteryn-vps, we własnej sieci i bez publicznej trasy. Każdej nocy wykonuje próbę importu ze snapshotu PG Laravela przez rolę tylko do odczytu, a także przyjmuje promowane wydania treści.

### Komunikacja

Platforma nigdy nie łączy się do hostów gry. Każdy wiersz poniżej to wywołanie wychodzące z gry, gatewaya, agenta, workera lub przeglądarki.

| Przepływ | Ścieżka / mechanizm |
| --- | --- |
| Admission | Rustowy gateway → p2-internal `/internal/v1/game-auth/native-admissions` (bearer, hash current + previous), IP klienta przekazywane dalej |
| Evidence, runtime status, raporty i unieważnienia scope, LCFA, intencje bootstrap, snapshot premium, powiadomienie o restore | Istniejące rodziny `/internal/v1`, zgodne bajt w bajt |
| Komendy postaci v2, dostarczanie sklepu, komendy domów i gildii | Gra pobiera `/internal/v1/game-auth/character-commands/*` (nowe pary komenda/wariant), `/internal/v1/store-delivery/*`, `/internal/v1/house-commands/*`, `/internal/v1/guild-commands/*` |
| Projekcje publiczne | Gra pcha `/internal/v1/game-public/*` |
| Szczegóły i postęp postaci (tylko właściciel) | Gra pcha `/internal/v1/game-account/character-details` (+ `/watermark`) |
| Komendy operatora, w tym start/koniec observe | Gra pobiera `/internal/v1/operator-commands/{pending,receipt}` |
| Feed observe | Gra pcha `/internal/v1/gm-observe/frames`, dedykowany certyfikat i pula połączeń. Przeglądarka staffu dostaje go przez SSE z p2-public `/admin/observe/{id}/stream`. |
| Agent węzła | Long-poll `/internal/v1/node-agent/commands/pending` plus heartbeat, potwierdzenia, logi i CSR |
| Wydania treści | p2-worker (preprod) pobiera z ghcr.io `oteryn-catalog:<release>`, `oteryn-atlas-world:<release>`, manifest wydania treści i cotygodniowe `oteryn-atlas-farm:<rrrr-tt>` |
| Object storage | Worker preprodu wgrywa obiekty. Worker produkcji robi kopię po stronie serwera (promocja), potem ponownie weryfikuje digesty. |
| Dane Atlasa do przeglądarek | Przeglądarka → `cdn.oteryn.com` GET z Range; wskaźnik z p2-public `/atlas/api/current` |
| Płatności | Webhooki Stripe → p2-public `/webhooks/stripe` (podpis weryfikowany); worker → API Stripe |
| KSeF | worker → API KSeF (faktury B2B) |

**Szczegóły ścieżek `/internal/v1`.** Te ścieżki działają identycznie jak dziś. Zestaw zgodności sprawdza każdą.

| Ścieżka `/internal/v1/...` | Wywołujący | Cel |
| --- | --- | --- |
| `game-auth/native-admissions` | rustowy gateway (bearer, hash SHA-256 current + previous) | Decyzja o wpuszczeniu dla ticketu |
| `game-auth/native-evidence` | gra | ReadAccountSecurityV1, ReadFreshSigningTrustV1, ReadRecovery…V2 |
| `game-auth/native-runtime-status` | gra | Raport stanu runtime |
| `game-auth/native-scope-assignments` | gra | Raport przypisań scope |
| `game-auth/native-scope-revocations` | gra | Zaimplementowane i objęte testem zgodności w M2 |
| `game-auth/native-account-characters` (+ `/watermark`) | gra | Snapshot LCFA (≤64 postacie, ≤16 KB); watermark co 3 s, maksymalna przerwa 10 s |
| `game-auth/character-bootstrap-intents/read` | gra | Odczyt intencji bootstrap |
| `game-auth/character-commands/{pending,read,receipt}` | gra | Kolejka komend, polling co 1 s, strona 32 |
| `products-entitlements/premium-snapshots/read` | gra | Snapshot premium |
| `game-auth/game-restore/notice` | gra | Powiadomienie o restore |
| `game-auth/character-commands/*` (nowe pary, GC2) | gra | Zmiana nazwy, usunięcie z karencją i anulowanie, widoczność, tworzenie v2, transfer właściciela i świata |
| `game-account/character-details` (+ `/watermark`) (GC3) | gra | Projekcja szczegółów postaci, tylko dla właściciela |
| `operator-commands/{pending,receipt}` (GC4) | gra | Broadcast, kick, mute, namelock, SaveAndShutdown, StartObserve/EndObserve |
| `store-delivery/*`, `house-commands/*`, `guild-commands/*` (GC13, GC14, GC12) | gra | Dostarczanie przedmiotów ze sklepu, aukcje i transfery domów, zarządzanie gildiami |
| `node-agent/{commands/pending,heartbeat,receipts,logs,csr}` | node-agent | Zarządzanie serwerami gry (sekcja „Zarządzanie serwerami gry”) |
| `gm-observe/frames` | gra (osobny certyfikat) | Zsanityzowane klatki podglądu sesji gracza przez GM |
| `game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}` | gra (GC1) | Public Projections v1 |

Limity klienta gry: connect 1 s, handshake 2 s, wymiana 3 s, 2 sloty pipeline. p99 platformy na tych trasach musi być wyraźnie poniżej 1 s. Każdy cel ma własny certyfikat klienta (np. `CN=oteryn-preprod-character-projection`). Tożsamość rozmówcy to DN certyfikatu plus pin SPKI, mapowane z allowlisty na enum `Peer`. Każda trasa deklaruje, które `Peer` przyjmuje.

Publiczne API dla launchera i klienta (`crates/platform-client`): `/oauth/authorize`, `/oauth/token` (PKCE S256, scope `game:ticket`), `POST /v1/game-auth/tickets`, `/v1/game-auth/device-sessions/*`, `GET /v1/game-auth/native-characters`, `GET /v1/client/directory`.

### Własność danych

Postacie i stan gry należą do serwera gry; platforma trzyma ich projekcje i wydaje komendy. Konta, logowanie, płatności i treści należą do platformy.

| Dane | Właściciel | Co ma platforma |
| --- | --- | --- |
| Tożsamości, poświadczenia, 2FA, passkeys, sesje, OAuth/OIDC, sesje urządzeń, generation bezpieczeństwa, floory evidence, klucze admission | Platforma | Źródło prawdy |
| Światy, kanały, trasy, topologia, directory, hosty, agenci, węzły, wydania, wdrożenia | Platforma | Źródło prawdy |
| Postacie, nazwy, progres, itemy, bank, domy (własność), gildie (członkostwo), party | Gra | Projekcje + intencje komend i potwierdzenia |
| Portfel coinów, zamówienia, płatności, faktury, ledger premium, escrow bazaru, oferty na aukcje domów | Platforma | Ledgery podwójnego zapisu |
| Snapshoty katalogu, sprite'y, wektory reguł, **eksport świata Atlasa, agregaty farm** | Gra (per wydanie / per tydzień) | Niezmienne kopie |
| **Publikacje Atlasa**: semantyka, piksele, buckety, overview, indeks, wyszukiwanie, crops | Pochodne budowane przez platformę wyłącznie z eksportów gry | Niezmienny pochodny model do odczytu, `identityAuthority=false`, nigdy autorytet |
| Nakładki mapy | Właściciel źródła (katalog: gra; nakładki właściciela: narzędzia platformy; nakładki staffu: observe) | Nigdy nie wpisywane do publikacji |
| Opisy wiki, tłumaczenia, CMS, support, sankcje, audyt, rekordy observe, dane narzędzi | Platforma | Źródło prawdy |
| Logi gry | Gra emituje | Kopia trzymana 14 dni na produkcji i 7 dni na preprodzie |

Bez kluczy obcych między bazami; referencje do gry to nieprzezroczyste typowane ID (`CharacterRef`, `WorldRef`).

### Reguły spójności

1. **Monotoniczne watermarki.** Każdy wiersz projekcji ma `(source_authority, epoch, revision)`; zapis tylko jeśli nowszy, cofnięcia odrzucane. UI pokazuje „nieaktualne od” po przerwie > 10 s.
2. **Idempotentne komendy.** `command_id` = UUIDv7 jako klucz główny; potwierdzenia zapisywane raz. UI pokazuje „tworzenie…” do potwierdzenia.
3. **Transakcyjny outbox.** Każda zmiana, która ma dotrzeć do gry, zapisuje się w tej samej transakcji co wiersz biznesowy. Bez podwójnych zapisów.
4. **Liczniki nigdy wstecz** (generation, floors, witness): `CHECK` + `UPDATE … WHERE new > old`. Import i import odwrotny przenoszą te wartości.
5. **Ledger** tylko dopisywany, podwójny zapis, suma = 0 na transakcję (deferred constraint trigger). Korekty to storna, nigdy edycja.
6. **Rezerwacje nazw** należą do gry; platforma robi tylko wstępne sprawdzenie, decyduje potwierdzenie gry.

### Bezpieczeństwo

| Obszar | Rozwiązanie |
| --- | --- |
| Hasła | argon2id (m=19456, t=2, p=1). Stare hashe weryfikowane, rehash przy logowaniu. Sprawdzenie HIBP (k-anonymity) przy ustawieniu i zmianie. |
| 2FA | TOTP (migrowane sekrety są szyfrowane na nowo key ringiem XChaCha20-Poly1305) plus kody odzyskiwania zapisane jako hashe argon2id. **Passkeys (WebAuthn)** jako pierwszy i jako drugi czynnik. |
| Stare keyed hashe | `IdentitySecret::keyedHash` (HMAC-SHA256 z APP_KEY) jako `LegacyKeyedHash` dla istniejących sesji urządzeń; nowe wiersze z nowym kluczem i kolumną wersji. |
| Sesje | Po stronie serwera w PG. Ciasteczko `__Host-`, Secure, HttpOnly, SameSite=Lax. ID sesji rotowane przy logowaniu i zmianie uprawnień. |
| Tryb sudo | Ponowne sprawdzenie hasła, TOTP lub passkeya, ważne 10 minut, związane z sesją. Wymagane dla każdej wrażliwej akcji z sekcji „Panel konta i postaci”, „Zarządzanie serwerami gry” i „Podgląd sesji gracza przez GM”. |
| CSRF | Token synchronizera na każdym żądaniu nie-GET (htmx wysyła go w nagłówku) plus sprawdzenie Origin. |
| Rate limity | Kubełki per IP (zaufany łańcuch Cloudflare/Caddy) i per konto: logowanie, rejestracja, reset, 2FA, ticket, oauth/token, webhooki i obliczenia narzędzi. Eskalacja do blokady z wpisem audytu. |
| OAuth / OIDC | Tylko authorization code + PKCE S256. Tokeny nieprzezroczyste, hashowane w spoczynku. id_token OIDC podpisany EdDSA, z discovery i rotacją JWKS. |
| mTLS | rustls, TLS 1.3, certyfikaty klienta o określonym przeznaczeniu, allowlista DN + SPKI, przeładowanie na gorąco (SIGHUP). |
| Sekrety | Pliki w `/run/secrets`, tryb 0400, uprawnienia sprawdzane przy ładowaniu (loader odrzuca pliki czytelne dla grupy lub wszystkich). Nigdy zmienne środowiskowe. |
| Audyt | Tylko dopisywany, łańcuch hashy dzienny, head eksportowany do logu. Wpis zawiera aktora, peer, akcję, cel, digest przed/po, request ID i IP. |
| Podpisy komend | Komendy węzła podpisuje klucz ops Ed25519 (osobny od klucza admission); wdrożenia obrazów wymagają dodatkowo podpisu wydania z CI gry. |
| Nagłówki | Restrykcyjne CSP (tylko self i `cdn.oteryn.com`, bez inline), HSTS, frame-ancestors none, Referrer-Policy. |
| Łańcuch dostaw | cargo-deny, cargo-audit, SBOM, keyless podpis sigstore na każdym obrazie. Wydania gry dodatkowo podpisane kluczem wydania Ed25519. |

### i18n

Zostają adresy `/{locale}/...` z `en` i `pl`; `/` wybiera język z Accept-Language, potem ciasteczka, domyślnie `en`. Tłumaczenia w Fluent (`locales/{en,pl}/*.ftl`), co jest ważne dla polskich form mnogich. `lang/en`, `lang/pl` i `pl.json` konwertuje jednorazowy skrypt. CI odrzuca brakujące klucze w którymkolwiek języku. Atlas ma wersje `/atlas` (EN) i `/pl/atlas` (PL) z parą `hreflang`.

## Moduły produktu

| Moduł | Kamień milowy | Deliverable gry |
| --- | --- | --- |
| Tożsamość, OAuth, dostawca OIDC, rdzeń bezpieczeństwa konta, passkeys | M1 | — |
| Parytet kontraktu wewnętrznego, rustowy gateway logowania, tworzenie i lista postaci, premium, rejestr | M2 | GC6 |
| Panel konta i postaci, publiczne strony, support, sankcje, odwołania, GDPR | M3 | GC1, GC2, GC3 |
| CMS, treści, pobieranie, launcher/updater, admin RBAC, audyt, moderacja GM (broadcast, kick, mute, namelock) | M4 | GC4 |
| Zarządzanie serwerami gry, multi-host, dark prod | M5 | GC4, GC7 |
| **Atlas + silnik `oteryn-map` + pipeline wydań treści + przygotowanie wycofania osobnego Atlasa** | **MA** | **GC16**, GC5 |
| Katalog, wiki (mapy na `oteryn-map`), podgląd stroju | M6 | GC5, GC8 |
| Narzędzia dla gracza, wszystkie mapy na `oteryn-map`; miejsca polowań połączone z eksploratorem farm Atlasa | M7 | GC8, GC9, GC11, GC16 (farmy) |
| Handel: Stripe, coiny, premium, sklep z dostarczaniem przedmiotów, prezenty, faktury z KSeF | M8 | GC13 |
| Bazar, transfer świata, domy i aukcje, zarządzanie gildiami z weba | M9 | GC2, GC12, GC14 |
| GM observe, webowy widok na `oteryn-map` i w kliencie | M10 | GC10 |
| Hardening, przeglądy, wydajność obciążeniowa i GPU, próby, UAT | M11 | GC15 (przypięte wydanie prod) |

## Struktura katalogów

### `Oteryn-Platform-2`

```
Oteryn-Platform-2/
├── Cargo.toml  rust-toolchain.toml (1.99.1)  deny.toml  vendor/{tokio-1.53.3,sqlx-core-0.9.0,sqlx-postgres-0.9.0}
├── .github/workflows/{ci.yml, e2e.yml, atlas-fullworld.yml, atlas-gpu-perf.yml, image.yml,
│                      deploy-preprod.yml, deploy-dark-prod.yml, deploy-prod.yml}
├── apps/
│   ├── platform/src/{main.rs, config.rs, roles/*.rs, ops/{game_auth,catalog,identity,mail,support,wiki,nodes,commerce}.rs}
│   ├── login-gateway/src/{main.rs, login.rs, admission_client.rs, alpn.rs, errors.rs}
│   ├── node-agent/src/{main.rs, enroll.rs, poll.rs, heartbeat.rs, logs.rs, redact.rs, supervisor_client.rs}
│   └── node-supervisor/src/{main.rs, socket.rs, verbs.rs, verify.rs, docker.rs, nodespec.rs, incarnation.rs, policy.rs}
├── crates/
│   ├── domain/src/{identity,auth,oidc,characters,registry,premium,ledger,commerce,bazaar,houses,guilds,
│   │               content,content_release,catalog,atlas,wiki,support,rbac,sanctions,ops,tools,account,gm}/
│   ├── db/{migrations/, src/}          # repozytoria, outbox, joby, zarządzanie partycjami
│   ├── security/src/{passwords,totp,webauthn,aead,laravel_legacy,sudo,csrf,sessions,keys,observe_grant}.rs
│   ├── internal-api/src/{tls.rs, peer.rs, routes/<family>.rs}
│   ├── node-wire/      catalog-import/    assets/ (object store, kompozytor strojów)
│   ├── payments/src/{stripe.rs, webhooks.rs, reconcile.rs, invoices.rs, ksef.rs}
│   ├── atlas-publish/src/{handoff.rs, semantic.rs, pixels.rs, buckets.rs, runtime_index.rs, overview.rs,
│   │                      search_index.rs, creature_shards.rs, farm.rs, crops.rs, content_graph.rs,
│   │                      crosscheck.rs, manifest.rs, verify.rs, upload.rs, promote.rs}
│   ├── tools/src/{bestiary,charms,exp,stamina,skills,imbuements,forge,damage,loot_split,session_parse,overlays}.rs
│   ├── web/{src/, templates/{layout,public,account,characters,game,wiki,tools,shop,bazaar,houses,guilds,
│   │        support,admin,ops,observe,atlas}/,
│   │        static/{css, js/htmx-4.0.0.min.js,
│   │                map-engine/{core/ (z Atlas src/browser), layers/ (src/layers), hunt/ (src/hunt-intelligence),
│   │                            apps/{atlas,mobile,search,farm,embed,tools,observe,replay}.mjs,
│   │                            element.mjs, overlays/{public,owner,staff}.mjs, workers/semantic-decode.mjs,
│   │                            map-engine.css}},
│   │        locales/{en,pl}/}
│   └── worker/src/jobs/{mail,outbox,retention,terminations,deletions,exports,content_release_pull,catalog_import,
│                        log_partitions,projection_alarms,payments_reconcile,invoices,ksef_submit,auctions,
│                        recording_retention,observe_oversight,atlas_build,atlas_verify,atlas_upload,atlas_promote,
│                        content_release_activate,atlas_gc,farm_pull}.rs
├── tools/
│   ├── legacy-import/src/{main.rs, laravel_crypt.rs, map/*.rs, verify.rs, reverse.rs}
│   └── atlas-oracle/{verify_publication.py, negative_tests.py, verify_overview.py, verify_authority_registry.py,
│                     verify_handoff.py, publication.py (kompilator do testu równoważności), README.md}   # tylko CI, Python stdlib
├── e2e/{playwright.config.mjs (retries: 0), package.json (@playwright/test 1.62.0), tests/{atlas,wiki-map,tools-map,
│        observe-replay,mobile}/, baselines/, fixtures/qualification/, support/}
├── docs/atlas/{PUBLICATION_V1.md, RUNTIME_V1.md, VERIFICATION.md, PARITY_CHECKLIST.md, provenance/}
├── tests/conformance/
└── deploy/
    ├── tofu/{oci_instances.tf, block_volumes.tf, nsg.tf, object_storage.tf, dns.tf (z hostami Atlasa)}
    ├── chisel/                         # definicje slice'ów ubuntu-26.04 tam, gdzie brakuje ich w upstreamie
    ├── compose/{base,v2-preprod,v2-testgame-a,v2-testgame-c,v2-prod,monitoring}.yml
    ├── caddy/{vps2.caddy, cutover.caddy, atlas-redirects.caddy}
    ├── systemd/{oteryn-node-agent.service, oteryn-node-supervisor.service, oteryn-watchdog.service}
    └── scripts/{deploy.sh, backup.sh, restore-drill.sh, dark-import.sh, cutover.sh, rollback.sh}
```

### Dodatki w Oteryn-Game

```
crates/{platform-wire, catalog, game-rules}/
tools/catalog-export/                    # eksporter katalogu w Rust; producer.py zostaje jako kontrola krzyżowa w CI
tools/atlas-world-export/                # Rust; natywny content/world (.b3) + appearances 15.30 -> paczka świata Atlasa
tools/game-atlas-*/                      # istniejące producenty przepięte na natywne wejścia; fixture'y jako testy złote
apps/game-server/src/{observe/, store_delivery/, house_commands/, guild/, account_details/, public_projection/,
                      operator/, telemetry/farm_aggregate.rs}
crates/protocol-oteryn/src/observe.rs    # capability OBSERVE_V1
apps/client/                             # HUD obserwatora; eksport sesji (analizator)
docs/contracts/{..._CATALOG_EXPORT_V2, _CATALOG_BUNDLE_V1, _CHARACTER_AUTHORITY_COMMANDS_V2, _OPERATOR_COMMANDS_V1,
                _ACCOUNT_CHARACTER_DETAILS_V1, _GM_OBSERVE_V1, _STORE_DELIVERY_V1, _HOUSE_COMMANDS_V1, _GUILDS_V1 (accepted),
                _CLIENT_SESSION_EXPORT_V1, OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1 (ACCEPTED),
                OTERYN_GAME_ATLAS_WORLD_BUNDLE_V1, OTERYN_CONTENT_RELEASE_MANIFEST_V1}.md
.github/workflows/release-content.yml    # katalog + atlas-world + manifest; cotygodniowy job farm
```

## Zarządzanie serwerami gry (M5)

### Model

- `oteryn-node-agent` działa jako użytkownik `oteryn-agent`. Nie ma sudo, nie należy do grupy docker i wykonuje tylko wychodzące połączenia mTLS. Należy do grupy `systemd-journal`, żeby czytać logi.
- `oteryn-node-supervisor` działa jako root, bez sieci. Jedynym interfejsem jest gniazdo Unix, tryb 0660, `root:oteryn-agent`, ze sprawdzeniem SO_PEERCRED. Przenosi `game-supervisor.py` jeden do jednego: trwała autoryzacja ops per inkarnacja, brak ponownego użycia zużytych plików startowych, zatrzymanie `reconciliation-required` i `restart: 'no'`.
- **Koperty komend** niosą `command_id`, `host_id`, `node_id`, czasownik, typowane argumenty, monotoniczny per host `seq` oraz `not_before` / `expires_at` (najwyżej 120 s). Platforma podpisuje je kluczem ops Ed25519. Supervisor weryfikuje przypięty klucz, `seq`, TTL i idempotencję.
- **Wdrożenia** dodatkowo wymagają podpisu wydania gry nad digestem obrazu, SHA gita, digestem paczki, `rules_abi`, zakresem migracji i `rollback_safe`.

### Dozwolone akcje (zamknięta lista; nic innego nie istnieje)

| Czasownik | Efekt | Strażnik |
| --- | --- | --- |
| `status` | Stan unitu, kontenera i obrazu; CPU, RAM i dysk hosta | Tylko odczyt |
| `start` | Nowa inkarnacja przez dostarczony przepływ `oteryn-game-ops` | Typowany node_id |
| `stop` | SIGTERM z 60 s zwłoki. Gra wykonuje pełny zapis (kryterium akceptacji GC4). | — |
| `restart` | stop, potem start | Najwyżej 6 na węzeł na godzinę |
| `deploy` | Pobranie obrazu po digeście, uruchomienie `oteryn-game-migrate` z tego obrazu, przełączenie, start | Digest obecny w manifeście podpisanym przez wydanie; jedno wdrożenie naraz na host |
| `apply-nodespec` | Renderowanie `node.toml` z szablonu należącego do supervisora | UUID-y; port w 7172–7199; kanały obecne w podpisanej generacji topologii; stałe ścieżki |
| `provision-node` | Katalog węzła (0700, uid 1001) i usługa compose ze stałego szablonu | Maksymalna liczba węzłów na host |
| `install-cert` | Certyfikat dla klucza wygenerowanego na hoście, podpisany przez pośrednie CA platformy | Wzorzec CN `oteryn-<env>-<purpose>-<node>` |
| `clear-reconciliation` | Usunięcie flagi po ludzkim przeglądzie | Cztery oczy |

**Nigdy niemożliwe:** dostęp do shella, dowolne argv, montowania i uprzywilejowane kontenery, sieć hosta, pliki poza katalogiem głównym węzła, inne unity systemd, zmiany pakietów i firewalla, reboot, odczyt sekretów, dostęp do PG gry poza binarką migrate z podpisanego obrazu.

**Sieć.** OpenTofu otwiera TCP 7172–7199 w grupie bezpieczeństwa sieci każdego hosta gry jeden raz (D18). Nie ma ręcznego kroku per kanał.

**Logi.**

- Kontenery gry używają sterownika logowania journald.
- Agent czyta `journalctl -o json -f --after-cursor`. Zaczernia tokeny bearer, 43-znakowe seedy base64url i linie Authorization. Wysyła paczki zstd do 256 KB i trzyma lokalny bufor 200 MB.
- Składowanie: `ops.node_log_lines`, partycje dzienne, indeks BRIN na `ts`, trigramowy indeks GIN na `message`, retencja przez usuwanie partycji.
- UI oferuje podgląd na żywo przez SSE (LISTEN/NOTIFY) i wyszukiwanie po czasie, poziomie, węźle i tekście. Eksport wymaga `ops.logs.export` i jest audytowany.

### Funkcje i przepływy

- **Widok serwerów.** Światy → kanały → węzły → hosty. Pokazuje stan oczekiwany i rzeczywisty, wydanie, status runtime i przypisania scope, heartbeat, liczbę online i flagę reconciliation.
- **Maintenance.** Przestawienie polityki trasy; gateway odpowiada wtedy ROUTE_UNAVAILABLE.
- **Odliczanie, zapis i wyłączenie.**
  1. Zaplanowanie.
  2. `Broadcast` operatora w T-15, T-5 i T-1 minut.
  3. Maintenance trasy w T-0.
  4. `SaveAndShutdown` (gra kończy z kodem 0).
  5. Agent zgłasza planowane zatrzymanie, więc supervisor nie restartuje.
  6. Następny czasownik się wykonuje.
- **Wdrożenie.**
  1. Wybór wydania.
  2. Odliczanie.
  3. `deploy`.
  4. Bramka zdrowia w 60 s: runtime status plus watermark LCFA nie starszy niż 10 s.
  5. **Pasujące wydanie treści (katalog i Atlas) aktywuje się automatycznie.**
  6. Rollback do poprzedniego digestu jest dozwolony tylko dla wydań oznaczonych `rollback_safe`.
- **Nowy kanał lub świat.**
  1. Wpis w rejestrze. Dla nowego świata `world:ensure` sprawdza go względem manifestu świata z wydania.
  2. Nowa generacja topologii.
  3. `provision-node`.
  4. `install-cert` (CSR agenta).
  5. `apply-nodespec`.
  6. `start`.
  7. Gra rejestruje się i zgłasza przypisanie scope.
  8. Publikacja trasy i podbicie epoki directory.
  9. Klient widzi nowy kanał.

### RBAC, audyt, potwierdzanie

Uprawnienia: `ops.nodes.view`, `ops.logs.view`, `ops.logs.export`, `ops.nodes.start`, `ops.nodes.stop`, `ops.nodes.restart`, `ops.deploy`, `ops.channels.manage`, `ops.worlds.manage`, `ops.maintenance.manage`, `ops.broadcast`, `ops.shutdown.schedule`, `ops.agents.enroll`, `ops.reconcile`.

- Akcje destrukcyjne wymagają trybu sudo, podglądu skutków i wpisania nazwy celu.
- Na produkcji wymagają też **czterech oczu**: deploy, zatrzymanie świata, usunięcie kanału i reconcile.
- Każdy krok jest audytowany.

### Utwardzenie agenta

NoNewPrivileges, ProtectSystem=strict, ProtectHome, PrivateTmp, pusty CapabilityBoundingSet, tylko AF_UNIX/INET, filtr syscalli `@system-service`. Jednorazowy token rejestracji ważny 15 minut. Pin SPKI. Lista unieważnień.

### Akceptacja M5

- Start, stop, restart, deploy i rollback działają na ch-A, ch-B (vps-2) i ch-C (oteryn-vps).
- Nowy kanał przechodzi od konsoli do widoczności w kliencie w mniej niż 10 minut.
- Supervisor przeżywa fuzzing czasowników.
- Sfałszowana lub wygasła komenda jest odrzucana.
- Opóźnienie podglądu logów jest mniejsze niż 3 s.
- Agent węzła jest przetestowany na awarię, utratę sieci i zużyty plik startowy.

## Wiki z katalogu gry (M6)

### Co istnieje w repo gry (origin/main)

- `content/` zawiera stworzenia, przedmioty (77 plików), NPC (1 282), loot (1 290 tabel), questy, obszary świata i rozmieszczenia (1 208 chunków `.b3`), domy (995), stroje, wierzchowce (252), zdolności, charmy (25), handel (324), podróże (56), biegłości (443) i powiązania prezentacji.
- Assety: appearances 15.30, 6 241 arkuszy lzma, mapa, staticmapdata i backdrop.
- `crates/client-assets` już dekoduje appearances i arkusze.
- Catalog Export v1 (8 capabilities) ma dziś tylko nieobsługiwany fixture. Laravelowe `app/GameCatalog/*` implementuje import, walidację, diff, aktywację, rollback i weryfikację.

### Kontrakt i paczka (GC5, dostarcza gra)

- **Catalog Export v2.** Dodaje capabilities: `creature_spawn`, `npc_trade_offer`, `npc_travel`, `house`, `outfit`, `mount`, `vocation`, `area`, `map_marker`, `charm`, `bestiary_entry`, `boss`, `imbuement`, `forge_rule`, `experience_table`, `stamina_rule`, `skill_rule`, `damage_rule`, `task`, `appearance_ref`.
  - **Każda capability jest dostarczona jako `supported + complete`** (kryterium akceptacji GC5 i GC11).
- **Bundle v1.** Artefakt OCI `ghcr.io/<org>/oteryn-catalog:<release>` zawierający:
  - manifest plus podpis Ed25519 (`manifest.json` + `manifest.sig`, klucz wydania);
  - `catalog.json`;
  - `rules-vectors.json`;
  - `assets/<sha256>.webp` (tylko przywoływane appearances; animowane sprite'y idle stworzeń; warstwy bazowa i maski dla strojów);
  - `licensing.json`.
- Paczka **nie zawiera kafelków mapy**. Dane mapy pochodzą wyłącznie z publikacji Atlasa, zbudowanej z GC16 w ramach tego samego manifestu wydania treści.
- Producent w Pythonie sprawdza digest krzyżowo w CI.

### Import i aktywacja

1. `content_release_pull` weryfikuje manifest.
2. `catalog_import` weryfikuje podpis, hashe plików, digest koperty, referencje, licencje i `rules_abi` oraz ponownie uruchamia wektory reguł.
3. Ładuje niezmienny snapshot i wgrywa assety (adresowane treścią).
4. Buduje modele do odczytu i indeksy odnośników krzyżowych.
5. Porównuje wydanie do aktywnego (`entity_changes`).
6. Uruchamiają się `atlas_build` i `atlas_verify` (sekcja „Oteryn-Atlas i wspólny silnik mapy”, X.4), w tym kontrole krzyżowe współrzędnych i tożsamości względem tego snapshotu.
7. Podgląd staffu.
8. `content_release_activate` przestawia katalog i Atlas w jednej transakcji. Rollback to odwrotne przestawienie. Ostatnie 5 wydań jest zachowanych.
9. Wdrożenia aktywują pasujące wydanie automatycznie.

### Strony

- URL `/{locale}/wiki/{type}/{slug}` dla creature, item, npc, spell, rune, quest, area, house, outfit, mount, vocation, charm i boss. `/wiki/k/{content_key}` przekierowuje na ten adres.
- **Zablokowany infobox:** statystyki; loot z szansą i liczbą; „wypada z”; „sprzedawane / skupowane przez” z cenami; „uczy czaru”; statystyki czarów i run; nagrody z questów; „występuje w” ze szpilką na mapie; dane domu; progi bestiary i punkty charmów.
- **Mapa.**
  - Każda encja z pozycją osadza `<oteryn-map mode="embed">` w współrzędnych gry tej encji, z nakładką `public` dla jej spawnów, ofert lub szpilek.
  - Bez JS pokazuje wyrenderowany wcześniej statyczny crop.
  - „Otwórz w Atlasie” prowadzi do `/atlas/{type}/{content_key}`.
  - Strony obszarów pokazują warstwę nazwanych obszarów.
- **Historia:** „Zmienione w wydaniu X” i „Usunięte w wydaniu X”.
- **Wyszukiwanie:** pełnotekstowe PG (`simple` + `unaccent`) plus `pg_trgm`, z autouzupełnianiem htmx. Wyszukiwanie serwisu i wyszukiwanie Atlasa rozwiązują się do tych samych tożsamości `content_key`.
- **Kategorie:** automatyczne plus redakcyjne.
- **Opisy:** sekcje per encja przez szkic → przegląd → publikację. Shortcode'y są walidowane przy aktywacji.
- **Migracja:** artykuły wiki z Laravela są migrowane; artykuły, które mapują się na encję, stają się jej opisem.

### i18n

Angielskie nazwy pochodzą z katalogu. Polskie nazwy i opisy to redakcyjna nakładka `catalog.translations`. **Bramka M6 wymaga 100% polskiego pokrycia nazw encji** dla stworzeń, przedmiotów, NPC, czarów, obszarów i questów. Tłumaczenie jest obsadzoną pracą w M6 z panelem pokrycia. Teksty UI Atlasa i nazwy obszarów z katalogu są w tej samej polskiej nakładce.

### Decyzja o crate'cie

`oteryn-catalog` jest osobnym crate'em, a nie częścią `platform-wire`: ma cykl życia per wydanie, duży rozmiar i innych konsumentów.

### Akceptacja M6

- Wydanie importuje się, weryfikuje i aktywuje razem ze swoją publikacją Atlasa.
- Każdy typ encji się renderuje, z osadzoną mapą na `oteryn-map`.
- Odnośniki krzyżowe są kompletne (każdy przedmiot z lootu ma „wypada z”).
- Wyszukiwanie p95 poniżej 150 ms.
- Przestawienie rollbacku przywraca katalog i Atlas razem.
- Polskie pokrycie wynosi 100%.
- **Krok wdrożeniowy:** preprod, potem promocja wydania na dark prod.

## Oteryn-Atlas i wspólny silnik mapy (MA)

### X.1 Decyzja: wchłaniamy kod

**Fakty.** Odczytane ze shallow clone na 0bbd188 i z Game origin/main 77af046c.

- **Runtime przeglądarki.** Atlas to zwykły runtime z modułów ES, bez bundlera. Ma ok. 366 plików `.mjs` razem z testami, a jedyną zależnością npm jest zależność deweloperska `@playwright/test` 1.62.0.
  - `src/browser/` zawiera fullworld-webgl, piksele progresywne, viewport-transform, prezentację stworzeń, inspektor, runtime animacji, inteligencję farm, wyszukiwanie semantyczne i zweryfikowany cache treści.
  - `src/layers/` zawiera minimapę, overview i regiony.
  - `src/hunt-intelligence/consumer.mjs` to konsument inteligencji polowań.
  - `web/` zawiera aplikacje fullworld, mobile, search i farm-explorer.
  - `web/fullworld.html` nie ma inline script.
- **Narzędzia budowania.** Publikację buduje 43 narzędzia w Pythonie: fabryka generacji, kompilator publikacji, pixel store, 256 bucketów pikseli adresowanych hashem treści, indeks runtime, minimapa/overview, kompilator semantyczny, przyrostowy graf treści i weryfikatory.
- **Hosting dziś.** Żywa strona działa na Synology (runner `oteryn-synology-atlas`, etykieta `oteryn-atlas`). Akceptacja wymaga, żeby etykieta kontenera `org.oteryn.revision` była równa nagłówkowi `X-Oteryn-Atlas-Revision`.
- **Wejścia po stronie gry.** Gra już posiada kontrakty eksportu Atlasa w `docs/contracts/OTERYN_GAME_ATLAS_*`: kontrakt eksportu, profile rozgrywki stworzeń, role NPC, profil wyszukiwania semantycznego, inteligencję farm i animowane appearances. Posiada też producentów w `tools/game-atlas-*`.
- **Źródło pełnego świata nie jest natywne.** `tools/game-atlas-fullworld-source` czyta przypięty **legacy** `world.otbm` i zip assetów **15.32**. Natywny świat gry to `content/world/placements/*.b3`, a dostarczane appearances to **15.30**.
- **Ograniczenia z dokumentów runtime.** Dekodowanie semantyczne działa w głównym wątku. Tablica tekstur rezerwuje cały wszechświat pikseli. Animacja jest ZABLOKOWANA do czasu dostarczenia danych faz/czasów. Nazwane warstwy faktyczne są ZABLOKOWANE lub NIEZNANE. Wszystkie te cztery ograniczenia są zamykane w MA (X.6 i GC16).

**Decyzja: wchłaniamy.** Runtime trafia do Platform-2 jako silnik `oteryn-map`. Kompilator publikacji jest przeportowany do crate'a Rust `atlas-publish`. Weryfikatory Pythona z Atlasa zostają jako niezależny oracle w CI.

Dlaczego wchłaniamy:

1. Jeden silnik mapy dla Atlasa, wiki, narzędzi i GM observe jest niemożliwy, gdy silnik pochodzi z innego repozytorium, origin i cyklu wydań.
2. Daje jeden przepływ publikacji i jedną aktywację razem z katalogiem. Osobny hosting zostawiłby dwa potoki podpisanych artefaktów, dwa CDN, CORS między originami i dwa rollbacki, które mogą się rozjechać.
3. Jedno przełączenie wycofuje hosta Synology. Model samego hostingu zostawiłby trzeci host produkcyjny z własnym uprawnieniem do deployu.
4. Kod łatwo wchłonąć. Nie ma zależności runtime ani kroku budowania i inline script, więc pasuje do restrykcyjnego CSP Platform-2 bez zmian.

**Odrzucone: hostowanie opublikowanych paczek z Platform-2.** Zostawia własny cykl wydań i renderer Atlasa, więc wiki, narzędzia i observe potrzebowałyby drugiego widoku mapy. To łamie wymóg jednego silnika.

**Granica autorytetu zachowana z `AGENTS.md` Atlasa.**

- Gra jest właścicielem semantyki World/Content, współrzędnych, pięter, kolejności i tożsamości.
- Moduł Atlasa tylko pochodnie wylicza z bezpiecznych publicznie eksportów gry.
- Wejścia legacy OTBM, Tibia, Canary i Crystal nigdy nie są zapasowym źródłem w runtime.
- Zasada „Platforma nie jest źródłem danych runtime Atlasa” jest egzekwowana przez D19: wejścia publikacji pochodzą wyłącznie z artefaktów GC16 i GC5. Dane platformy pojawiają się tylko jako osobne warstwy nakładek, nigdy wewnątrz publikacji ani jej indeksu wyszukiwania.

### X.2 Import kodu (tydzień 1 MA)

**Kod.**

- `git filter-repo` importuje `src/`, `web/`, `e2e/`, `tools/` i `docs/` do Platform-2 z zachowaniem historii i autorstwa.
- Własne narzędzie Atlasa `tools/governance/verify_extraction_provenance.py` tworzy rekord pochodzenia w `docs/atlas/provenance/`.
- Atesty prawne przechodzą do `docs/atlas/provenance/`.

**Repozytorium Atlasa.**

- README Atlasa zmienia PR do chronionego maina, żeby wskazywał na Platform-2. Od końca MA scalana jest tam wyłącznie ta zmiana wskaźnika.
- Repozytorium jest archiwizowane w T+14 d (sekcja „Migracja i przełączenie”).

**Jira.**

- Historie projektu KAN Atlas są przemapowane na historie MA Platform-2 przez koordynatora programu w ramach KAN-23.

### X.3 Wejście z gry: GC16 Atlas World Export v1 (patrz 11.2)

Każde wydanie gry publikuje artefakt OCI `ghcr.io/<org>/oteryn-atlas-world:<release>`. Jest podpisany kluczem wydania Ed25519 i wskazywany z tego samego manifestu wydania treści co paczka katalogu GC5.

| Część | Kontrakt (repo gry) |
| --- | --- |
| Projekcja źródłowa kafelków pełnego świata: per piętro i region, shardy JSONL z adresem logicznym i SHA-256, plus manifest przekazania generacji. **Produkowana z natywnego `content/world` (.b3) i appearances 15.30, które dostarcza klient.** Producent legacy OTBM nie jest już wejściem produkcyjnym. | `OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1` → ACCEPTED, plus `OTERYN_GAME_ATLAS_WORLD_BUNDLE_V1` (nowy) |
| Profile rozgrywki stworzeń, role NPC | `..._CREATURE_GAMEPLAY_PROFILES_V1`, `..._NPC_ROLES_V1` |
| Profil wyszukiwania semantycznego | `..._SEMANTIC_SEARCH_PROFILE_V1` |
| Animowane appearances z autorytatywnymi fazami i czasami. Odblokowuje animację. | `OTERYN_ATLAS_ANIMATED_APPEARANCES_V1` |
| Nazwane warstwy faktyczne: nazwy i granice obszarów i regionów z capability `area` katalogu. Odblokowuje nazwane warstwy. | Rejestr autorytetów wewnątrz paczki |
| Inteligencja farm/polowań: katalog polowań per wydanie oraz **cotygodniowa zagregowana wydajność** (supresja progiem k z sekcji 0) jako osobny artefakt `oteryn-atlas-farm:<rrrr-tt>` | `OTERYN_GAME_ATLAS_FARM_INTELLIGENCE_V1` |

### X.4 Przepływ publikacji (wspólny z katalogiem i assetami)

1. CI gry `release-content.yml` publikuje paczkę katalogu, paczkę świata Atlasa i manifest wydania treści. Manifest wiąże wydanie gry, SHA gita, digest obrazu, oba digesty paczek i `rules_abi` i jest podpisany.
2. p2-worker (preprod) uruchamia `content_release_pull`, który weryfikuje podpis i każdy digest.
3. `catalog_import` działa jak w sekcji „Wiki z katalogu gry”.
4. `atlas_build` uruchamia `crates/atlas-publish` przyrostowo, kluczowany ID treści chunków i pikseli względem poprzedniej zaufanej publikacji. Buduje:
   - publikację semantyczną (świat, piętro, chunki);
   - pixel store (paczki po 64 MiB);
   - 256 bucketów pikseli plus bundle `local-max`;
   - indeks runtime;
   - overview LOD minimapy;
   - shardy indeksu wyszukiwania;
   - shardy rozgrywki stworzeń;
   - dane farm;
   - **statyczne crops po stronie serwera** (webp) dla każdej encji katalogu z pozycją, używane jako fallback bez JS i jako miniatury wiki.
5. `atlas_verify` uruchamia dwie niezależne kontrole i obie muszą przejść:
   - weryfikator w Rust;
   - oracle w Pythonie (`tools/atlas-oracle/`: `verify_publication.py`, `negative_tests.py`, `verify_overview.py`, `verify_authority_registry.py`, `verify_handoff.py`) w piaskownicowym kontenerze joba.

   Uruchamia też kontrole krzyżowe, które odrzucają wydanie przy porażce:
   - każda współrzędna katalogu (spawny, markery, pozycje NPC, wejścia do domów, szpilki questów) rozwiązuje się do kafelka w publikacji tego samego wydania;
   - każda tożsamość stworzenia i NPC w Atlasie rozwiązuje się do `content_key` katalogu.
6. `atlas_upload` wgrywa obiekty adresowane treścią do `oteryn-assets/preprod/atlas/<sha256>`.
7. **Aktywacja to jedna transakcja PG.** Przestawia `catalog.active` i `atlas.active` na ten sam `content_release_id`.
   - Wdrożenia aktywują pasujące wydanie automatycznie (sekcja „Zarządzanie serwerami gry”).
   - Rollback to odwrotne przestawienie.
   - Aktywne wydanie i 4 poprzednie są zachowane. Worker usuwa obiekty bez referencji 30 dni po wycofaniu.
8. **Promocja na prod** (dark prod od M5, żywa produkcja od przełączenia) to kopia OCI po stronie serwera tych samych obiektów do prefiksu `prod/`. Digesty są weryfikowane ponownie, potem działa ta sama aktywacja w DB. Produkcja nigdy nie buduje.

Cotygodniowy artefakt farm przechodzi te same kroki 2, 5, 6, 7 i 8 pod własnym wskaźnikiem `atlas.farm_active`.

**Wierność portu do Rusta.** CI uruchamia też oryginalny kompilator Atlasa w Pythonie na fixture kwalifikacyjnym i na ograniczonym wycinku prawdziwego świata i wymaga identycznych bajt w bajt korzeni z obu kompilatorów. To zadanie jest stałe.

### X.5 Składowanie, CDN, cache

**Object storage.** Wszystkie obiekty są niezmienne i adresowane treścią pod `{env}/atlas/<sha256>`. Mutowalne obiekty nigdy nie idą do CDN.

**Wskaźnik bieżącej publikacji.** p2-public serwuje `/atlas/api/current` z bazy: `{content_release_id, publication_root, roots..., object_base}`, z `Cache-Control: public, max-age=30` i ETag. HTML też osadza korzeń, więc strona i dane zawsze do siebie pasują.

**Nagłówki obiektów na `cdn.oteryn.com`:**

- `Cache-Control: public, max-age=31536000, immutable`;
- `Accept-Ranges: bytes`;
- chunki i paczki są składowane jako `application/octet-stream` z kodowaniem identity, więc kompresja na brzegu nigdy nie psuje zakresów bajtów;
- manifesty JSON są kodowane brotli;
- CORS dopuszcza `https://oteryn.com` i `https://v2.oteryn.com` i wystawia `Content-Range`, `ETag` oraz `X-Oteryn-Atlas-Publication`.

**Cloudflare.** Reguła cache buforuje wszystko wszędzie; paczki mają najwyżej 64 MiB, co mieści się w limitach obiektów na brzegu.

**Przeglądarka.** Zweryfikowany cache treści zostaje: Cache API, kluczowany ID treści, ponownie hashowany przed użyciem.

**Nagłówki rewizji.** Przenoszą regułę akceptacji Atlasa do Platform-2:

- każda odpowiedź `/atlas` wysyła `X-Oteryn-Revision` (SHA gita Platform-2, równy etykiecie obrazu `org.oteryn.revision`) i `X-Oteryn-Atlas-Publication`;
- testy smoke wdrożenia sprawdzają, że nagłówek, etykieta obrazu i aktywny korzeń w DB zgadzają się.

**Rozmiary i budżety.**

- Rozmiar publikacji i czas budowy są mierzone w tygodniu 2 MA na bieżącym wydaniu.
- Alerty odpalają, gdy publikacja przekracza 20 GB albo build przyrostowy trwa dłużej niż 45 minut na vps-2.
- Scratch buildów to wolumen blokowy 60 GB na vps-2.

### X.6 Jeden silnik mapy: `oteryn-map`

**Pakowanie.**

- `crates/web/static/map-engine/` zawiera moduły ES Atlasa, serwowane z `/static/map-engine/<hash>/`.
- Atrybuty `<oteryn-map>`: `publication`, `floor`, `x`, `y`, `zoom`, `mode` (`full | embed | tools | observe | replay`), `layers` i `overlay-src`.

**Warstwy bazowe.** Pochodzą wyłącznie z publikacji:

- szczegół (semantyka i piksele);
- overview LOD minimapy;
- piętra;
- markery stworzeń i NPC;
- animacja;
- nazwane obszary;
- heatmapa farm/polowań.

**Warstwy nakładek.** Każda nakładka ma zadeklarowaną klasę prywatności, egzekwowaną po stronie serwera i klienta:

| Klasa | Źródło | Cache | Używane przez |
| --- | --- | --- | --- |
| `public` | Pozycje z katalogu (spawny, NPC, domy, szpilki questów i tasków, trasy podróży) z aktywnego wydania treści | CDN | Wiki, narzędzia, Atlas |
| `owner` | JSON z uwierzytelnieniem: cele bestiary, zapisane trasy, ślad sesji GC9 | `private, no-store` | Narzędzia gracza, konto |
| `staff` | SSE `/admin/observe/{id}/stream` (klatki na żywo) i `/admin/observe/{id}/replay` | `no-store`, trasa staffu, tryb sudo | GM observe |

**Reguły nakładek.**

- Element nakładki bez pozycji pochodzącej z gry (`{x, y, floor}` z katalogu, projekcji lub klatki observe) jest odrzucany i liczony w diagnostyce. Pozycje nigdy nie są wnioskowane.
- Nakładki nigdy nie wchodzą do publikacji ani do indeksu wyszukiwania.

**Prace runtime wykonane w MA.** Zamykają ograniczenia runtime z X.1:

- dekodowanie semantycznego JSONL przechodzi do Web Workera, z pomiarem przed i po;
- tablica tekstur stronicuje po bucketach, w budżecie `MAX_ARRAY_TEXTURE_LAYERS`, więc mobilne GPU nie rezerwują całego wszechświata pikseli;
- animacja działa na żywo z czasów GC16;
- warstwa nazwanych obszarów działa na żywo.

**Fallback bez WebGL2 i bez JS.** Strona renderuje statyczny crop po stronie serwera dla żądanej pozycji, z wyszukiwaniem SSR i panelem inspektora SSR. Strony wiki i narzędzi pozostają używalne bez JavaScriptu.

**Kontrola CI wymusza jeden silnik.** Zawodzi, gdy w `static/` istnieje jakakolwiek druga biblioteka mapy lub plik widoku, albo gdy odwołuje się do niej szablon.

### X.7 Trasy

| Trasa | Cel |
| --- | --- |
| `/atlas` (EN), `/pl/atlas` (PL); `/en/atlas` → 301 `/atlas` | Pełny świat: progresywne piksele, minimapa, piętra, markery, inspektor, animacja, wyszukiwanie, eksplorator farm, układ mobilny. Para `hreflang`. |
| `/atlas/@{x},{y},{floor},{zoom}` (+ `/pl/...`) | Link głęboki. Udostępnialny, kanoniczny, renderowany po stronie serwera ze statycznym cropem i obrazem Open Graph. |
| `/atlas/creature/{content_key}`, `/atlas/npc/{content_key}`, `/atlas/area/{key}`, `/atlas/hunt/{hunt_id}` | Widoki skupione. Linkują do `/{locale}/wiki/...`, a „występuje w” z wiki linkuje tutaj. |
| `/atlas/search?q=` | Wyniki SSR, na tych samych tożsamościach co shardy po stronie klienta |
| `/atlas/api/current`, `/atlas/api/crop/{x},{y},{floor},{zoom}.webp` | Wskaźnik oraz proxy cropów (serwowane z wcześniej wyrenderowanych cropów lub overview, cache w CDN) |
| Osadzenia | Strony encji i obszarów wiki, narzędzia (mapa bestiary, mapa tasków/dostaw, miejsca polowań, ślad sesji), GM observe na żywo i replay |

### X.8 Weryfikacja ścieżek mapy w Platform-2

**Możliwości danych.** Trzy capabilities Atlasa zostają:

- `qualification_fixture`: wycinek Thais z7 z GC16, zatwierdzony w repo;
- `bounded_real_world`: wycinek regionu bieżącego wydania treści, pobrany po digeście;
- `real_fullworld`.

**Joby.**

- `e2e.yml` działa na PR i w Merge Queue, z zerem ponowień (`retries: 0`, bez rerun-until-green). Uruchamia Playwright 1.62.0 na Chromium, Firefox i WebKit, desktop i mobile, i obejmuje scenariusze Atlasa, mapy wiki, mapy narzędzi i replayu observe. Bazowe obrazy wizualne zapisują digesty zrzutów, recenzenta i dokładną rewizję.
- `atlas-fullworld.yml` uruchamia `real_fullworld` co noc na runnerze self-hosted vps-2 przeciw publikacji preprodu, w headless Chromium z WebGL2.
- `atlas-gpu-perf.yml` działa na `molehill-gpu` przy każdej bramce kamienia milowego i w bramce przełączenia. Zapisuje czas klatki CPU, czas GPU gdy jest dostępny, liczbę draw calli, bajty sieci i uwierzytelnione oraz czas uploadu. Brakujące metryki zostają `null`.

### X.9 Wycofanie osobnego Atlasa (przy przełączeniu; runbook w sekcji „Migracja i przełączenie”)

1. **Inwentarz (tydzień 1 MA).** Odczyt nazw hostów podpiętych do Atlasa na Synology z DNS w Cloudflare i import ich do OpenTofu. Crawl żywej strony buduje mapę URL.
2. **Mapa przekierowań** (`deploy/caddy/atlas-redirects.caddy`, sprawdzana w CI względem crawla):

   | Stary URL | Nowy URL |
   | --- | --- |
   | `/`, `/fullworld.html` | 301 `https://oteryn.com/atlas`, query zmapowane na formę linku głębokiego |
   | `/semantic-search/*` | 301 `/atlas/search` |
   | `/creature-gameplay/*` | 301 `/atlas` |
   | `/proof/*` | 410 |

3. **T0+30.** OpenTofu kieruje nazwy hostów Atlasa na Caddy w oteryn-vps, który serwuje przekierowania.
4. **T+14 d.**
   - Zatrzymanie kontenera Atlasa na Synology i wyrejestrowanie jego runnera.
   - Archiwizacja `Oteryn/Oteryn-Atlas`.
   - Zapis końcowej rewizji osobnego Atlasa, korzenia publikacji i dowodów, których wymaga runbook live Atlasa.

**Rollback w oknie.** Do T+14 d kontener na Synology działa bez trasy. Rollback (a) lub (b) kieruje DNS z powrotem na niego.

### X.10 Akceptacja MA

1. **Build publikacji.** Bieżące wydanie treści GC16 i GC5 buduje się na vps-2, a weryfikator w Rust i oracle w Pythonie przechodzą. Dwa czyste buildy dają identyczne bajt w bajt korzenie, a zadanie równoważności z kompilatorem Pythona przechodzi.
2. **Parytet funkcji.** `/atlas` i `/pl/atlas` zgadzają się z osobnym Atlasem w jego końcowej rewizji, sprawdzone względem spisanej listy: progresywne piksele pełnego świata, minimapa, piętra, markery stworzeń i NPC, inspektor, inteligencja farm/polowań, wyszukiwanie semantyczne, mobile, linki głębokie. Ponad parytet:
   - animacja działa na żywo;
   - nazwane obszary działają na żywo;
   - dekodowanie semantyczne działa w workerze.
3. **Różnice wizualne.** Każda różnica wizualna względem osobnego Atlasa w tym samym regionie jest wymieniona w zrecenzowanym raporcie różnic i przypisana do treści natywnego świata (zmiana źródła GC16).
4. **Aktywacja wydania treści.**
   - Przestawia katalog i Atlas razem.
   - Przestawienie rollbacku przywraca oba.
   - Negatywne fixture'y (niedający się rozwiązać współrzędna, nieznana tożsamość, brak rekordu praw, niezgodny digest) każdy odrzuca aktywację.
5. **Jeden silnik.** Mapa stworzeń na wiki, mapa tasków w narzędziach i prototyp replayu observe na węźle testowym działają na `oteryn-map`, a kontrola CI jednego silnika jest zielona.
6. **Dostarczanie.**
   - Żądania zakresu przez CDN działają, a bajty z cache są ponownie hashowane.
   - CSP nie ma inline script ani żadnego origin poza self i `cdn.oteryn.com`.
   - Przebieg `molehill-gpu` jest zapisany.
   - Dowody wizualne są zrecenzowane i podpisane.
7. **Krok wdrożeniowy.** `deploy-preprod.yml` wypuszcza obraz. Wydanie treści `N` jest promowane do prefiksu dark prod i tam aktywowane, a test smoke dark prod przechodzi kontrole nagłówków rewizji i publikacji.

## Narzędzia dla gracza (M7)

**Każde narzędzie poniżej jest zbudowane i wdrożone.**

| Narzędzie | Dane | Dane osobiste |
| --- | --- | --- |
| Tracker bestiary i charmów: automatyczny z GC3 plus ręczny dla celów i planowania, z **mapą bestiary** lokalizacji spawnów na `oteryn-map` | `bestiary_entry`, `charm`, `creature_spawn` | Postęp z GC3 |
| Tracker bosstiary | `boss` | GC3 |
| Kalkulatory doświadczenia/poziomu, staminy i umiejętności/treningu | `experience_table`, `stamina_rule`, `skill_rule` | Wstępnie wypełnione z GC3 |
| Kalkulator imbuementów: koszt, materiały, źródła, ceny NPC | `imbuement`, `npc_trade_offer` | — |
| Symulator forge/exaltation: Monte Carlo po stronie serwera, najwyżej 100 tys. prób, deterministyczny seed, z cache | `forge_rule` | — |
| Kalkulator obrażeń/pancerza | `damage_rule`, przedmioty | Statystyki ekwipunku z GC3 |
| Podział lootu party z linkiem udostępniania (7 dni) | przedmioty | — |
| Analizator sesji polowania/party: import eksportu sesji klienta Oteryn (GC9), ze **śladem sesji** rysowanym jako nakładka `owner` | przedmioty, stworzenia, obszary | Zapisywane tylko na żądanie |
| **Miejsca polowań**: eksplorator farm Atlasa i inteligencja polowań, połączone z sekcją narzędzi i współdzielone z panelem farm w `/atlas` | katalog polowań GC16 plus tygodniowe agregaty (supresja poniżej progu) | — |
| Mapa tasków/dostaw: taski, questy, NPC i oferty, miejsca polowań, spawny, domy | `task`, `quest`, `area`, `map_marker`, `npc_trade_offer`, `creature_spawn` | — |

### Zgodność formuł z grą

- Dane (tabele, stałe, prawdopodobieństwa) podróżują w katalogu.
- Logika żyje w `oteryn-game-rules`, wyciągniętym w GC8 z `apps/game-server`: `durability/character_build.rs`, `spell/mana_training.rs`, `content/item_stats_promotion.rs` oraz kod forge, imbuementów, staminy, umiejętności i obrażeń, który kończy GC11. **Serwer gry i strona uruchamiają ten sam kod.**
- Granica: kategoria `shared-domain`, jawna krawędź z game-server, egzekwowana przez architecture-check, we własnym ADR.
- Strażnik dryfu: aktywacja ponownie uruchamia `rules-vectors.json` przeciw podlinkowanemu crate'owi i odmawia przy jakiejkolwiek niezgodności.
- Rejestr narzędzi sprawdza capabilities w runtime jako strażnik fail-closed. GC5, GC11 i GC16 dostarczają każdą capability, więc wszystkie narzędzia działają przy przełączeniu.

### Postęp osobisty i prywatność

- GC3 pcha postęp tylko dla właściciela: bestiary, charmy, bosstiary, osiągnięcia, stroje i wierzchowce, questy, umiejętności, stamina, statystyki ekwipunku.
- Użytkownicy mogą dodawać własne cele i wpisy planowania.
- Publiczne udostępnianie to przełącznik opt-in per postać.
- Wklejone lub zaimportowane sesje nie są zapisywane, dopóki użytkownik ich nie zapisze, i są prywatne, dopóki nie zostaną udostępnione linkiem.
- Wszystko można usunąć i wchodzi do eksportu GDPR.
- Nakładki osobiste używają odpowiedzi `private, no-store`, nigdy nie trafiają do CDN ani do publikacji.
- Agregaty farm nie zawierają identyfikatorów postaci ani kont.

### Renderowanie

Rust po stronie serwera z wymianą fragmentów htmx 4 (wejścia z debounce). Każda mapa używa jednego silnika `oteryn-map` (D3). Bez WASM.

### Akceptacja M7

- 100% równości z wektorami reguł gry.
- Dane trackera automatycznego zgadzają się z GC3.
- Narzędzia p95 poniżej 200 ms, symulator forge poniżej 1 s.
- Każde narzędzie działa bez JS; mapy degradują się do statycznych cropów.
- Liczby w miejscach polowań są równe wartościom artefaktu farm, a supresja jest przetestowana.
- **Krok wdrożeniowy:** preprod, potem dark prod.

## Panel konta i postaci („Moje konto”)

| Funkcja | Kontrakt | Kamień milowy |
| --- | --- | --- |
| Przegląd: status, czas premium, coiny, ocena bezpieczeństwa | Platforma (P) | M1 |
| Zmiana e-maila, potwierdzana ze starego i nowego adresu, link cofnięcia ważny 7 dni | P + podbicie generation evidence | M1 |
| Zmiana hasła ze sprawdzeniem HIBP | P + podbicie generation | M1 |
| 2FA TOTP + kody odzyskiwania; **passkeys** | P + podbicie generation | M1 |
| Klucz odzyskiwania | P | M1 |
| Sesje i urządzenia, zdalne wylogowanie, wylogowanie wszędzie | P + podbicie generation | M1 |
| Historia logowań i zdarzenia bezpieczeństwa, w tym admission do gry z IP klienta z rustowego gatewaya | P | M1 |
| Powiadomienia, język, strefa czasowa | P | M1 |
| Eksport GDPR (ZIP, link na 24 godziny, raz na 7 dni) | P | M3 |
| Usunięcie konta, karencja 30 dni, z możliwością anulowania; w karencji dostęp do gry zablokowany | P + evidence + usuwanie GC2 | M3 |
| Lista postaci na żywo: online, świat, poziom, profesja, ostatnie logowanie | LCFA + `world_online` + GC3 | M3 |
| Tworzenie postaci z nazwą, światem, płcią, profesją, strojem | GC2 `CreateCharacter` v2 | M3 |
| Zmiana nazwy (cooldown 30 dni; pierwsza za darmo, potem 250 coinów) | GC2 `RenameCharacter` | M3 (opłata w coinach aktywna od M8) |
| Usunięcie z karencją 30 dni i anulowanie | GC2 `RequestCharacterDeletion` / `CancelCharacterDeletion` | M3 |
| Ukrycie lub pokazanie na profilu publicznym | GC2 `SetCharacterVisibility` | M3 |
| Komentarz/podpis, główna postać, flagi prywatności per postać | P | M3 |
| Statystyki, zgony, osiągnięcia | GC3 + `character_deaths` | M3 |
| Podgląd stroju (webp składany po stronie serwera) | GC3 + assety katalogu | M6 |
| Haczyki narzędzi gracza per postać | GC3 | M7 |
| Historia coinów, historia premium, zakupy, prezenty, faktury | P (ledgery, Stripe, KSeF) | M8 |
| Sklep z dostarczaniem przedmiotów do gry | GC13 | M8 |
| Oferty i licytacje na bazarze | Konsument GC2 `TransferCharacterOwnership` | M9 |
| Transfer świata | GC2 `TransferCharacterWorld` | M9 |
| Domy: aukcje, oferty, opuszczenie, transfer | GC14 | M9 |
| Gildia: tworzenie, zaproszenia, rangi, wyrzucanie, opuszczanie, rozwiązanie, opis, logo | GC12 | M9 |
| Zgłoszenia do supportu; status bana i odwołania | P | M3 |

**Trasy** (`/{locale}/account/...`):

- `/`, `/security`, `/security/passkeys`, `/email` (+ `/confirm/{t}`, `/revert/{t}`), `/sessions`, `/history`, `/privacy`, `/preferences`, `/sudo`
- `/characters`, `/characters/new`, `/characters/{id}` (+ `/rename`, `/delete`, `/visibility`, `/settings`, `/transfer`)
- `/premium`, `/coins`, `/shop`, `/shop/orders`, `/invoices`, `/gifts`
- `/bazaar`, `/bazaar/{auction}`, `/houses`, `/houses/{id}`, `/guilds/{id}/manage`
- `/support/tickets`, `/sanctions`, `/tools`

**Szablony** żyją w `templates/{account,characters,shop,bazaar,houses,guilds}/`.

**Zasady UX.**

- Tryb sudo dla zmian poświadczeń, 2FA, passkeys i klucza odzyskiwania, wylogowania wszędzie, eksportu, usunięcia, każdej akcji cyklu życia postaci, każdego zakupu powyżej 500 coinów, każdej oferty na bazarze i każdej oferty na dom.
- Akcje destrukcyjne używają strony potwierdzenia, wymagają wpisania nazwy postaci i wysyłają e-mail.
- Oczekujące komendy pokazują swój stan na żywo przez polling htmx do przyjścia potwierdzenia, ze zlokalizowanymi komunikatami `CHAR_CMD_*`.

**Akceptacja M3.** Każdy wiersz M1/M3 działa end to end z węzłem testowym, w tym rundy GC2/GC3 w czasie poniżej 5 s.

## Podgląd sesji gracza przez GM (M10)

**Dwa tryby, oba zbudowane.**

- **Feed webowy.** `<oteryn-map mode="observe">` renderuje aktywną publikację z pełnym szczegółem Atlasa, wokół bieżącej pozycji celu. Nakładka `staff` niesie pozycję, ślad, pobliskie encje rysowane ich sprite'ami appearances przez runtime prezentacji stworzeń silnika, czat publiczny, akcje (fakty `ANALYSER_V1` plus zdarzenia ruchu i interakcji) oraz HP/MP. Ponieważ wdrożenia aktywują wydanie treści pasujące do działającego wydania gry, mapa zawsze zgadza się z żywym światem.
- **Obserwator w kliencie.** Klient Oteryn GM-a podłącza się tylko do odczytu i widzi dokładnie widok celu.

### GC10, dostarcza gra

- Platforma podpisuje grant z profilem `oteryn-observe-v1`. Claims: `observe_id`, `approval_id`, `gm_account_id`, `target_character_id`, `world_id`, `mode`, `pm_escalation`, najwyżej 30 minut, wygaśnięcie, audience.
- Gra weryfikuje podpis (keyring admission), audience i wygaśnięcie; że cel jest online; że GM nie jest na koncie celu. Następnie wiąże grant z **generacją sesji** celu.
- **Tryb feed.** `StartObserve` i `EndObserve` przychodzą przez komendy operatora. Klatki idą do `/internal/v1/gm-observe/frames` ok. raz na sekundę, najwyżej 64 KB, z `{observe_id, generation, seq}`. Używany jest dedykowany certyfikat i pula, najwyżej 3 równoczesne observe na świat.
- **Tryb klienta.** Capability protokołu `OBSERVE_V1` daje **sesję obserwatora bez aktora**, która odbija domeny stanu widoku celu. Każda komenda obserwatora poza `detach` jest odrzucana.
- **Strumień kończy się** przy wylogowaniu celu lub zmianie generacji, wygaśnięciu grantu, `EndObserve`, rozłączeniu GM-a lub restarcie gry. Platforma odrzuca klatki, których generacja lub `seq` się nie zgadza.
- **Wiadomości prywatne** są wykluczone u źródła, chyba że ustawiono `pm_escalation`. Nawet wtedy dołączane są tylko rozmowy z udziałem celu.
- Każda encja w klatce niesie swój `content_key` katalogu lub referencję appearance stroju oraz pozycję w grze.

### Bezpieczeństwo i prywatność

- **Uprawnienia:** `gm.observe.request`, `gm.observe.approve`, `gm.observe.pm_escalate` (tylko rola admin), `gm.recordings.view`, `gm.recordings.export`.
- **Każdy wniosek** wymaga powodu i linku do zgłoszenia, raportu lub sankcji. Zatwierdzenie czterech oczu jest potrzebne dla przeglądów bez otwartej sprawy, każdej eskalacji PM i każdego eksportu. Przy otwartej sprawie jeden GM rozpoczyna, a drugi przegląda w ciągu 24 godzin.
- **Zasady:** zawsze tylko do odczytu, tryb sudo, limit 30 minut, pełny audyt z łańcuchem hashy i miesięczny raport nadzorczy dla właściciela.
- **Powiadomienie:** gracz nie jest powiadamiany w czasie rzeczywistym, poza zgłoszeniami supportu zainicjowanymi przez gracza, gdzie adnotacja trafia do zgłoszenia. Monitoring jest ujawniony w regulaminie i polityce prywatności (art. 13).
- **RODO:** podstawa prawna art. 6 ust. 1 lit. f. Ocena prawnie uzasadnionego interesu oraz **DPIA powstają w M10 i są recenzowane przez polskiego prawnika od ochrony danych (pozycja bramki)**. Minimalizacja: feed nie niesie IP ani danych konta, a pobliscy gracze pojawiają się tylko z nazwą i pozycją. Sprzeciwy z art. 21 i wnioski o dostęp z art. 15 (z zaczernieniem) mają zdefiniowane procedury.
- Nakładka `staff` jest serwowana wyłącznie na trasach `/admin` z `no-store`. Nigdy nie jest cache'owana ani wysyłana do CDN.

### Nagrywanie i odtwarzanie

- **Każda sesja observe jest nagrywana** jako skompresowane klatki feedu. Replay używa `<oteryn-map mode="replay">` z suwakiem po nagranych klatkach i wydaniem treści aktywnym w czasie sesji (przypiętym przez `content_release_id` w `gm.observe_sessions`).
- Retencja:

  | Nagranie | Przechowywane |
  | --- | --- |
  | Niepowiązane ze sprawą | 30 dni |
  | Powiązane ze sprawą | Do zamknięcia sprawy i okna odwołań, plus 90 dni |
  | Dowolne nagranie | Najwyżej 1 rok, chyba że admin ustawi blokadę prawną (legal hold) |

- Każde obejrzenie i eksport jest logowane. Eksporty mają znak wodny i wymagają czterech oczu.

### Akceptacja M10

- Strumień pakietów celu jest identyczny bajt w bajt z obserwatorem i bez niego.
- Fencing generacji jest przetestowany przy ponownym logowaniu.
- Wykluczenie PM jest przetestowane.
- Audyt jest kompletny.
- Prawnik zatwierdził DPIA, regulamin i teksty prywatności.
- Widoki na żywo i replay renderują się na `oteryn-map` z właściwym wydaniem.
- **Krok wdrożeniowy:** preprod, potem dark prod.

## Schemat bazy platformy

| Schemat | Tabele |
| --- | --- |
| `identity` | identities, credentials, two_factor, recovery_codes, recovery_keys, webauthn_credentials, email_verifications, email_changes, password_resets, web_sessions, sudo_grants, security_state, security_events (partycje miesięczne), login_history (partycje miesięczne), preferences, notification_prefs, data_exports, deletion_requests, terminations |
| `auth` | oauth_clients, oauth_codes, oauth_tokens, oidc_keys, device_sessions, game_tickets, admission_keys_meta, evidence_floors, evidence_witness, gateway_bearers |
| `registry` | worlds, channels, routes, route_policies, topology_generations, directory_epochs |
| `characters` | command_intents, command_receipts, bootstrap_intents, lifecycle_intents, account_character_projection, details_projection, progress_projection, character_settings, projection_watermarks |
| `game_public` | highscores, character_profiles, character_deaths, houses, guilds, world_online, family_watermarks |
| `premium` | premium_ledger, premium_snapshots |
| `commerce` | products, prices, orders, payments, provider_events, refunds, ledger_accounts, ledger_transactions, ledger_entries, gifts, invoices, invoice_numbering, ksef_submissions, store_deliveries |
| `bazaar` | auctions, bids, escrow_holds, settlements |
| `houses` | auctions, bids, house_intents, house_receipts |
| `guilds` | guild_intents, guild_receipts, guild_media, guild_texts |
| `content` | news(+i18n), pages(+i18n), events, announcements, downloads, client_releases, updater_generations, media |
| `release` | content_releases (game_release, git_sha, image_digest, manifest_digest, signature, catalog_bundle_digest, atlas_world_digest, rules_abi, status, pulled_at), release_activations (env, content_release_id, activated_by, activated_at, reason, previous_id) |
| `catalog` | bundles, snapshots (z `content_release_id`), entities, relations, xref_item_drop, xref_npc_offer, xref_spell_teacher, xref_area_creature, xref_quest_reward, entity_changes, assets, translations, slugs. Aktywny wskaźnik żyje w `release.release_activations`. |
| `atlas` | publications (content_release_id, publication_root, semantic_root, pixel_root, bucket_root, runtime_index_root, overview_root, search_root, crops_root, bytes, object_count, builder_version, rust_verify_digest, oracle_verify_digest, equivalence_digest, status), publication_objects (publication_id, sha256, kind, bytes), active (env, publication_id, farm_publication_id), farm_publications (week, artifact_digest, signature, roots, suppression_policy_rev, status), build_jobs, crosscheck_failures, visual_evidence (revision, run_id, reviewer, playwright_result, screenshot_digests), standalone_retirement (hostnames, final_revision, final_publication_root, redirected_at, decommissioned_at) |
| `wiki` | prose_sections, prose_revisions, free_articles, categories, entity_categories, review_queue |
| `tools` | goals, manual_entries, saved_sessions, shared_splits, tool_settings, saved_routes, overlay_prefs |
| `support` | tickets, messages, reports, enforcement_records, appeals |
| `staff` | roles, permissions, role_permissions, staff_assignments |
| `ops` | hosts, agents, nodes, node_status, node_status_history, agent_commands, command_approvals, command_seq, releases, deployments, maintenance_windows, scheduled_shutdowns, operator_commands, operator_receipts, node_log_lines (partycje dzienne), audit_events, outbox, jobs |
| `gm` | observe_requests, observe_approvals, observe_sessions (z `content_release_id`), observe_frames (partycje dzienne), recordings, recording_access_log |

## Deploy na oteryn-vps i oteryn-vps-2

### CI/CD

| Workflow | Runner | Działanie |
| --- | --- | --- |
| `ci.yml` | `ubuntu-26.04` | fmt, clippy, testy z PG 19, kontrola sqlx offline, cargo-deny, zgodność (z przypiętym obrazem gry), **równoważność atlas-publish z kompilatorem Pythona na fixture i wycinku, uruchomienie oracle, kontrola jednego silnika** |
| `e2e.yml` | `ubuntu-26.04` | Playwright 1.62.0, 3 przeglądarki, desktop i mobile, `retries: 0`, dowody wizualne z digestami. Wymagany w Merge Queue. |
| `atlas-fullworld.yml` | Self-hosted, vps-2 | Co noc `real_fullworld` przeciw publikacji preprodu |
| `atlas-gpu-perf.yml` | `molehill-gpu` (efemeryczny) | Przy każdej bramce kamienia milowego i w bramce przełączenia |
| `image.yml` | `ubuntu-26.04-arm` | Natywne obrazy arm64 wszystkich czterech binarek; push do GHCR po digeście; SBOM; keyless podpis; etykieta `org.oteryn.revision` |
| `deploy-preprod.yml` | Self-hosted na oteryn-vps-2 | Automatycznie na `main`: migrate, `compose up`, testy smoke (w tym kontrola nagłówków rewizji i publikacji `/atlas`) |
| `deploy-dark-prod.yml` | Self-hosted na oteryn-vps, środowisko `v2-dark-prod` | Od M5, każde wydanie: aktualizacja stosu dark, promocja wydania treści, nocna próba importu |
| `deploy-prod.yml` | Self-hosted na oteryn-vps, środowisko `v2-prod` (ręczne zatwierdzenie) | Przełączenie i każde kolejne wydanie; promocja wydania treści |

Runnery: prywatne repo, efemeryczne, dedykowany użytkownik, stały `deploy.sh`, przypięte digesty, zachowane 3 ostatnie obrazy. Runner na Synology i kontener Atlasa na Synology są wycofywane w T+14 d (sekcja „Migracja i przełączenie”).

### Hosty i projekty compose

| Host | Projekt | Zawartość |
| --- | --- | --- |
| oteryn-vps-2 | `oteryn-v2-preprod` | p2-public, p2-internal, p2-worker, p2-pg19, login-gateway, Caddy |
| oteryn-vps-2 | `oteryn-v2-testgame-a` | Testowe węzły gry ch-A i ch-B, testowy PG gry, agent i supervisor |
| oteryn-vps-2 | `oteryn-monitoring` | Prometheus, Alertmanager (e-mail + webhook Discord), Grafana z logowaniem Oteryn OIDC |
| oteryn-vps | `oteryn-oracle` (istniejący) | Laravel, edge, nginx mTLS, gateway Go, gra prod. **Bez zmian do przełączenia.** |
| oteryn-vps | `oteryn-v2-testgame-c` | Testowy kanał ch-C we własnej sieci, zarządzany przez agenta |
| oteryn-vps | `oteryn-v2-prod` | Dark od M5; żywy od przełączenia |

- p2-worker na vps-2 montuje wolumen scratch 60 GB buildów w `/var/lib/oteryn/build`.
- p2-worker na oteryn-vps nie ma scratcha buildów, bo tylko promuje.

### Ustawienia wspólne

- **Kontenery:** uid 65532, system plików root tylko do odczytu, `cap_drop: ALL`, no-new-privileges, sekrety montowane w `/run/secrets`, healthchecki, logowanie journald.
- **Certyfikaty:** preprod używa własnego offline'owego root CA i online'owego pośredniego CA z ograniczeniem nazw. Produkcja używa istniejącego CA i certyfikatów serwera dla ścieżek gry i gatewaya oraz nowego pośredniego CA prod dla agentów.
- **Object storage.** Bucket OCI `oteryn-assets` z prefiksami `{env}/catalog/`, `{env}/atlas/` i `{env}/crops/` za `cdn.oteryn.com`. Cykl życia usuwa wyłącznie `atlas_gc`, nigdy reguły wieku bucketu.
- **Cloudflare.** Reguła cache dla `cdn.oteryn.com` i reguła transformacji dodająca nagłówki CORS i expose. Obie są zarządzane w OpenTofu.
- **Kopie zapasowe.** Archiwum WAL i kopie bazowe trafiają do osobnego bucketu OCI. Cotygodniowe ćwiczenie odtworzenia do preprodu. Od M5 codzienne ćwiczenie odtworzenia dark prod. Publikacje nie są backupowane: można je odbudować z podpisanych artefaktów gry, które ghcr.io zachowuje dla każdego wydania. Tabele DB `release` i `atlas` są backupowane razem z resztą.
- **System hosta.** oteryn-vps przechodzi na Ubuntu 26.04 w osobnym oknie w tygodniu 1 M0, po kopii boot volume i z testami smoke Laravela i gry po upgradzie. oteryn-vps-2 jest provisionowany na 26.04 przez OpenTofu.

## Migracja i przełączenie

**Warunki wstępne:**

- bramka M11 jest zielona;
- dark prod ma za sobą **14 kolejnych nocnych prób importu z zerem różnic w verify**;
- produkcyjne wydanie treści zgodne z GC15 jest wypromowane i zweryfikowane na dark prod.

### Zasady

- Bez podziału ruchu. Laravel działa bez zmian do T0.
- Wszystko jest ćwiczone na preprodzie v2 ze świeżych snapshotów produkcji: trzy próby przełączenia plus jedna z importem odwrotnym.
- Te same nazwy hostów, ścieżki, bajty `/internal/v1`, CA, subjecty certyfikatów, hashe bearerów i klucze Ed25519. Konfiguracje produkcyjnej gry i gatewaya się nie zmieniają. Rustowy gateway przejmuje adres i port gatewaya Go.

### Import danych (`tools/legacy-import`)

- Czyta odtworzony `pg_dump` Laravela (PG 17.11), nigdy żywą bazę; pisze do v2. Tryby: `import`, `verify`, `reverse`.
- Zachowuje: ID tożsamości, hashe argon2id i kody odzyskiwania; sekrety 2FA (odszyfrowane z laravelowego `encrypted` kluczem APP_KEY i zaszyfrowane key ringiem v2); klientów OAuth i redirecty; hashe sesji urządzeń jako wersja legacy; ID światów, kanałów i tras z epokami; `native_security_generation`, floory evidence i pliki witness; klucze admission z kid; intencje i potwierdzenia komend postaci; epoki i rewizje LCFA; ledger premium; całą treść CMS, wiki, media, eventy, pobieranie i updater; support, audyt i role (1:1 na 23 uprawnienia).
- Pomija: dane Canary i Player Companion, sesje, cache, joby i tickety Laravela.
- Publiczne dane gry nie są importowane: od przełączenia produkcyjny węzeł gry pcha do v2 Public Projections v1.
- `verify` porównuje liczby wierszy i digesty kolumn z sekretami, równość generacji, floorów i witness, sprawdza hasło konta testowego i rewizje LCFA z bazą gry (w próbach). Każda niezgodność przerywa import.
- Próby: na preprodzie v2 ze świeżych snapshotów prod: import, testowy węzeł gry, pełny test zgodności, logowanie, lista i tworzenie postaci, snapshot premium.

### Runbook (okno ≤60 minut)

| Kiedy | Krok |
| --- | --- |
| T-14 d | Zamrożenie Laravela. Zamrożenie maina `Oteryn/Oteryn-Atlas`. Ogłoszenie na stronie i Discordzie. |
| T-1 d | Przygotowanie sekretów prod: CA, klucz i certyfikat serwera, allowlista DN, hashe bearerów, klucze admission i kid, legacy APP_KEY, pliki witness evidence, klucz komend ops, pośrednie CA agentów, klucze live Stripe, token KSeF. Aktywacja wydania treści GC15 (katalog i Atlas) na dark prod. Zapis końcowej rewizji osobnego Atlasa i korzenia publikacji w `atlas.standalone_retirement`. |
| T0 | Maintenance trasy przez Laravela. Odpłynięcie graczy. Zatrzymanie gry przez `game-supervisor.py`. |
| T0+5 | `artisan down`. Zatrzymanie schedulera, kolejki i schedulera marketplace. Ostatnia kopia Laravela. |
| T0+10 | `legacy-import` + `verify`. Niezgodność przerywa i prowadzi do rollbacku (a). |
| T0+25 | Zatrzymanie nginx internal-mtls i gatewaya Go. Wyłączenie `oteryn-game.service`. Przeniesienie dark prod do `oteryn-oracle_private`: p2-internal na **172.18.0.12:8543**, p2-public, rustowy login-gateway pod IP gatewaya. Włączenie agenta i supervisora, które przejmują stan węzła z `/opt/oteryn-oracle/state/game-controller`; ustawiona flaga reconciliation przerywa. |
| T0+30 | Edge na Caddy 2.11.7 z `cutover.caddy` + `atlas-redirects.caddy` (`oteryn.com` → p2-public; `login.` / `gateway.` → login-gateway). OpenTofu stosuje DNS nazw hostów Atlasa na oteryn-vps. |
| T0+35 | Start gry z konsoli v2. Sprawdzenie rejestracji, runtime status, przypisania scope, watermarku LCFA ≤10 s, projekcji publicznych i szczegółów postaci. |
| T0+40 | Testy smoke: logowanie prawdziwym klientem, CreateCharacter v2, self-check evidence, sesja urządzenia, directory, testowy zakup live Stripe za 1 € ze zwrotem, wiki, narzędzia, podgląd logów, observe na postaci testowej staffu; `/atlas` i `/pl/atlas` ładują pełny świat; nagłówki rewizji i publikacji zgadzają się z etykietą obrazu i aktywnym korzeniem; działa jedno przekierowanie na każdą klasę starych URL Atlasa; osadzona mapa wiki się ładuje. |
| T0+50 | Zdjęcie maintenance. |
| T+14 d | Okno pracy równoległej się zamyka. Laravel jest zatrzymany, z bazą tylko do odczytu. Dla porównania działa staffowy Laravel tylko do odczytu na wewnętrznym hoście. Codzienny raport rekoncyliacji. W T+14 d wycofanie Laravela, gatewaya Go, runnera na Synology i **kontenera Atlasa na Synology**; archiwizacja `Oteryn/Oteryn-Atlas`. |

### Przekierowania

- Każdy istniejący publiczny adres zostaje; wycofane funkcje zwracają 301 albo 410.
- Mapa przekierowań sprawdzana w CI obejmuje zarówno crawl sitemapu Laravela, jak i crawl osobnego Atlasa.

### Rollback

- **(a) Przed T0+50:** przywrócenie edge, zatrzymanie v2, start nginx, Laravela i gatewaya Go, ponowne włączenie `oteryn-game.service`. Bez utraty danych.
- **(b) Od T0+50:** przećwiczony **import odwrotny** (`legacy-import reverse`), który przenosi tożsamości, hashe, generacje, floory, witness, intencje i potwierdzenia, epoki i ledgery. Potem powrót do poprzednich kontenerów. Zwykłe przełączenie z powrotem jest zabronione, bo gra odrzuciłaby cofnięte floory i rewizje. Dane istniejące tylko w v2 (katalog, wiki, ops, narzędzia, handel, observe) zostają w bazie v2 na kolejną próbę. Płatności Stripe przyjęte w v2 są rekoncyliowane do ledgera premium Laravela przez import odwrotny.
- W obu wariantach DNS nazw hostów Atlasa wraca na kontener Synology, który działa bez trasy do T+14 d.
- **Wyzwalacze:**
  - odsetek udanych logowań poniżej 95% przez 15 minut;
  - błędy admission lub evidence powyżej bazowych;
  - regresja licznika monotonicznego;
  - nieudany verify.
- **Okno zamyka się w T+14 d.** Wtedy Laravel, gateway Go, runner Synology i kontener Atlasa na Synology są wycofywani i archiwizowani (obraz Laravela, ostatni dump, APP_KEY w zimnym magazynie).

## Roadmapa

### 11.1 Kamienie milowe platformy

Każdy kamień milowy kończy się:

- wdrożeniem na preprod v2 przez `deploy-preprod.yml`;
- od M5 promocją i wdrożeniem na dark prod;
- demem dla właściciela;
- spełnionymi kryteriami akceptacji;
- od MA zapisanym przebiegiem `molehill-gpu`.

| M | Zakres | 1 dev | 2 dev | Start (2 dev) | Akceptacja (skrót) |
| --- | --- | --- | --- | --- | --- |
| M0 | Repo, CI na 26.04, chiselled runtime, OpenTofu (vps-2, wolumen scratch, NSG, buckety, DNS wraz z importem hostów Atlasa), oteryn-vps na 26.04, stos preprod, PG 19, CA, monitoring, layout/CSS/i18n, ADR-P2-0001, **poprawka ADR topologii w META** | 3–4 tyg. | 2–3 | 2026-11-02 | Obrazy podpisane; preprod zielony; oba ADR scalone. Grafana przez konto break-glass admina do czasu OIDC w M1. |
| M1 | Tożsamość, OAuth PKCE, dostawca OIDC, passkeys, rdzeń bezpieczeństwa, tryb sudo, legacy-import v0 | 5–6 | 3–4 | — | Zgodność OpenID Basic OP; Grafana używa OIDC Oteryn; hashe legacy i 2FA działają ze snapshotu prod |
| M2 | Parytet `/internal/v1`, rustowy gateway, zestaw zgodności, postacie, premium, rejestr | 5–6 | 3–4 | — | Zgodność 100%; właściciel gra na węźle testowym; p99 gatewaya nie gorsze niż Go |
| M3 | Panel konta i postaci, publiczne strony, support, sankcje, odwołania, GDPR | 5–6 | 3–4 | 2027-01-11 | Akceptacja panelu konta; zintegrowane GC1–GC3 |
| M4 | CMS, treści, launcher, RBAC, audyt, moderacja GM | 3–4 | 2–3 | — | Lista parytetu treści względem Laravela; działają broadcast, kick, mute, namelock |
| M5 | Zarządzanie serwerami, multi-host, dark prod | 5–7 | 3–4 | — | Akceptacja G; nocny import zielony |
| **MA** | **Import Atlasa, silnik `oteryn-map`, pipeline wydań treści, port `atlas-publish` do Rusta, oracle, harness e2e, CDN, trasy, dekodowanie w workerze, stronicowanie tekstur, animacja, nazwane warstwy, przygotowanie wycofania** | **9–11** | **5–7** | **2027-03-15** | X.10 |
| M6 | Katalog, wiki z osadzonymi mapami, podgląd stroju, polska nakładka | 5–6 | 3–4 | 2027-05-03 | Akceptacja W |
| M7 | Wszystkie narzędzia gracza, mapy, miejsca polowań | 4–5 | 2–3 | — | Akceptacja T |
| M8 | Handel: Stripe, zakup premium, dostarczanie sklepu, prezenty, faktury, KSeF, zwroty, rekoncyliacja | 5–6 | 3–4 | 2027-06-14 | Tryb testowy end to end; dzienna rekoncyliacja dokładna; środowisko testowe KSeF przyjmuje faktury; checkbox prawa konsumenckiego i regulamin na miejscu |
| M9 | Bazar, transfer świata, aukcje i zarządzanie domami, zarządzanie gildiami z weba | 5–7 | 3–4 | — | Każda saga kończy się i kompensuje przy błędzie; escrow w ledgerze bilansuje się do zera |
| M10 | GM observe na `oteryn-map` (web + klient), DPIA i przegląd prawny | 3–4 | 2 | — | Akceptacja O |
| M11 | Hardening, zewnętrzny przegląd bezpieczeństwa, testy obciążenia i GPU, 3 próby przełączenia + 1 odwrotna, UAT, runbooki | 3 | 2–3 | — | Bramka 11.3 |
| **Razem** | | **60–75 tyg.** | **36–45 tyg.** | | **Przełączenie w tygodniu 2027-09-13** przy 2 developerach; tydzień 2028-04-10 przy 1 |

### 11.2 Deliverable'y gry (repo Oteryn-Game)

Każda pozycja jest zobowiązana i zaplanowana, z bramką akceptacji. Terminy „potrzebny do” wynikają z kalendarza dla 2 developerów platformy. Dwóch developerów gry startuje 2026-11-02.

| Kolejność | ID | Deliverable | Potrzebny do | Nakład | Bramka akceptacji |
| --- | --- | --- | --- | --- | --- |
| 1 | GC6 | Wyciągnięcie crate'ów `platform-wire`, `catalog`, `game-rules`; toolchain 1.99.1; CI na ubuntu-26.04; łatka tokio 1.53.3; tag `contracts-v1.0.0` | Koniec M0 | 1–2 tyg. | architecture-check zielony; testy gry bez zmian; tag zużyty przez Platform-2 |
| 2 | GC1 | Producent Public Projections v1, wszystkie 7 rodzin | M3 (2027-01-11) | 2–3 | Zestaw zgodności; opóźnienie najwyżej 10 s |
| 3 | GC2 | Komendy postaci v2: Create v2, Rename, Request/CancelDeletion, SetVisibility, konsument TransferCharacterOwnership, TransferCharacterWorld | M3; części transferu i świata do M9 | 4–6 | Przetestowana każda para ścieżek COMMITTED i REJECTED |
| 4 | GC3 | Account Character Details v1 + postęp | M3 | 2–3 | Tylko właściciel; watermark; lista pokrycia pól |
| 5 | GC4 | Operator Commands v1: Broadcast, Kick, Mute, Namelock, SaveAndShutdown, Start/EndObserve; pełny zapis przy SIGTERM | M4 | 2–3 | Odliczanie end to end; zapis zweryfikowany po SIGTERM |
| 6 | GC7 | Wiele węzłów i kanałów na jednym hoście i między hostami | M5 | 2–3 | ch-A, ch-B, ch-C działają razem |
| 7 | **GC16** | **Atlas World Export v1:** natywny `content/world` + projekcja 15.30 w Rust `tools/atlas-world-export`; profile stworzeń; role NPC; profil wyszukiwania; fazy/czasy animacji; warstwa nazwanych obszarów; katalog polowań; tygodniowe agregaty farm z supresją; manifest wydania treści; fixture kwalifikacyjny Thais z7; kontrakt eksportu ustawiony na ACCEPTED | MA (2027-03-15) | 4–6 | Podpisana paczka dla bieżącego wydania; deterministyczna (dwa przebiegi identyczne); reprezentowane jest każde rozmieszczenie z `content/world`; brak wejścia legacy w ścieżce produkcyjnej; fixture opublikowany |
| 8 | GC5 | Catalog Export v2 + Bundle v1 (bez kafelków) + eksporter Rust + licencje + `release-content.yml` | MA (2027-03-15) | 3–5 | Wszystkie capabilities `supported + complete`; kontrola krzyżowa Pythona się zgadza; manifest wiąże GC5 i GC16 |
| 9 | GC8 | Wyciągnięcie game-rules + `RULES_ABI` + wektory (ADR wyciągnięcia logiki) | M6 (2027-05-03) | 2–3 | Serwer używa crate'a; wektory opublikowane |
| 10 | GC11 | Wypełnienie zestawów reguł i treści (forge, stamina, umiejętności, obrażenia, imbuementy, doomforging, npc-trade, taski, bestiary) | M7 | 6–8 | Każda ścieżka używana przez narzędzia wypełniona i gotowa do runtime |
| 11 | GC9 | Emisja `ANALYSER_V1` + Client Session Export v1 (przycisk eksportu w `apps/client`) | M7 | 2–3 | Import analizatora przechodzi pełną rundę; ślad ma pozycje z gry |
| 12 | GC13 | Store Delivery v1 (`DeliverStoreItems` + potwierdzenie, inbox) | M8 (2027-06-14) | 2–3 | Idempotentne; zgadza się z zamówieniami |
| 13 | GC12 | Implementacja GUILD-0 + Guild Commands v1 (create, invite, accept, ranks, kick, leave, disband) | M9 | 4–6 | Projekcja i komendy end to end |
| 14 | GC14 | House Commands v1 (rozliczenie aukcji z obciążeniem banku, zwolnienie, transfer) + saldo banku w GC3 | M9 | 2–3 | Aukcja się rozlicza albo przechodzi do następnego oferenta |
| 15 | GC10 | GM Observe v1, feed i klient (`OBSERVE_V1`, sesja bez aktora, filtr PM, HUD obserwatora); encje klatek niosą `content_key` albo referencję appearance oraz pozycję | M10 | 6–9 | Akceptacja O |
| 16 | GC15 | Przypięte wydanie prod do przełączenia ze wszystkim powyższym, w tym jego wydaniem treści | M11 | 1 | Podpisane wydanie + wydanie treści; zgodność względem tego digestu |

Łączny nakład po stronie gry to ok. 44–66 tygodni pracy developera. Dwóch developerów gry startujących 2026-11-02 daje ok. 90 tygodni pracy przed startem M11, więc każda pozycja ląduje co najmniej 2 tygodnie przed terminem „potrzebny do”.

### 11.3 Bramka przełączenia (wszystko musi być zielone)

1. **Parytet.** Każda funkcja z sekcji Moduły produktu, Zarządzanie serwerami gry, Wiki z katalogu gry, Narzędzia dla gracza, Oteryn-Atlas i wspólny silnik mapy, Panel konta i postaci oraz Podgląd sesji gracza przez GM działa na preprodzie v2 i dark prod. **Lista parytetu Atlasa (X.10) jest podpisana.**
2. **Kontrakt.** Zgodność wynosi 100% względem GC15, a wektory wire i reguł przechodzą. Wydanie treści GC15 przechodzi weryfikator w Rust, oracle w Pythonie, kontrolę równoważności kompilatorów oraz kontrole krzyżowe współrzędnych i tożsamości.
3. **Dane.** 3 próby przełączenia + 1 import odwrotny. 14 nocnych importów dark prod z zerem różnic. Zmierzone okno ≤60 minut.
4. **Bezpieczeństwo.**
   - Zewnętrzny przegląd auth, OIDC, płatności, supervisora węzła, observe i **klas prywatności nakładek mapy**.
   - Fuzzing czasowników supervisora.
   - Czysty cargo-deny.
   - Zweryfikowane CSRF, limity i tryb sudo.
   - Zweryfikowana legacy argon2, odszyfrowanie 2FA i wyszukiwanie po keyed-hashach na snapshocie prod.
   - CSP bez inline script.
5. **Prawo.** Zatwierdzenie prawnika dla regulaminu, polityki prywatności, DPIA, oceny uzasadnionego interesu dla observe, regulaminu sklepu (odstąpienie konsumenta dla treści cyfrowych) oraz faktur/KSeF. **Publikacja sprite'ów i pikseli podąża za wartością domyślną właściciela albo jej zmianą; atesty praw Atlasa są w `docs/atlas/provenance/`.**
6. **Wydajność.**
   - `/internal/v1` p99 poniżej 250 ms przy dwukrotnym oczekiwanym obciążeniu.
   - Strony publiczne p95 poniżej 300 ms.
   - Narzędzia p95 poniżej 200 ms.
   - p99 gatewaya nie gorsze niż gatewaya Go.
   - Pierwsza klatka overview `/atlas` poniżej 2,5 s przy zimnym ładowaniu 50 Mbit/s (profil z throttlingiem w Playwright).
   - Czas klatki CPU w profilu referencyjnym p95 poniżej 16 ms na `molehill-gpu`.
   - Przyrostowy build wydania treści poniżej 45 minut na vps-2.
7. **Operacje.**
   - Ćwiczenia odtworzenia zaliczone.
   - Alerty działają: opóźnienie projekcji, wiek komend, błędy admission, heartbeat agenta, dysk, błędy webhooków płatności, błędy KSeF, błąd budowy, błąd kontroli krzyżowej, publikacja powyżej 20 GB i 5xx z CDN.
   - Runbooki napisane. Staff przeszkolony.
   - Przekierowania osobnego Atlasa przechodzą względem crawla.
8. **Produkt.** Ciągi UI i nakładka nazw katalogu w 100% w EN i PL, w tym UI Atlasa i nazwy obszarów. SPF/DKIM/DMARC przechodzą. UAT właściciela podpisany. Dowody wizualne zrecenzowane dla każdej powierzchni mapy.

## Ryzyka i decyzje właściciela (z domyślnymi)

| Ryzyko / decyzja | Jak ograniczamy / wartość domyślna |
| --- | --- |
| Pojemność po stronie gry jest ścieżką krytyczną (GC1–GC16, 44–66 tygodni pracy developera) | 2 developerów gry od 2026-11-02; terminy „potrzebny do” 2 tygodnie przed użyciem; cotygodniowy przegląd kontraktów; zestaw zgodności jako wspólna definicja ukończenia |
| **GC16 zmienia źródło Atlasa z legacy OTBM/15.32 na natywne `.b3`/15.30**, więc mapy mogą różnić się od dzisiejszego osobnego Atlasa | Zrecenzowany raport różnic regionów w MA; każda różnica przypisana do natywnej treści; gra odpowiada za poprawki w `content/world` |
| **Wierność portu publikacji z Pythona do Rusta** | Stałe zadanie równoważności bajt w bajt z kompilatorem Pythona Atlasa i niezależny oracle w Pythonie na każdym wydaniu |
| **Rozmiar publikacji i czas budowy na 4 OCPU ARM** | Build przyrostowy kluczowany ID treści; buduj raz i promuj; scratch 60 GB; alerty 20 GB i 45 minut; rozmiar mierzony w tygodniu 2 MA |
| **Pamięć GPU na mobile i pokrycie WebGL2** | Stronicowanie tekstur po bucketach; dekodowanie semantyczne w workerze; fallback SSR ze statycznym cropem; scenariusze WebKit mobile w `e2e.yml` |
| **Silnik mapy jako ścieżka wycieku prywatności** (nakładki właściciela i staffu) | Klasy prywatności egzekwowane na serwerze i kliencie; `no-store`; nigdy w CDN ani publikacji; objęte zewnętrznym przeglądem |
| **Dryf współrzędnych lub tożsamości między katalogiem a Atlasem** | Jeden manifest wydania treści; kontrole krzyżowe aktywacji odrzucają każdą niezgodność; atomowe wspólne przestawienie i rollback |
| **Niezgodność governance** (ścisła weryfikacja Atlasa a szybkie CI platformy) | D21: zero ponowień, niezależny oracle, zrecenzowane dowody wizualne i Merge Queue na wszystkich ścieżkach mapy |
| **Nieznane nazwy hostów i linki głębokie osobnego Atlasa** | Inwentarz i crawl w tygodniu 1 MA; mapa przekierowań sprawdzana w CI; Synology działa bez trasy do T+14 d na potrzeby rollbacku |
| **Autorytet topologii** (ADR 0001 w META) | Poprawka META scalona w M0 |
| Wielkie przełączenie | 14 nocnych importów dark prod, 3 próby, import odwrotny, okno 60 minut, obiektywne wyzwalacze rollbacku |
| Zakres: 36–45 tygodni przy 2 developerach | Stałe bramki; każdy kamień milowy wdrożony i pokazany w demie; brak równoległych gałęzi funkcji starszych niż jeden kamień milowy |
| IP sprite'ów (grafika Tibia 15.30) | Wartość domyślna właściciela to publikacja; istnieje wstrzymanie per asset; atesty przeniesione; przegląd prawnika w bramce |
| Płatności, podatki i prawo (VAT, KSeF, odstąpienie konsumenta) | Stripe; faktury budowane wg polskich reguł; KSeF testowany w jego środowisku testowym; zatwierdzenie prawnika w bramce |
| GM observe: nadużycie przez staff, spór prawny lub RODO, wyciek obecności obserwatora, ruch feedu, nagrania z danymi osób trzecich | Cztery oczy, linki do spraw, nagrania, audyt z łańcuchem hashy, miesięczny raport dla właściciela, DPIA i przegląd prawnika. Tryb klienta to sesja bez aktora, a tryb feed działa w procesie gry i nic nie wysyła do klientów; test bajt w bajt strumienia celu. Dedykowany certyfikat i pula, paczkowanie 1 Hz, limit współbieżności. Minimalizacja, zaczernianie w eksportach, logowanie dostępu i retencja nagrań. |
| Agent węzła jako droga eskalacji | Zamknięte czasowniki, podpisane komendy, obrazy podpisane przez wydanie, supervisor bez sieci, agent bez uprawnień, limity |
| Regresja rustowego gatewaya | Zgodność bajt w bajt z fixture'ami gatewaya Go; test obciążenia; rollback (a) przywraca gateway Go |
| Przejęcie żywego węzła przez supervisora przy przełączeniu | Port semantyki `game-supervisor.py` jeden do jednego, test na stosie testgame, blokada przełączenia flagą reconciliation |
| Dryf reguł między grą a webem | Wspólny crate, `RULES_ABI`, wektory uruchamiane ponownie przy każdej aktywacji |
| Obciążenie tłumaczeniem (nakładka PL) | Obsadzona praca w M6 z panelem pokrycia; bramka na 100% |
| Pojemność hosta i dysk | Drugi host, alerty monitoringu, retencja partycji, czyszczenie obrazów, object storage na assety |
| Objętość logów w PG | Partycje dzienne, retencja 14 dni, alarmy rozmiaru |
| Nowość PG 19 | Aktualizacja do GA w tej samej wersji głównej wcześnie w M0–M1; 9 miesięcy pracy preprodu i dark prod przed przełączeniem |
| Rozbieżność wendorowanych tokio/sqlx | Identyczne łatki w obu repo, podbijane razem w GC6 i kolejnych wydaniach |
| Niezgodność glibc między builderem a obrazem uruchomieniowym | Builder Ubuntu 26.04 używany wyłącznie z chiselled runtime 26.04 |
| Self-hosted runner na hoście prod | Prywatne repo, efemeryczny runner, chronione środowiska, tylko przypięte digesty |
| Upgrade Ubuntu na oteryn-vps | Osobne okno w tygodniu 1 M0, kopia boot volume, testy smoke gry i Laravela po upgradzie |
| htmx 4 jest nowy (sierpień 2026) | Wendorowany po hashu, minimum JS (poza `oteryn-map`) |
| Migracja etykiety `ubuntu-latest` (19.10–19.11.2026) | Jawne `ubuntu-26.04(-arm)` w obu repo; CI gry schodzi z `ubuntu-24.04` w GC6 |
| Wierność odszyfrowania `encrypted`/APP_KEY | Testy na fixture'ach z obecnego Laravela i próby; APP_KEY w zimnym magazynie po wyłączeniu |
| Dane produkcyjne w środowisku prób (PII) | Środowisko prób niewystawione do internetu, snapshoty kasowane po każdej próbie, dostęp logowany |

Decyzje właściciela z wartościami domyślnymi i terminami zmiany są w sekcji 0. Żadna z nich nie blokuje budowy.

## Dowody dodane w tej rewizji (tylko do odczytu)

**Oteryn-Game:**

- `Cargo.toml` (`[patch.crates-io]`: wendorowane sqlx-core/postgres 0.9.0 i tokio 1.53.1), `rust-toolchain.toml` (1.94.0), `.github/workflows/rust.yml` (`ubuntu-24.04`, `+1.95.0`)
- `tools/game-platform-catalog/README.md`, `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`
- `content/{creatures,items,npcs,loot,quests,world,assets,cosmetics,houses,abilities,charms,services,presentations}/*`
- `rulesets/{character,progression,items,economy,combat}/*`
- `crates/client-assets/src/lib.rs`
- `apps/game-server/migrations/{0019,0020,0021,0056}*`
- `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §4.1–4.2 (wykluczenie ukrytych postaci; brak ostatniego logowania i online w profilu)
- `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (tylko Create/Transfer; płeć, profesja i strój nie są wejściami v1; zmiana nazwy, usunięcie i transfer świata poza zakresem v1)
- `docs/contracts/protocol-oteryn/v1/analyser_v1.proto` oraz kod capability analizatora (dla GM observe)
- `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md` i `tools/game-atlas-fullworld-source/README.md` (origin/main 77af046c)
- `workspace-boundaries.toml`, `apps/game-server/src/native_admission_source/descriptor.rs`

**Oteryn-Platform:**

- `deploy/oracle/{game-supervisor.py, oteryn-game.service, game-runtime.compose.yml, public-platform.compose.yml}`
- `app/GameCatalog/*` (import, validate, diff, activate, rollback, verify)
- `app/Admin/AdminPermission.php` (obecne uprawnienia)
- `routes/internal.php`

**Oteryn-Atlas (shallow clone, 0bbd188):**

- `src/browser/fullworld-runtime-v0.md`, `src/publication/fullworld-v0.md`, `tools/fullworld-publication/publication.py`
- `AGENTS.md` (granica autorytetu), `e2e/package.json` (Playwright 1.62.0)
