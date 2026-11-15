use super::*;
use crate::{keys, runtime};
use sqlx::{PgConnection, postgres::PgPoolOptions};
use std::{cell::RefCell, sync::Arc, time::Duration};
use uuid::Uuid;
use zeroize::Zeroizing;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Default)]
struct Memory(RefCell<Option<Zeroizing<Vec<u8>>>>);
impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Memory([redacted])")
    }
}
impl keys::KeyStorage for Memory {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, keys::KeyError> {
        Ok(self.0.borrow().clone())
    }
    fn save(&self, bytes: &[u8]) -> Result<(), keys::KeyError> {
        *self.0.borrow_mut() = Some(Zeroizing::new(bytes.to_vec()));
        Ok(())
    }
}
#[derive(Debug)]
struct Accounts;
impl AccountStateLookup for Accounts {
    fn state<'a>(&'a self, db: &'a mut PgConnection, account: Uuid) -> Future<'a, AccountState> {
        Box::pin(async move {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                    .bind(account)
                    .fetch_optional(db)
                    .await
                    .map_err(|_| Error::Unavailable)?;
            Ok(if status.as_deref() == Some("active") {
                AccountState::Active
            } else {
                AccountState::Unknown
            })
        })
    }
}
struct Fixture {
    db: runtime::tests::Database,
    request: AdmissionRequest,
    authority: NativeTickets,
    signer: Arc<dyn GrantSigner>,
    directory: std::path::PathBuf,
    account: Uuid,
}
impl Fixture {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db = runtime::tests::Database::new()
            .await?
            .ok_or("PORTAL_TEST_DATABASE_URL required")?;
        db.assignments().await?;
        let world: Uuid = runtime::tests::WORLD.parse()?;
        let channel: Uuid = runtime::tests::CHANNEL.parse()?;
        let account = Uuid::now_v7();
        let character = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'test-only','active','en',now())")
            .bind(account).bind(format!("{account}@example.test")).execute(&db.pool).await?;
        sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='game',complete_through=floor(extract(epoch FROM clock_timestamp())),observed_at=ceil(extract(epoch FROM clock_timestamp()))")
            .execute(&db.pool).await?;
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'game','1','1',0,decode(repeat('ab',32),'hex'))")
            .bind(account).execute(&db.pool).await?;
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Player','AVAILABLE')")
            .bind(account).bind(character).bind(world).execute(&db.pool).await?;
        let revision = grant::route_revision(
            1,
            world,
            channel,
            "node.example.test",
            7172,
            "node.example.test",
        )?;
        sqlx::query("INSERT INTO game_channels(world_id,channel_id,channel_key,native_login_enabled,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision) VALUES($1,$2,'alpha',true,'node.example.test',7172,'node.example.test',1,$3)")
            .bind(world).bind(channel).bind(&revision).execute(&db.pool).await?;
        let at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&db.pool)
                .await?;
        let mut report = runtime::tests::value("1", at).map_err(runtime::tests::TestError::from)?;
        if let oteryn_internal_api::canonical::Value::Object(fields) = &mut report {
            for (name, value) in fields {
                if name == "route_revision" {
                    *value = oteryn_internal_api::canonical::Value::String(revision.clone());
                }
            }
        }
        let report = oteryn_internal_api::canonical::encode(&report)?;
        // Ingestion owns the assignment lock and validates the report, just as production.
        let response = runtime::router(
            db.pool.clone(),
            runtime::tests::settings()?,
            runtime::tests::identities()?,
        )
        .oneshot(runtime::tests::request(
            "/internal/v1/game-auth/native-runtime-status",
            Some("node"),
            report,
        )?)
        .await?;
        assert!(response.status().is_success());
        let ticket = format!("audit05-ticket-{}", Uuid::now_v7());
        let request = AdmissionRequest::decode(format!(r#"{{"protocol_version":2,"game_login_ticket":"{ticket}","attempt_ref":"{}","character_id":"{character}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#,Uuid::now_v7()).as_bytes())?;
        sqlx::query("INSERT INTO game_login_tickets(ticket_hash,account_id,game_generation,native_generation,audience,expires_at) VALUES($1,$2,'1','1','oteryn-native-game-gateway',clock_timestamp()+interval '60 seconds')")
            .bind(request.ticket_hash()).bind(account).execute(&db.pool).await?;
        let directory = std::env::temp_dir().join(format!("audit05-{}", Uuid::now_v7()));
        std::fs::create_dir(&directory)?;
        let settings = keys::Settings {
            enabled: true,
            key_purpose: "fresh_entry".into(),
            clock_uncertainty_seconds: 0,
        };
        let registry =
            keys::PublicTrustRegistry::new(db.pool.clone(), settings.clone(), directory.clone())?;
        let mut ring = keys::Keyring::create(settings, Memory::default())?;
        let key = ring.generate_next("audit05")?;
        registry.publish(&key).await?;
        sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=trusted_at-interval '60 seconds'").execute(&db.pool).await?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&db.pool)
                .await?;
        ring.activate("audit05", now as u64, &registry.snapshot().await?)?;
        let signer: Arc<dyn GrantSigner> = Arc::new(KeyringSigner::new(ring, registry));
        let authority = NativeTickets {
            projection_authority: "game".into(),
            issuance: Issuance {
                enabled: true,
                environment: "testing".into(),
                runtime: runtime::tests::settings()?,
                signer: Some(signer.clone()),
                ..Issuance::default()
            },
        };
        Ok(Self {
            db,
            request,
            authority,
            signer,
            directory,
            account,
        })
    }
    async fn close(self) -> TestResult {
        self.db.close().await?;
        std::fs::remove_dir_all(self.directory)?;
        Ok(())
    }
}
use tower::ServiceExt;

