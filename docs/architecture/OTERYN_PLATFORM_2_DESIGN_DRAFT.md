# Oteryn Platform 2 (Rust) – projekt architektury

> **Status: DRAFT** (2026-10-15). Projekt do akceptacji przez właściciela; nic tu nie jest jeszcze zobowiązaniem ani kontraktem.
> Wersja robocza na żywo: https://claude.ai/code/artifact/e809ac5a-3f4c-42c2-91b6-b4eaa489b950
> W przygotowaniu (architekt): pełny panel konta i postaci gracza, narzędzia gracza (bestiary tracker, mapa tasków, symulator forge, kalkulatory, analizator polowań), podgląd sesji gracza przez GM-a.

Oteryn Platform 2 to nowa, kompletna platforma w Ruście: strona, konto i postacie gracza, wiki z katalogu gry, administracja i zarządzanie serwerami gry. Powstaje w osobnym repozytorium `Oteryn-Platform-2`, jako jedna binarka z rolami (public, internal, worker, migrate, ops) na axum + askama + htmx 4 + PostgreSQL 19. Kontrakt `/internal/v1` z serwerem gry zostaje bajt w bajt. Budujemy ją obok obecnej platformy, bez przenoszenia po kawałku; gdy będzie kompletna i przetestowana na własnym preprodzie, przełączamy ją w jednym oknie zamiast Laravela.

## Decyzje

| # | Decyzja |
| --- | --- |
| D1 | **Osobne repozytorium `Oteryn-Platform-2`.** Kontrakt gra↔platforma to jeden crate bez I/O, `oteryn-platform-wire`, który żyje w Oteryn-Game (gra jest właścicielem kontraktu). Platform 2 bierze go jako zależność git przypiętą do tagu (np. `platform-wire-v1.0.0`), zapisanego w `CROSS_REPOSITORY_CONTRACT_LOCK.json`. Rozjazd kontraktu zamyka test zgodności: każdy PR odpala przypięty obraz game-servera przeciw v2. |
| D2 | Jedna binarka `oteryn-platform` z rolami `serve-public`, `serve-internal`, `worker`, `migrate`, `ops`. Każda rola to osobny kontener. |
| D3 | Stack: axum 0.8.9 + tower-http 0.7 + askama 0.16 (SSR) + htmx 4.0 + sqlx 0.9 na PostgreSQL 19. Buildy i obrazy na Ubuntu 26.04 LTS. Bez Node i bez SPA. |
| D4 | `/internal/v1/*` zostaje bajt w bajt: ścieżki, kodeki, limity, subjecty certyfikatów i CA. mTLS kończy się w procesie (rustls) zamiast nginx + nagłówki FastCGI. |
| D5 | Gateway w Go zostaje na produkcji. Preprod v2 ma własną instancję gatewaya z tego samego kodu, podpiętą do v2. Port do Rusta dopiero po przełączeniu, z osobnym ADR. |
| D6 | v2 ma własną instancję PostgreSQL 19, osobną od Laravela (PG 17.11) i od każdej bazy gry. Platforma zna dane gry tylko z projekcji pchanych przez grę. |
| D7 | Bez strangler. Platform 2 powstaje jako osobny, kompletny produkt z własną bazą, CA, certyfikatami, adresami, gatewayem i węzłem gry testowej. Laravel działa bez zmian do dnia przełączenia. Jedno przełączenie: import danych, `platform-internal` przejmuje `172.18.0.12:8543` z produkcyjną tożsamością mTLS, Caddy kieruje `oteryn.com` na v2. |
| D8 | Cała integracja z Canary i Player Companion do usunięcia, bez portowania. |
| D9 | Hasła: argon2id; istniejące hashe PHC weryfikowane bez zmian, rehash przy logowaniu. Laravelowy APP\_KEY zostaje jako klucz legacy do HMAC i odszyfrowania sekretów 2FA. |
| D10 | Najszybszy efekt: podgląd v2 pod `next.oteryn.com` (na początku za Cloudflare Access) z natywnymi highscores, online, profilami i zgonami z testowego węzła gry. Bez ryzyka dla produkcji. |
| D11 | Toolchain: v2 startuje na Rust 1.99.1 (MSRV 1.99). Osobny PR podnosi Oteryn-Game z 1.94.0 / 1.95.0 na 1.99.1. |
| D12 | Runnery CI przypięte jawnie do `ubuntu-26.04` / `ubuntu-26.04-arm`, nigdy `ubuntu-latest` (GitHub przenosi ten label na 26.04 między 19.10 a 19.11.2026). |

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
| `game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}` | gra (jeszcze nie zbudowane) | Public Projections v1 |

Limity klienta gry: connect 1 s, handshake 2 s, wymiana 3 s, 2 sloty pipeline. p99 platformy na tych trasach musi być wyraźnie poniżej 1 s. Każdy cel ma własny certyfikat klienta (np. `CN=oteryn-preprod-character-projection`).

Publiczne API dla launchera/klienta (`crates/platform-client`): `/oauth/authorize`, `/oauth/token` (PKCE S256, scope `game:ticket`), `POST /v1/game-auth/tickets`, `/v1/game-auth/device-sessions/*`, `GET /v1/game-auth/native-characters`, `GET /v1/client/directory`.

## Repozytorium

Platform 2 to osobne repo `Oteryn-Platform-2`. Monorepo dałoby atomowe zmiany kontraktu, ale związałoby v2 z ciężkim CI gry, jej regułami nazw i granic, wendorowanymi łatkami tokio/sqlx i jednym rytmem toolchaina. Właściciel chce v2 budować, wydawać i uruchamiać osobno, więc osobne repo wygrywa.

Jak nie dopuścić do rozjazdu kontraktu:

1. `oteryn-platform-wire` zostaje w Oteryn-Game i zawiera tylko typy serde, bez I/O. Game-server re-eksportuje z niego obecne typy bez zmiany zachowania.
2. v2 bierze go jako zależność git przypiętą do tagu; tag jest zapisany w `CROSS_REPOSITORY_CONTRACT_LOCK.json`.
3. Job `conformance.yml` w v2 uruchamia na każdym PR przypięty obraz game-servera przeciw v2 i przechodzi przez każdą rodzinę `/internal/v1` prawdziwym kodem klienta gry.
4. Przed przełączeniem test zgodności idzie jeszcze raz przeciw dokładnemu digestowi obrazu gry z produkcji.

