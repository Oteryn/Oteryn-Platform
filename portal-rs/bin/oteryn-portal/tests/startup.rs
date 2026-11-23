//! Starts the real binary with default flags and checks that it serves.
//! Router assembly happens at startup, so an overlapping route between two
//! modules only shows up as a panic here, not in per-crate tests.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    Some(response)
}

#[test]
fn serve_starts_and_answers_health_and_legacy_redirects() -> Result<(), Box<dyn std::error::Error>>
{
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_oteryn-portal"))
            .arg("serve")
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("OTERYN_ENV", "testing")
            .env("OTERYN_BIND", format!("127.0.0.1:{port}"))
            .env("OTERYN_PUBLIC_ORIGIN", format!("http://127.0.0.1:{port}"))
            // Pools are lazy; startup must not need a reachable database.
            .env(
                "OTERYN_DATABASE_URL",
                "postgres://startup-test@127.0.0.1:1/startup_test",
            )
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let health = loop {
        if let Some(status) = server.0.try_wait()? {
            let mut stderr = String::new();
            if let Some(mut pipe) = server.0.stderr.take() {
                pipe.read_to_string(&mut stderr)?;
            }
            return Err(format!("server exited early ({status}): {stderr}").into());
        }
        if let Some(response) = get(port, "/healthz") {
            break response;
        }
        if Instant::now() > deadline {
            return Err("server did not answer /healthz within 30 s".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(health.starts_with("HTTP/1.1 200"), "{health}");
    let legacy = get(port, "/servers").ok_or("no response for /servers")?;
    assert!(legacy.starts_with("HTTP/1.1 301"), "{legacy}");
    assert!(
        legacy.to_ascii_lowercase().contains("location: /pl/worlds"),
        "{legacy}"
    );
    let unknown = get(port, "/no-such-page").ok_or("no response")?;
    assert!(unknown.starts_with("HTTP/1.1 404"), "{unknown}");
    Ok(())
}

#[test]
fn oracle_candidate_accepts_predecessor_image_check() -> Result<(), Box<dyn std::error::Error>> {
    let check = |directory: &str, oracle: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_oteryn-portal"));
        command
            .env_clear()
            .env("OTERYN_STATIC_DIR", directory)
            .arg("check-image");
        if oracle {
            command.env("OTERYN_ORACLE_DEPLOY_IMAGE", "1");
        }
        command.output()
    };
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../static");
    assert!(check(static_dir, true)?.status.success());
    assert!(check(static_dir, false)?.status.success());
    assert!(
        !check(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"), true)?
            .status
            .success()
    );
    Ok(())
}

#[tokio::test]
async fn live_signup_preflight_requires_staged_bilingual_editions_before_migration()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_db::test_support::{APPROVED_SIGNUP_EDITION, seed_signup_editions};
    use oteryn_identity::consents::DEFAULT_LEGAL_VERSION;

    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = oteryn_db::lazy_pool(&url)?;
    let schema = format!("signup_upgrade_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let separator = if url.contains('?') { '&' } else { '?' };
    let scoped_url = format!("{url}{separator}options=-csearch_path%3D{schema}");
    let pool = oteryn_db::lazy_pool(&scoped_url)?;
    let check = |command: &str, version: &str, oracle: bool| {
        let mut process = Command::new(env!("CARGO_BIN_EXE_oteryn-portal"));
        process
            .arg(command)
            .env_clear()
            .env("OTERYN_ENV", "production")
            .env("OTERYN_DATABASE_URL", &scoped_url)
            .env("OTERYN_TERMS_VERSION", version)
            .env("OTERYN_PRIVACY_VERSION", version)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if oracle {
            process.env("OTERYN_ORACLE_DEPLOY_IMAGE", "1");
        }
        process.status()
    };
    let upgrade = "check-signup-legal-upgrade";
    let strict = "check-signup-legal";
    assert!(check(upgrade, DEFAULT_LEGAL_VERSION, false)?.success());
    assert!(!check(strict, DEFAULT_LEGAL_VERSION, false)?.success());
    let preceding = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            oteryn_db::MIGRATOR
                .iter()
                .filter(|migration| migration.version < 20261122124500)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    preceding.run(&pool).await?;
    let legacy = (APPROVED_SIGNUP_EDITION.0, APPROVED_SIGNUP_EDITION.1, false);
    seed_signup_editions(&pool, DEFAULT_LEGAL_VERSION, legacy, legacy).await?;

    assert!(!check(strict, DEFAULT_LEGAL_VERSION, false)?.success());
    assert!(!check(upgrade, DEFAULT_LEGAL_VERSION, false)?.success());
    assert!(!check("migrate", DEFAULT_LEGAL_VERSION, true)?.success());
    let migration_applied = || {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=20261122124500)",
        )
        .fetch_one(&pool)
    };
    assert!(!migration_applied().await?);

    let approved = "approved-fixture-v2";
    for (version, locale) in [(" ", "en"), (" ", "pl"), (approved, "en"), (approved, "pl")] {
        sqlx::query(
            "WITH staged AS (
                UPDATE editorial_pages SET legal_version=$1 WHERE locale=$2 RETURNING *
             )
             INSERT INTO page_legal_versions(page_id,locale,version,content)
             SELECT page_id,locale,legal_version,jsonb_build_object(
                 'title',title,'body',body,'effective',legal_effective_date::text,'final_approved',true)
             FROM staged",
        )
        .bind(version)
        .bind(locale)
        .execute(&pool)
        .await?;
        let ready = version == approved && locale == "pl";
        assert_eq!(check(strict, version, false)?.success(), ready);
        assert_eq!(check(upgrade, version, false)?.success(), ready);
    }
    assert!(check("migrate", approved, true)?.success());
    assert!(migration_applied().await?);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM page_legal_versions WHERE version=$1")
            .bind(DEFAULT_LEGAL_VERSION)
            .fetch_one(&pool)
            .await?,
        4
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