#[tokio::test]
async fn audit05_01_real_admission_empty_cache_one_connection() -> TestResult {
    let mut f = Fixture::new().await?;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&f.db.pool)
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |db, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(schema)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
        .await?;
    // The signer registry deliberately retains the same single-slot pool.
    let settings = keys::Settings {
        enabled: true,
        key_purpose: "fresh_entry".into(),
        clock_uncertainty_seconds: 0,
    };
    let registry =
        keys::PublicTrustRegistry::new(pool.clone(), settings.clone(), f.directory.clone())?;
    let mut ring = keys::Keyring::create(settings, Memory::default())?;
    let key = ring.generate_next("single-slot")?;
    registry.publish(&key).await?;
    sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=trusted_at-interval '60 seconds'").execute(&pool).await?;
    let at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&pool)
        .await?;
    ring.activate("single-slot", at as u64, &registry.snapshot().await?)?;
    f.signer = Arc::new(KeyringSigner::new(ring, registry));
    f.authority.issuance.signer = Some(f.signer.clone());
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        store::admit(
            &pool,
            &f.request,
            &Accounts,
            &f.authority,
            Some(f.signer.as_ref()),
            None,
        ),
    )
    .await?;
    assert!(result.is_ok(), "{result:?}");
    pool.close().await;
    f.close().await
}

#[derive(Debug)]
struct DelayedSigner(Arc<dyn GrantSigner>);
impl GrantSigner for DelayedSigner {
    fn prepare(&self) -> Future<'_, ()> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            self.0.prepare().await
        })
    }
    fn sign(&self, payload: &[u8], expires: u64, now: u64) -> Result<String, Error> {
        self.0.sign(payload, expires, now)
    }
    fn resign(&self, input: &str, expires: u64, now: u64) -> Result<String, Error> {
        self.0.resign(input, expires, now)
    }
}
#[tokio::test]
async fn audit05_05_replay_expiry_after_preparation() -> TestResult {
    let f = Fixture::new().await?;
    let first = store::admit(
        &f.db.pool,
        &f.request,
        &Accounts,
        &f.authority,
        Some(f.signer.as_ref()),
        None,
    )
    .await;
    assert!(first.is_ok(), "{first:?}");
    sqlx::query("UPDATE native_admissions SET expires_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+1800")
        .execute(&f.db.pool).await?;
    let signer = DelayedSigner(f.signer.clone());
    let result = store::admit(
        &f.db.pool,
        &f.request,
        &Accounts,
        &f.authority,
        Some(&signer),
        None,
    )
    .await;
    assert_eq!(result.err().map(|r| r.error), Some(Error::GrantExpired));
    f.close().await
}