W Oteryn-Game dochodzi tylko crate `oteryn-platform-wire` i jedna nowa krawędź `oteryn-game-server` → `oteryn-platform-wire`. Do naprawy przy okazji: `rust-toolchain.toml` ma 1.94.0, a CI używa `+1.95.0`.

## Architektura

Ten sam obraz uruchamia dwa serwery HTTP i workera; każdy w osobnym kontenerze. `platform-internal` ma własny listener, trust store i limity zasobów, więc ruch publiczny nie zje 1-sekundowego budżetu gry. Przejmuje też IP i port nginx mTLS (172.18.0.12:8543), więc konfiguracje gry i gatewaya się nie zmieniają.

```
                Internet (Cloudflare)
                         |
               +-------------------+
               | caddy edge :443   |
               +---------+---------+
  oteryn.com/*  |        | login./gateway.         | TCP 7172
                v        v                         v
  +------------------+  +------------------+  +-----------+
  | platform-public  |  | game-gateway (Go)|  | game node |
  | serve-public     |  | :8080            |  | (systemd) |
  | :8000 HTTP       |  +--------+---------+  +-----+-----+
  +--------+---------+           | bearer          | klient mTLS
           |                     v                 v
           |          +-----------------------------------+
           |          | platform-internal :8543 (rustls)  |
           |          | serve-internal, mTLS w procesie   |
           |          +----------------+------------------+
           |                           |
  +--------v---------------------------v-----+   +------------------+
  | platform-postgres 17 (tylko platforma)   |<--| platform-worker  |
  +------------------------------------------+   | joby/outbox/mail |
                                                 +------------------+
  game-postgres: osobna instancja, platforma nigdy się nie łączy
```

Worker obsługuje: e-maile, wysyłkę outboxa, czyszczenie retencji, finalizację usunięć kont, alarmy świeżości projekcji, później bazar i webhooki płatności.

| Przepływ | Kierunek | Mechanizm |
| --- | --- | --- |
| Admission | gateway → platform-internal | HTTP + bearer serwisowy |
| Evidence, status, scope | gra → platform-internal | HTTP/1.1 mTLS, certyfikat per cel |
| Komendy postaci | gra **pobiera** pending | Polling co 1 s, idempotentne potwierdzenia |
| Projekcje (LCFA, publiczne, premium) | gra **pcha** snapshoty + watermarki | Upsert monotoniczny po rewizji |
| Nowe: komendy operatora (broadcast, kick, mute, maintenance) | gra **pobiera** `/internal/v1/operator-commands/pending` | Ta sama kolejka co komendy postaci |
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
| OAuth | Własny minimalny serwer OAuth2: tylko authorization code + PKCE S256, tokeny hashowane, te same ID klientów i scope. OIDC w P7. |
| mTLS | rustls 0.23 (aws-lc-rs), tylko TLS 1.3, osobne CA klientów, allowlista DN + SPKI, przeładowanie certyfikatów na SIGHUP. |
| Sekrety | Pliki w `/run/secrets/*`, tryb 0400, nigdy zmienne środowiskowe; loader odrzuca pliki czytelne dla grupy lub wszystkich. |
| Audyt | Tylko dopisywana tabela `audit_events` (aktor, peer, akcja, cel, digest przed/po, request ID, IP), łańcuch hashy per dzień. |
| Nagłówki | Restrykcyjne CSP (bez inline JS), HSTS, frame-ancestors none, Referrer-Policy. |
| Łańcuch dostaw | `cargo deny` / `cargo audit` w CI, przypięte zależności, SBOM do obrazów. |

**i18n:** zostają adresy `/{locale}/...` z `en` i `pl`; `/` wybiera język z Accept-Language, potem ciasteczka, domyślnie `en`. Tłumaczenia w Fluent (`locales/{en,pl}/*.ftl`) — ważne dla polskich form mnogich. `lang/en`, `lang/pl` i `pl.json` konwertuje jednorazowy skrypt. CI odrzuca brakujące klucze w którymkolwiek języku.

## Stack technologiczny

Wszystko w najnowszych stabilnych wersjach na 15.10.2026 (sprawdzone na crates.io, Docker Hub, blogu Rusta, postgresql.org i w changelogu GitHuba).

| Warstwa | Wybór | Wersja | Uwagi |
| --- | --- | --- | --- |
| Język | Rust | **1.99.1** (MSRV 1.99) | Gra jest na 1.94.0 (plik) / 1.95.0 (CI); podbicie gry osobnym PR (D11). |
| System (build, obrazy, runnery) | Ubuntu | **26.04 LTS** | Runnery `ubuntu-26.04-arm`, przypięte jawnie (D12). |
| Baza | PostgreSQL | **19** (dziś RC1) | Rozwój na 19 RC; przełączenie tylko na 19.x GA; awaryjnie 18.6 bez zmian w kodzie. |
| HTTP | axum + tower-http | 0.8.9 + 0.7.1 | axum 0.9 nie istnieje. |
| Szablony SSR | askama | 0.16.1 | Szablony kompilowane, błędy w czasie builda. |
| Interakcja w przeglądarce | htmx | **4.0.x** | Plik wendorowany po hashu (npm `latest` to wciąż 2.x). Bez Node i bez SPA. |
| Dostęp do bazy | sqlx | 0.9.0 | Zapytania sprawdzane offline (`sqlx prepare --check`). |
| TLS / mTLS | rustls (aws-lc-rs) | 0.23.46 | mTLS w procesie zamiast nginx. |
| Runtime async | tokio | 1.53.3 | Osobny lockfile od gry, więc łatka gry na 1.53.1 nie dotyczy v2. |
| HTTP klient | reqwest | 0.13.5 |  |
| Hasła | argon2 | 0.6.0 | Nowa generacja RustCrypto; duplikaty crate'ów krypto dopuszczone w `cargo deny`. |
| 2FA | totp-rs | 6.0.0 |  |
| Poczta | lettre | 0.11.23 |  |
| Tłumaczenia | fluent-templates | 0.15.1 | Pliki `.ftl` en/pl. |
| Obraz uruchomieniowy | Chiselled Ubuntu 26.04 | — | Awaryjnie `ubuntu:26.04` read-only, non-root. |
| Edge | Caddy | 2.11.7 | Istniejący kontener, tylko nowe bloki. |

