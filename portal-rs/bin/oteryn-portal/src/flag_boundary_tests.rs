mod flag_boundary {
    //! Exercise the production serve role and worker registry in a disposable schema.
    use super::super::*;
    use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
    use sha2::{Digest, Sha256};
    use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        net::{SocketAddr, TcpListener},
        path::Path,
        process::Command,
        time::Instant,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use uuid::Uuid;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const CHILD: &str = "FLAG_BOUNDARY_CHILD";
    const ROUTE_PROFILE: &str = "FLAG_BOUNDARY_ROUTE_PROFILE";
    const ROUTE_PROFILES: &str = "FLAG_BOUNDARY_ROUTE_PROFILES";
    const RETIRED_BINDS: &[&str] = &[
        "OTERYN_BAZAAR_INTERNAL_BIND",
        "OTERYN_LEAGUE_BIND",
        "PORTAL_ACCOUNT_CLOSURE_BIND",
    ];
    const LISTENER_SETTINGS: &[&str] = &[
        "OTERYN_BIND",
        "OTERYN_INTERNAL_TLS_BIND",
        "OTERYN_NATIVE_TLS_BIND",
        "PORTAL_NATIVE_ADMISSION_BIND",
        "OTERYN_BAZAAR_INTERNAL_BIND",
        "OTERYN_LEAGUE_BIND",
        "PORTAL_ACCOUNT_CLOSURE_BIND",
    ];
    const SESSION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STAFF_SESSION: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const CSRF: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    fn inventory() -> std::io::Result<String> {
        fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/FLAG_BOUNDARY.md"),
        )
    }

    fn inventory_rows(inventory: &str) -> impl Iterator<Item = &str> {
        inventory
            .lines()
            .skip_while(|line| !line.starts_with("| Flag |"))
            .skip(2)
            .take_while(|line| line.starts_with("|"))
    }

    fn collect_flags(root: &Path, found: &mut BTreeSet<String>) -> TestResult {
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() {
                collect_flags(&path, found)?;
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && path
                    .file_name()
                    .is_some_and(|name| name != "flag_boundary_tests.rs")
            {
                let source = fs::read_to_string(path)?;
                found.extend(
                    source
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .filter(|word| {
                            [
                                "OTERYN_",
                                "PORTAL_",
                                "GAME_AUTH_",
                                "GATEWAY_",
                                "PRODUCTS_ENTITLEMENTS_",
                                "PLAYERDATA_",
                            ]
                            .iter()
                            .any(|prefix| word.starts_with(prefix))
                                && word.ends_with("_ENABLED")
                                && word.bytes().all(|b| !b.is_ascii_lowercase())
                        })
                        .map(str::to_owned),
                );
            }
        }
        Ok(())
    }

    #[test]
    fn enabled_flags_are_inventoried() -> TestResult {
        let started = Instant::now();
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut found = BTreeSet::new();
        for area in ["crates", "bin"] {
            for package in fs::read_dir(workspace.join(area))? {
                let source = package?.path().join("src");
                if source.is_dir() {
                    collect_flags(&source, &mut found)?;
                }
            }
        }
        let declared_flags = FLAGS.iter().copied().collect::<BTreeSet<_>>();
        for flag in &found {
            assert!(
                declared_flags.contains(flag.as_str()),
                "undocumented source flag {flag}"
            );
        }
        let inventory = inventory()?;
        for flag in FLAGS {
            assert!(
                inventory_rows(&inventory).any(|line| line
                    .split('|')
                    .nth(1)
                    .is_some_and(|flags| flags.contains(&format!("`{flag}`")))),
                "{flag} is missing from FLAG_BOUNDARY.md"
            );
        }
        eprintln!(
            "scanned {} enabled flags in {:.3}s",
            found.len(),
            started.elapsed().as_secs_f64()
        );
        Ok(())
    }

    /// These jobs stay registered after flags go off so old data still expires.
    /// The table lists name the permitted deletes, updates and inserts.
    type RetentionJob = (
        &'static str,
        &'static str,
        &'static [&'static str],
        &'static [&'static str],
        &'static [&'static str],
    );
    const RETENTION: &[RetentionJob] = &[
        (
            "cleanup.deaths",
            "remove old public death details",
            &[],
            &["public_deaths_v2"],
            &[],
        ),
        (
            "cleanup.game-logins",
            "remove old game login history",
            &["pd_game_logins"],
            &[],
            &[],
        ),
        (
            "cleanup.game-ticket-budgets",
            "remove old ticket budget windows",
            &["game_ticket_budgets"],
            &[],
            &[],
        ),
        (
            "cleanup.hunting-places",
            "expire aggregate inputs and replay fences",
            &["pd_places_sessions", "pd_places_replay_fences"],
            &[],
            &[],
        ),
        (
            "cleanup.journal-payloads",
            "redact retained journal payloads and advance retention floors",
            &[
                "pd_g9_records",
                "pd_t78_records",
                "pd_records",
                "pd_deleted_scopes",
                "pd_conflicts",
                "pd_t76_records",
                "pd_t76_events",
            ],
            &[
                "pd_records",
                "pd_streams",
                "pd_g9_records",
                "pd_g9_streams",
                "pd_t76_streams",
                "pd_t76_records",
                "pd_t76_events",
                "pd_t78_records",
            ],
            &[],
        ),
        (
            "cleanup.mfa",
            "expire MFA challenges and grants",
            &["account_mfa_tokens"],
            &[],
            &[],
        ),
        (
            "cleanup.passkeys",
            "expire authentication challenges",
            &["passkey_ceremonies"],
            &[],
            &[],
        ),
        (
            "cleanup.rate-limits",
            "expire shared request budgets",
            &["web_rate_limits"],
            &[],
            &[],
        ),
        (
            "cleanup.sessions",
            "expire shared web sessions",
            &["web_sessions"],
            &[],
            &[],
        ),
        (
            "cleanup.tokens",
            "expire email verification history",
            &["account_email_tokens"],
            &[],
            &[],
        ),
        (
            "cleanup.world-kills",
            "retire expired world kills payloads",
            &[],
            &["stats_world_kills_v2"],
            &[],
        ),
        (
            "community.today.retention",
            "remove expired community snapshots",
            &[],
            &["today_snapshots"],
            &[],
        ),
        (
            "identity.account-closure-audit",
            "expire closure audit history",
            &["plat3_closure_audit_outbox"],
            &[],
            &[],
        ),
        (
            "league.retention",
            "expire league audit and feed data",
            &["league_audit", "league_feed_inbox"],
            &["league_subjects", "league_feed_requests"],
            &[],
        ),
        (
            "cleanup.support-tickets",
            "purge terminal tickets and messages after 180 days",
            &["tickets", "ticket_messages"],
            &[],
            &[],
        ),
        (
            "identity.finalize-terminations",
            "drain accepted requests subject to the Game proof and local blockers",
            &[],
            &["account_terminations"],
            &[],
        ),
        (
            "stats.world-sample-retention",
            "replace raw samples older than 90 days with daily aggregates",
            &["stats_world_samples"],
            &["stats_world_daily"],
            &["stats_world_daily"],
        ),
        (
            "stats.death-retention",
            "enforce the 30-day copied-death retention window",
            &["stats_character_events"],
            &[],
            &[],
        ),
        (
            "stats.v2-retention",
            "erase expired statistics payloads",
            &[],
            &["stats_v2_subjects"],
            &[],
        ),
    ];
    const CORE_JOBS: &[&str] = &["cleanup.support"];
    const DISABLED_JOBS: &[&str] = &[
        "admin.control.expire",
        "bazaar.authority",
        "commerce.refund",
        "email.account_exists",
        "email.password-reset",
        "email.staff_invitation",
        "email.verify",
        "identity.email-change",
        "league.ingest",
        "referrals.process",
        "stats.poll",
        "wallet.top_up",
    ];

    #[test]
    fn flags_off_keep_routes_jobs_and_tables_inert() -> TestResult {
        run_case("flags_off_keep_routes_jobs_and_tables_inert", false)
    }

    #[test]
    fn referrals_on_records_session_activity() -> TestResult {
        run_case("referrals_on_records_session_activity", true)
    }

    #[test]
    fn game_staff_browser_permissions() -> TestResult {
        run_case("game_staff_browser_permissions", false)
    }

    fn run_case(test: &str, referrals_enabled: bool) -> TestResult {
        if std::env::var_os(CHILD).is_some() {
            return tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(
                    tokio::task::LocalSet::new()
                        .run_until(exercise_boundary(test, referrals_enabled)),
                );
        }
        let route_profiles = tempfile::tempdir()?;
        if test == "game_staff_browser_permissions" {
            return launch_case(test, false, route_profiles.path(), "game-staff");
        }
        if test == "flags_off_keep_routes_jobs_and_tables_inert" {
            launch_case(test, false, route_profiles.path(), "enabled")?;
            launch_case(test, false, route_profiles.path(), "enabled-with-restore")?;
            launch_case(test, referrals_enabled, route_profiles.path(), "")?;
            let prerequisites = ROUTE_PREREQUISITES
                .iter()
                .flat_map(|group| group.flags.iter().copied())
                .collect::<BTreeSet<_>>();
            let mut failures = Vec::new();
            for prerequisite in prerequisites {
                if let Err(error) = launch_case(test, false, route_profiles.path(), prerequisite) {
                    failures.push(error.to_string());
                }
            }
            return if failures.is_empty() {
                Ok(())
            } else {
                Err(failures.join("\n").into())
            };
        }
        launch_case(test, referrals_enabled, route_profiles.path(), "")
    }

    fn launch_case(
        test: &str,
        referrals_enabled: bool,
        route_profiles: &Path,
        route_profile: &str,
    ) -> TestResult {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let listeners = LISTENER_SETTINGS
            .iter()
            .map(|_| TcpListener::bind("127.0.0.1:0"))
            .collect::<Result<Vec<_>, _>>()?;
        let addresses = listeners
            .iter()
            .map(TcpListener::local_addr)
            .collect::<Result<Vec<_>, _>>()?;
        let tls = tempfile::tempdir()?;
        make_tls(tls.path())?;
        let schema = format!("flag_boundary_{}", Uuid::now_v7().simple());
        let separator = if url.contains('?') { '&' } else { '?' };
        let scoped_url = format!("{url}{separator}options=-c%20search_path%3D{schema}");
        let name = format!("{}::{test}", module_path!());
        let mut command = Command::new(std::env::current_exe()?);
        // Process-local configuration without mutating the parallel test runner's environment.
        command
            .env_clear()
            .envs(FLAGS.iter().map(|flag| (*flag, "false")))
            .env(
                "PORTAL_REFERRALS_ENABLED",
                if referrals_enabled { "true" } else { "false" },
            )
            .args([
                "--exact",
                name.split_once("::").ok_or("test module")?.1,
                "--nocapture",
            ])
            .env(CHILD, tls.path())
            .env("OTERYN_ENV", "testing")
            .env("APP_ENV", "testing")
            .env("PORTAL_OAUTH_CONFIG", r#"{"enabled":false,"clients":[]}"#)
            .envs(
                LISTENER_SETTINGS
                    .iter()
                    .zip(addresses)
                    .map(|(key, address)| (*key, address.to_string())),
            )
            .env("OTERYN_DATABASE_URL", scoped_url)
            .env("PORTAL_TEST_DATABASE_URL", url)
            .env("OTERYN_PUBLIC_ORIGIN", "https://localhost")
            .env("OTERYN_ADMIN_HOST", "staff.localhost")
            .env("OTERYN_MFA_KEY", "19".repeat(32))
            .env(
                "OTERYN_INTERNAL_TLS_CERT_FILE",
                tls.path().join("server.pem"),
            )
            .env(
                "OTERYN_INTERNAL_TLS_KEY_FILE",
                tls.path().join("server.key"),
            )
            .env(
                "OTERYN_INTERNAL_TLS_CLIENT_CA_FILE",
                tls.path().join("ca.pem"),
            )
            .env("OTERYN_NATIVE_TLS_CERT_FILE", tls.path().join("server.pem"))
            .env("OTERYN_NATIVE_TLS_KEY_FILE", tls.path().join("server.key"))
            .env("FLAG_BOUNDARY_SCHEMA", &schema);
        if matches!(
            test,
            "referrals_on_records_legacy_session_refresh"
                | "referrals_on_records_anonymous_login_rotation"
                | "paused_intake_keeps_authenticated_cancellation"
                | "disabled_referral_connections_preserve_coverage"
        ) {
            command.env("OTERYN_ACCOUNT_LOGIN", "true");
        }

        if test == "game_staff_browser_permissions" {
            enable_route_profile(&mut command, tls.path())?;
            command.env(ROUTE_PROFILE, route_profile);
        }
        if test == "flags_off_keep_routes_jobs_and_tables_inert" {
            let route_dir = if route_profile.is_empty() {
                "disabled"
            } else {
                route_profile
            };
            let dir = route_profiles.join(route_dir);
            fs::create_dir_all(&dir)?;
            command.env("FLAG_BOUNDARY_ROUTE_DIR", dir);
            command.env(ROUTE_PROFILES, route_profiles);
            command.env(ROUTE_PROFILE, route_profile);
            if !route_profile.is_empty() {
                enable_route_profile(&mut command, tls.path())?;
                match route_profile {
                    "enabled-with-restore" => {
                        command.env("PORTAL_GAME_RESTORE_NOTICE_ENABLED", "true");
                        command.env("PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED", "false");
                    }
                    "enabled" => {}
                    disabled => {
                        command.env(disabled, "false");
                    }
                }
            }
        }
        drop(listeners);
        let output = command.output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        if !output.status.success() || !stdout.contains("test result: ok. 1 passed") {
            return Err(format!("{route_profile} profile failed:\n{stdout}\n{stderr}").into());
        }
        eprint!("{stderr}");
        Ok(())
    }

    fn enable_route_profile(command: &mut Command, tls: &Path) -> TestResult {
        command.envs(FLAGS.iter().map(|flag| (*flag, "true")));
        command.envs([
            // Restore notice and Store delivery are mutually exclusive at startup.
            ("PORTAL_GAME_RESTORE_NOTICE_ENABLED", "false"),
            ("PORTAL_OAUTH_CONFIG", r#"{"enabled":true,"clients":[{"client_id":"boundary","redirect_uris":["http://127.0.0.1/callback"]}]}"#),
            ("PORTAL_ADMIN_CONTROL_AGENTS_JSON", r#"[{"cn":"oteryn-admin-agent/boundary","identity":{"san":"flag-boundary-agent"},"allowed_nodes":["019ad000-0000-7000-8000-000000000001"],"allowed_worlds":[]}]"#),
            ("OTERYN_ADMIN_ANALYTICS_AUTHORITY", "flag-boundary-analytics"),
            ("OTERYN_ADMIN_ANALYTICS_PRIVACY", "flag-boundary"),
            ("OTERYN_ADMIN_ANALYTICS_PEER_SAN", "game-admin-analytics"),
            ("OTERYN_NOTIFY_UNSUBSCRIBE_KEY", "flag-boundary-notification-key-00000000"),
            ("OTERYN_NOTIFY_UNSUBSCRIBE_URL", "https://localhost/unsubscribe"),
            ("OTERYN_WEBAUTHN_RP_ID", "localhost"),
            ("OTERYN_SHOP_TOP_UP_PACKAGES", r#"[{"key":"coins-100","coins":100,"minor_units":250,"currency":"PLN"}]"#),
            ("OTERYN_SHOP_TEST_PAYMENT_KEY", "flag-boundary-shop-test-key-0000000000"),
            ("PORTAL_ATLAS_DELIVERY_ENABLED", "1"),
            ("PORTAL_WORLD_ENABLED", "1"),
        ]);
        let verification_key = ed25519_dalek::SigningKey::from_bytes(&[17; 32]).verifying_key();
        let key_hex = verification_key
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        fs::write(
            tls.join("bazaar-keys.json"),
            format!(r#"[{{"kid":"boundary","public_key_hex":"{key_hex}"}}]"#),
        )?;
        fs::write(
            tls.join("bazaar-fence.json"),
            r#"{"projection_epoch":1,"recovery_generation":1,"incarnation":"019ad000-0000-7000-8000-000000000001","character_history_complete_since":0}"#,
        )?;
        fs::write(
            tls.join("league-fence.json"),
            r#"{"source_authority":"oteryn:game:primary","projection_epoch":"1","recovery_generation":"1"}"#,
        )?;
        let spki_pin = |name: &str| -> TestResult<String> {
            use x509_parser::prelude::{FromDer, X509Certificate};
            let der = fs::read(tls.join(format!("{name}.der")))?;
            let (_, cert) = X509Certificate::from_der(&der)?;
            Ok(format!("{:x}", Sha256::digest(cert.public_key().raw)))
        };
        let client_pin = format!("{:x}", Sha256::digest(fs::read(tls.join("client.der"))?));
        command
            .env("OTERYN_ATLAS_DIRECTORY", tls)
            .env("OTERYN_RESET_DELIVERY_KEY", "02".repeat(32))
            .env("PORTAL_ACCOUNT_CLOSURE_PUBLIC_KEY_HEX", key_hex)
            .env("PORTAL_ACCOUNT_CLOSURE_KEY_ID", "boundary")
            .env("PORTAL_ACCOUNT_CLOSURE_RECOVERY_GENERATION", "1")
            .env("PORTAL_ACCOUNT_CLOSURE_BASE_URL", "https://localhost")
            .env("PORTAL_ACCOUNT_CLOSURE_CLIENT_CERT_SHA256", client_pin)
            .env(
                "PORTAL_ACCOUNT_CLOSURE_LCFA_CLIENT_CERT_SHA256",
                "21".repeat(32),
            )
            .env("OTERYN_BAZAAR_SOURCE_AUTHORITY", "oteryn:game:primary")
            .env("OTERYN_BAZAAR_RECEIPT_AUTHORITY", "oteryn:game:receipts")
            .env("OTERYN_BAZAAR_INTERNAL_BASE_URL", "https://localhost")
            .env(
                "OTERYN_BAZAAR_WORLD_IDS",
                "019ad000-0000-7000-8000-000000000001",
            )
            .env("OTERYN_BAZAAR_CLIENT_SPKI_SHA256", spki_pin("bazaar")?)
            .env("OTERYN_BAZAAR_RECEIPT_CLIENT_SPKI_SHA256", "41".repeat(32))
            .env("OTERYN_BAZAAR_FENCE_FILE", tls.join("bazaar-fence.json"))
            .env(
                "OTERYN_BAZAAR_SNAPSHOT_KEYS_FILE",
                tls.join("bazaar-keys.json"),
            )
            .env("OTERYN_LEAGUE_SOURCE_AUTHORITY", "oteryn:game:primary")
            .env("OTERYN_LEAGUE_BASE_URL", "https://localhost")
            .env("OTERYN_LEAGUE_CLIENT_SPKI_SHA256", spki_pin("league")?)
            .env("OTERYN_LEAGUE_RECOVERY_FILE", tls.join("league-fence.json"));
        Ok(())
    }

    async fn game_purposes_share_the_internal_mtls_address() -> TestResult {
        let address = std::env::var("OTERYN_INTERNAL_TLS_BIND")?.parse()?;
        let purposes = [
            ("client", "/internal/v1/game-auth/account-closure"),
            ("bazaar", "/internal/v1/game-events"),
            ("league", "/internal/v1/game-public/world-event-outcome"),
        ];
        let public = std::env::var("OTERYN_BIND")?.parse()?;
        let native = std::env::var("OTERYN_NATIVE_TLS_BIND")?.parse()?;
        for (purpose, path) in purposes {
            for (name, _) in purposes {
                let connector = client_tls_for(name)?;
                let response = exchange(address, Some(&connector), "POST", path).await?;
                let expected = if name == purpose { 400 } else { 401 };
                assert!(
                    response.starts_with(&format!("HTTP/1.1 {expected} ")),
                    "{name} on {path}: {response}"
                );
            }
            let connector = client_tls_for(purpose)?;
            for (address, tls) in [(public, None), (native, Some(&connector))] {
                let response = exchange(address, tls, "POST", path).await?;
                assert!(response.starts_with("HTTP/1.1 404 "), "{path}: {response}");
            }
        }
        for key in RETIRED_BINDS {
            let address: SocketAddr = std::env::var(key)?.parse()?;
            let error = tokio::net::TcpStream::connect(address)
                .await
                .err()
                .ok_or_else(|| format!("retired listener {key} accepted a connection"))?;
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
        }
        Ok(())
    }

    async fn capture_enabled_routes(pool: &PgPool, profile: &str) -> TestResult {
        let serve = tokio::task::spawn_local(async { crate::run(crate::Role::Serve).await });
        let result = async {
            wait_for_serve(&serve, std::env::var("OTERYN_BIND")?.parse()?).await?;
            if profile == "game-staff" {
                return exercise_game_staff_routes(pool).await;
            }
            if profile == "enabled" {
                game_purposes_share_the_internal_mtls_address().await?;
            }
            if !profile.starts_with("enabled") {
                exercise_prerequisite(pool, profile).await?;
            }
            Ok(())
        }
        .await;
        if serve.is_finished() {
            return serve.await?;
        }
        serve.abort();
        let _ = serve.await;
        result
    }

    fn make_tls(path: &Path) -> TestResult {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_key = KeyPair::generate()?;
        let ca = params.self_signed(&ca_key)?;
        fs::write(path.join("ca.pem"), ca.pem())?;
        for (name, purpose) in [
            ("server", ExtendedKeyUsagePurpose::ServerAuth),
            ("client", ExtendedKeyUsagePurpose::ClientAuth),
            ("bazaar", ExtendedKeyUsagePurpose::ClientAuth),
            ("league", ExtendedKeyUsagePurpose::ClientAuth),
        ] {
            let names = if matches!(name, "server" | "client") {
                vec!["localhost".into()]
            } else {
                Vec::new()
            };
            let mut params = CertificateParams::new(names)?;
            params.extended_key_usages.push(purpose);
            let key = KeyPair::generate()?;
            let cert = params.signed_by(&key, &ca, &ca_key)?;
            fs::write(path.join(format!("{name}.pem")), cert.pem())?;
            fs::write(path.join(format!("{name}.der")), cert.der())?;
            fs::write(path.join(format!("{name}.key")), key.serialize_pem())?;
        }
        Ok(())
    }

    async fn exercise_boundary(test: &str, referrals_enabled: bool) -> TestResult {
        let _database_guard = if test == "game_staff_browser_permissions" {
            oteryn_db::test_support::global_lock_guard().await?
        } else {
            None
        };
        let started = Instant::now();
        let config = oteryn_core::Config::from_env()?;
        let root = PgPoolOptions::new()
            .max_connections(1)
            .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
            .await?;
        let schema = std::env::var("FLAG_BOUNDARY_SCHEMA")?;
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(config.database_url())
            .await?;
        let result = async {
            oteryn_db::migrate(&pool).await?;
            if let Ok(profile) = std::env::var(ROUTE_PROFILE)
                && !profile.is_empty()
            {
                return capture_enabled_routes(&pool, &profile).await;
            }
            match test {
                "paused_intake_keeps_authenticated_cancellation" => {
                    exercise_cancellation(&pool).await
                }
                "disabled_referral_connections_preserve_coverage" => {
                    exercise_disabled_referral_connections(&pool).await
                }
                "referrals_on_records_legacy_session_refresh" => {
                    exercise_legacy_refresh(&pool).await
                }
                "referrals_on_records_anonymous_login_rotation" => {
                    exercise_login_rotation(&pool).await
                }
                _ if referrals_enabled => exercise_referrals_on(&pool).await,
                _ => exercise_schema(&pool).await,
            }
        }
        .await;
        pool.close().await;
        sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .execute(&root)
            .await?;
        result?;
        eprintln!(
            "referrals {referrals_enabled} case completed in {:.3}s",
            started.elapsed().as_secs_f64()
        );
        Ok(())
    }

    async fn seed_browser_accounts(pool: &PgPool) -> TestResult<(Uuid, Uuid)> {
        let account = Uuid::parse_str("019ad000-0000-7000-8000-000000000001")?;
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'flag-boundary@example.test','test-only','active','en',now())")
        .bind(account).execute(pool).await?;
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at,sudo_until) VALUES($1,$2,'flag-boundary',$3,now()+interval '1 hour',now()+interval '1 day',now()+interval '1 hour')")
        .bind(Sha256::digest(SESSION).to_vec()).bind(account).bind(CSRF).execute(pool).await?;
        let staff = Uuid::parse_str("019ad000-0000-7000-8000-000000000002")?;
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'flag-staff@example.test','test-only','active','en',now())")
            .bind(staff).execute(pool).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
            .bind(staff).execute(pool).await?;
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at,sudo_until) VALUES($1,$2,'flag-staff',$3,now()+interval '1 hour',now()+interval '1 day',now()+interval '1 hour')")
            .bind(Sha256::digest(STAFF_SESSION).to_vec()).bind(staff).bind(CSRF).execute(pool).await?;
        crate::test_staff::browser(
            pool,
            staff,
            &format!("__Host-oteryn_session={STAFF_SESSION}"),
        )
        .await?;
        Ok((account, staff))
    }

    async fn exercise_game_staff_routes(pool: &PgPool) -> TestResult {
        let (target, staff) = seed_browser_accounts(pool).await?;
        let address = std::env::var("OTERYN_BIND")?.parse()?;
        let mut tx = pool.begin().await?;
        oteryn_admin::audit::append(
            &mut tx,
            oteryn_admin::audit::Entry {
                context: oteryn_admin::audit::AuditContext {
                    actor: Some(staff),
                    request_id: Uuid::now_v7(),
                },
                action: "fixture.platform-audit",
                target: "platform-audit-visibility-marker",
                before: serde_json::Value::Null,
                after: serde_json::Value::Null,
            },
        )
        .await?;
        tx.commit().await?;
        let platform_pages = [
            "/admin/roles",
            "/admin/account-closure",
            "/admin/console/access",
            "/admin/news",
            "/admin/pages",
            "/admin/media",
            "/admin/wiki",
            "/admin/commerce",
            "/admin/en/bazaar",
        ];
        let player_detail = format!("/admin/players/{target}");
        for role in [
            "platform_admin",
            "game_master_senior",
            "game_viewer",
            "game_admin",
            "game_operator",
            "game_dba",
            "game_release",
            "game_security",
            "game_support",
            "game_analyst",
        ] {
            assert_eq!(
                sqlx::query("UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key=$2) WHERE account_id=$1")
                    .bind(staff).bind(role).execute(pool).await?.rows_affected(),
                1
            );
            let expected = if role == "platform_admin" {
                "200"
            } else {
                "403"
            };
            for path in platform_pages {
                let response = exchange(address, None, "GET", path).await?;
                assert_eq!(
                    response.split_whitespace().nth(1),
                    Some(expected),
                    "{role} GET {path}"
                );
            }
            if role == "platform_admin" {
                let response = exchange(
                    address,
                    None,
                    "GET",
                    "/admin/account-closure?lang=en",
                )
                .await?;
                assert_eq!(response.split_whitespace().nth(1), Some("200"));
                assert!(response.contains("<h1>Account closure status</h1>"));
            }
            if role.starts_with("game_") {
                for (method, path, expected) in [
                    ("GET", "/admin/players", "403"),
                    ("POST", "/admin/players", "405"),
                    ("GET", player_detail.as_str(), "403"),
                    ("POST", player_detail.as_str(), "403"),
                ] {
                    let response = exchange(address, None, method, path).await?;
                    assert_eq!(
                        response.split_whitespace().nth(1),
                        Some(expected),
                        "{role} {method} {path}"
                    );
                }
            }
            if matches!(role, "game_dba" | "game_release") {
                let path = "/admin/console/players";
                let response = exchange(address, None, "GET", path).await?;
                assert_eq!(
                    response.split_whitespace().nth(1),
                    Some("403"),
                    "{role} GET {path}"
                );
            }
            if matches!(role, "game_master_senior" | "platform_admin") {
                let platform = role == "platform_admin";
                for (path, platform_marker) in [
                    ("/admin/console/audit", "platform-audit-visibility-marker"),
                    (
                        "/admin/analytics/economy/data?start=86400&end=172800",
                        "Platform wallet ledger",
                    ),
                ] {
                    let response = exchange(address, None, "GET", path).await?;
                    assert_eq!(
                        response.split_whitespace().nth(1),
                        Some("200"),
                        "{role} GET {path}"
                    );
                    assert_eq!(
                        response.contains(platform_marker),
                        platform,
                        "{role} GET {path}"
                    );
                }
            }
            if role == "game_master_senior" {
                for path in [
                    "/console/dashboard",
                    "/admin/console/players",
                    "/admin/console/audit",
                    "/admin/analytics",
                ] {
                    let response = exchange(address, None, "GET", path).await?;
                    assert_eq!(
                        response.split_whitespace().nth(1),
                        Some("200"),
                        "{role} GET {path}"
                    );
                }
            }
            if role == "game_viewer" {
                let response = exchange(address, None, "GET", "/console/dashboard?lang=en").await?;
                assert_eq!(response.split_whitespace().nth(1), Some("200"));
                assert!(!response.contains("Platform metrics"));
                let id = "019ad000-0000-7000-8000-000000000003".parse::<Uuid>()?;
                let path = format!("/admin/moderation/enforcement/{id}");
                let response = exchange(address, None, "POST", &path).await?;
                assert_eq!(
                    response.split_whitespace().nth(1),
                    Some("403"),
                    "{role} POST {path}"
                );
                let created: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sanctions WHERE id=$1)")
                        .bind(id)
                        .fetch_one(pool)
                        .await?;
                assert!(!created);
            }
        }
        Ok(())
    }

    async fn exercise_schema(pool: &PgPool) -> TestResult {
        let (account, staff) = seed_browser_accounts(pool).await?;
        sqlx::query("INSERT INTO stats_character_events(character_ref,day,kind,event_key,details) SELECT $1,current_date-age,'death',age::text,jsonb_build_object('occurred_at',extract(epoch FROM now()-make_interval(days=>age))::bigint) FROM (VALUES (1),(31)) AS ages(age)")
        .bind(account).execute(pool).await?;

        sqlx::raw_sql(
            "WITH seeded AS (
                 INSERT INTO tickets(account_id,request_key,category,subject,status,closed_at)
                 SELECT id,uuidv7(),'other',age::text,'closed',now()-make_interval(days=>age)
                 FROM accounts CROSS JOIN (VALUES (179),(181)) ages(age)
                 WHERE email='flag-boundary@example.test' RETURNING id,account_id
             )
             INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,body)
                 SELECT id,account_id,'user','retention fixture' FROM seeded;
             INSERT INTO stats_world_samples(world,sampled_at,players_online)
                 SELECT id,now()-make_interval(days=>age),age
                 FROM accounts CROSS JOIN (VALUES (89),(91)) ages(age)
                 WHERE email='flag-boundary@example.test';
             WITH candidate AS (
                 INSERT INTO accounts(email,password_hash,status,locale,email_verified_at)
                     VALUES('due-termination@example.test','test-only','active','en',now())
                     RETURNING id
             )
             INSERT INTO account_terminations(account_id,created_at,scheduled_for,updated_at)
                 SELECT id,now()-interval '8 days',now()-interval '1 day',now()-interval '8 days'
                 FROM candidate;",
        )
        .execute(pool)
        .await?;

        // Leave the database campaign switch on to exercise the deployment
        // gate even when previously enabled campaign data exists.
        sqlx::raw_sql("UPDATE referral_campaign_settings SET enabled=true;
            INSERT INTO referral_links(account_id)
                SELECT id FROM accounts WHERE email='flag-boundary@example.test';
            INSERT INTO referral_campaign_invitations(inviter_id,invitee_id,kind)
                SELECT owner.id,staff.id,'new' FROM accounts owner CROSS JOIN accounts staff
                WHERE owner.email='flag-boundary@example.test' AND staff.email='flag-staff@example.test';")
            .execute(pool).await?;

        // Transactional operation counts catch UPDATE and insert-then-delete
        // without waiting for asynchronous pg_stat publication across pools.
        sqlx::raw_sql(
        "CREATE TABLE flag_boundary_writes(relation text, operation text);
         CREATE FUNCTION flag_boundary_track() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF TG_TABLE_NAME='web_sessions' AND TG_OP='UPDATE' AND
              to_jsonb(OLD)-ARRAY['last_seen_at','updated_at','idle_expires_at'] =
              to_jsonb(NEW)-ARRAY['last_seen_at','updated_at','idle_expires_at'] THEN
             RETURN NULL;
           END IF;
           INSERT INTO flag_boundary_writes VALUES (TG_TABLE_NAME,TG_OP);
           RETURN NULL;
         END $$;
         DO $$ DECLARE rel record; BEGIN
           FOR rel IN SELECT oid FROM pg_class WHERE relnamespace=current_schema()::regnamespace
                    AND relkind IN ('r','p') AND NOT relispartition
                    AND relname <> 'flag_boundary_writes' LOOP
             EXECUTE format('CREATE TRIGGER flag_boundary_rows AFTER INSERT OR UPDATE OR DELETE ON %s
                            FOR EACH ROW EXECUTE FUNCTION flag_boundary_track()',rel.oid::regclass);
             EXECUTE format('CREATE TRIGGER flag_boundary_truncate AFTER TRUNCATE ON %s
                            FOR EACH STATEMENT EXECUTE FUNCTION flag_boundary_track()',rel.oid::regclass);
           END LOOP;
         END $$;
         CREATE FUNCTION flag_boundary_counts() RETURNS TABLE (relation text, rows bigint)
         LANGUAGE plpgsql AS $$
         DECLARE rel record;
         BEGIN
           FOR rel IN SELECT oid, relname FROM pg_class
                        WHERE relnamespace=current_schema()::regnamespace
                          AND relkind IN ('r','p') AND NOT relispartition
                          AND relname <> 'flag_boundary_writes' ORDER BY relname LOOP
             RETURN QUERY EXECUTE format('SELECT %L::text, count(*) FROM %s',rel.relname,rel.oid::regclass);
           END LOOP;
         END $$;",
    )
    .execute(pool)
    .await?;

        let observability_tables = ["portal_errors", "portal_error_rate"];
        let mut baseline = row_counts(pool).await?;
        let serve = tokio::task::spawn_local(async { crate::run(crate::Role::Serve).await });
        let routes = exercise_routes(&serve).await;
        serve.abort();
        let _ = serve.await;
        let mut writes = take_writes(pool).await?;
        writes.retain(|(table, operation, _)| {
            !(observability_tables.contains(&table.as_str())
                && matches!(operation.as_str(), "INSERT" | "UPDATE"))
        });
        let mut failures = routes?;
        if !writes.is_empty() {
            failures.push(format!("serve wrote {writes:?}"));
        }
        let mut after_routes = row_counts(pool).await?;
        baseline.retain(|(table, _)| !observability_tables.contains(&table.as_str()));
        after_routes.retain(|(table, _)| !observability_tables.contains(&table.as_str()));
        if baseline != after_routes {
            failures.push(format!(
                "serve changed table row counts: {:?}",
                baseline
                    .iter()
                    .filter(|entry| !after_routes.contains(entry))
                    .collect::<Vec<_>>()
            ));
        }
        failures.extend(exercise_jobs(pool).await?);
        failures.extend(exercise_retention_outcomes(pool).await?);
        exercise_referral_callbacks_off(pool, staff).await?;
        if let Err(error) = exercise_unconfigured_referrals(pool, staff).await {
            failures.push(error.to_string());
        }
        if !failures.is_empty() {
            return Err(failures.join("\n").into());
        }
        Ok(())
    }

    async fn exercise_referral_callbacks_off(pool: &PgPool, buyer: Uuid) -> TestResult {
        let url = std::env::var("OTERYN_DATABASE_URL")?;
        let runtime = oteryn_web::session::configured_pool(&url)?;
        let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('callback@example.test','test-only','active','en',now()) RETURNING id")
            .fetch_one(&runtime).await?;
        sqlx::query("INSERT INTO security_events(account_id,kind) VALUES($1,'login_succeeded')")
            .bind(account)
            .execute(&runtime)
            .await?;
        sqlx::query("INSERT INTO pd_game_logins(account_id,login_event_id,projection_epoch,recovery_generation,source_revision,entered_at) VALUES($1,uuidv7(),1,1,1,(extract(epoch FROM now())*1000)::bigint)")
            .bind(account).execute(&runtime).await?;
        // The game-login projector upserts activity even for a presentation
        // record that has already expired.
        assert_eq!(
            sqlx::query("INSERT INTO referral_activity(account_id,last_activity) VALUES($1,now()) ON CONFLICT(account_id) DO UPDATE SET last_activity=GREATEST(referral_activity.last_activity,EXCLUDED.last_activity)")
                .bind(account).execute(&runtime).await?.rows_affected(),
            0
        );
        let bound: bool = sqlx::query_scalar(
            "SELECT referral_campaign_signup($1,code::text) FROM referral_links",
        )
        .bind(account)
        .fetch_one(&runtime)
        .await?;
        assert!(!bound, "signup must leave an unclaimed code alone");
        sqlx::query("SELECT referral_campaign_purchase($1,30,clock_timestamp()), referral_campaign_loyalty($1,12)")
            .bind(buyer).execute(&runtime).await?;
        let writes = take_writes(pool).await?;
        assert!(
            writes
                .iter()
                .all(|(relation, _, _)| !relation.starts_with("referral_") && relation != "outbox"),
            "referral callbacks wrote {writes:?}"
        );
        Ok(())
    }

    async fn exercise_unconfigured_referrals(pool: &PgPool, buyer: Uuid) -> TestResult {
        let url = std::env::var("OTERYN_DATABASE_URL")?;
        let eligible: bool = sqlx::query_scalar(
            "SELECT account_value_eligible(inviter_id) AND account_value_eligible(invitee_id)
               FROM referral_campaign_invitations WHERE invitee_id=$1",
        )
        .bind(buyer)
        .fetch_one(pool)
        .await?;
        assert!(
            eligible,
            "maintenance purchase fixture must be eligible for a reward"
        );
        for setting in [None, Some("unknown")] {
            let mut conn = <sqlx::PgConnection as sqlx::Connection>::connect(&url).await?;
            if let Some(setting) = setting {
                sqlx::query("SELECT set_config('oteryn.referrals_enabled',$1,false)")
                    .bind(setting)
                    .execute(&mut conn)
                    .await?;
            }
            sqlx::query(
                "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at)
                VALUES(uuidv7()::text||'@example.test','test-only','active','en',now())",
            )
            .execute(&mut conn)
            .await?;
            sqlx::query("SELECT referral_campaign_purchase($1,30,clock_timestamp()), referral_campaign_loyalty($1,12)")
                .bind(buyer).execute(&mut conn).await?;
            let writes = take_writes(pool).await?;
            if writes
                .iter()
                .any(|(table, _, _)| table.starts_with("referral_") || table == "outbox")
            {
                return Err(format!("maintenance connection {setting:?} wrote {writes:?}").into());
            }
        }
        Ok(())
    }

    async fn exercise_retention_outcomes(pool: &PgPool) -> TestResult<Vec<String>> {
        let mut failures = Vec::new();
        let tickets: Vec<String> = sqlx::query_scalar(
            "SELECT subject FROM tickets JOIN ticket_messages ON ticket_id=tickets.id ORDER BY subject",
        ).fetch_all(pool).await?;
        if tickets != ["179"] {
            failures.push(format!(
                "ticket retention kept {tickets:?}; expected only day 179 and its message"
            ));
        }
        let samples: Vec<i32> =
            sqlx::query_scalar("SELECT players_online FROM stats_world_samples")
                .fetch_all(pool)
                .await?;
        let rollup: Vec<(i64, i64, i32, i32)> = sqlx::query_as(
            "SELECT samples,players_sum,players_min,players_max FROM stats_world_daily",
        )
        .fetch_all(pool)
        .await?;
        if samples != [89] || rollup != [(1, 91, 91, 91)] {
            failures.push(format!(
                "world retention kept {samples:?} with rollup {rollup:?}"
            ));
        }
        let retried: bool = sqlx::query_scalar(
            "SELECT finalized_at IS NULL AND updated_at > scheduled_for AND updated_at > created_at
             AND updated_at > transaction_timestamp()-interval '1 minute'
             FROM account_terminations",
        )
        .fetch_one(pool)
        .await?;
        if !retried {
            failures
                .push("termination drain did not retry the due request without Game proof".into());
        }
        Ok(failures)
    }

    #[test]
    fn referrals_on_records_legacy_session_refresh() -> TestResult {
        run_case("referrals_on_records_legacy_session_refresh", true)
    }

    #[test]
    fn referrals_on_records_anonymous_login_rotation() -> TestResult {
        run_case("referrals_on_records_anonymous_login_rotation", true)
    }

    #[test]
    fn paused_intake_keeps_authenticated_cancellation() -> TestResult {
        run_case("paused_intake_keeps_authenticated_cancellation", false)
    }

    async fn browser_request(
        app: axum::Router,
        method: &str,
        path: &str,
        cookie: &str,
        form: &str,
    ) -> TestResult<axum::response::Response> {
        use axum::{
            body::Body,
            extract::{ConnectInfo, Extension},
            http::Request,
        };
        use tower::ServiceExt;
        let peer = ConnectInfo("127.0.0.1:12345".parse::<SocketAddr>()?);
        Ok(app
            .layer(Extension(oteryn_web::Language::En))
            .layer(Extension(peer))
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("host", "localhost")
                    .header("origin", "https://localhost")
                    .header("cookie", cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form.to_owned()))?,
            )
            .await?)
    }

    async fn exercise_cancellation(pool: &PgPool) -> TestResult {
        let hash = oteryn_identity::password::hash("boundary test password")?;
        let account: Uuid = sqlx::query_scalar(
            "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at)
             VALUES('cancellation@example.test',$1,'active','en',now()) RETURNING id",
        )
        .bind(hash)
        .fetch_one(pool)
        .await?;
        let (cookie, _, _) = login_browser(pool, "cancellation@example.test").await?;
        let raw = cookie.split_once('=').ok_or("invalid session cookie")?.1;
        sqlx::query("UPDATE web_sessions SET sudo_until=now()+interval '5 min' WHERE id_hash=$1")
            .bind(Sha256::digest(raw).to_vec())
            .execute(pool)
            .await?;
        let termination: Uuid = sqlx::query_scalar("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days') RETURNING id")
            .bind(account).fetch_one(pool).await?;
        let runtime = crate::account_closure::configured(pool.clone())?;
        let app = crate::account_lifecycle_http::configured_router(
            pool.clone(),
            runtime.terminations.clone(),
        );
        let path = "/en/account/privacy/delete";
        let response = browser_request(app.clone(), "GET", path, &cookie, "").await?;
        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 100_000).await?;
        let body = std::str::from_utf8(&body)?;
        assert!(body.contains("/en/account/privacy/delete/cancel"));
        let csrf = browser_csrf(body)?;
        let form =
            format!("_csrf={csrf}&current_password=boundary+test+password&confirmation=TERMINATE");
        assert_eq!(
            browser_request(app.clone(), "POST", path, &cookie, &form)
                .await?
                .status(),
            405
        );
        let cancel = "/en/account/privacy/delete/cancel";
        assert_eq!(
            browser_request(
                app.clone(),
                "POST",
                cancel,
                &cookie,
                "current_password=boundary+test+password"
            )
            .await?
            .status(),
            403
        );
        let response = browser_request(app, "POST", cancel, &cookie, &form).await?;
        assert_eq!(response.status(), 303);
        let cancelled: bool = sqlx::query_scalar("SELECT cancelled_at IS NOT NULL AND finalized_at IS NULL FROM account_terminations WHERE id=$1")
            .bind(termination).fetch_one(pool).await?;
        assert!(cancelled);
        Ok(())
    }

    #[test]
    fn disabled_referral_connections_preserve_coverage() -> TestResult {
        run_case("disabled_referral_connections_preserve_coverage", false)
    }

    async fn exercise_disabled_referral_connections(pool: &PgPool) -> TestResult {
        let url = std::env::var("OTERYN_DATABASE_URL")?;
        let coverage: (Uuid, time::OffsetDateTime) = sqlx::query_as(
            "UPDATE portal_referral_coverage SET generation=uuidv7(), covered_since=now()-interval '13 months'
             RETURNING generation,covered_since",
        ).fetch_one(pool).await?;
        let off = oteryn_web::session::configured_pool(&url)?;
        let sql = "SELECT generation,covered_since FROM portal_referral_coverage";
        assert_eq!(
            sqlx::query_as::<_, (Uuid, time::OffsetDateTime)>(sql)
                .fetch_one(&off)
                .await?,
            coverage
        );
        let (account, _) = seed_browser_accounts(pool).await?;
        let legacy = oteryn_db::lazy_pool(&url)?;
        let cookie = format!("__Host-oteryn_session={SESSION}");
        let response = browser_request(
            crate::login_http::configured_router(legacy),
            "GET",
            "/en/login",
            &cookie,
            "",
        )
        .await?;
        assert_eq!(response.status(), 302);
        assert_eq!(
            sqlx::query_as::<_, (Uuid, time::OffsetDateTime)>(sql)
                .fetch_one(pool)
                .await?,
            coverage
        );
        let activity: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM referral_activity WHERE account_id=$1)",
        )
        .bind(account)
        .fetch_one(pool)
        .await?;
        assert!(!activity);
        Ok(())
    }

    async fn exercise_legacy_refresh(pool: &PgPool) -> TestResult {
        let generation: Uuid = sqlx::query_scalar("SELECT referral_transition_coverage(true)")
            .fetch_one(pool)
            .await?;
        let url = std::env::var("OTERYN_DATABASE_URL")?;
        // A pre-upgrade process uses its ordinary pool with startup options set by deployment.
        let enabled = format!(
            "{url}%20-c%20oteryn.referrals_enabled%3Dtrue%20-c%20oteryn.referrals_generation%3D{generation}"
        );
        for (email, url, covered) in [
            ("old-refresh@example.test", enabled, true),
            ("unconfigured-refresh@example.test", url, false),
        ] {
            let account: Uuid = sqlx::query_scalar(
                "INSERT INTO accounts(email,password_hash,status,locale)
                 VALUES($1,'test-only','active','en') RETURNING id",
            )
            .bind(email)
            .fetch_one(pool)
            .await?;
            let legacy = oteryn_db::lazy_pool(&url)?;
            let previous: time::OffsetDateTime = sqlx::query_scalar(
                "INSERT INTO web_sessions(id_hash,account_id,last_seen_at,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at)
                 VALUES($1,$2,now()-interval '10 minutes','legacy',$3,now()+interval '1 hour',now()+interval '1 day') RETURNING last_seen_at",
            ).bind(Sha256::digest(SESSION).to_vec()).bind(account).bind(CSRF).fetch_one(pool).await?;
            let cookie = format!("__Host-oteryn_session={SESSION}");
            let response = browser_request(
                crate::login_http::configured_router(legacy),
                "GET",
                "/en/login",
                &cookie,
                "",
            )
            .await?;
            assert_eq!(response.status(), 302);
            let refreshed: time::OffsetDateTime =
                sqlx::query_scalar("SELECT last_seen_at FROM web_sessions WHERE account_id=$1")
                    .bind(account)
                    .fetch_one(pool)
                    .await?;
            assert!(refreshed > previous);
            sqlx::query("UPDATE web_sessions SET idle_expires_at=now()-interval '1 hour' WHERE account_id=$1")
                .bind(account).execute(pool).await?;
            oteryn_web::session::SessionCleanup { pool: pool.clone() }
                .clean()
                .await?;
            assert!(
                !sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM web_sessions WHERE account_id=$1)"
                )
                .bind(account)
                .fetch_one(pool)
                .await?
            );
            let recorded: Option<time::OffsetDateTime> = sqlx::query_scalar(
                "SELECT last_activity FROM referral_activity WHERE account_id=$1",
            )
            .bind(account)
            .fetch_optional(pool)
            .await?;
            assert_eq!(
                recorded,
                covered.then_some(refreshed),
                "the refresh must survive session expiry only with the deployment options"
            );
        }
        Ok(())
    }

    fn browser_csrf(html: &str) -> TestResult<&str> {
        html.split("name=\"_csrf\" value=\"")
            .nth(1)
            .and_then(|value| value.split('"').next())
            .ok_or_else(|| "missing browser form CSRF".into())
    }

    async fn login_browser(pool: &PgPool, email: &str) -> TestResult<(String, Uuid, Vec<u8>)> {
        use axum::http::header::SET_COOKIE;
        let app = crate::login_http::configured_router(pool.clone());
        let page = browser_request(app.clone(), "GET", "/en/login", "", "").await?;
        assert_eq!(page.status(), 200);
        let cookie = page.headers()[SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .ok_or("anonymous cookie")?
            .to_owned();
        let body = axum::body::to_bytes(page.into_body(), 100_000).await?;
        let token = browser_csrf(std::str::from_utf8(&body)?)?;
        let raw = cookie.split_once('=').ok_or("invalid anonymous cookie")?.1;
        let old_hash = Sha256::digest(raw).to_vec();
        let session_id = sqlx::query_scalar(
            "SELECT session_id FROM web_sessions WHERE id_hash=$1 AND account_id IS NULL",
        )
        .bind(&old_hash)
        .fetch_one(pool)
        .await?;
        let form = format!("_csrf={token}&email={email}&password=boundary+test+password");
        let response = browser_request(app, "POST", "/en/login", &cookie, &form).await?;
        assert_eq!(response.status(), 302);
        let rotated = response.headers()[SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .ok_or("rotated cookie")?
            .to_owned();
        Ok((rotated, session_id, old_hash))
    }

    async fn exercise_login_rotation(pool: &PgPool) -> TestResult {
        // Keep the real login-success event and suppress only its referral callback.
        sqlx::query("ALTER TABLE security_events DISABLE TRIGGER referral_activity_security")
            .execute(pool)
            .await?;
        let runtime = oteryn_web::session::configured_pool(&std::env::var("OTERYN_DATABASE_URL")?)?;
        let hash = oteryn_identity::password::hash("boundary test password")?;
        for (email, record) in [
            ("rotation@example.test", true),
            ("no-recorder@example.test", false),
        ] {
            if !record {
                sqlx::query("ALTER TABLE web_sessions DISABLE TRIGGER referral_activity_web")
                    .execute(pool)
                    .await?;
            }
            let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active','en',now()) RETURNING id")
                .bind(email).bind(&hash).fetch_one(pool).await?;
            let (_, session_id, old_hash) = login_browser(&runtime, email).await?;
            let (seen, activity): (time::OffsetDateTime, Option<time::OffsetDateTime>) = sqlx::query_as(
                "SELECT s.last_seen_at,r.last_activity FROM web_sessions s LEFT JOIN referral_activity r USING(account_id)
                 WHERE s.account_id=$1 AND s.session_id=$2 AND s.id_hash<>$3 AND s.authorization_generation=(SELECT native_security_generation FROM accounts WHERE id=$1)",
            ).bind(account).bind(session_id).bind(old_hash).fetch_one(pool).await?;
            assert_eq!(
                activity,
                record.then_some(seen),
                "only the real AuthorizedLogin rotation can supply activity"
            );
            let callbacks: i64 = sqlx::query_scalar("SELECT count(*) FROM security_events WHERE account_id=$1 AND kind='login_succeeded'")
                .bind(account).fetch_one(pool).await?;
            assert_eq!(callbacks, 1);
        }
        Ok(())
    }

    async fn exercise_referrals_on(pool: &PgPool) -> TestResult {
        let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('referral-browser@example.test','test-only','active','en',now()) RETURNING id")
            .fetch_one(pool).await?;
        sqlx::query("DELETE FROM referral_activity WHERE account_id=$1")
            .bind(account)
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,last_seen_at,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,now()-interval '10 minutes','flag-boundary',$3,now()+interval '1 hour',now()+interval '1 day')")
            .bind(Sha256::digest(SESSION).to_vec()).bind(account).bind(CSRF).execute(pool).await?;
        sqlx::query("UPDATE referral_campaign_settings SET enabled=true")
            .execute(pool)
            .await?;
        let serve = tokio::task::spawn_local(async { crate::run(crate::Role::Serve).await });
        let result = async {
            let address = std::env::var("OTERYN_BIND")?.parse()?;
            wait_for_serve(&serve, address).await?;
            let activity = "SELECT r.last_activity,s.last_seen_at FROM referral_activity r JOIN web_sessions s USING(account_id) WHERE r.account_id=$1";
            for _ in 0..2 {
                let response = exchange(address, None, "GET", "/en/account/referrals").await?;
                assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
                let (last_activity, last_seen): (time::OffsetDateTime, time::OffsetDateTime) =
                    sqlx::query_as(activity).bind(account).fetch_one(pool).await?;
                assert_eq!(last_activity, last_seen, "capture the committed session timestamp");
            }
            let newer: time::OffsetDateTime = sqlx::query_scalar("UPDATE referral_activity SET last_activity=now()+interval '1 hour' WHERE account_id=$1 RETURNING last_activity")
                .bind(account).fetch_one(pool).await?;
            let response = exchange(address, None, "GET", "/en/account/referrals").await?;
            assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
            let (last_activity, last_seen): (time::OffsetDateTime, time::OffsetDateTime) =
                sqlx::query_as(activity).bind(account).fetch_one(pool).await?;
            assert!(last_seen < newer, "fixture must precede the stored activity");
            assert_eq!(last_activity, newer, "session activity must not move the high-water mark backwards");
            sqlx::query("DELETE FROM web_sessions WHERE account_id=$1").bind(account).execute(pool).await?;
            let retained: time::OffsetDateTime = sqlx::query_scalar("SELECT last_activity FROM referral_activity WHERE account_id=$1")
                .bind(account).fetch_one(pool).await?;
            assert_eq!(retained, newer, "deleting the session must preserve referral activity");
            Ok(())
        }.await;
        serve.abort();
        let _ = serve.await;
        result
    }

    async fn row_counts(pool: &PgPool) -> TestResult<BTreeSet<(String, i64)>> {
        Ok(sqlx::query_as("SELECT * FROM flag_boundary_counts()")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect())
    }

    async fn take_writes(pool: &PgPool) -> TestResult<Vec<(String, String, i64)>> {
        Ok(sqlx::query_as("WITH writes AS (DELETE FROM flag_boundary_writes RETURNING *) SELECT relation,operation,count(*) FROM writes GROUP BY relation,operation ORDER BY relation,operation")
        .fetch_all(pool).await?)
    }

    async fn exercise_jobs(pool: &PgPool) -> TestResult<Vec<String>> {
        let registry = configured(pool.clone()).await?;
        let expected = RETENTION
            .iter()
            .map(|job| job.0)
            .chain(CORE_JOBS.iter().copied())
            .collect::<BTreeSet<_>>();
        let registered = registry
            .handlers
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut failures = Vec::new();
        let scheduled = registry
            .schedules
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if scheduled != expected {
            failures.push(format!("flag-off schedule mismatch: {scheduled:?}"));
        }
        if registered != expected {
            failures.push(format!(
                "worker registry mismatch: unexpected {:?}, missing {:?}",
                registered.difference(&expected).collect::<Vec<_>>(),
                expected.difference(&registered).collect::<Vec<_>>()
            ));
        }
        let startup_writes = take_writes(pool).await?;
        if !startup_writes.is_empty() {
            failures.push(format!("worker configuration wrote {startup_writes:?}"));
        }
        let before_jobs = row_counts(pool).await?;
        for (kind, handler) in &registry.handlers {
            let job = Job {
                id: Uuid::nil(),
                kind: kind.clone(),
                payload: "{}".into(),
                attempts: 1,
            };
            handler
                .run(&job)
                .await
                .map_err(|error| format!("{kind}: {error}"))?;
            let permission = RETENTION.iter().find(|entry| entry.0 == kind);
            for (table, operation, count) in take_writes(pool).await? {
                let allowed = permission.is_some_and(|(_, _, deletes, expires, creates)| {
                    match operation.as_str() {
                        "DELETE" => deletes.contains(&table.as_str()),
                        "UPDATE" => expires.contains(&table.as_str()),
                        "INSERT" => creates.contains(&table.as_str()),
                        _ => false,
                    }
                });
                if !allowed {
                    let reason = permission.map_or("no feature writes permitted", |job| job.1);
                    failures.push(format!(
                        "{kind} wrote {table}: {operation} ({count} rows), expected {reason}"
                    ));
                }
            }
        }
        let allowed_deletes = RETENTION
            .iter()
            .flat_map(|job| job.2.iter().copied())
            .collect::<BTreeSet<_>>();
        let allowed_inserts = RETENTION
            .iter()
            .flat_map(|job| job.4.iter().copied())
            .collect::<BTreeSet<_>>();
        let after_jobs = row_counts(pool).await?;
        for (table, count) in after_jobs.difference(&before_jobs) {
            let expected_deletion = allowed_deletes.contains(table.as_str())
                && before_jobs
                    .iter()
                    .any(|(old_table, old_count)| old_table == table && old_count > count);
            let expected_insertion = allowed_inserts.contains(table.as_str())
                && before_jobs
                    .iter()
                    .any(|(old_table, old_count)| old_table == table && old_count < count);
            if !expected_deletion && !expected_insertion {
                failures.push(format!("worker changed row count for {table} to {count}"));
            }
        }
        let death_keys: Vec<String> =
            sqlx::query_scalar("SELECT event_key FROM stats_character_events ORDER BY event_key")
                .fetch_all(pool)
                .await?;
        if death_keys != ["1"] {
            failures.push(format!(
                "stats.death-retention must retain only the recent fixture: {death_keys:?}"
            ));
        }
        let mut tx = pool.begin().await?;
        for kind in DISABLED_JOBS
            .iter()
            .copied()
            .chain([oteryn_notify::EMAIL_JOB])
        {
            outbox::enqueue(
                &mut tx,
                kind,
                "{}",
                "flag-boundary",
                outbox::OffsetDateTime::now_utc(),
                2,
            )
            .await?;
        }
        tx.commit().await?;
        take_writes(pool).await?;
        let claimed = registry.batch(pool).await?;
        if claimed != 0 {
            failures.push(format!(
                "worker claimed {claimed} queued jobs with every flag off"
            ));
        }
        let pending: Vec<String> = sqlx::query_scalar(
            "SELECT kind FROM outbox WHERE state <> 'pending' OR attempts <> 0 ORDER BY kind",
        )
        .fetch_all(pool)
        .await?;
        if !pending.is_empty() {
            failures.push(format!("disabled jobs changed state: {pending:?}"));
        }
        let writes = take_writes(pool).await?;
        if !writes.is_empty() {
            failures.push(format!("disabled job dispatch wrote {writes:?}"));
        }
        eprintln!(
            "ran {} registered jobs once and checked {} disabled queue kinds",
            registered.len(),
            DISABLED_JOBS.len() + 1
        );
        Ok(failures)
    }
    const FLAGS: &[&str] = &[
        "GATEWAY_NATIVE_LOGIN_ENABLED",
        "GATEWAY_NATIVE_PROTOCOL3_ENABLED",
        "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED",
        "GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED",
        "GAME_AUTH_NATIVE_EVIDENCE_ENABLED",
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED",
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED",
        "OTERYN_ACCOUNT_BADGES_ENABLED",
        "OTERYN_ACCOUNT_BALANCE_ENABLED",
        "OTERYN_ACCOUNT_CONSENTS_ENABLED",
        "OTERYN_ACCOUNT_CREDENTIALS",
        "OTERYN_ACCOUNT_EMAIL_CHANGE",
        "OTERYN_ACCOUNT_EXPORT_ENABLED",
        "OTERYN_ACCOUNT_LOGIN",
        "OTERYN_ACCOUNT_MFA",
        "OTERYN_ACCOUNT_OVERVIEW_ENABLED",
        "OTERYN_ACCOUNT_PASSKEYS",
        "OTERYN_ACCOUNT_PASSWORD_RESET",
        "OTERYN_ACCOUNT_PRIVACY_ENABLED",
        "OTERYN_ACCOUNT_RECOVERY_KEYS",
        "OTERYN_ACCOUNT_SESSIONS",
        "OTERYN_ADMIN_ANALYTICS_INGEST",
        "OTERYN_ADMIN_ANALYTICS_PRESENT",
        "OTERYN_ADMIN_CHARTS_EXTENDED_ENABLED",
        "OTERYN_ADMIN_COMMERCE",
        "OTERYN_ADMIN_CONSOLE",
        "OTERYN_ADMIN_CONTROL",
        "OTERYN_ADMIN_PANEL",
        "OTERYN_ADMIN_PLAYERS",
        "OTERYN_ADMIN_PLAYER_CHARACTERS",
        "OTERYN_ATLAS_BESTIARY_ENABLED",
        "OTERYN_ATLAS_DELIVERY_ENABLED",
        "OTERYN_ATLAS_ENABLED",
        "OTERYN_ATLAS_SPAWNS_ENABLED",
        "OTERYN_BAZAAR_ENABLED",
        "OTERYN_BAZAAR_MODERATION",
        "OTERYN_BAZAAR_PROFILES",
        "OTERYN_BAZAAR_PROMOTIONS",
        "OTERYN_CHARACTER_COMMANDS_ENABLED",
        "OTERYN_CHARACTER_COMMAND_CREATE_ENABLED",
        "OTERYN_CHARACTER_CREATE_WEB_ENABLED",
        "OTERYN_CHARACTER_PROFILES_ENABLED",
        "OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED",
        "OTERYN_CHARACTER_SERVICES_ENABLED",
        "OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED",
        "OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED",
        "OTERYN_CHARACTER_SERVICES_RENAME_ENABLED",
        "OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED",
        "OTERYN_COLLECTION_ENABLED",
        "OTERYN_COLLECTION_JOURNAL_READ_ENABLED",
        "OTERYN_COLLECTION_PVP_ENABLED",
        "OTERYN_COLLECTION_SETS_ENABLED",
        "OTERYN_DAILY_PUZZLES_ENABLED",
        "OTERYN_DOWNLOADS_ENABLED",
        "OTERYN_JOURNAL_REPORTS_ENABLED",
        "OTERYN_LCFA_ENABLED",
        "OTERYN_LEAGUE_ENABLED",
        "OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED",
        "OTERYN_PAYMENTS_ENABLED",
        "OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED",
        "OTERYN_PLAYERDATA_HUNTS_READ",
        "OTERYN_PLAYERDATA_PLACES_AGGREGATE",
        "OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED",
        "OTERYN_PREMIUM_ENABLED",
        "OTERYN_REFUNDS_ENABLED",
        "OTERYN_REPORTS_ENABLED",
        "OTERYN_SEASON_PASS_ENABLED",
        "OTERYN_SEASON_PASS_WALLET_ENABLED",
        "OTERYN_SECURITY_WIZARD_ENABLED",
        "OTERYN_SHOP_ENABLED",
        "OTERYN_SHOP_TEST_PAYMENTS_ENABLED",
        "OTERYN_SHOP_TOP_UPS_ENABLED",
        "OTERYN_SHOP_TRANSFERS_ENABLED",
        "OTERYN_STORE_CODES_ENABLED",
        "OTERYN_SUPPORT_TICKETS",
        "OTERYN_WALLET_ENABLED",
        "OTERYN_WORLD_ADMIN",
        "OTERYN_WORLD_CATALOG_PUBLICATION_ENABLED",
        "OTERYN_WORLD_ENABLED",
        "PANEL_PREMIUM_PAYMENTS",
        "PLAYERDATA_G10_HISTORY_ENABLED",
        "PLAYERDATA_G10_ITEMS_ENABLED",
        "PORTAL_ACCOUNT_CLOSURE_ENABLED",
        "PORTAL_ACCOUNT_TERMINATION_ENABLED",
        "PORTAL_ADMIN_CONTROL_ENABLED",
        "PORTAL_ATLAS_DELIVERY_ENABLED",
        "PORTAL_CALCULATORS_ENABLED",
        "PORTAL_CHARACTER_COMMANDS_ENABLED",
        "PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED",
        "PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED",
        "PORTAL_CHARACTER_STATS_ENABLED",
        "PORTAL_COIN_MARKET_ENABLED",
        "PORTAL_COIN_PURCHASES_ENABLED",
        "PORTAL_COIN_TRANSFERS_ENABLED",
        "PORTAL_DEATHS_ENABLED",
        "PORTAL_DEATHS_V2_INGEST_ENABLED",
        "PORTAL_G4_ACHIEVEMENTS_INGEST_ENABLED",
        "PORTAL_G4_ACHIEVEMENTS_SERVING_ENABLED",
        "PORTAL_G4_EQUIPMENT_INGEST_ENABLED",
        "PORTAL_G4_EQUIPMENT_SERVING_ENABLED",
        "PORTAL_G4_OUTFIT_INGEST_ENABLED",
        "PORTAL_G4_OUTFIT_SERVING_ENABLED",
        "PORTAL_G4_SKILLS_INGEST_ENABLED",
        "PORTAL_G4_SKILLS_SERVING_ENABLED",
        "PORTAL_GAME_ENFORCEMENT_ENABLED",
        "PORTAL_GAME_RESTORE_NOTICE_ENABLED",
        "PORTAL_GAME_TICKETS_ENABLED",
        "PORTAL_GROWTH_RANKINGS_ENABLED",
        "PORTAL_GUILDS_ENABLED",
        "PORTAL_GUILDS_INGEST_ENABLED",
        "PORTAL_GUILD_STATS_ENABLED",
        "PORTAL_HIRELINGS_ENABLED",
        "PORTAL_HOUSES_INGEST_ENABLED",
        "PORTAL_HOUSES_SERVING_ENABLED",
        "PORTAL_NATIVE_ADMISSIONS_ENABLED",
        "PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED",
        "PORTAL_NATIVE_PROTOCOL3_ENABLED",
        "PORTAL_NATIVE_RECOVERY_ENABLED",
        "PORTAL_ONLINE_ENABLED",
        "PORTAL_PARTY_ENABLED",
        "PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED",
        "PORTAL_PLAYERDATA_G10_INGEST_ENABLED",
        "PORTAL_PLAYERDATA_G9_INGEST_ENABLED",
        "PORTAL_PLAYERDATA_G9_REPLAY_COMMANDS_ENABLED",
        "PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED",
        "PORTAL_PUBLIC_PROFILES_ENABLED",
        "PORTAL_PUBLIC_PROJECTIONS_ENABLED",
        "PORTAL_PUBLIC_RANKINGS_ENABLED",
        "PORTAL_REFERRALS_ENABLED",
        "PORTAL_SANCTION_EXPORT_ENABLED",
        "PORTAL_SECURITY_REVOCATION_EXPORT_ENABLED",
        "PORTAL_STATS_ENABLED",
        "PORTAL_STATS_ONLINE_INTERVALS_ENABLED",
        "PORTAL_STATS_WATCHES_ENABLED",
        "PORTAL_STATS_WORLD_TOTALS_ENABLED",
        "PORTAL_WIKI_ENTITIES_ENABLED",
        "PORTAL_WIKI_PLACES_ENABLED",
        "PORTAL_WORLDS_ENABLED",
        "PORTAL_WORLD_DAILY_INGEST_ENABLED",
        "PORTAL_WORLD_DAILY_PUBLIC_ENABLED",
        "PORTAL_WORLD_ENABLED",
        "PORTAL_WORLD_STATS_ENABLED",
        "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED",
        "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED",
        "PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED",
        "PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED",
        "PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED",
        "PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED",
        "PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED",
    ];

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct Probe {
        listener: Listener,
        method: char,
        path: String,
        status: u16,
        location: Option<String>,
        fallback: bool,
    }

    fn route_probes() -> TestResult<BTreeSet<Probe>> {
        let mut probes = BTreeSet::from(["/admin/system", "/admin/system/data"].map(|path| Probe {
            listener: Listener::Public,
            method: 'G',
            path: path.to_owned(),
            status: 404,
            location: None,
            fallback: false,
        }));
        let inventory = inventory()?;
        for row in inventory_rows(&inventory) {
            let routes = row
                .split('|')
                .nth(2)
                .ok_or("missing inventory route column")?;
            for entry in routes.split('`').skip(1).step_by(2) {
                let fallback = entry.starts_with('~');
                let entry = entry.strip_prefix('~').unwrap_or(entry);
                let (methods, target) = entry.split_once(':').ok_or("expected METHOD:/path")?;
                let (methods, listener) = match methods.split_once('@') {
                    Some((methods, listener)) => (methods, Listener::parse(listener)?),
                    None => (methods, Listener::Public),
                };
                let (methods, status) = methods.find(|c: char| c.is_ascii_digit()).map_or(
                    Ok((methods, 404)),
                    |index| {
                        let (methods, status) = methods.split_at(index);
                        status.parse().map(|status| (methods, status))
                    },
                )?;
                let (path, location) = target
                    .split_once('>')
                    .map_or((target, None), |(path, location)| {
                        (path, Some(location.to_owned()))
                    });
                let mut paths = vec![path.to_owned()];
                for (parameter, values) in [
                    ("{lang}", &["en", "pl"][..]),
                    ("/{prefix}", &["", "/en", "/pl"]),
                    ("{id}", &["019ad000-0000-7000-8000-000000000001"]),
                    ("{name}", &["Boundary"]),
                    ("{token}", &[SESSION]),
                    ("{kind}", &["items", "creatures"]),
                    (
                        "{analytics}",
                        &["creatures", "activity", "levels", "health", "economy"],
                    ),
                    (
                        "{commerce_action}",
                        &["grant", "revoke", "adjust", "reconcile", "refund"],
                    ),
                    (
                        "{agent_op}",
                        &[
                            "hello",
                            "channel",
                            "results",
                            "rollups",
                            "heartbeat",
                            "stream",
                        ],
                    ),
                    ("{bazaar_action}", &["bid", "buy", "cancel"]),
                    (
                        "{bazaar_wire}",
                        &[
                            "commands/pending",
                            "commands/read",
                            "commands/reconcile",
                            "transferability",
                            "status",
                            "watermark",
                        ],
                    ),
                    ("{character_op}", &["pending", "read", "receipt"]),
                    (
                        "{g4_family}",
                        &[
                            "character-skills",
                            "character-achievements",
                            "character-equipment",
                            "character-outfit",
                        ],
                    ),
                    ("{g4_tab}", &["achievements", "equipment", "outfit"]),
                    (
                        "{stats_tab}",
                        &[
                            "history",
                            "experience",
                            "rankings",
                            "deaths",
                            "progress",
                            "online",
                        ],
                    ),
                    (
                        "{projection}",
                        &[
                            "highscores",
                            "character-profile",
                            "character-deaths",
                            "guild",
                            "watermark",
                        ],
                    ),
                    ("{ranking}", &["level", "experience", "magic", "loyalty"]),
                    ("{store_op}", &["pending", "read", "receipt", "reconcile"]),
                ] {
                    if path.contains(parameter) {
                        paths = paths
                            .into_iter()
                            .flat_map(|p| {
                                values.iter().map(move |value| p.replace(parameter, value))
                            })
                            .collect();
                    }
                }
                for path in paths {
                    assert!(!path.contains('{'), "unexpanded route {path}");
                    probes.extend(methods.chars().map(|method| Probe {
                        listener,
                        method,
                        path: path.clone(),
                        status,
                        location: location.clone(),
                        fallback,
                    }));
                }
            }
        }
        // Staff invitations use the existing admin host guard. Public
        // confirmation with a missing token also returns 404.
        for (method, path) in [
            ('G', "/admin/roles/invitations"),
            ('P', "/admin/roles/invitations"),
            ('P', "/admin/roles/invitations/019ad000-0000-7000-8000-000000000001/revoke"),
            ('G', "/pl/staff-invitation"),
            ('P', "/pl/staff-invitation"),
            ('G', "/en/staff-invitation"),
            ('P', "/en/staff-invitation"),
        ] {
            probes.insert(Probe {
                listener: Listener::Public,
                method,
                path: path.into(),
                status: 404,
                location: None,
                fallback: false,
            });
        }
        Ok(probes)
    }

    struct RoutePrerequisites {
        flags: &'static [&'static str],
        paths: &'static [&'static str],
        registration_removed: bool,
        disabled_status: Option<u16>,
    }
    const WORLD_FLAGS: &[&str] = &["OTERYN_WORLD_ENABLED", "PORTAL_WORLD_ENABLED"];
    const ROUTE_PREREQUISITES: &[RoutePrerequisites] = &[
        RoutePrerequisites {
            flags: &["OTERYN_SUPPORT_TICKETS"],
            paths: &["/admin/support/tickets", "/{lang}/admin/support/tickets"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_PANEL"],
            paths: &[
                "/admin/support/tickets",
                "/{lang}/admin/support/tickets",
                "/admin/moderation/reports",
                "/admin/analytics",
                "/admin/commerce",
                "/admin/world",
                "/admin/players",
                "/admin/system",
                "/console",
                "/admin/console",
                "/admin/control",
                "/{lang}/admin/store/codes",
            ],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_REPORTS_ENABLED"],
            paths: &["/admin/moderation/reports"],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_ANALYTICS_PRESENT"],
            paths: &["/admin/analytics"],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_COMMERCE"],
            paths: &["/admin/commerce"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_WORLD_ADMIN"],
            paths: &["/admin/world"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_PLAYERS"],
            paths: &["/admin/players"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_CONSOLE"],
            paths: &["/console", "/admin/console"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ADMIN_CONTROL"],
            paths: &["/admin/control"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_STORE_CODES_ENABLED"],
            paths: &["/{lang}/admin/store/codes"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["PORTAL_ACCOUNT_TERMINATION_ENABLED"],
            paths: &["/{lang}/account/privacy/delete"],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &["OTERYN_ACCOUNT_LOGIN", "OTERYN_ACCOUNT_MFA"],
            paths: &["/mfa", "/{lang}/mfa"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &[
                "OTERYN_CHARACTER_CREATE_WEB_ENABLED",
                "OTERYN_CHARACTER_COMMANDS_ENABLED",
                "OTERYN_CHARACTER_COMMAND_CREATE_ENABLED",
            ],
            paths: &[
                "/{lang}/account/characters/new",
                "/{lang}/account/characters/requests",
                "/{lang}/account/characters/suggest",
            ],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: WORLD_FLAGS,
            paths: &[
                "/atlas/spawns",
                "/atlas/spawns-text",
                "/atlas/private-markers",
                "/atlas/map-reports",
            ],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: WORLD_FLAGS,
            paths: &["/{lang}/atlas", "/atlas/tiles", "/{lang}/atlas/delivery"],
            registration_removed: false,
            disabled_status: Some(503),
        },
        RoutePrerequisites {
            flags: &["OTERYN_ATLAS_ENABLED", "OTERYN_ATLAS_SPAWNS_ENABLED"],
            paths: &[
                "/atlas/spawns",
                "/atlas/spawns-text",
                "/atlas/private-markers",
                "/atlas/map-reports",
            ],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &[
                "OTERYN_ATLAS_DELIVERY_ENABLED",
                "PORTAL_ATLAS_DELIVERY_ENABLED",
            ],
            paths: &["/{lang}/atlas/delivery"],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &[
                "OTERYN_ATLAS_DELIVERY_ENABLED",
                "PORTAL_ATLAS_DELIVERY_ENABLED",
            ],
            paths: &["/atlas/delivery.css"],
            registration_removed: true,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &[
                "OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED",
                "OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED",
            ],
            paths: &["/account/game-logins", "/{lang}/account/game-logins"],
            registration_removed: false,
            disabled_status: None,
        },
        RoutePrerequisites {
            flags: &[
                "OTERYN_SHOP_ENABLED",
                "OTERYN_SHOP_TOP_UPS_ENABLED",
                "OTERYN_SHOP_TEST_PAYMENTS_ENABLED",
            ],
            paths: &["/{lang}/shop/test-checkout"],
            registration_removed: true,
            disabled_status: None,
        },
    ];

    fn in_scope(pattern: &str, path: &str) -> bool {
        let count = pattern.split('/').count();
        let parts = path.split('/').take(count).collect::<Vec<_>>();
        specificity(pattern, &parts.join("/")).is_some()
    }

    async fn exercise_prerequisite(pool: &PgPool, flag: &str) -> TestResult {
        let (account, _) = seed_browser_accounts(pool).await?;
        sqlx::query(
            "INSERT INTO tickets(id,account_id,request_key,category,subject)
            VALUES($1,$1,uuidv7(),'other','route fixture')",
        )
        .bind(account)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO player_reports(id,reporter_account_id,request_key,report_type,category,target_reference)
             VALUES($1,$1,uuidv7(),'player','conduct','route fixture')",
        )
        .bind(account)
        .execute(pool)
        .await?;
        let profiles = std::env::var(ROUTE_PROFILES)?;
        let enabled = route_snapshot(&Path::new(&profiles).join("enabled"))?;
        let disabled = route_snapshot(&Path::new(&profiles).join(flag))?;
        if flag == "PORTAL_ACCOUNT_TERMINATION_ENABLED" {
            let closure = fs::read_to_string(Path::new(&profiles).join(flag).join("closure"))?;
            assert!(
                registered_routes(&closure)?.contains(&(
                    'P',
                    "/internal/v1/game-auth/account-closure/lifecycle/read".into(),
                )),
                "the Game proof listener must survive the intake pause"
            );
        }
        // Classify registrations from this profile as well, even if they are absent from
        // both the all-off and fully enabled profiles.
        let mut routes = enabled.clone();
        routes.extend(route_snapshot(
            &Path::new(&profiles).join("enabled-with-restore"),
        )?);
        routes.extend(route_snapshot(&Path::new(&profiles).join("disabled"))?);
        routes.extend(disabled.iter().cloned());
        let (probes, failures) = classify_routes(&route_probes()?, &routes)?;
        assert!(failures.is_empty(), "{flag}=false: {failures:?}");
        let address = std::env::var("OTERYN_BIND")?.parse()?;
        let mut checks = 0;
        for group in ROUTE_PREREQUISITES
            .iter()
            .filter(|group| group.flags.contains(&flag))
        {
            for path in group.paths {
                let mut count = 0;
                for (listener, method, route) in &enabled {
                    if *listener != Listener::Public || !in_scope(path, route) {
                        continue;
                    }
                    // The atlas page shares its prefix with independent features.
                    if *path == "/{lang}/atlas"
                        && route != path
                        && route != "/{lang}/atlas/delivery"
                    {
                        continue;
                    }
                    let registered = disabled.contains(&(*listener, *method, route.clone()));
                    if group.registration_removed && registered {
                        return Err(format!("{flag}=false left {method} {route} registered").into());
                    }
                    if flag == "OTERYN_REPORTS_ENABLED" {
                        assert!(registered, "forum report moderation must remain available");
                    }
                    for probe in probes.iter().filter(|probe| {
                        probe.listener == *listener
                            && probe.method == *method
                            && specificity(route, &probe.path).is_some()
                    }) {
                        let response =
                            exchange(address, None, http_method(*method)?, &probe.path).await?;
                        let status = if flag == "OTERYN_REPORTS_ENABLED"
                            && probe.path == "/admin/moderation/reports"
                        {
                            assert!(
                                !response.contains("route fixture"),
                                "legacy reports must stay hidden when OTERYN_REPORTS_ENABLED=false"
                            );
                            200
                        } else {
                            group.disabled_status.unwrap_or(probe.status)
                        };
                        assert!(
                            response.starts_with(&format!("HTTP/1.1 {status} ")),
                            "{flag}=false failed to disable {method} {}: {response}",
                            probe.path
                        );
                        checks += 1;
                        count += 1;
                    }
                }
                assert!(
                    count > 0,
                    "{flag} has no enabled production probes for {path}"
                );
            }
        }
        eprintln!(
            "{flag}=false: checked {checks} enabled method/path pairs with other prerequisites on"
        );
        Ok(())
    }

    fn http_method(method: char) -> TestResult<&'static str> {
        Ok(match method {
            'G' => "GET",
            'P' => "POST",
            'U' => "PUT",
            'D' => "DELETE",
            'A' | 'S' => "PATCH",
            'H' => "HEAD",
            'O' => "OPTIONS",
            'T' => "TRACE",
            'C' => "CONNECT",
            _ => return Err("unknown inventory HTTP method".into()),
        })
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Listener {
        Public,
        Native,
        Internal,
        Admission,
        Closure,
        Bazaar,
        League,
    }

    impl Listener {
        fn parse(name: &str) -> TestResult<Self> {
            Ok(match name {
                "public" => Self::Public,
                "native" => Self::Native,
                "internal" => Self::Internal,
                "admission" => Self::Admission,
                "closure" => Self::Closure,
                "bazaar" => Self::Bazaar,
                "league" => Self::League,
                _ => return Err(format!("unknown listener {name}").into()),
            })
        }

        fn bind_key(self) -> &'static str {
            match self {
                Self::Public => "OTERYN_BIND",
                Self::Native => "OTERYN_NATIVE_TLS_BIND",
                Self::Internal | Self::Closure | Self::Bazaar | Self::League => {
                    "OTERYN_INTERNAL_TLS_BIND"
                }
                Self::Admission => "PORTAL_NATIVE_ADMISSION_BIND",
            }
        }

        fn purpose_routes_are_absent_when_disabled(self) -> bool {
            matches!(self, Self::Closure | Self::Bazaar | Self::League)
        }
    }

    type MethodPaths = BTreeSet<(char, String)>;
    type Registration = (Listener, char, String);
    type Registrations = BTreeSet<Registration>;

    fn registered_routes(router: &str) -> TestResult<MethodPaths> {
        // Axum exposes route IDs, paths and explicit method slots through Router's Debug view.
        let routes = router
            .strip_prefix("Router { path_router: PathRouter { routes: {")
            .ok_or("expected Axum path-router registration data")?;
        let (methods, paths) = routes
            .split_once("}, node: Node { paths: {")
            .ok_or("expected Axum route IDs and path map")?;
        let (paths, _) = paths
            .split_once("} } }, fallback_router: ")
            .ok_or("expected end of Axum path map")?;
        let paths: BTreeMap<_, _> = paths
            .split("RouteId(")
            .skip(1)
            .filter_map(|entry| entry.split_once("): \""))
            .filter_map(|(id, entry)| entry.split_once('"').map(|(path, _)| (id, path)))
            .collect();
        let mut registrations = BTreeSet::new();
        for entry in methods.split("RouteId(").skip(1) {
            let (id, method_router) = entry.split_once("): ").ok_or("invalid Axum route ID")?;
            let path = paths.get(id).ok_or("Axum route ID has no path")?;
            if !method_router.starts_with("MethodRouter(MethodRouter { ") {
                // Services and nesting shims have no explicit HTTP method registration.
                assert!(
                    method_router.starts_with("Route("),
                    "unexpected Axum endpoint {method_router}"
                );
                registrations.insert(('S', (*path).to_owned()));
                continue;
            }
            if method_router.contains("allow_header: Skip") {
                registrations.insert(('S', (*path).to_owned()));
            }
            for (slot, method) in [
                ("get", 'G'),
                ("post", 'P'),
                ("put", 'U'),
                ("delete", 'D'),
                ("head", 'H'),
                ("options", 'O'),
                ("patch", 'A'),
                ("trace", 'T'),
                ("connect", 'C'),
            ] {
                let (_, value) = method_router
                    .split_once(&format!("{slot}: "))
                    .ok_or("missing Axum method slot")?;
                if !value.starts_with("None,") {
                    registrations.insert((method, (*path).to_owned()));
                }
            }
        }
        Ok(registrations)
    }

    fn route_snapshot(dir: &Path) -> TestResult<Registrations> {
        let mut routes = Registrations::new();
        for file in fs::read_dir(dir)? {
            let file = file?;
            let name = file.file_name();
            let name = name.to_str().ok_or("invalid route capture name")?;
            // The pre-MFA composition is nested as a fallback on these three listeners.
            let listeners = if name == "mfa-fallback" {
                vec![Listener::Public, Listener::Internal, Listener::Native]
            } else {
                vec![Listener::parse(name)?]
            };
            for (method, path) in registered_routes(&fs::read_to_string(file.path())?)? {
                routes.extend(
                    listeners
                        .iter()
                        .map(|listener| (*listener, method, path.clone())),
                );
            }
        }
        assert!(
            !routes.is_empty(),
            "empty production route snapshot at {}",
            dir.display()
        );
        Ok(routes)
    }

    fn specificity(pattern: &str, target: &str) -> Option<usize> {
        let mut score = 0;
        let mut parts = target.split('/');
        for part in pattern.split('/') {
            let target = parts.next()?;
            if part.starts_with("{*") {
                return Some(score);
            }
            if !part.starts_with('{') {
                if part != target {
                    return None;
                }
                score += 1;
            }
        }
        parts.next().is_none().then_some(score)
    }

    fn explicit_inventory() -> TestResult<(Registrations, MethodPaths)> {
        let mut always = Registrations::new();
        let mut public_internal_contracts = BTreeSet::new();
        for line in fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/FLAG_BOUNDARY_ALWAYS_ON.txt"),
        )?
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let mut fields = line.splitn(3, ' ');
            let listener = fields.next().ok_or("missing listener")?;
            let method = fields
                .next()
                .filter(|method| method.len() == 1)
                .and_then(|method| method.chars().next())
                .ok_or("invalid method")?;
            let path = fields
                .next()
                .filter(|path| path.starts_with('/'))
                .ok_or("invalid path")?
                .to_owned();
            if listener == "shared-internal" {
                assert!(
                    public_internal_contracts.insert((method, path)),
                    "duplicate shared internal contract"
                );
            } else {
                assert!(
                    always.insert((Listener::parse(listener)?, method, path)),
                    "duplicate always-on contract"
                );
            }
        }
        Ok((always, public_internal_contracts))
    }

    fn production_registrations(profiles: &Path) -> TestResult<Registrations> {
        let mut routes = Registrations::new();
        for profile in fs::read_dir(profiles)? {
            routes.extend(route_snapshot(&profile?.path())?);
        }
        Ok(routes)
    }

    fn check_route_classifier(documented: &BTreeSet<Probe>, routes: &Registrations) -> TestResult {
        let mut routes = routes.clone();
        let path = "/flag-boundary-unclassified";
        let extra =
            axum::Router::<()>::new().route(path, axum::routing::put(|| async { "fixture" }));
        for (method, path) in registered_routes(&format!("{extra:?}"))? {
            // A registration common to enabled and disabled profiles must still be classified.
            routes.insert((Listener::Public, method, path));
        }
        let same_path = "/internal/v1/game-auth/character-commands/pending";
        for listener in [Listener::Internal, Listener::Closure] {
            assert!(routes.contains(&(listener, 'P', same_path.into())));
        }
        // A third listener mounting a known method/path needs its own contract.
        routes.insert((Listener::Admission, 'P', same_path.into()));
        let (_, failures) = classify_routes(documented, &routes)?;
        assert!(
            failures.contains(&format!("unclassified Public U {path}")),
            "{failures:?}"
        );
        assert!(
            failures.contains(&format!("unclassified Admission P {same_path}")),
            "{failures:?}"
        );
        Ok(())
    }

    fn classify_routes(
        documented: &BTreeSet<Probe>,
        all: &Registrations,
    ) -> TestResult<(BTreeSet<Probe>, Vec<String>)> {
        let (always, shared_internal) = explicit_inventory()?;
        let mut contracts = documented.clone();
        for probe in documented {
            if probe.listener == Listener::Public {
                contracts.insert(Probe {
                    listener: Listener::Native,
                    ..probe.clone()
                });
                if !probe.path.starts_with("/internal/") {
                    contracts.insert(Probe {
                        listener: Listener::Internal,
                        status: 404,
                        location: None,
                        ..probe.clone()
                    });
                }
            }
        }
        // A small set of historically merged internal guards are mounted by the
        // browser router as well. Require explicit listener/path pairs for those.
        let mut targets: BTreeMap<(Listener, &str), Vec<&Probe>> = BTreeMap::new();
        for probe in &contracts {
            targets
                .entry((probe.listener, &probe.path))
                .or_default()
                .push(probe);
        }
        let mut classified = BTreeSet::new();
        let mut generated = BTreeSet::new();
        let mut failures = Vec::new();
        for ((listener, target), expected) in targets {
            let matches = all
                .iter()
                .filter_map(|route| {
                    (route.0 == listener)
                        .then(|| specificity(&route.2, target))
                        .flatten()
                        .map(|score| (score, route))
                })
                .collect::<Vec<_>>();
            let best = matches.iter().map(|(score, _)| *score).max();
            let mut registered = BTreeSet::new();
            for (_, route) in matches
                .into_iter()
                .filter(|(score, _)| Some(*score) == best)
            {
                let method = route.1;
                registered.insert(method);
                if let Some(contract) = expected
                    .iter()
                    .find(|probe| probe.method == method && !probe.fallback)
                {
                    classified.insert(route.clone());
                    generated.insert((*contract).clone());
                } else if !always.contains(route) {
                    failures.push(format!(
                        "registered {listener:?} {method} {} has no feature contract",
                        route.2
                    ));
                }
            }
            for probe in expected {
                if documented.contains(probe)
                    && !probe.fallback
                    && !registered.contains(&probe.method)
                {
                    failures.push(format!(
                        "documented {listener:?} {} {target} has no matching registration",
                        probe.method
                    ));
                }
                if probe.fallback || documented.contains(probe) {
                    generated.insert(probe.clone());
                }
            }
        }
        for &(ref listener, method, ref path) in all {
            let registration = (*listener, method, path.clone());
            if classified.contains(&registration) || always.contains(&registration) {
                continue;
            }
            let mirrored = (*listener == Listener::Native
                || *listener == Listener::Internal && !path.starts_with("/internal/"))
                && always.contains(&(Listener::Public, method, path.clone()));
            if mirrored {
                continue;
            }
            if matches!(listener, Listener::Public | Listener::Native)
                && shared_internal.contains(&(method, path.clone()))
            {
                let mut matches_contract = false;
                for probe in documented.iter().filter(|probe| {
                    probe.listener == Listener::Internal
                        && probe.method == method
                        && !probe.fallback
                        && specificity(path, &probe.path).is_some()
                }) {
                    generated.insert(Probe {
                        listener: *listener,
                        ..probe.clone()
                    });
                    matches_contract = true;
                }
                if matches_contract {
                    continue;
                }
            }
            failures.push(format!("unclassified {listener:?} {method} {path}"));
        }
        failures.sort();
        failures.dedup();
        Ok((generated, failures))
    }

    async fn exchange(
        address: SocketAddr,
        connector: Option<&tokio_rustls::TlsConnector>,
        method: &str,
        path: &str,
    ) -> TestResult<String> {
        async fn send(
            mut stream: impl AsyncReadExt + AsyncWriteExt + Unpin,
            request: &str,
        ) -> TestResult<String> {
            stream.write_all(request.as_bytes()).await?;
            let mut bytes = Vec::new();
            match stream.read_to_end(&mut bytes).await {
                Err(error)
                    if error.kind() == std::io::ErrorKind::UnexpectedEof && !bytes.is_empty() => {}
                result => {
                    result?;
                }
            }
            Ok(String::from_utf8(bytes)?)
        }

        let has_body = matches!(method, "POST" | "PUT" | "DELETE" | "PATCH");
        let json = path.starts_with("/internal/")
            || path.starts_with("/api/")
            || path.starts_with("/__test/");
        let staff =
            path.starts_with("/admin") || path.starts_with("/console") || path.contains("/admin/");
        let (host, session) = if staff {
            ("staff.localhost", STAFF_SESSION)
        } else {
            ("localhost", SESSION)
        };
        let fields = if path.ends_with("/account/privacy/delete/cancel") {
            "&current_password=boundary+test+password"
        } else if path.contains("/support/tickets/") && path.ends_with("/status") {
            "&lock_version=1&status=closed"
        } else if path.starts_with("/admin/support/tickets/") {
            "&lock_version=1&action=ticket_status&status=closed"
        } else if path.starts_with("/admin/moderation/reports/") {
            "&lock_version=1&action=moderate&status=reviewing&public_outcome=reviewing&moderator_notes=fixture"
        } else if path.starts_with("/admin/players/") {
            "&action=note&body=unexpected+game+staff+note"
        } else if path.starts_with("/admin/moderation/enforcement/") {
            "&lock_version=1&action=sanction&create=true&status=active&target=019ad000-0000-7000-8000-000000000001&category=suspension&public_reason=Test&moderator_notes=&effective_at=100&expires_at=200"
        } else if path.ends_with("/account/consents") {
            "&kind=analytics&version=flag-boundary&granted=on"
        } else if path.ends_with("/journal/reports/notes") {
            "&body=flag-boundary"
        } else if path.ends_with("/journal/reports/email") {
            "&enabled=true"
        } else {
            ""
        };
        let csrf = if path.ends_with("/account/notifications") {
            ""
        } else {
            CSRF
        };
        let form = format!("_csrf={csrf}{fields}");
        let target = if path.ends_with("/journal/reports/notes") {
            format!("{path}?kind=session&id=019ad000-0000-7000-8000-000000000001")
        } else {
            path.to_owned()
        };
        let (content_type, body) = if !has_body {
            ("text/plain", "")
        } else if json {
            ("application/json", "{}")
        } else {
            ("application/x-www-form-urlencoded", form.as_str())
        };
        let request = format!(
            "{method} {target} HTTP/1.1\r\nHost: {host}\r\nOrigin: https://{host}\r\nCookie: __Host-oteryn_session={session}\r\nAuthorization: Bearer {SESSION}\r\nX-CSRF-Token: {csrf}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            let stream = tokio::net::TcpStream::connect(address).await?;
            stream.set_nodelay(true)?;
            if let Some(connector) = connector {
                let stream = connector.connect("localhost".try_into()?, stream).await?;
                send(stream, &request).await
            } else {
                send(stream, &request).await
            }
        })
        .await?
    }

    fn client_tls() -> TestResult<tokio_rustls::TlsConnector> {
        client_tls_for("client")
    }

    fn client_tls_for(name: &str) -> TestResult<tokio_rustls::TlsConnector> {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
        let path = std::path::PathBuf::from(std::env::var_os(CHILD).ok_or("TLS fixture path")?);
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(path.join("ca.pem"))?)?;
        let mut client = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_client_auth_cert(
            vec![CertificateDer::from_pem_file(
                path.join(format!("{name}.pem")),
            )?],
            PrivateKeyDer::from_pem_file(path.join(format!("{name}.key")))?,
        )?;
        client.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(tokio_rustls::TlsConnector::from(Arc::new(client)))
    }

    async fn wait_for_serve(
        serve: &tokio::task::JoinHandle<TestResult>,
        public: SocketAddr,
    ) -> TestResult {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if serve.is_finished() {
                    return Err("serve stopped before readiness");
                }
                if exchange(public, None, "GET", "/healthz")
                    .await
                    .is_ok_and(|r| r.starts_with("HTTP/1.1 200 "))
                {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await??;
        Ok(())
    }

    async fn exercise_routes(
        serve: &tokio::task::JoinHandle<TestResult>,
    ) -> TestResult<Vec<String>> {
        let public = std::env::var("OTERYN_BIND")?.parse()?;
        let internal = std::env::var("OTERYN_INTERNAL_TLS_BIND")?.parse()?;
        let admission = std::env::var("PORTAL_NATIVE_ADMISSION_BIND")?.parse()?;
        let tls = client_tls()?;
        wait_for_serve(serve, public).await?;
        assert!(
            exchange(admission, None, "GET", "/health")
                .await?
                .starts_with("HTTP/1.1 200 ")
        );
        assert!(
            exchange(
                internal,
                Some(&tls),
                "POST",
                "/internal/v1/flag-boundary-missing"
            )
            .await?
            .starts_with("HTTP/1.1 404 ")
        );
        let profiles = std::env::var(ROUTE_PROFILES)?;
        let all = production_registrations(Path::new(&profiles))?;
        let documented = route_probes()?;
        check_route_classifier(&documented, &all)?;
        let (probes, mut failures) = classify_routes(&documented, &all)?;
        for key in RETIRED_BINDS {
            let address: SocketAddr = std::env::var(key)?.parse()?;
            match tokio::net::TcpStream::connect(address).await {
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => (),
                Err(error) => return Err(error.into()),
                Ok(_) => failures.push(format!("retired listener {key} accepted a connection")),
            }
        }
        for probe in &probes {
            let path = &probe.path;
            let method = http_method(probe.method)?;
            if probe.listener.purpose_routes_are_absent_when_disabled() {
                // The corresponding internal fallback probe covers the disabled purpose.
                continue;
            }
            let address = std::env::var(probe.listener.bind_key())?.parse()?;
            let connector =
                matches!(probe.listener, Listener::Internal | Listener::Native).then_some(&tls);
            let response = exchange(address, connector, method, path).await?;
            let status = response
                .split_ascii_whitespace()
                .nth(1)
                .unwrap_or("missing");
            let location_matches = probe.location.as_ref().is_none_or(|expected| {
                response.lines().any(|line| {
                    line.split_once(": ").is_some_and(|(name, value)| {
                        name.eq_ignore_ascii_case("location") && value == expected
                    })
                })
            });
            let protocol_matches =
                probe.status != 401 || response.contains("game_login_unavailable");
            if status.parse::<u16>().ok() != Some(probe.status)
                || !location_matches
                || !protocol_matches
            {
                failures.push(format!(
                    "{:?} {method} {path}: expected HTTP {}, received {status}",
                    probe.listener, probe.status
                ));
            }
        }
        eprintln!("probed {} method/path pairs", probes.len());
        Ok(failures)
    }
}
