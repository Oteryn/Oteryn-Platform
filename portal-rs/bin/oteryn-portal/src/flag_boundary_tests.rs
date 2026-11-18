mod flag_boundary {
    //! Exercise the production serve role and worker registry in a disposable schema.
    use super::super::*;
    use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
    use sha2::{Digest, Sha256};
    use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
    use std::{
        collections::BTreeSet,
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
    const CLOSED_LISTENERS: &[&str] = &[
        "OTERYN_BAZAAR_INTERNAL_BIND",
        "OTERYN_LEAGUE_BIND",
        "PORTAL_ACCOUNT_CLOSURE_BIND",
    ];
    const LISTENER_SETTINGS: &[&str] = &[
        "OTERYN_BIND",
        "OTERYN_INTERNAL_TLS_BIND",
        "PORTAL_NATIVE_ADMISSION_BIND",
        "OTERYN_BAZAAR_INTERNAL_BIND",
        "OTERYN_LEAGUE_BIND",
        "PORTAL_ACCOUNT_CLOSURE_BIND",
    ];
    const SESSION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STAFF_SESSION: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const CSRF: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const INVENTORY: &str = include_str!("../../../docs/FLAG_BOUNDARY.md");

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
        let inventory = FLAGS.iter().copied().collect::<BTreeSet<_>>();
        for flag in &found {
            assert!(
                inventory.contains(flag.as_str()),
                "undocumented source flag {flag}"
            );
        }
        for flag in FLAGS {
            assert!(
                INVENTORY
                    .lines()
                    .any(|line| line.starts_with("| `") && line.contains(&format!("`{flag}`"))),
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
    /// The table lists name every relation on which the job may delete or expire rows.
    type RetentionJob = (
        &'static str,
        &'static str,
        &'static [&'static str],
        &'static [&'static str],
    );
    const RETENTION: &[RetentionJob] = &[
        (
            "cleanup.deaths",
            "remove old public death details",
            &[],
            &["public_deaths_v2"],
        ),
        (
            "cleanup.game-logins",
            "remove old game login history",
            &["pd_game_logins"],
            &[],
        ),
        (
            "cleanup.game-ticket-budgets",
            "remove old ticket budget windows",
            &["game_ticket_budgets"],
            &[],
        ),
        (
            "cleanup.hunting-places",
            "expire aggregate inputs and replay fences",
            &["pd_places_sessions", "pd_places_replay_fences"],
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
        ),
        (
            "cleanup.mfa",
            "expire MFA challenges and grants",
            &["account_mfa_tokens"],
            &[],
        ),
        (
            "cleanup.passkeys",
            "expire authentication challenges",
            &["passkey_ceremonies"],
            &[],
        ),
        (
            "cleanup.rate-limits",
            "expire shared request budgets",
            &["web_rate_limits"],
            &[],
        ),
        (
            "cleanup.sessions",
            "expire shared web sessions",
            &["web_sessions"],
            &[],
        ),
        (
            "cleanup.tokens",
            "expire email verification history",
            &["account_email_tokens"],
            &[],
        ),
        (
            "cleanup.world-kills",
            "retire expired world kills payloads",
            &[],
            &["stats_world_kills_v2"],
        ),
        (
            "community.today.retention",
            "remove expired community snapshots",
            &[],
            &["today_snapshots"],
        ),
        (
            "identity.account-closure-audit",
            "expire closure audit history",
            &["plat3_closure_audit_outbox"],
            &[],
        ),
        (
            "league.retention",
            "expire league audit and feed data",
            &["league_audit", "league_feed_inbox"],
            &["league_subjects", "league_feed_requests"],
        ),
        (
            "stats.death-retention",
            "enforce the 30-day copied-death retention window",
            &["stats_character_events"],
            &[],
        ),
        (
            "stats.v2-retention",
            "erase expired statistics payloads",
            &[],
            &["stats_v2_subjects"],
        ),
    ];
    const CORE_JOBS: &[&str] = &["cleanup.support"];
    const DISABLED_JOBS: &[&str] = &[
        "admin.control.expire",
        "bazaar.authority",
        "cleanup.support-tickets",
        "commerce.refund",
        "email.account_exists",
        "email.password-reset",
        "email.verify",
        "identity.email-change",
        "identity.finalize-terminations",
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

    fn run_case(test: &str, referrals_enabled: bool) -> TestResult {
        if std::env::var_os(CHILD).is_some() {
            return tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(
                    tokio::task::LocalSet::new().run_until(exercise_boundary(referrals_enabled)),
                );
        }
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
            .env("FLAG_BOUNDARY_SCHEMA", &schema);
        drop(listeners);
        let output = command.output()?;
        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        assert!(
            output.status.success() && stdout.contains("test result: ok. 1 passed"),
            "{stdout}\n{stderr}"
        );
        eprint!("{stderr}");
        Ok(())
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
        ] {
            let mut params = CertificateParams::new(vec!["localhost".into()])?;
            params.extended_key_usages.push(purpose);
            let key = KeyPair::generate()?;
            let cert = params.signed_by(&key, &ca, &ca_key)?;
            fs::write(path.join(format!("{name}.pem")), cert.pem())?;
            fs::write(path.join(format!("{name}.key")), key.serialize_pem())?;
        }
        Ok(())
    }

    async fn exercise_boundary(referrals_enabled: bool) -> TestResult {
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
            if referrals_enabled {
                exercise_referrals_on(&pool).await
            } else {
                exercise_schema(&pool).await
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

    async fn exercise_schema(pool: &PgPool) -> TestResult {
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
        sqlx::query("INSERT INTO stats_character_events(character_ref,day,kind,event_key,details) SELECT $1,current_date-age,'death',age::text,jsonb_build_object('occurred_at',extract(epoch FROM now()-make_interval(days=>age))::bigint) FROM (VALUES (1),(31)) AS ages(age)")
        .bind(account).execute(pool).await?;

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

        let baseline = row_counts(pool).await?;
        let serve = tokio::task::spawn_local(async { crate::run(crate::Role::Serve).await });
        let routes = exercise_routes(&serve).await;
        serve.abort();
        let _ = serve.await;
        let writes = take_writes(pool).await?;
        let mut failures = routes?;
        if !writes.is_empty() {
            failures.push(format!("serve wrote {writes:?}"));
        }
        let after_routes = row_counts(pool).await?;
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
        exercise_referral_callbacks_off(pool, staff).await?;
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
                let allowed =
                    permission.is_some_and(|(_, _, deletes, expires)| match operation.as_str() {
                        "DELETE" => deletes.contains(&table.as_str()),
                        "UPDATE" => expires.contains(&table.as_str()),
                        _ => false,
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
        let after_jobs = row_counts(pool).await?;
        for (table, count) in after_jobs.difference(&before_jobs) {
            let expected_deletion = allowed_deletes.contains(table.as_str())
                && before_jobs
                    .iter()
                    .any(|(old_table, old_count)| old_table == table && old_count > count);
            if !expected_deletion {
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

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct Probe {
        method: char,
        path: String,
        status: u16,
        location: Option<String>,
    }

    fn route_probes() -> TestResult<BTreeSet<Probe>> {
        let mut probes = BTreeSet::new();
        for row in INVENTORY.lines().filter(|line| line.starts_with("| `")) {
            let routes = row
                .split('|')
                .nth(2)
                .ok_or("missing inventory route column")?;
            for entry in routes.split('`').skip(1).step_by(2) {
                let (methods, target) = entry.split_once(':').ok_or("expected METHOD:/path")?;
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
                    (
                        "{g4_tab}",
                        &["skills", "achievements", "equipment", "outfit"],
                    ),
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
                        method,
                        path: path.clone(),
                        status,
                        location: location.clone(),
                    }));
                }
            }
        }
        Ok(probes)
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

        let has_body = matches!(method, "POST" | "PUT" | "DELETE");
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
        let fields = if path.ends_with("/account/consents") {
            "&kind=analytics&version=flag-boundary&granted=on"
        } else if path.ends_with("/journal/reports/notes") {
            "&body=flag-boundary"
        } else if path.ends_with("/journal/reports/email") {
            "&enabled=true"
        } else {
            ""
        };
        let form = format!("_csrf={CSRF}{fields}");
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
            "{method} {target} HTTP/1.1\r\nHost: {host}\r\nOrigin: https://{host}\r\nCookie: __Host-oteryn_session={session}\r\nAuthorization: Bearer {SESSION}\r\nX-CSRF-Token: {CSRF}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
        let path = std::path::PathBuf::from(std::env::var_os(CHILD).ok_or("TLS fixture path")?);
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(path.join("ca.pem"))?)?;
        let client = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_client_auth_cert(
            vec![CertificateDer::from_pem_file(path.join("client.pem"))?],
            PrivateKeyDer::from_pem_file(path.join("client.key"))?,
        )?;
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
        let probes = route_probes()?;
        let mut failures = Vec::new();
        for key in CLOSED_LISTENERS {
            let address: SocketAddr = std::env::var(key)?.parse()?;
            match tokio::net::TcpStream::connect(address).await {
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => (),
                Err(error) => return Err(error.into()),
                Ok(_) => failures.push(format!("disabled listener {key} accepted a connection")),
            }
        }
        for probe in &probes {
            let path = &probe.path;
            let method = match probe.method {
                'G' => "GET",
                'P' => "POST",
                'U' => "PUT",
                'D' => "DELETE",
                _ => return Err("unknown inventory HTTP method".into()),
            };
            let (address, connector) = if path
                .starts_with("/internal/v1/game-auth/native-admissions")
                || path.starts_with("/internal/v1/game-auth/native-recoveries")
            {
                (admission, None)
            } else if path.starts_with("/internal/") {
                (internal, Some(&tls))
            } else {
                (public, None)
            };
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
                    "{method} {path}: expected HTTP {}, received {status}",
                    probe.status
                ));
            }
        }
        eprintln!("probed {} method/path pairs", probes.len());
        Ok(failures)
    }
}