## Moduły produktu

Pełna strona do zarządzania serwerem, kontem i postaciami, w kolejności faz. Kolumna „Zależność w grze” pokazuje, gdzie potrzebna jest praca po stronie serwera.

| Moduł | Zawartość | Faza | Zależność w grze |
| --- | --- | --- | --- |
| Obserwowalność | `/metrics`, `/healthz`, `/readyz`, opóźnienia projekcji, wiek komend, histogramy admission | P0 | brak |
| Natywne strony gry | Highscores, profil postaci, zgony, online, domy, strony gildii | **P1** | **Producent Public Projections v1 (nowe)** |
| Strona publiczna | Home, newsy, strony/regulaminy, pobieranie, eventy, ogłoszenia, status serwera | P1 (status) / P3 | Runtime status (jest) |
| Konto | Rejestracja, weryfikacja maila, logowanie, 2FA, reset hasła, sesje/urządzenia, zmiana maila, usunięcie konta | P2 | Evidence (jest) |
| Postacie | Tworzenie, lista, stan komend; później zmiana nazwy/usunięcie/transfer świata | P2 / P6 | CreateCharacter (jest); Rename/Delete/Transfer (nowe) |
| Game auth | OAuth PKCE, tickety, device sessions, directory, admissions, evidence, keyring, topologia | P2 | jest |
| Panel admina/GM (minimum) | Dashboard, użytkownicy, ban (= `disabled_at` + generation), RBAC, audyt | P2 | jest |
| Launcher/updater | Wydania klienta, podpisane generacje updatera, manifest kanału | P3 | brak |
| Wiki | Czytanie, redakcja, rewizje, media | P3 | Import katalogu z `tools/game-platform-catalog` |
| Katalog gry | Import / diff / walidacja / aktywacja / rollback | P3 | Narzędzie katalogu (jest) |
| Support | Zgłoszenia, raporty graczy, sankcje, odwołania, retencja | P4 | Operator Commands v1 (nowe) |
| Panel GM (pełny) | Sankcje, broadcast, kick, mute, namelock, maintenance (route policy → ROUTE\_UNAVAILABLE), operacje świata | P4 | Operator Commands v1 (nowe) |
| RBAC | 23 obecne uprawnienia 1:1 + `gm.*`, `ops.*`, `ledger.*` | P2 / P4 | brak |
| Sklep/płatności | Katalog, zamówienia, adapter providera, webhooki, ledger portfela, zakup premium | P5 | Snapshot premium (jest); dostawa itemów (nowe, opcjonalne) |
| Bazar | Natywna aukcja postaci z depozytem w ledgerze → komenda TransferCharacterOwnership | P6 | Konsument Transfer w grze (nowe) |
| Gildie/domy z weba | Najpierw widok z projekcji, potem komendy zarządzania | P7 | GUILD-0 + komendy gildii (nowe) |
| OAuth/OIDC | PKCE teraz; dostawca OIDC (discovery, JWKS, id\_token) dla forum/Discorda | P2 / P7 | brak |

Do usunięcia bez portowania: źródło danych Canary, `IdentityCanaryAccount`, bazar Canary, parser logów Player Companion, system szablonów strony głównej (zastąpiony stałym układem z blokami CMS).

## Struktura katalogów

Układ crate'ów Platform 2. Po wydzieleniu \`apps/platform\`, \`crates/platform-\*\`, \`tools\`, \`tests\` i \`deploy\` leżą w korzeniu repo Oteryn-Platform-2; w Oteryn-Game zostaje tylko \`crates/platform-wire\` i zmiana w game-serverze. `platform-security` jest osobno, żeby powierzchnię kryptograficzną audytować niezależnie. `platform-internal-api` zależy tylko od `platform-wire`, `-domain`, `-db`, `-security`, a `platform-web` nie może go importować (wymuszone w `[edges]`).

```text
Oteryn-Platform-2/
├── Cargo.toml                         # workspace, rust-version = "1.99"
├── rust-toolchain.toml                # channel = "1.99.1"
├── deny.toml
├── docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json   # tag oteryn-platform-wire
├── .github/workflows/{ci.yml, conformance.yml, image.yml, deploy.yml}   # ubuntu-26.04-arm
├── apps/
│   └── platform/
│       ├── Dockerfile                 # multi-stage arm64, chiselled Ubuntu 26.04
│       └── src/
│           ├── main.rs                # clap: serve-public | serve-internal | worker | migrate | ops
│           ├── config.rs              # platform.toml + /run/secrets (sprawdza uprawnienia)
│           ├── roles/{public.rs, internal.rs, worker.rs, migrate.rs}
│           └── ops/{game_auth.rs, catalog.rs, identity.rs, mail.rs, support.rs, wiki.rs}
├── crates/
│   ├── platform-domain/               # czysta logika, bez I/O
│   │   └── src/{identity/, auth/, characters/, registry/, premium/, ledger/, content/,
│   │            wiki/, support/, rbac/, sanctions/, shop/, bazaar/, i18n.rs}
│   ├── platform-db/
│   │   ├── migrations/                # 0001_identity.sql ... (sqlx, PG 19)
│   │   └── src/{pool.rs, repo/<moduł>.rs, outbox.rs, jobs.rs, projections.rs}
│   ├── platform-security/             # argon2, totp, AEAD key ring, odczyt legacy Laravela,
│   │   └── src/                       # LegacyKeyedHash, keyring Ed25519, CSRF, sesje
│   ├── platform-internal-api/         # axum /internal/v1, akceptor mTLS rustls
│   │   └── src/{tls.rs, peer.rs, routes/<rodzina>.rs}
│   ├── platform-web/
│   │   ├── src/{router.rs, middleware/, handlers/<moduł>.rs, oauth/, api_v1/}
│   │   ├── templates/{layout/, public/, account/, characters/, game/, wiki/, support/, admin/}
│   │   ├── static/{css/, js/htmx-4.0.x.min.js, img/}
│   │   └── locales/{en,pl}/*.ftl
│   └── platform-worker/
│       └── src/{runner.rs, jobs/{mail, outbox, retention, terminations,
│                                 projection_alarms, payments}.rs}
├── tools/
│   └── legacy-import/                 # Laravel PG 17 -> v2 PG 19: import | verify | reverse
│       └── src/{main.rs, laravel_crypt.rs, map/<tabela>.rs, verify.rs, reverse.rs}
├── tests/
│   └── conformance/                   # przypięty obraz game-servera vs platform-internal
└── deploy/
    └── oracle/
        ├── compose.base.yml           # public, internal, worker, migrate, ops, postgres:19
        ├── compose.v2-preprod.yml     # sieć 172.19.0.0/24, CA v2, gateway-v2, game-v2
        ├── compose.production.yml     # platform-internal na 172.18.0.12:8543, prod CA
        ├── caddy/{next.caddy, cutover.caddy}
        ├── secrets/README.md          # tylko nazwy i uprawnienia, bez wartości
        └── scripts/{deploy.sh, backup.sh, restore-drill.sh, rehearse-import.sh,
                     cutover.sh, rollback.sh}

Oteryn-Game/ (zmiany)
├── crates/platform-wire/              # NOWY kontrakt bez I/O, tag platform-wire-v1.0.0
│   └── src/{lib.rs, paths.rs, bounds.rs, errors.rs, evidence.rs, admission.rs,
│            account_characters.rs, character_commands.rs, operator_commands.rs,
│            runtime_status.rs, scope.rs, restore.rs, premium.rs, public_projections.rs}
└── apps/game-server/                  # re-eksport typów z platform-wire, bez zmiany zachowania
```