#[tokio::test]
async fn audit05_09_overflow_refuses_entire_world() -> TestResult {
    let f = Fixture::new().await?;
    let world: Uuid = runtime::tests::WORLD.parse()?;
    for index in 0..64 {
        let channel = Uuid::now_v7();
        let host = format!("node-{index}.example.test");
        let revision = grant::route_revision(1, world, channel, &host, 7172, &host)?;
        sqlx::query("INSERT INTO game_channels(world_id,channel_id,channel_key,native_login_enabled,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision) VALUES($1,$2,$3,true,$4,7172,$4,1,$5)")
            .bind(world).bind(channel).bind(format!("channel-{index}")).bind(host).bind(revision).execute(&f.db.pool).await?;
    }
    let mut tx = f.db.pool.begin().await?;
    // Every route lacks fresh evidence at this time; preserve the fail-closed
    // selection assertion without imposing an undocumented World capacity.
    let result = grant::select3(
        &mut tx,
        &f.authority.issuance.runtime,
        world,
        None,
        2000000000,
    )
    .await;
    assert_eq!(result.err(), Some(Error::RouteUnavailable));
    tx.rollback().await?;
    f.close().await
}

#[tokio::test]
async fn audit05_04_replay_waits_for_account_before_attempt() -> TestResult {
    let f = Fixture::new().await?;
    let first = store::admit(
        &f.db.pool,
        &f.request,
        &Accounts,
        &f.authority,
        Some(f.signer.as_ref()),
        None,
    )
    .await;
    assert!(first.is_ok(), "{first:?}");
    let mut writer = f.db.pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(f.account)
        .execute(&mut *writer)
        .await?;
    let pool = f.db.pool.clone();
    let authority = f.authority.clone();
    let signer = f.signer.clone();
    let ticket = f.request.ticket();
    let request = AdmissionRequest::decode(format!(r#"{{"protocol_version":2,"game_login_ticket":"{ticket}","attempt_ref":"{}","character_id":"{}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#,f.request.attempt_ref,f.request.character_id).as_bytes())?;
    let task = tokio::spawn(async move {
        store::admit(
            &pool,
            &request,
            &Accounts,
            &authority,
            Some(signer.as_ref()),
            None,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2),async {
        loop {
            // PostgreSQL caches activity observations for this transaction.
            // Refresh them so the barrier can observe a waiter arriving later.
            sqlx::query("SELECT pg_stat_clear_snapshot()")
                .execute(&mut *writer).await?;
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%accounts WHERE id=%' AND pid<>pg_backend_pid())").fetch_one(&mut *writer).await?;
            if waiting { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_,sqlx::Error>(())
    }).await??;
    sqlx::query("SET LOCAL lock_timeout='300ms'")
        .execute(&mut *writer)
        .await?;
    sqlx::query("UPDATE accounts SET password_hash='changed-test-password' WHERE id=$1")
        .bind(f.account)
        .execute(&mut *writer)
        .await?;
    writer.commit().await?;
    assert_eq!(
        task.await?.err().map(|r| r.error),
        Some(Error::TicketRejected)
    );
    f.close().await
}

#[tokio::test]
async fn audit05_05_fresh_expiry_after_database_work() -> TestResult {
    let mut f = Fixture::new().await?;
    f.authority.issuance.ttl_seconds = 5;
    // Delay the actual final insert until only half a second remains.
    sqlx::raw_sql("CREATE FUNCTION delay_admission_insert() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_sleep(greatest(0,(NEW.expires_at_ms-extract(epoch FROM clock_timestamp())*1000-500)/1000)); RETURN NEW; END; $$; CREATE TRIGGER delay_admission BEFORE INSERT ON native_admissions FOR EACH ROW EXECUTE FUNCTION delay_admission_insert()")
        .execute(&f.db.pool).await?;
    let result = store::admit(
        &f.db.pool,
        &f.request,
        &Accounts,
        &f.authority,
        Some(f.signer.as_ref()),
        None,
    )
    .await;
    assert_eq!(result.err().map(|r| r.error), Some(Error::GrantExpired));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions")
        .fetch_one(&f.db.pool)
        .await?;
    assert_eq!(count, 0);
    f.close().await
}

#[tokio::test]
async fn audit05_05_handle_replacement_rechecks_expiry_after_lock_wait() -> TestResult {
    let f = Fixture::new().await?;
    let first = store::admit(
        &f.db.pool,
        &f.request,
        &Accounts,
        &f.authority,
        Some(f.signer.as_ref()),
        None,
    )
    .await;
    assert!(first.is_ok(), "{first:?}");
    sqlx::query("INSERT INTO native_recovery_handles(handle_hash,account_id,character_id,world_id,channel_id,security_generation,play_generation,attempt_ref,issued_at,expires_at) VALUES(decode(repeat('ab',32),'hex'),$1,$2,$3,$4,1,1,$5,floor(extract(epoch FROM clock_timestamp()))::bigint,floor(extract(epoch FROM clock_timestamp()))::bigint+2)")
        .bind(f.account).bind(f.request.character_id).bind(runtime::tests::WORLD.parse::<Uuid>()?).bind(runtime::tests::CHANNEL.parse::<Uuid>()?).bind(f.request.attempt_ref).execute(&f.db.pool).await?;
    let mut writer = f.db.pool.begin().await?;
    let expires: i64 =
        sqlx::query_scalar("SELECT expires_at FROM native_recovery_handles FOR UPDATE")
            .fetch_one(&mut *writer)
            .await?;
    let mut reader = f.db.pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *reader)
        .await?;
    let attempt = f.request.attempt_ref;
    let task = tokio::spawn(async move {
        protocol3::replace(
            &mut reader,
            Handles::On {
                lifetime_seconds: 3600,
                require_device_binding: false,
                devices_enabled: false,
            },
            attempt,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2),async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')").bind(pid).fetch_one(&mut *writer).await?;
            if waiting { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_,sqlx::Error>(())
    }).await??;
    sqlx::query("SELECT pg_sleep(greatest(0,$1::double precision-extract(epoch FROM clock_timestamp())+0.05))").bind(expires as f64).execute(&mut *writer).await?;
    writer.commit().await?;
    assert!(
        task.await??.is_none(),
        "expired handle was returned after its lock wait"
    );
    f.close().await
}

/// Invalid rows before a live route must not hide it from listing, admission or recovery.
#[tokio::test]
async fn gaa_14_complete_candidates_admission_listing_and_recovery() -> TestResult {
    let f = Fixture::new().await?;
    let world: Uuid = runtime::tests::WORLD.parse()?;
    let channel: Uuid = runtime::tests::CHANNEL.parse()?;
    for index in 0..64_u128 {
        let id = Uuid::from_u128(0x019b0000000070008000000000000000 + index);
        let host = format!("earlier-{index}.example.test");
        let revision = grant::route_revision(1, world, id, &host, 7172, &host)?;
        sqlx::query("INSERT INTO game_channels(world_id,channel_id,channel_key,native_login_enabled,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision) VALUES($1,$2,$3,$4,$5,7172,$5,1,$6)")
            .bind(world).bind(id).bind(if index % 3 == 0 { None } else { Some(format!("earlier-{index}")) }).bind(index % 3 != 1).bind(host).bind(if index % 3 == 2 { "invalid".to_owned() } else { revision }).execute(&f.db.pool).await?;
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&f.db.pool)
            .await?;
    let mut tx = f.db.pool.begin().await?;
    let runtime = &f.authority.issuance.runtime;
    let found = grant::candidates(&mut tx, runtime, world, now).await?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].route.channel, channel);
    assert_eq!(
        grant::select3(&mut tx, runtime, world, Some(channel), now)
            .await?
            .0
            .channel,
        channel
    );
    assert_eq!(
        recovery::test_target(
            &mut tx,
            runtime,
            f.account,
            world,
            channel,
            f.request.character_id,
            now
        )
        .await
        .map_err(|e| format!("recovery target: {e:?}"))?,
        channel
    );
    tx.rollback().await?;
    // A valid duplicate after the old cutoff excludes both endpoints, even when disabled.
    let duplicate = Uuid::now_v7();
    let revision = grant::route_revision(
        1,
        world,
        duplicate,
        "node.example.test",
        7172,
        "node.example.test",
    )?;
    sqlx::query("INSERT INTO game_channels(world_id,channel_id,channel_key,native_login_enabled,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision) VALUES($1,$2,'duplicate',false,'node.example.test',7172,'node.example.test',1,$3)").bind(world).bind(duplicate).bind(revision).execute(&f.db.pool).await?;
    let mut tx = f.db.pool.begin().await?;
    assert!(
        grant::candidates(&mut tx, runtime, world, now)
            .await?
            .is_empty()
    );
    assert_eq!(
        grant::select3(&mut tx, runtime, world, Some(channel), now)
            .await
            .err(),
        Some(Error::RouteUnavailable)
    );
    assert_eq!(
        recovery::test_target(
            &mut tx,
            runtime,
            f.account,
            world,
            channel,
            f.request.character_id,
            now
        )
        .await
        .err(),
        Some(recovery::RecoveryError::RouteUnavailable)
    );
    tx.rollback().await?;
    f.close().await
}