## Schemat bazy platformy

Jedna baza PostgreSQL tylko dla platformy, podzielona na schematy. ID kont, klientów OAuth, światów i kanałów zachowane z Laravela.

| Schemat | Główne tabele |
| --- | --- |
| `identity` | identities, credentials, two\_factor, recovery\_codes, email\_verifications, password\_resets, web\_sessions, security\_state (generation, disabled\_at, terminated\_at), terminations |
| `auth` | oauth\_clients, oauth\_codes, oauth\_tokens (hashowane), device\_sessions (legacy keyed hash + wersja), game\_tickets (sha256), admission\_keys\_meta, evidence\_floors, evidence\_witness |
| `registry` | worlds, channels, routes, route\_policies (maintenance), topology\_generations, directory\_epochs |
| `characters` | command\_intents, command\_receipts, bootstrap\_intents, account\_character\_projection, projection\_watermarks |
| `game_public` | highscores, character\_profiles, character\_deaths, houses, guilds, world\_online, family\_watermarks |
| `premium` | premium\_ledger, premium\_snapshots |
| `commerce` | products, prices, orders, payments, provider\_events, ledger\_accounts, ledger\_transactions, ledger\_entries |
| `content` | news (+i18n), pages (+i18n), events, announcements, downloads, client\_releases, updater\_generations, media |
| `wiki` | articles, revisions, categories, review\_queue, catalog\_versions, catalog\_items |
| `support` | tickets, messages, reports, enforcement\_records, appeals |
| `staff` | roles, permissions, role\_permissions, staff\_assignments |
| `ops` | audit\_events, outbox, jobs, operator\_commands, operator\_receipts |
| `bazaar` (P6) | auctions, bids, escrow\_holds |

## Deploy na oteryn-vps

### CI i obraz

- Buildy na hostowanych runnerach GitHuba `ubuntu-26.04-arm` (natywne ARM64, jak Oracle Ampere). VPS tylko pobiera obrazy.
- Workflowy: `ci.yml` (fmt, clippy `-D warnings`, testy z PG 19, `sqlx prepare --check`, `cargo deny`, zgodność kluczy i18n), `conformance.yml` (przypięty obraz gry + v2 + gateway), `image.yml` (obraz arm64, SBOM, podpis cosign, push do ghcr po SHA), `deploy.yml` (pull po digest, migracje, compose up).
- Nowy runner na oteryn-vps z etykietami `oteryn-vps,v2`: osobny użytkownik `gh-runner-v2`, kontener `ubuntu:26.04`, tryb `--ephemeral`, dostęp tylko do `/opt/oteryn-v2/**` przez ograniczony skrypt sudoers. Runner na Synology znika przy przełączeniu.
- Obraz: builder `ubuntu:26.04` + Rust 1.99.1 (`cargo build --release --locked`, `SQLX_OFFLINE=true`); rootfs z `chisel cut --release ubuntu-26.04` (libc6, libgcc-s1, ca-certificates, base-files, tzdata); finalnie `FROM scratch`, `USER 65532`. Awaryjnie `ubuntu:26.04` read-only. Gotowość sliceów chisel dla 26.04 do sprawdzenia przed M0.
- Dysk: trzy ostatnie digesty, `docker image prune` po deployu, logi `max-size=10m`.

### Środowiska na jednym hoście

| Środowisko | Projekt compose | Adresy | Baza | Gra | mTLS |
| --- | --- | --- | --- | --- | --- |
| prod (obecny) | `oteryn-oracle`, `oteryn-oracle-runtime` | oteryn.com, login., gateway., game.oteryn.com:7172 | Laravel PG 17.11 | prod (systemd) | prod CA, nginx na `172.18.0.12:8543` |
| v2-preprod | `oteryn-v2` | `next.oteryn.com`, `login-next.oteryn.com`, `game-next.oteryn.com:7272` | `platform2-postgres` (PG 19) | `game-v2` z producentem Public Projections + `game-v2-postgres` | CA v2, `platform-internal` na `172.19.0.12:8543` |
| v2-rehearsal (czasowo) | `oteryn-v2-rehearsal` | brak publicznych (tylko tunel SSH) | odtworzony snapshot prod → import | odtworzony snapshot bazy gry | CA v2 |

- Sieć v2: `oteryn-v2_private` (`172.19.0.0/24`), baza testowej gry na `oteryn-v2_game` (`172.30.85.0/24`). Kontenery: `platform-public`, `platform-internal`, `platform-worker`, `platform-postgres`, `gateway-v2`, `game-v2`, `game-v2-postgres`.
- Węzeł testowy gry ma własną CA (`Oteryn v2 Preprod Internal CA`) i certyfikaty `CN=oteryn-v2-<cel>`. Przy przełączeniu `platform-internal` ładuje produkcyjną CA, certyfikat i listę DN — to zmiana konfiguracji, nie kodu.
- Caddy dostaje tylko nowe bloki dla `next.` i `login-next.` oraz trasę TCP na 7272; istniejące bloki się nie zmieniają. `next.` za Cloudflare Access, dopóki właściciel go nie otworzy.
- `game-next` działa jak prod: systemd + `game-supervisor.py`, osobna jednostka `oteryn-game-v2.service`.
- v2-preprod nigdy nie czyta danych prod, poza próbami importu w zamkniętym środowisku rehearsal.
- Zasoby: przed M0 sprawdzić kształt A1. Jeśli RAM < ok. 12 GB, węzeł testowy gry chodzi tylko w oknach testów, a usługi platformy mają twarde `mem_limit` (albo drugi VPS A1 na preprod).

### Sekrety i kopie

- Sekrety w `/opt/oteryn-v2/secrets/<env>/`, montowane read-only jako Docker secrets, 0400, uid 65532. Sekrety produkcyjne (klucze Ed25519 admission, certyfikat i klucz serwera, CA, hashe bearerów gatewaya, legacy APP\_KEY) trafiają do `/opt/oteryn-v2/secrets/production/` dopiero w oknie przełączenia.
- PG 19 z archiwizacją WAL i nocnym base backupem poza maszynę (Object Storage). Test odtworzenia co tydzień od M1.

## Zarządzanie serwerami gry

Z panelu admina: lista hostów, światów, kanałów i węzłów, status i zdrowie na żywo, logi (podgląd na żywo i wyszukiwanie), start, stop i restart, maintenance, zapowiadany restart lub wyłączenie z odliczaniem, wdrożenie nowej wersji gry, dodanie kanału lub świata.

### Model: agent + supervisor (pull)

Platforma nigdy nie łączy się z hostem — tak jak gra, to host pyta platformę. Kto ma dostęp do socketu Dockera, ma roota, więc komponent sieciowy nie dotyka Dockera.

| Komponent | Działa jako | Sieć | Co może |
| --- | --- | --- | --- |
| `oteryn-node-agent` | użytkownik `oteryn-agent` | tylko wychodząca, mTLS do `platform-internal` | Pobiera komendy (long-poll), sprawdza podpis i lokalną politykę, woła supervisor, czyta journald, wysyła logi i heartbeat. |
| `oteryn-node-supervisor` | root, systemd, bez sieci | brak | Wykonuje tylko zamkniętą listę akcji poniżej. Zastępuje `game-supervisor.py` (port jego maszyny stanów, w tym `reconciliation-required` i zakaz ponownego użycia zużytego pliku startowego). |

Agent i supervisor rozmawiają przez socket `/run/oteryn-supervisor.sock` (`root:oteryn-sup 0660`, sprawdzanie `SO_PEERCRED`, ramki ≤ 64 KiB).

Nowe endpointy (agent → platforma): `POST /internal/v1/node-agent/commands/poll` (long-poll 25 s), `.../commands/receipt`, `.../heartbeat` (co 5 s), `.../logs` (paczki zstd ≤ 1 MiB lub co 2 s). Jednorazowa rejestracja agenta: `POST /v1/node-agent/enroll` (token 15 min + CSR → certyfikat).

### Dozwolone akcje (pełna lista)

| Akcja | Argumenty | Uwagi |
| --- | --- | --- |
| `Status` | — | Jednostki, kontenery `oteryn-*`, digesty obrazów, dysk, RAM |
| `StartNode` | node\_id | Przez istniejący flow autoryzacji startu, potem `compose up` |
| `StopNode` | node\_id, graceful | SIGTERM, 60 s; zapis „planowany stop” |
| `RestartNode` | node\_id | Stop + start jako jedna zmiana inkarnacji |
| `ApplyNodeSpec` | świat, kanały, port 7172–7199, digest obrazu, limity, rewizja | Renderuje `node.toml` i compose z szablonów supervisora; żadnych dowolnych kluczy, ścieżek, argumentów |
| `PullImage` | digest `ghcr.io/oteryn/oteryn-game@sha256:…` | Tylko jeśli podpisany manifest wydania zawiera ten digest |
| `RunMigrations` | node\_id, digest | `oteryn-game-migrate` z tego samego obrazu |
| `RotateNodeCertificate` | node\_id (CSR) | Klucz generowany na hoście |
| `RemoveNode` | node\_id | Zatrzymuje i archiwizuje konfigurację; nigdy nie usuwa danych gry |
| `CollectDiagnostics` | node\_id | `compose ps`, ostatnie 1000 linii logów, `docker inspect` z ukrytym env |

Niemożliwe z poziomu www: shell/exec w kontenerach, dowolne obrazy, mounty, zmienne środowiska, porty spoza zakresu, pliki hosta, pakiety, klucze SSH, firewall, zapytania do bazy gry, kasowanie wolumenów i kopii, aktualizacja samego agenta/supervisora.

### Bezpieczeństwo komend

1. Podgląd (cel, wpływ, liczba graczy online, dokładna akcja) → potwierdzenie z ponownym 2FA (5 min) i wpisaniem nazwy celu.
2. Na produkcji zasada czterech oczu dla stopu, wyłączenia świata, wdrożenia, usunięcia węzła i tworzenia świata. Na preprodzie jeden zatwierdzający.
3. Koperta komendy podpisana Ed25519 osobnym kluczem ops (nie kluczem admission), ważna ≤ 120 s, z rosnącym numerem.
4. Agent sprawdza podpis, host, ważność, numer i lokalną politykę (limit restartów na godzinę, okno zamrożenia, lista akcji dla hosta). Supervisor ponownie waliduje argumenty.
5. Każdy krok trafia do audytu: żądanie, zatwierdzenie, wysłanie, potwierdzenie, odrzucenia.

### Status, logi, kanały, wdrożenia

- **Status:** heartbeat agenta (CPU, RAM, dysk, jednostki, kontenery, restarty, digest, inkarnacja) + raporty gry (runtime status, scope assignments, świeżość watermarku LCFA). Widok świat → kanał → węzeł → host ze stanami `healthy | degraded | down | maintenance | reconciling`; brak danych > 15 s = nieaktualne. Metryki Prometheus.
- **Logi:** kontener gry loguje do journald; agent czyta tylko journald, ukrywa tokeny, klucze i maile, wysyła do partycji dziennych w PG. Podgląd na żywo przez SSE, wyszukiwanie po czasie, węźle, poziomie i tekście (trigram). Retencja 14 dni prod, 3 dni preprod; przy > ok. 2 GB/dzień zamiana na VictoriaLogs za interfejsem `LogStore`.
- **Nowy kanał / świat:** wpis w rejestrze → nowa generacja topologii i NodeSpec z wolnym portem → komendy (certyfikat, obraz, spec, migracje, start) → gra rejestruje się i raportuje → platforma publikuje trasę i podbija epokę katalogu; klient widzi kanał. Otwarcie portu w Oracle security list to ręczny krok z checklisty (platforma nie ma poświadczeń chmury).
- **Restart / wyłączenie / wdrożenie z odliczaniem:** komunikaty T-15, T-5, T-1 min (Operator Commands v1) → maintenance → `SaveAndShutdown` → przy wdrożeniu: obraz, migracje, spec, start → sprawdzenie zdrowia → zdjęcie maintenance. Do czasu Operator Commands v1: `StopNode` z SIGTERM (zespół gry musi potwierdzić pełny zapis). Cofnięcie wersji tylko gdy manifest mówi `rollback_safe = true`.
- **Uprawnienia `ops.*`:** `nodes.view`, `logs.view`, `logs.export`, `nodes.start`, `nodes.stop`, `nodes.restart`, `deploy`, `channels.manage`, `worlds.manage`, `maintenance.manage`, `broadcast`, `shutdown.schedule`, `agents.enroll`, `agents.revoke`, `approve`. Role: GM (podgląd, logi, broadcast), Operator (+ restart, maintenance, zaplanowane wyłączenia), Owner (wszystko).

## Wiki z katalogu gry

Wiki w stylu TibiaWiki BR / Fandom: każdy potwór, przedmiot, NPC, czar, quest, obszar, profesja, strój, wierzchowiec, dom i osiągnięcie ma stronę generowaną z danych gry. Dane są zablokowane (pochodzą z katalogu), opisy piszą redaktorzy z recenzją.

### Eksport po stronie gry

- Gra ma już bogatą treść w `content/` (potwory z bestiary i bosstiary, 77 plików przedmiotów, NPC, loot z prawdopodobieństwami, questy, czary, stroje, domy, handel i podróże NPC, obszary, 1208 fragmentów mapy) oraz assety klienta 15.30, które `crates/client-assets` już dekoduje. Catalog Export v1 jest tylko propozycją z fixture'em.
- Nowy crate `oteryn-catalog` w Oteryn-Game (typy schematu v2, kanoniczny JSON, digest, walidacja; bez I/O), osobny od `platform-wire`.
- **Catalog Export v2** (nadzbiór v1): doda `vocation`, `outfit`, `mount`, `house`, `area`, `hunting_place`, `npc_trade_offer`, `npc_travel`, `creature_spawn_summary`, `bestiary_entry`, `boss`, `achievement`, `spell_stats`, `item_stats`. Każda kategoria ma poziom kompletności; wiki pokazuje baner „dane niepełne” zamiast twierdzić, że czegoś nie ma.
- **Asset Bundle v1** (`tools/catalog-assets`): sprite'y WebP (też animowane), stroje, kafelki minimapy XYZ 256 px dla pięter 0–15, `assets.json`.
- Przy każdym tagu gry CI publikuje obraz, `catalog.json` i assety jako artefakt OCI `ghcr.io/oteryn/oteryn-catalog:<release>` plus manifest wydania podpisany kluczem wydania.

### Import i aktywacja w platformie

- Worker pobiera artefakt, sprawdza podpis i digesty, zapisuje niezmienny snapshot w `catalog.*`.
- Assety trafiają do Oracle Object Storage (S3) pod `cdn.oteryn.com` za Cloudflare (`immutable`).
- Modele do czytania (`wiki.entity_views`, indeksy powiązań, wyszukiwanie) i lista zmian względem poprzedniego snapshotu budują się obok. Aktywacja to atomowa zmiana wskaźnika; rollback to powrót do jednego z 5 ostatnich snapshotów. Domyślnie aktywacja razem z wdrożeniem danego wydania gry.

### Strony

- Adresy: `/{locale}/wiki/{typ}/{slug}`, stały link `/{locale}/wiki/k/{content_key}`, przekierowania po zmianie nazwy.
- Strona: infobox z danymi, sprite, statystyki, potem powiązania:

| Encja | Powiązania |
| --- | --- |
| Przedmiot | wypada z (potwór, szansa, ilość), sprzedaje / kupuje (NPC, cena, miasto), nagroda za (quest) |
| Potwór | tabela loota, występuje w (obszary + pinezka na mapie) |
| NPC | oferty handlowe, podróże, lokalizacja |
| Czar / runa | statystyki, profesje |
| Obszar | mapa (Leaflet + kafelki) i potwory |

- Dalej opisy redakcyjne i historia zmian („zmienione w wydaniu X”). Usunięte encje zostają z oznaczeniem „usunięte w wydaniu X”.
- Wyszukiwanie: PG FTS z `unaccent` + `pg_trgm` (polski przez `simple` + trigram, bo PG nie ma polskiego stemmera); kategorie automatyczne (rasa, klasa przedmiotu, profesja, miasto) i redakcyjne.
- Odwołania do danych w treści, np. `{{item:oteryn:item.x|price@npc:oteryn:npc.y}}`, rozwiązywane przy renderowaniu.
- Polskie nazwy: nakładka tłumaczeń z recenzją, angielski jako zapas z oznaczeniem „przetłumaczę”; jeśli gra kiedyś wyeksportuje nazwy po polsku, mają pierwszeństwo.
- Istniejące artykuły wiki z Laravela przechodzą jako redakcyjne i są automatycznie łączone z encjami po nazwie; stare adresy dostają 301.

## Migracja i przełączenie

Jest jedno przełączenie. Do tego momentu Laravel i produkcyjny węzeł gry działają jak dziś, a v2 nic nie zapisuje na produkcji. Przełączenie przenosi naraz trzy rzeczy: dane (import), endpoint dla gry (`172.18.0.12:8543` z produkcyjną tożsamością mTLS) i publiczne trasy (Caddy `oteryn.com`, `/oauth/*`, `/v1/*`). Gateway w Go działa dalej bez zmian konfiguracji, bo upstream ma ten sam adres i port.

### Import danych (`tools/legacy-import`)

- Czyta odtworzony `pg_dump` Laravela (PG 17.11), nigdy żywą bazę; pisze do v2 PG 19. Tryby: `import`, `verify`, `reverse`.
- Zachowuje: ID tożsamości, hashe argon2id i kody odzyskiwania; sekrety 2FA (odszyfrowane z Laravelowego `encrypted` kluczem APP\_KEY i zaszyfrowane key ringiem v2); klientów OAuth i redirecty; hashe sesji urządzeń jako wersja legacy; ID światów, kanałów i tras z epokami; `native_security_generation`, floory evidence i pliki witness; klucze admission z kid; intencje i potwierdzenia komend postaci; epoki i rewizje LCFA; ledger premium; całą treść CMS, wiki, media, eventy, pobieranie i updater; support, audyt i role (1:1 na 23 uprawnienia).
- Pomija: dane Canary i Player Companion, sesje, cache, joby i tickety Laravela.
- Publiczne dane gry nie są importowane: po przełączeniu produkcyjny węzeł gry zaczyna pchać do v2 Public Projections v1. Wersja gry z producentem musi być wdrożona przed przełączeniem, z producentem wyłączonym.
- `verify` porównuje liczby wierszy i digesty kolumn z sekretami, równość generacji, floorów i witness, sprawdza hasło konta testowego i rewizje LCFA z bazą gry (tylko w próbach). Każda niezgodność przerywa import.
- Próby: co najmniej 3 na `oteryn-v2-rehearsal` z odtworzonych snapshotów prod: import, testowy węzeł gry, pełny test zgodności, logowanie, lista i tworzenie postaci, snapshot premium. Cel: poniżej 5 minut.

### Bramka gotowości (wszystko zielone, podpisuje właściciel)

| # | Warunek |
| --- | --- |
| G1 | M0–M5 gotowe; v2-preprod działa 14 dni bez błędów Sev-1/Sev-2. |
| G2 | Test zgodności przechodzi przeciw dokładnemu digestowi obrazu gry, który będzie na produkcji (z producentem projekcji). |
| G3 | 3 czyste próby z rzędu; import + verify < 5 min; próba importu odwrotnego czysta. |
| G4 | Lista zgodności funkcji podpisana; każda używana trasa Laravela ma odpowiednik w v2 albo wpis 301/410 w `redirects.toml`. |
| G5 | Przegląd bezpieczeństwa: CSRF, limity, cookies, CSP, zewnętrzny skan TLS/mTLS, czysty `cargo deny`, uprawnienia sekretów. |
| G6 | Test obciążenia: internal API p99 < 150 ms przy tempie gry plus 200 rps na stronach publicznych; brak przekroczeń 1 s. |
| G7 | Każda komenda ops z artisana przeniesiona do `oteryn-platform ops`; self-check admission zielony; odtworzenie < 30 min; dashboardy i alerty działają. |
| G8 | Oba tłumaczenia kompletne; strony prawne zatwierdzone. |
| G9 | Rollback przetestowany; dyżur na dzień przełączenia i 7 dni po. |

### Przebieg przełączenia (okno ok. 45 min)

1. **T-7 dni:** wdrożyć wersję gry z wyłączonym producentem projekcji; zamrozić deploye Laravela.
2. **T-0:** tryb maintenance tras gry przez Laravela (gateway odpowiada ROUTE\_UNAVAILABLE), wyjście graczy, `artisan down`, stop workerów i schedulera.
3. Ostatni `pg_dump` Laravela, zapis LSN `L0`. Baza i kontenery Laravela zostają zatrzymane, ale nietknięte.
4. `legacy-import import && verify` do nowej, produkcyjnej instancji PG 19; kopia plików kluczy i witness.
5. Stop nginx `internal-mtls`; start `platform-internal` z `compose.production.yml` na `172.18.0.12:8543` z produkcyjną CA i certyfikatem.
6. Przeładowanie Caddy z `cutover.caddy`: `oteryn.com` → v2.
7. Smoke test: self-check admission, evidence/LCFA/watermark w logach gry, prawdziwe logowanie konta staff, lista i tworzenie postaci, snapshot premium.
8. Włączenie producenta projekcji; wszystkie rodziny świeże w 10 s.
9. Zdjęcie maintenance przez v2. Decyzja go/no-go należy do właściciela; twardy limit T+60 min, potem rollback.
10. Po T+7 usunąć bloki `next.` i `login-next.`; v2-preprod zostaje stałym preprodem.

### Rollback

- **Przed krokiem 9** (brak zapisów graczy): stop `platform-internal`, start nginx, poprzedni Caddyfile, `artisan up`, workery Laravela, wyłączenie producenta. Baza Laravela jest nietknięta na `L0`. Ok. 5 minut.
- **Po kroku 9:** `legacy-import reverse` z v2 do kopii bazy Laravela z `L0` (nowe i zmienione tożsamości, hashe, 2FA, podniesione generacje/floory/witness, komendy, rewizje LCFA, wpisy ledgera), potem zamiana kontenerów jak wyżej.
- **Zakazane:** powrót do bazy z `L0` bez importu odwrotnego. Gra zobaczyłaby cofnięte generacje i odrzuciłaby evidence oraz projekcje.
- Wyzwalacze: błędy admission > 2 %, regresja evidence, opóźnienie projekcji > 60 s przez 10 min, błąd verify.
- Okno rollbacku: 7 dni. Potem Laravel jest archiwizowany (obraz, ostatni dump, APP\_KEY w zimnym magazynie) i wyłączany.

## Roadmapa

Kamienie milowe budowy na v2-preprod (nie etapy produkcji). Nakład zakłada jednego seniora Rust na platformie; zespół gry pracuje równolegle.

| Kamień | Zakres | Platforma | Gra | Wyjście (na v2-preprod) |
| --- | --- | --- | --- | --- |
| **M0 Fundamenty** | Repo, CI na `ubuntu-26.04-arm`, obraz chiselled, compose i runner, PG 19, loader konfiguracji i sekretów, layout i CSS, i18n en/pl, obserwowalność, tag `oteryn-platform-wire` v1, CA v2 | 3 tyg. | Wydzielenie wire crate; PR z Rust 1.99.1 | `next.oteryn.com/healthz`; szkielet testu zgodności |
| **M1 Rdzeń dla gry** | Wszystkie rodziny `/internal/v1`, ops rejestru/topologii/kluczy, klucze Ed25519 admission | 3–4 tyg. | brak | Testowy węzeł gry działa w pełni na v2; zgodność zielona |
| **M2 Tożsamość + natywne dane publiczne** | Rejestracja, logowanie, 2FA, reset, urządzenia; OAuth PKCE; tickety; tworzenie i lista postaci; highscores, profile, zgony, online, domy | 3–4 tyg. | Producent Public Projections v1 (2–3 tyg.) | Prawdziwy klient loguje się przez `login-next`, tworzy postać i gra na `game-next:7272`; strony pokazują natywne dane. **Podgląd dla właściciela.** |
| **M3 Treści** | Newsy, strony, prawne, eventy, ogłoszenia, pobieranie, wydania launchera, wiki (czytanie, edycja, recenzja, media), katalog | 4–5 tyg. | brak | Zgodność treści; importer obejmuje wszystkie tabele treści |
| **M4 Administracja i support** | RBAC (23 uprawnienia + `gm.*`/`ops.*`), audyt, zgłoszenia, kary, odwołania, bany przez generację, maintenance przez route policy; Operator Commands v1 (broadcast, kick, mute) | 3–4 tyg. | Konsument Operator Commands v1 (2 tyg.) | Praca GM bez SSH |
| **M5 Premium i ledger** | Katalog produktów, portfel z podwójnym zapisem, premium z dostawcą testowym, rekoncyliacja | 2–3 tyg. | brak | Niezmienniki ledgera udowodnione |
| **M6 Przygotowanie przełączenia** | Importer i import odwrotny, 3 próby, test obciążenia, przegląd bezpieczeństwa, mapa przekierowań, runbooki, bramka G1–G9 | 2–3 tyg. | Wydanie gry z producentem | **Bramka zielona → jedno przełączenie** |
| M7 (po przełączeniu) | Prawdziwy dostawca płatności; zmiana nazwy, usunięcie i transfer postaci; natywny Bazaar z escrow | 6–10 tyg. | Rozszerzenia konsumenta komend (3–4 tyg.) | — |
| M8 (po przełączeniu) | Zarządzanie gildiami i domami przez www, dostawca OIDC, ewentualny gateway w Rust (tylko z ADR) | 6–8 tyg. | GUILD-0 | — |

- **MVP = M0–M6:** wszystko, czego dziś faktycznie używa się w Laravelu, plus natywne dane publiczne, Operator Commands v1, pełne ops CLI i sprawdzony importer z importem odwrotnym. Płatności, Bazaar i gildie nie są wymagane, bo w Laravelu natywnie i tak nie działają.
- **Szacunek:** ok. **20–26 tygodni** pracy nad platformą do przełączenia; po stronie gry ok. 5–7 tygodni równolegle.
- **Pierwszy widoczny efekt:** M2 ok. 9–11 tygodnia albo ok. 6 tygodnia, jeśli drugi developer zrobi M1 i M2 równolegle.

## Ryzyka i otwarte decyzje

| Ryzyko / decyzja | Jak ograniczamy / rekomendacja |
| --- | --- |
| Jedno duże przełączenie (bez strangler) | Bramka G1–G9, 3 próby na prawdziwych snapshotach, przetestowany import odwrotny, go/no-go właściciela z limitem T+60 min. |
| Dwa stosy na jednym VPS ARM (RAM, dysk, CPU) | Najpierw sprawdzić kształt A1. Buildy poza maszyną, testowy węzeł gry na żądanie, `mem_limit`, czyszczenie obrazów. Opcja: drugi VPS A1 na preprod. |
| Rozjazd kontraktu między repozytoriami | Wire crate bez I/O przypięty tagiem, lock JSON, test zgodności na każdym PR, G2 na dokładnym digestowi z prod. |
| Brak producenta Public Projections w grze | Twarda zależność M2 i przełączenia. Zamknąć kontrakt CANDIDATE w pierwszym tygodniu. |
| PostgreSQL 19 to dziś RC1 | Rozwój na RC; przełączenie tylko na 19.x GA (spodziewane dużo przed M6). Awaryjnie 18.6, ten sam kod. |
| Toolchain: gra 1.94.0 (plik) / 1.95.0 (CI) vs v2 1.99.1 | v2 niezależny (D1). PR podbijający grę do 1.99.1 i naprawiający rozjazd; spodziewane nowe linty clippy. |
| Łatka tokio `=1.53.1` w grze vs 1.53.3 | Bez wpływu na v2 (osobny lockfile). Zespół gry sprawdza, czy łatka jest wciąż potrzebna. |
| argon2 0.6: podwójne crate'y krypto obok ed25519-dalek 2.2 | Duplikaty dopuszczone w `cargo deny`; wrócić do tematu, gdy ed25519-dalek przejdzie na nową linię. |
| htmx 4 jest nowy (sierpień 2026) | Greenfield, więc od razu 4.x, wendorowany po hashu; minimum JS, żeby powrót do 2.x był tani. |
| Slice'y chisel dla Ubuntu 26.04 niezweryfikowane | Awaryjnie `ubuntu:26.04` read-only non-root; ten sam glibc, zero zmian w kodzie. |
| Migracja etykiety `ubuntu-latest` (19.10–19.11.2026) | Jawne `ubuntu-26.04(-arm)` w obu repo; CI gry zejść z `ubuntu-24.04` w PR toolchaina. |
| Wierność odszyfrowania `encrypted`/APP\_KEY | Testy na fixture'ach z obecnego Laravela i próby; APP\_KEY w zimnym magazynie po wyłączeniu. |
| Dane produkcyjne w środowisku prób (PII) | Rehearsal niewystawiony do internetu, snapshoty kasowane po każdej próbie, dostęp logowany. |
| **Otwarte:** `next.oteryn.com` jako stały preprod czy `preprod.oteryn.com` | Rekomendacja: po przełączeniu zmienić na `preprod.oteryn.com`. |
| **Otwarte:** drugi VPS na v2-preprod | Rekomendacja: tak, jeśli A1 ma mniej niż 12 GB RAM. |
| **Otwarte:** dostawca płatności (Przelewy24 / Stripe z BLIK) | Decyzja przed M7; nie blokuje przełączenia. |
| **Otwarte:** upgrade systemu oteryn-vps do Ubuntu 26.04 | Rekomendacja: tak, ale po przełączeniu + 7 dni, w osobnym oknie. Kontenery już są na 26.04. |
| **Otwarte:** podbicie toolchaina gry do 1.99.1 | Rekomendacja: tak, osobnym PR w Oteryn-Game. |
